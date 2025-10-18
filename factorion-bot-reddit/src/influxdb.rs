use chrono::{DateTime, Utc};
use influxdb::{Client as InfluxDbClient, Error as InfluxDbError, InfluxDbWriteable};
use std::{sync::LazyLock, time::SystemTime};

pub const SOURCE: &str = "reddit";

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
pub struct CommentMeasurement {
    pub time: DateTime<Utc>,
    pub comment_id: String,
    #[influxdb(tag)]
    pub author: String,
    #[influxdb(tag)]
    pub subreddit: String,
    #[influxdb(tag)]
    pub language: String,
    #[influxdb(tag)]
    pub source: String,
}

pub async fn log_comment_reply(
    influx_client: &Option<InfluxDbClient>,
    comment_id: &str,
    author: &str,
    subreddit: &str,
    language: &str,
) -> Result<(), InfluxDbError> {
    if let Some(influx_client) = influx_client {
        influx_client
            .query(vec![
                CommentMeasurement {
                    time: Utc::now(),
                    comment_id: comment_id.to_string(),
                    author: author.to_string(),
                    subreddit: subreddit.to_string(),
                    language: language.to_string(),
                    source: SOURCE.to_string(),
                }
                .into_query("replied_to_comment"),
            ])
            .await?;
    }
    Ok(())
}

pub async fn log_time_consumed(
    influx_client: &Option<InfluxDbClient>,
    start: SystemTime,
    end: SystemTime,
    metric_name: &str,
) -> Result<(), InfluxDbError> {
    if let Some(influx_client) = influx_client {
        influx_client
            .query(vec![
                TimeMeasurement {
                    time: Utc::now(),
                    time_consumed: end.duration_since(start).unwrap().as_secs_f64(),
                    source: SOURCE.to_string(),
                }
                .into_query(metric_name),
            ])
            .await?;
    }
    Ok(())
}
