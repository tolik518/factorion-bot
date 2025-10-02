//! Parses comments and generates the reply.

use crate::rug::integer::IntegerExt64;
use crate::rug::{Complete, Integer};

use crate::calculation_results::Calculation;
use crate::calculation_tasks::CalculationJob;
use crate::parse::parse;

use std::fmt::Write;
use std::ops::*;
macro_rules! impl_bitwise {
    ($s_name:ident {$($s_fields:ident),*}, $t_name:ident, $fn_name:ident) => {
        impl $t_name for $s_name {
            type Output = Self;
            fn $fn_name(self, rhs: Self) -> Self {
                Self {
                    $($s_fields: self.$s_fields.$fn_name(rhs.$s_fields),)*
                }
            }
        }
    };
}
macro_rules! impl_all_bitwise {
    ($s_name:ident {$($s_fields:ident,)*}) => {impl_all_bitwise!($s_name {$($s_fields),*});};
    ($s_name:ident {$($s_fields:ident),*}) => {
        impl_bitwise!($s_name {$($s_fields),*}, BitOr, bitor);
        impl_bitwise!($s_name {$($s_fields),*}, BitXor, bitxor);
        impl_bitwise!($s_name {$($s_fields),*}, BitAnd, bitand);
        impl Not for $s_name {
            type Output = Self;
            fn not(self) -> Self {
                Self {
                    $($s_fields: self.$s_fields.not(),)*
                }
            }
        }
    };
}

/// The primary abstraction.
/// Construct -> Extract -> Calculate -> Get Reply
///
/// Uses a generic for Metadata (meta).
///
/// Uses three type-states exposed as the aliases [CommentConstructed], [CommentExtracted], and [CommentCalculated].
#[derive(Debug, Clone, PartialEq, PartialOrd, Ord, Eq)]
pub struct Comment<Meta, S> {
    /// Metadata (generic)
    pub meta: Meta,
    /// Data for the current step
    pub calculation_list: S,
    /// If Some will prepend a "Hey {string}!" to the reply.
    pub notify: Option<String>,
    pub status: Status,
    pub commands: Commands,
    /// How long the reply may at most be
    pub max_length: usize,
}
/// Base [Comment], contains the comment text, if it might have a calculation. Use [extract](Comment::extract).
pub type CommentConstructed<Meta> = Comment<Meta, String>;
/// Extracted [Comment], contains the calculations to be done. Use [calc](Comment::calc).
pub type CommentExtracted<Meta> = Comment<Meta, Vec<CalculationJob>>;
/// Calculated [Comment], contains the results along with how we go to them. Use [get_reply](Comment::get_reply).
pub type CommentCalculated<Meta> = Comment<Meta, Vec<Calculation>>;

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Default)]
pub struct Status {
    pub already_replied_or_rejected: bool,
    pub not_replied: bool,
    pub number_too_big_to_calculate: bool,
    pub no_factorial: bool,
    pub reply_would_be_too_long: bool,
    pub factorials_found: bool,
}

impl_all_bitwise!(Status {
    already_replied_or_rejected,
    not_replied,
    number_too_big_to_calculate,
    no_factorial,
    reply_would_be_too_long,
    factorials_found,
});
#[allow(dead_code)]
impl Status {
    pub const NONE: Self = Self {
        already_replied_or_rejected: false,
        not_replied: false,
        number_too_big_to_calculate: false,
        no_factorial: false,
        reply_would_be_too_long: false,
        factorials_found: false,
    };
    pub const ALREADY_REPLIED_OR_REJECTED: Self = Self {
        already_replied_or_rejected: true,
        ..Self::NONE
    };
    pub const NOT_REPLIED: Self = Self {
        not_replied: true,
        ..Self::NONE
    };
    pub const NUMBER_TOO_BIG_TO_CALCULATE: Self = Self {
        number_too_big_to_calculate: true,
        ..Self::NONE
    };
    pub const NO_FACTORIAL: Self = Self {
        no_factorial: true,
        ..Self::NONE
    };
    pub const REPLY_WOULD_BE_TOO_LONG: Self = Self {
        reply_would_be_too_long: true,
        ..Self::NONE
    };
    pub const FACTORIALS_FOUND: Self = Self {
        factorials_found: true,
        ..Self::NONE
    };
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Default, PartialOrd, Ord)]
pub struct Commands {
    /// Turn all integers into scientific notiation if that makes them shorter.
    pub shorten: bool,
    /// Return all the intermediate results for nested calculations.
    pub steps: bool,
    /// Parse and calculate termials.
    pub termial: bool,
    /// Disable the beginning note.
    pub no_note: bool,
    pub post_only: bool,
}
impl_all_bitwise!(Commands {
    shorten,
    steps,
    termial,
    no_note,
    post_only,
});
#[allow(dead_code)]
impl Commands {
    pub const NONE: Self = Self {
        shorten: false,
        steps: false,
        termial: false,
        no_note: false,
        post_only: false,
    };
    pub const SHORTEN: Self = Self {
        shorten: true,
        ..Self::NONE
    };
    pub const STEPS: Self = Self {
        steps: true,
        ..Self::NONE
    };
    pub const TERMIAL: Self = Self {
        termial: true,
        ..Self::NONE
    };
    pub const NO_NOTE: Self = Self {
        no_note: true,
        ..Self::NONE
    };
    pub const POST_ONLY: Self = Self {
        post_only: true,
        ..Self::NONE
    };
}

