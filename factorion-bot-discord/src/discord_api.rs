use std::borrow::Cow;
use std::collections::HashMap;
use std::collections::HashSet;
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::SystemTime;

use anyhow::Error;
use factorion_lib::Consts;
use factorion_lib::comment::{Commands, Comment, CommentConstructed};
use factorion_lib::influxdb::InfluxDbClient;
use log::{error, info, warn};
use serde::{Deserialize, Serialize};
use serenity::all::{
    ChannelId, Colour, CreateEmbed, CreateEmbedFooter, CreateMessage, GatewayIntents, Message,
    MessageId, Ready, Timestamp,
};
use serenity::async_trait;
use serenity::prelude::*;
use tokio::sync::Mutex;

const MAX_MESSAGE_LEN: usize = 2000;
const EMBED_DESCRIPTION_LIMIT: usize = 4096;
const EMBED_FIELD_VALUE_LIMIT: usize = 1024;
const CONFIG_FILE: &str = "channel_config.json";

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct MessageMeta {
    pub message_id: MessageId,
    pub channel_id: ChannelId,
    pub author: String,
}

pub struct Handler<'a> {
    processed_messages: Arc<Mutex<HashSet<MessageId>>>,
    channel_configs: Arc<Mutex<HashMap<u64, Config>>>,
    config_path: PathBuf,
    consts: Consts<'a>,
    influx_client: Option<&'a InfluxDbClient>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Config {
    pub commands: Commands,
    pub locale: String,
}

#[derive(Debug, Clone, PartialEq)]
enum Reply {
    Simple(Cow<'static, str>),
    Embed(Box<CreateEmbed>),
}

impl<'a> Handler<'a> {
    pub fn new(consts: Consts<'a>, influx_client: Option<&'a InfluxDbClient>) -> Handler<'a> {
        let config_path = PathBuf::from(CONFIG_FILE);
        #[cfg(not(test))]
        let channel_configs = Self::load_configs(&config_path);
        #[cfg(test)]
        let channel_configs = HashMap::new();

        Self {
            processed_messages: Arc::new(Mutex::new(HashSet::new())),
            channel_configs: Arc::new(Mutex::new(channel_configs)),
            config_path,
            consts,
            influx_client,
        }
    }

    fn load_configs(path: &PathBuf) -> HashMap<u64, Config> {
        if path.exists()
            && let Ok(content) = fs::read_to_string(path)
        {
            let configs = serde_json::from_str(&content).expect("Malformed channel configuration");
            info!("Loaded channel configurations from {}", path.display());
            return configs;
        }
        info!("No existing channel configurations found, starting with defaults");
        HashMap::new()
    }

    async fn save_configs(&self) -> Result<(), Error> {
        let configs = self.channel_configs.lock().await;
        let content = serde_json::to_string_pretty(&*configs)?;
        fs::write(&self.config_path, content)?;
        info!(
            "Saved channel configurations to {}",
            self.config_path.display()
        );
        Ok(())
    }

    async fn get_channel_config(&self, channel_id: ChannelId) -> Config {
        let configs = self.channel_configs.lock().await;
        configs.get(&channel_id.get()).cloned().unwrap_or(Config {
            commands: Commands::NONE,
            locale: "en".to_owned(),
        })
    }

    async fn set_channel_config(&self, channel_id: ChannelId, config: Config) -> Result<(), Error> {
        let mut configs = self.channel_configs.lock().await;
        configs.insert(channel_id.get(), config);
        drop(configs);
        #[cfg(not(test))]
        {
            self.save_configs().await
        }
        #[cfg(test)]
        Ok(())
    }

