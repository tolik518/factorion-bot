use serde::{Deserialize, Serialize};
use std::{borrow::Cow, collections::HashMap};

#[derive(Deserialize, Serialize)]
pub struct Locale<'a> {
    pub bot_disclaimer: Cow<'a, str>,
    pub notes: Notes<'a>,
    pub format: Format<'a>,
    pub number_format: NumFormat,
}

#[derive(Deserialize, Serialize)]
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
}

#[derive(Deserialize, Serialize)]
pub struct Format<'a> {
    pub termial: Cow<'a, str>,
    pub factorial: Cow<'a, str>,
    pub uple: Cow<'a, str>,
    pub sub: Cow<'a, str>,
    pub num_overrides: HashMap<usize, Cow<'a, str>>,
    pub rough_number: Cow<'a, str>,
    pub exact: Cow<'a, str>,
    pub rough: Cow<'a, str>,
    pub approx: Cow<'a, str>,
    pub digits: Cow<'a, str>,
    pub order: Cow<'a, str>,
    pub all_that: Cow<'a, str>,
}

#[derive(Deserialize, Serialize)]
pub struct NumFormat {
    pub decimal: char,
}