impl Commands {
    fn contains_command_format(text: &str, command: &str) -> bool {
        let pattern1 = format!("\\[{command}\\]");
        let pattern2 = format!("[{command}]");
        let pattern3 = format!("!{command}");
        text.contains(&pattern1) || text.contains(&pattern2) || text.contains(&pattern3)
    }

    pub fn from_comment_text(text: &str) -> Self {
        Self {
            shorten: Self::contains_command_format(text, "short")
                || Self::contains_command_format(text, "shorten"),
            steps: Self::contains_command_format(text, "steps")
                || Self::contains_command_format(text, "all"),
            termial: Self::contains_command_format(text, "termial")
                || Self::contains_command_format(text, "triangle"),
            no_note: Self::contains_command_format(text, "no note")
                || Self::contains_command_format(text, "no_note"),
            post_only: false,
        }
    }
    pub fn overrides_from_comment_text(text: &str) -> Self {
        Self {
            shorten: !Self::contains_command_format(text, "long"),
            steps: !(Self::contains_command_format(text, "no steps")
                | Self::contains_command_format(text, "no_steps")),
            termial: !(Self::contains_command_format(text, "no termial")
                | Self::contains_command_format(text, "no_termial")),
            no_note: !Self::contains_command_format(text, "note"),
            post_only: true,
        }
    }
}

const FOOTER_TEXT: &str = "\n*^(This action was performed by a bot.)*";

macro_rules! contains_comb {
    // top level (advance both separately)
    ($var:ident, [$start:tt,$($start_rest:tt),* $(,)?], [$end:tt,$($end_rest:tt),* $(,)?]) => {
        $var.contains(concat!($start, $end)) || contains_comb!($var, [$($start_rest),*], [$end,$($end_rest),*]) || contains_comb!(@inner $var, [$start,$($start_rest),*], [$($end_rest),*])
    };
    // inner (advance only end)
    (@inner $var:ident, [$start:tt,$($start_rest:tt),* $(,)?], [$end:tt,$($end_rest:tt),* $(,)?]) => {
        $var.contains(concat!($start,$end)) || contains_comb!(@inner $var, [$start,$($start_rest),*], [$($end_rest),*])
    };
    // top level (advance both separately) singular end (advance only start)
    ($var:ident, [$start:tt,$($start_rest:tt),* $(,)?], [$end:tt $(,)?]) => {
        $var.contains(concat!($start, $end)) || contains_comb!($var, [$($start_rest),*], [$end])
    };
    // top level (advance both separately) singular start (advance only end)
    ($var:ident, [$start:tt $(,)?], [$end:tt,$($end_rest:tt),* $(,)?]) => {
        $var.contains(concat!($start, $end)) || contains_comb!(@inner $var, [$start], [$($end_rest),*])
    };
    // inner (advance only end) singular end (advance only start, so nothing)
    (@inner $var:ident, [$start:tt,$($start_rest:tt),* $(,)?], [$end:tt $(,)?]) => {
        $var.contains(concat!($start,$end))
    };
    // inner (advance only end) singular end (advance only end)
    (@inner $var:ident, [$start:tt $(,)?], [$end:tt,$($end_rest:tt),* $(,)?]) => {
        $var.contains(concat!($start,$end)) || contains_comb!(@inner $var, [$start], [$($end_rest),*])
    };
    // top level (advance both separately) singular start and end (no advance)
    ($var:ident, [$start:tt $(,)?], [$end:tt $(,)?]) => {
        $var.contains(concat!($start, $end))
    };
    // inner (advance only end) singular start and end (no advance)
    (@inner $var:ident, [$start:tt $(,)?], [$end:tt $(,)?]) => {
        $var.contains(concat!($start,$end))
    };
}

