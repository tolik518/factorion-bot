//! This module handles the calulation of pending calculation tasks
use crate::calculation_results::Number;
use crate::math::FLOAT_PRECISION;

use crate::{
    calculation_results::{CalculatedFactorial, Calculation},
    math,
};

use rug::{ops::Pow, Float, Integer};
use std::{str::FromStr, sync::LazyLock};

// Limit for exact calculation, set to limit calculation time
pub(crate) const UPPER_CALCULATION_LIMIT: u64 = 1_000_000;
// Limit for approximation, set to ensure enough accuracy (5 decimals)
pub(crate) static UPPER_APPROXIMATION_LIMIT: LazyLock<Integer> = LazyLock::new(|| {
    Integer::from_str("1000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000").unwrap()
});
// Limit for exact subfactorial calculation, set to limit calculation time
pub(crate) const UPPER_SUBFACTORIAL_LIMIT: u64 = 1_000_000;

pub(crate) static TOO_BIG_NUMBER: LazyLock<Integer> =
    LazyLock::new(|| Integer::from_str(&format!("1{}", "0".repeat(9999))).unwrap());

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum CalculationJob {
    Factorial(FactorialTask),
}
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct FactorialTask {
    pub(crate) base: CalculationBase,
    pub(crate) level: i32,
}
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum CalculationBase {
    Num(Number),
    Calc(Box<CalculationJob>),
}

impl CalculationJob {
    pub fn is_part_of(&self, other: &Self) -> bool {
        self == other
            || match other {
                Self::Factorial(FactorialTask {
                    base: CalculationBase::Calc(inner),
                    level: _,
                }) => self.is_part_of(inner),
                _ => false,
            }
    }
    pub fn execute(self, include_steps: bool) -> Vec<Option<Calculation>> {
        match self {
            CalculationJob::Factorial(fact) => fact.execute(include_steps).into_iter().collect(),
        }
    }
    pub fn get_depth(&self) -> usize {
        match self {
            Self::Factorial(fact) => fact.get_depth(),
        }
    }
}
impl FactorialTask {
    fn execute(self, include_steps: bool) -> Vec<Option<Calculation>> {
        let FactorialTask { base, level } = self;
        match base {
            CalculationBase::Num(num) => {
                vec![Self::calculate_appropriate_factorial(num, level)]
            }
            CalculationBase::Calc(factorial) => {
                let mut factorials = factorial.execute(include_steps);
                match factorials.last() {
                    Some(Some(Calculation {
                        factorial: res,
                        levels,
                        value: number,
                    })) => {
                        let res = match res {
                            CalculatedFactorial::Exact(res) => Number::Int(res.clone()),
                            CalculatedFactorial::Approximate(base, exponent) => {
                                let res = base.as_float()
                                    * Float::with_val(FLOAT_PRECISION, 10).pow(exponent);
                                let Some(res) = res.to_integer() else {
                                    let base_levels = levels;
                                    let mut levels = vec![level];
                                    levels.extend(base_levels);
                                    return vec![Some(Calculation {
                                        value: number.clone(),
                                        levels,
                                        factorial: CalculatedFactorial::ApproximateDigitsTower(
                                            1,
                                            exponent.clone() + math::length(exponent),
                                        ),
                                    })];
                                };
                                Number::Int(res)
                            }
                            CalculatedFactorial::ApproximateDigits(digits) => {
                                let base_levels = levels;
                                let mut levels = vec![level];
                                levels.extend(base_levels);
                                return vec![Some(Calculation {
                                    value: number.clone(),
                                    levels,
                                    factorial: CalculatedFactorial::ApproximateDigitsTower(
                                        1,
                                        digits.clone() + math::length(digits),
                                    ),
                                })];
                            }
                            CalculatedFactorial::ApproximateDigitsTower(depth, exponent) => {
                                let base_levels = levels;
                                let mut levels = vec![level];
                                levels.extend(base_levels);
                                let mut extra = if depth < &5 {
                                    Float::with_val(FLOAT_PRECISION, exponent)
                                } else {
                                    Float::new(FLOAT_PRECISION)
                                };
                                'calc_extra: for _ in 0..*depth {
                                    if extra < 1 {
                                        break 'calc_extra;
                                    }
                                    extra = extra.log10();
                                }
                                return vec![Some(Calculation {
                                    value: number.clone(),
                                    levels,
                                    factorial: CalculatedFactorial::ApproximateDigitsTower(
                                        depth + 1,
                                        exponent.clone()
                                            + extra
                                                .to_integer_round(rug::float::Round::Down)
                                                .map(|(n, _)| n)
                                                .unwrap_or(0.into()),
                                    ),
                                })];
                            }
                            CalculatedFactorial::Gamma(gamma) => Number::Float(gamma.clone()),
                        };
                        let factorial =
                            Self::calculate_appropriate_factorial(res, level).map(|mut res| {
                                let current_levels = res.levels;
                                res.levels = levels.clone();
                                res.levels.extend(current_levels);
                                res.value = number.clone();
                                res
                            });
                        if include_steps {
                            factorials.push(factorial);
                        } else {
                            factorials = vec![factorial];
                        }
                    }
                    _ => return factorials,
                };
                factorials
            }
        }
    }
    fn calculate_appropriate_factorial(num: Number, level: i32) -> Option<Calculation> {
        let calc_num = match &num {
            Number::Float(num) => {
                let res = math::fractional_factorial(num.as_float().clone());
                if res.is_finite() {
                    return Some(Calculation {
                        value: Number::Float(num.clone()),
                        levels: vec![1],
                        factorial: CalculatedFactorial::Gamma(res.into()),
                    });
                } else {
                    num.as_float().to_integer()?
                }
            }
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
                        factorial: CalculatedFactorial::ApproximateDigits(factorial),
                    }
                // Check if the number is within a reasonable range to compute
                } else if calc_num > UPPER_CALCULATION_LIMIT {
                    let factorial = math::approximate_factorial(calc_num.clone());
                    Calculation {
                        value: Number::Int(calc_num),
                        levels: vec![level],
                        factorial: CalculatedFactorial::Approximate(
                            factorial.0.into(),
                            factorial.1,
                        ),
                    }
                } else {
                    let calc_num = calc_num.to_u64().expect("Failed to convert BigInt to u64");
                    let factorial = math::factorial(calc_num, level);
                    Calculation {
                        value: num,
                        levels: vec![level],
                        factorial: CalculatedFactorial::Exact(factorial),
                    }
                },
            )
        } else if level == -1 {
            if calc_num > *UPPER_APPROXIMATION_LIMIT {
                let factorial = math::approximate_multifactorial_digits(calc_num.clone(), 1);
                Some(Calculation {
                    value: num,
                    levels: vec![-1],
                    factorial: CalculatedFactorial::ApproximateDigits(factorial),
                })
            } else if calc_num > UPPER_SUBFACTORIAL_LIMIT {
                let factorial = math::approximate_subfactorial(calc_num.clone());
                Some(Calculation {
                    value: num,
                    levels: vec![-1],
                    factorial: CalculatedFactorial::Approximate(factorial.0.into(), factorial.1),
                })
            } else {
                let calc_num = calc_num.to_u64().expect("Failed to convert BigInt to u64");
                let factorial = math::subfactorial(calc_num);
                Some(Calculation {
                    value: num,
                    levels: vec![-1],
                    factorial: CalculatedFactorial::Exact(factorial),
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
