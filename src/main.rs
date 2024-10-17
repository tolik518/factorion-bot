use std::ops::{Mul, Shl};
use num_bigint::{BigInt, ToBigInt};
use num_traits::{One, Zero};
use regex::Regex;
use tokio;

use reddit_api::RedditClient;

mod reddit_api;

const REDDIT_SUBREDDIT: &str = "test";
const UPPER_LIMIT: i64 = 100_001;
const FOOTER_TEXT: &str = "*^(I am a bot, called factorion, and this action was performed automatically. Please contact u/tolik518 if you have any questions or concerns or just visit me on github https://github.com/tolik518/factorion-bot/)*";
const API_COMMENT_COUNT: u32 = 2;
const SLEEP_DURATION: u64 = 60;
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let reddit_client = RedditClient::new().await?;

    // Regex to find factorial numbers
    let regex = Regex::new(r"\b(\d+)!\B").unwrap();

    // Polling Reddit for new comments
    loop {
        println!("Polling Reddit for new comments...");
        let response = reddit_client.get_comments(REDDIT_SUBREDDIT, API_COMMENT_COUNT).await.unwrap();

        println!("Statuscode: {:#?}", response.status());
        if let Some(www_authenticate) = response.headers().get("www-authenticate") {
            match www_authenticate.to_str() {
                Ok(value) => println!("www-authenticate: {}", value),
                Err(_) => println!("Failed to convert www-authenticate header value to string"),
            }
        }

        if !response.status().is_success() {
            println!("Failed to get comments: {:#?}", response);
            continue;
        }

        let response = response.json::<serde_json::Value>().await?;

        //println!("Response: {:#?}", response);

        if let Some(comments) = response["data"]["children"].as_array() {
            println!("Found {} comments", comments.len());
            for comment in comments {
                //println!("Comment: {:#?}", comment);
                let body = comment["data"]["body"].as_str().unwrap_or("");
                println!("\x1b[90m======================================================================\x1b[0m");
                println!("\x1b[90mComment: {}\x1b[0m", body);

                // create a bigInt list
                let mut factorial_list = Vec::new();

                let comment_id = comment["data"]["id"].as_str().unwrap();
                let full_comment_id = format!("t1_{}", comment_id); // Prepend "t1_" to the comment_id
                //TODO: Remove this debug print
                //TODO: Get children of comment
                println!("Comment ID: {}", full_comment_id);
                println!("Data: {:#?}", reddit_client.get_comment_children(&*full_comment_id, API_COMMENT_COUNT)
                    .await.unwrap()
                    .json::<serde_json::Value>()
                    .await.unwrap());

                for regex_capture in regex.captures_iter(body)
                {
                    let num = regex_capture[1].parse::<i64>().unwrap();

                    // Check if the number is within a reasonable range to compute
                    if num > UPPER_LIMIT {
                        println!("## The factorial of {} is too large for me to compute safely.", num);
                    } else {
                        // check if the comment is already replied to by the bot
                        // if yes, skip the comment
                        //TODO: doesn't work like this. store the comment id in a file(?) and check if the comment id is already replied to
                        if let Some(replies) = comment["data"]["replies"]["data"]["children"].as_array() {
                            let mut already_replied = false;
                            for reply in replies {
                                if reply["data"]["author"].as_str() == Some("factorion-bot") {
                                    already_replied = true;
                                    break;
                                }
                            }
                            if already_replied {
                                println!("## Already replied to this comment");
                                continue;
                            }
                        }

                        let factorial = factorial(num);
                        factorial_list.push((num, factorial.clone()));
                    }
                }

                if !factorial_list.is_empty() {
                    let mut reply: String = "".to_owned();
                    for (num, factorial) in factorial_list {
                        reply = format!("{reply}The factorial of {num} is {factorial}.\n");
                    }
                    reply = format!("{reply}\n{FOOTER_TEXT}");
                    println!("Would have replied:\n{}", reply);
                    //reddit_client.reply_to_comment(&comment, &reply).await?;
                }
            }
        }

        // Sleep to avoid hitting API rate limits
        tokio::time::sleep(tokio::time::Duration::from_secs(SLEEP_DURATION)).await;
    }
}

fn factorial(n: i64) -> BigInt {
    if n < 2 {
        return One::one();
    }
    factorial_recursive(1, n)
}

fn factorial_recursive(low: i64, high: i64) -> BigInt {
    if low > high {
        One::one()
    } else if low == high {
        BigInt::from(low)
    } else if high - low == 1 {
        BigInt::from(low) * BigInt::from(high)
    } else {
        let mid = (low + high) / 2;
        let left = factorial_recursive(low, mid);
        let right = factorial_recursive(mid + 1, high);
        left * right
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_factorial() {
        assert_eq!(factorial(00), 1.to_bigint().unwrap());
        assert_eq!(factorial(01), 1.to_bigint().unwrap());
        assert_eq!(factorial(02), 2.to_bigint().unwrap());
        assert_eq!(factorial(03), 6.to_bigint().unwrap());
        assert_eq!(factorial(04), 24.to_bigint().unwrap());
        assert_eq!(factorial(05), 120.to_bigint().unwrap());
        assert_eq!(factorial(06), 720.to_bigint().unwrap());
        assert_eq!(factorial(07), 5040.to_bigint().unwrap());
        assert_eq!(factorial(08), 40320.to_bigint().unwrap());
        assert_eq!(factorial(09), 362880.to_bigint().unwrap());
        assert_eq!(factorial(10), 3628800.to_bigint().unwrap());
    }

    #[test]
    fn test_calculate_factorials_with_interesting_lengths(){
        let result = factorial(22);
        assert_eq!(22, result.to_string().len(), "{}", result);

        let result = factorial(23);
        assert_eq!(23, result.to_string().len(), "{}", result);

        let result = factorial(24);
        assert_eq!(24, result.to_string().len(), "{}", result);

        let result = factorial(82);
        assert_eq!(123, result.to_string().len(), "{}", result);

        let result = factorial(3909);
        assert_eq!(12346, result.to_string().len(), "{}", result);

        let result = factorial(574);
        assert_eq!(1337, result.to_string().len(), "{}", result);
    }

    #[test]
    fn test_calculate_factorial_hundred_thousand() {
        let num = 100_001;
        let result = factorial(num);
        assert_eq!(result.to_string().len(), 456579);
    }
}