impl<Meta> CommentConstructed<Meta> {
    /// Takes a raw comment, finds the factorials and commands, and packages it, also checks if it might have something to calculate.
    pub fn new(comment_text: &str, meta: Meta, pre_commands: Commands, max_length: usize) -> Self {
        let command_overrides = Commands::overrides_from_comment_text(comment_text);
        let commands: Commands =
            (Commands::from_comment_text(comment_text) | pre_commands) & command_overrides;

        let mut status: Status = Default::default();

        let text = if Self::might_have_factorial(comment_text) {
            comment_text.to_owned()
        } else {
            status.no_factorial = true;
            String::new()
        };

        Comment {
            meta,
            notify: None,
            calculation_list: text,
            status,
            commands,
            max_length: max_length - FOOTER_TEXT.len() - 10,
        }
    }

    fn might_have_factorial(text: &str) -> bool {
        contains_comb!(
            text,
            [
                "0", "1", "2", "3", "4", "5", "6", "7", "8", "9", ")", "e", "pi", "phi", "tau",
                "π", "ɸ", "τ"
            ],
            ["!", "?"]
        ) || contains_comb!(
            text,
            ["!"],
            [
                "0", "1", "2", "3", "4", "5", "6", "7", "8", "9", "(", "e", "pi", "phi", "tau",
                "π", "ɸ", "τ"
            ]
        )
    }

    /// Extracts the calculations using [parse](mod@crate::parse).
    pub fn extract(self) -> CommentExtracted<Meta> {
        let Comment {
            meta,
            calculation_list: comment_text,
            notify,
            mut status,
            commands,
            max_length,
        } = self;
        let pending_list: Vec<CalculationJob> = parse(&comment_text, commands.termial);

        if pending_list.is_empty() {
            status.no_factorial = true;
        }

        Comment {
            meta,
            calculation_list: pending_list,
            notify,
            status,
            commands,
            max_length,
        }
    }

