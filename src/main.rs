use core::panic;
use dotenvy::dotenv;
use influxdb::INFLUX_CLIENT;
use reddit_api::RedditClient;
use reddit_comment::{Commands, Status};
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

    let subreddit_commands = std::env::var("SUBREDDITS").unwrap_or_default();
    let subreddit_commands = subreddit_commands.leak();
    let commands = subreddit_commands
        .split('+')
        .map(|s| s.split_once(':').unwrap_or_default())
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
    SUBREDDIT_COMMANDS.set(commands).unwrap();

    let sleep_between_requests =
        std::env::var("SLEEP_BETWEEN_REQUESTS").expect("SLEEP_BETWEEN_REQUESTS must be set.");
    let sleep_between_requests = sleep_between_requests.as_str().parse().unwrap();

    let check_mentions = std::env::var("CHECK_MENTIONS").expect("CHECK_MENTIONS must be set");
    let check_mentions = check_mentions == "true";

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

    // Polling Reddit for new comments
    loop {
        let today: OffsetDateTime = SystemTime::now().into();
        println!(
            "{} - {} | Polling Reddit for new comments...",
            today.date(),
            today.time()
        );

        let start = SystemTime::now();
        let comments = reddit_client
            .get_comments(&mut already_replied_or_rejected, check_mentions)
            .await
            .unwrap_or_default();
        let end = SystemTime::now();

        influxdb::log_time_consumed(influx_client, start, end, "get_comments").await?;

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
                let reply: String = comment.get_reply();
                match reddit_client.reply_to_comment(comment, &reply).await {
                    Ok(_) => {
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
                // Sleep to not spam comments too quickly
                sleep(Duration::from_secs(2)).await;
                continue;
            }
            println!(" -> unknown");
        }
        let end = SystemTime::now();

        influxdb::log_time_consumed(influx_client, start, end, "comment_loop").await?;

        let mut file = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(false) // This will clear the file contents if it already exists
            .open(COMMENT_IDS_FILE_PATH)
            .expect("Unable to open or create file");

        for comment_id in already_replied_or_rejected.iter() {
            writeln!(file, "{}", comment_id).expect("Unable to write to file");
        }

        // Sleep to avoid hitting API rate limits
        sleep(Duration::from_secs(sleep_between_requests)).await;
    }
}