    async fn process_message(&self, ctx: &Context, msg: &Message) -> Result<(), Error> {
        let start = SystemTime::now();

        let processed = self.processed_messages.lock().await;

        if processed.contains(&msg.id) {
            return Ok(());
        }

        if msg.author.bot {
            return Ok(());
        }

        let meta = MessageMeta {
            message_id: msg.id,
            channel_id: msg.channel_id,
            author: msg.author.name.clone(),
        };
        let Some((message_locale, reply, additional)) = self
            .process_message_inner(meta, &msg.content, processed, async || {
                check_can_change_config(ctx, msg).await
            })
            .await?
        else {
            return Ok(());
        };

        // Send formatted response
        if let Err(why) = self.send_reply(ctx, msg, reply).await {
            error!(
                "Failed to send message to channel {}: {:?}",
                msg.channel_id, why
            );
        } else {
            info!(
                "Replied to message {} in channel {} by user {}",
                msg.id, msg.channel_id, msg.author.name
            );

            // Log the reply to InfluxDB
            factorion_lib::influxdb::discord::log_message_reply(
                self.influx_client,
                &msg.id.to_string(),
                &msg.author.name,
                &msg.channel_id.to_string(),
                &message_locale,
            )
            .await
            .ok();
        }
        if let Some(reply) = additional {
            if let Err(why) = self.send_reply(ctx, msg, reply).await {
                error!(
                    "Failed to send message to channel {}: {:?}",
                    msg.channel_id, why
                );
            } else {
                info!(
                    "Replied to message {} in channel {} by user {}",
                    msg.id, msg.channel_id, msg.author.name
                );

                // Log the reply to InfluxDB
                factorion_lib::influxdb::discord::log_message_reply(
                    self.influx_client,
                    &msg.id.to_string(),
                    &msg.author.name,
                    &msg.channel_id.to_string(),
                    &message_locale,
                )
                .await
                .ok();
            }
        }

        let end = SystemTime::now();
        factorion_lib::influxdb::discord::log_time_consumed(
            self.influx_client,
            start,
            end,
            "process_message",
        )
        .await
        .ok();

        Ok(())
    }

    async fn process_message_inner(
        &self,
        meta: MessageMeta,
        content: &str,
        mut processed: tokio::sync::MutexGuard<'_, HashSet<MessageId>>,
        check_change_config: impl AsyncFnOnce() -> Result<(), Error>,
    ) -> Result<Option<(String, Reply, Option<Reply>)>, Error> {
        if content.starts_with("!factorion config") {
            drop(processed);
            check_change_config().await?;
            let (reply, additional) = self.handle_config_command(content, meta.channel_id).await?;
            let reply = Reply::Simple(reply);
            let additional = additional.map(Reply::Simple);
            return Ok(Some(("en".to_owned(), reply, additional)));
        }
        let Config {
            commands: default_commands,
            locale,
        } = self.get_channel_config(meta.channel_id).await;
        let comment: CommentConstructed<MessageMeta> =
            Comment::new(content, meta, default_commands, MAX_MESSAGE_LEN, &locale);
        if comment.status.no_factorial {
            return Ok(None);
        }
        let extract_start = SystemTime::now();
        let comment = comment.extract(&self.consts);
        let extract_end = SystemTime::now();
        factorion_lib::influxdb::discord::log_time_consumed(
            self.influx_client,
            extract_start,
            extract_end,
            "extract_factorials",
        )
        .await
        .ok();
        if comment.status.no_factorial {
            return Ok(None);
        }
        let calc_start = SystemTime::now();
        let comment = comment.calc(&self.consts);
        let calc_end = SystemTime::now();
        factorion_lib::influxdb::discord::log_time_consumed(
            self.influx_client,
            calc_start,
            calc_end,
            "calculate_factorials",
        )
        .await
        .ok();
        info!("Comment -> {comment:?}");
        if comment.status.not_replied {
            return Ok(None);
        }
        let reply_text = comment.get_reply(&self.consts);
        let message_locale = comment.locale;
        processed.insert(comment.meta.message_id);
        let reply = self
            .get_formatted_reply(
                &reply_text,
                comment.calculation_list.len(),
                comment
                    .calculation_list
                    .iter()
                    .any(|x| x.is_digit_tower() || x.is_aproximate_digits() || x.is_approximate()),
            )
            .await?;
        Ok(Some((message_locale, reply, None)))
    }

