#![allow(deprecated)] // base64::encode is deprecated

use crate::reddit_comment::{RedditComment, Status, MAX_COMMENT_LENGTH};
use base64::engine::general_purpose::STANDARD_NO_PAD;
use base64::Engine;
use chrono::{DateTime, NaiveDateTime, Utc};
use dotenv::dotenv;
use reqwest::header::{HeaderMap, CONTENT_TYPE, USER_AGENT};
use reqwest::{Client, Error, Response};
use serde::Deserialize;
use serde_json::{from_str, json, Value};

#[derive(Deserialize, Debug)]
struct TokenResponse {
    access_token: String,
}

struct Token {
    access_token: String,
    expiration_time: DateTime<Utc>,
}

const REDDIT_TOKEN_URL: &str = "https://ssl.reddit.com/api/v1/access_token";
const REDDIT_COMMENT_URL: &str = "https://oauth.reddit.com/api/comment";

pub(crate) struct RedditClient {
    client: Client,
    token: Token,
}

impl RedditClient {
    pub(crate) async fn new() -> Result<Self, Box<dyn std::error::Error>> {
        dotenv().ok();
        let client_id = std::env::var("APP_CLIENT_ID").expect("APP_CLIENT_ID must be set.");
        let secret = std::env::var("APP_SECRET").expect("APP_SECRET must be set.");

        let token: Token = RedditClient::get_reddit_token(client_id, secret).await?;
        let user_agent = format!(
            "factorion-bot:v{} (by /u/tolik518)",
            env!("CARGO_PKG_VERSION")
        );

        let mut headers = HeaderMap::new();
        headers.insert(USER_AGENT, user_agent.parse()?);

        let client = Client::builder().default_headers(headers).build()?;

        Ok(Self { client, token })
    }

    pub(crate) async fn get_comments(
        &mut self,
        subreddit: &str,
        limit: u32,
        already_replied_to_comments: &[String],
    ) -> Result<Vec<RedditComment>, ()> {
        if self.is_token_expired() {
            println!("Token expired, getting new token");
            self.token = RedditClient::get_reddit_token(
                std::env::var("APP_CLIENT_ID").expect("APP_CLIENT_ID must be set."),
                std::env::var("APP_SECRET").expect("APP_SECRET must be set."),
            )
            .await
            .expect("Failed to get token");
        }

        let response = self
            .client
            .get(format!(
                "https://oauth.reddit.com/r/{}/comments/?limit={}",
                subreddit, limit
            ))
            .bearer_auth(&self.token.access_token)
            .send()
            .await
            .expect("Failed to get comments");

        match RedditClient::check_response_status(&response) {
            Ok(_) => Ok(
                RedditClient::extract_comments(response, already_replied_to_comments)
                    .await
                    .expect("Failed to extract comments"),
            ),
            Err(_) => Err(()),
        }
    }

    fn is_token_expired(&self) -> bool {
        let now = Utc::now();
        let expired = now > self.token.expiration_time;

        println!(
            "Now: {:#?} | Expiration time: {:#?}",
            now, self.token.expiration_time
        );
        println!("Token expired: {:#?}", expired);

        expired
    }

    pub(crate) async fn reply_to_comment(
        &self,
        comment: RedditComment,
        reply: &str,
    ) -> Result<(), Error> {
        let params = json!({
            "thing_id": format!("t1_{}", comment.id),
            "text": reply
        });

        let response = self
            .client
            .post(REDDIT_COMMENT_URL)
            .bearer_auth(&self.token.access_token)
            .form(&params)
            .send()
            .await?;

        let response_text = &response.text().await?;
        let response_text = response_text.as_str();
        let response_json =
            from_str::<Value>(response_text).expect("Failed to convert response to json");
        let response_status_ok = RedditClient::is_success(response_text);

        if response_status_ok {
            println!(
                "Comment ID {} -> Status OK: {:#?}",
                comment.id,
                RedditClient::get_error_message(response_json)
            );
        } else {
            println!(
                "Comment ID {} -> Status FAILED: {:#?}",
                comment.id,
                RedditClient::get_error_message(response_json)
            );
        }

        Ok(())
    }

    fn get_error_message(response_json: Value) -> String {
        let jquery: &Vec<Value> = response_json["jquery"]
            .as_array()
            .expect("Failed to get jquery array");

        // search for arrays which have array, which have a string value that's not empty
        let mut error_message = jquery
            .iter()
            .filter(|array| !array[2].as_str().unwrap_or("").is_empty())
            .map(|array| array[3][0].as_str().unwrap_or("").to_string())
            .collect::<Vec<String>>()
            .join(" ");

        error_message = error_message
            .split_whitespace()
            .collect::<Vec<&str>>()
            .join(" ");

        error_message
    }

