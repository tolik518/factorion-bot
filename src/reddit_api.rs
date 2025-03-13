#![allow(deprecated)] // base64::encode is deprecated

use crate::reddit_comment::{RedditComment, Status};
use anyhow::{anyhow, Error};
use base64::engine::general_purpose::STANDARD_NO_PAD;
use base64::Engine;
use chrono::{DateTime, NaiveDateTime, Utc};
use reqwest::header::{HeaderMap, CONTENT_TYPE, USER_AGENT};
use reqwest::{Client, Response};
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
        already_replied_to_comments: &mut Vec<String>,
    ) -> Result<Vec<RedditComment>, ()> {
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

        let (subs_response, mentions_response) = join!(
            self.client
                .get(format!(
                    "{}/r/{}/comments/?limit={}",
                    REDDIT_OAUTH_URL, subreddit, limit
                ))
                .bearer_auth(&self.token.access_token)
                .send(),
            self.client
                .get(format!(
                    "{}/message/mentions/?limit={}",
                    REDDIT_OAUTH_URL, limit
                ))
                .bearer_auth(&self.token.access_token)
                .send()
        );
        let subs_response = subs_response.expect("Failed to get comments");
        let mentions_response = mentions_response.expect("Failed to get comments");

        match RedditClient::check_response_status(&subs_response)
            .and(RedditClient::check_response_status(&mentions_response))
        {
            Ok(_) => {
                let (mentions, paths) =
                    RedditClient::extract_comments(mentions_response, already_replied_to_comments)
                        .await
                        .expect("Failed to extract comments");
                let mut parents = Vec::new();
                for path in paths {
                    let response = self
                        .client
                        .get(format!("{}{}", REDDIT_OAUTH_URL, path))
                        .bearer_auth(&self.token.access_token)
                        .send()
                        .await
                        .expect("Failed to get comment");
                    let parent = RedditClient::extract_comment(
                        &response
                            .json::<Value>()
                            .await
                            .expect("Response isn't JSON")
                            .as_array_mut()
                            .expect("Malformed JSON")
                            .remove(0),
                        already_replied_to_comments,
                    );
                    parents.push(parent);
                }
                let (mut res, _) =
                    RedditClient::extract_comments(subs_response, already_replied_to_comments)
                        .await
                        .expect("Failed to extract comments");
                res.extend(mentions);
                res.extend(parents);
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

    fn extract_summon_parent_path(comment: &Value) -> Option<String> {
        if comment["data"]["body"].as_str() == Some("u/factorion-bot")
            && comment["kind"].as_str() == Some("t1")
        {
            let mut context = comment["data"]["context"].as_str().map(|s| s.to_string())?;
            context.truncate(context.rfind("/").unwrap_or(context.len()));
            context.truncate(context.rfind("/").unwrap_or(context.len()) + 1);
            let parent_id = comment["data"]["parent_id"].as_str().map(|s| &s[3..])?;
            context.push_str(parent_id);
            context.push('/');
            Some(context)
        } else {
            None
        }
    }
    async fn extract_comments(
        response: Response,
        already_replied_to_comments: &mut Vec<String>,
    ) -> Result<(Vec<RedditComment>, Vec<String>), Box<dyn std::error::Error>> {
        let response_json = response.json::<Value>().await?;
        let comments_json = response_json["data"]["children"]
            .as_array()
            .cloned()
            .unwrap_or_default();

        let mut comments = Vec::new();
        let mut parent_paths = Vec::new();
        for comment in comments_json {
            comments.push(Self::extract_comment(&comment, already_replied_to_comments));
            if let Some(path) = Self::extract_summon_parent_path(&comment) {
                parent_paths.push(path);
            }
        }

        Ok((comments, parent_paths))
    }
    fn extract_comment(
        comment: &Value,
        already_replied_to_comments: &mut Vec<String>,
    ) -> RedditComment {
        let comment_text = comment["data"]["body"].as_str().unwrap_or("");
        let author = comment["data"]["author"].as_str().unwrap_or("");
        let subreddit = comment["data"]["subreddit"].as_str().unwrap_or("");
        let comment_id = comment["data"]["id"].as_str().unwrap_or_default();

        if already_replied_to_comments.contains(&comment_id.to_string()) {
            RedditComment::new_already_replied(comment_id, author, subreddit)
        } else {
            already_replied_to_comments.push(comment_id.to_string());
            let mut comment = RedditComment::new(comment_text, comment_id, author, subreddit);

            comment.add_status(Status::NOT_REPLIED);

            comment
        }
    }
}

#[cfg(test)]
mod tests {
    use tokio::{
        io::{AsyncReadExt, AsyncWriteExt},
        net::TcpListener,
    };

    use super::*;

    async fn dummy_server(reqeuest_response_pairs: &[(&str, &str)]) -> std::io::Result<()> {
        let listen = TcpListener::bind("127.0.0.1:9384").await?;
        for (expected_request, response) in reqeuest_response_pairs {
            let mut sock = listen.accept().await?.0;
            let mut request = vec![0; 10000];
            let len = sock.read(&mut request).await?;
            request.truncate(len);
            let request = String::from_utf8(request).expect("Got invalid utf8");
            if !(&request == expected_request) {
                panic!(
                    "Wrong request: {:?}\nExpected: {:?}",
                    request, expected_request
                );
            }
            sock.write_all(response.as_bytes()).await?;
            sock.flush().await?;
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
        let (status, client) = join!(
            dummy_server(&[(
                "POST / HTTP/1.1\r\nuser-agent: factorion-bot:v1.4.0 (by /u/tolik518)\r\ncontent-type: application/x-www-form-urlencoded\r\nauthorization: Basic YW4gaWQ6YSBzZWNyZXQ=\r\naccept: */*\r\nhost: 127.0.0.1:9384\r\ncontent-length: 93\r\n\r\ngrant_type=password&username=a+username&password=a+password&scope=read+submit+privatemessages",
                "HTTP/1.1 200 OK\n\n{\"access_token\": \"eyJhbGciOiJSUzI1NiIsImtpZCI6IlNIQTI1NjpzS3dsMnlsV0VtMjVmcXhwTU40cWY4MXE2OWFFdWFyMnpLMUdhVGxjdWNZIiwidHlwIjoiSldUIn0.eyJzdWIiOiJ1c2dyIiwiZXhwIjoxNzM1MTQ0NjI0LjQ2OTAyLCJpYXQiOjE3MzUwNTgyMjQuNDY5MDIsImp0aSI6IlpDM0Y2YzVXUGh1a09zVDRCcExaa0lmam1USjBSZyIsImNpZCI6IklJbTJha1RaRDFHWXd5Y1lXTlBKWVEiLCJsaWQiOiJ0dl96bnJ5dTJvM1QiLCJhaWQiOiJ0Ml96bnJ5dT1vMjQiLCJsY2EiOjE3MTQ4MjU0NzQ3MDIsInNjcCI6ImVKeUtWaXBLVFV4UjBsRXFMazNLelN4UmlnVUVBQUpfX3pGR0JaMCIsImZsbyI6OX0.o3X9CJAUED1iYsFs8h_02NvaDMmPVSIaZgz3aPjEGm3zF5cG2-G2tU7yIJUtqGICxT0W3-PAso0jwrrx3ScSGucvhEiUVXOiGcCZSzPfLnwuGxtRa_lNEkrsLAVlhN8iXBRGds8YkJ0MFWn4JRwhi8beV3EsFkEzN6IsESuA33WUQQgGs0Ij5oH0If3EMLoBoDVQvWdp2Yno0SV9xdODP6pMJSKZD5HVgWGzprFlN2VWmgb4HXs3mrxbE5bcuO_slah0xcqnhcXmlYCdRCSqeEUtlW8pS4Wtzzs7BL5E70A5LHmHJfGJWCh-loInwarxeq_tVPoxikzqBrTIEsLmPA\"}"
            )]),
            RedditClient::new()
        );
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
        let mut already_replied = vec![];
        let (status, comments) = join!(
            async {
                dummy_server(&[(
                    "GET /r/test_subreddit/comments/?limit=100 HTTP/1.1\r\nauthorization: Bearer token\r\naccept: */*\r\nhost: 127.0.0.1:9384\r\n\r\n",
                    "HTTP/1.1 200 OK\n\n{\"data\":{\"children\":[]}}"
                ),(
                    "GET /message/mentions/?limit=100 HTTP/1.1\r\nauthorization: Bearer token\r\naccept: */*\r\nhost: 127.0.0.1:9384\r\n\r\n",
                    "HTTP/1.1 200 OK\n\n{\"data\":{\"children\":[{\"kind\": \"t1\",\"data\":{\"body\":\"u/factorion-bot\",\"parent_id\":\"t1_m38msum\",\"context\":\"/r/some_sub/8msu32a/some_post/m38msun/?context=3\"}}]}}"
                ),(
                    "GET /r/some_sub/8msu32a/some_post/m38msum/ HTTP/1.1\r\nauthorization: Bearer token\r\naccept: */*\r\nhost: 127.0.0.1:9384\r\n\r\n",
                    "HTTP/1.1 200 OK\n\n[{\"data\":{\"id\":\"m38msum\", \"body\":\"That's 57!\"}}]"
                )]).await
            },
            client.get_comments("test_subreddit", 100, &mut already_replied)
        );
        status.unwrap();
        let comments = comments.unwrap();
        assert_eq!(comments.len(), 2);
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
                               "id": "m38msun",
                               "locked": false,
                              "unrepliable_reason": null
                           }
                       },
                       {
                           "kind": "t1",
                           "data": {
                               "author": "Little_Tweetybird_",
                               "author_fullname": "t2_b5n60qnt",
                               "body": "u/factorion-bot",
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
        let comments = RedditClient::extract_comments(response, &mut already_replied)
            .await
            .unwrap();
        assert_eq!(comments.0.len(), 3);
        assert_eq!(comments.1, ["/r/some_sub/8msu32a/some_post/m38msum/"]);
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