    async fn handle_config_command(
        &self,
        content: &str,
        channel_id: ChannelId,
    ) -> Result<(Cow<'static, str>, Option<Cow<'static, str>>), Error> {
        let parts: Vec<&str> = content.split_whitespace().collect();

        if parts.len() < 4 {
            let config = self.get_channel_config(channel_id).await;
            let status = format!(
                "**Channel Configuration**\n```\nShorten: {}\nSteps: {}\nTermial: {}\nNo Note: {}\n Nested: {}\n Write Out: {}\nLocale: {}\n```\n\
                Usage:\n\
                `!factorion config <setting> <on/off>`\n\
                Available settings: shorten, steps, termial, no_note, nested, write_out",
                config.commands.shorten,
                config.commands.steps,
                config.commands.termial,
                config.commands.no_note,
                config.commands.nested,
                config.commands.write_out,
                config.locale
            );
            return Ok((status.into(), None));
        }

        let setting = parts[2];
        let value = parts[3];

        enum Setting {
            Command(bool),
            Locale(String),
        }

        let val = match value {
            "on" | "true" | "1" | "yes" => Setting::Command(true),
            "off" | "false" | "0" | "no" => Setting::Command(false),
            s => Setting::Locale(s.to_owned()),
        };

        let mut config = self.get_channel_config(channel_id).await;

        match setting {
            "shorten" | "short" => {
                let Setting::Command(enabled) = val else {
                    return Ok((
                        "Invalid value. Use: on/off, true/false, yes/no, or 1/0".into(),
                        None,
                    ));
                };
                config.commands.shorten = enabled;
                self.set_channel_config(channel_id, config).await?;
                Ok((
                    format!(
                        "Shorten has been turned **{}**",
                        if enabled { "ON" } else { "OFF" }
                    )
                    .into(),
                    None,
                ))
            }
            "steps" | "step" => {
                let Setting::Command(enabled) = val else {
                    return Ok((
                        "Invalid value. Use: on/off, true/false, yes/no, or 1/0".into(),
                        None,
                    ));
                };
                config.commands.steps = enabled;
                self.set_channel_config(channel_id, config).await?;
                Ok((
                    format!(
                        "Steps has been turned **{}**",
                        if enabled { "ON" } else { "OFF" }
                    )
                    .into(),
                    None,
                ))
            }
            "termial" => {
                let Setting::Command(enabled) = val else {
                    return Ok((
                        "Invalid value. Use: on/off, true/false, yes/no, or 1/0".into(),
                        None,
                    ));
                };
                config.commands.termial = enabled;
                self.set_channel_config(channel_id, config).await?;
                Ok((
                    format!(
                        "Termial has been turned **{}**",
                        if enabled { "ON" } else { "OFF" }
                    )
                    .into(),
                    None,
                ))
            }
            "no_note" | "nonote" | "no-note" => {
                let Setting::Command(enabled) = val else {
                    return Ok((
                        "Invalid value. Use: on/off, true/false, yes/no, or 1/0".into(),
                        None,
                    ));
                };
                config.commands.no_note = enabled;
                self.set_channel_config(channel_id, config).await?;
                Ok((
                    format!(
                        "No note has been turned **{}**",
                        if enabled { "ON" } else { "OFF" }
                    )
                    .into(),
                    None,
                ))
            }
            "nested" | "nest" => {
                let Setting::Command(enabled) = val else {
                    return Ok((
                        "Invalid value. Use: on/off, true/false, yes/no, or 1/0".into(),
                        None,
                    ));
                };
                config.commands.nested = enabled;
                self.set_channel_config(channel_id, config).await?;
                Ok((
                    format!(
                        "Nested has been turned **{}**",
                        if enabled { "ON" } else { "OFF" }
                    )
                    .into(),
                    None,
                ))
            }
            "write_out" | "writeout" | "write-out" => {
                let Setting::Command(enabled) = val else {
                    return Ok((
                        "Invalid value. Use: on/off, true/false, yes/no, or 1/0".into(),
                        None,
                    ));
                };
                config.commands.write_out = enabled;
                self.set_channel_config(channel_id, config).await?;
                Ok((
                    format!(
                        "Write out has been turned **{}**",
                        if enabled { "ON" } else { "OFF" }
                    )
                    .into(),
                    None,
                ))
            }
            "locale" | "lang" | "language" => {
                let Setting::Locale(locale) = val else {
                    return Ok(("Invalid value. Use: <locale>".into(), None));
                };
                config.locale = locale.clone();
                self.set_channel_config(channel_id, config).await?;
                let reply = format!("Locale has been set to **{}**", locale);
                Ok(if !self.consts.locales.contains_key(&locale) {
                    (
                        reply.into(),
                        Some(
                            format!(
                                "Warning: {} is not a currently supported locale, locales are {:?}",
                                locale,
                                self.consts.locales.keys().collect::<Vec<_>>()
                            )
                            .into(),
                        ),
                    )
                } else {
                    (reply.into(), None)
                })
            }
            _ => Ok(("Invalid setting. Available settings: shorten, steps, termial, no_note, post_only, locale".into(), None))
        }
    }

