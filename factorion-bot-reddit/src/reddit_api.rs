#![allow(deprecated)] // base64::encode is deprecated

use std::borrow::Cow;
use std::collections::HashMap;
use std::fmt::Write;
use std::sync::LazyLock;

use crate::{
    COMMENT_COUNT, MAX_ALREADY_REPLIED_LEN, SUBREDDIT_COMMANDS, SubredditEntry, SubredditMode,
};
use anyhow::{Error, anyhow};
use base64::Engine;
use base64::engine::general_purpose::STANDARD_NO_PAD;
use chrono::{DateTime, NaiveDateTime, Utc};
use factorion_lib::comment::{Commands, Comment, CommentCalculated, CommentConstructed, Status};
use futures::future::OptionFuture;
use id::{DenseId, id_to_dense};
use log::{debug, error, info, log, warn};
use reqwest::header::{CONTENT_TYPE, HeaderMap, USER_AGENT};
use reqwest::{Client, RequestBuilder, Response, Url};
use serde::{Deserialize, Serialize};
use serde_json::{Value, from_str, json};
use tokio::join;
use tokio::sync::Mutex;

#[derive(Deserialize, Debug)]
struct TokenResponse {
    access_token: String,
}

struct Token {
    access_token: String,
    expiration_time: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct Meta {
    pub id: String,
    pub author: String,
    pub subreddit: String,
    pub thread: String,
    pub used_commands: bool,
}

#[derive(PartialEq, PartialOrd, Eq, Ord, Debug, Clone, Deserialize, Serialize)]
pub(crate) struct Thread {
    pub id: DenseId,
    pub calcs: Vec<(factorion_lib::CalculationJob, usize)>,
}

#[cfg(not(test))]
const REDDIT_OAUTH_URL: &str = "https://oauth.reddit.com";
#[cfg(test)]
const REDDIT_OAUTH_URL: &str = "http://127.0.0.1:9384";
#[cfg(not(test))]
const REDDIT_TOKEN_URL: &str = "https://www.reddit.com/api/v1/access_token";
#[cfg(test)]
const REDDIT_TOKEN_URL: &str = "http://127.0.0.1:9384";
#[cfg(not(test))]
const REDDIT_COMMENT_URL: &str = "https://oauth.reddit.com/api/comment";
#[cfg(test)]
const REDDIT_COMMENT_URL: &str = "http://127.0.0.1:9384";

const MAX_COMMENT_LEN: usize = 10_000;

pub(crate) struct RedditClient {
    client: Client,
    token: Token,
}
#[derive(Debug, Clone)]
pub(crate) struct RateLimitErr;
impl std::fmt::Display for RateLimitErr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Ratelimit hit! Got 429.")
    }
}
impl std::error::Error for RateLimitErr {}
#[derive(Debug, Clone, Default)]
pub struct LastIds {
    pub comments: (String, String),
    pub posts: (String, String),
    pub mentions: (String, String),
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
    /// Panics if `SUBREDDITS` or `COMMENT_COUNT` is uninitialized, if the env vars APP_CLIENT_ID or APP_SECRET are unset, or if it receives a malformed response from the api.
    pub(crate) async fn get_comments(
        &mut self,
        already_replied_to_comments: &mut Vec<DenseId>,
        check_mentions: bool,
        check_posts: bool,
        last_ids: &mut LastIds,
    ) -> Result<(Vec<CommentConstructed<Meta>>, (f64, f64)), ()> {
        static SUBREDDIT_URL: LazyLock<Option<Url>> = LazyLock::new(|| {
            let mut subreddits = SUBREDDIT_COMMANDS
                .get()
                .expect("Subreddit commands uninitialized")
                .iter()
                .filter(|(_, entry)| entry.mode == SubredditMode::All)
                .map(|(sub, _)| sub.to_string())
                .collect::<Vec<_>>();
            subreddits.sort();
            info!("Setting comments to be checked in: {subreddits:?}");
            if !(subreddits.is_empty() || subreddits == [""]) {
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
                .iter()
                .filter(|(_, entry)| entry.mode != SubredditMode::None)
                .map(|(sub, _)| sub.to_string())
                .collect::<Vec<_>>();
            post_subreddits.sort();
            info!("Setting posts to be checked in: {post_subreddits:?}");
            if !(post_subreddits.is_empty() || post_subreddits == [""]) {
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
            Url::parse(&format!("{REDDIT_OAUTH_URL}/message/inbox")).expect("Failed to parse Url")
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

        let mut reset_timer = (600.0, 0.0);

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
                let request = add_query(request, &last_ids.comments.0);
                request.bearer_auth(&self.token.access_token).send()
            })),
            OptionFuture::from(
                check_posts
                    .then_some(SUBREDDIT_POSTS_URL.clone())
                    .flatten()
                    .map(|subreddit_url| {
                        let request = self.client.get(subreddit_url);
                        let request = add_query(request, &last_ids.posts.0);
                        request.bearer_auth(&self.token.access_token).send()
                    })
            ),
            OptionFuture::from(check_mentions.then_some(MENTION_URL.clone()).map(
                |subreddit_url| {
                    let request = self.client.get(subreddit_url);
                    let request = add_query(request, &last_ids.mentions.0);
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
                    let (a, b, t, id) = self
                        .extract_comments(
                            mentions_response,
                            already_replied_to_comments,
                            true,
                            SUBREDDIT_COMMANDS.get().unwrap(),
                            &HashMap::new(),
                        )
                        .await
                        .expect("Failed to extract comments");

                    reset_timer = Self::update_reset_timer(reset_timer, t);

                    if !a.is_empty()
                        && last_ids.mentions.1 != ""
                        && !a.iter().any(|x| x.meta.id == last_ids.mentions.1)
                    {
                        warn!(
                            "Failed to keep up with mentions. last_id: {}",
                            last_ids.mentions.1
                        );
                    }

                    if let Some(id) = id {
                        last_ids.mentions = id;
                    }
                    (Some(a), Some(b))
                } else {
                    (None, None)
                };
                let mut res = if let Some(subs_response) = subs_response {
                    let (a, _, t, id) = self
                        .extract_comments(
                            subs_response,
                            already_replied_to_comments,
                            false,
                            SUBREDDIT_COMMANDS.get().unwrap(),
                            &HashMap::new(),
                        )
                        .await
                        .expect("Failed to extract comments");

                    reset_timer = Self::update_reset_timer(reset_timer, t);

                    if !a.is_empty()
                        && last_ids.comments.1 != ""
                        && !a.iter().any(|x| x.meta.id == last_ids.comments.1)
                    {
                        warn!(
                            "Failed to keep up with comments. last_id: {}",
                            last_ids.comments.1
                        );
                    }

                    if let Some(id) = id {
                        last_ids.comments = id;
                    }
                    a
                } else {
                    Vec::new()
                };
                if let Some(posts_response) = posts_response {
                    let (posts, _, t, id) = self
                        .extract_comments(
                            posts_response,
                            already_replied_to_comments,
                            false,
                            SUBREDDIT_COMMANDS.get().unwrap(),
                            &HashMap::new(),
                        )
                        .await
                        .expect("Failed to extract comments");

                    reset_timer = Self::update_reset_timer(reset_timer, t);

                    if !posts.is_empty()
                        && last_ids.posts.1 != ""
                        && !posts.iter().any(|x| x.meta.id == last_ids.posts.1)
                    {
                        warn!(
                            "Failed to keep up with posts. last_id: {}",
                            last_ids.posts.1
                        );
                    }

                    if let Some(id) = id {
                        last_ids.posts = id;
                    }
                    res.extend(posts);
                }
                if let Some(ids) = ids
                    && !ids.is_empty()
                {
                    'get_summons: loop {
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
                            let (comments, _, t, _) = self
                                .extract_comments(
                                    response,
                                    already_replied_to_comments,
                                    true,
                                    SUBREDDIT_COMMANDS.get().unwrap(),
                                    &ids.into_iter().collect(),
                                )
                                .await
                                .expect("Failed to extract comments");

                            reset_timer = Self::update_reset_timer(reset_timer, t);

                            res.extend(comments);
                        } else if response.status().as_u16() == 429 {
                            tokio::time::sleep(std::time::Duration::from_secs(
                                reset_timer.0.ceil() as u64,
                            ))
                            .await;
                            continue 'get_summons;
                        }
                        break 'get_summons;
                    }
                }
                if let Some(mentions) = mentions {
                    res.extend(mentions);
                }
                Ok((res, reset_timer))
            }
            Err(_) => Err(()),
        }
    }

    fn update_reset_timer(
        mut current_reset_timer: (f64, f64),
        t: Option<(f64, f64)>,
    ) -> (f64, f64) {
        if let Some(t) = t {
            debug!(
                "Update time. t.0: {:?}, time.0: {:?}",
                t.0, current_reset_timer.0
            );
            if t.0 < current_reset_timer.0 {
                current_reset_timer = t;
            }
        } else {
            warn!("Missing ratelimit")
        }
        current_reset_timer
    }

    fn is_token_expired(&self) -> bool {
        let now = Utc::now();
        now > self.token.expiration_time
    }

    /// Replies to the given `comment` with the given `reply`.
    /// # Panic
    /// May panic on a malformed response is received from the api.
    pub(crate) async fn reply_to_comment(
        &mut self,
        comment: &CommentCalculated<Meta>,
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
            "thing_id": comment.meta.id,
            "text": reply
        });

        let response = self
            .client
            .post(REDDIT_COMMENT_URL)
            .bearer_auth(&self.token.access_token)
            .form(&params)
            .send()
            .await?;

        if response.status().as_u16() == 429 {
            Err(RateLimitErr)?
        };

        let response_headers = response.headers();
        let ratelimit_remaining: Option<f64> = response_headers
            .get("X-Ratelimit-Remaining")
            .map(|x| x.to_str().unwrap().parse().unwrap());
        let ratelimit_reset: Option<f64> = response_headers
            .get("X-Ratelimit-Reset")
            .map(|x| x.to_str().unwrap().parse().unwrap());

        let response_text = &response.text().await?;
        let response_text = response_text.as_str();
        let response_json =
            from_str::<Value>(response_text).expect("Failed to convert response to json");
        let response_status_err = !RedditClient::is_success(response_text);

        let error_message = RedditClient::get_error_message(response_json);

        if response_status_err {
            let level = if error_message.contains("error.COMMENTER_BLOCKED_POSTER") {
                log::Level::Warn
            } else if error_message.contains("error.DELETED_COMMENT") {
                log::Level::Info
            } else {
                log::Level::Error
            };

            log!(
                level,
                "Comment ID {} by {} in {} -> Status FAILED: {:?}",
                comment.meta.id,
                comment.meta.author,
                comment.meta.subreddit,
                error_message
            );
            return match level {
                log::Level::Error => Err(anyhow!("Failed to reply to comment")),
                _ => Ok(ratelimit_reset
                    .and_then(|reset| ratelimit_remaining.map(|remaining| (reset, remaining)))),
            };
        }

        info!(
            "Comment ID {} -> Status OK: {:?}",
            comment.meta.id, error_message
        );

        Ok(ratelimit_reset
            .and_then(|reset| ratelimit_remaining.map(|remaining| (reset, remaining))))
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
            error!("Failed to get token: {response:?}");
            return Err("Failed to get token".into());
        }

        let response = response.json::<TokenResponse>().await?;

        let token_expiration_time = Self::get_expiration_time_from_jwt(&response.access_token);

        info!("Fetched new token. Will expire: {token_expiration_time:?}");

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
            if response.status().as_u16() == 500 {
                error!(
                    "Failed to get comments. Statuscode: {:?}. Internal server error.",
                    response.status()
                );
            } else {
                error!(
                    "Failed to get comments. Statuscode: {:?}. Response: {:?}",
                    response.status(),
                    response
                );
            }
            return Err(());
        }

        Ok(())
    }

    fn extract_summon_parent_id(comment: &Value) -> Option<String> {
        let parent_id = comment["data"]["parent_id"].as_str()?.to_string();
        Some(parent_id)
    }
    async fn extract_comments(
        &self,
        response: Response,
        already_replied_to_comments: &mut Vec<DenseId>,
        is_mention: bool,
        subs: &HashMap<&str, SubredditEntry>,
        mention_map: &HashMap<String, (String, Commands, String)>,
    ) -> Result<
        (
            Vec<CommentConstructed<Meta>>,
            Vec<(String, (String, Commands, String))>,
            Option<(f64, f64)>,
            Option<(String, String)>,
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
            let thread = comment["data"]["permalink"]
                .as_str()
                .or_else(|| comment["data"]["context"].as_str())
                .and_then(|x| x.split('/').nth(4))
                .unwrap_or("");
            let reply_body;
            let commands = subs
                .get("")
                .map(|entry| entry.commands)
                .unwrap_or(Commands::NONE);
            let (locale, commands) = if matches!(kind, "t1" | "t3") {
                let sub = comment["data"]["subreddit"].as_str().unwrap_or_default();
                if let Some(SubredditEntry {
                    locale,
                    commands,
                    mode: _,
                }) = subs.get(sub)
                {
                    (*locale, *commands)
                } else {
                    // To minimize the need to clone, we store leaked strings.
                    // That is acceptable, as it cleanup of this would be hard,
                    // and the amount of data leaked is very small
                    // (2 Bytes plus effectively up to 30 Bytes ca. 9 times a day
                    // => ca. 100 kB a year)
                    static LANG_CACHE: LazyLock<Mutex<HashMap<String, &str>>> =
                        LazyLock::new(|| Mutex::new(HashMap::new()));
                    if let Some(locale) = LANG_CACHE.lock().await.get(sub) {
                        (*locale, commands)
                    } else {
                        let request = self.client.get(format!("{REDDIT_OAUTH_URL}/r/{sub}/about"));
                        let reply = request.bearer_auth(&self.token.access_token).send().await?;
                        reply_body = reply.json::<Value>().await?;
                        let locale = reply_body["data"]["lang"]
                            .as_str()
                            .map(|x| &*x.to_owned().leak())
                            .unwrap_or("en");
                        LANG_CACHE.lock().await.insert(sub.to_owned(), locale);
                        info!("Added to lang cache {sub}:{locale}");
                        (locale, commands)
                    }
                }
            } else {
                ("en", commands)
            };
            let extracted_comment = match kind {
                // Comment
                "t1" => Self::extract_comment(
                    comment,
                    already_replied_to_comments,
                    is_mention,
                    mention_map,
                    locale,
                    thread,
                    commands,
                    |comment| Cow::Borrowed(comment["data"]["body"].as_str().unwrap_or("")),
                ),
                // Post
                "t3" => Self::extract_comment(
                    comment,
                    already_replied_to_comments,
                    is_mention,
                    mention_map,
                    locale,
                    thread,
                    commands,
                    |comment| {
                        let post_text = comment["data"]["selftext"].as_str().unwrap_or("");
                        let post_title = comment["data"]["title"].as_str().unwrap_or("");
                        let post_flair = comment["data"]["link_flair_text"].as_str().unwrap_or("");
                        Cow::Owned(format!("{post_title} {post_flair} {post_text}"))
                    },
                ),
                // Message
                "t4" => Self::extract_comment(
                    comment,
                    already_replied_to_comments,
                    true,
                    mention_map,
                    locale,
                    thread,
                    commands,
                    |comment| Cow::Borrowed(comment["data"]["body"].as_str().unwrap_or("")),
                ),
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
                && let Some(path) = Self::extract_summon_parent_id(comment)
            {
                parent_paths.push((
                    path,
                    (
                        extracted_comment.meta.id.clone(),
                        extracted_comment.commands,
                        extracted_comment.meta.author.clone(),
                    ),
                ));
            }
            comments.push(extracted_comment);
        }
        let id = if comments.is_empty() {
            warn!("No comments. Requested comment (last_id or summon) is gone.");
            Some((String::new(), String::new()))
        } else {
            comments
                .get(1)
                .map(|o| (o.meta.id.clone(), comments.get(0).unwrap().meta.id.clone()))
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
        already_replied_to_comments: &mut Vec<DenseId>,
        do_termial: bool,
        mention_map: &HashMap<String, (String, Commands, String)>,
        locale: &str,
        thread: &str,
        commands: Commands,
        extract_body: impl Fn(&Value) -> Cow<str>,
    ) -> Option<CommentConstructed<Meta>> {
        let author = comment["data"]["author"].as_str().unwrap_or("");
        let subreddit = comment["data"]["subreddit"].as_str().unwrap_or("");
        let comment_id = comment["data"]["name"].as_str().unwrap_or_default();
        let dense_id =
            id_to_dense(comment_id).unwrap_or_else(|_| panic!("Malformed comment id {comment_id}"));
        let body = extract_body(comment);

        if let Some(i) = dense_id.slice_contains_rev(already_replied_to_comments) {
            // Check if we might lose this id (causing double reply)
            if let Some(min) = already_replied_to_comments
                .len()
                .checked_sub(MAX_ALREADY_REPLIED_LEN / 5 * 4)
                && i < min
            {
                already_replied_to_comments.push(dense_id);
            }
            Some(Comment::new_already_replied(
                Meta {
                    id: comment_id.to_owned(),
                    author: author.to_owned(),
                    subreddit: subreddit.to_owned(),
                    thread: thread.to_owned(),
                    used_commands: false,
                },
                MAX_COMMENT_LEN,
                locale,
            ))
        } else {
            already_replied_to_comments.push(dense_id);
            let pre_commands = if do_termial {
                Commands::TERMIAL
            } else {
                Commands::NONE
            } | commands;
            let Ok(mut comment) = std::panic::catch_unwind(|| {
                Comment::new(
                    &body,
                    Meta {
                        id: comment_id.to_owned(),
                        author: author.to_owned(),
                        subreddit: subreddit.to_owned(),
                        thread: thread.to_owned(),
                        used_commands: false,
                    },
                    pre_commands,
                    MAX_COMMENT_LEN,
                    locale,
                )
            }) else {
                error!("Failed to construct comment {comment_id}!");
                return None;
            };
            comment.meta.used_commands = !(comment.commands == pre_commands);
            if let Some((mention, commands, mention_author)) = mention_map.get(comment_id) {
                comment.meta.id = mention.clone();
                comment.commands = *commands;
                comment.notify = Some(format!("u/{author}"));
                comment.meta.author = mention_author.clone();
            }

            comment.add_status(Status::NOT_REPLIED);

            Some(comment)
        }
    }
}