    /// Constructs an empty comment with [Status] already_replied_or_rejected set.
    pub fn new_already_replied(meta: Meta, max_length: usize) -> Self {
        let text = String::new();
        let status: Status = Status {
            already_replied_or_rejected: true,
            ..Default::default()
        };
        let commands: Commands = Default::default();

        Comment {
            meta,
            notify: None,
            calculation_list: text,
            status,
            commands,
            max_length: max_length - FOOTER_TEXT.len() - 10,
        }
    }
}
impl<Meta, S> Comment<Meta, S> {
    pub fn add_status(&mut self, status: Status) {
        self.status = self.status | status;
    }
}
impl<Meta> CommentExtracted<Meta> {
    /// Does the calculations using [calculation_tasks](crate::calculation_tasks).
    pub fn calc(self) -> CommentCalculated<Meta> {
        let Comment {
            meta,
            calculation_list: pending_list,
            notify,
            mut status,
            commands,
            max_length,
        } = self;
        let mut calculation_list: Vec<Calculation> = pending_list
            .into_iter()
            .flat_map(|calc| calc.execute(commands.steps))
            .filter_map(|x| {
                if x.is_none() {
                    status.number_too_big_to_calculate = true;
                };
                x
            })
            .collect();

        calculation_list.sort();
        calculation_list.dedup();
        calculation_list.sort_by_key(|x| x.steps.len());

        if calculation_list.is_empty() {
            status.no_factorial = true;
        } else {
            status.factorials_found = true;
        }
        Comment {
            meta,
            calculation_list,
            notify,
            status,
            commands,
            max_length,
        }
    }
}
impl<Meta> CommentCalculated<Meta> {
    /// Does the formatting for the reply using [calculation_result](crate::calculation_results).
    pub fn get_reply(&self) -> String {
        let mut note = self
            .notify
            .as_ref()
            .map(|user| format!("Hey {user}! \n\n"))
            .unwrap_or_default();

        let too_big_number = Integer::u64_pow_u64(10, self.max_length as u64).complete();
        let too_big_number = &too_big_number;

        // Add Note
        let multiple = self.calculation_list.len() > 1;
        if !self.commands.no_note {
            if self
                .calculation_list
                .iter()
                .any(Calculation::is_digit_tower)
            {
                if multiple {
                    let _ = note.write_str("Some of these are so large, that I can't even give the number of digits of them, so I have to make a power of ten tower.\n\n");
                } else {
                    let _ = note.write_str("That is so large, that I can't even give the number of digits of it, so I have to make a power of ten tower.\n\n");
                }
            } else if self
                .calculation_list
                .iter()
                .any(Calculation::is_aproximate_digits)
            {
                if multiple {
                    let _ = note.write_str("Some of these are so large, that I can't even approximate them well, so I can only give you an approximation on the number of digits.\n\n");
                } else {
                    let _ = note.write_str("That number is so large, that I can't even approximate it well, so I can only give you an approximation on the number of digits.\n\n");
                }
            } else if self
                .calculation_list
                .iter()
                .any(Calculation::is_approximate)
            {
                if multiple {
                    let _ = note.write_str(
                "Some of those are so large, that I can't calculate them, so I'll have to approximate.\n\n",
            );
                } else {
                    let _ = note.write_str(
                "That is so large, that I can't calculate it, so I'll have to approximate.\n\n",
            );
                }
            } else if self.calculation_list.iter().any(Calculation::is_rounded) {
                let _ = note.write_str("I can't calculate that large factorials of decimals. So I had to round at some point.\n\n");
            } else if self
                .calculation_list
                .iter()
                .any(|c| c.is_too_long(too_big_number))
            {
                if multiple {
                    let _ = note.write_str("If I post the whole numbers, the comment would get too long. So I had to turn them into scientific notation.\n\n");
                } else {
                    let _ = note.write_str("If I post the whole number, the comment would get too long. So I had to turn it into scientific notation.\n\n");
                }
            }
        }

        // Add Factorials
        let mut reply = self
            .calculation_list
            .iter()
            .fold(note.clone(), |mut acc, factorial| {
                let _ = factorial.format(&mut acc, self.commands.shorten, false, too_big_number);
                acc
            });

        // If the reply was too long try force shortening all factorials
        if reply.len() > self.max_length
            && !self.commands.shorten
            && !self
                .calculation_list
                .iter()
                .all(|fact| fact.is_too_long(too_big_number))
        {
            if note.is_empty() && !self.commands.no_note {
                let _ = note.write_str("If I post the whole numbers, the comment would get too long. So I had to turn them into scientific notation.\n\n");
            };
            reply = self
                .calculation_list
                .iter()
                .fold(note, |mut acc, factorial| {
                    let _ = factorial.format(&mut acc, true, false, too_big_number);
                    acc
                });
        }

        // Remove factorials until we can fit them in a comment
        let note = "If I posted all numbers, the comment would get too long. So I had to remove some of them. \n\n";
        if reply.len() > self.max_length {
            let mut factorial_list: Vec<String> = self
                .calculation_list
                .iter()
                .map(|fact| {
                    let mut res = String::new();
                    let _ = fact.format(&mut res, true, false, too_big_number);
                    res
                })
                .collect();
            'drop_last: {
                while note.len() + factorial_list.iter().map(|s| s.len()).sum::<usize>()
                    > self.max_length
                {
                    // remove last factorial (probably the biggest)
                    factorial_list.pop();
                    if factorial_list.is_empty() {
                        if self.calculation_list.len() == 1 {
                            let note = "That is so large, I can't even fit it in a comment with a power of 10 tower, so I'll have to use tetration!\n\n";
                            reply = self.calculation_list.iter().fold(
                                note.to_string(),
                                |mut acc, factorial| {
                                    let _ = factorial.format(&mut acc, true, true, too_big_number);
                                    acc
                                },
                            );
                            if reply.len() <= self.max_length {
                                break 'drop_last;
                            }
                        }
                        reply = "Sorry, but the reply text for all those number would be _really_ long, so I'd rather not even try posting lmao\n".to_string();
                        break 'drop_last;
                    }
                }
                reply = factorial_list
                    .iter()
                    .fold(note.to_string(), |acc, factorial| {
                        format!("{acc}{factorial}")
                    });
            }
        }