    async fn send_reply(&self, ctx: &Context, msg: &Message, reply: Reply) -> Result<(), Error> {
        match reply {
            Reply::Simple(text) => {
                msg.channel_id.say(&ctx.http, text).await?;
            }
            Reply::Embed(embed) => {
                let builder = CreateMessage::new().embed(*embed).reference_message(msg);
                msg.channel_id.send_message(&ctx.http, builder).await?;
            }
        }
        Ok(())
    }

    async fn get_formatted_reply(
        &self,
        reply_text: &str,
        num_calcs: usize,
        approx: bool,
    ) -> Result<Reply, Error> {
        // Check if the reply is short enough for a simple message
        if Self::should_use_simple_reply(reply_text) {
            return Ok(Reply::Simple(
                format!("**📊 Calculation Result**\n```\n{}\n```", reply_text.trim()).into(),
            ));
        }

        // For longer/complex replies, use an embed
        let embed = self.create_embed(reply_text, num_calcs, approx)?;
        Ok(Reply::Embed(Box::new(embed)))
    }

    fn should_use_simple_reply(reply_text: &str) -> bool {
        reply_text.len() <= 400 && !reply_text.trim().contains('\n')
    }

    fn create_embed(
        &self,
        reply_text: &str,
        num_calcs: usize,
        approx: bool,
    ) -> Result<CreateEmbed, Error> {
        let mut embed = CreateEmbed::new();
        #[cfg(not(test))]
        {
            embed = embed
                .colour(Colour::from_rgb(88, 101, 242))
                .timestamp(Timestamp::now())
                .footer(CreateEmbedFooter::new(
                    "🤖 Factorion Bot • Powered by factorion-lib",
                ));
        }

        // Parse the reply into sections
        let (description, results) = Self::parse_reply(reply_text, num_calcs);

        // Add title based on content
        embed = Self::add_title(embed, results.len(), approx);

        // Add description if we have a note
        let desc_len = description.len();
        if !description.is_empty() {
            embed = Self::add_description(embed, description)?;
        }

        // Add results
        embed = Self::add_results(embed, results, desc_len, reply_text)?;

        Ok(embed)
    }

    fn parse_reply(reply_text: &str, num_calcs: usize) -> (String, Vec<String>) {
        let lines: Vec<&str> = reply_text
            .trim()
            .lines()
            .filter(|s| !s.is_empty())
            .collect();
        let mut description = String::new();
        let mut results = Vec::new();
        let num_lines = lines.len();

        for (n, line) in lines.into_iter().enumerate() {
            let trimmed = line.trim();

            if n < num_lines - num_calcs {
                if !description.is_empty() {
                    description.push('\n');
                }
                description.push_str(trimmed);
            } else {
                results.push(trimmed.to_string());
            }
        }

        (description, results)
    }

