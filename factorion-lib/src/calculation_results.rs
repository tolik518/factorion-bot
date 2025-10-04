//! This module handles the formatting of the calculations (`The factorial of Subfactorial of 5 is`, etc.)
use crate::FLOAT_PRECISION;

use crate::rug::float::OrdFloat;
use crate::rug::ops::{NegAssign, NotAssign, Pow};
use crate::rug::{Float, Integer};
use std::borrow::Cow;
use std::fmt;
use std::fmt::Write;
use std::sync::OnceLock;

pub mod recommended {
    pub const NUMBER_DECIMALS_SCIENTIFIC: usize = 30;
}
static NUMBER_DECIMALS_SCIENTIFIC: OnceLock<usize> = OnceLock::new();

use crate::AlreadyInit;
pub fn init(number_decimals_scientific: usize) -> Result<(), AlreadyInit> {
    static INITIALIZING: std::sync::Mutex<()> = std::sync::Mutex::new(());
    let _guard = INITIALIZING.lock();
    NUMBER_DECIMALS_SCIENTIFIC
        .set(number_decimals_scientific)
        .map_err(|_| AlreadyInit)?;
    Ok(())
}
pub fn init_default() -> Result<(), AlreadyInit> {
    init(recommended::NUMBER_DECIMALS_SCIENTIFIC)
}

impl fmt::Debug for CalculationResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fn truncate<T: fmt::Debug>(val: &T) -> String {
            let s = format!("{val:?}");
            if s.len() > 25 {
                format!("{}...", &s[..20])
            } else {
                s
            }
        }

        match self {
            CalculationResult::Exact(n) => write!(f, "Exact({})", truncate(n)),
            CalculationResult::Approximate(of, int) => {
                write!(
                    f,
                    "Approximate({}, {})",
                    truncate(&of.as_float()),
                    truncate(int)
                )
            }
            CalculationResult::ApproximateDigits(i, n) => {
                write!(f, "ApproximateDigits({}, {})", i, truncate(n))
            }
            CalculationResult::ApproximateDigitsTower(i, b, u, n) => {
                write!(
                    f,
                    "ApproximateDigitsTower({}, {}, {}, {})",
                    i,
                    b,
                    u,
                    truncate(n)
                )
            }
            CalculationResult::Float(of) => write!(f, "Float({})", truncate(&of.as_float())),
            CalculationResult::ComplexInfinity => write!(f, "ComplexInfinity"),
        }
    }
}

/// The result of a calculation in various formats.
#[derive(Clone, PartialEq, Ord, Eq, Hash, PartialOrd)]
pub enum CalculationResult {
    Exact(Integer),
    /// a * 10^b
    Approximate(OrdFloat, Integer),
    /// b digits (a is whether the number is negative)
    ApproximateDigits(bool, Integer),
    /// (^(c)10)^d digits (a is whether is negative, b is negative number of digits (super small))
    ApproximateDigitsTower(bool, bool, u32, Integer),
    Float(OrdFloat),
    ComplexInfinity,
}

/// A number in various formats. An alias of [CalculationResult].
pub type Number = CalculationResult;

impl Number {
    pub fn negate(&mut self) {
        match self {
            Self::Approximate(x, _) | Self::Float(x) => x.as_float_mut().neg_assign(),
            Self::Exact(n) => n.neg_assign(),
            Self::ApproximateDigitsTower(n, _, _, _) | Self::ApproximateDigits(n, _) => {
                n.not_assign()
            }
            Self::ComplexInfinity => {}
        }
    }
    pub fn is_too_long(&self, too_big_number: &Integer) -> bool {
        let n = match self {
            CalculationResult::Exact(n)
            | CalculationResult::ApproximateDigits(_, n)
            | CalculationResult::Approximate(_, n)
            | CalculationResult::ApproximateDigitsTower(_, _, _, n) => n,
            CalculationResult::Float(_) | CalculationResult::ComplexInfinity => return false,
        };
        n > too_big_number
    }
}
impl From<Integer> for Number {
    fn from(value: Integer) -> Self {
        Number::Exact(value)
    }
}
impl From<i32> for Number {
    fn from(value: i32) -> Self {
        Number::Exact(value.into())
    }
}
impl From<Float> for Number {
    fn from(value: Float) -> Self {
        Number::Float(value.into())
    }
}
impl std::fmt::Display for Number {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.format(f, f.alternate(), f.sign_minus())
    }
}