        // Check if any of the calculated results are factorions and add interesting message
        let factorions: Vec<String> = self
            .calculation_list
            .iter()
            .filter_map(|calc| {
                if calc.is_factorion() {
                    if let crate::calculation_results::CalculationResult::Exact(ref num) = calc.result {
                        Some(num.to_string())
                    } else {
                        None
                    }
                } else {
                    None
                }
            })
            .collect();

        if !factorions.is_empty() {
            let factorion_message = if factorions.len() == 1 {
                format!(
                    "\n**Interesting!** {} is a [factorion](https://en.wikipedia.org/wiki/Factorion) - a number that equals the sum of the factorial of its digits!\n",
                    factorions[0]
                )
            } else {
                format!(
                    "\n**Interesting!** {} are [factorions](https://en.wikipedia.org/wiki/Factorion) - numbers that equal the sum of the factorial of their digits!\n",
                    factorions.join(", ")
                )
            };
            reply.push_str(&factorion_message);
        }

        reply.push_str(FOOTER_TEXT);
        reply
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        calculation_results::Number,
        calculation_tasks::{CalculationBase, CalculationJob},
    };

    const MAX_LENGTH: usize = 10_000;

    use super::*;

    type Comment<S> = super::Comment<(), S>;

    #[test]
    fn test_extraction_dedup() {
        let _ = crate::init_default();
        let jobs = parse("24! -24! 2!? (2!?)!", true);
        assert_eq!(
            jobs,
            [
                CalculationJob {
                    base: CalculationBase::Num(Number::Exact(24.into())),
                    level: 1,
                    negative: 0
                },
                CalculationJob {
                    base: CalculationBase::Num(Number::Exact(24.into())),
                    level: 1,
                    negative: 1
                },
                CalculationJob {
                    base: CalculationBase::Calc(Box::new(CalculationJob {
                        base: CalculationBase::Num(Number::Exact(2.into())),
                        level: 1,
                        negative: 0
                    })),
                    level: -1,
                    negative: 0
                },
                CalculationJob {
                    base: CalculationBase::Calc(Box::new(CalculationJob {
                        base: CalculationBase::Calc(Box::new(CalculationJob {
                            base: CalculationBase::Num(Number::Exact(2.into())),
                            level: 1,
                            negative: 0
                        })),
                        level: -1,
                        negative: 0
                    })),
                    level: 1,
                    negative: 0
                }
            ]
        );
    }

    #[test]
    fn test_commands_from_comment_text() {
        let _ = crate::init_default();
        let cmd1 = Commands::from_comment_text("!shorten!all !triangle !no_note");
        assert!(cmd1.shorten);
        assert!(cmd1.steps);
        assert!(cmd1.termial);
        assert!(cmd1.no_note);
        assert!(!cmd1.post_only);
        let cmd2 = Commands::from_comment_text("[shorten][all] [triangle] [no_note]");
        assert!(cmd2.shorten);
        assert!(cmd2.steps);
        assert!(cmd2.termial);
        assert!(cmd2.no_note);
        assert!(!cmd2.post_only);
        let comment = r"\[shorten\]\[all\] \[triangle\] \[no_note\]";
        let cmd3 = Commands::from_comment_text(comment);
        assert!(cmd3.shorten);
        assert!(cmd3.steps);
        assert!(cmd3.termial);
        assert!(cmd3.no_note);
        assert!(!cmd3.post_only);
        let cmd4 = Commands::from_comment_text("shorten all triangle no_note");
        assert!(!cmd4.shorten);
        assert!(!cmd4.steps);
        assert!(!cmd4.termial);
        assert!(!cmd4.no_note);
        assert!(!cmd4.post_only);
    }

    #[test]
    fn test_commands_overrides_from_comment_text() {
        let _ = crate::init_default();
        let cmd1 = Commands::overrides_from_comment_text("long no_steps no_termial note");
        assert!(cmd1.shorten);
        assert!(cmd1.steps);
        assert!(cmd1.termial);
        assert!(cmd1.no_note);
        assert!(cmd1.post_only);
    }

    #[test]
    fn test_might_have_factorial() {
        let _ = crate::init_default();
        assert!(Comment::might_have_factorial("5!"));
        assert!(Comment::might_have_factorial("3?"));
        assert!(!Comment::might_have_factorial("!?"));
    }

    #[test]
    fn test_new_already_replied() {
        let _ = crate::init_default();
        let comment = Comment::new_already_replied((), MAX_LENGTH);
        assert_eq!(comment.calculation_list, "");
        assert!(comment.status.already_replied_or_rejected);
    }
}