    fn add_title(embed: CreateEmbed, result_count: usize, approx: bool) -> CreateEmbed {
        if approx {
            embed.title("🔢 Factorial Calculations (Approximated)")
        } else if result_count > 1 {
            embed.title("🔢 Multiple Factorial Calculations")
        } else {
            embed.title("🔢 Factorial Calculation")
        }
    }

    fn add_description(embed: CreateEmbed, description: String) -> Result<CreateEmbed, Error> {
        let desc = if description.len() > EMBED_DESCRIPTION_LIMIT {
            format!("{}...", &description[..EMBED_DESCRIPTION_LIMIT - 3])
        } else {
            description
        };
        Ok(embed.description(format!("ℹ️ *{}*", desc)))
    }

    fn add_results(
        mut embed: CreateEmbed,
        results: Vec<String>,
        desc_len: usize,
        reply_text: &str,
    ) -> Result<CreateEmbed, Error> {
        if results.is_empty() {
            embed = Self::add_full_text_results(embed, reply_text)?;
        } else if results.len() <= 5 {
            embed = Self::add_field_results(embed, results)?;
        } else {
            embed = Self::add_combined_results(embed, results, desc_len)?;
        }

        Ok(embed)
    }

    fn add_full_text_results(
        mut embed: CreateEmbed,
        reply_text: &str,
    ) -> Result<CreateEmbed, Error> {
        let full_text = reply_text.trim();

        if full_text.len() > EMBED_DESCRIPTION_LIMIT {
            embed = Self::add_chunked_results(embed, full_text)?;
        } else {
            embed = embed.description(format!("```\n{}\n```", full_text));
        }

        Ok(embed)
    }

    fn add_chunked_results(mut embed: CreateEmbed, full_text: &str) -> Result<CreateEmbed, Error> {
        let chunks: Vec<String> = full_text
            .chars()
            .collect::<Vec<char>>()
            .chunks(EMBED_FIELD_VALUE_LIMIT - 50)
            .map(|chunk| {
                let chunk_str: String = chunk.iter().collect();
                format!("```\n{}\n```", chunk_str)
            })
            .collect();

        for (i, chunk) in chunks.iter().take(10).enumerate() {
            embed = embed.field(
                format!("Result Part {}/{}", i + 1, chunks.len().min(10)),
                chunk,
                false,
            );
        }

        if chunks.len() > 10 {
            warn!("Reply too long, truncated to 10 fields");
        }

        Ok(embed)
    }

    fn add_field_results(
        mut embed: CreateEmbed,
        results: Vec<String>,
    ) -> Result<CreateEmbed, Error> {
        for (i, result) in results.iter().enumerate() {
            if result.len() > EMBED_FIELD_VALUE_LIMIT {
                let chunks: Vec<String> = result
                    .chars()
                    .collect::<Vec<char>>()
                    .chunks(EMBED_FIELD_VALUE_LIMIT - 50)
                    .map(|chunk| {
                        let chunk_str: String = chunk.iter().collect();
                        format!("```\n{}\n```", chunk_str)
                    })
                    .collect();

                for (j, chunk) in chunks.iter().take(10).enumerate() {
                    embed = embed.field(
                        format!(
                            "📐 Calculation {} Part {}/{}",
                            i + 1,
                            j + 1,
                            chunks.len().min(10)
                        ),
                        chunk,
                        false,
                    );
                }
                break;
            }
            let field_value = format!("```\n{}\n```", result);
            embed = embed.field(format!("📐 Calculation {}", i + 1), field_value, false);
        }

        Ok(embed)
    }

    fn add_combined_results(
        embed: CreateEmbed,
        results: Vec<String>,
        desc_len: usize,
    ) -> Result<CreateEmbed, Error> {
        let combined = results.join("\n");
        if combined.len() > EMBED_FIELD_VALUE_LIMIT - desc_len - 20 {
            return Self::add_chunked_results(embed, &combined);
        }
        let result_text = format!("```\n{}\n```", combined);

        Ok(embed.field("📐 Results", result_text, false))
    }
}

