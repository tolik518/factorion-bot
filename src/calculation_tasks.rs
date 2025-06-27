//! This module handles the calulation of pending calculation tasks
use crate::calculation_results::Number;
use crate::math::FLOAT_PRECISION;

use crate::{
    calculation_results::{Calculation, CalculationResult},
    math,
};

use rug::{Float, Integer, ops::Pow};
use std::{str::FromStr, sync::LazyLock};

// Limit for exact calculation, set to limit calculation time
pub(crate) const UPPER_CALCULATION_LIMIT: u64 = 1_000_000;
// Limit for approximation, set to ensure enough accuracy (5 decimals)
pub(crate) static UPPER_APPROXIMATION_LIMIT: LazyLock<Integer> =
    LazyLock::new(|| Integer::from_str(&format!("1{}", "0".repeat(300))).unwrap());
// Limit for exact subfactorial calculation, set to limit calculation time
pub(crate) const UPPER_SUBFACTORIAL_LIMIT: u64 = 1_000_000;
// Limit for exact termial calculation, set to limit calculation time (absurdly high)
pub(crate) static UPPER_TERMIAL_LIMIT: LazyLock<Integer> =
    LazyLock::new(|| Integer::from_str(&format!("1{}", "0".repeat(10000))).unwrap());
// Limit for approximation, set to ensure enough accuracy (5 decimals)
pub(crate) static UPPER_TERMIAL_APPROXIMATION_LIMIT: LazyLock<Float> = LazyLock::new(|| {
    let mut max = Float::with_val(FLOAT_PRECISION, rug::float::Special::Infinity);
    max.next_down();
    max
});

pub(crate) const INTEGER_CONSTRUCTION_LIMIT: i64 = 100_000_000;
pub(crate) static TOO_BIG_NUMBER: LazyLock<Integer> =
    LazyLock::new(|| Integer::from_str(&format!("1{}", "0".repeat(9999))).unwrap());

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct CalculationJob {
    pub(crate) base: CalculationBase,
    pub(crate) level: i32,
    pub(crate) negative: u32,
}
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum CalculationBase {
    Num(Number),
    Calc(Box<CalculationJob>),
}

