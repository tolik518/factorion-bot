//! Parses text and extracts calculations

use crate::locale::{self, NumFormat};
use crate::rug::{integer::IntegerExt64, Complete, Float, Integer};

use crate::Consts;
use crate::{
    calculation_results::Number,
    calculation_tasks::{CalculationBase, CalculationJob},
};

pub mod recommended {
    use factorion_math::rug::Integer;

    pub static INTEGER_CONSTRUCTION_LIMIT: fn() -> Integer = || 100_000_000u128.into();
}

const POI_STARTS: &[char] = &[
    NEGATION,
    '!', // PREFIX_OPS
    '.', // Decimal separators
    ESCAPE,
    '0', // Digits
    '1',
    '2',
    '3',
    '4',
    '5',
    '6',
    '7',
    '8',
    '9',
    'p', // Constants
    'e',
    't',
    'π',
    'ɸ',
    'τ',
    URI_POI,
    SPOILER_POI,
    SPOILER_HTML_POI,
    PAREN_START,
    PAREN_END,
];

const NEGATION: char = '-';
const PAREN_START: char = '(';
const PAREN_END: char = ')';
const ESCAPE: char = '\\';
const URI_START: &str = "://";
const URI_POI: char = ':';
const SPOILER_START: &str = ">!";
const SPOILER_END: &str = "!<";
const SPOILER_POI: char = '>';
const SPOILER_HTML_START: &str = "&gt;!";
const SPOILER_HTML_END: &str = "!&lt;";
const SPOILER_HTML_POI: char = '&';

const CONSTANT_STARTS: &[char] = &['p', 'e', 't', 'π', 'ɸ', 'τ'];
static E: fn(u32) -> Number = |prec| Number::Float(Float::with_val(prec, 1).exp().into());
static PHI: fn(u32) -> Number = |prec| {
    Number::Float(Float::into(
        ((1.0 + Float::with_val(prec, 5).sqrt()) as Float) / 2.0,
    ))
};
static PI: fn(u32) -> Number =
    |prec| Number::Float(Float::with_val(prec, crate::rug::float::Constant::Pi).into());
static TAU: fn(u32) -> Number = |prec| {
    Number::Float(Float::into(
        Float::with_val(prec, crate::rug::float::Constant::Pi) * 2.0,
    ))
};

const PREFIX_OPS: [char; 1] = ['!'];
#[allow(dead_code)]
const POSTFIX_OPS: [char; 2] = ['!', '?'];

const INTEGER_ONLY_OPS: [i32; 1] = [0];

