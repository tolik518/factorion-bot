use std::sync::OnceLock;

use factorion_math as math;
use rug::Integer;
pub mod calculation_results;
pub mod calculation_tasks;
pub mod comment;
pub mod parse;

pub static FLOAT_PRECISION: OnceLock<u32> = OnceLock::new();
pub mod recommended {
    pub use super::math::recommended::FLOAT_PRECISION;
    pub use crate::calculation_results::recommended::*;
    pub use crate::calculation_tasks::recommended::*;
    pub use crate::parse::recommended::*;
}
#[derive(Debug, Clone, Copy)]
pub struct AlreadyInit;
pub fn init(
    float_precision: u32,
    upper_calculation_limit: Integer,
    upper_approximation_limit: Integer,
    upper_subfactorial_limit: Integer,
    upper_termial_limit: Integer,
    upper_termial_approximation_limit: Integer,
    integer_construction_limit: Integer,
    number_decimals_scientific: usize,
) -> Result<(), AlreadyInit> {
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
    let mut already = false;
    if let Err(_) = FLOAT_PRECISION.set(recommended::FLOAT_PRECISION) {
        already = true;
    }
    if let Err(_) = parse::init_default() {
        already = true;
    }
    if let Err(_) = calculation_tasks::init_default() {
        already = true;
    }
    if let Err(_) = calculation_results::init_default() {
        already = true;
    }
    (!already).then_some(()).ok_or(AlreadyInit)
}