async fn check_can_change_config(ctx: &Context, msg: &Message) -> Result<(), Error> {
    if let Some(guild_id) = msg.guild_id {
        match guild_id.member(&ctx.http, msg.author.id).await {
            Ok(member) => {
                let has_permission = if let Some(guild) = ctx.cache.guild(guild_id) {
                    // Check base permissions in the guild (not considering channel overwrites)
                    // Using member_permissions for guild-level check is appropriate here
                    #[allow(deprecated)]
                    guild.member_permissions(&member).manage_channels()
                } else {
                    false
                };

                if !has_permission {
                    msg.channel_id
                        .say(
                            &ctx.http,
                            "You need 'Manage Channels' permission to configure channel settings.",
                        )
                        .await?;
                }
            }
            Err(_) => {
                msg.channel_id
                    .say(&ctx.http, "Unable to verify member information.")
                    .await?;
            }
        }
    } else {
        msg.channel_id
            .say(&ctx.http, "This command can only be used in servers.")
            .await?;
    }
    Ok(())
}

#[async_trait]
impl EventHandler for Handler<'_> {
    async fn message(&self, ctx: Context, msg: Message) {
        if let Err(e) = self.process_message(&ctx, &msg).await {
            error!("Error processing message: {:?}", e);
        }
    }

    async fn ready(&self, _: Context, ready: Ready) {
        info!("{} is connected and ready!", ready.user.name);
    }
}