pub fn parse(
    mut text: &str,
    do_termial: bool,
    consts: &Consts,
    locale: &NumFormat,
) -> Vec<CalculationJob> {
    // Parsing rules:
    // - prefix has precedence before suffix (unimplemented)
    // - anything within a spoiler should be ignored
    // - operations may be nested through parentheses
    // - operations can be negated through -
    // - parens may contain:
    //   - numbers
    //   - operations
    //   - parens
    //   - whitespace
    // - operations are:
    //   - subfactorials !n
    //   - (multi-)factorials n!+
    //   - termials n?
    // - numbers are in order:
    //   - a string of digits
    //   - a decimal separator and further digits
    //   - a base 10 exponent, which is:
    //     - an e or E followed by
    //     - optionally a + or -
    //     - a string of digits
    // - numbers need to at least have the first or second criteria

    // Parsing:
    // 1. skip to interesting
    // 2. If spoiler, skip
    // 3. If negation, save
    // 4. If paren start, push (with negation)
    // 5. If paren end, pop
    //   1. If had prefix, use base, set base
    //   2. If has postfix, use base, set base
    // 6. If prefix
    //   1. If on number, set base
    //     1. If has postfix, use base, set base
    //   2. If on paren, push (with negation and level)
    // 7. If number, parse
    //   1. If has postfix, set base
    //   2. If in parens, set as base
    // 8. If on toplevel, add base to jobs
    //
    // when setting base:
    // 1. If base is set, add previous to jobs
    // 2. override base
    let mut jobs = Vec::new();
    let mut base: Option<CalculationBase> = None;
    let mut paren_steps: Vec<(u32, Option<i32>, bool)> = Vec::new();
    let mut current_negative: u32 = 0;
    let mut last_len = usize::MAX;
    let mut had_text_before = false;
    while !text.is_empty() {
        if last_len == text.len() {
            panic!("Parser caught in a loop! Text: \"{text}\"")
        }
        last_len = text.len();

        text = text.trim_start();
        if text.len() != last_len {
            current_negative = 0;
            had_text_before = false;
        }
        // Text (1.)
        let Some(position_of_interest) = text.find(POI_STARTS) else {
            break;
        };
        if position_of_interest != 0 {
            // poison paren
            if let Some(step) = paren_steps.last_mut() {
                step.2 = true;
            }
            current_negative = 0;
            had_text_before = false;
        }
        let had_text =
            text[..position_of_interest].ends_with(char::is_alphabetic) || had_text_before;
        had_text_before = false;
        // so we can just ignore everything before
        text = &text[position_of_interest..];
        if text.starts_with(ESCAPE) {
            // Escapes
            text = &text[1..];
            let end = if text.starts_with(SPOILER_START) {
                1
            } else if text.starts_with(SPOILER_HTML_START) {
                4
            } else if text.starts_with(URI_START) {
                3
            } else {
                0
            };
            text = &text[end..];
            continue;
        } else if text.starts_with(URI_START) {
            // URI
            let end = text.find(char::is_whitespace).unwrap_or(text.len());
            text = &text[end..];
            continue;
        } else if text.starts_with(SPOILER_START) {
            // Spoiler (2.)
            let mut end = 0;
            loop {
                // look for next end tag
                if let Some(e) = text[end..].find(SPOILER_END) {
                    if e == 0 {
                        panic!("Parser loop Spoiler! Text \"{text}\"");
                    }
                    end += e;
                    // is escaped -> look further
                    if text[end.saturating_sub(1)..].starts_with(ESCAPE) {
                        end += 1;
                        continue;
                    }
                    break;
                } else {
                    // if we find none, we skip only the start (without the !)
                    end = 0;
                    break;
                }
            }
            current_negative = 0;
            text = &text[end + 1..];
            continue;
        } else if text.starts_with(SPOILER_HTML_START) {
            // Spoiler (html) (2.)
            let mut end = 0;
            loop {
                // look for next end tag
                if let Some(e) = text[end..].find(SPOILER_HTML_END) {
                    if e == 0 {
                        panic!("Parser loop Spoiler! Text \"{text}\"");
                    }
                    end += e;
                    // is escaped -> look further
                    if text[end.saturating_sub(1)..].starts_with(ESCAPE) {
                        end += 1;
                        continue;
                    }
                    break;
                } else {
                    // if we find none, we skip only the start (without the !)
                    end = 0;
                    break;
                }
            }
            current_negative = 0;
            text = &text[end + 4..];
            continue;
        } else if text.starts_with(NEGATION) {
            // Negation (3.)
            let end = text.find(|c| c != NEGATION).unwrap_or(text.len());
            current_negative = end as u32;
            text = &text[end..];
            continue;
        } else if text.starts_with(PAREN_START) {
            // Paren Start (without prefix op) (4.)
            paren_steps.push((current_negative, None, false));
            // Submit current base (we won't use it anymore)
            if let Some(CalculationBase::Calc(job)) = base.take() {
                jobs.push(*job);
            }
            current_negative = 0;
            text = &text[1..];
            continue;
        } else if text.starts_with(PAREN_END) {
            // Paren End (5.)
            text = &text[1..];
            current_negative = 0;
            // Paren mismatch?
            let Some(step) = paren_steps.pop() else {
                continue;
            };
            // poisoned paren
            if step.2 {
                if let Some(CalculationBase::Calc(job)) = base.take() {
                    jobs.push(*job);
                }
                // no number (maybe var) => poison outer paren
                if let Some(step) = paren_steps.last_mut() {
                    step.2 = true;
                }
                continue;
            }
            let mut had_op = false;
            // Prefix? (5.2.)
            if let Some(level) = step.1 {
                // base available?
                let Some(inner) = base.take() else {
                    // no number (maybe var) => poison outer paren
                    if let Some(step) = paren_steps.last_mut() {
                        step.2 = true;
                    }
                    continue;
                };
                if let (CalculationBase::Num(Number::Float(_)), true) =
                    (&inner, INTEGER_ONLY_OPS.contains(&level))
                {
                    continue;
                }
                base = Some(CalculationBase::Calc(Box::new(CalculationJob {
                    base: inner,
                    level,
                    negative: 0,
                })));
                had_op = true;
            }
            // Postfix? (5.1.)
            let Some(levels) = parse_ops(&mut text, false, do_termial) else {
                base.take();
                // no number (maybe var) => poison outer paren
                if let Some(step) = paren_steps.last_mut() {
                    step.2 = true;
                }
                continue;
            };
            if !levels.is_empty() {
                // Set as base (5.1.2.)
                for level in levels {
                    // base available?
                    let Some(inner) = base.take() else {
                        continue;
                    };
                    base = Some(CalculationBase::Calc(Box::new(CalculationJob {
                        base: inner,
                        level,
                        negative: 0,
                    })));
                    had_op = true;
                }
            }
            if !had_op {
                match &mut base {
                    Some(CalculationBase::Calc(job)) => job.negative += step.0,
                    Some(CalculationBase::Num(n)) => {
                        if step.0 % 2 != 0 {
                            n.negate();
                        }
                    }
                    None => {}
                }
            } else {
                match &mut base {
                    Some(CalculationBase::Num(n)) => {
                        if step.0 % 2 == 1 {
                            n.negate();
                        }
                    }
                    Some(CalculationBase::Calc(job)) => job.negative += step.0,
                    None => {
                        // no number (maybe var) => poison outer paren
                        if let Some(step) = paren_steps.last_mut() {
                            step.2 = true;
                        }
                    }
                }
                continue;
            };
        } else if text.starts_with(PREFIX_OPS) {
            // Prefix OP (6.)
            let Ok(level) = parse_op(&mut text, true, do_termial) else {
                // also skip number to prevent stuff like "!!!1!" getting through
                parse_num(&mut text, false, true, consts, locale);
                continue;
            };
            // On number (6.1.)
            if let Some(num) = parse_num(&mut text, false, true, consts, locale) {
                // set base (6.1.2.)
                if let Some(CalculationBase::Calc(job)) = base.take() {
                    // multiple number, likely expression => poision paren
                    if let Some(step) = paren_steps.last_mut() {
                        step.2 = true;
                    }
                    jobs.push(*job);
                }
                if let (Number::Float(_), true) = (&num, INTEGER_ONLY_OPS.contains(&level)) {
                    continue;
                }
                base = Some(CalculationBase::Calc(Box::new(CalculationJob {
                    base: CalculationBase::Num(num),
                    level,
                    negative: current_negative,
                })));
                current_negative = 0;
                let Some(levels) = parse_ops(&mut text, false, do_termial) else {
                    continue;
                };
                for level in levels {
                    // base available?
                    let Some(inner) = base.take() else {
                        continue;
                    };
                    base = Some(CalculationBase::Calc(Box::new(CalculationJob {
                        base: inner,
                        level,
                        negative: 0,
                    })));
                }
            } else {
                // on paren? (6.2.)
                if text.starts_with(PAREN_START) {
                    paren_steps.push((current_negative, Some(level), false));
                    current_negative = 0;
                    text = &text[1..];
                }
                continue;
            };
        } else {
            // Number (7.)
            if text.starts_with('.') && !text[1..].starts_with(char::is_numeric) {
                // Is a period
                text = &text[1..];
                continue;
            }
            let Some(num) = parse_num(&mut text, had_text, false, consts, locale) else {
                had_text_before = true;
                // advance one char to avoid loop
                let mut end = 1;
                while !text.is_char_boundary(end) && end < text.len() {
                    end += 1;
                }
                text = &text[end.min(text.len())..];
                continue;
            };
            // postfix? (7.1.)
            let Some(levels) = parse_ops(&mut text, false, do_termial) else {
                continue;
            };
            if !levels.is_empty() {
                let levels = levels.into_iter();
                if let Some(CalculationBase::Calc(job)) = base.take() {
                    // multiple number, likely expression => poision paren
                    if let Some(step) = paren_steps.last_mut() {
                        step.2 = true;
                    }
                    jobs.push(*job);
                }
                base = Some(CalculationBase::Num(num));
                for level in levels {
                    let previous = base.take().unwrap();
                    if let (CalculationBase::Num(Number::Float(_)), true) =
                        (&previous, INTEGER_ONLY_OPS.contains(&level))
                    {
                        continue;
                    }
                    base = Some(CalculationBase::Calc(Box::new(CalculationJob {
                        base: previous,
                        level,
                        negative: 0,
                    })))
                }
                if let Some(CalculationBase::Calc(job)) = &mut base {
                    job.negative = current_negative;
                }
            } else {
                // in parens? (7.2.)
                if !paren_steps.is_empty() {
                    let mut num = num;
                    if current_negative % 2 == 1 {
                        num.negate();
                    }

                    if base.is_none() {
                        base = Some(CalculationBase::Num(num))
                    } else {
                        // multiple number, likely expression => poision paren
                        if let Some(step) = paren_steps.last_mut() {
                            step.2 = true;
                        }
                    }
                }
            }
            current_negative = 0;
        };
        // toplevel? (8.)
        if paren_steps.is_empty()
            && let Some(CalculationBase::Calc(job)) = base.take()
        {
            jobs.push(*job);
        }
    }
    if let Some(CalculationBase::Calc(job)) = base.take() {
        jobs.push(*job);
    }
    jobs.sort();
    jobs.dedup();
    jobs
}