pub mod id {
    use serde::{Deserialize, Serialize};

    /// A dense representation of reddit fullnames (ids)
    ///
    /// Uses a u64 underneath, utilising the top 3 bits for the tag and the rest for the id.
    /// This means, that there may be upto 2^61 ids in reddits sequential id system, which will never be reached.
    #[repr(transparent)]
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
    pub struct DenseId(u64);
    impl DenseId {
        pub fn raw(&self) -> u64 {
            self.0
        }
        pub fn from_raw(v: u64) -> Self {
            Self(v)
        }
        /// Stolen from the contains implementation for ints, just reverse order
        #[inline]
        pub fn slice_contains_rev(&self, arr: &[Self]) -> Option<usize> {
            // Make our LANE_COUNT 4x the normal lane count (aiming for 128 bit vectors).
            // The compiler will nicely unroll it.
            const LANE_COUNT: usize = 4 * (128 / (size_of::<DenseId>() * 8));
            // SIMD
            let mut chunks = arr.rchunks_exact(LANE_COUNT);
            for (c, chunk) in (&mut chunks).enumerate() {
                if let Some(i) = chunk
                    .iter()
                    .rev()
                    .enumerate()
                    .find_map(|(i, x)| (*x == *self).then_some(i))
                {
                    return Some(arr.len() - (c * LANE_COUNT + i) - 1);
                }
            }
            // Scalar remainder
            let l = chunks.remainder().len();

            chunks
                .remainder()
                .iter()
                .rev()
                .enumerate()
                .find(|(_, x)| **x == *self)
                .map(|(i, _)| l - i - 1)
        }
    }
    impl TryFrom<&str> for DenseId {
        type Error = ParseIdErr;
        fn try_from(value: &str) -> Result<Self, Self::Error> {
            id_to_dense(value)
        }
    }
    #[derive(Debug, Clone)]
    pub enum ParseIdErr {
        InvalidTag,
        #[allow(dead_code)]
        ParseIntErr(std::num::ParseIntError),
    }
    pub fn id_to_dense(id: &str) -> Result<DenseId, ParseIdErr> {
        let tag: u64 = match &id[..3] {
            // Message
            "t1_" => 1,
            "t2_" => 2,
            // Post
            "t3_" => 3,
            // Comment
            "t4_" => 4,
            "t5_" => 5,
            "t6_" => 6,
            _ => Err(ParseIdErr::InvalidTag)?,
        };
        let id = u64::from_str_radix(&id[3..], 36).map_err(ParseIdErr::ParseIntErr)?;
        // Pack it
        let tag = tag << 61;
        let packed = tag | id;
        Ok(DenseId(packed))
    }
}

