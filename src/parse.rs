use rug::{Complete, Float, Integer, integer::IntegerExt64};

use crate::{
    calculation_results::Number,
    calculation_tasks::{CalculationBase, CalculationJob, INTEGER_CONSTRUCTION_LIMIT},
    math::{self, FLOAT_PRECISION},
};

const POI_STARTS: [char; 19] = [
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

const PREFIX_OPS: [char; 1] = ['!'];
#[allow(dead_code)]
const POSTFIX_OPS: [char; 2] = ['!', '?'];

const INTEGER_ONLY_OPS: [i32; 1] = [0];

pub fn parse(mut text: &str, do_termial: bool) -> Vec<CalculationJob> {
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
    while !text.is_empty() {
        if last_len == text.len() {
            panic!("Parser caught in a loop! Text: \"{text}\"")
        }
        last_len = text.len();

        text = text.trim_start();
        if text.len() != last_len {
            current_negative = 0;
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
        }
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
                // Prefix? (5.1.1.)
                if step.1.is_some() {
                    continue;
                }
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
                parse_num(&mut text);
                continue;
            };
            // On number (6.1.)
            if let Some(num) = parse_num(&mut text) {
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
            let Some(num) = parse_num(&mut text) else {
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
        if paren_steps.is_empty() {
            if let Some(CalculationBase::Calc(job)) = base.take() {
                jobs.push(*job);
            }
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

fn parse_num(text: &mut &str) -> Option<Number> {
    let integer_part = {
        let end = text.find(|c: char| !c.is_numeric()).unwrap_or(text.len());
        let part = &text[..end];
        *text = &text[end..];
        part
    };
    let decimal_part = if text.starts_with(['.', ',']) {
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
    if exponent >= decimal_part.len() as i64
        && exponent <= INTEGER_CONSTRUCTION_LIMIT - integer_part.len() as i64
    {
        let exponent = exponent - decimal_part.len();
        let n = format!("{integer_part}{decimal_part}")
            .parse::<Integer>()
            .ok()?;
        let num = n * Integer::u64_pow_u64(10, exponent.to_u64().unwrap()).complete();
        Some(Number::Exact(num))
    } else if exponent <= INTEGER_CONSTRUCTION_LIMIT - integer_part.len() as i64 {
        let x = Float::parse(format!(
            "{integer_part}.{decimal_part}{}{}{}",
            if !exponent_part.0.is_empty() { "e" } else { "" },
            if exponent_part.1 { "-" } else { "" },
            exponent_part.0
        ))
        .ok()?;
        let x = Float::with_val(FLOAT_PRECISION, x);
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
        let x = Float::with_val(FLOAT_PRECISION, x);
        if x.is_finite() {
            let (b, e) = math::adjust_approximate((x, exponent));
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
    #[test]
    fn test_text_only() {
        let jobs = parse("just some words of encouragement!", true);
        assert_eq!(jobs, []);
    }
    #[test]
    fn test_factorial() {
        let jobs = parse("a factorial 15!", true);
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
        let jobs = parse("a factorial 15!!! actually a multi", true);
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
        let jobs = parse("a factorial !15 actually a sub", true);
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
        let jobs = parse("not well defined !!!15", true);
        assert_eq!(jobs, []);
    }
    #[test]
    fn test_termial() {
        let jobs = parse("a termial 15?", true);
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
        let jobs = parse("not enabled 15?", false);
        assert_eq!(jobs, []);
    }
    #[test]
    fn test_multitermial() {
        let jobs = parse("a termial 15??? actually a multi", true);
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
        let jobs = parse("a termial ?15 actually a sub", true);
        assert_eq!(jobs, []);
    }
    #[test]
    fn test_chain() {
        let jobs = parse("a factorialchain (15!)!", true);
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
        let jobs = parse("a factorialchain !(15!)", true);
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
        let jobs = parse("a factorialchain -15!?", true);
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
        let jobs = parse("a factorial ---15!", true);
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
        let jobs = parse("a factorial --- 15!", true);
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
        let jobs = parse("a factorial (15)!", true);
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
        let jobs = parse("a factorial (15!)", true);
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
        let jobs = parse("a factorial 1.5!", true);
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
        let jobs = parse("a factorial -(--(-(-(-3))!))!", true);
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
        let jobs = parse(">!5 a factorial 15! !<", true);
        assert_eq!(jobs, []);
    }
    #[test]
    fn test_incomplete_tag() {
        let jobs = parse(">!5 a factorial 15!", true);
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
        let jobs = parse("\\>!5 a factorial 15! !<", true);
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
        let jobs = parse(">!5 a factorial 15! \\!<", true);
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
        let jobs = parse(
            "https://something.somewhere/with/path/and?tag=siufgiufgia3873844hi8743!hfsf",
            true,
        );
        assert_eq!(jobs, []);
    }

    #[test]
    fn test_uri_poi_doesnt_cause_infinite_loop() {
        let jobs = parse("84!:", true);
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
        let jobs = parse(
            "\\://something.somewhere/with/path/and?tag=siufgiufgia3873844hi8743!hfsf",
            true,
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
        let jobs = parse("(x-2)! (2 word)! ((x/k)-3)! (,x-4)!", true);
        assert_eq!(jobs, []);
    }

    #[test]
    fn test_multi_number_paren() {
        let jobs = parse("(5-2)!", true);
        assert_eq!(jobs, []);
    }
    #[test]
    fn test_arbitrary_input() {
        arbtest(|u| {
            let text: &str = u.arbitrary()?;
            let _ = parse(text, u.arbitrary()?);
            Ok(())
        });
    }

    #[test]
    fn test_parse_num() {
        let num = parse_num(&mut "1.5more !");
        assert_eq!(
            num,
            Some(Number::Float(Float::with_val(FLOAT_PRECISION, 1.5).into()))
        );
        let num = parse_num(&mut "1,5more !");
        assert_eq!(
            num,
            Some(Number::Float(Float::with_val(FLOAT_PRECISION, 1.5).into()))
        );
        let num = parse_num(&mut ".5more !");
        assert_eq!(
            num,
            Some(Number::Float(Float::with_val(FLOAT_PRECISION, 0.5).into()))
        );
        let num = parse_num(&mut "1more !");
        assert_eq!(num, Some(1.into()));
        let num = parse_num(&mut "1.0more !");
        assert_eq!(num, Some(1.into()));
        let num = parse_num(&mut "1.5e2more !");
        assert_eq!(num, Some(150.into()));
        let num = parse_num(&mut "1e2more !");
        assert_eq!(num, Some(100.into()));
        let num = parse_num(&mut "1.531e2more !");
        let Some(Number::Float(f)) = num else {
            panic!("Not a float")
        };
        assert!(Float::abs(f.as_float().clone() - 153.1) < 0.0000001);
        let num = parse_num(&mut "5e-1more !");
        assert_eq!(
            num,
            Some(Number::Float(Float::with_val(FLOAT_PRECISION, 0.5).into()))
        );
        let num = parse_num(&mut "e2more !");
        assert_eq!(num, None);
    }
    #[allow(clippy::uninlined_format_args)]
    #[test]
    fn test_biggest_num() {
        let num = parse_num(&mut format!("9e{}", INTEGER_CONSTRUCTION_LIMIT).as_str());
        assert!(matches!(num, Some(Number::Approximate(_, _))));
        let num = parse_num(&mut format!("9e{}", INTEGER_CONSTRUCTION_LIMIT - 1).as_str());
        assert!(num.is_some());
    }
}