enum ParseOpErr {
    NonOp,
    InvalidOp,
}

fn parse_op(text: &mut &str, prefix: bool, do_termial: bool) -> Result<i32, ParseOpErr> {
    let op = text.chars().next().ok_or(ParseOpErr::NonOp)?;
    let end = text.find(|c| c != op).unwrap_or(text.len());
    let res = match op {
        '!' => {
            if prefix {
                if end != 1 {
                    Err(ParseOpErr::InvalidOp)
                } else {
                    Ok(0)
                }
            } else {
                Ok(end as i32)
            }
        }
        '?' => {
            if !do_termial {
                Err(ParseOpErr::NonOp)
            } else if prefix {
                Err(ParseOpErr::InvalidOp)
            } else {
                Ok(-(end as i32))
            }
        }
        _ => return Err(ParseOpErr::NonOp),
    };
    *text = &text[end..];
    res
}

fn parse_ops(text: &mut &str, prefix: bool, do_termial: bool) -> Option<Vec<i32>> {
    let mut res = Vec::new();
    loop {
        match parse_op(text, prefix, do_termial) {
            Ok(op) => res.push(op),
            Err(ParseOpErr::NonOp) => break,
            Err(ParseOpErr::InvalidOp) => return None,
        }
    }
    Some(res)
}

