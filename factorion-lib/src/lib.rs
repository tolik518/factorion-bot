#![doc = include_str!("../README.md")]
use std::sync::{Mutex, OnceLock};

use factorion_math as math;
use rug::Integer;
pub mod calculation_results;
pub mod calculation_tasks;
pub mod comment;
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

static FLOAT_PRECISION: OnceLock<u32> = OnceLock::new();
/// Recommended values for [`init`]
pub mod recommended {
    pub use super::math::recommended::FLOAT_PRECISION;
    pub use crate::calculation_results::recommended::*;
    pub use crate::calculation_tasks::recommended::*;
    pub use crate::parse::recommended::*;
}

static INITIALIZING: Mutex<()> = Mutex::new(());
#[derive(Debug, Clone, Copy)]
pub struct AlreadyInit;
#[allow(clippy::too_many_arguments)]
pub fn init(
    float_precision: u32,
    upper_calculation_limit: Integer,
    upper_approximation_limit: Integer,
    upper_subfactorial_limit: Integer,
    upper_termial_limit: Integer,
    upper_termial_approximation_limit: u32,
    integer_construction_limit: Integer,
    number_decimals_scientific: usize,
) -> Result<(), AlreadyInit> {
    let _guard = INITIALIZING.lock().unwrap();
    FLOAT_PRECISION
        .set(float_precision)
        .map_err(|_| AlreadyInit)?;
    parse::init(integer_construction_limit)?;
    calculation_tasks::init(
        upper_calculation_limit,
        upper_approximation_limit,
        upper_subfactorial_limit,
        upper_termial_limit,
        upper_termial_approximation_limit,
    )?;
    calculation_results::init(number_decimals_scientific)?;
    Ok(())
}
pub fn init_default() -> Result<(), AlreadyInit> {
    let _guard = INITIALIZING.lock().unwrap();
    FLOAT_PRECISION
        .set(recommended::FLOAT_PRECISION)
        .map_err(|_| AlreadyInit)?;
    parse::init_default()?;
    calculation_tasks::init_default()?;
    calculation_results::init_default()?;
    Ok(())
}
