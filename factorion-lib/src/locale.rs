#![doc = include_str!("../Locales.md")]
use std::{borrow::Cow, collections::HashMap};

#[cfg(any(feature = "serde", test))]
use serde::{Deserialize, Serialize};

#[cfg(any(feature = "serde", test))]
pub fn get_en() -> Locale<'static> {
    serde_json::de::from_str(include_str!("en.json")).unwrap()
}
#[cfg(any(feature = "serde", test))]
pub fn get_en_fuck() -> Locale<'static> {
    serde_json::de::from_str(include_str!("en_fuck.json")).unwrap()
}
#[cfg(any(feature = "serde", test))]
pub fn get_de() -> Locale<'static> {
    serde_json::de::from_str(include_str!("de.json")).unwrap()
}
#[cfg(any(feature = "serde", test))]
pub fn get_ru() -> Locale<'static> {
    serde_json::de::from_str(include_str!("ru.json")).unwrap()
}
#[cfg(any(feature = "serde", test))]
pub fn get_it() -> Locale<'static> {
    serde_json::de::from_str(include_str!("it.json")).unwrap()
}
#[cfg(any(feature = "serde", test))]
pub fn get_all() -> impl Iterator<Item = (&'static str, Locale<'static>)> {
    [
        ("en", get_en()),
        ("en_fuck", get_en_fuck()),
        ("de", get_de()),
        ("ru", get_ru()),
        ("it", get_it()),
    ]
    .into_iter()
}

#[derive(Debug, Clone)]
#[cfg_attr(any(feature = "serde", test), derive(Serialize, Deserialize))]
pub struct Locale<'a> {
    pub bot_disclaimer: Cow<'a, str>,
    pub notes: Notes<'a>,
    pub format: Format<'a>,
}
#[derive(Debug, Clone)]
#[cfg_attr(any(feature = "serde", test), derive(Serialize, Deserialize))]
pub struct Notes<'a> {
    pub tower: Cow<'a, str>,
    pub tower_mult: Cow<'a, str>,
    pub digits: Cow<'a, str>,
    pub digits_mult: Cow<'a, str>,
    pub approx: Cow<'a, str>,
    pub approx_mult: Cow<'a, str>,
    pub round: Cow<'a, str>,
    pub round_mult: Cow<'a, str>,
    pub too_big: Cow<'a, str>,
    pub too_big_mult: Cow<'a, str>,
    pub remove: Cow<'a, str>,
    pub tetration: Cow<'a, str>,
    pub no_post: Cow<'a, str>,
    pub mention: Cow<'a, str>,
    pub limit_hit: Option<Cow<'a, str>>,
    pub write_out_unsupported: Option<Cow<'a, str>>,
}

#[derive(Debug, Clone)]
#[cfg_attr(any(feature = "serde", test), derive(Serialize, Deserialize))]
pub struct Format<'a> {
    pub capitalize_calc: bool,
    pub termial: Cow<'a, str>,
    pub factorial: Cow<'a, str>,
    pub uple: Cow<'a, str>,
    pub sub: Cow<'a, str>,
    pub negative: Cow<'a, str>,
    pub num_overrides: HashMap<i32, Cow<'a, str>>,
    pub force_num: bool,
    pub nest: Cow<'a, str>,
    pub rough_number: Cow<'a, str>,
    pub exact: Cow<'a, str>,
    pub rough: Cow<'a, str>,
    pub approx: Cow<'a, str>,
    pub digits: Cow<'a, str>,
    pub order: Cow<'a, str>,
    pub all_that: Cow<'a, str>,
    pub number_format: NumFormat,
}

#[derive(Debug, Clone)]
#[cfg_attr(any(feature = "serde", test), derive(Serialize, Deserialize))]
pub struct NumFormat {
    pub decimal: char,
}
