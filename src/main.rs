use reddit_api::RedditClient;
use reddit_comment::Status;
use std::collections::HashSet;
use std::error::Error;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::time::SystemTime;
use time::OffsetDateTime;
use tokio::time::{sleep, Duration};

mod math;
mod reddit_api;
pub(crate) mod reddit_comment;

const API_COMMENT_COUNT: u32 = 100;
const COMMENT_IDS_FILE_PATH: &str = "comment_ids.txt";

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let mut reddit_client = RedditClient::new().await?;
    let subreddits = std::env::var("SUBREDDITS").expect("SUBREDDITS must be set.");
    let subreddits = subreddits.as_str();

    let sleep_between_requests = std::env::var("SLEEP_BETWEEN_REQUESTS").expect("SUBREDDITS must be set.");
    let sleep_between_requests = sleep_between_requests.as_str().parse().unwrap();

    // read comment_ids from the file
    let already_replied_to_comments: String =
        fs::read_to_string(COMMENT_IDS_FILE_PATH).unwrap_or("".to_string());

    if already_replied_to_comments.is_empty() {
        println!("No comment_ids found in the file");
    } else {
        println!("Found comment_ids in the file");
    }

    let mut already_replied_to_comments: Vec<String> = already_replied_to_comments
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

        let comments = reddit_client
            .get_comments(subreddits, API_COMMENT_COUNT, &already_replied_to_comments)
            .await
            .unwrap_or_default();

        println!("Found {} comments", comments.len());

        for comment in comments {
            let comment_id = comment.id.clone();
            let status_set: HashSet<_> = comment.status.iter().cloned().collect();
            let should_answer = status_set.contains(&Status::FactorialsFound)
                && status_set.contains(&Status::NotReplied);

            if status_set.contains(&Status::NoFactorial) {
                continue;
            }

            print!("Comment ID {} -> {:?}", comment.id, comment.status);

            if status_set.contains(&Status::NumberTooBig) {
                println!(" -> {:?}", comment.factorial_list);
                continue;
            }

            if status_set.contains(&Status::AlreadyReplied) {
                println!(" [already replied] ");
                continue;
            }
            if status_set.contains(&Status::FactorialsFound) {
                println!(" -> {:?}", comment.factorial_list);
            }
            if should_answer {
                let reply: String = comment.get_reply();
                already_replied_to_comments.push(comment_id.clone());
                reddit_client.reply_to_comment(comment, &reply).await?;
                // Sleep to not spam comments too quickly
                sleep(Duration::from_secs(2)).await;
                continue;
            }
            println!(" [unknown] ");
        }

        let mut file = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(false) // This will clear the file contents if it already exists
            .open(COMMENT_IDS_FILE_PATH)
            .expect("Unable to open or create file");

        for comment_id in already_replied_to_comments.iter() {
            writeln!(file, "{}", comment_id).expect("Unable to write to file");
        }

        // Sleep to avoid hitting API rate limits
        sleep(Duration::from_secs(sleep_between_requests)).await;
    }
}
