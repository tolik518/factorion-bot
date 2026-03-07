//! This module holds the underlying formatting functions used in [`calculation_result`]
use crate::{Consts, locale};
use factorion_math::rug::{Float, Integer, ops::Pow};
use std::{borrow::Cow, fmt::Write};

const SINGLES: [&str; 10] = [
    "", "un", "duo", "tre", "quattuor", "quin", "sex", "septen", "octo", "novem",
];
const SINGLES_LAST: [&str; 10] = [
    "", "un", "du", "tr", "quadr", "quint", "sext", "sept", "oct", "non",
];
const TENS: [&str; 10] = [
    "",
    "dec",
    "vigin",
    "trigin",
    "quadragin",
    "quinquagin",
    "sexagin",
    "septuagin",
    "octogin",
    "nonagin",
];
const HUNDREDS: [&str; 10] = [
    "",
    "cen",
    "ducen",
    // Note this differs from the wikipedia list to disambiguate from 103, which continuing the pattern should be trecentuple
    "tricen",
    "quadringen",
    "quingen",
    "sescen",
    "septingen",
    "octingen",
    "nongen",
];
// Note that other than milluple, these are not found in a list, but continue the pattern from mill with different starts
const THOUSANDS: [&str; 10] = [
    "", "mill", "bill", "trill", "quadrill", "quintill", "sextill", "septill", "octill", "nonill",
];
const TEN_THOUSANDS: [&str; 10] = [
    "",
    "decill",
    "vigintill",
    "trigintill",
    "quadragintill",
    "quinquagintill",
    "sexagintill",
    "septuagintill",
    "octogintill",
    "nonagintill",
];
const HUNDRED_THOUSANDS: [&str; 10] = [
    "",
    "centill",
    "ducentill",
    "tricentill",
    "quadringentill",
    "quingentill",
    "sescentill",
    "septingentill",
    "octingentill",
    "nongentill",
];
const BINDING_T: [[bool; 10]; 6] = [
    // Singles
    [
        false, false, false, false, false, false, false, false, false, false,
    ],
    // Tens
    [false, false, true, true, true, true, true, true, true, true],
    // Hundreds
    [false, true, true, true, true, true, true, true, true, true],
    // Thousands
    [
        false, false, false, false, false, false, false, false, false, false,
    ],
    // Tenthousands
    [
        false, false, false, false, false, false, false, false, false, false,
    ],
    // Hundredthousands
    [
        false, false, false, false, false, false, false, false, false, false,
    ],
];
pub fn get_factorial_level_string<'a>(level: i32, locale: &'a locale::Format<'a>) -> Cow<'a, str> {
    if let Some(s) = locale.num_overrides().get(&level) {
        return s.as_ref().into();
    }
    match level {
        0 => locale.sub().as_ref().into(),
        1 => "{factorial}".into(),
        ..=999999 if !locale.force_num() => {
            let singles = if level < 10 { SINGLES_LAST } else { SINGLES };
            let mut acc = String::new();
            let mut n = level;
            let s = n % 10;
            n /= 10;
            acc.write_str(singles[s as usize]).unwrap();
            let t = n % 10;
            n /= 10;
            acc.write_str(TENS[t as usize]).unwrap();
            let h = n % 10;
            n /= 10;
            acc.write_str(HUNDREDS[h as usize]).unwrap();
            let th = n % 10;
            n /= 10;
            acc.write_str(THOUSANDS[th as usize]).unwrap();
            let tth = n % 10;
            n /= 10;
            acc.write_str(TEN_THOUSANDS[tth as usize]).unwrap();
            let hth = n % 10;
            acc.write_str(HUNDRED_THOUSANDS[hth as usize]).unwrap();
            // Check if we need tuple not uple
            let last_written = [s, t, h, th, tth, hth]
                .iter()
                .cloned()
                .enumerate()
                .rev()
                .find(|(_, n)| *n != 0)
                .unwrap();
            if BINDING_T[last_written.0][last_written.1 as usize] {
                acc.write_str("t").unwrap();
            }
            acc.write_str(locale.uple()).unwrap();

            acc.into()
        }
        _ => {
            let mut suffix = String::new();
            write!(&mut suffix, "{level}-{{factorial}}").unwrap();
            suffix.into()
        }
    }
}
const EN_SINGLES: [&str; 10] = [
    "", "one ", "two ", "three ", "four ", "five ", "six ", "seven ", "eight ", "nine ",
];
const EN_TENS: [&str; 10] = [
    "", "ten ", "twenty ", "thirty ", "forty ", "fivety ", "sixty ", "seventy ", "eighty ",
    "ninety ",
];
const EN_TENS_SINGLES: [&str; 10] = [
    "ten ",
    "eleven ",
    "twelve ",
    "thirteen ",
    "fourteen ",
    "fiveteen ",
    "sixteen ",
    "seventeen ",
    "eighteen ",
    "nineteen ",
];
const SINGLES_LAST_ILLION: [&str; 10] = [
    "", "m", "b", "tr", "quadr", "quint", "sext", "sept", "oct", "non",
];
// TODO: localize (illion, illiard, type of scale, numbers, digit order, thousand)
pub fn write_out_number(acc: &mut String, num: &Integer, consts: &Consts) -> std::fmt::Result {
    if num == &0 {
        return acc.write_str("zero");
    }
    let negative = num < &0;
    let num = Float::with_val(consts.float_precision, num).abs();
    let ten = Float::with_val(consts.float_precision, 10);
    let digit_blocks = num
        .clone()
        .log10()
        .to_u32_saturating_round(factorion_math::rug::float::Round::Down)
        .unwrap()
        / 3;
    if negative {
        acc.write_str("minus ")?;
    }
    for digit_blocks_left in (digit_blocks.saturating_sub(5)..=digit_blocks).rev() {
        let current_digits = Float::to_u32_saturating_round(
            &((num.clone() / ten.clone().pow(digit_blocks_left * 3)) % 1000),
            factorion_math::rug::float::Round::Down,
        )
        .unwrap();
        let mut n = current_digits;
        let s = n % 10;
        n /= 10;
        let t = n % 10;
        n /= 10;
        let h = n % 10;
        acc.write_str(EN_SINGLES[h as usize])?;
        if h != 0 {
            acc.write_str("hundred ")?;
        }
        if t == 1 {
            acc.write_str(EN_TENS_SINGLES[s as usize])?;
        } else {
            acc.write_str(EN_TENS[t as usize])?;
            acc.write_str(EN_SINGLES[s as usize])?;
        }

        if digit_blocks_left > 0 && current_digits != 0 {
            let singles = if digit_blocks_left < 10 {
                SINGLES_LAST_ILLION
            } else {
                SINGLES
            };
            let mut n = digit_blocks_left - 1;
            if n > 0 {
                let s = n % 10;
                n /= 10;
                acc.write_str(singles[s as usize]).unwrap();
                let t = n % 10;
                n /= 10;
                acc.write_str(TENS[t as usize]).unwrap();
                let h = n % 10;
                n /= 10;
                acc.write_str(HUNDREDS[h as usize]).unwrap();
                let th = n % 10;
                n /= 10;
                acc.write_str(THOUSANDS[th as usize]).unwrap();
                let tth = n % 10;
                n /= 10;
                acc.write_str(TEN_THOUSANDS[tth as usize]).unwrap();
                let hth = n % 10;
                acc.write_str(HUNDRED_THOUSANDS[hth as usize]).unwrap();
                // Check if we need tuple not uple
                let last_written = [s, t, h, th, tth, hth]
                    .iter()
                    .cloned()
                    .enumerate()
                    .rev()
                    .find(|(_, n)| *n != 0)
                    .unwrap();
                if BINDING_T[last_written.0][last_written.1 as usize] {
                    acc.write_str("t").unwrap();
                }
                acc.write_str("illion ")?;
            } else {
                acc.write_str("thousand ")?;
            }
        }
    }
    acc.pop();
    Ok(())
}
/// Rounds a base 10 number string. \
/// Uses the last digit to decide the rounding direction. \
/// Rounds over 9s. This does **not** keep the length or turn rounded over digits into zeros. \
/// If the input is all 9s, this will round to 10. \
/// Stops when a decimal period is encountered, removing it.
///
/// Returns whether it overflowed.
///
/// # Panic
/// This function may panic if less than two digits are supplied, or if it contains a non-digit of base 10, that is not a period.
pub fn round(number: &mut String) -> bool {
    // Check additional digit if we need to round
    if let Some(digit) = number
        .pop()
        .map(|n| n.to_digit(10).expect("Not a base 10 number"))
        && digit >= 5
    {
        let mut last_digit = number
            .pop()
            .and_then(|n| n.to_digit(10))
            .expect("Not a base 10 number");
        // Carry over at 9s
        while last_digit == 9 {
            let Some(digit) = number.pop() else {
                // If we reached the end we get 10
                number.push_str("10");
                return true;
            };
            // Stop at decimal
            if digit == '.' {
                break;
            }
            let digit = digit.to_digit(10).expect("Not a base 10 number");
            last_digit = digit;
        }
        // Round up
        let _ = write!(number, "{}", last_digit + 1);
    }
    false
}
pub fn truncate(number: &Integer, consts: &Consts) -> (String, bool) {
    let prec = consts.float_precision;
    if number == &0 {
        return (number.to_string(), false);
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
            .pow((length.clone() - consts.number_decimals_scientific - 1u8).max(Integer::ZERO))
            .to_integer()
            .unwrap());
    let mut truncated_number = truncated_number.to_string();
    if truncated_number.len() > consts.number_decimals_scientific {
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
    if length > consts.number_decimals_scientific + 1 {
        (format!("{truncated_number} × 10^{length}"), true)
    } else {
        (orig_number.to_string(), false)
    }
}
pub fn format_float(acc: &mut String, number: &Float, consts: &Consts) -> std::fmt::Result {
    // -a.b x 10^c
    // -
    // a
    // .b
    // x 10^c
    let mut number = number.clone();
    let negative = number.is_sign_negative();
    number = number.abs();
    if number == 0 {
        return acc.write_char('0');
    }
    let exponent = number
        .clone()
        .log10()
        .to_integer_round(factorion_math::rug::float::Round::Down)
        .expect("Could not round exponent")
        .0;
    if exponent > consts.number_decimals_scientific
        || exponent < -(consts.number_decimals_scientific as isize)
    {
        number = number / Float::with_val(consts.float_precision, &exponent).exp10();
    }
    let mut whole_number = number
        .to_integer_round(factorion_math::rug::float::Round::Down)
        .expect("Could not get integer part")
        .0;
    let decimal_part = number - &whole_number + 1;
    let mut decimal_part = format!("{decimal_part}");
    // Remove "1."
    decimal_part.remove(0);
    decimal_part.remove(0);
    decimal_part.truncate(consts.number_decimals_scientific + 1);
    if decimal_part.len() > consts.number_decimals_scientific {
        if round(&mut decimal_part) {
            decimal_part.clear();
            whole_number += 1;
        }
    }
    if let Some(mut digit) = decimal_part.pop() {
        while digit == '0' {
            digit = match decimal_part.pop() {
                Some(x) => x,
                None => break,
            }
        }
        decimal_part.push(digit);
    }
    if negative {
        acc.write_str("-")?;
    }
    write!(acc, "{whole_number}")?;
    if !decimal_part.is_empty() && decimal_part != "0" {
        acc.write_str(".")?;
        acc.write_str(&decimal_part)?;
    }
    if exponent > consts.number_decimals_scientific
        || exponent < -(consts.number_decimals_scientific as isize)
    {
        write!(acc, " × 10^{exponent}")?;
    }
    Ok(())
}

pub fn replace(s: &mut String, search_start: usize, from: &str, to: &str) -> usize {
    if let Some(start) = s[search_start..].find(from) {
        let start = start + search_start;
        s.replace_range(start..(start + from.len()), to);
        start
    } else {
        s.len()
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::rug::Integer;
    use std::str::FromStr;

    #[test]
    fn test_round_down() {
        let mut number = String::from("1929472373");
        round(&mut number);
        assert_eq!(number, "192947237");
    }

    #[test]
    fn test_round_up() {
        let mut number = String::from("74836748625");
        round(&mut number);
        assert_eq!(number, "7483674863");
    }

    #[test]
    fn test_round_carry() {
        let mut number = String::from("24999999995");
        round(&mut number);
        assert_eq!(number, "25");
    }

    #[test]
    fn test_factorial_level_string() {
        let en = locale::get_en();
        assert_eq!(get_factorial_level_string(1, &en.format()), "{factorial}");
        assert_eq!(
            get_factorial_level_string(2, &en.format()),
            "double-{factorial}"
        );
        assert_eq!(
            get_factorial_level_string(3, &en.format()),
            "triple-{factorial}"
        );
        assert_eq!(
            get_factorial_level_string(10, &en.format()),
            "decuple-{factorial}"
        );
        assert_eq!(
            get_factorial_level_string(45, &en.format()),
            "quinquadragintuple-{factorial}"
        );
        assert_eq!(
            get_factorial_level_string(50, &en.format()),
            "quinquagintuple-{factorial}"
        );
        assert_eq!(
            get_factorial_level_string(100, &en.format()),
            "centuple-{factorial}"
        );
        assert_eq!(
            get_factorial_level_string(521, &en.format()),
            "unviginquingentuple-{factorial}"
        );
        assert_eq!(
            get_factorial_level_string(1000, &en.format()),
            "milluple-{factorial}"
        );
        assert_eq!(
            get_factorial_level_string(4321, &en.format()),
            "unvigintricenquadrilluple-{factorial}"
        );
        assert_eq!(
            get_factorial_level_string(89342, &en.format()),
            "duoquadragintricennonilloctogintilluple-{factorial}"
        );
        assert_eq!(
            get_factorial_level_string(654321, &en.format()),
            "unvigintricenquadrillquinquagintillsescentilluple-{factorial}"
        );
        assert_eq!(
            get_factorial_level_string(1000000, &en.format()),
            "1000000-{factorial}"
        );
        let de = locale::get_de();
        assert_eq!(get_factorial_level_string(1, &de.format()), "{factorial}");
        assert_eq!(
            get_factorial_level_string(2, &de.format()),
            "doppel{factorial}"
        );
        assert_eq!(
            get_factorial_level_string(3, &de.format()),
            "trippel{factorial}"
        );
        assert_eq!(
            get_factorial_level_string(45, &de.format()),
            "quinquadragintupel{factorial}"
        );
    }

    #[test]
    fn test_write_out_number() {
        let consts = Consts::default();
        let mut acc = String::new();
        write_out_number(
            &mut acc,
            &"1234567890123456789000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000"
                .parse()
                .unwrap(),
            &consts,
        )
        .unwrap();
        assert_eq!(
            acc,
            "one tredeccentillion two hundred thirty four duodeccentillion five hundred sixty seven undeccentillion eight hundred ninety deccentillion one hundred twenty three novemcentillion four hundred fivety six octocentillion"
        );
        let mut acc = String::new();
        write_out_number(&mut acc, &"123456789".parse().unwrap(), &consts).unwrap();
        assert_eq!(
            acc,
            "one hundred twenty three million four hundred fivety six thousand seven hundred eighty nine"
        );
    }

    #[test]
    fn test_truncate() {
        let consts = Consts::default();
        assert_eq!(truncate(&Integer::from_str("0").unwrap(), &consts,).0, "0");
        assert_eq!(
            truncate(&Integer::from_str("-1").unwrap(), &consts,).0,
            "-1"
        );
        assert_eq!(
            truncate(
                &Integer::from_str(&format!("1{}", "0".repeat(300))).unwrap(),
                &consts
            )
            .0,
            "1 × 10^300"
        );
        assert_eq!(
            truncate(
                &-Integer::from_str(&format!("1{}", "0".repeat(300))).unwrap(),
                &consts
            )
            .0,
            "-1 × 10^300"
        );
        assert_eq!(
            truncate(
                &Integer::from_str(&format!("1{}", "0".repeat(2000000))).unwrap(),
                &consts
            )
            .0,
            "1 × 10^2000000"
        );
    }

    #[test]
    fn test_format_float() {
        let consts = Consts::default();
        let x = Float::with_val(consts.float_precision, 1.5);
        let mut acc = String::new();
        format_float(&mut acc, &x, &consts).unwrap();
        assert_eq!(acc, "1.5");
        let x = Float::with_val(consts.float_precision, -1.5);
        let mut acc = String::new();
        format_float(&mut acc, &x, &consts).unwrap();
        assert_eq!(acc, "-1.5");
        let x = Float::with_val(consts.float_precision, 1);
        let mut acc = String::new();
        format_float(&mut acc, &x, &consts).unwrap();
        assert_eq!(acc, "1");
        let x = Float::with_val(consts.float_precision, 1.5)
            * Float::with_val(consts.float_precision, 50000).exp10();
        let mut acc = String::new();
        format_float(&mut acc, &x, &consts).unwrap();
        assert_eq!(acc, "1.5 × 10^50000");
    }
}
