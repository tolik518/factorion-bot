//! This module handles the calculation of pending calculation tasks

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
    pub static UPPER_SUBFACTORIAL_LIMIT: fn() -> Integer = || 1_000_000.into();
    // Limit for exact termial calculation, set to limit calculation time (absurdly high)
    pub static UPPER_TERMIAL_LIMIT: fn() -> Integer = || Integer::u64_pow_u64(10, 10_000).complete();
    // Limit for approximation, set to ensure enough accuracy (5 decimals)
    // Based on max float. (bits)
    pub static UPPER_TERMIAL_APPROXIMATION_LIMIT: u32 = 1_073_741_822;
}

/// Representation of the calculation to be done
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
#[cfg_attr(any(feature = "serde", test), derive(Serialize, Deserialize))]
pub struct CalculationJob {
    pub base: CalculationBase,
    /// Type of the calculation
    pub level: i32,
    /// Number of negations encountered
    pub negative: u32,
}
/// The basis of a calculation, whether [Number] or [CalculationJob].
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
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
                        Self::calculate_appropriate_factorial(num.clone(), level, negative, consts)
                            .map(|res| Calculation {
                                value: num,
                                steps: vec![(level, negative % 2 == 1)],
                                result: res,
                            }),
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
                    let factorial = Self::calculate_appropriate_factorial(
                        res, level, negative, consts,
                    )
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
    fn calculate_appropriate_factorial(
        num: Number,
        level: i32,
        negative: u32,
        consts: &Consts,
    ) -> Option<CalculationResult> {
        let prec = consts.float_precision;
        let calc_num = match num {
            CalculationResult::Approximate(base, exponent) => {
                let res = base.as_float() * Float::with_val(prec, 10).pow(&exponent);
                if Float::is_finite(&(res.clone() * math::APPROX_FACT_SAFE_UPPER_BOUND_FACTOR)) {
                    res.to_integer().unwrap()
                } else {
                    return Some(if base.as_float() < &0.0 {
                        CalculationResult::ComplexInfinity
                    } else if level < 0 {
                        let termial = math::approximate_approx_termial(
                            (Float::from(base), exponent),
                            -level as u32,
                        );
                        CalculationResult::Approximate(termial.0.into(), termial.1)
                    } else {
                        let mut exponent = exponent;
                        exponent.add_from(math::length(&exponent, prec));
                        CalculationResult::ApproximateDigitsTower(false, false, 1, exponent)
                    });
                }
            }
            CalculationResult::ApproximateDigits(was_neg, digits) => {
                return Some(if digits.is_negative() {
                    CalculationResult::Float(Float::new(prec).into())
                } else if was_neg {
                    CalculationResult::ComplexInfinity
                } else if level < 0 {
                    CalculationResult::ApproximateDigits(false, (digits - 1) * 2 + 1)
                } else {
                    let mut digits = digits;
                    digits.add_from(math::length(&digits, prec));
                    CalculationResult::ApproximateDigitsTower(false, false, 1, digits)
                });
            }
            CalculationResult::ApproximateDigitsTower(was_neg, neg, depth, exponent) => {
                return Some(if neg {
                    CalculationResult::Float(Float::new(prec).into())
                } else if was_neg {
                    CalculationResult::ComplexInfinity
                } else if level < 0 {
                    CalculationResult::ApproximateDigitsTower(false, false, depth, exponent)
                } else {
                    CalculationResult::ApproximateDigitsTower(false, false, depth + 1, exponent)
                });
            }
            CalculationResult::ComplexInfinity => return Some(CalculationResult::ComplexInfinity),
            Number::Float(num) => match level {
                ..-1 => {
                    // We don't support multitermials of decimals
                    return None;
                }
                -1 => {
                    let res: Float = math::fractional_termial(num.as_float().clone())
                        * if negative % 2 != 0 { -1 } else { 1 };
                    if res.is_finite() {
                        return Some(CalculationResult::Float(res.into()));
                    } else {
                        num.as_float().to_integer()?
                    }
                }
                0 => {
                    // We don't support subfactorials of deciamals
                    return None;
                }
                1 => {
                    let res: Float = math::fractional_factorial(num.as_float().clone())
                        * if negative % 2 != 0 { -1 } else { 1 };
                    if res.is_finite() {
                        return Some(CalculationResult::Float(res.into()));
                    } else {
                        num.as_float().to_integer()?
                    }
                }
                2.. => {
                    let res: Float =
                        math::fractional_multifactorial(num.as_float().clone(), level as u32)
                            * if negative % 2 != 0 { -1 } else { 1 };
                    if res.is_finite() {
                        return Some(CalculationResult::Float(res.into()));
                    } else {
                        num.as_float().to_integer()?
                    }
                }
            },
            Number::Exact(num) => num,
        };
        if level > 0 {
            Some(if calc_num < 0 && level == 1 {
                CalculationResult::ComplexInfinity
            } else if calc_num < 0 {
                let factor = math::negative_multifacorial_factor(calc_num.clone(), level);
                match (factor, -level - 1 > calc_num) {
                    (Some(factor), true) => {
                        let mut res = Self::calculate_appropriate_factorial(
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
                                let (b, e) =
                                    math::adjust_approximate((factor / Float::from(b), -e));
                                CalculationResult::Approximate(b.into(), e)
                            }
                            CalculationResult::ApproximateDigits(wn, n) => {
                                CalculationResult::ApproximateDigits(wn, -n)
                            }
                            CalculationResult::ApproximateDigitsTower(
                                wn,
                                negative,
                                depth,
                                base,
                            ) => CalculationResult::ApproximateDigitsTower(
                                wn, !negative, depth, base,
                            ),
                            CalculationResult::ComplexInfinity => {
                                CalculationResult::Exact(0.into())
                            }
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
            } else if calc_num > consts.upper_approximation_limit {
                let factorial =
                    math::approximate_multifactorial_digits(calc_num.clone(), level as u32, prec);
                CalculationResult::ApproximateDigits(negative % 2 != 0, factorial)
            // Check if the number is within a reasonable range to compute
            } else if calc_num > consts.upper_calculation_limit {
                let factorial = if level == 0 {
                    math::approximate_factorial(calc_num.clone(), prec)
                } else {
                    math::approximate_multifactorial(calc_num.clone(), level as u32, prec)
                };
                CalculationResult::Approximate(
                    ((factorial.0 * if negative % 2 != 0 { -1 } else { 1 }) as Float).into(),
                    factorial.1,
                )
            } else {
                let calc_num = calc_num.to_u64().expect("Failed to convert BigInt to u64");
                let factorial = math::factorial(calc_num, level as u32)
                    * if negative % 2 != 0 { -1 } else { 1 };
                CalculationResult::Exact(factorial)
            })
        } else if level == 0 {
            Some(if calc_num < 0 {
                CalculationResult::ComplexInfinity
            } else if calc_num > consts.upper_approximation_limit {
                let factorial = math::approximate_multifactorial_digits(calc_num.clone(), 1, prec);
                CalculationResult::ApproximateDigits(negative % 2 != 0, factorial)
            } else if calc_num > consts.upper_subfactorial_limit {
                let factorial = math::approximate_subfactorial(calc_num.clone(), prec);
                CalculationResult::Approximate(
                    ((factorial.0 * if negative % 2 != 0 { -1 } else { 1 }) as Float).into(),
                    factorial.1,
                )
            } else {
                let calc_num = calc_num.to_u64().expect("Failed to convert BigInt to u64");
                let factorial =
                    math::subfactorial(calc_num) * if negative % 2 != 0 { -1 } else { 1 };
                CalculationResult::Exact(factorial)
            })
        } else if level < 0 {
            Some(
                if calc_num.significant_bits() > consts.upper_termial_approximation_limit {
                    let termial = math::approximate_termial_digits(calc_num, -level as u32, prec);
                    CalculationResult::ApproximateDigits(negative % 2 != 0, termial)
                } else if calc_num > consts.upper_termial_limit {
                    let termial = math::approximate_termial(calc_num, -level as u32, prec);
                    CalculationResult::Approximate(
                        ((termial.0 * if negative % 2 != 0 { -1 } else { 1 }) as Float).into(),
                        termial.1,
                    )
                } else {
                    let termial = if level < -1 {
                        math::multitermial(calc_num, -level as u32)
                    } else {
                        math::termial(calc_num)
                    };
                    let termial = termial * if negative % 2 != 0 { -1 } else { 1 };
                    CalculationResult::Exact(termial)
                },
            )
        } else {
            unreachable!()
        }
    }
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
