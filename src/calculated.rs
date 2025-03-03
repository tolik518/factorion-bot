//! This module handles the formatting of the calculations (`The factorial of Subfactorial of 5 is`, etc.)
use crate::math::{self, adjust_approximate_factorial, FLOAT_PRECISION};
use crate::pending::TOO_BIG_NUMBER;
use crate::reddit_comment::{NUMBER_DECIMALS_SCIENTIFIC, PLACEHOLDER};
use rug::float::OrdFloat;
use rug::ops::Pow;
use rug::{Float, Integer};
use std::fmt::Write;

#[derive(Debug, Clone, PartialEq, Ord, Eq, Hash, PartialOrd)]
pub(crate) enum CalculatedFactorial {
    Exact(Integer),
    Approximate(OrdFloat, Integer),
    ApproximateDigits(Integer),
}

#[derive(Debug, Clone, PartialEq, Ord, Eq, Hash, PartialOrd)]
pub(crate) struct Factorial {
    pub(crate) number: Integer,
    pub(crate) levels: Vec<i32>,
    pub(crate) factorial: CalculatedFactorial,
}

#[derive(Debug, Clone, PartialEq, Ord, Eq, Hash, PartialOrd)]
pub(crate) struct Gamma {
    pub(crate) number: OrdFloat,
    pub(crate) gamma: OrdFloat,
}

#[derive(Debug, Clone, PartialEq, Ord, Eq, Hash, PartialOrd)]
pub(crate) enum Calculation {
    Factorial(Factorial),
    Gamma(Gamma),
}

impl Calculation {
    pub(crate) fn format(
        &self,
        acc: &mut String,
        force_shorten: bool,
    ) -> Result<(), std::fmt::Error> {
        match self {
            Self::Factorial(fact) => fact.format(acc, force_shorten),
            Self::Gamma(gamma) => gamma.format(acc),
        }
    }
    pub(crate) fn is_aproximate_digits(&self) -> bool {
        matches!(
            self,
            Calculation::Factorial(Factorial {
                factorial: CalculatedFactorial::ApproximateDigits(_),
                ..
            })
        )
    }
    pub(crate) fn is_approximate(&self) -> bool {
        matches!(
            self,
            Calculation::Factorial(Factorial {
                factorial: CalculatedFactorial::Approximate(_, _),
                ..
            })
        )
    }
    pub(crate) fn is_too_long(&self) -> bool {
        match self {
            Self::Factorial(fact) => fact.is_too_long(),
            Self::Gamma(_) => false,
        }
    }
}

impl Factorial {
    pub(crate) fn format(
        &self,
        acc: &mut String,
        force_shorten: bool,
    ) -> Result<(), std::fmt::Error> {
        let factorial_string = self.levels.iter().rev().fold(String::new(), |a, e| {
            format!(
                "{}{}{}",
                a,
                Self::get_factorial_level_string(*e),
                PLACEHOLDER
            )
        });
        match &self.factorial {
            CalculatedFactorial::Exact(factorial) => {
                let factorial = if self.is_too_long() || force_shorten {
                    Self::truncate(factorial, true)
                } else {
                    factorial.to_string()
                };
                write!(
                    acc,
                    "{}{} is {} \n\n",
                    factorial_string, self.number, factorial
                )
            }
            CalculatedFactorial::Approximate(base, exponent) => {
                let (base, exponent) =
                    adjust_approximate_factorial((base.as_float().clone(), exponent.clone()));
                let exponent = if self.is_too_long() || force_shorten {
                    format!("({})", Self::truncate(&exponent, false))
                } else {
                    exponent.to_string()
                };
                let number = if self.number > *TOO_BIG_NUMBER || force_shorten {
                    Self::truncate(&self.number, false)
                } else {
                    self.number.to_string()
                };
                let base = base.to_f64();
                write!(
                    acc,
                    "{}{} is approximately {} × 10^{} \n\n",
                    factorial_string, number, base, exponent
                )
            }
            CalculatedFactorial::ApproximateDigits(digits) => {
                let digits = if self.is_too_long() || force_shorten {
                    Self::truncate(digits, false)
                } else {
                    digits.to_string()
                };
                let number = if self.number > *TOO_BIG_NUMBER || force_shorten {
                    Self::truncate(&self.number, false)
                } else {
                    self.number.to_string()
                };
                write!(
                    acc,
                    "{}{} has approximately {} digits \n\n",
                    factorial_string, number, digits
                )
            }
        }
    }

    fn truncate(number: &Integer, add_roughly: bool) -> String {
        let length = (Float::with_val(FLOAT_PRECISION, number).ln() / &*math::LN10)
            .to_integer_round(rug::float::Round::Down)
            .unwrap()
            .0;
        let truncated_number: Integer = number
            / (Float::with_val(FLOAT_PRECISION, 10)
                .pow((length.clone() - NUMBER_DECIMALS_SCIENTIFIC - 1u8).max(Integer::ZERO))
                .to_integer()
                .unwrap());
        let mut truncated_number = truncated_number.to_string();
        if truncated_number.len() > NUMBER_DECIMALS_SCIENTIFIC {
            math::round(&mut truncated_number);
        }
        if let Some(mut digit) = truncated_number.pop() {
            while digit == '0' {
                digit = match truncated_number.pop() {
                    Some(x) => x,
                    None => break,
                }
            }
            truncated_number.push(digit);
        }
        // Only add decimal if we have more than one digit
        if truncated_number.len() > 1 {
            truncated_number.insert(1, '.'); // Decimal point
        }
        if length > NUMBER_DECIMALS_SCIENTIFIC + 1 {
            format!(
                "{}{} × 10^{}",
                if add_roughly { "roughly " } else { "" },
                truncated_number,
                length
            )
        } else {
            number.to_string()
        }
    }

