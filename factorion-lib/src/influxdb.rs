use chrono::{DateTime, Utc};
pub use influxdb::{Client as InfluxDbClient, Error as InfluxDbError, InfluxDbWriteable};
use std::{sync::LazyLock, time::SystemTime};

/// Initialize the InfluxDB client from environment variables
pub static INFLUX_CLIENT: LazyLock<Option<InfluxDbClient>> = LazyLock::new(|| {
    let host = std::env::var("INFLUXDB_HOST").ok()?;
    let bucket = std::env::var("INFLUXDB_BUCKET").ok()?;
    let token = std::env::var("INFLUXDB_TOKEN").ok()?;
    Some(InfluxDbClient::new(host, bucket).with_token(token))
});

#[derive(InfluxDbWriteable)]
pub struct TimeMeasurement {
    pub time: DateTime<Utc>,
    pub time_consumed: f64,
    #[influxdb(tag)]
    pub source: String,
}

#[derive(InfluxDbWriteable)]
pub struct ReplyMeasurement {
    pub time: DateTime<Utc>,
    pub item_id: String,
    #[influxdb(tag)]
    pub author: String,
    #[influxdb(tag)]
    pub location: String,
    #[influxdb(tag)]
    pub language: String,
    #[influxdb(tag)]
    pub source: String,
}

/// Log a reply to a comment/message
pub async fn log_reply(
    influx_client: &Option<InfluxDbClient>,
    item_id: &str,
    author: &str,
    location: &str,
    language: &str,
    source: &str,
    metric_name: &str,
) -> Result<(), InfluxDbError> {
    if let Some(influx_client) = influx_client {
        influx_client
            .query(vec![
                ReplyMeasurement {
                    time: Utc::now(),
                    item_id: item_id.to_string(),
                    author: author.to_string(),
                    location: location.to_string(),
                    language: language.to_string(),
                    source: source.to_string(),
                }
                .into_query(metric_name),
            ])
            .await?;
    }
    Ok(())
}

/// Log time consumed for a particular operation
pub async fn log_time_consumed(
    influx_client: &Option<InfluxDbClient>,
    start: SystemTime,
    end: SystemTime,
    source: &str,
    metric_name: &str,
) -> Result<(), InfluxDbError> {
    if let Some(influx_client) = influx_client {
        influx_client
            .query(vec![
                TimeMeasurement {
                    time: Utc::now(),
                    time_consumed: end.duration_since(start).unwrap().as_secs_f64(),
                    source: source.to_string(),
                }
                .into_query(metric_name),
            ])
            .await?;
    }
    Ok(())
}

// Reddit-specific functions
pub mod reddit {
    use super::*;
    
    const SOURCE: &str = "reddit";

    /// Log a reply to a Reddit comment
    pub async fn log_comment_reply(
        influx_client: &Option<InfluxDbClient>,
        comment_id: &str,
        author: &str,
        subreddit: &str,
        language: &str,
    ) -> Result<(), InfluxDbError> {
        super::log_reply(
            influx_client,
            comment_id,
            author,
            subreddit,
            language,
            SOURCE,
            "replied_to_comment",
        )
        .await
    }

    /// Log time consumed for an operation
    pub async fn log_time_consumed(
        influx_client: &Option<InfluxDbClient>,
        start: SystemTime,
        end: SystemTime,
        metric_name: &str,
    ) -> Result<(), InfluxDbError> {
        super::log_time_consumed(influx_client, start, end, SOURCE, metric_name).await
    }
}

// Discord-specific functions
pub mod discord {
    use super::*;
    
    const SOURCE: &str = "discord";

    /// Log a reply to a Discord message
    pub async fn log_message_reply(
        influx_client: &Option<InfluxDbClient>,
        message_id: &str,
        author: &str,
        channel: &str,
        language: &str,
    ) -> Result<(), InfluxDbError> {
        super::log_reply(
            influx_client,
            message_id,
            author,
            channel,
            language,
            SOURCE,
            "replied_to_message",
        )
        .await
    }

    /// Log time consumed for an operation
    pub async fn log_time_consumed(
        influx_client: &Option<InfluxDbClient>,
        start: SystemTime,
        end: SystemTime,
        metric_name: &str,
    ) -> Result<(), InfluxDbError> {
        super::log_time_consumed(influx_client, start, end, SOURCE, metric_name).await
    }
}
