//! This module handles the calculation of pending calculation tasks

use std::ops::ControlFlow;

use factorion_math::rug::Integer;
use factorion_math::rug::float::OrdFloat;
use factorion_math::rug::ops::AddFrom;
#[cfg(any(feature = "serde", test))]
use serde::{Deserialize, Serialize};

use crate::calculation_results::Number;

use crate::Consts;
use crate::{
    calculation_results::{Calculation, CalculationResult},
    math,
};

use crate::rug::{Float, ops::Pow};

pub mod recommended {
    use factorion_math::rug::Complete;
    use factorion_math::rug::integer::IntegerExt64;

    use crate::rug::Integer;
    // Limit for exact calculation, set to limit calculation time
    pub static UPPER_CALCULATION_LIMIT: fn() -> Integer = || 1_000_000.into();
    // Limit for approximation, set to ensure enough accuracy (5 decimals)
    pub static UPPER_APPROXIMATION_LIMIT: fn() -> Integer =
        || Integer::u64_pow_u64(10, 300).complete();
    // Limit for exact subfactorial calculation, set to limit calculation time
    pub static UPPER_SUBFACTORIAL_LIMIT: fn() -> Integer = || 100_000.into();
    // Limit for exact termial calculation, set to limit calculation time (absurdly high)
    pub static UPPER_TERMIAL_LIMIT: fn() -> Integer = || Integer::u64_pow_u64(10, 10000).complete();
    // Limit for approximation, set to ensure enough accuracy (5 decimals)
    // Based on max float. (bits)
    pub static UPPER_TERMIAL_APPROXIMATION_LIMIT: u32 = 1073741822;
}

/// Representation of the calculation to be done
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(any(feature = "serde", test), derive(Serialize, Deserialize))]
pub struct CalculationJob {
    pub base: CalculationBase,
    /// Type of the calculation
    pub level: i32,
    /// Number of negations encountered
    pub negative: u32,
}
/// The basis of a calculation, whether [Number] or [CalculationJob].
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(any(feature = "serde", test), derive(Serialize, Deserialize))]
pub enum CalculationBase {
    Num(Number),
    Calc(Box<CalculationJob>),
}

impl CalculationJob {
    /// Execute the calculation. \
    /// If include_steps is enabled, will return all intermediate results.
    pub fn execute(self, include_steps: bool, consts: &Consts) -> Vec<Option<Calculation>> {
        let CalculationJob {
            mut base,
            mut level,
            mut negative,
        } = self;
        let size = {
            let mut n = 1;
            let mut b = &base;
            while let CalculationBase::Calc(inner) = b {
                n += 1;
                b = &inner.base;
            }
            n
        };
        // TODO: Maybe ignore include steps if size is too big (we can't respond properly anyway)
        let mut steps = Vec::with_capacity(size);
        let mut calcs = loop {
            match base {
                CalculationBase::Num(num) => {
                    break vec![
                        calculate_appropriate_factorial(num.clone(), level, negative, consts).map(
                            |res| Calculation {
                                value: num,
                                steps: vec![(level, negative % 2 == 1)],
                                result: res,
                            },
                        ),
                    ];
                }
                CalculationBase::Calc(calc) => {
                    steps.push((level, negative));
                    CalculationJob {
                        base,
                        level,
                        negative,
                    } = *calc;
                }
            }
        };
        for (i, (level, negative)) in steps.into_iter().rev().enumerate() {
            let calc = if include_steps && i < 30 {
                calcs.last().cloned()
            } else {
                calcs.pop()
            };
            match calc {
                Some(Some(Calculation {
                    result: res,
                    mut steps,
                    value: number,
                })) => {
                    let factorial = calculate_appropriate_factorial(res, level, negative, consts)
                        .map(|res| {
                            steps.push((level, negative % 2 == 1));
                            Calculation {
                                value: number,
                                steps,
                                result: res,
                            }
                        });
                    calcs.push(factorial);
                }
                _ => return calcs,
            };
        }
        calcs
    }
}

