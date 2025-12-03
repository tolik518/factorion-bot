#![doc = include_str!("../README.md")]
use dotenvy::dotenv;
use factorion_lib::{
    Consts,
    comment::{Commands, Comment, Status},
    influxdb::INFLUX_CLIENT,
    locale::Locale,
    rug::{Complete, Integer, integer::IntegerExt64},
};
use log::{error, info, warn};
use reddit_api::RedditClient;
use reddit_api::id::DenseId;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::error::Error;
use std::fs::OpenOptions;
use std::io::Write;
use std::panic;
use std::sync::OnceLock;
use std::time::SystemTime;
use tokio::time::{Duration, sleep};

use crate::reddit_api::Thread;

mod reddit_api;

const API_COMMENT_COUNT: u32 = 100;
const ALREADY_REPLIED_IDS_FILE_PATH: &str = "already_replied_ids.dat";
const MAX_ALREADY_REPLIED_LEN: usize = 100_000;
const THREAD_CALCS_FILE_PATH: &str = "thread_calcs.dat";
const MAX_THREAD_CALCS_LEN: usize = 100;
const MAX_REPETITIONS_PER_THREAD: usize = 10;
static COMMENT_COUNT: OnceLock<u32> = OnceLock::new();
static SUBREDDIT_COMMANDS: OnceLock<HashMap<&str, SubredditEntry>> = OnceLock::new();

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
struct SubredditEntry {
    #[serde(default = "en_str")]
    locale: &'static str,
    #[serde(default = "Commands::default")]
    commands: Commands,
}
fn en_str() -> &'static str {
    "en"
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    init();

    let consts = Consts {
        float_precision: std::env::var("FLOAT_PRECISION")
            .map(|s| s.parse().unwrap())
            .unwrap_or_else(|_| factorion_lib::recommended::FLOAT_PRECISION),
        upper_calculation_limit: std::env::var("UPPER_CALCULATION_LIMIT")
            .map(|s| s.parse().unwrap())
            .unwrap_or_else(|_| factorion_lib::recommended::UPPER_CALCULATION_LIMIT()),
        upper_approximation_limit: std::env::var("UPPER_APPROXIMATION_LIMIT")
            .map(|s| Integer::u64_pow_u64(10, s.parse().unwrap()).complete())
            .unwrap_or_else(|_| factorion_lib::recommended::UPPER_APPROXIMATION_LIMIT()),
        upper_subfactorial_limit: std::env::var("UPPER_SUBFACTORIAL_LIMIT")
            .map(|s| s.parse().unwrap())
            .unwrap_or_else(|_| factorion_lib::recommended::UPPER_SUBFACTORIAL_LIMIT()),
        upper_termial_limit: std::env::var("UPPER_TERMIAL_LIMIT")
            .map(|s| Integer::u64_pow_u64(10, s.parse().unwrap()).complete())
            .unwrap_or_else(|_| factorion_lib::recommended::UPPER_TERMIAL_LIMIT()),
        upper_termial_approximation_limit: std::env::var("UPPER_TERMIAL_APPROXIMATION_LIMIT")
            .map(|s| s.parse().unwrap())
            .unwrap_or_else(|_| factorion_lib::recommended::UPPER_TERMIAL_APPROXIMATION_LIMIT),
        integer_construction_limit: std::env::var("INTEGER_CONSTRUCTION_LIMIT")
            .map(|s| s.parse().unwrap())
            .unwrap_or_else(|_| factorion_lib::recommended::INTEGER_CONSTRUCTION_LIMIT()),
        number_decimals_scientific: std::env::var("NUMBER_DECIMALS_SCIENTIFIC")
            .map(|s| s.parse().unwrap())
            .unwrap_or_else(|_| factorion_lib::recommended::NUMBER_DECIMALS_SCIENTIFIC),
        locales: std::env::var("LOCALES_DIR")
            .map(|dir| {
                let files = std::fs::read_dir(dir).unwrap();
                let mut map = HashMap::new();
                for (key, value) in files
                    .map(|file| {
                        let file = file.unwrap();
                        let locale: Locale<'static> = serde_json::de::from_str(
                            std::fs::read_to_string(file.path()).unwrap().leak(),
                        )
                        .unwrap();
                        (file.file_name().into_string().unwrap(), locale)
                    })
                    .collect::<Box<_>>()
                {
                    map.insert(key, value);
                }
                map
            })
            .unwrap_or_else(|_| {
                factorion_lib::locale::get_all()
                    .map(|(k, v)| (k.to_owned(), v))
                    .into()
            }),
        default_locale: "en".to_owned(),
    };

    let influx_client = &*INFLUX_CLIENT;

    if influx_client.is_none() {
        warn!("InfluxDB client not configured. No influxdb metrics will be logged.");
    } else {
        info!("InfluxDB client configured. Metrics will be logged.");
    }

    let mut reddit_client = RedditClient::new().await?;
    COMMENT_COUNT.set(API_COMMENT_COUNT).unwrap();
    let mut requests_per_loop = 0.0;

    let dont_reply = std::env::var("DONT_REPLY").unwrap_or_default();
    let dont_reply = dont_reply == "true";

    let sub_entries = if let Ok(path) = std::env::var("SUBREDDITS_FILE") {
        if let Ok(_) = std::env::var("SUBREDDITS") {
            panic!("SUBREDDITS and SUBREDDITS_FILE can not be set simultaneusly!")
        }
        let text = std::fs::read_to_string(path).unwrap();
        serde_json::de::from_str(text.leak()).expect("Subreddits File has invalid format")
    } else {
        let subreddit_commands = std::env::var("SUBREDDITS").unwrap_or_default();
        let subreddit_commands = subreddit_commands.leak();
        subreddit_commands
            .split('+')
            .map(|s| s.split_once(':').expect("Locale is unset"))
            .map(|(s, r)| (s, r.split_once(':').unwrap_or((r, ""))))
            .map(|(s, (l, c))| (s, if l.is_empty() { "en" } else { l }, c))
            .filter(|s| !(s.0.is_empty() && s.1.is_empty()))
            .map(|(sub, locale, commands)| {
                (
                    sub,
                    SubredditEntry {
                        locale,
                        commands: commands
                            .split(',')
                            .map(|command| match command.trim() {
                                "shorten" => Commands::SHORTEN,
                                "termial" => Commands::TERMIAL,
                                "steps" => Commands::STEPS,
                                "no_note" => Commands::NO_NOTE,
                                "post_only" => Commands::POST_ONLY,
                                "dont_check" => Commands::DONT_CHECK,
                                "" => Commands::NONE,
                                s => panic!("Unknown command in subreddit {sub}: {s}"),
                            })
                            .fold(Commands::NONE, |a, e| a | e),
                    },
                )
            })
            .collect::<HashMap<_, _>>()
    };
    if !sub_entries.is_empty() {
        requests_per_loop += 1.0;
        if !sub_entries.values().all(|v| v.commands.post_only) {
            requests_per_loop += 1.0;
        }
    }
    SUBREDDIT_COMMANDS.set(sub_entries).unwrap();

    let check_mentions = std::env::var("CHECK_MENTIONS").expect("CHECK_MENTIONS must be set");
    let check_mentions = check_mentions == "true";
    if check_mentions {
        requests_per_loop += 1.0;
    }
    let check_posts = std::env::var("CHECK_POSTS").expect("CHECK_POSTS must be set");
    let check_posts = check_posts == "true";

    let posts_every = std::env::var("POSTS_EVERY").unwrap_or("1".to_owned());
    let posts_every: u8 = posts_every.parse().expect("POSTS_EVERY is not a number");
    let mentions_every = std::env::var("MENTIONS_EVERY").unwrap_or("1".to_owned());
    let mentions_every: u8 = mentions_every
        .parse()
        .expect("MENTIONS_EVERY is not a number");

    // read comment_ids from the file
    let mut already_replied_or_rejected: Vec<DenseId> = read_comment_ids();
    if already_replied_or_rejected.is_empty() {
        info!("No comment_ids found in the file");
    } else {
        info!("Found comment_ids in the file");
    }
    let mut last_ids = Default::default();

    let mut thread_calcs: Vec<Thread> = read_thread_calcs();
    if thread_calcs.is_empty() {
        info!("No comment_ids found in the file");
    } else {
        info!("Found comment_ids in the file");
    }

    // Polling Reddit for new comments
    for i in (0..u8::MAX).cycle() {
        info!("Polling Reddit for new comments...");
        let mut thread_calcs_changed = false;

        let start = SystemTime::now();
        // force checking of "old" messages ca. every 15 minutes
        if i == 0 {
            last_ids = Default::default();
        }
        let (comments, mut rate) = reddit_client
            .get_comments(
                &mut already_replied_or_rejected,
                check_mentions && i % mentions_every == 0,
                check_posts && i % posts_every == 0,
                &mut last_ids,
            )
            .await
            .unwrap_or((Default::default(), (60.0, 0.0)));
        let end = SystemTime::now();

        factorion_lib::influxdb::reddit::log_time_consumed(
            influx_client,
            start,
            end,
            "get_comments",
        )
        .await?;

        let start = SystemTime::now();
        let mut comments = comments
            .into_iter()
            .filter_map(|c| {
                let id = c.meta.id.clone();
                match std::panic::catch_unwind(|| Comment::extract(c, &consts)) {
                    Ok(c) => Some(c),
                    Err(_) => {
                        error!("Failed to calculate comment {id}!");
                        None
                    }
                }
            })
            .collect::<Vec<_>>();
        let end = SystemTime::now();

        factorion_lib::influxdb::reddit::log_time_consumed(
            influx_client,
            start,
            end,
            "extract_factorials",
        )
        .await?;

        // Remove repetitive calcs
        for comment in &mut comments {
            if comment.meta.used_commands || comment.calculation_list.is_empty() {
                continue;
            }
            let Ok(mut dense_id) = u64::from_str_radix(&comment.meta.thread, 36) else {
                warn!("Failed to make id dense {}", comment.meta.thread);
                continue;
            };
            dense_id |= 3 << 61;
            let dense_id = DenseId::from_raw(dense_id);
            let thread = thread_calcs
                .iter()
                .enumerate()
                .find_map(|(i, x)| (x.id == dense_id).then_some(i))
                .unwrap_or_else(|| {
                    thread_calcs.push(Thread {
                        id: dense_id,
                        calcs: vec![],
                    });
                    thread_calcs.len() - 1
                });
            let mut thread = thread_calcs.remove(thread);
            comment.calculation_list.retain(|calc| {
                thread
                    .calcs
                    .iter_mut()
                    .find(|(c, _)| c == calc)
                    .map(|(_, n)| {
                        *n += 1;
                        *n < MAX_REPETITIONS_PER_THREAD
                    })
                    .unwrap_or(true)
            });
            comment.status.limit_hit = comment.calculation_list.iter().any(|calc| {
                thread
                    .calcs
                    .iter()
                    .any(|(c, n)| c == calc && *n + 1 == MAX_REPETITIONS_PER_THREAD)
            });

            thread
                .calcs
                .extend(comment.calculation_list.iter().map(|x| (x.clone(), 0)));
            thread.calcs.sort_unstable();
            thread.calcs.reverse();
            thread.calcs.dedup_by(|a, b| a.0 == b.0);

            thread_calcs.push(thread);
            thread_calcs_changed = true;
        }

        let start = SystemTime::now();
        let comments = comments
            .into_iter()
            .filter_map(|c| {
                let id = c.meta.id.clone();
                match std::panic::catch_unwind(|| Comment::calc(c, &consts)) {
                    Ok(c) => Some(c),
                    Err(_) => {
                        error!("Failed to calculate comment {id}!");
                        None
                    }
                }
            })
            .collect::<Vec<_>>();
        let end = SystemTime::now();

        factorion_lib::influxdb::reddit::log_time_consumed(
            influx_client,
            start,
            end,
            "calculate_factorials",
        )
        .await?;

        if already_replied_or_rejected.len() > MAX_ALREADY_REPLIED_LEN {
            let extra = already_replied_or_rejected.len() - MAX_ALREADY_REPLIED_LEN;
            already_replied_or_rejected.drain(..extra);
        }

        write_comment_ids(&already_replied_or_rejected);

        if thread_calcs.len() > MAX_THREAD_CALCS_LEN {
            let extra = thread_calcs.len() - MAX_THREAD_CALCS_LEN;
            thread_calcs.drain(..extra);
            thread_calcs_changed = true;
        }

        if thread_calcs_changed {
            write_thread_calcs(&thread_calcs);
        }

        let start = SystemTime::now();
        for comment in comments {
            let comment_id = &comment.meta.id;
            let comment_author = &comment.meta.author;
            let comment_subreddit = &comment.meta.subreddit;
            let comment_locale = &comment.locale;

            let status: Status = comment.status;
            let should_answer = status.factorials_found && status.not_replied;

            if status.no_factorial && !status.number_too_big_to_calculate {
                continue;
            }

            if status.factorials_found {
                info!("Comment -> {comment:?}");
            }

            if should_answer {
                let Ok(reply): Result<String, _> =
                    std::panic::catch_unwind(|| comment.get_reply(&consts))
                else {
                    error!("Failed to format comment!");
                    continue;
                };
                // Sleep to not spam comments too quickly
                let pause = if rate.1 < 1.0 {
                    rate.0 + 5.0
                } else if rate.1 < 4.0 {
                    rate.0 / rate.1 + 2.0
                } else {
                    2.0
                };
                sleep(Duration::from_secs(pause as u64)).await;
                if !dont_reply {
                    'reply: loop {
                        match reddit_client.reply_to_comment(&comment, &reply).await {
                            Ok(t) => {
                                if let Some(t) = t {
                                    rate = t;
                                } else {
                                    warn!("Missing ratelimit");
                                }
                                factorion_lib::influxdb::reddit::log_comment_reply(
                                    influx_client,
                                    &comment_id,
                                    &comment_author,
                                    &comment_subreddit,
                                    &comment_locale,
                                )
                                .await?;
                            }
                            Err(e) => match e.downcast::<reddit_api::RateLimitErr>() {
                                Ok(_) => {
                                    error!("Hit the ratelimit!");
                                    sleep(Duration::from_secs(rate.0.ceil() as u64)).await;
                                    continue 'reply;
                                }
                                Err(e) => error!("Failed to reply to comment: {e:?}"),
                            },
                        }
                        break 'reply;
                    }
                }
                continue;
            }
            info!(" -> unknown");
        }
        let end = SystemTime::now();

        factorion_lib::influxdb::reddit::log_time_consumed(
            influx_client,
            start,
            end,
            "comment_loop",
        )
        .await?;

        let sleep_between_requests = if rate.1 < requests_per_loop + 1.0 {
            rate.0 + 1.0
        } else {
            (rate.0 / rate.1 * requests_per_loop).max(2.0) + 1.0
        };
        // Sleep to avoid hitting API rate limits
        sleep(Duration::from_secs(sleep_between_requests.ceil() as u64)).await;
    }
    Ok(())
}

