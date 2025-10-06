use std::collections::HashSet;
use std::sync::Arc;

use anyhow::Error;
use factorion_lib::comment::{Commands, Comment, CommentConstructed};
use log::{error, info, warn};
use serenity::all::{
    ChannelId, Colour, CreateEmbed, CreateEmbedFooter, CreateMessage,
    GatewayIntents, Message, MessageId, Ready, Timestamp,
};
use serenity::async_trait;
use serenity::prelude::*;
use tokio::sync::Mutex;

const MAX_MESSAGE_LEN: usize = 2000;
const EMBED_DESCRIPTION_LIMIT: usize = 4096;
const EMBED_FIELD_VALUE_LIMIT: usize = 1024;

#[derive(Debug, Clone)]
pub struct MessageMeta {
    pub message_id: MessageId,
    pub channel_id: ChannelId,
    pub author: String,
}

pub struct Handler {
    processed_messages: Arc<Mutex<HashSet<MessageId>>>,
}

impl Handler {
    pub fn new() -> Self {
        Self {
            processed_messages: Arc::new(Mutex::new(HashSet::new())),
        }
    }

    async fn process_message(&self, ctx: &Context, msg: &Message) -> Result<(), Error> {
        let mut processed = self.processed_messages.lock().await;
        
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

        let commands = Commands::from_comment_text(&msg.content);
        
        let comment: CommentConstructed<MessageMeta> =
            Comment::new(&msg.content, meta, commands, MAX_MESSAGE_LEN);

        if comment.status.no_factorial {
            return Ok(());
        }

        let comment = comment.extract();
        
        if comment.status.no_factorial {
            return Ok(());
        }

        let comment = comment.calc();

        // Check if we should reply based on the comment's status
        if comment.status.not_replied {
            return Ok(());
        }

        let reply_text = comment.get_reply();

        processed.insert(msg.id);

        // Send formatted response
        if let Err(why) = self.send_formatted_reply(ctx, msg, &reply_text).await {
            error!(
                "Failed to send message to channel {}: {:?}",
                msg.channel_id, why
            );
        } else {
            info!(
                "Replied to message {} in channel {} by user {}",
                msg.id, msg.channel_id, msg.author.name
            );
        }

        Ok(())
    }

    async fn send_formatted_reply(
        &self,
        ctx: &Context,
        msg: &Message,
        reply_text: &str,
    ) -> Result<(), Error> {
        // Check if the reply is short enough for a simple message
        if Self::should_use_simple_reply(reply_text) {
            return Self::send_simple_reply(ctx, msg, reply_text).await;
        }

        // For longer/complex replies, use an embed
        let embed = self.create_embed(reply_text)?;
        
        // Send the embed
        let builder = CreateMessage::new()
            .embed(embed)
            .reference_message(msg);
        
        msg.channel_id.send_message(&ctx.http, builder).await?;

        Ok(())
    }

    fn should_use_simple_reply(reply_text: &str) -> bool {
        reply_text.len() <= 400 && !reply_text.contains('\n')
    }

    async fn send_simple_reply(
        ctx: &Context,
        msg: &Message,
        reply_text: &str,
    ) -> Result<(), Error> {
        let formatted = format!(
            "**📊 Calculation Result**\n```\n{}\n```",
            reply_text.trim_end_matches("*^(This action was performed by a bot.)*").trim()
        );
        
        msg.channel_id.say(&ctx.http, formatted).await?;
        Ok(())
    }

    fn create_embed(&self, reply_text: &str) -> Result<CreateEmbed, Error> {
        let mut embed = CreateEmbed::new()
            .colour(Colour::from_rgb(88, 101, 242))
            .timestamp(Timestamp::now())
            .footer(CreateEmbedFooter::new("🤖 Factorion Bot • Powered by factorion-lib"));

        // Parse the reply into sections
        let (description, results) = Self::parse_reply(reply_text);
        
        // Add title based on content
        embed = Self::add_title(embed, &description, results.len());
        
        // Add description if we have a note
        let desc_len = description.len();
        if !description.is_empty() {
            embed = Self::add_description(embed, description)?;
        }

        // Add results
        embed = Self::add_results(embed, results, desc_len, reply_text)?;

        Ok(embed)
    }

