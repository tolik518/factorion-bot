use dotenvy::dotenv;
use factorion_lib::{
    Consts,
    comment::{Commands, Comment, Formatting},
    locale::Locale,
};
use std::collections::HashMap;
use std::error::Error;
use std::io::Write;
use std::panic;
use factorion_lib::comment::{CommentCalculated, CommentConstructed, CommentExtracted};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    init();

    let consts = Consts {
        float_precision: factorion_lib::recommended::FLOAT_PRECISION,
        upper_calculation_limit: factorion_lib::recommended::UPPER_CALCULATION_LIMIT(),
        upper_approximation_limit: factorion_lib::recommended::UPPER_APPROXIMATION_LIMIT(),
        upper_subfactorial_limit: factorion_lib::recommended::UPPER_SUBFACTORIAL_LIMIT(),
        upper_termial_limit:factorion_lib::recommended::UPPER_TERMIAL_LIMIT(),
        upper_termial_approximation_limit: factorion_lib::recommended::UPPER_TERMIAL_APPROXIMATION_LIMIT,
        integer_construction_limit: factorion_lib::recommended::INTEGER_CONSTRUCTION_LIMIT(),
        number_decimals_scientific: 16,
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

    let args: Vec<String> = std::env::args().collect();
    let comment = args[1].clone();

    //let consts = Consts::default();
    let comment: CommentConstructed<&str> = Comment::new(&*comment, "meta", Commands::TERMIAL | Commands::NO_NOTE, 10_000, "en");
    let comment: CommentExtracted<&str> = comment.extract(&consts);
    let comment: CommentCalculated<&str> = comment.calc(&consts);

    let reply = comment.get_reply(&consts, Formatting::None);
    println!("{}", reply);

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

        println!("Thread panicked at {location} with message: {message}");
    }));
}