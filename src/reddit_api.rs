#![allow(deprecated)] // base64::encode is deprecated

use std::collections::HashMap;
use std::fmt::Write;
use std::sync::LazyLock;

use crate::reddit_comment::{Commands, RedditComment, Status};
use crate::{COMMENT_COUNT, SUBREDDITS, TERMIAL_SUBREDDITS};
use anyhow::{anyhow, Error};
use base64::engine::general_purpose::STANDARD_NO_PAD;
use base64::Engine;
use chrono::{DateTime, NaiveDateTime, Utc};
use reqwest::header::{HeaderMap, CONTENT_TYPE, USER_AGENT};
use reqwest::{Client, Response, Url};
use serde::Deserialize;
use serde_json::{from_str, json, Value};
use tokio::join;

#[derive(Deserialize, Debug)]
struct TokenResponse {
    access_token: String,
}

struct Token {
    access_token: String,
    expiration_time: DateTime<Utc>,
}

#[cfg(not(test))]
const REDDIT_OAUTH_URL: &str = "https://oauth.reddit.com";
#[cfg(test)]
const REDDIT_OAUTH_URL: &str = "http://127.0.0.1:9384";
#[cfg(not(test))]
const REDDIT_TOKEN_URL: &str = "https://ssl.reddit.com/api/v1/access_token";
#[cfg(test)]
const REDDIT_TOKEN_URL: &str = "http://127.0.0.1:9384";
#[cfg(not(test))]
const REDDIT_COMMENT_URL: &str = "https://oauth.reddit.com/api/comment";
#[cfg(test)]
const REDDIT_COMMENT_URL: &str = "http://127.0.0.1:9384";

pub(crate) struct RedditClient {
    client: Client,
    token: Token,
}

impl RedditClient {
    /// Creates a new client using the env variables APP_CLIENT_ID and APP_SECRET.
    /// # Panic
    /// Panics if the env vars are not set.
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

    /// Fetches comments from the `SUBREDDITS` and mentions with the set limit of `COMMENT_COUNT`, and creates/calculates the factorials from the response.
    /// And adds the comments to `already_replied_to_comments` to ignore them in the future.
    /// # Panic
    /// Panics if `SUBREDDITS` or `COMMENT_COUNT` is uninitialized, if the env vars APP_CLIENT_ID or APP_SECRET are unset, or if it recieves a malformed response from the api.
    pub(crate) async fn get_comments(
        &mut self,
        already_replied_to_comments: &mut Vec<String>,
        check_mentions: bool,
    ) -> Result<Vec<RedditComment>, ()> {
        static SUBREDDIT_URL: LazyLock<Url> = LazyLock::new(|| {
            Url::parse(&format!(
                "{}/r/{}/comments/?limit={}",
                REDDIT_OAUTH_URL,
                SUBREDDITS.get().expect("Subreddits uninitailized"),
                COMMENT_COUNT.get().expect("Comment count uninitialzed")
            ))
            .expect("Failed to parse Url")
        });
        static MENTION_URL: LazyLock<Url> = LazyLock::new(|| {
            Url::parse(&format!(
                "{}/message/mentions/?limit={}",
                REDDIT_OAUTH_URL,
                COMMENT_COUNT.get().expect("Comment count uninitialzed")
            ))
            .expect("Failed to parse Url")
        });
        #[cfg(not(test))]
        if self.is_token_expired() {
            println!("Token expired, getting new token");
            self.token = RedditClient::get_reddit_token(
                std::env::var("APP_CLIENT_ID").expect("APP_CLIENT_ID must be set."),
                std::env::var("APP_SECRET").expect("APP_SECRET must be set."),
            )
            .await
            .expect("Failed to get token");
        }

        let (subs_response, mentions_response) = if check_mentions {
            let (a, b) = join!(
                self.client
                    .get(SUBREDDIT_URL.clone())
                    .bearer_auth(&self.token.access_token)
                    .send(),
                self.client
                    .get(MENTION_URL.clone())
                    .bearer_auth(&self.token.access_token)
                    .send()
            );
            (a, Some(b))
        } else {
            let a = self
                .client
                .get(SUBREDDIT_URL.clone())
                .bearer_auth(&self.token.access_token)
                .send()
                .await;
            (a, None)
        };
        let subs_response = subs_response.expect("Failed to get comments");
        let mentions_response = mentions_response.map(|x| x.expect("Failed to get comments"));

        match RedditClient::check_response_status(&subs_response).and(
            mentions_response
                .as_ref()
                .map(RedditClient::check_response_status)
                .unwrap_or(Ok(())),
        ) {
            Ok(_) => {
                let (mentions, ids) = if let Some(mentions_response) = mentions_response {
                    let (a, b) = RedditClient::extract_comments(
                        mentions_response,
                        already_replied_to_comments,
                        true,
                        TERMIAL_SUBREDDITS.get().copied().unwrap_or_default(),
                        &HashMap::new(),
                    )
                    .await
                    .expect("Failed to extract comments");
                    (Some(a), Some(b))
                } else {
                    (None, None)
                };
                let (mut res, _) = RedditClient::extract_comments(
                    subs_response,
                    already_replied_to_comments,
                    false,
                    TERMIAL_SUBREDDITS.get().copied().unwrap_or_default(),
                    &HashMap::new(),
                )
                .await
                .expect("Failed to extract comments");
                if let Some(ids) = ids {
                    let response = self
                        .client
                        .get(format!(
                            "{}/api/info?id={}",
                            REDDIT_OAUTH_URL,
                            ids.iter()
                                .map(|(id, _)| id)
                                .fold(String::new(), |mut a, e| {
                                    let _ = write!(a, "{e}");
                                    a
                                })
                        ))
                        .bearer_auth(&self.token.access_token)
                        .send()
                        .await
                        .expect("Failed to get comment");
                    if Self::check_response_status(&response).is_ok() {
                        let (comments, _) = Self::extract_comments(
                            response,
                            already_replied_to_comments,
                            true,
                            TERMIAL_SUBREDDITS.get().copied().unwrap_or_default(),
                            &ids.into_iter().collect(),
                        )
                        .await
                        .expect("Failed to extract comments");
                        res.extend(comments);
                    }
                }
                if let Some(mentions) = mentions {
                    res.extend(mentions);
                }
                Ok(res)
            }
            Err(_) => Err(()),
        }
    }
    #[allow(unused)]
    fn is_token_expired(&self) -> bool {
        let now = Utc::now();

        now > self.token.expiration_time
    }

