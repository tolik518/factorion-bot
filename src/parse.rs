use rug::{integer::IntegerExt64, Complete, Float, Integer};

use crate::{
    calculation_results::Number,
    calculation_tasks::{CalculationBase, CalculationJob, INTEGER_CONSTRUCTION_LIMIT},
    math::FLOAT_PRECISION,
};

const POI_STARTS: [char; 17] = [
    '-', '!', '.', '0', '1', '2', '3', '4', '5', '6', '7', '8', '9', '>', '&', '(', ')',
];

const PREFIX_OPS: [char; 1] = ['!'];
const POSTFIX_OPS: [char; 2] = ['!', '?'];

const INTEGER_ONLY_OPS: [i32; 1] = [-1];

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
    let mut paren_steps: Vec<(u32, Option<i32>)> = Vec::new();
    let mut current_negative: u32 = 0;
    while !text.is_empty() {
        // Text (1.)
        let Some(position_of_interest) = text.find(POI_STARTS) else {
            break;
        };
        if position_of_interest != 0 {
            current_negative = 0;
        }
        // so we can just ignore everything before
        text = &text[position_of_interest..];
        if text.starts_with(">!") {
            // Spoiler (2.)
            let end = text.find("!<").unwrap_or(1);
            current_negative = 0;
            text = &text[end + 1..];
            continue;
        } else if text.starts_with(">") {
            current_negative = 0;
            text = &text[1..]
        } else if text.starts_with("&gt;!") {
            // Spoiler (html) (2.)
            let end = text.find("!&lt;").unwrap_or(1);
            current_negative = 0;
            text = &text[end + 4..];
            continue;
        } else if text.starts_with("&") {
            current_negative = 0;
            text = &text[1..]
        } else if text.starts_with("-") {
            // Negation (3.)
            let end = text.find(|c| c != '-').unwrap_or(text.len());
            current_negative = end as u32;
            text = &text[end..];
            continue;
        } else if text.starts_with("(") {
            // Paren Start (without prefix op) (4.)
            paren_steps.push((current_negative, None));
            // Submit current base (we won't use it anymore)
            if let Some(CalculationBase::Calc(job)) = base.take() {
                jobs.push(*job);
            }
            current_negative = 0;
            text = &text[1..];
            continue;
        } else if text.starts_with(")") {
            // Paren End (5.)
            text = &text[1..];
            current_negative = 0;
            // Paren mismatch?
            let Some(step) = paren_steps.pop() else {
                continue;
            };
            let mut had_op = false;
            // Prefix? (5.2.)
            if let Some(level) = step.1 {
                // base available?
                let Some(inner) = base.take() else {
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
                if let Some(CalculationBase::Calc(job)) = &mut base {
                    job.negative = step.0
                }
            } else {
                match &mut base {
                    Some(CalculationBase::Num(n)) => {
                        if step.0 % 2 == 1 {
                            n.negate();
                        }
                    }
                    Some(CalculationBase::Calc(job)) => job.negative += step.0,
                    None => {}
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
                if let Some(previous) = base.take() {
                    if let CalculationBase::Calc(job) = previous {
                        jobs.push(*job);
                    }
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
                if text.starts_with("(") {
                    paren_steps.push((current_negative, Some(level)));
                    current_negative = 0;
                    text = &text[1..];
                }
                continue;
            };
        } else {
            // Number (7.)
            let Some(num) = parse_num(&mut text) else {
                continue;
            };
            // postfix? (7.1.)
            let Some(levels) = parse_ops(&mut text, false, do_termial) else {
                continue;
            };
            if !levels.is_empty() {
                let levels = levels.into_iter();
                if let Some(previous) = base.take() {
                    if let CalculationBase::Calc(job) = previous {
                        jobs.push(*job);
                    }
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

                    if let None = &base {
                        base = Some(CalculationBase::Num(num))
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
                    dbg!(*text);
                    Err(ParseOpErr::InvalidOp)
                } else {
                    Ok(-1)
                }
            } else {
                Ok(end as i32)
            }
        }
        '?' => {
            if !do_termial {
                Err(ParseOpErr::NonOp)
            } else if prefix || end != 1 {
                dbg!(*text);
                Err(ParseOpErr::InvalidOp)
            } else {
                Ok(0)
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
        let mut e = exponent_part.0.parse::<i64>().ok()?;
        if exponent_part.1 {
            e *= -1;
        }
        e
    } else {
        0
    };
    if exponent >= decimal_part.len() as i64 && exponent <= INTEGER_CONSTRUCTION_LIMIT {
        let exponent = exponent as u64 - decimal_part.len() as u64;
        let n = format!("{integer_part}{decimal_part}")
            .parse::<Integer>()
            .ok()?;
        let num = n * Integer::u64_pow_u64(10, exponent).complete();
        Some(Number::Int(num))
    } else {
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
            Some(Number::Int(n))
        } else {
            Some(Number::Float(x.into()))
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
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
                level: -1,
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
                level: 0,
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
        // NOTE: is planned to change if multitermials are added
        assert_eq!(jobs, []);
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
                level: -1,
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
                level: 0,
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
        let jobs = parse("a factorial -(--(-15))!", true);
        assert_eq!(
            jobs,
            [CalculationJob {
                base: CalculationBase::Num((-15).into()),
                level: 1,
                negative: 1
            }]
        );
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
        assert_eq!(num, Some(Number::Int(1.into())));
        let num = parse_num(&mut "1.0more !");
        assert_eq!(num, Some(Number::Int(1.into())));
        let num = parse_num(&mut "1.5e2more !");
        assert_eq!(num, Some(Number::Int(150.into())));
        let num = parse_num(&mut "1e2more !");
        assert_eq!(num, Some(Number::Int(100.into())));
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
}
