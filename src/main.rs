use num_bigint::{BigInt, ToBigInt};
use num_traits::{One, Zero};
use regex::Regex;
use reqwest::{Client, header};
use serde_json::json;
use tokio;
use dotenv::dotenv;

use reqwest::header::{HeaderMap, AUTHORIZATION, USER_AGENT, CONTENT_TYPE};
use serde::Deserialize;

#[derive(Deserialize, Debug)]
struct TokenResponse {
    access_token: String,
    // Other fields can be added here if necessary
}

async fn get_reddit_token(client_id: &str, client_secret: &str) -> Result<String, Box<dyn std::error::Error>> {
    let password = std::env::var("REDDIT_PASSWORD").expect("REDDIT_PASSWORD must be set.");
    let username = std::env::var("REDDIT_USERNAME").expect("REDDIT_USERNAME must be set.");
    let client = Client::new();
    let auth_value = format!("Basic {}", base64::encode(format!("{}:{}", client_id, client_secret)));

    let mut headers = HeaderMap::new();
    headers.insert(AUTHORIZATION, auth_value.parse()?);
    headers.insert(USER_AGENT, "factorion-bot:v0.0.1 (by /u/tolik518)".parse()?);
    headers.insert(CONTENT_TYPE, "application/x-www-form-urlencoded".parse()?);

    let params = [("grant_type", "password"), ("username", username.as_str()), ("password", password.as_str())];
    println!("Params: {:#?}", params);
    let response = client.post("https://www.reddit.com/api/v1/access_token")
        .headers(headers)
        .form(&params)
        .send()
        .await?;

    if !response.status().is_success() {
        println!("Failed to get token: {:#?}", response);
        return Err("Failed to get token".into());
    }

    let response = response.json::<TokenResponse>().await?;
    Ok(response.access_token)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv().ok(); // This line loads the environment variables from the ".env" file.
    let client_id = std::env::var("APP_CLIENT_ID").expect("APP_CLIENT_ID must be set.");
    let secret = std::env::var("APP_SECRET").expect("APP_SECRET must be set.");
    let token = get_reddit_token(client_id.as_str(), secret.as_str()).await?;
    let user_agent = "factorion-bot:v0.0.1 (by /u/tolik518)";

    // Set the header with your credentials
    let mut headers = HeaderMap::new();
    headers.insert("Authorization", header::HeaderValue::from_str(&format!("bearer {}", token))?);
    headers.insert("User-Agent", header::HeaderValue::from_str(user_agent)?);
    let client = Client::builder().default_headers(headers).build()?;

    // Regex to find factorial numbers
    let re = Regex::new(r"\b(\d+)\!\B").unwrap();

    // Define a reasonable upper limit for factorial computation
    let upper_limit = 100000;

    // Polling Reddit for new comments
    loop {
        println!("Polling Reddit for new comments...");
        let response = client.get("https://oauth.reddit.com/r/Qazaqstan/comments/?limit=10")
            .send()
            .await?;

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
                    let num = "99999".parse::<i64>().unwrap();

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
                        reply_to_comment(&client, &comment, &reply).await?;
                    }
                }
            }
        }

        // Sleep to avoid hitting API rate limits
        tokio::time::sleep(tokio::time::Duration::from_secs(60)).await;
    }
}

async fn reply_to_comment(client: &Client, comment: &serde_json::Value, reply: &str) -> Result<(), reqwest::Error> {
    let comment_id = comment["data"]["id"].as_str().unwrap();
    println!("Replying to comment {}", comment_id);
    let params = json!({ "thing_id": format!("t1_{}", comment_id), "text": reply });
    println!("Response client: {:#?}", client);
    let response = client.post("https://oauth.reddit.com/api/comment")
        .json(&params)
        .send()
        .await?;
    println!("Reply status: {:#?}", response.text().await?);
    Ok(())
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
