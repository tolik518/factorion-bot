use num_bigint::{BigInt, ToBigInt};
use num_traits::One;
use regex::Regex;
use tokio;

use reddit_api::RedditClient;

mod reddit_api;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let reddit_client = RedditClient::new().await?;

    // Regex to find factorial numbers
    let re = Regex::new(r"\b(\d+)!\B").unwrap();

    // Define a reasonable upper limit for factorial computation
    let upper_limit = 100000;

    // Polling Reddit for new comments
    loop {
        println!("Polling Reddit for new comments...");
        let response = reddit_client.get_comments("mathmemes", 10).await.unwrap();

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
                println!("\x1b[90m======================================================================\x1b[0");
                println!("\x1b[90mComment: {}\x1b[0m", body);
                for cap in re.captures_iter(body) {
                    let num = cap[1].parse::<i64>().unwrap();

                    // Check if the number is within a reasonable range to compute
                    if num > upper_limit {
                        println!("## The factorial of {} is too large for me to compute safely.", num);
                    } else {
                        // check if the comment is already replied to by the bot
                        // if yes, skip the comment
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

                        let factorial = calculate_factorial(&num.to_bigint().unwrap());
                        let reply = format!("The factorial of {} is {}.\n\n^I am a bot, called factorion, and this action was performed automatically. Please contact u/tolik518 of this subreddit if you have any questions or concerns.", num, factorial);
                        reddit_client.reply_to_comment(&comment, &reply).await?;
                    }
                }
            }
        }

        // Sleep to avoid hitting API rate limits
        tokio::time::sleep(tokio::time::Duration::from_secs(60)).await;
    }
}

fn calculate_factorial(n: &BigInt) -> BigInt {
    let mut result = One::one();
    println!("## Calculating factorial of {}", n);
    let mut i = One::one();
    let one = One::one();
    while i <= *n {
        result *= &i;
        i += &one;
    }
    println!("## Result: {}", result);
    result
}
