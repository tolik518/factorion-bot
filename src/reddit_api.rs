#![allow(deprecated)] // base64::encode is deprecated

use std::collections::HashMap;
use std::fmt::Write;
use std::sync::LazyLock;

use crate::reddit_comment::{
    Commands, RedditComment, RedditCommentCalculated, RedditCommentConstructed, Status,
};
use crate::{COMMENT_COUNT, SUBREDDIT_COMMANDS};
use anyhow::{anyhow, Error};
use base64::engine::general_purpose::STANDARD_NO_PAD;
use base64::Engine;
use chrono::{DateTime, NaiveDateTime, Utc};
use futures::future::OptionFuture;
use log::{error, info, warn};
use reqwest::header::{HeaderMap, CONTENT_TYPE, USER_AGENT};
use reqwest::{Client, RequestBuilder, Response, Url};
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
        check_posts: bool,
        last_ids: &mut (String, String, String),
    ) -> Result<(Vec<RedditCommentConstructed>, (f64, f64)), ()> {
        static SUBREDDIT_URL: LazyLock<Option<Url>> = LazyLock::new(|| {
            let mut subreddits = SUBREDDIT_COMMANDS
                .get()
                .expect("Subreddit commands uninitialized")
                .iter()
                .filter(|(_, commands)| !commands.post_only)
                .map(|(sub, _)| sub.to_string())
                .collect::<Vec<_>>();
            subreddits.sort();
            if !subreddits.is_empty() {
                Some(
                    Url::parse(&format!(
                        "{}/r/{}/comments",
                        REDDIT_OAUTH_URL,
                        subreddits
                            .into_iter()
                            .reduce(|a, e| format!("{a}+{e}"))
                            .unwrap_or_default(),
                    ))
                    .expect("Failed to parse Url"),
                )
            } else {
                None
            }
        });
        static SUBREDDIT_POSTS_URL: LazyLock<Option<Url>> = LazyLock::new(|| {
            let mut post_subreddits = SUBREDDIT_COMMANDS
                .get()
                .expect("Subreddit commands uninitialized")
                .keys()
                .map(ToString::to_string)
                .collect::<Vec<_>>();
            post_subreddits.sort();
            if !post_subreddits.is_empty() {
                Some(
                    Url::parse(&format!(
                        "{}/r/{}/new",
                        REDDIT_OAUTH_URL,
                        post_subreddits
                            .into_iter()
                            .reduce(|a, e| format!("{a}+{e}"))
                            .unwrap_or_default(),
                    ))
                    .expect("Failed to parse Url"),
                )
            } else {
                None
            }
        });
        static MENTION_URL: LazyLock<Url> = LazyLock::new(|| {
            Url::parse(&format!("{}/message/inbox", REDDIT_OAUTH_URL,))
                .expect("Failed to parse Url")
        });
        #[cfg(not(test))]
        if self.is_token_expired() {
            info!("Token expired, getting new token");
            self.token = RedditClient::get_reddit_token(
                std::env::var("APP_CLIENT_ID").expect("APP_CLIENT_ID must be set."),
                std::env::var("APP_SECRET").expect("APP_SECRET must be set."),
            )
            .await
            .expect("Failed to get token");
        }

        let mut time = (600.0, 0.0);

        fn add_query(request: RequestBuilder, after: &String) -> RequestBuilder {
            if after.is_empty() {
                request.query(&[(
                    "limit",
                    &COMMENT_COUNT
                        .get()
                        .expect("Comment count uninitialzed")
                        .to_string(),
                )])
            } else {
                request.query(&[
                    (
                        "limit",
                        &COMMENT_COUNT
                            .get()
                            .expect("Comment count uninitialized")
                            .to_string(),
                    ),
                    ("before", after),
                ])
            }
        }

        let (subs_response, posts_response, mentions_response) = join!(
            OptionFuture::from(SUBREDDIT_URL.clone().map(|subreddit_url| {
                let request = self.client.get(subreddit_url);
                let request = add_query(request, &last_ids.0);
                request.bearer_auth(&self.token.access_token).send()
            })),
            OptionFuture::from(
                check_posts
                    .then_some(SUBREDDIT_POSTS_URL.clone())
                    .flatten()
                    .map(|subreddit_url| {
                        let request = self.client.get(subreddit_url);
                        let request = add_query(request, &last_ids.1);
                        request.bearer_auth(&self.token.access_token).send()
                    })
            ),
            OptionFuture::from(check_mentions.then_some(MENTION_URL.clone()).map(
                |subreddit_url| {
                    let request = self.client.get(subreddit_url);
                    let request = add_query(request, &last_ids.2);
                    request.bearer_auth(&self.token.access_token).send()
                }
            )),
        );
        let subs_response = subs_response.map(|x| x.expect("Failed to get comments"));
        let posts_response = posts_response.map(|x| x.expect("Failed to get comments"));
        let mentions_response = mentions_response.map(|x| x.expect("Failed to get comments"));

        match subs_response
            .as_ref()
            .map(RedditClient::check_response_status)
            .unwrap_or(Ok(()))
            .and(
                posts_response
                    .as_ref()
                    .map(RedditClient::check_response_status)
                    .unwrap_or(Ok(())),
            )
            .and(
                mentions_response
                    .as_ref()
                    .map(RedditClient::check_response_status)
                    .unwrap_or(Ok(())),
            ) {
            Ok(_) => {
                let (mentions, ids) = if let Some(mentions_response) = mentions_response {
                    let (a, b, t, id) = RedditClient::extract_comments(
                        mentions_response,
                        already_replied_to_comments,
                        true,
                        SUBREDDIT_COMMANDS.get().unwrap(),
                        &HashMap::new(),
                    )
                    .await
                    .expect("Failed to extract comments");
                    if let Some(t) = t {
                        if t.0 < time.0 {
                            time = t;
                        }
                    } else {
                        warn!("Missing ratelimit")
                    }
                    if let Some(id) = id {
                        last_ids.2 = id;
                    };
                    (Some(a), Some(b))
                } else {
                    (None, None)
                };
                let mut res = if let Some(subs_response) = subs_response {
                    let (a, _, t, id) = RedditClient::extract_comments(
                        subs_response,
                        already_replied_to_comments,
                        false,
                        SUBREDDIT_COMMANDS.get().unwrap(),
                        &HashMap::new(),
                    )
                    .await
                    .expect("Failed to extract comments");
                    if let Some(t) = t {
                        if t.0 < time.0 {
                            time = t;
                        }
                    } else {
                        warn!("Missing ratelimit");
                    }
                    if let Some(id) = id {
                        last_ids.0 = id;
                    };
                    a
                } else {
                    Vec::new()
                };
                if let Some(posts_response) = posts_response {
                    let (posts, _, t, id) = RedditClient::extract_comments(
                        posts_response,
                        already_replied_to_comments,
                        false,
                        SUBREDDIT_COMMANDS.get().unwrap(),
                        &HashMap::new(),
                    )
                    .await
                    .expect("Failed to extract comments");
                    if let Some(t) = t {
                        if t.0 < time.0 {
                            time = t;
                        }
                    } else {
                        warn!("Missing ratelimit");
                    }
                    if let Some(id) = id {
                        last_ids.1 = id;
                    };
                    res.extend(posts);
                }
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
                        let (comments, _, t, _) = Self::extract_comments(
                            response,
                            already_replied_to_comments,
                            true,
                            SUBREDDIT_COMMANDS.get().unwrap(),
                            &ids.into_iter().collect(),
                        )
                        .await
                        .expect("Failed to extract comments");
                        if let Some(t) = t {
                            if t.0 < time.0 {
                                time = t;
                            }
                        } else {
                            warn!("Missing ratelimit");
                        }
                        res.extend(comments);
                    }
                }
                if let Some(mentions) = mentions {
                    res.extend(mentions);
                }
                Ok((res, time))
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
        &mut self,
        comment: RedditCommentCalculated,
        reply: &str,
    ) -> Result<Option<(f64, f64)>, Error> {
        #[cfg(not(test))]
        if self.is_token_expired() {
            info!("Token expired, getting new token");
            self.token = RedditClient::get_reddit_token(
                std::env::var("APP_CLIENT_ID").expect("APP_CLIENT_ID must be set."),
                std::env::var("APP_SECRET").expect("APP_SECRET must be set."),
            )
            .await
            .expect("Failed to get token");
        }

        let params = json!({
            "thing_id": comment.id,
            "text": reply
        });

        let response = self
            .client
            .post(REDDIT_COMMENT_URL)
            .bearer_auth(&self.token.access_token)
            .form(&params)
            .send()
            .await?;

        let response_headers = response.headers();
        let remaining: Option<f64> = response_headers
            .get("X-Ratelimit-Remaining")
            .map(|x| x.to_str().unwrap().parse().unwrap());
        let reset: Option<f64> = response_headers
            .get("X-Ratelimit-Reset")
            .map(|x| x.to_str().unwrap().parse().unwrap());

        let response_text = &response.text().await?;
        let response_text = response_text.as_str();
        let response_json =
            from_str::<Value>(response_text).expect("Failed to convert response to json");
        let response_status_err = !RedditClient::is_success(response_text);

        let error_message = RedditClient::get_error_message(response_json);

        if response_status_err {
            if error_message.contains("error.COMMENTER_BLOCKED_POSTER") {
                warn!(
                    "Comment ID {} by {} in {} -> Status FAILED: {:?}",
                    comment.id, comment.author, comment.subreddit, error_message
                );
                return Ok(reset.and_then(|reset| remaining.map(|remaining| (reset, remaining))));
            }

            if error_message.contains("error.DELETED_COMMENT") {
                info!(
                    "Comment ID {} by {} in {} -> Status FAILED: {:?}",
                    comment.id, comment.author, comment.subreddit, error_message
                );
                return Ok(reset.and_then(|reset| remaining.map(|remaining| (reset, remaining))));
            }

            error!(
                "Comment ID {} by {} in {} -> Status FAILED: {:?}",
                comment.id,
                comment.author,
                comment.subreddit,
                error_message
            );
            return Err(anyhow!("Failed to reply to comment"));
        }

        info!(
            "Comment ID {} -> Status OK: {:?}",
            comment.id,
            error_message
        );

        Ok(reset.and_then(|reset| remaining.map(|remaining| (reset, remaining))))
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
            error!("Failed to get token: {:?}", response);
            return Err("Failed to get token".into());
        }

        let response = response.json::<TokenResponse>().await?;

        let token_expiration_time = Self::get_expiration_time_from_jwt(&response.access_token);

        info!(
            "Fetched new token. Will expire: {:?}",
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
            error!(
                "Failed to get comments. Statuscode: {:?}. Response: {:?}",
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
        commands: &HashMap<&str, Commands>,
        mention_map: &HashMap<String, (String, Commands, String)>,
    ) -> Result<
        (
            Vec<RedditCommentConstructed>,
            Vec<(String, (String, Commands, String))>,
            Option<(f64, f64)>,
            Option<String>,
        ),
        Box<dyn std::error::Error>,
    > {
        let empty_vec = Vec::new();
        let headers = response.headers();
        let remaining: Option<f64> = headers
            .get("X-Ratelimit-Remaining")
            .map(|x| x.to_str().unwrap().parse().unwrap());
        let reset: Option<f64> = headers
            .get("X-Ratelimit-Reset")
            .map(|x| x.to_str().unwrap().parse().unwrap());

        let response_json = response.json::<Value>().await?;
        let comments_json = response_json["data"]["children"]
            .as_array()
            .unwrap_or(&empty_vec);

        already_replied_to_comments.reserve(comments_json.len());
        let mut comments = Vec::with_capacity(comments_json.len());
        let mut parent_paths = Vec::new();
        for comment in comments_json {
            let kind = comment["kind"].as_str().unwrap_or_default();
            let msg_type = comment["data"]["type"].as_str().unwrap_or_default();
            let extracted_comment = match kind {
                "t1" => Self::extract_comment(
                    comment,
                    already_replied_to_comments,
                    is_mention,
                    commands,
                    mention_map,
                ),
                "t3" => Self::extract_post(comment, already_replied_to_comments, commands),
                "t4" => Self::extract_message(comment, already_replied_to_comments, commands),
                e => {
                    error!(
                        "Encountered unknown kind: {e} at id {}",
                        comment["data"]["id"].as_str().unwrap_or_default()
                    );
                    continue;
                }
            };
            let Some(extracted_comment) = extracted_comment else {
                continue;
            };
            if is_mention
                && kind == "t1"
                && msg_type == "username_mention"
                && !extracted_comment.status.already_replied_or_rejected
                && extracted_comment.status.no_factorial
            {
                if let Some(path) = Self::extract_summon_parent_id(comment) {
                    parent_paths.push((
                        path,
                        (
                            extracted_comment.id.clone(),
                            extracted_comment.commands,
                            extracted_comment.author.clone(),
                        ),
                    ));
                }
            }
            comments.push(extracted_comment);
        }
        let id = if comments.is_empty() {
            Some(String::new())
        } else {
            comments.get(1).map(|comment| comment.id.clone())
        };

        Ok((
            comments,
            parent_paths,
            reset.and_then(|reset| remaining.map(|remaining| (reset, remaining))),
            id,
        ))
    }
    fn extract_comment(
        comment: &Value,
        already_replied_to_comments: &mut Vec<String>,
        do_termial: bool,
        commands: &HashMap<&str, Commands>,
        mention_map: &HashMap<String, (String, Commands, String)>,
    ) -> Option<RedditCommentConstructed> {
        let comment_text = comment["data"]["body"].as_str().unwrap_or("");
        let author = comment["data"]["author"].as_str().unwrap_or("");
        let subreddit = comment["data"]["subreddit"].as_str().unwrap_or("");
        let comment_id = comment["data"]["name"].as_str().unwrap_or_default();

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
                    if do_termial {
                        Commands::TERMIAL
                    } else {
                        Commands::NONE
                    } | commands.get(subreddit).copied().unwrap_or(Commands::NONE),
                )
            }) else {
                error!("Failed to construct comment {comment_id}!");
                return None;
            };
            if let Some((mention, commands, mention_author)) = mention_map.get(comment_id) {
                comment.id = mention.clone();
                comment.commands = *commands;
                comment.notify = Some(author.to_string());
                comment.author = mention_author.clone();
            }

            comment.add_status(Status::NOT_REPLIED);

            Some(comment)
        }
    }
    fn extract_message(
        message: &Value,
        already_replied_to_comments: &mut Vec<String>,
        commands: &HashMap<&str, Commands>,
    ) -> Option<RedditCommentConstructed> {
        let message_text = message["data"]["body"].as_str().unwrap_or("");
        let author = message["data"]["author"].as_str().unwrap_or("");
        let subreddit = message["data"]["subreddit"].as_str().unwrap_or("");
        let comment_id = message["data"]["name"].as_str().unwrap_or_default();

        if already_replied_to_comments.contains(&comment_id.to_string()) {
            Some(RedditComment::new_already_replied(
                comment_id, author, subreddit,
            ))
        } else {
            already_replied_to_comments.push(comment_id.to_string());
            let Ok(mut comment) = std::panic::catch_unwind(|| {
                RedditComment::new(
                    message_text,
                    comment_id,
                    author,
                    subreddit,
                    Commands::TERMIAL | commands.get(subreddit).copied().unwrap_or(Commands::NONE),
                )
            }) else {
                error!("Failed to construct comment {comment_id}!");
                return None;
            };

            comment.add_status(Status::NOT_REPLIED);

            Some(comment)
        }
    }
    fn extract_post(
        post: &Value,
        already_replied_to_comments: &mut Vec<String>,
        commands: &HashMap<&str, Commands>,
    ) -> Option<RedditCommentConstructed> {
        let post_text = post["data"]["selftext"].as_str().unwrap_or("");
        let post_title = post["data"]["title"].as_str().unwrap_or("");
        let post_flair = post["data"]["link_flair_text"].as_str().unwrap_or("");
        let author = post["data"]["author"].as_str().unwrap_or("");
        let subreddit = post["data"]["subreddit"].as_str().unwrap_or("");
        let post_id = post["data"]["name"].as_str().unwrap_or_default();

        let body = format!("{post_title} {post_text} {post_flair}");

        Some(
            if already_replied_to_comments.contains(&post_id.to_string()) {
                RedditComment::new_already_replied(post_id, author, subreddit)
            } else {
                already_replied_to_comments.push(post_id.to_string());
                let Ok(mut comment) = std::panic::catch_unwind(|| {
                    RedditComment::new(
                        &body,
                        post_id,
                        author,
                        subreddit,
                        commands.get(subreddit).copied().unwrap_or(Commands::NONE),
                    )
                }) else {
                    error!("Failed to construct comment {post_id}!");
                    return None;
                };

                comment.add_status(Status::NOT_REPLIED);

                comment
            },
        )
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

    use crate::calculation_results::{Calculation, Number};

    use super::*;

    async fn dummy_server(reqeuest_response_pairs: &[(&str, &str)]) -> std::io::Result<()> {
        let listen = TcpListener::bind("127.0.0.1:9384").await?;
        for (expected_request, response) in reqeuest_response_pairs {
            let mut sock = timeout(Duration::from_secs(5), listen.accept()).await??.0;
            let mut request = vec![0; 10000];
            let len = timeout(Duration::from_millis(300), sock.read(&mut request)).await??;
            request.truncate(len);
            let request = String::from_utf8(request).expect("Got invalid utf8");
            if &request != expected_request {
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
            if let Ok(lock) = SEQUENTIAL_LOCK.lock() {
                return lock;
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
        let mut client = RedditClient {
            client: Client::new(),
            token: Token {
                access_token: "token".to_string(),
                expiration_time: Utc::now(),
            },
        };
        let (status, reply_status) = join!(
            dummy_server(&[(
                "POST / HTTP/1.1\r\nauthorization: Bearer token\r\ncontent-type: application/x-www-form-urlencoded\r\naccept: */*\r\nhost: 127.0.0.1:9384\r\ncontent-length: 32\r\n\r\ntext=I+relpy&thing_id=t1_some_id",
                "HTTP/1.1 200 OK\r\nx-ratelimit-remaining: 10\r\nx-ratelimit-reset: 200\n\n{\"success\": true}"
            )]),
            client.reply_to_comment(RedditComment::new_already_replied("t1_some_id", "author", "subressit").extract().calc(), "I relpy")
        );
        status.unwrap();
        let reply_status = reply_status.unwrap();
        assert_eq!(reply_status, Some((200.0, 10.0)));
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
        let _ = SUBREDDIT_COMMANDS.set(
            [
                ("test_subreddit", Commands::TERMIAL),
                ("post_subreddit", Commands::POST_ONLY),
            ]
            .into(),
        );
        let _ = COMMENT_COUNT.set(100);
        let mut already_replied = vec![];
        let mut last_ids = (
            "t1_m86nsre".to_owned(),
            "t3_83us27sa".to_owned(),
            "".to_owned(),
        );
        let (status, comments) = join!(
            async {
                dummy_server(&[(
                    "GET /r/test_subreddit/comments?limit=100&before=t1_m86nsre HTTP/1.1\r\nauthorization: Bearer token\r\naccept: */*\r\nhost: 127.0.0.1:9384\r\n\r\n",
                    "HTTP/1.1 200 OK\r\nx-ratelimit-remaining: 10\r\nx-ratelimit-reset: 200\n\n{\"data\":{\"children\":[]}}"
                ),(
                    "GET /r/post_subreddit+test_subreddit/new?limit=100&before=t3_83us27sa HTTP/1.1\r\nauthorization: Bearer token\r\naccept: */*\r\nhost: 127.0.0.1:9384\r\n\r\n",
                    "HTTP/1.1 200 OK\r\nx-ratelimit-remaining: 9\r\nx-ratelimit-reset: 200\n\n{\"data\":{\"children\":[]}}"
                ),(
                    "GET /message/inbox?limit=100 HTTP/1.1\r\nauthorization: Bearer token\r\naccept: */*\r\nhost: 127.0.0.1:9384\r\n\r\n",
                    "HTTP/1.1 200 OK\r\nx-ratelimit-remaining: 8\r\nx-ratelimit-reset: 199\n\n{\"data\":{\"children\":[{\"kind\":\"t1\",\"data\":{\"author\":\"mentioner\",\"body\":\"u/factorion-bot !termial\",\"type\":\"username_mention\",\"parent_id\":\"t1_m38msum\"}}]}}"
                ),(
                    "GET /api/info?id=t1_m38msum HTTP/1.1\r\nauthorization: Bearer token\r\naccept: */*\r\nhost: 127.0.0.1:9384\r\n\r\n",
                    "HTTP/1.1 200 OK\r\nx-ratelimit-remaining: 7\r\nx-ratelimit-reset: 170\n\n{\"data\": {\"children\": [{\"kind\": \"t1\",\"data\":{\"name\":\"t1_m38msum\", \"body\":\"That's 57!?\"}}]}}"
                )]).await
            },
            client.get_comments(&mut already_replied, true, true, &mut last_ids)
        );
        status.unwrap();
        let (comments, rate) = comments.unwrap();
        let comments = comments
            .into_iter()
            .map(|c| c.extract().calc())
            .collect::<Vec<_>>();
        assert_eq!(comments.len(), 2);
        assert_eq!(comments[0].id, "");
        assert_eq!(comments[0].author, "mentioner");
        assert_eq!(comments[0].notify.as_ref().unwrap(), "");
        assert_eq!(comments[0].commands, Commands::TERMIAL);
        assert_eq!(comments[0].calculation_list[0].steps, [(1, 0), (0, 0)]);
        assert_eq!(rate, (170.0, 7.0))
    }

    #[tokio::test]
    async fn test_extract_comments() {
        let response = Response::from(http::Response::builder().status(200).header("X-Ratelimit-Remaining", "10").header("X-Ratelimit-Reset", "350").body(r#"{
               "data": {
                   "children": [
                       {
                           "kind": "t1",
                           "data": {
                               "author": "Little_Tweetybird_",
                               "author_fullname": "t2_b5n60qnt",
                               "body": "comment 1!!",
                               "body_html": "&lt;div class=\"md\"&gt;&lt;p&gt;comment 1!!&lt;/p&gt;\n&lt;/div&gt;",
                               "name": "t1_m38msum",
                               "locked": false,
                               "unrepliable_reason": null
                           }
                       },
                       {
                           "kind": "t1",
                           "data": {
                               "author": "Little_Tweetybird_",
                               "author_fullname": "t2_b5n60qnt",
                               "body": "comment 2",
                               "body_html": "&lt;div class=\"md\"&gt;&lt;p&gt;comment 2&lt;/p&gt;\n&lt;/div&gt;",
                               "name": "t1_m38msug",
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
                               "name": "t1_m38msun",
                               "type": "username_mention",
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
            &HashMap::new(),
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
                    "t1_m38msun".to_string(),
                    Commands {
                        termial: true,
                        ..Default::default()
                    },
                    "Little_Tweetybird_".to_string(),
                )
            )]
        );
        println!("{:?}", comments);
        assert_eq!(comments.2, Some((350.0, 10.0)));
    }

    #[tokio::test]
    async fn test_extract_posts() {
        let response = Response::from(http::Response::builder().status(200).header("X-Ratelimit-Remaining", "10").header("X-Ratelimit-Reset", "350").body(r#"{
               "data": {
                   "children": [
                       {
                           "kind": "t3",
                           "data": {
                               "author": "Little_Tweetybird_",
                               "author_fullname": "t2_b5n60qnt",
                               "title": "Thats just 1",
                               "selftext": "comment 1!!",
                               "selftext_html": "&lt;div class=\"md\"&gt;&lt;p&gt;comment 1!!&lt;/p&gt;\n&lt;/div&gt;",
                               "name": "t3_m38msum",
                               "locked": false,
                               "unrepliable_reason": null
                           }
                       },
                       {
                           "kind": "t3",
                           "data": {
                               "author": "Little_Tweetybird_",
                               "author_fullname": "t2_b5n60qnt",
                               "title": "2!",
                               "selftext": "comment 2",
                               "selftext_html": "&lt;div class=\"md\"&gt;&lt;p&gt;comment 2&lt;/p&gt;\n&lt;/div&gt;",
                               "name": "t3_m38msug",
                               "locked": false,
                              "unrepliable_reason": null
                           }
                       },
                       {
                           "kind": "t3",
                           "data": {
                               "author": "Little_Tweetybird_",
                               "author_fullname": "t2_b5n60qnt",
                               "title": "A mention",
                               "selftext": "u/factorion-bot",
                               "selftext_html": "&lt;div class=\"md\"&gt;&lt;p&gt;u/factorion-bot&lt;/p&gt;\n&lt;/div&gt;",
                               "link_flair_text": "!10",
                               "name": "t1_m38msun",
                               "parent_id": "t3_m38msum",
                               "context": "/r/some_sub/8msu32a/some_post/m38msun/?context=3"
                           }
                       }
                   ]
               }
           }"#).unwrap());
        let mut already_replied = vec![];
        let (comments, _, t, id) = RedditClient::extract_comments(
            response,
            &mut already_replied,
            false,
            &HashMap::new(),
            &HashMap::new(),
        )
        .await
        .unwrap();
        let comments = comments
            .into_iter()
            .map(|c| c.extract().calc())
            .collect::<Vec<_>>();
        assert_eq!(comments.len(), 3);
        assert_eq!(
            comments[0].calculation_list,
            [Calculation {
                value: Number::Int(1.into()),
                steps: vec![(2, 0)],
                result: crate::calculation_results::CalculationResult::Exact(1.into())
            }]
        );
        assert_eq!(
            comments[1].calculation_list,
            [Calculation {
                value: Number::Int(2.into()),
                steps: vec![(1, 0)],
                result: crate::calculation_results::CalculationResult::Exact(2.into())
            }]
        );
        assert_eq!(
            comments[2].calculation_list,
            [Calculation {
                value: Number::Int(10.into()),
                steps: vec![(-1, 0)],
                result: crate::calculation_results::CalculationResult::Exact(1334961.into())
            }]
        );
        println!("{:?}", comments);
        assert_eq!(t, Some((350.0, 10.0)));
        assert_eq!(id.unwrap(), "t3_m38msug");
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
