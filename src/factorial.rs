use crate::math;
use crate::reddit_comment::{NUMBER_DECIMALS_SCIENTIFIC, PLACEHOLDER};
use rug::Integer;
use std::fmt::{Error, Write};

// Limit for exact calculation, set to limit calculation time
pub(crate) const UPPER_CALCULATION_LIMIT: u64 = 1_000_000;
// Limit for approximation, set to ensure enough accuracy (aftewards, only single correct decimals have been observed)
pub(crate) const UPPER_APPROXIMATION_LIMIT: u64 = 500_000_000_000;
// Limit for number of digits approximation, set to prevent panics, due to the result overflowing u128
pub(crate) const UPPER_DIGIT_APPROXIMATION_LIMIT: u128 =
    1_000_000_000_000_000_000_000_000_000_000_000_000;

pub(crate) const UPPER_SUBFACTORIAL_LIMIT: u64 = 25_206;

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum CalculatedFactorial {
    Exact(Integer),
    Approximate(f64, u64),
    ApproximateDigits(u128),
}

#[derive(Debug, Clone, PartialEq, Ord, Eq, Hash, PartialOrd)]
pub(crate) struct Factorial {
    pub(crate) number: u128,
    pub(crate) level: i32,
    pub(crate) factorial: CalculatedFactorial,
}

pub(crate) struct FactorialTree {
    pub(crate) factorial: Factorial,
    pub(crate) children: Vec<FactorialTree>,
}

impl Ord for CalculatedFactorial {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        match (self, other) {
            (Self::Exact(this), Self::Exact(other)) => this.cmp(other),
            (Self::Exact(_), _) => std::cmp::Ordering::Greater,
            (Self::Approximate(this_base, this_exp), Self::Approximate(other_base, other_exp)) => {
                let exp_ord = this_exp.cmp(other_exp);
                let std::cmp::Ordering::Equal = exp_ord else {
                    return exp_ord;
                };
                this_base.total_cmp(other_base)
            }
            (Self::Approximate(_, _), _) => std::cmp::Ordering::Greater,
            (Self::ApproximateDigits(this), Self::ApproximateDigits(other)) => this.cmp(other),
            (Self::ApproximateDigits(_), _) => std::cmp::Ordering::Less,
        }
    }
}

impl PartialOrd for CalculatedFactorial {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Eq for CalculatedFactorial {}

impl std::hash::Hash for CalculatedFactorial {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        match self {
            Self::Exact(factorial) => {
                state.write_u8(1);
                factorial.hash(state);
            }
            Self::Approximate(base, exponent) => {
                state.write_u8(2);
                base.to_bits().hash(state);
                exponent.hash(state);
            }
            Self::ApproximateDigits(digits) => {
                state.write_u8(3);
                digits.hash(state);
            }
        }
    }
}

impl Factorial {
    pub(crate) fn format(
        &self,
        acc: &mut String,
        force_shorten: bool,
    ) -> Result<(), std::fmt::Error> {
        let factorial_level_string = Factorial::get_factorial_level_string(self.level);
        match &self.factorial {
            CalculatedFactorial::Exact(factorial) => {
                if self.is_too_long() || force_shorten {
                    self.truncate(acc, factorial_level_string, factorial)
                } else {
                    write!(
                        acc,
                        "{}{}{} is {} \n\n",
                        factorial_level_string, PLACEHOLDER, self.number, factorial
                    )
                }
            }
            CalculatedFactorial::Approximate(base, exponent) => {
                write!(
                    acc,
                    "{}{}{} is approximately {} \n\n",
                    factorial_level_string,
                    PLACEHOLDER,
                    self.number,
                    math::format_approximate_factorial((*base, *exponent))
                )
            }
            CalculatedFactorial::ApproximateDigits(digits) => {
                write!(
                    acc,
                    "{}{}{} has approximately {} digits \n\n",
                    factorial_level_string, PLACEHOLDER, self.number, digits
                )
            }
        }
    }

