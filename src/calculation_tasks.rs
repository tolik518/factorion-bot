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
}
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum CalculationBase {
    Num(Number),
    Calc(Box<CalculationJob>),
}

impl CalculationJob {
    pub fn execute(self, include_steps: bool) -> Vec<Option<Calculation>> {
        let CalculationJob { base, level } = self;
        match base {
            CalculationBase::Num(num) => {
                vec![Self::calculate_appropriate_factorial(num, level)]
            }
            CalculationBase::Calc(calc) => {
                let mut calcs = calc.execute(include_steps);
                match calcs.last() {
                    Some(Some(Calculation {
                        result: res,
                        levels,
                        value: number,
                    })) => {
                        let res = match res {
                            CalculationResult::Exact(res) => Ok(Number::Int(res.clone())),
                            CalculationResult::Approximate(base, exponent) => {
                                let res = base.as_float()
                                    * Float::with_val(FLOAT_PRECISION, 10).pow(exponent);
                                match res.to_integer() {
                                    None => Err(if level == 0 {
                                        let termial = math::approximate_approx_termial((
                                            base.as_float().clone(),
                                            exponent.clone(),
                                        ));
                                        CalculationResult::Approximate(termial.0.into(), termial.1)
                                    } else {
                                        CalculationResult::ApproximateDigitsTower(
                                            1,
                                            exponent.clone() + math::length(exponent),
                                        )
                                    }),
                                    Some(res) => Ok(Number::Int(res)),
                                }
                            }
                            CalculationResult::ApproximateDigits(digits) => Err(if level == 0 {
                                CalculationResult::ApproximateDigits((digits.clone() - 1) * 2 + 1)
                            } else {
                                CalculationResult::ApproximateDigitsTower(
                                    1,
                                    digits.clone() + math::length(digits),
                                )
                            }),
                            CalculationResult::ApproximateDigitsTower(depth, exponent) => {
                                Err(if level == 0 {
                                    CalculationResult::ApproximateDigitsTower(
                                        *depth,
                                        exponent.clone(),
                                    )
                                } else {
                                    CalculationResult::ApproximateDigitsTower(
                                        depth + 1,
                                        exponent.clone(),
                                    )
                                })
                            }
                            CalculationResult::Float(gamma) => Ok(Number::Float(gamma.clone())),
                        };
                        let factorial = match res {
                            Ok(res) => {
                                Self::calculate_appropriate_factorial(res, level).map(|mut res| {
                                    let current_levels = res.levels;
                                    res.levels = levels.clone();
                                    res.levels.extend(current_levels);
                                    res.value = number.clone();
                                    res
                                })
                            }
                            Err(result) => {
                                let mut levels = levels.clone();
                                levels.push(level);
                                Some(Calculation {
                                    value: number.clone(),
                                    levels,
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
    fn calculate_appropriate_factorial(num: Number, level: i32) -> Option<Calculation> {
        let calc_num = match &num {
            Number::Float(num) => match level {
                1 => {
                    let res = math::fractional_factorial(num.as_float().clone());
                    if res.is_finite() {
                        return Some(Calculation {
                            value: Number::Float(num.clone()),
                            levels: vec![1],
                            result: CalculationResult::Float(res.into()),
                        });
                    } else {
                        num.as_float().to_integer()?
                    }
                }
                0 => {
                    let res = math::fractional_termial(num.as_float().clone());
                    if res.is_finite() {
                        return Some(Calculation {
                            value: Number::Float(num.clone()),
                            levels: vec![0],
                            result: CalculationResult::Float(res.into()),
                        });
                    } else {
                        num.as_float().to_integer()?
                    }
                }
                _ => unimplemented!(),
            },
            Number::Int(num) => num.clone(),
        };
        if level > 0 {
            // Check if we can approximate the number of digits
            Some(
                if calc_num > *UPPER_APPROXIMATION_LIMIT
                    || (level > 1 && calc_num > UPPER_CALCULATION_LIMIT)
                {
                    let factorial =
                        math::approximate_multifactorial_digits(calc_num.clone(), level);
                    Calculation {
                        value: num,
                        levels: vec![level],
                        result: CalculationResult::ApproximateDigits(factorial),
                    }
                // Check if the number is within a reasonable range to compute
                } else if calc_num > UPPER_CALCULATION_LIMIT {
                    let factorial = math::approximate_factorial(calc_num.clone());
                    Calculation {
                        value: Number::Int(calc_num),
                        levels: vec![level],
                        result: CalculationResult::Approximate(factorial.0.into(), factorial.1),
                    }
                } else {
                    let calc_num = calc_num.to_u64().expect("Failed to convert BigInt to u64");
                    let factorial = math::factorial(calc_num, level);
                    Calculation {
                        value: num,
                        levels: vec![level],
                        result: CalculationResult::Exact(factorial),
                    }
                },
            )
        } else if level == -1 {
            if calc_num > *UPPER_APPROXIMATION_LIMIT {
                let factorial = math::approximate_multifactorial_digits(calc_num.clone(), 1);
                Some(Calculation {
                    value: num,
                    levels: vec![-1],
                    result: CalculationResult::ApproximateDigits(factorial),
                })
            } else if calc_num > UPPER_SUBFACTORIAL_LIMIT {
                let factorial = math::approximate_subfactorial(calc_num.clone());
                Some(Calculation {
                    value: num,
                    levels: vec![-1],
                    result: CalculationResult::Approximate(factorial.0.into(), factorial.1),
                })
            } else {
                let calc_num = calc_num.to_u64().expect("Failed to convert BigInt to u64");
                let factorial = math::subfactorial(calc_num);
                Some(Calculation {
                    value: num,
                    levels: vec![-1],
                    result: CalculationResult::Exact(factorial),
                })
            }
        } else if level == 0 {
            if calc_num > *UPPER_TERMIAL_APPROXIMATION_LIMIT {
                let termial = math::approximate_termial_digits(calc_num);
                Some(Calculation {
                    value: num,
                    levels: vec![0],
                    result: CalculationResult::ApproximateDigits(termial),
                })
            } else if calc_num > *UPPER_TERMIAL_LIMIT {
                let termial = math::approximate_termial(calc_num);
                Some(Calculation {
                    value: num,
                    levels: vec![0],
                    result: CalculationResult::Approximate(termial.0.into(), termial.1),
                })
            } else {
                let termial = math::termial(calc_num);
                Some(Calculation {
                    value: num,
                    levels: vec![0],
                    result: CalculationResult::Exact(termial),
                })
            }
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
