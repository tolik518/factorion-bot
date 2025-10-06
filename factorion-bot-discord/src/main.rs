#![doc = include_str!("../README.md")]
use dotenvy::dotenv;
use log::{error, info};
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

    factorion_lib::init(
        std::env::var("FLOAT_PRECISION")
            .unwrap_or("1000".to_owned())
            .parse()
            .expect("FLOAT_PRECISION is not a valid number"),
        std::env::var("UPPER_CALCULATION_LIMIT")
            .unwrap_or("3000".to_owned())
            .parse()
            .expect("UPPER_CALCULATION_LIMIT is not a valid number"),
        std::env::var("UPPER_APPROXIMATION_LIMIT")
            .unwrap_or("1000000".to_owned())
            .parse()
            .expect("UPPER_APPROXIMATION_LIMIT is not a valid number"),
        std::env::var("UPPER_SUBFACTORIAL_LIMIT")
            .unwrap_or("100000".to_owned())
            .parse()
            .expect("UPPER_SUBFACTORIAL_LIMIT is not a valid number"),
        std::env::var("UPPER_TERMIAL_LIMIT")
            .unwrap_or("100000".to_owned())
            .parse()
            .expect("UPPER_TERMIAL_LIMIT is not a valid number"),
        std::env::var("UPPER_TERMIAL_APPROXIMATION_LIMIT")
            .unwrap_or("1000000".to_owned())
            .parse()
            .expect("UPPER_TERMIAL_APPROXIMATION_LIMIT is not a valid number"),
        std::env::var("INTEGER_CONSTRUCTION_LIMIT")
            .unwrap_or("100000".to_owned())
            .parse()
            .expect("INTEGER_CONSTRUCTION_LIMIT is not a valid number"),
        std::env::var("NUMBER_DECIMALS_SCIENTIFIC")
            .unwrap_or("5".to_owned())
            .parse()
            .expect("NUMBER_DECIMALS_SCIENTIFIC is not a valid number"),
    )
    .expect("Failed to initialize factorion-lib");

    info!("factorion-lib initialized successfully");
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    init();

    let token = std::env::var("DISCORD_TOKEN").expect("DISCORD_TOKEN must be set in environment");

    info!("Starting Discord bot...");

    discord_api::start_bot(token).await?;

    Ok(())
}