impl CalculationJob {
    pub fn execute(self, include_steps: bool) -> Vec<Option<Calculation>> {
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
                    break vec![Self::calculate_appropriate_factorial(num, level, negative)];
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
        for (level, negative) in steps.into_iter().rev() {
            let calc = if include_steps {
                calcs.last().cloned()
            } else {
                calcs.pop()
            };
            match calc {
                Some(Some(Calculation {
                    result: res,
                    steps,
                    value: number,
                })) => {
                    let neg = steps.last().unwrap().1;
                    let res = match res {
                        CalculationResult::Exact(res) => Ok(Number::Int(res)),
                        CalculationResult::Approximate(base, exponent) => {
                            let res = base.as_float()
                                * Float::with_val(FLOAT_PRECISION, 10).pow(&exponent);
                            match res.to_integer() {
                                None => Err(if neg % 2 != 0 {
                                    CalculationResult::ComplexInfinity
                                } else if level < 0 {
                                    let termial = math::approximate_approx_termial(
                                        (Float::from(base), exponent),
                                        -level as u32,
                                    );
                                    CalculationResult::Approximate(termial.0.into(), termial.1)
                                } else {
                                    CalculationResult::ApproximateDigitsTower(
                                        false,
                                        1,
                                        math::length(&exponent) + exponent,
                                    )
                                }),
                                Some(res) => Ok(Number::Int(res)),
                            }
                        }
                        CalculationResult::ApproximateDigits(digits) => {
                            Err(if digits.is_negative() {
                                CalculationResult::Float(Float::new(FLOAT_PRECISION).into())
                            } else if neg % 2 != 0 {
                                CalculationResult::ComplexInfinity
                            } else if level < 0 {
                                CalculationResult::ApproximateDigits((digits - 1) * 2 + 1)
                            } else {
                                CalculationResult::ApproximateDigitsTower(
                                    false,
                                    1,
                                    math::length(&digits) + digits,
                                )
                            })
                        }
                        CalculationResult::ApproximateDigitsTower(negative, depth, exponent) => {
                            Err(if negative {
                                CalculationResult::Float(Float::new(FLOAT_PRECISION).into())
                            } else if neg % 2 != 0 {
                                CalculationResult::ComplexInfinity
                            } else if level < 0 {
                                CalculationResult::ApproximateDigitsTower(false, depth, exponent)
                            } else {
                                CalculationResult::ApproximateDigitsTower(
                                    false,
                                    depth + 1,
                                    exponent,
                                )
                            })
                        }
                        CalculationResult::Float(gamma) => Ok(Number::Float(gamma)),
                        CalculationResult::ComplexInfinity => {
                            Err(CalculationResult::ComplexInfinity)
                        }
                    };
                    let factorial = match res {
                        Ok(res) => Self::calculate_appropriate_factorial(res, level, negative).map(
                            |mut res| {
                                let current_steps = res.steps;
                                res.steps = steps;
                                res.steps.extend(current_steps);
                                res.value = number;
                                res
                            },
                        ),
                        Err(result) => {
                            let mut steps = steps;
                            steps.push((level, negative));
                            Some(Calculation {
                                value: number,
                                steps,
                                result,
                            })
                        }
                    };
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
    ) -> Option<Calculation> {
        let calc_num = match &num {
            Number::Float(num) => match level {
                1 => {
                    let res: Float = math::fractional_factorial(num.as_float().clone())
                        * if negative % 2 != 0 { -1 } else { 1 };
                    if res.is_finite() {
                        return Some(Calculation {
                            value: Number::Float(num.clone()),
                            steps: vec![(1, negative)],
                            result: CalculationResult::Float(res.into()),
                        });
                    } else {
                        num.as_float().to_integer()?
                    }
                }
                ..0 => {
                    let res: Float = math::fractional_termial(num.as_float().clone())
                        * if negative % 2 != 0 { -1 } else { 1 };
                    if res.is_finite() {
                        return Some(Calculation {
                            value: Number::Float(num.clone()),
                            steps: vec![(0, negative)],
                            result: CalculationResult::Float(res.into()),
                        });
                    } else {
                        num.as_float().to_integer()?
                    }
                }
                k => {
                    let res: Float = math::fractional_multifactorial(num.as_float().clone(), k)
                        * if negative % 2 != 0 { -1 } else { 1 };
                    if res.is_finite() {
                        return Some(Calculation {
                            value: Number::Float(num.clone()),
                            steps: vec![(k, negative)],
                            result: CalculationResult::Float(res.into()),
                        });
                    } else {
                        num.as_float().to_integer()?
                    }
                }
            },
            Number::Int(num) => num.clone(),
        };
        if level > 0 {
            Some(if calc_num < 0 && level == 1 {
                Calculation {
                    value: num,
                    steps: vec![(level, negative)],
                    result: CalculationResult::ComplexInfinity,
                }
            } else if calc_num < 0 {
                let factor = math::negative_multifacorial_factor(calc_num.clone(), level);
                match (factor, -level - 1 > calc_num) {
                    (Some(factor), true) => {
                        let mut res = Self::calculate_appropriate_factorial(
                            Number::Int(-calc_num.clone() - level),
                            level,
                            negative,
                        )?;
                        res.value = num;
                        res.result = match res.result {
                            CalculationResult::Exact(n) => {
                                let n = Float::with_val(FLOAT_PRECISION, n);
                                CalculationResult::Float((factor / n).into())
                            }
                            CalculationResult::Approximate(b, e) => {
                                let (b, e) =
                                    math::adjust_approximate((factor / Float::from(b), -e));
                                CalculationResult::Approximate(b.into(), e)
                            }
                            CalculationResult::ApproximateDigits(n) => {
                                CalculationResult::ApproximateDigits(-n)
                            }
                            CalculationResult::ApproximateDigitsTower(negative, depth, base) => {
                                CalculationResult::ApproximateDigitsTower(!negative, depth, base)
                            }
                            CalculationResult::ComplexInfinity => {
                                CalculationResult::Exact(0.into())
                            }
                            CalculationResult::Float(f) => {
                                CalculationResult::Float((factor / Float::from(f)).into())
                            }
                        };

                        res
                    }
                    (factor, _) => Calculation {
                        value: num,
                        steps: vec![(level, negative)],
                        result: factor
                            .map(CalculationResult::Exact)
                            .unwrap_or(CalculationResult::ComplexInfinity),
                    },
                }
                // Check if we can approximate the number of digits
            } else if calc_num > *UPPER_APPROXIMATION_LIMIT {
                let factorial = math::approximate_multifactorial_digits(calc_num.clone(), level);
                Calculation {
                    value: num,
                    steps: vec![(level, negative)],
                    result: CalculationResult::ApproximateDigits(factorial),
                }
            // Check if the number is within a reasonable range to compute
            } else if calc_num > UPPER_CALCULATION_LIMIT {
                let factorial = if level == 0 {
                    math::approximate_factorial(calc_num.clone())
                } else {
                    math::approximate_multifactorial(calc_num.clone(), level)
                };
                Calculation {
                    value: Number::Int(calc_num),
                    steps: vec![(level, negative)],
                    result: CalculationResult::Approximate(
                        ((factorial.0 * if negative % 2 != 0 { -1 } else { 1 }) as Float).into(),
                        factorial.1,
                    ),
                }
            } else {
                let calc_num = calc_num.to_u64().expect("Failed to convert BigInt to u64");
                let factorial =
                    math::factorial(calc_num, level) * if negative % 2 != 0 { -1 } else { 1 };
                Calculation {
                    value: num,
                    steps: vec![(level, negative)],
                    result: CalculationResult::Exact(factorial),
                }
            })
        } else if level == 0 {
            Some(if calc_num < 0 {
                Calculation {
                    value: num,
                    steps: vec![(level, negative)],
                    result: CalculationResult::ComplexInfinity,
                }
            } else if calc_num > *UPPER_APPROXIMATION_LIMIT {
                let factorial = math::approximate_multifactorial_digits(calc_num.clone(), 1);
                Calculation {
                    value: num,
                    steps: vec![(level, negative)],
                    result: CalculationResult::ApproximateDigits(factorial),
                }
            } else if calc_num > UPPER_SUBFACTORIAL_LIMIT {
                let factorial = math::approximate_subfactorial(calc_num.clone());
                Calculation {
                    value: num,
                    steps: vec![(level, negative)],
                    result: CalculationResult::Approximate(
                        ((factorial.0 * if negative % 2 != 0 { -1 } else { 1 }) as Float).into(),
                        factorial.1,
                    ),
                }
            } else {
                let calc_num = calc_num.to_u64().expect("Failed to convert BigInt to u64");
                let factorial =
                    math::subfactorial(calc_num) * if negative % 2 != 0 { -1 } else { 1 };
                Calculation {
                    value: num,
                    steps: vec![(level, negative)],
                    result: CalculationResult::Exact(factorial),
                }
            })
        } else if level < 0 {
            Some(if calc_num > *UPPER_TERMIAL_APPROXIMATION_LIMIT {
                let termial = math::approximate_termial_digits(calc_num, -level as u32);
                Calculation {
                    value: num,
                    steps: vec![(level, negative)],
                    result: CalculationResult::ApproximateDigits(termial),
                }
            } else if calc_num > *UPPER_TERMIAL_LIMIT {
                let termial = math::approximate_termial(calc_num, -level as u32);
                Calculation {
                    value: num,
                    steps: vec![(level, negative)],
                    result: CalculationResult::Approximate(
                        ((termial.0 * if negative % 2 != 0 { -1 } else { 1 }) as Float).into(),
                        termial.1,
                    ),
                }
            } else {
                let termial = if level < -1 {
                    math::multitermial(calc_num, -level as u32)
                } else {
                    math::termial(calc_num)
                };
                let termial = termial * if negative % 2 != 0 { -1 } else { 1 };
                Calculation {
                    value: num,
                    steps: vec![(level, negative)],
                    result: CalculationResult::Exact(termial),
                }
            })
        } else {
            None
        }
    }
}
