use core::panic;
use dotenvy::dotenv;
use influxdb::INFLUX_CLIENT;
use reddit_api::RedditClient;
use reddit_comment::{Commands, RedditComment, Status};
use std::collections::HashMap;
use std::error::Error;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::sync::OnceLock;
use std::time::SystemTime;
use time::OffsetDateTime;
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
    dotenv().ok();
    let influx_client = &*INFLUX_CLIENT;

    if influx_client.is_none() {
        eprintln!("InfluxDB client not configured. No influxdb metrics will be logged.");
    } else {
        println!("InfluxDB client configured. Metrics will be logged.");
    }

    let mut reddit_client = RedditClient::new().await?;
    COMMENT_COUNT.set(API_COMMENT_COUNT).unwrap();
    let mut requests_per_loop = 0.0;

    let subreddit_commands = std::env::var("SUBREDDITS").unwrap_or_default();
    let subreddit_commands = subreddit_commands.leak();
    let commands = subreddit_commands
        .split('+')
        .map(|s| s.split_once(':').unwrap_or((s, "")))
        .filter(|s| s.0 != "")
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
        println!("No comment_ids found in the file");
    } else {
        println!("Found comment_ids in the file");
    }

    let mut already_replied_or_rejected: Vec<String> = already_replied_to_comments
        .lines()
        .map(|s| s.to_string())
        .collect::<Vec<String>>();
    let mut last_ids = Default::default();

    // Polling Reddit for new comments
    for i in (0..u8::MAX).cycle() {
        let today: OffsetDateTime = SystemTime::now().into();
        println!(
            "{} - {} | Polling Reddit for new comments...",
            today.date(),
            today.time()
        );

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
                        println!("Failed to calculate comment {id}!");
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
                        println!("Failed to calculate comment {id}!");
                        None
                    }
                }
            })
            .collect::<Vec<_>>();
        let end = SystemTime::now();

        influxdb::log_time_consumed(influx_client, start, end, "calculate_factorials").await?;

        let mut file = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(false) // This will clear the file contents if it already exists
            .open(COMMENT_IDS_FILE_PATH)
            .expect("Unable to open or create file");

        for comment_id in already_replied_or_rejected.iter() {
            writeln!(file, "{}", comment_id).expect("Unable to write to file");
        }

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

            print!("Comment ID {} -> {:?}", comment.id, comment.status);

            if status.number_too_big_to_calculate {
                println!(" -> number too big to calculate");
                continue;
            }

            if status.already_replied_or_rejected {
                println!(" -> already replied or rejected");
                continue;
            }

            if status.factorials_found {
                println!(" -> {:?}", comment.calculation_list);
            }
            if should_answer {
                let Ok(reply): Result<String, _> = std::panic::catch_unwind(|| comment.get_reply())
                else {
                    println!("Failed to format comment!");
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
                match reddit_client.reply_to_comment(comment, &reply).await {
                    Ok(t) => {
                        rate = t;
                        influxdb::log_comment_reply(
                            influx_client,
                            &comment_id,
                            &comment_author,
                            &comment_subreddit,
                        )
                        .await?;
                    }
                    Err(e) => eprintln!("Failed to reply to comment: {:?}", e),
                }
                continue;
            }
            println!(" -> unknown");
        }
        let end = SystemTime::now();

        influxdb::log_time_consumed(influx_client, start, end, "comment_loop").await?;

        let sleep_between_requests = if rate.1 < requests_per_loop + 1.0 {
            rate.0 + 5.0
        } else if rate.1 < requests_per_loop * 4.0 {
            rate.0 / rate.1 + 2.0
        } else {
            ((rate.0 / rate.1) - 2.0).max(2.0)
        };
        // Sleep to avoid hitting API rate limits
        sleep(Duration::from_secs(sleep_between_requests as u64)).await;
    }
    Ok(())
}