fn parse_num(
    text: &mut &str,
    had_text: bool,
    had_op: bool,
    consts: &Consts,
    locale: &NumFormat,
) -> Option<Number> {
    let prec = consts.float_precision;
    if text.starts_with(CONSTANT_STARTS) {
        let (n, x) = if text.starts_with("pi") {
            ("pi".len(), PI(prec))
        } else if text.starts_with("π") {
            ("π".len(), PI(prec))
        } else if text.starts_with("phi") {
            ("phi".len(), PHI(prec))
        } else if text.starts_with("ɸ") {
            ("ɸ".len(), PHI(prec))
        } else if text.starts_with("tau") {
            ("tau".len(), TAU(prec))
        } else if text.starts_with("τ") {
            ("τ".len(), TAU(prec))
        } else if text.starts_with("e") {
            ("e".len(), E(prec))
        } else {
            return None;
        };
        if had_text || text[n..].starts_with(char::is_alphabetic) {
            return None;
        }
        *text = &text[n..];
        return Some(x);
    }

    let integer_part = {
        let end = text.find(|c: char| !c.is_numeric()).unwrap_or(text.len());
        let part = &text[..end];
        *text = &text[end..];
        part
    };
    let decimal_part = if text.starts_with(*locale.decimal()) {
        *text = &text[1..];
        let end = text.find(|c: char| !c.is_numeric()).unwrap_or(text.len());
        let part = &text[..end];
        *text = &text[end..];
        part
    } else {
        &text[..0]
    };
    let exponent_part = if text.starts_with(['e', 'E']) {
        *text = &text[1..];
        let negative = if text.starts_with('+') {
            *text = &text[1..];
            false
        } else if text.starts_with('-') {
            *text = &text[1..];
            true
        } else {
            false
        };
        let end = text.find(|c: char| !c.is_numeric()).unwrap_or(text.len());
        let part = &text[..end];
        *text = &text[end..];
        (part, negative)
    } else {
        (&text[..0], false)
    };
    let fraction_part = if !had_op && text.starts_with(['/']) {
        *text = &text[1..];
        let end = text.find(|c: char| !c.is_numeric()).unwrap_or(text.len());
        let part = &text[..end];
        *text = &text[end..];
        part
    } else {
        &text[..0]
    };
    if text.starts_with(POSTFIX_OPS) && !fraction_part.is_empty() {
        let n = fraction_part.parse::<Integer>().ok()?;
        return Some(Number::Exact(n));
    }
    if integer_part.is_empty() && decimal_part.is_empty() {
        return None;
    }
    let exponent = if !exponent_part.0.is_empty() {
        let mut e = exponent_part.0.parse::<Integer>().ok()?;
        if exponent_part.1 {
            e *= -1;
        }
        e
    } else {
        0.into()
    };
    let divisor = if !fraction_part.is_empty() {
        fraction_part.parse::<Integer>().ok()?
    } else {
        Integer::ONE.clone()
    };
    if exponent >= decimal_part.len() as i64
        && exponent <= consts.integer_construction_limit.clone() - integer_part.len() as i64
        && (divisor == 1 || exponent >= consts.integer_construction_limit.clone() / 10)
    {
        let exponent = exponent - decimal_part.len();
        let n = format!("{integer_part}{decimal_part}")
            .parse::<Integer>()
            .ok()?;
        let num = (n * Integer::u64_pow_u64(10, exponent.to_u64().unwrap()).complete()) / divisor;
        Some(Number::Exact(num))
    } else if exponent <= consts.integer_construction_limit.clone() - integer_part.len() as i64 {
        let x = Float::parse(format!(
            "{integer_part}.{decimal_part}{}{}{}",
            if !exponent_part.0.is_empty() { "e" } else { "" },
            if exponent_part.1 { "-" } else { "" },
            exponent_part.0
        ))
        .ok()?;
        let x = Float::with_val(prec, x) / divisor;
        if x.is_integer() {
            let n = x.to_integer().unwrap();
            Some(Number::Exact(n))
        } else if x.is_finite() {
            Some(Number::Float(x.into()))
        } else {
            None
        }
    } else {
        let x = Float::parse(format!("{integer_part}.{decimal_part}")).ok()?;
        let x = Float::with_val(prec, x) / divisor;
        if x.is_finite() {
            let (b, e) = crate::math::adjust_approximate((x, exponent));
            Some(Number::Approximate(b.into(), e))
        } else {
            None
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::calculation_tasks::CalculationBase::Num;
    use arbtest::arbtest;

    use crate::recommended::FLOAT_PRECISION;

    #[test]
    fn test_text_only() {
        let consts = Consts::default();
        let jobs = parse(
            "just some words of encouragement!",
            true,
            &consts,
            &NumFormat::V1(&locale::v1::NumFormat { decimal: '.' }),
        );
        assert_eq!(jobs, []);
    }
    #[test]
    fn test_factorial() {
        let consts = Consts::default();
        let jobs = parse(
            "a factorial 15!",
            true,
            &consts,
            &NumFormat::V1(&locale::v1::NumFormat { decimal: '.' }),
        );
        assert_eq!(
            jobs,
            [CalculationJob {
                base: CalculationBase::Num(15.into()),
                level: 1,
                negative: 0
            }]
        );
    }
    #[test]
    fn test_multifactorial() {
        let consts = Consts::default();
        let jobs = parse(
            "a factorial 15!!! actually a multi",
            true,
            &consts,
            &NumFormat::V1(&locale::v1::NumFormat { decimal: '.' }),
        );
        assert_eq!(
            jobs,
            [CalculationJob {
                base: CalculationBase::Num(15.into()),
                level: 3,
                negative: 0
            }]
        );
    }
    #[test]
    fn test_subfactorial() {
        let consts = Consts::default();
        let jobs = parse(
            "a factorial !15 actually a sub",
            true,
            &consts,
            &NumFormat::V1(&locale::v1::NumFormat { decimal: '.' }),
        );
        assert_eq!(
            jobs,
            [CalculationJob {
                base: CalculationBase::Num(15.into()),
                level: 0,
                negative: 0
            }]
        );
    }
    #[test]
    fn test_submultifactorial() {
        let consts = Consts::default();
        let jobs = parse(
            "not well defined !!!15",
            true,
            &consts,
            &NumFormat::V1(&locale::v1::NumFormat { decimal: '.' }),
        );
        assert_eq!(jobs, []);
    }
    #[test]
    fn test_termial() {
        let consts = Consts::default();
        let jobs = parse(
            "a termial 15?",
            true,
            &consts,
            &NumFormat::V1(&locale::v1::NumFormat { decimal: '.' }),
        );
        assert_eq!(
            jobs,
            [CalculationJob {
                base: CalculationBase::Num(15.into()),
                level: -1,
                negative: 0
            }]
        );
    }
    #[test]
    fn test_no_termial() {
        let consts = Consts::default();
        let jobs = parse(
            "not enabled 15?",
            false,
            &consts,
            &NumFormat::V1(&locale::v1::NumFormat { decimal: '.' }),
        );
        assert_eq!(jobs, []);
    }
    #[test]
    fn test_multitermial() {
        let consts = Consts::default();
        let jobs = parse(
            "a termial 15??? actually a multi",
            true,
            &consts,
            &NumFormat::V1(&locale::v1::NumFormat { decimal: '.' }),
        );
        assert_eq!(
            jobs,
            [CalculationJob {
                base: CalculationBase::Num(15.into()),
                level: -3,
                negative: 0
            }]
        );
    }
    #[test]
    fn test_subtermial() {
        let consts = Consts::default();
        let jobs = parse(
            "a termial ?15 actually a sub",
            true,
            &consts,
            &NumFormat::V1(&locale::v1::NumFormat { decimal: '.' }),
        );
        assert_eq!(jobs, []);
    }
    #[test]
    fn test_chain() {
        let consts = Consts::default();
        let jobs = parse(
            "a factorialchain (15!)!",
            true,
            &consts,
            &NumFormat::V1(&locale::v1::NumFormat { decimal: '.' }),
        );
        assert_eq!(
            jobs,
            [CalculationJob {
                base: CalculationBase::Calc(Box::new(CalculationJob {
                    base: CalculationBase::Num(15.into()),
                    level: 1,
                    negative: 0
                })),
                level: 1,
                negative: 0
            }]
        );
    }
    #[test]
    fn test_mixed_chain() {
        let consts = Consts::default();
        let jobs = parse(
            "a factorialchain !(15!)",
            true,
            &consts,
            &NumFormat::V1(&locale::v1::NumFormat { decimal: '.' }),
        );
        assert_eq!(
            jobs,
            [CalculationJob {
                base: CalculationBase::Calc(Box::new(CalculationJob {
                    base: CalculationBase::Num(15.into()),
                    level: 1,
                    negative: 0
                })),
                level: 0,
                negative: 0
            }]
        );
    }
    #[test]
    fn test_postfix_chain() {
        let consts = Consts::default();
        let jobs = parse(
            "a factorialchain -15!?",
            true,
            &consts,
            &NumFormat::V1(&locale::v1::NumFormat { decimal: '.' }),
        );
        assert_eq!(
            jobs,
            [CalculationJob {
                base: CalculationBase::Calc(Box::new(CalculationJob {
                    base: CalculationBase::Num(15.into()),
                    level: 1,
                    negative: 0
                })),
                level: -1,
                negative: 1
            }]
        );
    }
    #[test]
    fn test_negative() {
        let consts = Consts::default();
        let jobs = parse(
            "a factorial ---15!",
            true,
            &consts,
            &NumFormat::V1(&locale::v1::NumFormat { decimal: '.' }),
        );
        assert_eq!(
            jobs,
            [CalculationJob {
                base: CalculationBase::Num(15.into()),
                level: 1,
                negative: 3
            }]
        );
    }
    #[test]
    fn test_negative_gap() {
        let consts = Consts::default();
        let jobs = parse(
            "a factorial --- 15!",
            true,
            &consts,
            &NumFormat::V1(&locale::v1::NumFormat { decimal: '.' }),
        );
        assert_eq!(
            jobs,
            [CalculationJob {
                base: CalculationBase::Num(15.into()),
                level: 1,
                negative: 0
            }]
        );
    }
    #[test]
    fn test_paren() {
        let consts = Consts::default();
        let jobs = parse(
            "a factorial (15)!",
            true,
            &consts,
            &NumFormat::V1(&locale::v1::NumFormat { decimal: '.' }),
        );
        assert_eq!(
            jobs,
            [CalculationJob {
                base: CalculationBase::Num(15.into()),
                level: 1,
                negative: 0
            }]
        );
    }
    #[test]
    fn test_in_paren() {
        let consts = Consts::default();
        let jobs = parse(
            "a factorial (15!)",
            true,
            &consts,
            &NumFormat::V1(&locale::v1::NumFormat { decimal: '.' }),
        );
        assert_eq!(
            jobs,
            [CalculationJob {
                base: CalculationBase::Num(15.into()),
                level: 1,
                negative: 0
            }]
        );
    }
    #[test]
    fn test_decimal() {
        let consts = Consts::default();
        let jobs = parse(
            "a factorial 1.5!",
            true,
            &consts,
            &NumFormat::V1(&locale::v1::NumFormat { decimal: '.' }),
        );
        assert_eq!(
            jobs,
            [CalculationJob {
                base: CalculationBase::Num(Float::with_val(FLOAT_PRECISION, 1.5).into()),
                level: 1,
                negative: 0
            }]
        );
    }
    #[test]
    fn test_paren_negation() {
        let consts = Consts::default();
        let jobs = parse(
            "a factorial -(--(-(-(-3))!))!",
            true,
            &consts,
            &NumFormat::V1(&locale::v1::NumFormat { decimal: '.' }),
        );
        assert_eq!(
            jobs,
            [CalculationJob {
                base: CalculationBase::Calc(Box::new(CalculationJob {
                    base: CalculationBase::Num(3.into()),
                    level: 1,
                    negative: 3
                })),
                level: 1,
                negative: 1
            }]
        );
    }
    #[test]
    fn test_tag() {
        let consts = Consts::default();
        let jobs = parse(
            ">!5 a factorial 15! !<",
            true,
            &consts,
            &NumFormat::V1(&locale::v1::NumFormat { decimal: '.' }),
        );
        assert_eq!(jobs, []);
    }
    #[test]
    fn test_incomplete_tag() {
        let consts = Consts::default();
        let jobs = parse(
            ">!5 a factorial 15!",
            true,
            &consts,
            &NumFormat::V1(&locale::v1::NumFormat { decimal: '.' }),
        );
        assert_eq!(
            jobs,
            [
                CalculationJob {
                    base: CalculationBase::Num(5.into()),
                    level: 0,
                    negative: 0
                },
                CalculationJob {
                    base: CalculationBase::Num(15.into()),
                    level: 1,
                    negative: 0
                }
            ]
        );
    }
    #[test]
    fn test_escaped_tag() {
        let consts = Consts::default();
        let jobs = parse(
            "\\>!5 a factorial 15! !<",
            true,
            &consts,
            &NumFormat::V1(&locale::v1::NumFormat { decimal: '.' }),
        );
        assert_eq!(
            jobs,
            [
                CalculationJob {
                    base: CalculationBase::Num(5.into()),
                    level: 0,
                    negative: 0
                },
                CalculationJob {
                    base: CalculationBase::Num(15.into()),
                    level: 1,
                    negative: 0
                }
            ]
        );
    }
    #[test]
    fn test_escaped_tag2() {
        let consts = Consts::default();
        let jobs = parse(
            ">!5 a factorial 15! \\!<",
            true,
            &consts,
            &NumFormat::V1(&locale::v1::NumFormat { decimal: '.' }),
        );
        assert_eq!(
            jobs,
            [
                CalculationJob {
                    base: CalculationBase::Num(5.into()),
                    level: 0,
                    negative: 0
                },
                CalculationJob {
                    base: CalculationBase::Num(15.into()),
                    level: 1,
                    negative: 0
                }
            ]
        );
    }

    #[test]
    fn test_url() {
        let consts = Consts::default();
        let jobs = parse(
            "https://something.somewhere/with/path/and?tag=siufgiufgia3873844hi8743!hfsf",
            true,
            &consts,
            &NumFormat::V1(&locale::v1::NumFormat { decimal: '.' }),
        );
        assert_eq!(jobs, []);
    }

    #[test]
    fn test_uri_poi_doesnt_cause_infinite_loop() {
        let consts = Consts::default();
        let jobs = parse(
            "84!:",
            true,
            &consts,
            &NumFormat::V1(&locale::v1::NumFormat { decimal: '.' }),
        );
        assert_eq!(
            jobs,
            [CalculationJob {
                base: Num(84.into()),
                level: 1,
                negative: 0
            }]
        );
    }
    #[test]
    fn test_escaped_url() {
        let consts = Consts::default();
        let jobs = parse(
            "\\://something.somewhere/with/path/and?tag=siufgiufgia3873844hi8743!hfsf",
            true,
            &consts,
            &NumFormat::V1(&locale::v1::NumFormat { decimal: '.' }),
        );
        assert_eq!(
            jobs,
            [CalculationJob {
                base: CalculationBase::Num(8743.into()),
                level: 1,
                negative: 0
            }]
        );
    }

    #[test]
    fn test_word_in_paren() {
        let consts = Consts::default();
        let jobs = parse(
            "(x-2)! (2 word)! ((x/k)-3)! (,x-4)!",
            true,
            &consts,
            &NumFormat::V1(&locale::v1::NumFormat { decimal: '.' }),
        );
        assert_eq!(jobs, []);
    }

    #[test]
    fn test_multi_number_paren() {
        let consts = Consts::default();
        let jobs = parse(
            "(5-2)!",
            true,
            &consts,
            &NumFormat::V1(&locale::v1::NumFormat { decimal: '.' }),
        );
        assert_eq!(jobs, []);
    }
    #[test]
    fn test_arbitrary_input() {
        let consts = Consts::default();
        arbtest(|u| {
            let text: &str = u.arbitrary()?;
            let _ = parse(
                text,
                u.arbitrary()?,
                &consts,
                &NumFormat::V1(&locale::v1::NumFormat { decimal: '.' }),
            );
            Ok(())
        });
    }

    #[test]
    fn test_constant() {
        let consts = Consts::default();
        let jobs = parse(
            "!espi!",
            true,
            &consts,
            &NumFormat::V1(&locale::v1::NumFormat { decimal: '.' }),
        );
        assert_eq!(jobs, []);
        let jobs = parse(
            "some. pi!",
            true,
            &consts,
            &consts.locales.get("en").unwrap().format().number_format(),
        );
        assert_eq!(
            jobs,
            [CalculationJob {
                base: CalculationBase::Num(Number::Float(
                    Float::with_val(FLOAT_PRECISION, factorion_math::rug::float::Constant::Pi)
                        .into()
                )),
                level: 1,
                negative: 0
            }]
        );
    }

    #[test]
    fn test_fraction() {
        let consts = Consts::default();
        let jobs = parse(
            "!5/6!",
            true,
            &consts,
            &NumFormat::V1(&locale::v1::NumFormat { decimal: '.' }),
        );
        assert_eq!(
            jobs,
            [
                CalculationJob {
                    base: CalculationBase::Num(Number::Exact(5.into())),
                    level: 0,
                    negative: 0
                },
                CalculationJob {
                    base: CalculationBase::Num(Number::Exact(6.into())),
                    level: 1,
                    negative: 0
                }
            ]
        );
        let jobs = parse(
            "5/6!",
            true,
            &consts,
            &NumFormat::V1(&locale::v1::NumFormat { decimal: '.' }),
        );
        assert_eq!(
            jobs,
            [CalculationJob {
                base: CalculationBase::Num(Number::Exact(6.into())),
                level: 1,
                negative: 0
            }]
        );
        let jobs = parse(
            "(10/2)!",
            true,
            &consts,
            &NumFormat::V1(&locale::v1::NumFormat { decimal: '.' }),
        );
        assert_eq!(
            jobs,
            [CalculationJob {
                base: CalculationBase::Num(Number::Exact(5.into())),
                level: 1,
                negative: 0
            },]
        );
    }

    #[test]
    fn test_parse_num() {
        let consts = Consts::default();
        let num = parse_num(
            &mut "1.5more !",
            false,
            false,
            &consts,
            &NumFormat::V1(&locale::v1::NumFormat { decimal: '.' }),
        );
        assert_eq!(
            num,
            Some(Number::Float(Float::with_val(FLOAT_PRECISION, 1.5).into()))
        );
        let num = parse_num(
            &mut "1,5more !",
            false,
            false,
            &consts,
            &NumFormat::V1(&locale::v1::NumFormat { decimal: ',' }),
        );
        assert_eq!(
            num,
            Some(Number::Float(Float::with_val(FLOAT_PRECISION, 1.5).into()))
        );
        let num = parse_num(
            &mut ".5more !",
            false,
            false,
            &consts,
            &NumFormat::V1(&locale::v1::NumFormat { decimal: '.' }),
        );
        assert_eq!(
            num,
            Some(Number::Float(Float::with_val(FLOAT_PRECISION, 0.5).into()))
        );
        let num = parse_num(
            &mut "1more !",
            false,
            true,
            &consts,
            &NumFormat::V1(&locale::v1::NumFormat { decimal: '.' }),
        );
        assert_eq!(num, Some(1.into()));
        let num = parse_num(
            &mut "1.0more !",
            true,
            false,
            &consts,
            &NumFormat::V1(&locale::v1::NumFormat { decimal: '.' }),
        );
        assert_eq!(num, Some(1.into()));
        let num = parse_num(
            &mut "1.5e2more !",
            false,
            false,
            &consts,
            &NumFormat::V1(&locale::v1::NumFormat { decimal: '.' }),
        );
        assert_eq!(num, Some(150.into()));
        let num = parse_num(
            &mut "1e2more !",
            false,
            false,
            &consts,
            &NumFormat::V1(&locale::v1::NumFormat { decimal: '.' }),
        );
        assert_eq!(num, Some(100.into()));
        let num = parse_num(
            &mut "1.531e2more !",
            false,
            false,
            &consts,
            &NumFormat::V1(&locale::v1::NumFormat { decimal: '.' }),
        );
        let Some(Number::Float(f)) = num else {
            panic!("Not a float")
        };
        assert!(Float::abs(f.as_float().clone() - 153.1) < 0.0000001);
        let num = parse_num(
            &mut "5e-1more !",
            false,
            false,
            &consts,
            &NumFormat::V1(&locale::v1::NumFormat { decimal: '.' }),
        );
        assert_eq!(
            num,
            Some(Number::Float(Float::with_val(FLOAT_PRECISION, 0.5).into()))
        );
        let num = parse_num(
            &mut "e2more !",
            true,
            false,
            &consts,
            &NumFormat::V1(&locale::v1::NumFormat { decimal: '.' }),
        );
        assert_eq!(num, None);
        let num = parse_num(
            &mut "es !",
            false,
            false,
            &consts,
            &NumFormat::V1(&locale::v1::NumFormat { decimal: '.' }),
        );
        assert_eq!(num, None);
        let num = parse_num(
            &mut "e !",
            false,
            false,
            &consts,
            &NumFormat::V1(&locale::v1::NumFormat { decimal: '.' }),
        );
        assert_eq!(num, Some(E(FLOAT_PRECISION)));
        let num = parse_num(
            &mut "pi !",
            false,
            false,
            &consts,
            &NumFormat::V1(&locale::v1::NumFormat { decimal: '.' }),
        );
        assert_eq!(num, Some(PI(FLOAT_PRECISION)));
        let num = parse_num(
            &mut "π !",
            false,
            false,
            &consts,
            &NumFormat::V1(&locale::v1::NumFormat { decimal: '.' }),
        );
        assert_eq!(num, Some(PI(FLOAT_PRECISION)));
        let num = parse_num(
            &mut "phi !",
            false,
            false,
            &consts,
            &NumFormat::V1(&locale::v1::NumFormat { decimal: '.' }),
        );
        assert_eq!(num, Some(PHI(FLOAT_PRECISION)));
        let num = parse_num(
            &mut "ɸ !",
            false,
            false,
            &consts,
            &NumFormat::V1(&locale::v1::NumFormat { decimal: '.' }),
        );
        assert_eq!(num, Some(PHI(FLOAT_PRECISION)));
        let num = parse_num(
            &mut "tau !",
            false,
            false,
            &consts,
            &NumFormat::V1(&locale::v1::NumFormat { decimal: '.' }),
        );
        assert_eq!(num, Some(TAU(FLOAT_PRECISION)));
        let num = parse_num(
            &mut "τ !",
            false,
            false,
            &consts,
            &NumFormat::V1(&locale::v1::NumFormat { decimal: '.' }),
        );
        assert_eq!(num, Some(TAU(FLOAT_PRECISION)));
        let num = parse_num(
            &mut "1/2 !",
            false,
            false,
            &consts,
            &NumFormat::V1(&locale::v1::NumFormat { decimal: '.' }),
        );
        assert_eq!(
            num,
            Some(Number::Float(Float::with_val(FLOAT_PRECISION, 0.5).into()))
        );
        let num = parse_num(
            &mut "10/2 !",
            false,
            false,
            &consts,
            &NumFormat::V1(&locale::v1::NumFormat { decimal: '.' }),
        );
        assert_eq!(num, Some(Number::Exact(5.into())));
        let num = parse_num(
            &mut "1.5/2 !",
            false,
            false,
            &consts,
            &NumFormat::V1(&locale::v1::NumFormat { decimal: '.' }),
        );
        assert_eq!(
            num,
            Some(Number::Float(Float::with_val(FLOAT_PRECISION, 0.75).into()))
        );
        let num = parse_num(
            &mut "10e10000000000/2 !",
            false,
            false,
            &consts,
            &NumFormat::V1(&locale::v1::NumFormat { decimal: '.' }),
        );
        assert_eq!(
            num,
            Some(Number::Approximate(
                Float::with_val(FLOAT_PRECISION, 5).into(),
                10000000000u64.into()
            ))
        );
        let num = parse_num(
            &mut "10/2 !",
            false,
            true,
            &consts,
            &NumFormat::V1(&locale::v1::NumFormat { decimal: '.' }),
        );
        assert_eq!(num, Some(Number::Exact(10.into())));
        let num = parse_num(
            &mut "10/2!",
            false,
            false,
            &consts,
            &NumFormat::V1(&locale::v1::NumFormat { decimal: '.' }),
        );
        assert_eq!(num, Some(Number::Exact(2.into())));
    }
    #[allow(clippy::uninlined_format_args)]
    #[test]
    fn test_biggest_num() {
        let consts = Consts::default();
        let num = parse_num(
            &mut format!("9e{}", recommended::INTEGER_CONSTRUCTION_LIMIT()).as_str(),
            true,
            false,
            &consts,
            &NumFormat::V1(&locale::v1::NumFormat { decimal: '.' }),
        );
        assert!(matches!(num, Some(Number::Approximate(_, _))));
        let num = parse_num(
            &mut format!("9e{}", recommended::INTEGER_CONSTRUCTION_LIMIT() - 1).as_str(),
            false,
            false,
            &consts,
            &NumFormat::V1(&locale::v1::NumFormat { decimal: '.' }),
        );
        assert!(num.is_some());
    }
}