    fn parse_reply(reply_text: &str) -> (String, Vec<String>) {
        let lines: Vec<&str> = reply_text.lines().collect();
        let mut description = String::new();
        let mut results = Vec::new();
        let mut in_note = true;
        
        for line in lines {
            let trimmed = line.trim();
            
            // Skip the footer
            if trimmed.contains("This action was performed by a bot") {
                continue;
            }
            
            // Empty line marks end of note section
            if trimmed.is_empty() {
                in_note = false;
                continue;
            }
            
            if in_note {
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

    fn add_title(mut embed: CreateEmbed, description: &str, result_count: usize) -> CreateEmbed {
        if description.contains("approximate") || description.contains("large") {
            embed.title("🔢 Factorial Calculations (Approximated)")
        } else if result_count > 1 {
            embed.title("🔢 Multiple Factorial Calculations")
        } else {
            embed.title("🔢 Factorial Calculation")
        }
    }

    fn add_description(mut embed: CreateEmbed, description: String) -> Result<CreateEmbed, Error> {
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

    fn add_full_text_results(mut embed: CreateEmbed, reply_text: &str) -> Result<CreateEmbed, Error> {
        let full_text = reply_text
            .trim_end_matches("*^(This action was performed by a bot.)*")
            .trim();
        
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
            .enumerate()
            .map(|(_i, chunk)| {
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

    fn add_field_results(mut embed: CreateEmbed, results: Vec<String>) -> Result<CreateEmbed, Error> {
        for (i, result) in results.iter().enumerate() {
            let field_value = if result.len() > EMBED_FIELD_VALUE_LIMIT {
                format!("```\n{}...\n```", &result[..EMBED_FIELD_VALUE_LIMIT - 20])
            } else {
                format!("```\n{}\n```", result)
            };
            
            embed = embed.field(
                format!("📐 Calculation {}", i + 1),
                field_value,
                false,
            );
        }
        
        Ok(embed)
    }

    fn add_combined_results(
        mut embed: CreateEmbed,
        results: Vec<String>,
        desc_len: usize,
    ) -> Result<CreateEmbed, Error> {
        let combined = results.join("\n");
        let result_text = if combined.len() > EMBED_DESCRIPTION_LIMIT - desc_len - 20 {
            format!("```\n{}...\n```", &combined[..EMBED_DESCRIPTION_LIMIT - desc_len - 30])
        } else {
            format!("```\n{}\n```", combined)
        };
        
        Ok(embed.field("📐 Results", result_text, false))
    }
}

#[async_trait]
impl EventHandler for Handler {
    async fn message(&self, ctx: Context, msg: Message) {
        if let Err(e) = self.process_message(&ctx, &msg).await {
            error!("Error processing message: {:?}", e);
        }
    }

    async fn ready(&self, _: Context, ready: Ready) {
        info!("{} is connected and ready!", ready.user.name);
    }
}

pub async fn start_bot(token: String) -> Result<(), Error> {
    // Configure gateway intents
    // MESSAGE_CONTENT is a privileged intent that must be enabled in Discord Developer Portal:
    // 1. Go to https://discord.com/developers/applications/{your_app_id}/bot
    // 2. Scroll to "Privileged Gateway Intents"
    // 3. Enable "MESSAGE CONTENT INTENT"
    // 4. Save changes and restart the bot
    let intents = GatewayIntents::GUILDS 
        | GatewayIntents::GUILD_MESSAGES 
        | GatewayIntents::MESSAGE_CONTENT;

    let mut client = Client::builder(&token, intents)
        .event_handler(Handler::new())
        .await?;

    info!("Starting Discord bot...");
    
    client.start().await?;

    Ok(())
}