    fn is_success(response_text: &str) -> bool {
        let response_json =
            from_str::<Value>(response_text).expect("Failed to convert response to json");

        response_json["success"].as_bool().unwrap_or(false)
    }

    async fn get_reddit_token(
        client_id: String,
        client_secret: String,
    ) -> Result<Token, Box<dyn std::error::Error>> {
        let password = std::env::var("REDDIT_PASSWORD").expect("REDDIT_PASSWORD must be set.");
        let username = std::env::var("REDDIT_USERNAME").expect("REDDIT_USERNAME must be set.");

        let version = env!("CARGO_PKG_VERSION");
        let user_agent = format!("factorion-bot:v{version} (by /u/tolik518)");

        let mut headers = HeaderMap::new();
        headers.insert(USER_AGENT, user_agent.parse()?);
        headers.insert(CONTENT_TYPE, "application/x-www-form-urlencoded".parse()?);

        let params = [
            ("grant_type", "password"),
            ("username", username.as_str()),
            ("password", password.as_str()),
            ("scope", "read submit"),
        ];

        let response = Client::new()
            .post(REDDIT_TOKEN_URL)
            .headers(headers)
            .basic_auth(client_id, Some(client_secret))
            .form(&params)
            .send()
            .await?;

        if !response.status().is_success() {
            println!("Failed to get token: {:#?}", response);
            return Err("Failed to get token".into());
        }

        let response = response.json::<TokenResponse>().await?;

        let token_expiration_time = Self::get_expiration_time_from_jwt(&response.access_token);

        println!(
            "Fetched new token. Will expire: {:#?}",
            token_expiration_time
        );

        Ok(Token {
            access_token: response.access_token,
            expiration_time: token_expiration_time,
        })
    }

    fn get_expiration_time_from_jwt(jwt: &str) -> DateTime<Utc> {
        let jwt = jwt.split('.').collect::<Vec<&str>>();
        let jwt_payload = jwt[1];
        let jwt_payload = STANDARD_NO_PAD
            .decode(jwt_payload.as_bytes())
            .expect("Failed to decode jwt payload");

        let jwt_payload =
            String::from_utf8(jwt_payload).expect("Failed to convert jwt payload to string");

        let jwt_payload =
            from_str::<Value>(&jwt_payload).expect("Failed to convert jwt payload to json");

        let exp = jwt_payload["exp"]
            .as_f64()
            .expect("Failed to get exp field");
        let naive = NaiveDateTime::from_timestamp(exp as i64, 0);
        let datetime: DateTime<Utc> = DateTime::from_utc(naive, Utc);

        datetime
    }

    fn check_response_status(response: &Response) -> Result<(), ()> {
        println!("Statuscode: {:#?}", response.status());
        if let Some(www_authenticate) = response.headers().get("www-authenticate") {
            match www_authenticate.to_str() {
                Ok(value) => println!("www-authenticate: {}", value),
                Err(_) => {
                    println!("Failed to convert www-authenticate header value to string");
                    return Err(());
                }
            }
        }

        if !response.status().is_success() {
            println!("Failed to get comments: {:#?}", response);
            return Err(());
        }

        Ok(())
    }

    async fn extract_comments(
        response: Response,
        already_replied_to_comments: &[String],
    ) -> Result<Vec<RedditComment>, Box<dyn std::error::Error>> {
        let response_json = response.json::<Value>().await?;
        let comments_json = response_json["data"]["children"]
            .as_array()
            .cloned()
            .unwrap_or_default();

        let mut comments = Vec::new();
        for comment in comments_json {
            let body = comment["data"]["body"].as_str().unwrap_or("");

            let comment_id = comment["data"]["id"]
                .as_str()
                .unwrap_or_default()
                .to_string();

            let mut comment = RedditComment::new(body, &comment_id);

            // set some statuses
            if !comment.status.contains(&Status::ReplyWouldBeTooLong)
                && (comment.get_reply().len() as i64 > MAX_COMMENT_LENGTH)
            {
                comment.add_status(Status::ReplyWouldBeTooLong);
            }

            if already_replied_to_comments.contains(&comment_id) {
                comment.add_status(Status::AlreadyReplied);
            } else {
                comment.add_status(Status::NotReplied);
            }
            comments.push(comment);
        }

        Ok(comments)
    }
}
