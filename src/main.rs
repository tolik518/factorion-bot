use dotenvy::dotenv;
use influxdb::INFLUX_CLIENT;
use log::{error, info, warn};
use reddit_api::RedditClient;
use reddit_comment::{Commands, RedditComment, Status};
use std::collections::HashMap;
use std::error::Error;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::panic;
use std::sync::OnceLock;
use std::time::SystemTime;
use tokio::time::{sleep, Duration};

mod calculation_results;
mod calculation_tasks;
mod influxdb;
mod math;
mod reddit_api;
pub(crate) mod reddit_comment;

const API_COMMENT_COUNT: u32 = 100;
const COMMENT_IDS_FILE_PATH: &str = "comment_ids.txt";
static COMMENT_COUNT: OnceLock<u32> = OnceLock::new();
static SUBREDDIT_COMMANDS: OnceLock<HashMap<&str, Commands>> = OnceLock::new();

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    init();

    let influx_client = &*INFLUX_CLIENT;

    if influx_client.is_none() {
        warn!("InfluxDB client not configured. No influxdb metrics will be logged.");
    } else {
        info!("InfluxDB client configured. Metrics will be logged.");
    }

    let mut reddit_client = RedditClient::new().await?;
    COMMENT_COUNT.set(API_COMMENT_COUNT).unwrap();
    let mut requests_per_loop = 0.0;

    let dont_reply = std::env::var("DONT_REPLY").unwrap_or_default();
    let dont_reply = dont_reply == "true";

    let subreddit_commands = std::env::var("SUBREDDITS").unwrap_or_default();
    let subreddit_commands = subreddit_commands.leak();
    let commands = subreddit_commands
        .split('+')
        .map(|s| s.split_once(':').unwrap_or((s, "")))
        .filter(|s| !s.0.is_empty())
        .map(|(sub, commands)| {
            (
                sub,
                commands
                    .split(',')
                    .map(|command| match command.trim() {
                        "shorten" => Commands::SHORTEN,
                        "termial" => Commands::TERMIAL,
                        "steps" => Commands::STEPS,
                        "no_note" => Commands::NO_NOTE,
                        "post_only" => Commands::POST_ONLY,
                        "" => Commands::NONE,
                        s => panic!("Unknown command in subreddit {sub}: {s}"),
                    })
                    .fold(Commands::NONE, |a, e| a | e),
            )
        })
        .collect::<HashMap<_, _>>();
    if !commands.is_empty() {
        requests_per_loop += 1.0;
        if !commands.values().all(|v| v.post_only) {
            requests_per_loop += 1.0;
        }
    }
    SUBREDDIT_COMMANDS.set(commands).unwrap();

    let check_mentions = std::env::var("CHECK_MENTIONS").expect("CHECK_MENTIONS must be set");
    let check_mentions = check_mentions == "true";
    if check_mentions {
        requests_per_loop += 1.0;
    }
    let check_posts = std::env::var("CHECK_POSTS").expect("CHECK_POSTS must be set");
    let check_posts = check_posts == "true";

    let posts_every = std::env::var("POSTS_EVERY").unwrap_or("1".to_owned());
    let posts_every: u8 = posts_every.parse().expect("POSTS_EVERY is not a number");
    let mentions_every = std::env::var("MENTIONS_EVERY").unwrap_or("1".to_owned());
    let mentions_every: u8 = mentions_every
        .parse()
        .expect("MENTIONS_EVERY is not a number");

    // read comment_ids from the file
    let already_replied_to_comments: String =
        fs::read_to_string(COMMENT_IDS_FILE_PATH).unwrap_or("".to_string());

    if already_replied_to_comments.is_empty() {
        info!("No comment_ids found in the file");
    } else {
        info!("Found comment_ids in the file");
    }

    let mut already_replied_or_rejected: Vec<String> = already_replied_to_comments
        .lines()
        .map(|s| s.to_string())
        .collect::<Vec<String>>();
    let mut last_ids = Default::default();

    // Polling Reddit for new comments
    for i in (0..u8::MAX).cycle() {
        info!("Polling Reddit for new comments...");

        let start = SystemTime::now();
        let (comments, mut rate) = reddit_client
            .get_comments(
                &mut already_replied_or_rejected,
                check_mentions && i % mentions_every == 0,
                check_posts && i % posts_every == 0,
                &mut last_ids,
            )
            .await
            .unwrap_or_default();
        let end = SystemTime::now();

        influxdb::log_time_consumed(influx_client, start, end, "get_comments").await?;

        let start = SystemTime::now();
        let comments = comments
            .into_iter()
            .filter_map(|c| {
                let id = c.id.clone();
                match std::panic::catch_unwind(|| RedditComment::extract(c)) {
                    Ok(c) => Some(c),
                    Err(_) => {
                        error!("Failed to calculate comment {id}!");
                        None
                    }
                }
            })
            .collect::<Vec<_>>();
        let end = SystemTime::now();

        influxdb::log_time_consumed(influx_client, start, end, "extract_factorials").await?;

        let start = SystemTime::now();
        let comments = comments
            .into_iter()
            .filter_map(|c| {
                let id = c.id.clone();
                match std::panic::catch_unwind(|| RedditComment::calc(c)) {
                    Ok(c) => Some(c),
                    Err(_) => {
                        error!("Failed to calculate comment {id}!");
                        None
                    }
                }
            })
            .collect::<Vec<_>>();
        let end = SystemTime::now();

        influxdb::log_time_consumed(influx_client, start, end, "calculate_factorials").await?;

        write_comment_ids(&already_replied_or_rejected)?;

        let start = SystemTime::now();
        for comment in comments {
            let comment_id = comment.id.clone();
            let comment_author = comment.author.clone();
            let comment_subreddit = comment.subreddit.clone();

            let status: Status = comment.status;
            let should_answer = status.factorials_found && status.not_replied;

            if status.no_factorial && !status.number_too_big_to_calculate {
                continue;
            }

            if status.factorials_found {
                info!("Comment -> {:?}", comment);
            }

            if should_answer {
                let Ok(reply): Result<String, _> = std::panic::catch_unwind(|| comment.get_reply())
                else {
                    error!("Failed to format comment!");
                    continue;
                };
                // Sleep to not spam comments too quickly
                let pause = if rate.1 < 1.0 {
                    rate.0 + 5.0
                } else if rate.1 < 4.0 {
                    rate.0 / rate.1 + 2.0
                } else {
                    2.0
                };
                sleep(Duration::from_secs(pause as u64)).await;
                if !dont_reply {
                    match reddit_client.reply_to_comment(comment, &reply).await {
                        Ok(t) => {
                            if let Some(t) = t {
                                rate = t;
                            } else {
                                warn!("Missing ratelimit");
                            }
                            influxdb::log_comment_reply(
                                influx_client,
                                &comment_id,
                                &comment_author,
                                &comment_subreddit,
                            )
                            .await?;
                        }
                        Err(e) => error!("Failed to reply to comment: {:?}", e),
                    }
                }
                continue;
            }
            info!(" -> unknown");
        }
        let end = SystemTime::now();

        influxdb::log_time_consumed(influx_client, start, end, "comment_loop").await?;

        let sleep_between_requests = if rate.1 < requests_per_loop + 1.0 {
            rate.0 + 1.0
        } else {
            (rate.0 / rate.1 * requests_per_loop).max(2.0) + 1.0
        };
        // Sleep to avoid hitting API rate limits
        sleep(Duration::from_secs(sleep_between_requests.ceil() as u64)).await;
    }
    Ok(())
}

