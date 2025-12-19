#![doc = include_str!("../README.md")]
use dotenvy::dotenv;
use factorion_lib::Consts;
use factorion_lib::influxdb::INFLUX_CLIENT;
use factorion_lib::locale::Locale;
use factorion_lib::rug::integer::IntegerExt64;
use factorion_lib::rug::{Complete, Integer};
use log::{error, info, warn};
use std::collections::HashMap;
use std::error::Error;
use std::panic;

mod discord_api;

fn init() {
    dotenv().ok();
    env_logger::builder()
        .format(|buf, record| {
            use std::io::Write;
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

    info!("factorion-lib initialized successfully");
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
                        let mut locale: Locale<'static> = serde_json::de::from_str(
                            std::fs::read_to_string(file.path()).unwrap().leak(),
                        )
                        .unwrap();
                        locale.set_bot_disclaimer("".into());
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
                    .map(|(k, mut v)| {
                        v.set_bot_disclaimer("".into());
                        (k.to_owned(), v)
                    })
                    .into()
            }),
        default_locale: "en".to_owned(),
    };

    let token = std::env::var("DISCORD_TOKEN").expect("DISCORD_TOKEN must be set in environment");

    info!("Starting Discord bot...");

    if INFLUX_CLIENT.is_none() {
        warn!("InfluxDB client not configured. No influxdb metrics will be logged.");
    } else {
        info!("InfluxDB client configured. Metrics will be logged.");
    }

    discord_api::start_bot(token, consts, &*INFLUX_CLIENT).await?;

    Ok(())
}