fn calculate_appropriate_factorial(
    num: Number,
    level: i32,
    negative: u32,
    consts: &Consts,
) -> Option<CalculationResult> {
    let prec = consts.float_precision;
    let calc_num = match num {
        CalculationResult::ComplexInfinity => return Some(CalculationResult::ComplexInfinity),
        Number::Float(num) | CalculationResult::Approximate(num, _)
            if !num.as_float().is_finite() =>
        {
            return Some(CalculationResult::ComplexInfinity);
        }
        CalculationResult::Approximate(base, exponent) => {
            match calculate_or_extract_approximate(level, consts, prec, base, exponent) {
                ControlFlow::Continue(value) => value,
                ControlFlow::Break(value) => return value,
            }
        }

        CalculationResult::ApproximateDigits(was_neg, digits) => {
            match calculate_or_extract_approximate_digits(level, consts, prec, was_neg, digits) {
                ControlFlow::Continue(value) => value,
                ControlFlow::Break(value) => return value,
            }
        }
        CalculationResult::ApproximateDigitsTower(was_neg, neg, depth, exponent) => {
            return calculate_approximate_digits_tower(level, prec, was_neg, neg, depth, exponent);
        }
        Number::Float(num) => match calculate_or_extract_float(level, negative, num) {
            ControlFlow::Continue(num) => num,
            ControlFlow::Break(val) => return val,
        },
        Number::Exact(num) if num.significant_bits() >= math::rug::float::exp_max() as u32 => {
            return calculate_exact_as_approximate(level, negative, consts, num);
        }
        Number::Exact(num) => num,
    };
    Some(if level > 0 {
        calculate_k_factorial(level, negative, consts, prec, &calc_num)?
    } else if level == 0 {
        calculate_subfactorial(negative, consts, prec, &calc_num)
    } else if level < 0 {
        calculate_termial(level, negative, consts, prec, calc_num)
    } else {
        unreachable!()
    })
}

fn calculate_termial(
    level: i32,
    negative: u32,
    consts: &Consts<'_>,
    prec: u32,
    calc_num: Integer,
) -> CalculationResult {
    if calc_num.significant_bits() > consts.upper_termial_approximation_limit {
        let termial = math::approximate_termial_digits(calc_num, level.unsigned_abs(), prec);
        CalculationResult::ApproximateDigits(!negative.is_multiple_of(2), termial)
    } else if *calc_num.as_abs() > consts.upper_termial_limit {
        let termial = math::approximate_termial(calc_num, level.unsigned_abs(), prec);
        CalculationResult::Approximate(
            ((termial.0 * if !negative.is_multiple_of(2) { -1 } else { 1 }) as Float).into(),
            termial.1,
        )
    } else {
        let termial = if level < -1 {
            math::multitermial(calc_num, level.unsigned_abs())
        } else {
            math::termial(calc_num)
        };
        let termial = termial * if !negative.is_multiple_of(2) { -1 } else { 1 };
        CalculationResult::Exact(termial)
    }
}

fn calculate_subfactorial(
    negative: u32,
    consts: &Consts<'_>,
    prec: u32,
    calc_num: &Integer,
) -> CalculationResult {
    if *calc_num < 0 {
        CalculationResult::ComplexInfinity
    } else if *calc_num > consts.upper_approximation_limit {
        let factorial = math::approximate_multifactorial_digits(calc_num.clone(), 1, prec);
        CalculationResult::ApproximateDigits(!negative.is_multiple_of(2), factorial)
    } else if *calc_num > consts.upper_subfactorial_limit {
        let factorial = math::approximate_subfactorial(calc_num.clone(), prec);
        CalculationResult::Approximate(
            ((factorial.0 * if !negative.is_multiple_of(2) { -1 } else { 1 }) as Float).into(),
            factorial.1,
        )
    } else {
        let calc_num = calc_num
            .to_u64()
            .unwrap_or_else(|| panic!("Failed to convert BigInt to u64: {calc_num}"));
        let factorial =
            math::subfactorial(calc_num) * if !negative.is_multiple_of(2) { -1 } else { 1 };
        CalculationResult::Exact(factorial)
    }
}