    /// Replies to the given `comment` with the given `reply`.
    /// # Panic
    /// May panic on a malformed response is recieved from the api.
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
            ("scope", "read submit privatemessages"),
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

        println!(
            "Now: {:#?} | Expiration time: {:#?}",
            Utc::now(),
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

    fn extract_summon_parent_id(comment: &Value) -> Option<String> {
        let parent_id = comment["data"]["parent_id"].as_str()?.to_string();
        Some(parent_id)
    }
    async fn extract_comments(
        response: Response,
        already_replied_to_comments: &mut Vec<String>,
        is_mention: bool,
        termial_subreddits: &str,
        mention_map: &HashMap<String, (String, Commands)>,
    ) -> Result<(Vec<RedditComment>, Vec<(String, (String, Commands))>), Box<dyn std::error::Error>>
    {
        let empty_vec = Vec::new();
        let response_json = response.json::<Value>().await?;
        let comments_json = response_json["data"]["children"]
            .as_array()
            .unwrap_or(&empty_vec);

        already_replied_to_comments.reserve(comments_json.len());
        let mut comments = Vec::with_capacity(comments_json.len());
        let mut parent_paths = Vec::new();
        for comment in comments_json {
            let Some(extracted_comment) = Self::extract_comment(
                comment,
                already_replied_to_comments,
                is_mention,
                termial_subreddits,
                mention_map,
            ) else {
                continue;
            };
            if is_mention
                && extracted_comment.status.no_factorial
                && !extracted_comment.status.already_replied_or_rejected
            {
                if let Some(path) = Self::extract_summon_parent_id(comment) {
                    parent_paths.push((
                        path,
                        (extracted_comment.id.clone(), extracted_comment.commands),
                    ));
                }
            }
            comments.push(extracted_comment);
        }

        Ok((comments, parent_paths))
    }
    fn extract_comment(
        comment: &Value,
        already_replied_to_comments: &mut Vec<String>,
        do_termial: bool,
        termial_subreddits: &str,
        mention_map: &HashMap<String, (String, Commands)>,
    ) -> Option<RedditComment> {
        let comment_text = comment["data"]["body"].as_str().unwrap_or("");
        let author = comment["data"]["author"].as_str().unwrap_or("");
        let subreddit = comment["data"]["subreddit"].as_str().unwrap_or("");
        let comment_id = comment["data"]["id"].as_str().unwrap_or_default();

        if already_replied_to_comments.contains(&comment_id.to_string()) {
            Some(RedditComment::new_already_replied(
                comment_id, author, subreddit,
            ))
        } else {
            already_replied_to_comments.push(comment_id.to_string());
            let Ok(mut comment) = std::panic::catch_unwind(|| {
                RedditComment::new(
                    comment_text,
                    comment_id,
                    author,
                    subreddit,
                    do_termial || termial_subreddits.split('+').any(|sub| sub == subreddit),
                )
            }) else {
                println!("Failed to construct comment!");
                return None;
            };
            if let Some((mention, commands)) = mention_map.get(&format!("t1_{comment_id}")) {
                comment.id = mention.clone();
                comment.commands = *commands;
            }

            comment.add_status(Status::NOT_REPLIED);

            Some(comment)
        }
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use tokio::{
        io::{AsyncReadExt, AsyncWriteExt},
        net::TcpListener,
        time::timeout,
    };

    use super::*;

    async fn dummy_server(reqeuest_response_pairs: &[(&str, &str)]) -> std::io::Result<()> {
        let listen = TcpListener::bind("127.0.0.1:9384").await?;
        for (expected_request, response) in reqeuest_response_pairs {
            let mut sock = timeout(Duration::from_secs(5), listen.accept()).await??.0;
            let mut request = vec![0; 10000];
            let len = timeout(Duration::from_millis(300), sock.read(&mut request)).await??;
            request.truncate(len);
            let request = String::from_utf8(request).expect("Got invalid utf8");
            if !(&request == expected_request) {
                panic!(
                    "Wrong request: {:?}\nExpected: {:?}",
                    request, expected_request
                );
            }
            timeout(
                Duration::from_millis(50),
                sock.write_all(response.as_bytes()),
            )
            .await??;
            timeout(Duration::from_millis(300), sock.flush()).await??;
        }
        Ok(())
    }
    pub static SEQUENTIAL_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
    fn sequential<'a>() -> std::sync::MutexGuard<'a, ()> {
        loop {
            SEQUENTIAL_LOCK.clear_poison();
            match SEQUENTIAL_LOCK.lock() {
                Ok(lock) => return lock,
                Err(_) => {}
            }
        }
    }

    #[tokio::test]
    async fn test_new_client() {
        let _lock = sequential();
        // SAFETY: All envvar operations are tested Sequentially
        unsafe {
            std::env::set_var("APP_CLIENT_ID", "an id");
            std::env::set_var("APP_SECRET", "a secret");
            std::env::set_var("REDDIT_PASSWORD", "a password");
            std::env::set_var("REDDIT_USERNAME", "a username");
        }

        let request = format!(
            "POST / HTTP/1.1\r\nuser-agent: factorion-bot:v{} (by /u/tolik518)\r\ncontent-type: application/x-www-form-urlencoded\r\nauthorization: Basic YW4gaWQ6YSBzZWNyZXQ=\r\naccept: */*\r\nhost: 127.0.0.1:9384\r\ncontent-length: 93\r\n\r\ngrant_type=password&username=a+username&password=a+password&scope=read+submit+privatemessages",
            env!("CARGO_PKG_VERSION")
        );

        let req_resp = [(
            request.as_str(),
            "HTTP/1.1 200 OK\n\n{\"access_token\": \"eyJhbGciOiJSUzI1NiIsImtpZCI6IlNIQTI1NjpzS3dsMnlsV0VtMjVmcXhwTU40cWY4MXE2OWFFdWFyMnpLMUdhVGxjdWNZIiwidHlwIjoiSldUIn0.eyJzdWIiOiJ1c2dyIiwiZXhwIjoxNzM1MTQ0NjI0LjQ2OTAyLCJpYXQiOjE3MzUwNTgyMjQuNDY5MDIsImp0aSI6IlpDM0Y2YzVXUGh1a09zVDRCcExaa0lmam1USjBSZyIsImNpZCI6IklJbTJha1RaRDFHWXd5Y1lXTlBKWVEiLCJsaWQiOiJ0dl96bnJ5dTJvM1QiLCJhaWQiOiJ0Ml96bnJ5dT1vMjQiLCJsY2EiOjE3MTQ4MjU0NzQ3MDIsInNjcCI6ImVKeUtWaXBLVFV4UjBsRXFMazNLelN4UmlnVUVBQUpfX3pGR0JaMCIsImZsbyI6OX0.o3X9CJAUED1iYsFs8h_02NvaDMmPVSIaZgz3aPjEGm3zF5cG2-G2tU7yIJUtqGICxT0W3-PAso0jwrrx3ScSGucvhEiUVXOiGcCZSzPfLnwuGxtRa_lNEkrsLAVlhN8iXBRGds8YkJ0MFWn4JRwhi8beV3EsFkEzN6IsESuA33WUQQgGs0Ij5oH0If3EMLoBoDVQvWdp2Yno0SV9xdODP6pMJSKZD5HVgWGzprFlN2VWmgb4HXs3mrxbE5bcuO_slah0xcqnhcXmlYCdRCSqeEUtlW8pS4Wtzzs7BL5E70A5LHmHJfGJWCh-loInwarxeq_tVPoxikzqBrTIEsLmPA\"}"
        )];

        let (status, client) = join!(dummy_server(&req_resp), RedditClient::new());
        status.unwrap();
        client.unwrap();
    }

    #[tokio::test]
    async fn test_reply_to_comment() {
        let _lock = sequential();
        let client = RedditClient {
            client: Client::new(),
            token: Token {
                access_token: "token".to_string(),
                expiration_time: Utc::now(),
            },
        };
        let (status, reply_status) = join!(
            dummy_server(&[(
                "POST / HTTP/1.1\r\nauthorization: Bearer token\r\ncontent-type: application/x-www-form-urlencoded\r\naccept: */*\r\nhost: 127.0.0.1:9384\r\ncontent-length: 32\r\n\r\ntext=I+relpy&thing_id=t1_some_id",
                "HTTP/1.1 200 OK\n\n{\"success\": true}"
            )]),
            client.reply_to_comment(RedditComment::new_already_replied("some_id", "author", "subressit"), "I relpy")
        );
        status.unwrap();
        reply_status.unwrap();
    }

    #[tokio::test]
    async fn test_get_comments() {
        let _lock = sequential();
        let mut client = RedditClient {
            client: Client::new(),
            token: Token {
                access_token: "token".to_string(),
                expiration_time: Utc::now(),
            },
        };
        let _ = SUBREDDITS.set("test_subreddit");
        let _ = COMMENT_COUNT.set(100);
        let mut already_replied = vec![];
        let (status, comments) = join!(
            async {
                dummy_server(&[(
                    "GET /r/test_subreddit/comments/?limit=100 HTTP/1.1\r\nauthorization: Bearer token\r\naccept: */*\r\nhost: 127.0.0.1:9384\r\n\r\n",
                    "HTTP/1.1 200 OK\n\n{\"data\":{\"children\":[]}}"
                ),(
                    "GET /message/mentions/?limit=100 HTTP/1.1\r\nauthorization: Bearer token\r\naccept: */*\r\nhost: 127.0.0.1:9384\r\n\r\n",
                    "HTTP/1.1 200 OK\n\n{\"data\":{\"children\":[{\"kind\": \"t1\",\"data\":{\"body\":\"u/factorion-bot\",\"parent_id\":\"t1_m38msum\"}}]}}"
                ),(
                    "GET /api/info?id=t1_m38msum HTTP/1.1\r\nauthorization: Bearer token\r\naccept: */*\r\nhost: 127.0.0.1:9384\r\n\r\n",
                    "HTTP/1.1 200 OK\n\n{\"data\": {\"children\": [{\"data\":{\"id\":\"m38msum\", \"body\":\"That's 57!?\"}}]}}"
                )]).await
            },
            client.get_comments(&mut already_replied, true)
        );
        status.unwrap();
        let comments = comments.unwrap();
        assert_eq!(comments.len(), 2);
        assert_eq!(comments[0].calculation_list[0].steps, [(1, 0), (0, 0)]);
    }

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
                               "id": "m38msug",
                               "locked": false,
                              "unrepliable_reason": null
                           }
                       },
                       {
                           "kind": "t1",
                           "data": {
                               "author": "Little_Tweetybird_",
                               "author_fullname": "t2_b5n60qnt",
                               "body": "u/factorion-bot !termial",
                               "body_html": "&lt;div class=\"md\"&gt;&lt;p&gt;u/factorion-bot&lt;/p&gt;\n&lt;/div&gt;",
                               "id": "m38msun",
                               "parent_id": "t1_m38msum",
                               "context": "/r/some_sub/8msu32a/some_post/m38msun/?context=3"
                           }
                       }
                   ]
               }
           }"#).unwrap());
        let mut already_replied = vec![];
        let comments = RedditClient::extract_comments(
            response,
            &mut already_replied,
            true,
            "",
            &HashMap::new(),
        )
        .await
        .unwrap();
        assert_eq!(comments.0.len(), 3);
        assert_eq!(
            comments.1,
            [(
                "t1_m38msum".to_string(),
                (
                    "m38msun".to_string(),
                    Commands {
                        termial: true,
                        ..Default::default()
                    }
                )
            )]
        );
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