impl CalculationResult {
    /// Formats a number. \
    /// Shorten turns integers into scientific notation if that makes them shorter. \
    /// Aggressive enables tertation for towers.
    fn format(
        &self,
        f: &mut std::fmt::Formatter<'_>,
        shorten: bool,
        agressive: bool,
    ) -> std::fmt::Result {
        use std::fmt::Display;
        match &self {
            CalculationResult::Exact(factorial) => {
                if shorten {
                    f.write_str(&truncate(factorial, true))
                } else {
                    factorial.fmt(f)
                }
            }
            CalculationResult::Approximate(base, exponent) => {
                let base = base.as_float();
                if !base.to_f64().is_finite() {
                    f.write_fmt(format_args!("{base:.30}"))?;
                } else {
                    base.to_f64().fmt(f)?;
                };
                f.write_str(" × 10^")?;
                if shorten {
                    f.write_str("(")?;
                    f.write_str(&truncate(exponent, false))?;
                    f.write_str(")")
                } else {
                    exponent.fmt(f)
                }
            }
            CalculationResult::ApproximateDigits(_, digits) => {
                if shorten {
                    f.write_str(&truncate(digits, false))?;
                } else {
                    digits.fmt(f)?;
                }
                f.write_str(" digits")
            }
            CalculationResult::ApproximateDigitsTower(_, negative, depth, exponent) => {
                f.write_str(if *negative { "-" } else { "" })?;
                if !agressive {
                    if *depth > 0 {
                        f.write_fmt(format_args!("10^("))?;
                    }
                    if *depth > 1 {
                        f.write_str(&"10\\^".repeat(*depth as usize - 1))?;
                        f.write_str("(")?;
                    }
                    if shorten {
                        f.write_str(&truncate(exponent, false))?;
                    } else {
                        exponent.fmt(f)?;
                    }
                    if *depth > 1 {
                        f.write_str("\\)")?;
                    }
                    if *depth > 0 {
                        f.write_str(")")?;
                    }
                } else {
                    let mut extra = 0;
                    let mut exponent = Float::with_val(
                        *FLOAT_PRECISION
                            .get()
                            .expect("FLOAT_PRECISION unititialized, use init"),
                        exponent,
                    );
                    while exponent >= 10 {
                        extra += 1;
                        exponent = exponent.log10();
                    }
                    f.write_str("^(")?;
                    (depth + extra).fmt(f)?;
                    f.write_str(")10")?;
                }
                f.write_str(" digits")
            }
            CalculationResult::Float(gamma) => {
                if !gamma.as_float().to_f64().is_finite() {
                    f.write_fmt(format_args!("{:.30}", gamma.as_float()))
                } else {
                    gamma.as_float().to_f64().fmt(f)
                }
            }
            CalculationResult::ComplexInfinity => f.write_str("∞\u{0303}"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Ord, Eq, Hash, PartialOrd)]
pub struct Calculation {
    /// The base number
    pub value: Number,
    /// Steps taken during calculation (level, negation)
    pub steps: Vec<(i32, u32)>,
    pub result: CalculationResult,
}

impl Calculation {
    pub fn is_digit_tower(&self) -> bool {
        matches!(
            self,
            Calculation {
                result: CalculationResult::ApproximateDigitsTower(_, _, _, _),
                ..
            }
        )
    }
    pub fn is_aproximate_digits(&self) -> bool {
        matches!(
            self,
            Calculation {
                result: CalculationResult::ApproximateDigits(_, _),
                ..
            }
        )
    }
    pub fn is_approximate(&self) -> bool {
        matches!(
            self,
            Calculation {
                result: CalculationResult::Approximate(_, _),
                ..
            }
        )
    }
    pub fn is_rounded(&self) -> bool {
        matches!(
            self,
            Calculation {
                value: Number::Float(_),
                ..
            }
        ) && !matches!(
            self,
            Calculation {
                result: CalculationResult::Float(_),
                ..
            }
        )
    }
    pub fn is_too_long(&self, too_big_number: &Integer) -> bool {
        self.result.is_too_long(too_big_number) || self.value.is_too_long(too_big_number)
    }
    
    /// Checks if the result is a factorion (a number that equals the sum of the factorial of its digits).
    /// In base 10, only 1, 2, 145, and 40585 are factorions (proven).
    /// We only check for 145 and 40585 as 1 and 2 are trivial cases.
    /// Returns true if the result is 145 or 40585, false otherwise.
    pub fn is_factorion(&self) -> bool {
        if let CalculationResult::Exact(ref result_num) = self.result {
            // Only 145 and 40585 are interesting factorions in base 10
            *result_num == Integer::from(145) || *result_num == Integer::from(40585)
        } else {
            false
        }
    }
}

impl Calculation {
    /// Formats a Calcucation. \
    /// Force shorten shortens all integers, if that makes them smaller. \
    /// Agressive shorten replaces the description of what steps were taken with "All that of" and truns towers into tetration. \
    /// Too big number is from when the integer part automatically gets shortened.
    pub fn format(
        &self,
        acc: &mut String,
        force_shorten: bool,
        agressive_shorten: bool,
        too_big_number: &Integer,
    ) -> Result<(), std::fmt::Error> {
        let mut factorial_string = if !agressive_shorten {
            self.steps.iter().rev().fold(String::new(), |mut a, e| {
                let negative_str = if e.1 > 0 { "negative " } else { "" };
                let negative_strength = if e.1 > 1 {
                    format!("{}y ", Self::get_factorial_level_string(e.1 as i32))
                } else {
                    String::new()
                };
                let _ = match e.0 {
                    -1 => write!(a, "the {negative_strength}{negative_str}termial of "),
                    ..-1 => write!(
                        a,
                        "{}{}{}termial of ",
                        negative_strength,
                        negative_str,
                        Self::get_factorial_level_string(-e.0),
                    ),
                    1 => write!(a, "the {negative_strength}{negative_str}factorial of "),
                    0 | 2.. => write!(
                        a,
                        "{}{}{}factorial of ",
                        negative_strength,
                        negative_str,
                        Self::get_factorial_level_string(e.0),
                    ),
                };
                a
            })
        } else {
            "all that of ".to_string()
        };
        factorial_string[..1].make_ascii_uppercase();
        let number = &self.value;
        let is = if matches!(
            self.result,
            CalculationResult::ApproximateDigits(_, _)
                | CalculationResult::ApproximateDigitsTower(_, _, _, _)
        ) {
            "has"
        } else {
            "is"
        };
        use CalculationResult as Cr;
        let approximate = match (&self.result, &self.value) {
            (Cr::ApproximateDigitsTower(_, _, _, _), _) => " on the order of",
            (Cr::Approximate(_, _) | Cr::ApproximateDigits(_, _) | Cr::Float(_), _)
            | (_, Number::Float(_)) => " approximately",
            _ => "",
        };
        let factorial = &self.result;
        write!(acc, "{factorial_string}")?;
        if agressive_shorten {
            write!(acc, "{number:-#}")?
        } else if number.is_too_long(too_big_number) || force_shorten {
            write!(acc, "{number:#}")?
        } else {
            write!(acc, "{number}")?
        }
        write!(acc, " {is}{approximate} ")?;
        if agressive_shorten {
            write!(acc, "{factorial:-#}")?
        } else if factorial.is_too_long(too_big_number) || force_shorten {
            write!(acc, "{factorial:#}")?
        } else {
            write!(acc, "{factorial}")?
        }
        write!(acc, " \n\n")
    }

    fn get_factorial_level_string(level: i32) -> Cow<'static, str> {
        match level {
            0 => "sub".into(),
            1 => "the ".into(),
            2 => "double-".into(),
            3 => "triple-".into(),
            4 => "quadruple-".into(),
            5 => "quintuple-".into(),
            6 => "sextuple-".into(),
            7 => "septuple-".into(),
            8 => "octuple-".into(),
            9 => "nonuple-".into(),
            10 => "decuple-".into(),
            11 => "undecuple-".into(),
            12 => "duodecuple-".into(),
            13 => "tredecuple-".into(),
            14 => "quattuordecuple-".into(),
            15 => "quindecuple-".into(),
            16 => "sexdecuple-".into(),
            17 => "septendecuple-".into(),
            18 => "octodecuple-".into(),
            19 => "novemdecuple-".into(),
            20 => "vigintuple-".into(),
            21 => "unvigintuple-".into(),
            22 => "duovigintuple-".into(),
            23 => "trevigintuple-".into(),
            24 => "quattuorvigintuple-".into(),
            25 => "quinvigintuple-".into(),
            26 => "sexvigintuple-".into(),
            27 => "septenvigintuple-".into(),
            28 => "octovigintuple-".into(),
            29 => "novemvigintuple-".into(),
            30 => "trigintuple-".into(),
            31 => "untrigintuple-".into(),
            32 => "duotrigintuple-".into(),
            33 => "tretrigintuple-".into(),
            34 => "quattuortrigintuple-".into(),
            35 => "quintrigintuple-".into(),
            36 => "sextrigintuple-".into(),
            37 => "septentrigintuple-".into(),
            38 => "octotrigintuple-".into(),
            39 => "novemtrigintuple-".into(),
            40 => "quadragintuple-".into(),
            41 => "unquadragintuple-".into(),
            42 => "duoquadragintuple-".into(),
            43 => "trequadragintuple-".into(),
            44 => "quattuorquadragintuple-".into(),
            45 => "quinquadragintuple-".into(),
            _ => {
                let mut suffix = String::new();
                write!(&mut suffix, "{level}-").unwrap();
                suffix.into()
            }
        }
    }
}
/// Rounds a base 10 number string. \
/// Uses the last digit to decide the rounding direction. \
/// Rounds over 9s. This does **not** keep the length or turn rounded over digits into zeros. \
/// If the input is all 9s, this will round to 10. \
///
/// # Panic
/// This function may panic if less than two digits are supplied, or if it contains a non-digit of base 10.
fn round(number: &mut String) {
    // Check additional digit if we need to round
    if let Some(digit) = number
        .pop()
        .map(|n| n.to_digit(10).expect("Not a base 10 number"))
    {
        if digit >= 5 {
            let mut last_digit = number
                .pop()
                .and_then(|n| n.to_digit(10))
                .expect("Not a base 10 number");
            // Carry over at 9s
            while last_digit == 9 {
                let Some(digit) = number
                    .pop()
                    .map(|n| n.to_digit(10).expect("Not a base 10 number"))
                else {
                    // If we reached the end we get 10
                    number.push_str("10");
                    return;
                };
                last_digit = digit;
            }
            // Round up
            let _ = write!(number, "{}", last_digit + 1);
        }
    }
}
fn truncate(number: &Integer, add_roughly: bool) -> String {
    let prec = *FLOAT_PRECISION
        .get()
        .expect("FLOAT_PRECISION unititialized, use init");
    if number == &0 {
        return number.to_string();
    }
    let negative = number.is_negative();
    let orig_number = number;
    let number = number.clone().abs();
    let length = (Float::with_val(prec, &number).ln() / Float::with_val(prec, 10).ln())
        .to_integer_round(crate::rug::float::Round::Down)
        .unwrap()
        .0;
    let truncated_number: Integer = &number
        / (Float::with_val(prec, 10)
            .pow(
                (length.clone()
                    - NUMBER_DECIMALS_SCIENTIFIC
                        .get()
                        .expect("NUMBER_DECIMALS_SCIENTIFIC uninitialized, use init")
                    - 1u8)
                    .max(Integer::ZERO),
            )
            .to_integer()
            .unwrap());
    let mut truncated_number = truncated_number.to_string();
    if truncated_number.len()
        > *NUMBER_DECIMALS_SCIENTIFIC
            .get()
            .expect("NUMBER_DECIMALS_SCIENTIFIC uninitialized, use init")
    {
        round(&mut truncated_number);
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
    if negative {
        truncated_number.insert(0, '-');
    }
    if length
        > NUMBER_DECIMALS_SCIENTIFIC
            .get()
            .expect("NUMBER_DECIMALS_SCIENTIFIC uninitialized, use init")
            + 1
    {
        format!(
            "{}{} × 10^{}",
            if add_roughly { "roughly " } else { "" },
            truncated_number,
            length
        )
    } else {
        orig_number.to_string()
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::recommended::FLOAT_PRECISION;
    use crate::rug::Integer;
    use std::{str::FromStr, sync::LazyLock};

    static TOO_BIG_NUMBER: LazyLock<Integer> =
        LazyLock::new(|| Integer::from_str(&format!("1{}", "0".repeat(9999))).unwrap());

    #[test]
    fn test_round_down() {
        let _ = crate::init_default();
        let mut number = String::from("1929472373");
        round(&mut number);
        assert_eq!(number, "192947237");
    }

    #[test]
    fn test_round_up() {
        let _ = crate::init_default();
        let mut number = String::from("74836748625");
        round(&mut number);
        assert_eq!(number, "7483674863");
    }

    #[test]
    fn test_round_carry() {
        let _ = crate::init_default();
        let mut number = String::from("24999999995");
        round(&mut number);
        assert_eq!(number, "25");
    }

    #[test]
    fn test_factorial_level_string() {
        let _ = crate::init_default();
        assert_eq!(Calculation::get_factorial_level_string(1), "the ");
        assert_eq!(Calculation::get_factorial_level_string(2), "double-");
        assert_eq!(Calculation::get_factorial_level_string(3), "triple-");
        assert_eq!(
            Calculation::get_factorial_level_string(45),
            "quinquadragintuple-"
        );
        assert_eq!(Calculation::get_factorial_level_string(50), "50-");
    }

    #[test]
    fn test_truncate() {
        let _ = crate::init_default();
        assert_eq!(truncate(&Integer::from_str("0").unwrap(), false), "0");
        assert_eq!(truncate(&Integer::from_str("-1").unwrap(), false), "-1");
        assert_eq!(
            truncate(
                &Integer::from_str(&format!("1{}", "0".repeat(300))).unwrap(),
                false
            ),
            "1 × 10^300"
        );
        assert_eq!(
            truncate(
                &-Integer::from_str(&format!("1{}", "0".repeat(300))).unwrap(),
                false
            ),
            "-1 × 10^300"
        );
        assert_eq!(
            truncate(
                &Integer::from_str(&format!("1{}", "0".repeat(2000000))).unwrap(),
                false
            ),
            "1 × 10^2000000"
        );
    }

    #[test]
    fn test_factorial_format() {
        let _ = crate::init_default();
        let mut acc = String::new();
        let factorial = Calculation {
            value: 5.into(),
            steps: vec![(1, 0)],
            result: CalculationResult::Exact(Integer::from(120)),
        };
        factorial
            .format(&mut acc, false, false, &TOO_BIG_NUMBER)
            .unwrap();
        assert_eq!(acc, "The factorial of 5 is 120 \n\n");

        let mut acc = String::new();
        let factorial = Calculation {
            value: 5.into(),
            steps: vec![(0, 0)],
            result: CalculationResult::Exact(Integer::from(120)),
        };
        factorial
            .format(&mut acc, false, false, &TOO_BIG_NUMBER)
            .unwrap();
        assert_eq!(acc, "Subfactorial of 5 is 120 \n\n");

        let mut acc = String::new();
        let factorial = Calculation {
            value: 5.into(),
            steps: vec![(1, 0)],
            result: CalculationResult::Approximate(
                Float::with_val(FLOAT_PRECISION, 1.2).into(),
                5.into(),
            ),
        };
        factorial
            .format(&mut acc, false, false, &TOO_BIG_NUMBER)
            .unwrap();
        assert_eq!(acc, "The factorial of 5 is approximately 1.2 × 10^5 \n\n");

        let mut acc = String::new();
        let factorial = Calculation {
            value: 5.into(),
            steps: vec![(1, 0)],
            result: CalculationResult::ApproximateDigits(false, 3.into()),
        };
        factorial
            .format(&mut acc, false, false, &TOO_BIG_NUMBER)
            .unwrap();
        assert_eq!(acc, "The factorial of 5 has approximately 3 digits \n\n");

        let mut acc = String::new();
        let factorial = Calculation {
            value: 5.into(),
            steps: vec![(1, 0)],
            result: CalculationResult::Exact(Integer::from(120)),
        };
        factorial
            .format(&mut acc, true, false, &TOO_BIG_NUMBER)
            .unwrap();
        assert_eq!(acc, "The factorial of 5 is 120 \n\n");
    }
}

#[cfg(test)]
mod test {
    use std::{str::FromStr, sync::LazyLock};

    use super::*;

    use crate::recommended::FLOAT_PRECISION;
    static TOO_BIG_NUMBER: LazyLock<Integer> =
        LazyLock::new(|| Integer::from_str(&format!("1{}", "0".repeat(9999))).unwrap());

    // NOTE: The factorials here might be wrong, but we don't care, we are just testing the formatting

    #[test]
    fn test_format_factorial() {
        let _ = crate::init_default();
        let fact = Calculation {
            value: 10.into(),
            steps: vec![(3, 0)],
            result: CalculationResult::Exact(280.into()),
        };
        let mut s = String::new();
        fact.format(&mut s, false, false, &TOO_BIG_NUMBER).unwrap();
        assert_eq!(s, "Triple-factorial of 10 is 280 \n\n");
    }
    #[test]
    fn test_format_factorial_exact_of_decimal() {
        let _ = crate::init_default();
        let fact = Calculation {
            value: Number::Float(Float::with_val(FLOAT_PRECISION, 0.5).into()),
            steps: vec![(3, 0)],
            result: CalculationResult::Exact(280.into()),
        };
        let mut s = String::new();
        fact.format(&mut s, false, false, &TOO_BIG_NUMBER).unwrap();
        assert_eq!(s, "Triple-factorial of 0.5 is approximately 280 \n\n");
    }
    #[test]
    fn test_format_factorial_force_shorten_small() {
        let _ = crate::init_default();
        let fact = Calculation {
            value: 10.into(),
            steps: vec![(3, 0)],
            result: CalculationResult::Exact(280.into()),
        };
        let mut s = String::new();
        fact.format(&mut s, true, false, &TOO_BIG_NUMBER).unwrap();
        assert_eq!(s, "Triple-factorial of 10 is 280 \n\n");
    }
    #[test]
    fn test_format_factorial_force_shorten_large() {
        let _ = crate::init_default();
        let fact = Calculation {
            value: 100.into(),
            steps: vec![(1, 0)],
            result: CalculationResult::Exact(
                Integer::from_str("232019615953125000000000000000000").unwrap(),
            ),
        };
        let mut s = String::new();
        fact.format(&mut s, false, false, &TOO_BIG_NUMBER).unwrap();
        assert_eq!(
            s,
            "The factorial of 100 is 232019615953125000000000000000000 \n\n"
        );
    }
    #[test]
    fn test_format_factorial_auto_shorten() {
        let _ = crate::init_default();
        let fact = Calculation {
            value: 3249.into(),
            steps: vec![(1,0)],
            result: CalculationResult::Exact(
                Integer::from_str("64123376882765521838840963030568127691878727205333658692200854486404915724268122521695176119279253635876611090137291969570276913721864797759577004121543081865516901512445483449601769965060634861857064173938704305418376606356891014609023859758096597956259938348528946750437026172549655426092377089294607836520057856104816993984697675759579496157280331714452191401635250556082973306115574519424960196953201395066132365440977075392087489735146885581823595966673458107135041749084983583726462930633422893526599365244406644257808664472819062579590372326362859263197427382391737724371130194668325697913147795807287917882271125437793075279592752221056089408917956641344121781056494896664298954714463291743622978314854242079926982168325256172879601086193725507405771749789801611825741625380077209528888301112734777086106637653242107578812065387025070985682845983714635115865868052531038040737170581029905537322341939002838113744745962782070030988628668438192063964391415488312555937962867645737183703289987989371752808444472206166983181218698452231772212240017445423758860236449146575513014084114116542491422920779703202877962388772371297148878539228082497149672927873860981295756607109411429871735683677151117763870227460722732815888175758276344884954699572217509595160880510811349033936358665103889507929390456055037630508759624182491412136058522758117862715726418213812122827526330257260872329993280938592007320434494018056858434839424498517707440601396194949605570023576625190771463278168007414358018195714385208103590743168343592988436427551751120123934640886569178657972642734992568217335134536548423867468448461752994160896483162496996197629537563875663545967947035030506174219867102227347745166308776568259737417457622753953177779829173739659562549005900681020920836575654282170728038645671253311902327576757877160190593437037925134089334990083104974051379653937615220306281104735360028696101767109606466502484676624025302461421267416025443536877684785195571046059926349413586237838043863850610251583618438829618642246353724734656122845609571531588284708710081901687161770748138296656576032229319208279032435434327330035540657667361558905445221013396376775953367966087790302411507662731788873698999846238792500590360394500083923341408008981770566937535640769993694293230514231436990415482012055539596871513163008100690298424743718490882019179903258642028365049142613374709689558800856050749214398290563852574062566904927777093160819034619946818734041081848355062039645388238813669985569729968236449074797273410844560761607809842265309788155248298117938165414543689689754240992067831705834383207309250573018855640140957274364918049364842508738871690383100660359882462072065885517245667353800113210423157317762013988734352812105163694758108035856505778854524789188318600594132430921277654972526820920812190785994887939816114878915385423211996897729890266102145491069991647131611614465930571202528403443141981609375073983780241828798986101030035167624885608168623694530984934856402415662119456280967778213695343026782085453754332973412779641743296676142192492849866399186979810426206090031375249707803725234273693273721779240257093247268647749842459507965336971004339619911629224227060334233904444450352505466038312828689977755744971204784911189528493222070017894145493878499832441010771999957866634720057779638435426615168763950876432375766350648344132624416041623318009761058787995614968607413528076499437020919653085121078341947075546317831737787160036257151637941590867306372647047747729689844801136819011517526975033214302293538465503160183447374945622710595033673253137034231320031041035890947260824330728621640030383790059199531556893062561713763583025693789382680375603227866194301270004745201382665157844733507781537231595412109690534099208802055220457258238249940538761563309465648945964188442431661762589082015016756223358648046396366827537498425276338958018446839292802529780142385903309447658806351362744163752044896322012923382835852429065564336560491610071025646451525782856813152304143339115660276089535216189729579966851236899105440783686498435516601131545345163557980985342246336986737955743799192164259513473592703473521185371309681754246866522812455448210758136891890444056252857117200446002038652603259983493405505521897860879586618028713025173570291196046254005672495787117170419665767607647184551353826735583363126537373726390620854105626900247296291639985561481625404296348051054604042180512892657285238147263167051884385297470314430200590079012539964786079859359747123150407661818942489735756835032462952010303051169237940063644470670372188286551571968317499183600768353941744706305961785518398629201507525785967571188931895809109770264983907551256060144219899670118351808815620474425273993244741972143504134827047237929839845492209316520698259428270901257484509899386082594602760813392081897348940617781009158927227690469330327639146118508499255466535663882163793101115885899345523332216762566667486023534622719542192198250458735391090024294254053186440646305309340840685145289223131431157156390489399333752075193525158125680201419183806547205312873264380358849214095835479613319512867197427682723250079990981586869733293245764804577570764831692705888317075918673294669326798053736223321604803330275717540789920865913177228227111643923604665959921096208765542277777829882980225810940866410254096689483571105776785837917708633884075471298045453873223073787369262426626913405098535070631297346400765749139515252242178612533747493270131589184346851060077512732273563896936880596142362061341020737937605198462006142952423931616201569440226926787162077801883794168906567939864710313203688516686488132607069944238278930371283198545637735863991249832218463680910774912311493673518088306563853170521159963238305666024221618323515872866318153226269712890565361382209276094137857215708859605439920538254391240145615109307534437972388439697355227469268959991826344643967606862639207957142695059497774782782862380576527665249011786632721781635858363134217267161265609789721847126531549373639397319541419174824349828634414533913160986280670700117904134971824878639490677063427559640621162799757094469987184056964512589036737188936656494184932005003301076625555129466247988108160104882718140259576746243025950653945267030862681712132414998384138315991964228278130346276982182371619123375659027762342810200791337975076096607162500887202849331840711439619934443487228446573730294798389422723901661778354768525095757656920903185278358954945675520361768231577076750321654682566951617894418024879897723932943778739392625374786945631297844013055183788373235917906391604745846654356151085578611880261515860397623972021392725059655970516681719822949498069366408864396412928494605832710960284204937215373010567096882590065428759248976242854170628853902061231484918006271406155707387649451852150396381227895427254475130432845540997751264574249884576973754475522081887586009543117655192564603663203594121977491966995919938707026254622729082886656923266824175261927609862131917883084745112234024557978747561458733390353402381353061864973111801478933098174668694254024372053350135966105816774315863351432700501507214833910835095241116220945368287364828423032249431110250529198415073098056537298790818802403747860478015395740166511031245261193793854201285682331906071528112005073514650997116494101706639070013374677115821301361236988511929513457351929738018793684759539098410509535113338894579685309152120362751957602730649344150813012563246391457667149097699631546631367291707994927436193366185835774355812730356484690902974319470019544218388669048171395399380611906621586431005917959473642252829970939300283923684023821586277795276767391621510747281802893209607052311085173753725616353413592446675522238914835135290803927878090361225614843018882327106532840756094139114333346621153175254833577042328095480536834801026590432360931424294133543336408702705440236553526213058195627059654976746315636170233701887454392139871178240463495036735780991998499617099173145932919728906603992606395026374552882029156921168342421270810263586384930758466962518032019544198713384832174173447126633137813741748004660781750992387224960402183367639878315847417040125065349322346833085734948541674565230896990919815801676540094611430605654337096768783494147476599630304276589463660992695730097812987784061106253993478908686689107637583574009574525664941872851644555317421340687668414081763994364249671165252652825318436095248164540239487724330276498957490699548343852181838068378612444949106850962864407345130509165857647406496109100001533123176834579856292423765079015513705518869769002090306548513909235083737585930276738943593954668225536658208962591163051195501324651032924378645456520478535714079874404144783894706654731307268880764144813567558473827034967105368425271973138213726718055181321006250745589786136935583735915890517993411416086214277469794370188740010736604373520529352427775875772577651690552630708696044935360500197728514057299685757816479040563926362665221456966339198099627395349937057349473111399655105587183432516687910987518148931239145857422059143761070545360054386871218955184209375241453611589548642653321253873363792347807426924575722280463634222994099258528815002881358362491008896204800000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000").unwrap()
            ),
        };
        let mut s = String::new();
        fact.format(&mut s, false, false, &TOO_BIG_NUMBER).unwrap();
        assert_eq!(
            s,
            "The factorial of 3249 is roughly 6.412337688276552183884096303057 × 10^10000 \n\n"
        );
    }
    #[test]
    fn test_format_factorial_chain() {
        let _ = crate::init_default();
        let fact = Calculation {
            value: 5.into(),
            steps: vec![(3, 0), (1, 0)],
            result: CalculationResult::Exact(3628800.into()),
        };
        let mut s = String::new();
        fact.format(&mut s, false, false, &TOO_BIG_NUMBER).unwrap();
        assert_eq!(s, "The factorial of triple-factorial of 5 is 3628800 \n\n");
    }
    #[test]
    fn test_format_factorial_negative() {
        let _ = crate::init_default();
        let fact = Calculation {
            value: 0.into(),
            steps: vec![(1, 3)],
            result: CalculationResult::Exact(3628800.into()),
        };
        let mut s = String::new();
        fact.format(&mut s, false, false, &TOO_BIG_NUMBER).unwrap();
        assert_eq!(s, "The triple-y negative factorial of 0 is 3628800 \n\n");
        let fact = Calculation {
            value: 0.into(),
            steps: vec![(1, 1)],
            result: CalculationResult::Exact(3628800.into()),
        };
        let mut s = String::new();
        fact.format(&mut s, false, false, &TOO_BIG_NUMBER).unwrap();
        assert_eq!(s, "The negative factorial of 0 is 3628800 \n\n");
    }
    #[test]
    fn test_format_approximate_factorial() {
        let _ = crate::init_default();
        let fact = Calculation {
            value: 0.into(),
            steps: vec![(1, 0)],
            result: CalculationResult::Approximate(
                Float::with_val(FLOAT_PRECISION, 2.83947).into(),
                10043.into(),
            ),
        };
        let mut s = String::new();
        fact.format(&mut s, false, false, &TOO_BIG_NUMBER).unwrap();
        assert_eq!(
            s,
            "The factorial of 0 is approximately 2.83947 × 10^10043 \n\n"
        );
    }
    #[test]
    fn test_format_approximate_digits_factorial() {
        let _ = crate::init_default();
        let fact = Calculation {
            value: 0.into(),
            steps: vec![(1, 0)],
            result: CalculationResult::ApproximateDigits(false, 10043394.into()),
        };
        let mut s = String::new();
        fact.format(&mut s, false, false, &TOO_BIG_NUMBER).unwrap();
        assert_eq!(
            s,
            "The factorial of 0 has approximately 10043394 digits \n\n"
        );
    }
    #[test]
    fn test_format_complex_infinity_factorial() {
        let _ = crate::init_default();
        let fact = Calculation {
            value: 0.into(),
            steps: vec![(1, 0)],
            result: CalculationResult::ComplexInfinity,
        };
        let mut s = String::new();
        fact.format(&mut s, false, false, &TOO_BIG_NUMBER).unwrap();
        assert_eq!(s, "The factorial of 0 is ∞\u{0303} \n\n");
    }
    #[test]
    fn test_format_digits_tower() {
        let _ = crate::init_default();
        let fact = Calculation {
            value: 0.into(),
            steps: vec![(1, 0)],
            result: CalculationResult::ApproximateDigitsTower(false, false, 9, 10375.into()),
        };
        let mut s = String::new();
        fact.format(&mut s, false, false, &TOO_BIG_NUMBER).unwrap();
        assert_eq!(
            s,
            "The factorial of 0 has on the order of 10^(10\\^10\\^10\\^10\\^10\\^10\\^10\\^10\\^(10375\\)) digits \n\n"
        );
    }
    #[test]
    fn test_format_digits_tower_negative() {
        let _ = crate::init_default();
        let fact = Calculation {
            value: 0.into(),
            steps: vec![(1, 0)],
            result: CalculationResult::ApproximateDigitsTower(false, true, 9, 10375.into()),
        };
        let mut s = String::new();
        fact.format(&mut s, false, false, &TOO_BIG_NUMBER).unwrap();
        assert_eq!(
            s,
            "The factorial of 0 has on the order of -10^(10\\^10\\^10\\^10\\^10\\^10\\^10\\^10\\^(10375\\)) digits \n\n"
        );
    }
    #[test]
    fn test_format_digits_tower_tet() {
        let _ = crate::init_default();
        let fact = Calculation {
            value: 0.into(),
            steps: vec![(1, 0)],
            result: CalculationResult::ApproximateDigitsTower(false, false, 9, 10375.into()),
        };
        let mut s = String::new();
        fact.format(&mut s, false, true, &TOO_BIG_NUMBER).unwrap();
        assert_eq!(s, "All that of 0 has on the order of ^(10)10 digits \n\n");
    }
    #[test]
    fn test_format_gamma() {
        let _ = crate::init_default();
        let fact = Calculation {
            value: Number::Float(Float::with_val(FLOAT_PRECISION, 9.2).into()),
            steps: vec![(1, 0)],
            result: CalculationResult::Float(Float::with_val(FLOAT_PRECISION, 893.83924421).into()),
        };
        let mut s = String::new();
        fact.format(&mut s, false, false, &TOO_BIG_NUMBER).unwrap();
        assert_eq!(s, "The factorial of 9.2 is approximately 893.83924421 \n\n");
    }
    #[test]
    fn test_format_gamma_fallback() {
        let _ = crate::init_default();
        let fact = Calculation {
            value: Number::Float(Float::with_val(FLOAT_PRECISION, 0).into()),
            steps: vec![(1, 0)],
            result: {
                let mut m = Float::with_val(FLOAT_PRECISION, f64::MAX);
                m.next_up();
                CalculationResult::Float(m.into())
            },
        };
        let mut s = String::new();
        fact.format(&mut s, false, false, &TOO_BIG_NUMBER).unwrap();
        assert_eq!(
            s,
            "The factorial of 0 is approximately 179769313486231570000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000 \n\n"
        );
    }
    #[test]
    fn test_format_approximate_factorial_shorten() {
        let _ = crate::init_default();
        let fact = Calculation {
            value: Number::Exact(
                Integer::from_str("2018338437429423744923849374833232131").unwrap(),
            ),
            steps: vec![(1, 0)],
            result: CalculationResult::Approximate(
                Float::with_val(FLOAT_PRECISION, 2.8394792834).into(),
                Integer::from_str("10094283492304894983443984102489842984271").unwrap(),
            ),
        };
        let mut s = String::new();
        fact.format(&mut s, true, false, &TOO_BIG_NUMBER).unwrap();
        assert_eq!(
            s,
            "The factorial of roughly 2.018338437429423744923849374833 × 10^36 is approximately 2.8394792834 × 10^(1.009428349230489498344398410249 × 10^40) \n\n"
        );
    }
    #[test]
    fn test_format_approximate_digits_factorial_shorten() {
        let _ = crate::init_default();
        let fact = Calculation {
            value: Number::Exact(
                Integer::from_str("2313820948092579283573259490834298719").unwrap(),
            ),
            steps: vec![(1, 0)],
            result: CalculationResult::ApproximateDigits(
                false,
                Integer::from_str("9842371208573508275237815084709374240128347012847").unwrap(),
            ),
        };
        let mut s = String::new();
        fact.format(&mut s, true, false, &TOO_BIG_NUMBER).unwrap();
        assert_eq!(
            s,
            "The factorial of roughly 2.313820948092579283573259490834 × 10^36 has approximately 9.842371208573508275237815084709 × 10^48 digits \n\n"
        );
    }
    #[test]
    fn test_format_digits_tower_shorten() {
        let _ = crate::init_default();
        let fact = Calculation {
            value: Number::Exact(
                Integer::from_str("13204814708471087502685784603872164320053271").unwrap(),
            ),
            steps: vec![(1, 0)],
            result: CalculationResult::ApproximateDigitsTower(
                false,
                false,
                9,
                Integer::from_str("7084327410873502875032857120358730912469148632").unwrap(),
            ),
        };
        let mut s = String::new();
        fact.format(&mut s, true, false, &TOO_BIG_NUMBER).unwrap();
        assert_eq!(
            s,
            "The factorial of roughly 1.320481470847108750268578460387 × 10^43 has on the order of 10^(10\\^10\\^10\\^10\\^10\\^10\\^10\\^10\\^(7.084327410873502875032857120359 × 10^45\\)) digits \n\n"
        );
    }
    #[test]
    fn test_format_huge() {
        let _ = crate::init_default();
        let fact = Calculation {
            value: 0.into(),
            steps: vec![(1, 0)],
            result: CalculationResult::Exact({
                let mut r = Float::with_val(FLOAT_PRECISION, crate::rug::float::Special::Infinity);
                r.next_down();
                r.to_integer().unwrap()
            }),
        };
        let mut s = String::new();
        fact.format(&mut s, false, false, &TOO_BIG_NUMBER).unwrap();
        assert_eq!(
            s,
            "The factorial of 0 is roughly 2.098578716467387692404358116884 × 10^323228496 \n\n"
        );
    }

    #[test]
    fn test_calculation_is_approximate() {
        let _ = crate::init_default();
        let c1 = Calculation {
            value: 0.into(),
            steps: vec![],
            result: CalculationResult::Approximate(
                Float::with_val(FLOAT_PRECISION, 2.0).into(),
                1.into(),
            ),
        };
        assert!(c1.is_approximate());
        let c2 = Calculation {
            value: 0.into(),
            steps: vec![],
            result: CalculationResult::Exact(1.into()),
        };
        assert!(!c2.is_approximate());
    }

    #[test]
    fn test_calculation_is_rounded() {
        let _ = crate::init_default();
        let c1 = Calculation {
            value: Number::Float(Float::with_val(FLOAT_PRECISION, 1.23).into()),
            steps: vec![],
            result: CalculationResult::Approximate(
                Float::with_val(FLOAT_PRECISION, 0.0).into(),
                0.into(),
            ),
        };
        assert!(c1.is_rounded());
        let c2 = Calculation {
            value: Number::Float(Float::with_val(FLOAT_PRECISION, 1.23).into()),
            steps: vec![],
            result: CalculationResult::Float(Float::with_val(FLOAT_PRECISION, 1.23).into()),
        };
        assert!(!c2.is_rounded());
        let c3 = Calculation {
            value: 1.into(),
            steps: vec![],
            result: CalculationResult::Exact(1.into()),
        };
        assert!(!c3.is_rounded());
    }

    #[test]
    fn test_is_too_long() {
        let _ = crate::init_default();
        let small = Calculation {
            value: 1.into(),
            steps: vec![],
            result: CalculationResult::Exact(1.into()),
        };
        assert!(!small.is_too_long(&TOO_BIG_NUMBER));
        let big = Calculation {
            value: 1.into(),
            steps: vec![],
            result: CalculationResult::Exact((*TOO_BIG_NUMBER).clone() + 1),
        };
        assert!(big.is_too_long(&TOO_BIG_NUMBER));
        let fl = Calculation {
            value: 1.into(),
            steps: vec![],
            result: CalculationResult::Float(Float::with_val(FLOAT_PRECISION, 1.0).into()),
        };
        assert!(!fl.is_too_long(&TOO_BIG_NUMBER));
    }

    #[test]
    fn test_is_factorion_known_factorions() {
        let _ = crate::init_default();
        
        // Test known interesting factorions in base 10: 145, 40585
        // Note: 1 and 2 are also factorions but excluded as trivial cases

        // 145 is a factorion: 1! + 4! + 5! = 1 + 24 + 120 = 145
        let factorion_145 = Calculation {
            value: 145.into(),
            steps: vec![(1, 0)],
            result: CalculationResult::Exact(145.into()),
        };
        assert!(factorion_145.is_factorion());

        // 40585 is a factorion: 4! + 0! + 5! + 8! + 5! = 24 + 1 + 120 + 40320 + 120 = 40585
        let factorion_40585 = Calculation {
            value: 40585.into(),
            steps: vec![(1, 0)],
            result: CalculationResult::Exact(40585.into()),
        };
        assert!(factorion_40585.is_factorion());
    }

    #[test]
    fn test_is_factorion_non_factorions() {
        let _ = crate::init_default();
        
        // Test numbers that are NOT factorions
        
        // 1 and 2 are factorions but excluded as trivial
        let not_interesting_1 = Calculation {
            value: 1.into(),
            steps: vec![(1, 0)],
            result: CalculationResult::Exact(1.into()),
        };
        assert!(!not_interesting_1.is_factorion());

        let not_interesting_2 = Calculation {
            value: 2.into(),
            steps: vec![(1, 0)],
            result: CalculationResult::Exact(2.into()),
        };
        assert!(!not_interesting_2.is_factorion());
        
        // 3 is not a factorion: 3! = 6 ≠ 3
        let not_factorion_3 = Calculation {
            value: 3.into(),
            steps: vec![(1, 0)],
            result: CalculationResult::Exact(3.into()),
        };
        assert!(!not_factorion_3.is_factorion());

        // 120 is not a factorion: 1! + 2! + 0! = 1 + 2 + 1 = 4 ≠ 120
        let not_factorion_120 = Calculation {
            value: 120.into(),
            steps: vec![(1, 0)],
            result: CalculationResult::Exact(120.into()),
        };
        assert!(!not_factorion_120.is_factorion());

        // 144 is not a factorion: 1! + 4! + 4! = 1 + 24 + 24 = 49 ≠ 144
        let not_factorion_144 = Calculation {
            value: 144.into(),
            steps: vec![(1, 0)],
            result: CalculationResult::Exact(144.into()),
        };
        assert!(!not_factorion_144.is_factorion());
    }

    #[test]
    fn test_is_factorion_edge_cases() {
        let _ = crate::init_default();
        
        // Test edge cases
        
        // 0 is not a factorion
        let zero = Calculation {
            value: 0.into(),
            steps: vec![(1, 0)],
            result: CalculationResult::Exact(0.into()),
        };
        assert!(!zero.is_factorion());

        // Approximate result should return false
        let approximate = Calculation {
            value: 145.into(),
            steps: vec![(1, 0)],
            result: CalculationResult::Approximate(
                Float::with_val(FLOAT_PRECISION, 145.0).into(),
                0.into(),
            ),
        };
        assert!(!approximate.is_factorion());

        // Float result should return false
        let float_result = Calculation {
            value: 145.into(),
            steps: vec![(1, 0)],
            result: CalculationResult::Float(Float::with_val(FLOAT_PRECISION, 145.0).into()),
        };
        assert!(!float_result.is_factorion());
    }
}
