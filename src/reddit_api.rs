#![allow(deprecated)] // base64::encode is deprecated

use crate::reddit_comment::{RedditComment, Status, MAX_COMMENT_LENGTH};
use anyhow::{anyhow, Error};
use base64::engine::general_purpose::STANDARD_NO_PAD;
use base64::Engine;
use chrono::{DateTime, NaiveDateTime, Utc};
use reqwest::header::{HeaderMap, CONTENT_TYPE, USER_AGENT};
use reqwest::{Client, Response};
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
        let response_status_err = !RedditClient::is_success(response_text);

        if response_status_err {
            eprintln!(
                "Comment ID {} -> Status FAILED: {:#?}",
                comment.id,
                RedditClient::get_error_message(response_json)
            );
            return Err(anyhow!("Failed to reply to comment"));
        }

        println!(
            "Comment ID {} -> Status OK: {:#?}",
            comment.id,
            RedditClient::get_error_message(response_json)
        );

        Ok(())
    }

    fn get_error_message(response_json: Value) -> String {
        let default_error_message = &vec![json!([""])];
        let jquery: &Vec<Value> = response_json["jquery"]
            .as_array()
            .unwrap_or(default_error_message);

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
        if !response.status().is_success() {
            println!(
                "Failed to get comments. Statuscode: {:#?}. Response: {:#?}",
                response.status(),
                response
            );
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
            let author = comment["data"]["author"].as_str().unwrap_or("");
            let subreddit = comment["data"]["subreddit"].as_str().unwrap_or("");

            let comment_id = comment["data"]["id"].as_str().unwrap_or_default();

            let mut comment = RedditComment::new(
                body,
                comment_id,
                author,
                subreddit
            );

            // set some statuses
            if !comment.status.contains(&Status::ReplyWouldBeTooLong)
                && (comment.get_reply().len() as i64 > MAX_COMMENT_LENGTH)
            {
                comment.add_status(Status::ReplyWouldBeTooLong);
            }

            if already_replied_to_comments.contains(&comment_id.clone().to_string()) {
                comment.add_status(Status::AlreadyReplied);
            } else {
                comment.add_status(Status::NotReplied);
            }
            comments.push(comment);
        }

        Ok(comments)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_extract_comments() {
        let response = Response::from(http::Response::builder().status(200).body(r#"{
               "data": {
                   "children": [
                       {
                           "data": {
                               "author": "Little_Tweetybird_",
                               "author_fullname": "t2_b5n60qnt",
                               "body": "comment 1!!",
                               "body_html": "&lt;div class=\"md\"&gt;&lt;p&gt;comment 1!!&lt;/p&gt;\n&lt;/div&gt;",
                               "id": "m38msum",
                               "locked": false,
                               "unrepliable_reason": null
                           }
                       },
                       {
                           "data": {
                               "author": "Little_Tweetybird_",
                               "author_fullname": "t2_b5n60qnt",
                               "body": "comment 2",
                               "body_html": "&lt;div class=\"md\"&gt;&lt;p&gt;comment 2&lt;/p&gt;\n&lt;/div&gt;",
                               "id": "m38msun",
                               "locked": false,
                              "unrepliable_reason": null
                           }
                       }
                   ]
               }
           }"#).unwrap());
        let comments = RedditClient::extract_comments(response, &[]).await.unwrap();
        assert_eq!(comments.len(), 2);
        println!("{:#?}", comments);
    }

    #[test]
    fn test_check_response_status() {
        let response = Response::from(http::Response::builder().status(200).body("").unwrap());
        assert_eq!(RedditClient::check_response_status(&response), Ok(()));

        let response = Response::from(http::Response::builder().status(404).body("").unwrap());
        assert_eq!(RedditClient::check_response_status(&response), Err(()));
    }

    #[test]
    fn test_get_expiration_time_from_jwt() {
        let jwt = "eyJhbGciOiJSUzI1NiIsImtpZCI6IlNIQTI1NjpzS3dsMnlsV0VtMjVmcXhwTU40cWY4MXE2OWFFdWFyMnpLMUdhVGxjdWNZIiwidHlwIjoiSldUIn0.eyJzdWIiOiJ1c2dyIiwiZXhwIjoxNzM1MTQ0NjI0LjQ2OTAyLCJpYXQiOjE3MzUwNTgyMjQuNDY5MDIsImp0aSI6IlpDM0Y2YzVXUGh1a09zVDRCcExaa0lmam1USjBSZyIsImNpZCI6IklJbTJha1RaRDFHWXd5Y1lXTlBKWVEiLCJsaWQiOiJ0dl96bnJ5dTJvM1QiLCJhaWQiOiJ0Ml96bnJ5dT1vMjQiLCJsY2EiOjE3MTQ4MjU0NzQ3MDIsInNjcCI6ImVKeUtWaXBLVFV4UjBsRXFMazNLelN4UmlnVUVBQUpfX3pGR0JaMCIsImZsbyI6OX0.o3X9CJAUED1iYsFs8h_02NvaDMmPVSIaZgz3aPjEGm3zF5cG2-G2tU7yIJUtqGICxT0W3-PAso0jwrrx3ScSGucvhEiUVXOiGcCZSzPfLnwuGxtRa_lNEkrsLAVlhN8iXBRGds8YkJ0MFWn4JRwhi8beV3EsFkEzN6IsESuA33WUQQgGs0Ij5oH0If3EMLoBoDVQvWdp2Yno0SV9xdODP6pMJSKZD5HVgWGzprFlN2VWmgb4HXs3mrxbE5bcuO_slah0xcqnhcXmlYCdRCSqeEUtlW8pS4Wtzzs7BL5E70A5LHmHJfGJWCh-loInwarxeq_tVPoxikzqBrTIEsLmPA";

        let actual: DateTime<Utc> = RedditClient::get_expiration_time_from_jwt(jwt);
        let expected: DateTime<Utc> =
            DateTime::from_naive_utc_and_offset(NaiveDateTime::from_timestamp(1735144624, 0), Utc);
        assert_eq!(actual, expected);
    }
}