fn init() {
    dotenv().ok();
    env_logger::builder()
        .format(|buf, record| {
            let style = buf.default_level_style(record.level());
            writeln!(
                buf,
                "{style}{} | {} | {} | {}",
                record.level(),
                record.target(),
                buf.timestamp(),
                record.args()
            )
        })
        .init();

    panic::set_hook(Box::new(|panic_info| {
        let location = panic_info
            .location()
            .map(|l| format!("{}:{}", l.file(), l.line()))
            .unwrap_or_else(|| "unknown location".to_string());

        let message = panic_info
            .payload()
            .downcast_ref::<&str>()
            .map(|s| s.to_string())
            .or_else(|| panic_info.payload().downcast_ref::<String>().cloned())
            .unwrap_or_else(|| format!("Unknown panic payload: {panic_info:?}"));

        error!("Thread panicked at {location} with message: {message}");
    }));
}

fn write_comment_ids(already_replied_or_rejected: &[DenseId]) {
    let mut file = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(ALREADY_REPLIED_IDS_FILE_PATH)
        .expect("Unable to open or create file");

    let raw = already_replied_or_rejected
        .iter()
        .flat_map(|id| id.raw().to_le_bytes())
        .collect::<Vec<_>>();

    file.write_all(&raw).expect("Unable to write to file");
}
fn read_comment_ids() -> Vec<DenseId> {
    let raw = std::fs::read(ALREADY_REPLIED_IDS_FILE_PATH).unwrap_or_default();
    const DENSE_SIZE: usize = std::mem::size_of::<DenseId>();
    // TODO(optimize): use `as_chunks` if available (1.88.0 and up)
    raw.chunks_exact(DENSE_SIZE)
        .map(|bytes| DenseId::from_raw(u64::from_le_bytes(bytes.try_into().unwrap())))
        .collect()
}

fn write_thread_calcs(thread_calcs: &[Thread]) {
    let file = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(THREAD_CALCS_FILE_PATH)
        .expect("Unable to open or create file");

    postcard::to_io(thread_calcs, file).unwrap();
}

fn read_thread_calcs() -> Vec<Thread> {
    if !std::fs::exists(THREAD_CALCS_FILE_PATH).expect("Unable to check for file") {
        return Vec::new();
    }
    let file = std::fs::read(THREAD_CALCS_FILE_PATH).expect("Unable to read file");
    postcard::from_bytes(&file).expect("Malformed thread_calcs file")
}