fn calculate_k_factorial(
    level: i32,
    negative: u32,
    consts: &Consts<'_>,
    prec: u32,
    calc_num: &Integer,
) -> Option<CalculationResult> {
    Some(if *calc_num < 0 && level == 1 {
        CalculationResult::ComplexInfinity
    } else if *calc_num < 0 {
        let factor = math::negative_multifacorial_factor(calc_num.clone(), level);
        match (factor, -level - 1 > *calc_num) {
            (Some(factor), true) => {
                let mut res = calculate_appropriate_factorial(
                    Number::Exact(-calc_num.clone() - level),
                    level,
                    negative,
                    consts,
                )?;
                res = match res {
                    CalculationResult::Exact(n) => {
                        let n = Float::with_val(prec, n);
                        CalculationResult::Float((factor / n).into())
                    }
                    CalculationResult::Approximate(b, e) => {
                        let (b, e) = math::adjust_approximate((factor / Float::from(b), -e));
                        CalculationResult::Approximate(b.into(), e)
                    }
                    CalculationResult::ApproximateDigits(wn, n) => {
                        CalculationResult::ApproximateDigits(wn, -n)
                    }
                    CalculationResult::ApproximateDigitsTower(wn, negative, depth, base) => {
                        CalculationResult::ApproximateDigitsTower(wn, !negative, depth, base)
                    }
                    CalculationResult::ComplexInfinity => CalculationResult::Exact(0.into()),
                    CalculationResult::Float(f) => {
                        CalculationResult::Float((factor / Float::from(f)).into())
                    }
                };

                res
            }
            (factor, _) => factor
                .map(CalculationResult::Exact)
                .unwrap_or(CalculationResult::ComplexInfinity),
        }
        // Check if we can approximate the number of digits
    } else if *calc_num > consts.upper_approximation_limit {
        let factorial =
            math::approximate_multifactorial_digits(calc_num.clone(), level as u32, prec);
        CalculationResult::ApproximateDigits(!negative.is_multiple_of(2), factorial)
    // Check if the number is within a reasonable range to compute
    } else if *calc_num > consts.upper_calculation_limit {
        let factorial = if level == 0 {
            math::approximate_factorial(calc_num.clone(), prec)
        } else {
            math::approximate_multifactorial(calc_num.clone(), level as u32, prec)
        };
        CalculationResult::Approximate(
            ((factorial.0 * if !negative.is_multiple_of(2) { -1 } else { 1 }) as Float).into(),
            factorial.1,
        )
    } else {
        let calc_num = calc_num
            .to_u64()
            .unwrap_or_else(|| panic!("Failed to convert BigInt to u64: {calc_num}"));
        let factorial = math::factorial(calc_num, level as u32)
            * if !negative.is_multiple_of(2) { -1 } else { 1 };
        CalculationResult::Exact(factorial)
    })
}

fn calculate_exact_as_approximate(
    level: i32,
    negative: u32,
    consts: &Consts<'_>,
    num: Integer,
) -> Option<CalculationResult> {
    let sig_bits = num.significant_bits();
    return calculate_appropriate_factorial(
        CalculationResult::Approximate(
            (Float::with_val(consts.float_precision, num / consts.float_precision)
                / (sig_bits - consts.float_precision))
                .into(),
            sig_bits.into(),
        ),
        level,
        negative,
        consts,
    );
}

fn calculate_or_extract_float(
    level: i32,
    negative: u32,
    num: OrdFloat,
) -> ControlFlow<Option<CalculationResult>, Integer> {
    ControlFlow::Continue(match level {
        ..-1 => {
            // We don't support multitermials of decimals
            return ControlFlow::Break(None);
        }
        -1 => {
            let res: Float = math::fractional_termial(num.as_float().clone())
                * if !negative.is_multiple_of(2) { -1 } else { 1 };
            if res.is_finite() {
                return ControlFlow::Break(Some(CalculationResult::Float(res.into())));
            } else {
                let Some(num) = num.as_float().to_integer() else {
                    return ControlFlow::Break(None);
                };
                num
            }
        }
        0 => {
            // We don't support subfactorials of deciamals
            return ControlFlow::Break(None);
        }
        1 => {
            let res: Float = math::fractional_factorial(num.as_float().clone())
                * if !negative.is_multiple_of(2) { -1 } else { 1 };
            if res.is_finite() {
                return ControlFlow::Break(Some(CalculationResult::Float(res.into())));
            } else {
                let Some(num) = num.as_float().to_integer() else {
                    return ControlFlow::Break(None);
                };
                num
            }
        }
        2.. => {
            let res: Float = math::fractional_multifactorial(num.as_float().clone(), level as u32)
                * if !negative.is_multiple_of(2) { -1 } else { 1 };
            if res.is_finite() {
                return ControlFlow::Break(Some(CalculationResult::Float(res.into())));
            } else {
                let Some(num) = num.as_float().to_integer() else {
                    return ControlFlow::Break(None);
                };
                num
            }
        }
    })
}