#[allow(clippy::await_holding_lock)]
#[cfg(test)]
mod tests {
    use std::time::Duration;

    use tokio::{
        io::{AsyncReadExt, AsyncWriteExt},
        net::TcpListener,
        time::timeout,
    };

    use factorion_lib::{
        Consts,
        calculation_results::{Calculation, CalculationResult, Number},
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
            if &request != expected_request {
                panic!("Wrong request: {request:?}\nExpected: {expected_request:?}");
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
            "HTTP/1.1 200 OK\n\n{\"access_token\": \"eyJhbGciOiJSUzI1NiIsImtpZCI6IlNIQTI1NjpzS3dsMnlsV0VtMjVmcXhwTU40cWY4MXE2OWFFdWFyMnpLMUdhVGxjdWNZIiwidHlwIjoiSldUIn0.eyJzdWIiOiJ1c2dyIiwiZXhwIjoxNzM1MTQ0NjI0LjQ2OTAyLCJpYXQiOjE3MzUwNTgyMjQuNDY5MDIsImp0aSI6IlpDM0Y2YzVXUGh1a09zVDRCcExaa0lmam1USjBSZyIsImNpZCI6IklJbTJha1RaRDFHWXd5Y1lXTlBKWVEiLCJsaWQiOiJ0dl96bnJ5dTJvM1QiLCJhaWQiOiJ0Ml96bnJ5dT1vMjQiLCJsY2EiOjE3MTQ4MjU0NzQ3MDIsInNjcCI6ImVKeUtWaXBLVFV4UjBsRXFMazNLelN4UmlnVUVBQUpfX3pGR0JaMCIsImZsbyI6OX0.o3X9CJAUED1iYsFs8h_02NvaDMmPVSIaZgz3aPjEGm3zF5cG2-G2tU7yIJUtqGICxT0W3-PAso0jwrrx3ScSGucvhEiUVXOiGcCZSzPfLnwuGxtRa_lNEkrsLAVlhN8iXBRGds8YkJ0MFWn4JRwhi8beV3EsFkEzN6IsESuA33WUQQgGs0Ij5oH0If3EMLoBoDVQvWdp2Yno0SV9xdODP6pMJSKZD5HVgWGzprFlN2VWmgb4HXs3mrxbE5bcuO_slah0xcqnhcXmlYCdRCSqeEUtlW8pS4Wtzzs7BL5E70A5LHmHJfGJWCh-loInwarxeq_tVPoxikzqBrTIEsLmPA\"}",
        )];

