//! This module handles the calulation of pending calculations
use crate::{
    calculated::{CalculatedFactorial, Calculation, Factorial, Gamma},
    math::{self, FLOAT_PRECISION},
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
pub enum PendingCalculation {
    Factorial(PendingFactorial),
    Gamma(PendingGamma),
}
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct PendingFactorial {
    pub(crate) base: PendingFactorialBase,
    pub(crate) level: i32,
}
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum PendingFactorialBase {
    Number(Integer),
    Factorial(Box<PendingFactorial>),
}
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct PendingGamma {
    pub(crate) number: rug::float::OrdFloat,
}

impl PendingCalculation {
    pub fn part_of(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Factorial(this), Self::Factorial(other)) => this.part_of(other),
            _ => false,
        }
    }
    pub fn calculate(self, include_steps: bool) -> Vec<Option<Calculation>> {
        match self {
            PendingCalculation::Factorial(fact) => fact
                .calculate(include_steps)
                .into_iter()
                .map(|x| x.map(Calculation::Factorial))
                .collect(),
            PendingCalculation::Gamma(gamma) => {
                let res = math::fractional_factorial(gamma.number.as_float().clone());
                if res.is_finite() {
                    vec![Some(Calculation::Gamma(Gamma {
                        number: gamma.number,
                        gamma: res.into(),
                    }))]
                } else {
                    vec![None]
                }
            }
        }
    }
}
impl PendingFactorial {
    pub fn part_of(&self, mut other: &Self) -> bool {
        if self == other {
            return true;
        }
        while let PendingFactorialBase::Factorial(base) = &other.base {
            other = base;
            if self == other {
                return true;
            }
        }
        false
    }
    fn calculate(self, include_steps: bool) -> Vec<Option<Factorial>> {
        let PendingFactorial { base, level } = self;
        match base {
            PendingFactorialBase::Number(num) => {
                vec![Self::calculate_appropriate_factorial(num, level)]
            }
            PendingFactorialBase::Factorial(factorial) => {
                let mut factorials = factorial.calculate(include_steps);
                match factorials.last() {
                    Some(Some(Factorial {
                        factorial: res,
                        levels,
                        number,
                    })) => {
                        let res = match res {
                            CalculatedFactorial::Exact(res) => res.clone(),
                            CalculatedFactorial::Approximate(base, exponent) => {
                                let res = base.as_float()
                                    * Float::with_val(FLOAT_PRECISION, 10).pow(exponent);
                                let Some(res) = res.to_integer() else {
                                    return factorials;
                                };
                                res
                            }
                            _ => return factorials,
                        };
                        let factorial =
                            Self::calculate_appropriate_factorial(res, level).map(|mut res| {
                                let current_levels = res.levels;
                                res.levels = levels.clone();
                                res.levels.extend(current_levels);
                                res.number = number.clone();
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
    fn calculate_appropriate_factorial(num: Integer, level: i32) -> Option<Factorial> {
        if level > 0 {
            // Check if we can approximate the number of digits
            Some(
                if num > *UPPER_APPROXIMATION_LIMIT || (level > 1 && num > UPPER_CALCULATION_LIMIT)
                {
                    let factorial = math::approximate_multifactorial_digits(num.clone(), level);
                    Factorial {
                        number: num,
                        levels: vec![level],
                        factorial: CalculatedFactorial::ApproximateDigits(factorial),
                    }
                // Check if the number is within a reasonable range to compute
                } else if num > UPPER_CALCULATION_LIMIT {
                    let factorial = math::approximate_factorial(num.clone());
                    Factorial {
                        number: num,
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
                        number: num,
                        levels: vec![level],
                        factorial: CalculatedFactorial::Exact(factorial),
                    }
                },
            )
        } else if level == -1 {
            if num > *UPPER_APPROXIMATION_LIMIT {
                let factorial = math::approximate_multifactorial_digits(num.clone(), 1);
                Some(Factorial {
                    number: num,
                    levels: vec![-1],
                    factorial: CalculatedFactorial::ApproximateDigits(factorial),
                })
            } else if num > UPPER_SUBFACTORIAL_LIMIT {
                let factorial = math::approximate_subfactorial(num.clone());
                Some(Factorial {
                    number: num,
                    levels: vec![-1],
                    factorial: CalculatedFactorial::Approximate(factorial.0.into(), factorial.1),
                })
            } else {
                let calc_num = num.to_u64().expect("Failed to convert BigInt to u64");
                let factorial = math::subfactorial(calc_num);
                Some(Factorial {
                    number: num,
                    levels: vec![-1],
                    factorial: CalculatedFactorial::Exact(factorial),
                })
            }
        } else {
            None
        }
    }
}
