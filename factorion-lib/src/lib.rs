#![doc = include_str!("../README.md")]

use std::collections::HashMap;

use factorion_math as math;
use rug::Integer;
pub mod calculation_results;
pub mod calculation_tasks;
pub mod comment;
pub mod locale;
pub mod parse;
/// The result of a calculation
pub use calculation_results::Calculation;
/// The format prepped for calculation
pub use calculation_tasks::CalculationJob;
/// Convenient abstraction for comments with commands
pub use comment::{Commands, Comment};
/// The version of rug we use (for convenience)
pub use factorion_math::rug;
/// The parser
pub use parse::parse;

use crate::locale::Locale;

pub mod recommended {
    pub use crate::calculation_results::recommended::*;
    pub use crate::calculation_tasks::recommended::*;
    pub use crate::parse::recommended::*;
    pub use factorion_math::recommended::FLOAT_PRECISION;
}

pub struct Consts<'a> {
    pub float_precision: u32,
    pub upper_calculation_limit: Integer,
    pub upper_approximation_limit: Integer,
    pub upper_subfactorial_limit: Integer,
    pub upper_termial_limit: Integer,
    pub upper_termial_approximation_limit: u32,
    pub integer_construction_limit: Integer,
    pub number_decimals_scientific: usize,
    pub locales: HashMap<String, Locale<'a>>,
    pub default_locale: String,
}
impl Default for Consts<'_> {
    fn default() -> Self {
        Consts {
            float_precision: math::recommended::FLOAT_PRECISION,
            upper_calculation_limit: calculation_tasks::recommended::UPPER_CALCULATION_LIMIT(),
            upper_approximation_limit: calculation_tasks::recommended::UPPER_APPROXIMATION_LIMIT(),
            upper_subfactorial_limit: calculation_tasks::recommended::UPPER_SUBFACTORIAL_LIMIT(),
            upper_termial_limit: calculation_tasks::recommended::UPPER_TERMIAL_LIMIT(),
            upper_termial_approximation_limit:
                calculation_tasks::recommended::UPPER_TERMIAL_APPROXIMATION_LIMIT,
            integer_construction_limit: parse::recommended::INTEGER_CONSTRUCTION_LIMIT(),
            number_decimals_scientific:
                calculation_results::recommended::NUMBER_DECIMALS_SCIENTIFIC,
            locales: HashMap::from([
                ("en".to_owned(), locale::get_en()),
                ("de".to_owned(), locale::get_de()),
            ]),
            default_locale: "en".to_owned(),
        }
    }
}