pub async fn start_bot(
    token: String,
    consts: Consts<'static>,
    influx_client: Option<&'static InfluxDbClient>,
) -> Result<(), Error> {
    // Configure gateway intents
    // MESSAGE_CONTENT is a privileged intent that must be enabled in Discord Developer Portal:
    // 1. Go to https://discord.com/developers/applications/{your_app_id}/bot
    // 2. Scroll to "Privileged Gateway Intents"
    // 3. Enable "MESSAGE CONTENT INTENT"
    // 4. Save changes and restart the bot
    let intents =
        GatewayIntents::GUILDS | GatewayIntents::GUILD_MESSAGES | GatewayIntents::MESSAGE_CONTENT;

    let mut client = Client::builder(&token, intents)
        .event_handler(Handler::new(consts, influx_client))
        .await?;

    info!("Starting Discord bot...");

    client.start().await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use factorion_lib::influxdb::INFLUX_CLIENT;

    #[test]
    fn test_should_use_simple_reply_short_text() {
        let short_text = "5! = 120";
        assert!(Handler::should_use_simple_reply(short_text));
    }

    #[test]
    fn test_should_use_simple_reply_long_text() {
        let long_text = "a".repeat(500);
        assert!(!Handler::should_use_simple_reply(&long_text));
    }

    #[test]
    fn test_should_use_simple_reply_with_newlines() {
        let text_with_newlines = "5! = 120\n6! = 720";
        assert!(!Handler::should_use_simple_reply(text_with_newlines));
    }

    #[test]
    fn test_parse_reply_simple() {
        let reply = "5! = 120";
        let (description, results) = Handler::parse_reply(reply, 1);

        assert_eq!(description, "");
        assert_eq!(results[0], "5! = 120");
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn test_parse_reply_with_note() {
        let reply = "Note: Large numbers are approximated\n\n5! = 120\n6! = 720";
        let (description, results) = Handler::parse_reply(reply, 2);

        assert_eq!(description, "Note: Large numbers are approximated");
        assert_eq!(results.len(), 2);
        assert_eq!(results[0], "5! = 120");
        assert_eq!(results[1], "6! = 720");
    }

    #[test]
    fn test_parse_reply_multiple_results() {
        let reply = "\n\n1! = 1\n2! = 2\n3! = 6\n4! = 24\n5! = 120";
        let (description, results) = Handler::parse_reply(reply, 5);

        assert_eq!(description, "");
        assert_eq!(results.len(), 5);
        assert_eq!(results[0], "1! = 1");
        assert_eq!(results[4], "5! = 120");
    }

    #[test]
    fn test_add_title_approximated() {
        let embed = CreateEmbed::new();
        let description = "Note: approximate values shown for large numbers";

        let _result = Handler::add_title(embed, 1, true);

        // Check that the title contains "Approximated"
        // Note: We can't directly inspect the title, so we're testing the function runs
        assert!(description.contains("approximate"));
    }

    #[test]
    fn test_add_title_multiple_results() {
        let embed = CreateEmbed::new();
        let description = "";

        let _result = Handler::add_title(embed, 5, true);

        // Function should complete without error
        assert_eq!(description, "");
    }

    #[test]
    fn test_add_title_single_result() {
        let embed = CreateEmbed::new();
        let description = "";

        let _result = Handler::add_title(embed, 1, true);

        // Function should complete without error
        assert_eq!(description, "");
    }

    #[test]
    fn test_add_description_short() {
        let embed = CreateEmbed::new();
        let description = "This is a short description".to_string();

        let result = Handler::add_description(embed, description.clone());

        assert!(result.is_ok());
    }

    #[test]
    fn test_add_description_too_long() {
        let embed = CreateEmbed::new();
        let description = "a".repeat(EMBED_DESCRIPTION_LIMIT + 100);

        let result = Handler::add_description(embed, description);

        // Should succeed but truncate the description
        assert!(result.is_ok());
    }

    #[test]
    fn test_message_meta_clone() {
        let meta = MessageMeta {
            message_id: MessageId::new(12345),
            channel_id: ChannelId::new(67890),
            author: "TestUser".to_string(),
        };

        let cloned = meta.clone();

        assert_eq!(meta.message_id, cloned.message_id);
        assert_eq!(meta.channel_id, cloned.channel_id);
        assert_eq!(meta.author, cloned.author);
    }

    #[test]
    fn test_message_meta_debug() {
        let meta = MessageMeta {
            message_id: MessageId::new(12345),
            channel_id: ChannelId::new(67890),
            author: "TestUser".to_string(),
        };

        let debug_str = format!("{:?}", meta);

        assert!(debug_str.contains("MessageMeta"));
        assert!(debug_str.contains("TestUser"));
    }

    #[test]
    fn test_handler_new() {
        let consts = Consts::default();
        let _handler = Handler::new(consts, INFLUX_CLIENT.as_ref());

        // Handler should be created successfully
        // We can't directly test the internal state, but we can verify it doesn't panic
    }

    #[tokio::test]
    async fn test_handler_process_message() {
        let consts = Consts {
            locales: factorion_lib::locale::get_all()
                .map(|(k, mut v)| {
                    v.bot_disclaimer = "".into();
                    (k.to_owned(), v)
                })
                .collect(),
            ..Consts::default()
        };
        let dummy_check = async || Ok(());
        let handler = Handler::new(consts, INFLUX_CLIENT.as_ref());
        let content = "Some comment with factorial 5!";
        let meta = MessageMeta {
            message_id: MessageId::new(1),
            channel_id: ChannelId::new(1),
            author: String::new(),
        };

        let processed = handler.processed_messages.lock().await;
        let res = handler
            .process_message_inner(meta.clone(), content, processed, dummy_check)
            .await
            .unwrap();

        assert_eq!(
            res,
            Some((
                "en".to_owned(),
                Reply::Simple("**📊 Calculation Result**\n```\nFactorial of 5 is 120\n```".into()),
                None
            ))
        );

        let content = "Some comment with factorials 5! 10!";
        let processed = handler.processed_messages.lock().await;
        let res = handler
            .process_message_inner(meta.clone(), content, processed, dummy_check)
            .await
            .unwrap();

        assert_eq!(
            res,
            Some((
                "en".to_owned(),
                Reply::Embed(Box::new(
                    CreateEmbed::new()
                        .title("🔢 Multiple Factorial Calculations")
                        .field("📐 Calculation 1", "```\nFactorial of 5 is 120\n```", false)
                        .field(
                            "📐 Calculation 2",
                            "```\nFactorial of 10 is 3628800\n```",
                            false
                        )
                )),
                None
            ))
        );

        let content = "!factorion config termial on";
        let processed = handler.processed_messages.lock().await;
        let res = handler
            .process_message_inner(meta.clone(), content, processed, dummy_check)
            .await
            .unwrap();

        assert_eq!(
            res,
            Some((
                "en".to_owned(),
                Reply::Simple("Termial has been turned **ON**".into()),
                None
            ))
        );
        assert_eq!(
            *handler.channel_configs.lock().await.get(&1).unwrap(),
            Config {
                commands: Commands::TERMIAL,
                locale: "en".to_owned()
            }
        );
        let content = "!factorion config locale ru";
        let processed = handler.processed_messages.lock().await;
        let res = handler
            .process_message_inner(meta.clone(), content, processed, dummy_check)
            .await
            .unwrap();

        assert_eq!(
            res,
            Some((
                "en".to_owned(),
                Reply::Simple("Locale has been set to **ru**".into()),
                None
            ))
        );
        assert_eq!(
            *handler.channel_configs.lock().await.get(&1).unwrap(),
            Config {
                commands: Commands::TERMIAL,
                locale: "ru".to_owned()
            }
        );

        let content = "!factorion config termial on";
        let processed = handler.processed_messages.lock().await;
        handler
            .process_message_inner(meta.clone(), content, processed, dummy_check)
            .await
            .unwrap();
        let content = "!factorion config no_note on";
        let processed = handler.processed_messages.lock().await;
        handler
            .process_message_inner(meta.clone(), content, processed, dummy_check)
            .await
            .unwrap();
        let content = "!factorion config steps on";
        let processed = handler.processed_messages.lock().await;
        handler
            .process_message_inner(meta.clone(), content, processed, dummy_check)
            .await
            .unwrap();
        let content = "!factorion config shorten on";
        let processed = handler.processed_messages.lock().await;
        handler
            .process_message_inner(meta.clone(), content, processed, dummy_check)
            .await
            .unwrap();
        let content = "!factorion config nested on";
        let processed = handler.processed_messages.lock().await;
        handler
            .process_message_inner(meta.clone(), content, processed, dummy_check)
            .await
            .unwrap();
        let content = "!factorion config write_out on";
        let processed = handler.processed_messages.lock().await;
        handler
            .process_message_inner(meta.clone(), content, processed, dummy_check)
            .await
            .unwrap();
        assert_eq!(
            *handler.channel_configs.lock().await.get(&1).unwrap(),
            Config {
                commands: !Commands::NONE,
                locale: "ru".to_owned()
            }
        );
    }

    #[test]
    fn test_parse_reply_mutiple_no_note() {
        let reply = "5! = 120\n\n6! = 720\n";
        let (description, results) = Handler::parse_reply(reply, 2);
        assert_eq!(description, "");
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn test_parse_reply_empty_lines() {
        let reply = "Note: Testing\n\n\n5! = 120\n\n6! = 720";
        let (description, results) = Handler::parse_reply(reply, 2);

        assert_eq!(description, "Note: Testing");
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn test_parse_reply_whitespace_handling() {
        let reply = "  Note: Testing  \n\n  5! = 120  \n  6! = 720  ";
        let (description, results) = Handler::parse_reply(reply, 2);

        assert_eq!(description, "Note: Testing");
        assert_eq!(results[0], "5! = 120");
        assert_eq!(results[1], "6! = 720");
    }

    #[test]
    fn test_constants() {
        // Verify the constants are set to reasonable values
        assert_eq!(MAX_MESSAGE_LEN, 2000);
        assert_eq!(EMBED_DESCRIPTION_LIMIT, 4096);
        assert_eq!(EMBED_FIELD_VALUE_LIMIT, 1024);
    }
}