    fn truncate(
        &self,
        acc: &mut String,
        factorial_level_string: &str,
        factorial: &Integer,
    ) -> Result<(), Error> {
        let mut truncated_number = factorial.to_string();
        let length = truncated_number.len();
        truncated_number.truncate(NUMBER_DECIMALS_SCIENTIFIC + 2); // There is one digit before the decimals and the digit for rounding

        // Round if we had to truncate
        if truncated_number.len() >= NUMBER_DECIMALS_SCIENTIFIC + 2 {
            math::round(&mut truncated_number);
        };
        // Only add decimal if we have more than one digit
        if truncated_number.len() > 1 {
            truncated_number.insert(1, '.'); // Decimal point
        }
        if length > NUMBER_DECIMALS_SCIENTIFIC + 1 {
            write!(
                acc,
                "{}{}{} is roughly {} × 10^{} \n\n",
                factorial_level_string,
                PLACEHOLDER,
                self.number,
                truncated_number,
                length - 1
            )
        } else {
            write!(
                acc,
                "{}{}{} is {} \n\n",
                factorial_level_string, PLACEHOLDER, self.number, factorial
            )
        }
    }

    pub(crate) fn is_aproximate_digits(&self) -> bool {
        matches!(self.factorial, CalculatedFactorial::ApproximateDigits(_))
    }
    pub(crate) fn is_approximate(&self) -> bool {
        matches!(self.factorial, CalculatedFactorial::Approximate(_, _))
    }
    pub(crate) fn is_too_long(&self) -> bool {
        match self.level {
            1 => self.number > 3249,
            2 => self.number > 5982,
            3 => self.number > 8572,
            4 => self.number > 11077,
            5 => self.number > 13522,
            6 => self.number > 15920,
            7 => self.number > 18282,
            8 => self.number > 20613,
            9 => self.number > 22920,
            10 => self.number > 25208,
            11 => self.number > 27479,
            12 => self.number > 29735,
            13 => self.number > 31977,
            14 => self.number > 34207,
            15 => self.number > 36426,
            16 => self.number > 38635,
            17 => self.number > 40835,
            18 => self.number > 43027,
            19 => self.number > 45212,
            20 => self.number > 47390,
            21 => self.number > 49562,
            22 => self.number > 51728,
            23 => self.number > 53889,
            24 => self.number > 56045,
            25 => self.number > 58197,
            26 => self.number > 60345,
            27 => self.number > 62489,
            28 => self.number > 64630,
            29 => self.number > 66768,
            30 => self.number > 68903,
            31 => self.number > 71036,
            32 => self.number > 73167,
            33 => self.number > 75296,
            34 => self.number > 77423,
            35 => self.number > 79548,
            36 => self.number > 81672,
            37 => self.number > 83794,
            38 => self.number > 85915,
            39 => self.number > 88035,
            40 => self.number > 90154,
            41 => self.number > 92272,
            42 => self.number > 94389,
            43 => self.number > 96505,
            44 => self.number > 98620,
            45 => self.number > 100734,
            _ => false,
        }
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

#[cfg(test)]
mod tests {
    use super::*;
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
            number: 5,
            level: 1,
            factorial: CalculatedFactorial::Exact(Integer::from(120)),
        };
        factorial.format(&mut acc, false).unwrap();
        assert_eq!(acc, "The factorial of 5 is 120 \n\n");

        let mut acc = String::new();
        let factorial = Factorial {
            number: 5,
            level: -1,
            factorial: CalculatedFactorial::Exact(Integer::from(120)),
        };
        factorial.format(&mut acc, false).unwrap();
        assert_eq!(acc, "Subfactorial of 5 is 120 \n\n");

        let mut acc = String::new();
        let factorial = Factorial {
            number: 5,
            level: 1,
            factorial: CalculatedFactorial::Approximate(120.0, 3),
        };
        factorial.format(&mut acc, false).unwrap();
        assert_eq!(acc, "The factorial of 5 is approximately 1.2 × 10^5 \n\n");

        let mut acc = String::new();
        let factorial = Factorial {
            number: 5,
            level: 1,
            factorial: CalculatedFactorial::ApproximateDigits(3),
        };
        factorial.format(&mut acc, false).unwrap();
        assert_eq!(acc, "The factorial of 5 has approximately 3 digits \n\n");

        let mut acc = String::new();
        let factorial = Factorial {
            number: 5,
            level: 1,
            factorial: CalculatedFactorial::Exact(Integer::from(120)),
        };
        factorial.format(&mut acc, true).unwrap();
        assert_eq!(acc, "The factorial of 5 is 120 \n\n");
    }
}