    pub(crate) fn is_too_long(&self) -> bool {
        let n = match &self.factorial {
            CalculatedFactorial::Exact(n)
            | CalculatedFactorial::ApproximateDigits(n)
            | CalculatedFactorial::Approximate(_, n) => n,
        };
        n > &*TOO_BIG_NUMBER
    }

    pub(crate) fn get_factorial_level_string(level: i32) -> &'static str {
        let prefix = match level {
            -1 => "Sub",
            1 => "The ",
            2 => "Double-",
            3 => "Triple-",
            4 => "Quadruple-",
            5 => "Quintuple-",
            6 => "Sextuple-",
            7 => "Septuple-",
            8 => "Octuple-",
            9 => "Nonuple-",
            10 => "Decuple-",
            11 => "Undecuple-",
            12 => "Duodecuple-",
            13 => "Tredecuple-",
            14 => "Quattuordecuple-",
            15 => "Quindecuple-",
            16 => "Sexdecuple-",
            17 => "Septendecuple-",
            18 => "Octodecuple-",
            19 => "Novemdecuple-",
            20 => "Vigintuple-",
            21 => "Unvigintuple-",
            22 => "Duovigintuple-",
            23 => "Trevigintuple-",
            24 => "Quattuorvigintuple-",
            25 => "Quinvigintuple-",
            26 => "Sexvigintuple-",
            27 => "Septenvigintuple-",
            28 => "Octovigintuple-",
            29 => "Novemvigintuple-",
            30 => "Trigintuple-",
            31 => "Untrigintuple-",
            32 => "Duotrigintuple-",
            33 => "Tretrigintuple-",
            34 => "Quattuortrigintuple-",
            35 => "Quintrigintuple-",
            36 => "Sextrigintuple-",
            37 => "Septentrigintuple-",
            38 => "Octotrigintuple-",
            39 => "Novemtrigintuple-",
            40 => "Quadragintuple-",
            41 => "Unquadragintuple-",
            42 => "Duoquadragintuple-",
            43 => "Trequadragintuple-",
            44 => "Quattuorquadragintuple-",
            45 => "Quinquadragintuple-",
            _ => {
                let mut suffix = String::new();
                write!(&mut suffix, "{}-", level).unwrap();
                Box::leak(suffix.into_boxed_str())
            }
        };

        prefix
    }
}

impl Gamma {
    pub(crate) fn format(&self, acc: &mut String) -> Result<(), std::fmt::Error> {
        write!(
            acc,
            "{}{}{} is approximately {} \n\n",
            Factorial::get_factorial_level_string(1),
            PLACEHOLDER,
            self.number.as_float(),
            self.gamma.as_float()
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use math::FLOAT_PRECISION;
    use rug::Integer;

    #[test]
    fn test_factorial_level_string() {
        assert_eq!(Factorial::get_factorial_level_string(1), "The ");
        assert_eq!(Factorial::get_factorial_level_string(2), "Double-");
        assert_eq!(Factorial::get_factorial_level_string(3), "Triple-");
        assert_eq!(
            Factorial::get_factorial_level_string(45),
            "Quinquadragintuple-"
        );
        assert_eq!(Factorial::get_factorial_level_string(50), "50-");
    }

    #[test]
    fn test_factorial_format() {
        let mut acc = String::new();
        let factorial = Factorial {
            number: 5.into(),
            levels: vec![1],
            factorial: CalculatedFactorial::Exact(Integer::from(120)),
        };
        factorial.format(&mut acc, false).unwrap();
        assert_eq!(acc, "The factorial of 5 is 120 \n\n");

        let mut acc = String::new();
        let factorial = Factorial {
            number: 5.into(),
            levels: vec![-1],
            factorial: CalculatedFactorial::Exact(Integer::from(120)),
        };
        factorial.format(&mut acc, false).unwrap();
        assert_eq!(acc, "Subfactorial of 5 is 120 \n\n");

        let mut acc = String::new();
        let factorial = Factorial {
            number: 5.into(),
            levels: vec![1],
            factorial: CalculatedFactorial::Approximate(
                Float::with_val(FLOAT_PRECISION, 120).into(),
                3.into(),
            ),
        };
        factorial.format(&mut acc, false).unwrap();
        assert_eq!(acc, "The factorial of 5 is approximately 1.2 × 10^5 \n\n");

        let mut acc = String::new();
        let factorial = Factorial {
            number: 5.into(),
            levels: vec![1],
            factorial: CalculatedFactorial::ApproximateDigits(3.into()),
        };
        factorial.format(&mut acc, false).unwrap();
        assert_eq!(acc, "The factorial of 5 has approximately 3 digits \n\n");

        let mut acc = String::new();
        let factorial = Factorial {
            number: 5.into(),
            levels: vec![1],
            factorial: CalculatedFactorial::Exact(Integer::from(120)),
        };
        factorial.format(&mut acc, true).unwrap();
        assert_eq!(acc, "The factorial of 5 is 120 \n\n");
    }
}