        let (status, client) = join!(dummy_server(&req_resp), RedditClient::new());
        status.unwrap();
        client.unwrap();
    }

    #[tokio::test]
    async fn test_reply_to_comment() {
        let _lock = sequential();
        let consts = Consts::default();
        let mut client = RedditClient {
            client: Client::new(),
            token: Token {
                access_token: "token".to_string(),
                expiration_time: Utc::now(),
            },
        };
        let comment = Comment::new_already_replied(
            Meta {
                id: "t1_some_id".to_owned(),
                author: "author".to_owned(),
                subreddit: "subressit".to_owned(),
                thread: "t3_sdsd8e".to_owned(),
                used_commands: false,
            },
            MAX_COMMENT_LEN,
            "en",
        )
        .extract(&consts)
        .calc(&consts);
        let (status, reply_status) = join!(
            dummy_server(&[(
                "POST / HTTP/1.1\r\nauthorization: Bearer token\r\ncontent-type: application/x-www-form-urlencoded\r\naccept: */*\r\nhost: 127.0.0.1:9384\r\ncontent-length: 32\r\n\r\ntext=I+relpy&thing_id=t1_some_id",
                "HTTP/1.1 200 OK\r\nx-ratelimit-remaining: 10\r\nx-ratelimit-reset: 200\n\n{\"success\": true}"
            )]),
            client.reply_to_comment(&comment, "I relpy")
        );
        status.unwrap();
        let reply_status = reply_status.unwrap();
        assert_eq!(reply_status, Some((200.0, 10.0)));
    }

    #[tokio::test]
    async fn test_get_comments() {
        let _lock = sequential();
        let consts = Consts::default();
        let mut client = RedditClient {
            client: Client::new(),
            token: Token {
                access_token: "token".to_string(),
                expiration_time: Utc::now(),
            },
        };
        let _ = SUBREDDIT_COMMANDS.set(
            [
                (
                    "test_subreddit",
                    SubredditEntry {
                        locale: "en",
                        commands: Commands::TERMIAL,
                        mode: SubredditMode::All,
                    },
                ),
                (
                    "post_subreddit",
                    SubredditEntry {
                        locale: "en",
                        commands: Commands::NONE,
                        mode: SubredditMode::PostOnly,
                    },
                ),
            ]
            .into(),
        );
        let _ = COMMENT_COUNT.set(100);
        let mut already_replied = vec![];
        let mut last_ids = LastIds {
            comments: ("t1_m86nsre".to_owned(), "".to_owned()),
            posts: ("t3_83us27sa".to_owned(), "".to_owned()),
            mentions: ("".to_owned(), "".to_owned()),
        };
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
                    "HTTP/1.1 200 OK\r\nx-ratelimit-remaining: 8\r\nx-ratelimit-reset: 199\n\n{\"data\":{\"children\":[{\"kind\":\"t1\",\"data\":{\"name\": \"t1_m38msug\", \"subreddit\": \"test_subreddit\", \"author\":\"mentioner\",\"body\":\"u/factorion-bot !termial\",\"type\":\"username_mention\",\"parent_id\":\"t1_m38msum\"}}]}}"
                ),(
                    "GET /api/info?id=t1_m38msum HTTP/1.1\r\nauthorization: Bearer token\r\naccept: */*\r\nhost: 127.0.0.1:9384\r\n\r\n",
                    "HTTP/1.1 200 OK\r\nx-ratelimit-remaining: 7\r\nx-ratelimit-reset: 170\n\n{\"data\": {\"children\": [{\"kind\": \"t1\",\"data\":{\"name\":\"t1_m38msum\", \"subreddit\": \"post_subreddit\", \"body\":\"That's 57!?\"}}]}}"
                )]).await
            },
            client.get_comments(&mut already_replied, true, true, &mut last_ids)
        );
        status.unwrap();
        let (comments, rate) = comments.unwrap();
        let comments = comments
            .into_iter()
            .map(|c| c.extract(&consts).calc(&consts))
            .collect::<Vec<_>>();
        assert_eq!(comments.len(), 2);
        assert_eq!(comments[0].meta.id, "t1_m38msug");
        assert_eq!(comments[0].meta.author, "mentioner");
        assert_eq!(comments[0].notify.as_ref().unwrap(), "u/");
        assert_eq!(comments[0].commands, Commands::TERMIAL);
        assert_eq!(
            comments[0].calculation_list[0].steps,
            [(1, false), (-1, false)]
        );
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
                               "subreddit": "sub",
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
                               "subreddit": "sub",
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
                               "subreddit": "sub",
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
        let comments = RedditClient {
            client: Client::new(),
            token: Token {
                access_token: String::new(),
                expiration_time: Default::default(),
            },
        }
        .extract_comments(
            response,
            &mut already_replied,
            true,
            &HashMap::from([(
                "sub",
                SubredditEntry {
                    locale: "en",
                    commands: Commands::NONE,
                    mode: SubredditMode::All,
                },
            )]),
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
                    Commands::TERMIAL,
                    "Little_Tweetybird_".to_string(),
                )
            )]
        );
        println!("{comments:?}");
        assert_eq!(comments.2, Some((350.0, 10.0)));
    }

    #[tokio::test]
    async fn test_extract_posts() {
        let consts = Consts::default();
        let response = Response::from(http::Response::builder().status(200).header("X-Ratelimit-Remaining", "10").header("X-Ratelimit-Reset", "350").body(r#"{
               "data": {
                   "children": [
                       {
                           "kind": "t3",
                           "data": {
                               "author": "Little_Tweetybird_",
                               "author_fullname": "t2_b5n60qnt",
                               "subreddit": "sub",
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
                               "subreddit": "sub",
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
                               "subreddit": "sub",
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
        let (comments, _, t, id) = RedditClient {
            client: Client::new(),
            token: Token {
                access_token: String::new(),
                expiration_time: Default::default(),
            },
        }
        .extract_comments(
            response,
            &mut already_replied,
            false,
            &HashMap::from([(
                "sub",
                SubredditEntry {
                    locale: "en",
                    commands: Commands::NONE,
                    mode: SubredditMode::All,
                },
            )]),
            &HashMap::new(),
        )
        .await
        .unwrap();
        let comments = comments
            .into_iter()
            .map(|c| c.extract(&consts).calc(&consts))
            .collect::<Vec<_>>();
        assert_eq!(comments.len(), 3);
        assert_eq!(
            comments[0].calculation_list,
            [Calculation {
                value: Number::Exact(1.into()),
                steps: vec![(2, false)],
                result: CalculationResult::Exact(1.into())
            }]
        );
        assert_eq!(
            comments[1].calculation_list,
            [Calculation {
                value: Number::Exact(2.into()),
                steps: vec![(1, false)],
                result: CalculationResult::Exact(2.into())
            }]
        );
        assert_eq!(
            comments[2].calculation_list,
            [Calculation {
                value: Number::Exact(10.into()),
                steps: vec![(0, false)],
                result: CalculationResult::Exact(1334961.into())
            }]
        );
        println!("{comments:?}");
        assert_eq!(t, Some((350.0, 10.0)));
        assert_eq!(
            id.unwrap(),
            ("t3_m38msug".to_owned(), "t3_m38msum".to_owned())
        );
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
