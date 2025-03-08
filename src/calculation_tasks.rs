//! This module handles the calulation of pending calculation tasks
use crate::calculation_results::Tower;
use crate::math::FLOAT_PRECISION;

use crate::{
    calculation_results::{CalculatedFactorial, Calculation, Factorial, Gamma},
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
    Gamma(GammaTask),
}
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct FactorialTask {
    pub(crate) base: CalculationBase,
    pub(crate) level: i32,
}
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum CalculationBase {
    Number(Integer),
    Calc(Box<CalculationJob>),
}
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct GammaTask {
    pub(crate) value: rug::float::OrdFloat,
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
            CalculationJob::Gamma(gamma) => {
                let res = math::fractional_factorial(gamma.value.as_float().clone());
                if res.is_finite() {
                    vec![Some(Calculation::Gamma(Gamma {
                        value: gamma.value,
                        gamma: res.into(),
                    }))]
                } else {
                    vec![None]
                }
            }
        }
    }
    pub fn get_depth(&self) -> usize {
        match self {
            Self::Factorial(fact) => fact.get_depth(),
            Self::Gamma(_) => 0,
        }
    }
}
impl FactorialTask {
    fn execute(self, include_steps: bool) -> Vec<Option<Calculation>> {
        let FactorialTask { base, level } = self;
        match base {
            CalculationBase::Number(num) => {
                vec![Self::calculate_appropriate_factorial(num, level).map(Calculation::Factorial)]
            }
            CalculationBase::Calc(factorial) => {
                let mut factorials = factorial.execute(include_steps);
                match factorials.last() {
                    Some(Some(Calculation::Factorial(Factorial {
                        factorial: res,
                        levels,
                        value: number,
                    }))) => {
                        let res = match res {
                            CalculatedFactorial::Exact(res) => res.clone(),
                            CalculatedFactorial::Approximate(base, exponent) => {
                                let res = base.as_float()
                                    * Float::with_val(FLOAT_PRECISION, 10).pow(exponent);
                                let Some(res) = res.to_integer() else {
                                    let base_levels = levels;
                                    let mut levels = vec![level];
                                    levels.extend(base_levels);
                                    return vec![Some(Calculation::Factorial(Factorial {
                                        value: number.clone(),
                                        levels,
                                        factorial: CalculatedFactorial::ApproximateDigitsTower(
                                            Tower {
                                                depth: 1,
                                                base: exponent.clone(),
                                            },
                                        ),
                                    }))];
                                };
                                res
                            }
                            CalculatedFactorial::ApproximateDigits(digits) => {
                                let base_levels = levels;
                                let mut levels = vec![level];
                                levels.extend(base_levels);
                                return vec![Some(Calculation::Factorial(Factorial {
                                    value: number.clone(),
                                    levels,
                                    factorial: CalculatedFactorial::ApproximateDigitsTower(Tower {
                                        depth: 1,
                                        base: digits.clone(),
                                    }),
                                }))];
                            }
                            CalculatedFactorial::ApproximateDigitsTower(Tower { depth, base }) => {
                                let base_levels = levels;
                                let mut levels = vec![level];
                                levels.extend(base_levels);
                                return vec![Some(Calculation::Factorial(Factorial {
                                    value: number.clone(),
                                    levels,
                                    factorial: CalculatedFactorial::ApproximateDigitsTower(Tower {
                                        depth: depth + 1,
                                        base: base.clone(),
                                    }),
                                }))];
                            }
                        };
                        let factorial = Self::calculate_appropriate_factorial(res, level)
                            .map(|mut res| {
                                let current_levels = res.levels;
                                res.levels = levels.clone();
                                res.levels.extend(current_levels);
                                res.value = number.clone();
                                res
                            })
                            .map(Calculation::Factorial);
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
    fn calculate_appropriate_factorial(num: Integer, level: i32) -> Option<Factorial> {
        if level > 0 {
            // Check if we can approximate the number of digits
            Some(
                if num > *UPPER_APPROXIMATION_LIMIT || (level > 1 && num > UPPER_CALCULATION_LIMIT)
                {
                    let factorial = math::approximate_multifactorial_digits(num.clone(), level);
                    Factorial {
                        value: num,
                        levels: vec![level],
                        factorial: CalculatedFactorial::ApproximateDigits(factorial),
                    }
                // Check if the number is within a reasonable range to compute
                } else if num > UPPER_CALCULATION_LIMIT {
                    let factorial = math::approximate_factorial(num.clone());
                    Factorial {
                        value: num,
                        levels: vec![level],
                        factorial: CalculatedFactorial::Approximate(
                            factorial.0.into(),
                            factorial.1,
                        ),
                    }
                } else {
                    let calc_num = num.to_u64().expect("Failed to convert BigInt to u64");
                    let factorial = math::factorial(calc_num, level);
                    Factorial {
                        value: num,
                        levels: vec![level],
                        factorial: CalculatedFactorial::Exact(factorial),
                    }
                },
            )
        } else if level == -1 {
            if num > *UPPER_APPROXIMATION_LIMIT {
                let factorial = math::approximate_multifactorial_digits(num.clone(), 1);
                Some(Factorial {
                    value: num,
                    levels: vec![-1],
                    factorial: CalculatedFactorial::ApproximateDigits(factorial),
                })
            } else if num > UPPER_SUBFACTORIAL_LIMIT {
                let factorial = math::approximate_subfactorial(num.clone());
                Some(Factorial {
                    value: num,
                    levels: vec![-1],
                    factorial: CalculatedFactorial::Approximate(factorial.0.into(), factorial.1),
                })
            } else {
                let calc_num = num.to_u64().expect("Failed to convert BigInt to u64");
                let factorial = math::subfactorial(calc_num);
                Some(Factorial {
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
            CalculationBase::Number(_) => 0,
            CalculationBase::Calc(calc) => calc.get_depth() + 1,
        }
    }
}