fn calculate_approximate_digits_tower(
    level: i32,
    prec: u32,
    was_neg: bool,
    neg: bool,
    depth: Integer,
    exponent: Integer,
) -> Option<CalculationResult> {
    Some(if neg {
        CalculationResult::Float(Float::new(prec).into())
    } else if was_neg {
        CalculationResult::ComplexInfinity
    } else if level < 0 {
        CalculationResult::ApproximateDigitsTower(false, false, depth, exponent)
    } else {
        CalculationResult::ApproximateDigitsTower(false, false, depth + 1, exponent)
    })
}

fn calculate_or_extract_approximate_digits(
    level: i32,
    consts: &Consts<'_>,
    prec: u32,
    was_neg: bool,
    digits: Integer,
) -> ControlFlow<Option<CalculationResult>, Integer> {
    ControlFlow::Continue(if digits <= consts.integer_construction_limit {
        let x: Float = Float::with_val(prec, 10).pow(digits.clone() - 1);
        x.to_integer().unwrap()
    } else {
        return ControlFlow::Break(Some(if digits.is_negative() {
            CalculationResult::Float(Float::new(prec).into())
        } else if was_neg {
            CalculationResult::ComplexInfinity
        } else if level < 0 {
            let mut one = Float::with_val(consts.float_precision, 1);
            if was_neg {
                one *= -1;
            }
            let termial = math::approximate_approx_termial((one, digits), -level as u32);
            if termial.0 == 1 {
                CalculationResult::ApproximateDigits(false, termial.1)
            } else {
                CalculationResult::Approximate(termial.0.into(), termial.1)
            }
        } else {
            let mut digits = digits;
            digits.add_from(math::length(&digits, prec));
            CalculationResult::ApproximateDigitsTower(false, false, 1.into(), digits)
        }));
    })
}

fn calculate_or_extract_approximate(
    level: i32,
    consts: &Consts<'_>,
    prec: u32,
    base: OrdFloat,
    exponent: Integer,
) -> ControlFlow<Option<CalculationResult>, Integer> {
    ControlFlow::Continue(if exponent <= consts.integer_construction_limit {
        let x: Float = base.as_float() * Float::with_val(prec, 10).pow(&exponent);
        x.to_integer().unwrap()
    } else {
        return ControlFlow::Break(Some(if base.as_float() < &0.0 {
            CalculationResult::ComplexInfinity
        } else if level < 0 {
            let termial =
                math::approximate_approx_termial((Float::from(base), exponent), -level as u32);
            if termial.0 == 1 {
                CalculationResult::ApproximateDigits(false, termial.1)
            } else {
                CalculationResult::Approximate(termial.0.into(), termial.1)
            }
        } else {
            let mut exponent = exponent;
            exponent.add_from(math::length(&exponent, prec));
            CalculationResult::ApproximateDigitsTower(false, false, 1.into(), exponent)
        }));
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    use factorion_math::recommended::FLOAT_PRECISION;

    #[test]
    fn test_unsupported_calcs() {
        let consts = Consts::default();
        // Subfactorial
        let job = CalculationJob {
            base: CalculationBase::Num(Number::Float(Float::with_val(FLOAT_PRECISION, 1.5).into())),
            level: 0,
            negative: 0,
        };
        assert_eq!(job.execute(false, &consts), vec![None]);
        // Multitermial
        let job = CalculationJob {
            base: CalculationBase::Num(Number::Float(Float::with_val(FLOAT_PRECISION, 1.5).into())),
            level: -2,
            negative: 0,
        };
        assert_eq!(job.execute(false, &consts), vec![None]);
        let job = CalculationJob {
            base: CalculationBase::Num(Number::Float(Float::with_val(FLOAT_PRECISION, 1.5).into())),
            level: -51,
            negative: 0,
        };
        assert_eq!(job.execute(false, &consts), vec![None]);
    }
}