fn init() {
    dotenv().ok();
    env_logger::builder()
        .format(|buf, record| {
            let style = buf.default_level_style(record.level());
            writeln!(
                buf,
                "{style}{} | {} | {} | {}",
                record.level(),
                record.target(),
                buf.timestamp(),
                record.args()
            )
        })
        .init();

    panic::set_hook(Box::new(|panic_info| {
        let location = panic_info
            .location()
            .map(|l| format!("{}:{}", l.file(), l.line()))
            .unwrap_or_else(|| "unknown location".to_string());

        let message = panic_info
            .payload()
            .downcast_ref::<&str>()
            .map(|s| s.to_string())
            .or_else(|| panic_info.payload().downcast_ref::<String>().cloned())
            .unwrap_or_else(|| format!("Unknown panic payload: {:?}", panic_info));

        error!("Thread panicked at {} with message: {}", location, message);
    }));
}

fn write_comment_ids(already_replied_or_rejected: &[String]) -> Result<(), Box<dyn Error>> {
    let mut file = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(false)
        .open(COMMENT_IDS_FILE_PATH)
        .expect("Unable to open or create file");

    for comment_id in already_replied_or_rejected.iter() {
        writeln!(file, "{}", comment_id).expect("Unable to write to file");
    }
    Ok(())
}
