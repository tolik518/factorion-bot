use dotenv::dotenv;
use reqwest::{Client, Response};
use reqwest::header::{AUTHORIZATION, CONTENT_TYPE, HeaderMap, USER_AGENT};
use serde::Deserialize;
use serde_json::json;

#[derive(Deserialize, Debug)]
struct TokenResponse {
    access_token: String,
}

const REDDIT_TOKEN_URL: &str = "https://ssl.reddit.com/api/v1/access_token";
const REDDIT_COMMENT_URL: &str = "https://oauth.reddit.com/api/comment";

pub(crate) struct RedditClient {
    client: Client,
    token: String,
    user_agent: String,
}

impl RedditClient {
    pub(crate) async fn new() -> Result<Self, Box<dyn std::error::Error>> {
        dotenv().ok(); // Loads the environment variables from the ".env" file.
        let client_id = std::env::var("APP_CLIENT_ID").expect("APP_CLIENT_ID must be set.");
        let secret = std::env::var("APP_SECRET").expect("APP_SECRET must be set.");

        //TODO: get token every 24 hours
        let token = format!("bearer {}", RedditClient::get_reddit_token(client_id, secret).await?);
        let user_agent = format!("factorion-bot:v{} (by /u/tolik518)", env!("CARGO_PKG_VERSION"));

        let mut headers = HeaderMap::new();
        headers.insert(AUTHORIZATION, token.parse()?);
        headers.insert(USER_AGENT, user_agent.parse()?);

        let client = Client::builder()
            .default_headers(headers)
            .build()?;

        Ok(Self {
            client,
            token,
            user_agent,
        })
    }

    pub(crate) async fn get_comments(&self, subreddit: &str, limit: u32) -> Result<Response, reqwest::Error> {
        let response = self.client
            .get(&format!("https://oauth.reddit.com/r/{}/comments/?limit={}", subreddit, limit))
            .send()
            .await?;


        Ok(response)
    }

    pub(crate) async fn get_comment_children(&self, comment_id: &str, limit: u32) -> Result<Response, reqwest::Error> {
        let response = self.client
            .get(&format!("https://oauth.reddit.com/api/info/?id={}&limit={}", comment_id, limit))
            .send()
            .await?;

        Ok(response)
    }

    pub(crate) async fn reply_to_comment(&self, comment: &serde_json::Value, reply: &str) -> Result<(), reqwest::Error> {
        let comment_id = comment["data"]["id"].as_str().unwrap();
        let params = json!({
            "thing_id": format!("t1_{}", comment_id),
            "text": reply
        });

        println!("Replying to comment t1_{}", comment_id);

        let response = self.client
            .post(REDDIT_COMMENT_URL)
            .form(&params)
            .send()
            .await?;

        if !response.status().is_success() {
            println!("Failed to reply to comment: {:#?}", response);
        } else {
            println!("Response Status: {}", response.status());
            println!("Response Body: {:#?}", response.text().await);
        }

        Ok(())
    }

    async fn get_reddit_token(client_id: String, client_secret: String) -> Result<String, Box<dyn std::error::Error>> {
        let password = std::env::var("REDDIT_PASSWORD").expect("REDDIT_PASSWORD must be set.");
        let username = std::env::var("REDDIT_USERNAME").expect("REDDIT_USERNAME must be set.");

        let auth_value = format!("Basic {}", base64::encode(format!("{}:{}", client_id, client_secret)));
        let version = env!("CARGO_PKG_VERSION");
        let user_agent = format!("factorion-bot:v{version} (by /u/tolik518)");

        let mut headers = HeaderMap::new();
        headers.insert(AUTHORIZATION, auth_value.parse()?);
        headers.insert(USER_AGENT, user_agent.parse()?);
        headers.insert(CONTENT_TYPE, "application/x-www-form-urlencoded".parse()?);

        let params = [
            ("grant_type", "password"),
            ("username", username.as_str()),
            ("password", password.as_str()),
            ("scope", "read submit")
        ];

        let response = Client::new().post(REDDIT_TOKEN_URL)
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
}
