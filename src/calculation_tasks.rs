//! This module handles the calulation of pending calculation tasks
use crate::calculation_results::Number;
use crate::math::FLOAT_PRECISION;

use crate::{
    calculation_results::{Calculation, CalculationResult},
    math,
};

use rug::{ops::Pow, Float, Integer};
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
            base,
            level,
            negative,
        } = self;
        match base {
            CalculationBase::Num(num) => {
                vec![Self::calculate_appropriate_factorial(num, level, negative)]
            }
            CalculationBase::Calc(calc) => {
                let mut calcs = calc.execute(include_steps);
                match calcs.last() {
                    Some(Some(Calculation {
                        result: res,
                        steps,
                        value: number,
                    })) => {
                        let neg = steps.last().unwrap().1;
                        let res = match res {
                            CalculationResult::Exact(res) => Ok(Number::Int(res.clone())),
                            CalculationResult::Approximate(base, exponent) => {
                                let res = base.as_float()
                                    * Float::with_val(FLOAT_PRECISION, 10).pow(exponent);
                                match res.to_integer() {
                                    None => Err(if neg % 2 != 0 {
                                        CalculationResult::ComplexInfinity
                                    } else if level == 0 {
                                        let termial = math::approximate_approx_termial((
                                            base.as_float().clone(),
                                            exponent.clone(),
                                        ));
                                        CalculationResult::Approximate(termial.0.into(), termial.1)
                                    } else {
                                        CalculationResult::ApproximateDigitsTower(
                                            false,
                                            1,
                                            exponent.clone() + math::length(exponent),
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
                                } else if level == 0 {
                                    CalculationResult::ApproximateDigits(
                                        (digits.clone() - 1) * 2 + 1,
                                    )
                                } else {
                                    CalculationResult::ApproximateDigitsTower(
                                        false,
                                        1,
                                        digits.clone() + math::length(digits),
                                    )
                                })
                            }
                            CalculationResult::ApproximateDigitsTower(
                                negative,
                                depth,
                                exponent,
                            ) => Err(if *negative {
                                CalculationResult::Float(Float::new(FLOAT_PRECISION).into())
                            } else if neg % 2 != 0 {
                                CalculationResult::ComplexInfinity
                            } else if level == 0 {
                                CalculationResult::ApproximateDigitsTower(
                                    false,
                                    *depth,
                                    exponent.clone(),
                                )
                            } else {
                                CalculationResult::ApproximateDigitsTower(
                                    false,
                                    depth + 1,
                                    exponent.clone(),
                                )
                            }),
                            CalculationResult::Float(gamma) => Ok(Number::Float(gamma.clone())),
                            CalculationResult::ComplexInfinity => {
                                Err(CalculationResult::ComplexInfinity)
                            }
                        };
                        let factorial = match res {
                            Ok(res) => Self::calculate_appropriate_factorial(res, level, negative)
                                .map(|mut res| {
                                    let current_steps = res.steps;
                                    res.steps = steps.clone();
                                    res.steps.extend(current_steps);
                                    res.value = number.clone();
                                    res
                                }),
                            Err(result) => {
                                let mut steps = steps.clone();
                                steps.push((level, negative));
                                Some(Calculation {
                                    value: number.clone(),
                                    steps,
                                    result,
                                })
                            }
                        };
                        if include_steps {
                            calcs.push(factorial);
                        } else {
                            calcs = vec![factorial];
                        }
                    }
                    _ => return calcs,
                };
                calcs
            }
        }
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
                0 => {
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
        } else if level == -1 {
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
                    steps: vec![(-1, negative)],
                    result: CalculationResult::ApproximateDigits(factorial),
                }
            } else if calc_num > UPPER_SUBFACTORIAL_LIMIT {
                let factorial = math::approximate_subfactorial(calc_num.clone());
                Calculation {
                    value: num,
                    steps: vec![(-1, negative)],
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
                    steps: vec![(-1, negative)],
                    result: CalculationResult::Exact(factorial),
                }
            })
        } else if level == 0 {
            Some(if calc_num > *UPPER_TERMIAL_APPROXIMATION_LIMIT {
                let termial = math::approximate_termial_digits(calc_num);
                Calculation {
                    value: num,
                    steps: vec![(0, negative)],
                    result: CalculationResult::ApproximateDigits(termial),
                }
            } else if calc_num > *UPPER_TERMIAL_LIMIT {
                let termial = math::approximate_termial(calc_num);
                Calculation {
                    value: num,
                    steps: vec![(0, negative)],
                    result: CalculationResult::Approximate(
                        ((termial.0 * if negative % 2 != 0 { -1 } else { 1 }) as Float).into(),
                        termial.1,
                    ),
                }
            } else {
                let termial = math::termial(calc_num) * if negative % 2 != 0 { -1 } else { 1 };
                Calculation {
                    value: num,
                    steps: vec![(0, negative)],
                    result: CalculationResult::Exact(termial),
                }
            })
        } else {
            None
        }
    }

    pub fn get_depth(&self) -> usize {
        match &self.base {
            CalculationBase::Num(_) => 0,
            CalculationBase::Calc(calc) => calc.get_depth() + 1,
        }
    }
}
