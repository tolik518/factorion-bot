use crate::factorial::{
    CalculatedFactorial, Factorial, UPPER_APPROXIMATION_LIMIT, UPPER_CALCULATION_LIMIT,
    UPPER_SUBFACTORIAL_LIMIT,
};
use crate::math::{self, FLOAT_PRECISION};
use fancy_regex::Regex;
use num_traits::ToPrimitive;
use rug::ops::Pow;
use rug::{Float, Integer};
use std::fmt::Write;
use std::str::FromStr;
use std::sync::LazyLock;

#[derive(Debug)]
pub(crate) struct RedditComment {
    pub(crate) id: String,
    pub(crate) factorial_list: Vec<Factorial>,
    pub(crate) author: String,
    pub(crate) subreddit: String,
    pub(crate) status: Status,
    pub(crate) commands: Commands,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Default)]
pub(crate) struct Status {
    pub(crate) already_replied_or_rejected: bool,
    pub(crate) not_replied: bool,
    pub(crate) number_too_big_to_calculate: bool,
    pub(crate) no_factorial: bool,
    pub(crate) reply_would_be_too_long: bool,
    pub(crate) factorials_found: bool,
}

impl std::ops::BitOr for Status {
    type Output = Status;
    fn bitor(self, rhs: Self) -> Self::Output {
        Status {
            already_replied_or_rejected: self.already_replied_or_rejected
                | rhs.already_replied_or_rejected,
            not_replied: self.not_replied | rhs.not_replied,
            number_too_big_to_calculate: self.number_too_big_to_calculate
                | rhs.number_too_big_to_calculate,
            no_factorial: self.no_factorial | rhs.no_factorial,
            reply_would_be_too_long: self.reply_would_be_too_long | rhs.reply_would_be_too_long,
            factorials_found: self.factorials_found | rhs.factorials_found,
        }
    }
}
#[allow(dead_code)]
impl Status {
    pub(crate) const NONE: Self = Self {
        already_replied_or_rejected: false,
        not_replied: false,
        number_too_big_to_calculate: false,
        no_factorial: false,
        reply_would_be_too_long: false,
        factorials_found: false,
    };
    pub(crate) const ALREADY_REPLIED_OR_REJECTED: Self = Self {
        already_replied_or_rejected: true,
        ..Self::NONE
    };
    pub(crate) const NOT_REPLIED: Self = Self {
        not_replied: true,
        ..Self::NONE
    };
    pub(crate) const NUMBER_TOO_BIG_TO_CALCULATE: Self = Self {
        number_too_big_to_calculate: true,
        ..Self::NONE
    };
    pub(crate) const NO_FACTORIAL: Self = Self {
        no_factorial: true,
        ..Self::NONE
    };
    pub(crate) const REPLY_WOULD_BE_TOO_LONG: Self = Self {
        reply_would_be_too_long: true,
        ..Self::NONE
    };
    pub(crate) const FACTORIALS_FOUND: Self = Self {
        factorials_found: true,
        ..Self::NONE
    };
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Default)]
pub(crate) struct Commands {
    shorten: bool,
    include_steps: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct PendingFactorial {
    base: PendingFactorialBase,
    level: i32,
}
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
enum PendingFactorialBase {
    Number(Integer),
    Factorial(Box<PendingFactorial>),
}

pub(crate) const PLACEHOLDER: &str = "factorial of ";
const FOOTER_TEXT: &str =
    "\n*^(This action was performed by a bot. Please DM me if you have any questions.)*";
pub(crate) const MAX_COMMENT_LENGTH: i64 = 10_000 - 10 - FOOTER_TEXT.len() as i64;
pub(crate) const NUMBER_DECIMALS_SCIENTIFIC: usize = 30;

impl RedditComment {
    pub(crate) fn new(comment_text: &str, id: &str, author: &str, subreddit: &str) -> Self {
        static FACTORIAL_REGEX: LazyLock<Regex> = LazyLock::new(|| {
            Regex::new(r"(?<![,.?!\d])\b(\d+)(!+)(?![<\d]|&lt;)").expect("Invalid factorial regex")
        });
        static SUBFACTORIAL_REGEX: LazyLock<Regex> = LazyLock::new(|| {
            Regex::new(r"(?<![,.!?\d])(!)\b(\d+)(?![<\d]|&lt;)")
                .expect("Invalid subfactorial regex")
        });
        static FACTORIAL_CHAIN_REGEX: LazyLock<Regex> = LazyLock::new(|| {
            Regex::new(r"(?<![,.?!\d])\(([\d!\(\)]+)\)(!+)(?![<\d]|&lt;)")
                .expect("Invalid factorial-chain regex")
        });

        let mut commands: Commands = Commands {
            shorten: false,
            include_steps: false,
        };

        if comment_text.contains("\\[short\\]")
            || comment_text.contains("\\[shorten\\]")
            || comment_text.contains("!short")
            || comment_text.contains("!shorten")
        {
            commands.shorten = true;
        }
        if comment_text.contains("\\[steps\\]")
            || comment_text.contains("\\[all\\]")
            || comment_text.contains("!steps")
            || comment_text.contains("!all")
        {
            commands.include_steps = true;
        }

        let mut factorial_list: Vec<PendingFactorial> = Vec::new();
        let mut status: Status = Default::default();

        // for every regex/factorial in the comment
        for regex_capture in FACTORIAL_REGEX.captures_iter(comment_text) {
            let regex_capture = regex_capture.expect("Failed to capture regex");

            let num = regex_capture[1]
                .parse::<Integer>()
                .expect("Failed to parse number");

            let factorial_level = regex_capture[2]
                .len()
                .to_i32()
                .expect("Failed to convert exclamation count to i32");

            factorial_list.push(PendingFactorial {
                base: PendingFactorialBase::Number(num),
                level: factorial_level,
            });
        }

        for regex_capture in SUBFACTORIAL_REGEX.captures_iter(comment_text) {
            let regex_capture = regex_capture.expect("Failed to capture regex");

            let num = regex_capture[2]
                .parse::<Integer>()
                .expect("Failed to parse number");

            factorial_list.push(PendingFactorial {
                base: PendingFactorialBase::Number(num),
                level: -1,
            });
        }

        for regex_capture in FACTORIAL_CHAIN_REGEX.captures_iter(comment_text) {
            let regex_capture = regex_capture.expect("Failed to capture regex");

            // Get outermost capture info (level and inner)
            let mut factorial_levels = vec![regex_capture[2]
                .len()
                .to_i32()
                .expect("Failed to convert exclamation count to i32")];
            let mut current_string = regex_capture[1].to_string();

            // Recurse to the innermost chain capture
            while let Some(regex_capture) = FACTORIAL_CHAIN_REGEX
                .captures(&current_string)
                .expect("Failed to capture regex")
            {
                factorial_levels.push(
                    regex_capture[2]
                        .len()
                        .to_i32()
                        .expect("Failed to convert exclamation count to i32"),
                );
                current_string = regex_capture[1].to_string();
            }

            // Get the normal factorial at the core
            let Some(regex_capture) = FACTORIAL_REGEX
                .captures(&current_string)
                .expect("Failed to capture regex")
            else {
                continue;
            };

            let factorial_level = regex_capture[2]
                .len()
                .to_i32()
                .expect("Failed to convert exclamation count to i32");
            let num = regex_capture[1]
                .parse::<Integer>()
                .expect("Failed to parse number");

            // Package it all as a PendingFactorial
            let mut factorial = PendingFactorial {
                base: PendingFactorialBase::Number(num),
                level: factorial_level,
            };
            // Remove duplicate base (also captured by factorial regex)
            if let Some((i, _)) = factorial_list
                .iter()
                .enumerate()
                .find(|(_, fact)| *fact == &factorial)
            {
                factorial_list.remove(i);
            }
            for factorial_level in factorial_levels.into_iter().rev() {
                factorial = PendingFactorial {
                    base: PendingFactorialBase::Factorial(Box::new(factorial)),
                    level: factorial_level,
                }
            }
            factorial_list.push(factorial);
        }

        factorial_list.sort();
        factorial_list.dedup();

        let factorial_list: Vec<Factorial> = factorial_list
            .into_iter()
            .flat_map(|fact| Self::calculate_pending(fact, commands.include_steps))
            .filter_map(|x| {
                if x.is_none() {
                    status.number_too_big_to_calculate = true;
                };
                x
            })
            .collect();

        if factorial_list.is_empty() {
            status.no_factorial = true;
        } else {
            status.factorials_found = true;
        }

        RedditComment {
            id: id.to_string(),
            author: author.to_string(),
            subreddit: subreddit.to_string(),
            factorial_list,
            status,
            commands,
        }
    }

    fn calculate_pending(
        PendingFactorial { base, level }: PendingFactorial,
        include_steps: bool,
    ) -> Vec<Option<Factorial>> {
        match base {
            PendingFactorialBase::Number(num) => {
                vec![Self::calculate_appropriate_factorial(num, level)]
            }
            PendingFactorialBase::Factorial(factorial) => {
                let mut factorials = Self::calculate_pending(*factorial, include_steps);
                match factorials.last() {
                    Some(Some(Factorial {
                        factorial: res,
                        levels,
                        number,
                    })) => {
                        let res = match res {
                            CalculatedFactorial::Exact(res) => res.clone(),
                            CalculatedFactorial::Approximate(base, exponent) => {
                                let res = base * Float::with_val(FLOAT_PRECISION, 10).pow(exponent);
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
                if num > Integer::from_str(UPPER_APPROXIMATION_LIMIT).unwrap()
                    || (level > 1 && num > UPPER_CALCULATION_LIMIT)
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
                        factorial: CalculatedFactorial::Approximate(factorial.0, factorial.1),
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
            //TODO: Implement subfactorial further
            if num > UPPER_SUBFACTORIAL_LIMIT {
                None
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

    pub(crate) fn new_already_replied(id: &str, author: &str, subreddit: &str) -> Self {
        let factorial_list: Vec<Factorial> = Vec::new();
        let status: Status = Status {
            already_replied_or_rejected: true,
            ..Default::default()
        };
        let commands: Commands = Default::default();

        RedditComment {
            id: id.to_string(),
            author: author.to_string(),
            subreddit: subreddit.to_string(),
            factorial_list,
            status,
            commands,
        }
    }

    pub(crate) fn add_status(&mut self, status: Status) {
        self.status = self.status | status;
    }

    pub(crate) fn get_reply(&self) -> String {
        let mut note = String::new();

        // Add Note
        let multiple = self.factorial_list.len() > 1;
        if self
            .factorial_list
            .iter()
            .any(Factorial::is_aproximate_digits)
        {
            if multiple {
                let _ = note.write_str("Some of these are so large, that I can't even approximate them well, so I can only give you an approximation on the number of digits.\n\n");
            } else {
                let _ = note.write_str("That number is so large, that I can't even approximate it well, so I can only give you an approximation on the number of digits.\n\n");
            }
        } else if self.factorial_list.iter().any(Factorial::is_approximate) {
            if multiple {
                let _ = note.write_str(
                "Sorry, some of those are so large, that I can't calculate them, so I'll have to approximate.\n\n",
            );
            } else {
                let _ = note.write_str(
                "Sorry, that is so large, that I can't calculate it, so I'll have to approximate.\n\n",
            );
            }
        } else if self.factorial_list.iter().any(Factorial::is_too_long) {
            if multiple {
                let _ = note.write_str("If I post the whole numbers, the comment would get too long, as reddit only allows up to 10k characters. So I had to turn them into scientific notation.\n\n");
            } else {
                let _ = note.write_str("If I post the whole number, the comment would get too long, as reddit only allows up to 10k characters. So I had to turn it into scientific notation.\n\n");
            }
        }

        // Add Factorials
        let mut reply = self
            .factorial_list
            .iter()
            .fold(note.clone(), |mut acc, factorial| {
                let _ = factorial.format(&mut acc, self.commands.shorten);
                acc
            });

        // If the reply was too long try force shortening all factorials
        if reply.len() > MAX_COMMENT_LENGTH as usize
            && !self.commands.shorten
            && !self.factorial_list.iter().all(|fact| fact.is_too_long())
        {
            if note.is_empty() {
                let _ = note.write_str("If I post the whole numbers, the comment would get too long, as reddit only allows up to 10k characters. So I had to turn them into scientific notation.\n\n");
            };
            reply = self.factorial_list.iter().fold(note, |mut acc, factorial| {
                let _ = factorial.format(&mut acc, true);
                acc
            });
        }

        // Remove factorials until we can fit them in a comment
        let note = "If I posted all numbers, the comment would get too long, as reddit only allows up to 10k characters. So I had to remove some of them. \n\n";
        if reply.len() > MAX_COMMENT_LENGTH as usize {
            let mut factorial_list: Vec<String> = self
                .factorial_list
                .iter()
                .map(|fact| {
                    let mut res = String::new();
                    let _ = fact.format(&mut res, true);
                    res
                })
                .collect();
            'drop_last: {
                while note.len() + factorial_list.iter().map(|s| s.len()).sum::<usize>()
                    > MAX_COMMENT_LENGTH as usize
                {
                    // remove last factorial (probably the biggest)
                    factorial_list.pop();
                    if factorial_list.is_empty() {
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

        reply.push_str(FOOTER_TEXT);
        reply
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_comment_new() {
        let comment = RedditComment::new(
            "This is a test comment with a factorial of 5! and 6!",
            "123",
            "test_author",
            "test_subreddit",
        );
        assert_eq!(comment.id, "123");
        assert_eq!(
            comment.factorial_list,
            vec![
                Factorial {
                    number: 5.into(),
                    levels: vec![1],
                    factorial: CalculatedFactorial::Exact(Integer::from(120)),
                },
                Factorial {
                    number: 6.into(),
                    levels: vec![1],
                    factorial: CalculatedFactorial::Exact(Integer::from(720)),
                },
            ],
        );
        assert_eq!(comment.status, Status::FACTORIALS_FOUND);
    }

    #[test]
    fn test_comment_new_double_factorial() {
        let comment = RedditComment::new(
            "This is a test comment with an n-factorial 6!!",
            "123",
            "test_author",
            "test_subreddit",
        );
        assert_eq!(
            comment.factorial_list,
            vec![Factorial {
                number: 6.into(),
                levels: vec![2],
                factorial: CalculatedFactorial::Exact(Integer::from(48)),
            }]
        );
        assert_eq!(comment.status, Status::FACTORIALS_FOUND);
    }

    #[test]
    fn test_comment_new_triple_factorial() {
        let comment = RedditComment::new(
            "This is a test comment with an n-factorial 6!!!",
            "123",
            "test_author",
            "test_subreddit",
        );
        assert_eq!(
            comment.factorial_list,
            vec![Factorial {
                number: 6.into(),
                levels: vec![3],
                factorial: CalculatedFactorial::Exact(Integer::from(18)),
            }]
        );
        assert_eq!(comment.status, Status::FACTORIALS_FOUND);
    }

    #[test]
    fn test_comment_new_spoiler() {
        let comment = RedditComment::new(
            ">!This is a spoiler comment 5!<",
            "123",
            "test_author",
            "test_subreddit",
        );
        assert_eq!(comment.factorial_list, vec![]);
        assert_eq!(comment.status, Status::NO_FACTORIAL);
    }

    #[test]
    fn test_comment_new_spoiler_html_encoded() {
        let comment = RedditComment::new(
            "&gt;!This is a spoiler comment 5!&lt;",
            "123",
            "test_author",
            "test_subreddit",
        );
        assert_eq!(comment.factorial_list, vec![]);
        assert_eq!(comment.status, Status::NO_FACTORIAL);
    }

    #[test]
    fn test_comment_new_subfactorial() {
        let comment = RedditComment::new(
            "This is a spoiler comment !5",
            "123",
            "test_author",
            "test_subreddit",
        );

        assert_eq!(
            comment.factorial_list,
            vec![Factorial {
                number: 5.into(),
                levels: vec![-1],
                factorial: CalculatedFactorial::Exact(Integer::from(44)),
            }]
        );
    }

    #[test]
    fn test_comment_new_exclamations_one() {
        let comment = RedditComment::new(
            "This is a test with exclamation mark stuff!!!1!",
            "123",
            "test_author",
            "test_subreddit",
        );
        assert_eq!(comment.factorial_list, vec![]);
        assert_eq!(comment.status, Status::NO_FACTORIAL);
    }

    #[test]
    fn test_comment_new_exclamations_eleven() {
        let comment = RedditComment::new(
            "This is a test with exclamation mark stuff!!!11!",
            "123",
            "test_author",
            "test_subreddit",
        );
        assert_eq!(comment.factorial_list, vec![]);
        assert_eq!(comment.status, Status::NO_FACTORIAL);
    }

    #[test]
    fn test_comment_new_decimals() {
        let comment = RedditComment::new(
            "This is a test comment with decimal number 0.5!",
            "123",
            "test_author",
            "test_subreddit",
        );
        assert_eq!(comment.factorial_list, vec![]);
        assert_eq!(comment.status, Status::NO_FACTORIAL);
    }

    #[test]
    fn test_comment_new_comma_decimals() {
        let comment = RedditComment::new(
            "This is a test comment with decimal number 0,5!",
            "123",
            "test_author",
            "test_subreddit",
        );
        assert_eq!(comment.factorial_list, vec![]);
        assert_eq!(comment.status, Status::NO_FACTORIAL);
    }

    #[test]
    #[ignore = "currently obsolete"]
    fn test_comment_new_big_number_and_normal_number() {
        let comment = RedditComment::new(
            "This is a test comment with a factorial of 555555555555555555555555555555555555555555! and 6!",
            "123",
            "test_author",
            "test_subreddit",
        );
        assert_eq!(comment.id, "123");
        assert_eq!(
            comment.factorial_list,
            vec![Factorial {
                number: 6.into(),
                levels: vec![1],
                factorial: CalculatedFactorial::Exact(Integer::from(720))
            }]
        );
        assert_eq!(
            comment.status,
            Status::FACTORIALS_FOUND | Status::NUMBER_TOO_BIG_TO_CALCULATE
        );
    }

    #[test]
    #[ignore = "currently obsolete"]
    fn test_comment_new_very_big_number() {
        let very_big_number = "9".repeat(10_000) + "!";
        let comment = RedditComment::new(&very_big_number, "123", "test_author", "test_subreddit");
        assert_eq!(comment.id, "123");
        assert_eq!(comment.factorial_list, vec![]);
        assert_eq!(
            comment.status,
            Status::FACTORIALS_FOUND | Status::NUMBER_TOO_BIG_TO_CALCULATE
        );
    }

    #[test]
    fn test_add_status() {
        let mut comment = RedditComment::new(
            "This is a test comment with a factorial of 5! and 6!",
            "123",
            "test_author",
            "test_subreddit",
        );
        comment.add_status(Status::NOT_REPLIED);
        assert_eq!(
            comment.status,
            Status::FACTORIALS_FOUND | Status::NOT_REPLIED
        );
    }

    #[test]
    fn test_can_reply_to_factorial_that_is_subfactorial() {
        let comment = RedditComment::new(
            "This comment has a subfactorial which is also a factorial !23!",
            "123",
            "test_author",
            "test_subreddit",
        );
        assert_eq!(
            comment.get_reply(),
            "Subfactorial of 23 is 9510425471055777937262 \n\n\n*^(This action was performed by a bot. Please DM me if you have any questions.)*"
        );
    }

    #[test]
    fn test_command_shorten() {
        let comment = RedditComment::new(
            "This comment would like the short version of this factorial 200! \\[short\\]",
            "123",
            "test_author",
            "test_subreddit",
        );
        let reply = comment.get_reply();
        assert_eq!(reply, "The factorial of 200 is roughly 7.886578673647905035523632139322 × 10^374 \n\n\n*^(This action was performed by a bot. Please DM me if you have any questions.)*");
    }

    #[test]
    fn test_command_steps() {
        let comment = RedditComment::new(
            "This comment would like to know all the steps to this factorial chain ((3!)!)! \\[all\\] \\[short\\]",
            "123",
            "test_author",
            "test_subreddit",
        );
        let reply = comment.get_reply();
        assert_eq!(reply, "The factorial of 3 is 6 \n\nThe factorial of The factorial of 3 is 720 \n\nThe factorial of The factorial of The factorial of 3 is roughly 2.601218943565795100204903227081 × 10^1746 \n\n\n*^(This action was performed by a bot. Please DM me if you have any questions.)*");
    }

    #[test]
    fn test_reply_text_shorten() {
        let comment = RedditComment::new(
            "3500! 3501! 3502! 3503! 3504! 3505! 3506! 3507! 3508! 3509! 3510! 3511! 3512! 3513! 3514! 3515! 3516! 3517! 3518! 3519! 3520! 3521! 3522! 3523! 3524! 3525! 3526! 3527! 3528! 3529! 3530! 3531! 3532! 3533! 3534! 3535! 3536! 3537! 3538! 3539! 3540! 3541! 3542! 3543! 3544! 3545! 3546! 3547! 3548! 3549! 3550! 3551! 3552! 3553! 3554! 3555! 3556! 3557! 3558! 3559! 3560! 3561! 3562! 3563! 3564! 3565! 3566! 3567! 3568! 3569! 3570! 3571! 3572! 3573! 3574! 3575! 3576! 3577! 3578! 3579! 3580! 3581! 3582! 3583! 3584! 3585! 3586! 3587! 3588! 3589! 3590! 3591! 3592! 3593! 3594! 3595! 3596! 3597! 3598! 3599! 3600! 3600! 3601! 3602! 3603! 3604! 3605! 3606! 3607! 3608! 3609! 3610! 3611! 3612! 3613! 3614! 3615! 3616! 3617! 3618! 3619! 3620! 3621! 3622! 3623! 3624! 3625! 3626! 3627! 3628! 3629! 3630! 3631! 3632! 3633! 3634! 3636! 3636! 3637! 3638! 3639! 3640! 3641! 3642! 3643! 3644! 3645! 3646! 3647! 3648! 3649! 3650! 3651! 3652! 3653! 3654! 3655! 3656! 3657! 3658! 3659! 3660! 3661! 3662! 3663! 3664! 3665! 3666! 3667! 3668! 3669! 3670! 3671! 3672! 3673! 3674! 3675! 3676! 3677! 3678! 3679! 3680! 3681! 3682! 3683! 3684! 3685! 3686! 3687! 3688! 3689! 3690! 3691! 3692! 3693! 3694! 3695! 3696! 3697! 3698! 3699! 3600!",
            "123",
            "test_author",
            "test_subreddit"
        );
        let reply = comment.get_reply();
        assert_eq!(
            reply,
            "If I posted all numbers, the comment would get too long, as reddit only allows up to 10k characters. So I had to remove some of them. \n\nThe factorial of 3500 is roughly 2.391128199477649525095387493694 × 10^10886 \n\nThe factorial of 3501 is roughly 8.371339826371250987358951615421 × 10^10889 \n\nThe factorial of 3502 is roughly 2.931643207195212095773104855721 × 10^10893 \n\nThe factorial of 3503 is roughly 1.026954615480482797149318630959 × 10^10897 \n\nThe factorial of 3504 is roughly 3.598448972643611721211212482880 × 10^10900 \n\nThe factorial of 3505 is roughly 1.261256364911585908284529975249 × 10^10904 \n\nThe factorial of 3506 is roughly 4.421964815380020194445562093225 × 10^10907 \n\nThe factorial of 3507 is roughly 1.550783060753773082192058626094 × 10^10911 \n\nThe factorial of 3508 is roughly 5.440146977124235972329741660337 × 10^10914 \n\nThe factorial of 3509 is roughly 1.908947574272894402690506348612 × 10^10918 \n\nThe factorial of 3510 is roughly 6.700405985697859353443677283629 × 10^10921 \n\nThe factorial of 3511 is roughly 2.352512541578518418994075094282 × 10^10925 \n\nThe factorial of 3512 is roughly 8.262024046023756687507191731119 × 10^10928 \n\nThe factorial of 3513 is roughly 2.902449047368145724321276455142 × 10^10932 \n\nThe factorial of 3514 is roughly 1.019920595245166407526496546337 × 10^10936 \n\nThe factorial of 3515 is roughly 3.585020892286759922455635360374 × 10^10939 \n\nThe factorial of 3516 is roughly 1.260493345728024788735401392708 × 10^10943 \n\nThe factorial of 3517 is roughly 4.433155096925463181982406698153 × 10^10946 \n\nThe factorial of 3518 is roughly 1.559583963098377947421410676410 × 10^10950 \n\nThe factorial of 3519 is roughly 5.488175966143191996975944170287 × 10^10953 \n\nThe factorial of 3520 is roughly 1.931837940082403582935532347941 × 10^10957 \n\nThe factorial of 3521 is roughly 6.802001387030143015516009397101 × 10^10960 \n\nThe factorial of 3522 is roughly 2.395664888512016370064738509659 × 10^10964 \n\nThe factorial of 3523 is roughly 8.439927402227833671738073769528 × 10^10967 \n\nThe factorial of 3524 is roughly 2.974230416545088585920497196382 × 10^10971 \n\nThe factorial of 3525 is roughly 1.048416221832143726536975261725 × 10^10975 \n\nThe factorial of 3526 is roughly 3.696715598180138779769374772841 × 10^10978 \n\nThe factorial of 3527 is roughly 1.303831591478134947624658482381 × 10^10982 \n\nThe factorial of 3528 is roughly 4.599917854734860095219795125840 × 10^10985 \n\nThe factorial of 3529 is roughly 1.623311010935932127603065699909 × 10^10989 \n\nThe factorial of 3530 is roughly 5.730287868603840410438821920679 × 10^10992 \n\nThe factorial of 3531 is roughly 2.023364646404016048925948020192 × 10^10996 \n\nThe factorial of 3532 is roughly 7.146523931098984684806448407317 × 10^10999 \n\nThe factorial of 3533 is roughly 2.524866904857271289142118222305 × 10^11003 \n\nThe factorial of 3534 is roughly 8.922879641765596735828245797626 × 10^11006 \n\nThe factorial of 3535 is roughly 3.154237953364138446115284889461 × 10^11010 \n\nThe factorial of 3536 is roughly 1.115338540309559354546364736913 × 10^11014 \n\nThe factorial of 3537 is roughly 3.944952417074911437030492074462 × 10^11017 \n\nThe factorial of 3538 is roughly 1.395724165161103666421388095945 × 10^11021 \n\nThe factorial of 3539 is roughly 4.939467820505145875465292471549 × 10^11024 \n\nThe factorial of 3540 is roughly 1.748571608458821639914713534928 × 10^11028 \n\nThe factorial of 3541 is roughly 6.191692065552687426938000627181 × 10^11031 \n\nThe factorial of 3542 is roughly 2.193097329618761886621439822147 × 10^11035 \n\nThe factorial of 3543 is roughly 7.770143838839273364299761289869 × 10^11038 \n\nThe factorial of 3544 is roughly 2.753738976484638480307835401129 × 10^11042 \n\nThe factorial of 3545 is roughly 9.762004671638043412691276497004 × 10^11045 \n\nThe factorial of 3546 is roughly 3.461606856562850194140326645838 × 10^11049 \n\nThe factorial of 3547 is roughly 1.227831952022842963861573861279 × 10^11053 \n\nThe factorial of 3548 is roughly 4.356347765777046835780864059816 × 10^11056 \n\nThe factorial of 3549 is roughly 1.546067822074273922018628654829 × 10^11060 \n\nThe factorial of 3550 is roughly 5.488540768363672423166131724642 × 10^11063 \n\nThe factorial of 3551 is roughly 1.948980826845940077466293375421 × 10^11067 \n\nThe factorial of 3552 is roughly 6.922779896956779155160274069494 × 10^11070 \n\nThe factorial of 3553 is roughly 2.459663697388743633828445376891 × 10^11074 \n\nThe factorial of 3554 is roughly 8.741644780519594874626294869471 × 10^11077 \n\nThe factorial of 3555 is roughly 3.107654719474715977929647826097 × 10^11081 \n\nThe factorial of 3556 is roughly 1.105082018245209001751782766960 × 10^11085 \n\nThe factorial of 3557 is roughly 3.930776738898208419231091302077 × 10^11088 \n\nThe factorial of 3558 is roughly 1.398570363699982555562422285279 × 10^11092 \n\nThe factorial of 3559 is roughly 4.977511924408237915246660913308 × 10^11095 \n\nThe factorial of 3560 is roughly 1.771994245089332697827811285138 × 10^11099 \n\nThe factorial of 3561 is roughly 6.310071506763113736964835986375 × 10^11102 \n\nThe factorial of 3562 is roughly 2.247647470709021113106874578347 × 10^11106 \n\nThe factorial of 3563 is roughly 8.00836793813624222599979412265 × 10^11109 \n\nThe factorial of 3564 is roughly 2.854182333151756729346326625312 × 10^11113 \n\nThe factorial of 3565 is roughly 1.017516001768601274011965441924 × 10^11117 \n\nThe factorial of 3566 is roughly 3.628462062306832143126668765900 × 10^11120 \n\nThe factorial of 3567 is roughly 1.294272417624847025453282748797 × 10^11124 \n\nThe factorial of 3568 is roughly 4.617963986085454186817312847707 × 10^11127 \n\nThe factorial of 3569 is roughly 1.648151346633898599275098955346 × 10^11131 \n\nThe factorial of 3570 is roughly 5.883900307483017999412103270587 × 10^11134 \n\nThe factorial of 3571 is roughly 2.101140799802185727590062077927 × 10^11138 \n\nThe factorial of 3572 is roughly 7.505274936893407418951701742354 × 10^11141 \n\nThe factorial of 3573 is roughly 2.681634734952014470791443032543 × 10^11145 \n\nThe factorial of 3574 is roughly 9.584162542718499718608617398309 × 10^11148 \n\nThe factorial of 3575 is roughly 3.426338109021863649402580719895 × 10^11152 \n\nThe factorial of 3576 is roughly 1.225258507786218441026362865435 × 10^11156 \n\nThe factorial of 3577 is roughly 4.382749682351303363551299969659 × 10^11159 \n\nThe factorial of 3578 is roughly 1.568147836345296343478655129144 × 10^11163 \n\nThe factorial of 3579 is roughly 5.612401106279815613310106707207 × 10^11166 \n\nThe factorial of 3580 is roughly 2.009239596048173989565018201180 × 10^11170 \n\nThe factorial of 3581 is roughly 7.195086993448511056632330178426 × 10^11173 \n\nThe factorial of 3582 is roughly 2.577280161053256660485700669912 × 10^11177 \n\nThe factorial of 3583 is roughly 9.234394817053818614520265500295 × 10^11180 \n\nThe factorial of 3584 is roughly 3.309607102432088591444063155306 × 10^11184 \n\nThe factorial of 3585 is roughly 1.186494146221903760032696641177 × 10^11188 \n\nThe factorial of 3586 is roughly 4.254768008351746883477250155261 × 10^11191 \n\nThe factorial of 3587 is roughly 1.526185284595771607103289630692 × 10^11195 \n\nThe factorial of 3588 is roughly 5.475952801129628526286603194924 × 10^11198 \n\nThe factorial of 3589 is roughly 1.965319460325423678084261886658 × 10^11202 \n\nThe factorial of 3590 is roughly 7.055496862568271004322500173102 × 10^11205 \n\nThe factorial of 3591 is roughly 2.533628923348266117652209812161 × 10^11209 \n\nThe factorial of 3592 is roughly 9.100795092666971894606737645283 × 10^11212 \n\nThe factorial of 3593 is roughly 3.269915676795243001732200835950 × 10^11216 \n\nThe factorial of 3594 is roughly 1.175207694240210334822552980440 × 10^11220 \n\nThe factorial of 3595 is roughly 4.224871660793556153687077964683 × 10^11223 \n\nThe factorial of 3596 is roughly 1.519263849221362792865873236100 × 10^11227 \n\nThe factorial of 3597 is roughly 5.464792065649241965938546030252 × 10^11230 \n\nThe factorial of 3598 is roughly 1.966232185220597259344688861685 × 10^11234 \n\nThe factorial of 3599 is roughly 7.076469634608929536381535213204 × 10^11237 \n\nThe factorial of 3600 is roughly 2.547529068459214633097352676753 × 10^11241 \n\nThe factorial of 3601 is roughly 9.173652175521631893783566988988 × 10^11244 \n\nThe factorial of 3602 is roughly 3.304349513622891808140840829434 × 10^11248 \n\nThe factorial of 3603 is roughly 1.190557129758327918473144950845 × 10^11252 \n\nThe factorial of 3604 is roughly 4.290767895649013818177214402845 × 10^11255 \n\nThe factorial of 3605 is roughly 1.546821826381469481452885792226 × 10^11259 \n\nThe factorial of 3606 is roughly 5.577839505931578950119106166766 × 10^11262 \n\nThe factorial of 3607 is roughly 2.011926709789520527307961594352 × 10^11266 \n\nThe factorial of 3608 is roughly 7.259031568920590062527125432424 × 10^11269 \n\nThe factorial of 3609 is roughly 2.619784493223440953566039568562 × 10^11273 \n\nThe factorial of 3610 is roughly 9.457422020536621842373402842508 × 10^11276 \n\nThe factorial of 3611 is roughly 3.41507509161577414728103576643 × 10^11280 \n\nThe factorial of 3612 is roughly 1.233525123091617621997910118834 × 10^11284 \n\nThe factorial of 3613 is roughly 4.456726269730014468278449259348 × 10^11287 \n\nThe factorial of 3614 is roughly 1.610660873880427228835831562329 × 10^11291 \n\nThe factorial of 3615 is roughly 5.822539059077744432241531097818 × 10^11294 \n\nThe factorial of 3616 is roughly 2.105430123762512386698537644971 × 10^11298 \n\nThe factorial of 3617 is roughly 7.61534075764900730268861066186 × 10^11301 \n\nThe factorial of 3618 is roughly 2.755230286117410842112739337461 × 10^11305 \n\nThe factorial of 3619 is roughly 9.971178405458909837606003662271 × 10^11308 \n\nThe factorial of 3620 is roughly 3.609566582776125361213373325742 × 10^11312 \n\nThe factorial of 3621 is roughly 1.307024059623234993295362481251 × 10^11316 \n\n\n*^(This action was performed by a bot. Please DM me if you have any questions.)*"
        );
    }

    #[test]
    #[ignore = "currently obsolete"]
    fn test_reply_too_long() {
        let comment = RedditComment::new(
            &format!("{}!", "9".repeat(9999)),
            "1234",
            "test_author",
            "test_subreddit",
        );
        let reply = comment.get_reply();
        assert_eq!(
            reply,
            "Sorry, but the reply text for all those number would be _really_ long, so I'd rather not even try posting lmao\n\n*^(This action was performed by a bot. Please DM me if you have any questions.)*"
        );
    }

    #[test]
    fn test_get_reply_for_multifactorial() {
        let comment = RedditComment {
            id: "123".to_string(),
            factorial_list: vec![Factorial {
                number: 10.into(),
                levels: vec![3],
                factorial: CalculatedFactorial::Exact(Integer::from(280)),
            }],
            author: "test_author".to_string(),
            subreddit: "test_subreddit".to_string(),
            status: Status::FACTORIALS_FOUND,
            commands: Default::default(),
        };

        let reply = comment.get_reply();
        assert_eq!(reply, "Triple-factorial of 10 is 280 \n\n\n*^(This action was performed by a bot. Please DM me if you have any questions.)*");
    }

    #[test]
    fn test_get_reply_for_subfactorial() {
        let comment = RedditComment {
            id: "123".to_string(),
            factorial_list: vec![Factorial {
                number: 5.into(),
                levels: vec![-1],
                factorial: CalculatedFactorial::Exact(Integer::from(44)),
            }],
            author: "test_author".to_string(),
            subreddit: "test_subreddit".to_string(),
            status: Status::FACTORIALS_FOUND,
            commands: Default::default(),
        };

        let reply = comment.get_reply();
        assert_eq!(reply, "Subfactorial of 5 is 44 \n\n\n*^(This action was performed by a bot. Please DM me if you have any questions.)*");
    }
    #[test]
    fn test_get_reply_for_big_subfactorial() {
        let comment = RedditComment {
            id: "123".to_string(),
            factorial_list: vec![Factorial {
                number: 5000.into(),
                levels: vec![-1],
                factorial: CalculatedFactorial::Exact(math::subfactorial(5000)),
            }],
            author: "test_author".to_string(),
            subreddit: "test_subreddit".to_string(),
            status: Status::FACTORIALS_FOUND,
            commands: Default::default(),
        };

        let reply = comment.get_reply();
        assert_eq!(reply, "If I post the whole number, the comment would get too long, as reddit only allows up to 10k characters. So I had to turn it into scientific notation.\n\nSubfactorial of 5000 is roughly 1.555606884589543595233339289773 × 10^16325 \n\n\n*^(This action was performed by a bot. Please DM me if you have any questions.)*");
    }

    #[test]
    fn test_get_reply_for_high_multifactorial() {
        let comment = RedditComment {
            id: "123".to_string(),
            factorial_list: vec![Factorial {
                number: 10.into(),
                levels: vec![46],
                factorial: CalculatedFactorial::Exact(Integer::from(10)),
            }],
            author: "test_author".to_string(),
            subreddit: "test_subreddit".to_string(),
            status: Status::FACTORIALS_FOUND,
            commands: Default::default(),
        };

        let reply = comment.get_reply();
        assert_eq!(reply, "46-factorial of 10 is 10 \n\n\n*^(This action was performed by a bot. Please DM me if you have any questions.)*");
    }

    #[test]
    fn test_get_reply_for_multiple() {
        let comment = RedditComment {
            id: "123".to_string(),
            factorial_list: vec![
                Factorial {
                    number: 5.into(),
                    levels: vec![1],
                    factorial: CalculatedFactorial::Exact(Integer::from(120)),
                },
                Factorial {
                    number: 6.into(),
                    levels: vec![1],
                    factorial: CalculatedFactorial::Exact(Integer::from(720)),
                },
            ],
            author: "test_author".to_string(),
            subreddit: "test_subreddit".to_string(),
            status: Status::FACTORIALS_FOUND,
            commands: Default::default(),
        };

        let reply = comment.get_reply();
        assert_eq!(reply, "The factorial of 5 is 120 \n\nThe factorial of 6 is 720 \n\n\n*^(This action was performed by a bot. Please DM me if you have any questions.)*");
    }

    #[test]
    fn test_get_reply_too_long_with_multiple_numbers() {
        let comment = RedditComment {
            id: "123".to_string(),
            factorial_list: vec![
                Factorial {
                    number: 5.into(),
                    levels: vec![2],
                    factorial: CalculatedFactorial::Exact(Integer::from(60)),
                },
                Factorial {
                    number: 6.into(),
                    levels: vec![1],
                    factorial: CalculatedFactorial::Exact(Integer::from(720)),
                },
                Factorial {
                    number: 3249.into(),
                    levels: vec![1],
                    factorial: CalculatedFactorial::Exact(math::factorial(3249, 1)),
                },
            ],
            author: "test_author".to_string(),
            subreddit: "test_subreddit".to_string(),
            status: Status::FACTORIALS_FOUND | Status::REPLY_WOULD_BE_TOO_LONG,
            commands: Default::default(),
        };

        let reply = comment.get_reply();
        assert_eq!(reply, "If I post the whole numbers, the comment would get too long, as reddit only allows up to 10k characters. So I had to turn them into scientific notation.\n\nDouble-factorial of 5 is 60 \n\nThe factorial of 6 is 720 \n\nThe factorial of 3249 is roughly 6.412337688276552183884096303057 × 10^10000 \n\n\n*^(This action was performed by a bot. Please DM me if you have any questions.)*");
    }

    #[test]
    fn test_get_reply_too_long_from_new_comment() {
        let comment = RedditComment::new(
            "This is a test comment with a factorial of 4000!",
            "1234",
            "test_author",
            "test_subreddit",
        );

        let reply = comment.get_reply();
        assert_eq!(reply, "If I post the whole number, the comment would get too long, as reddit only allows up to 10k characters. So I had to turn it into scientific notation.\n\nThe factorial of 4000 is roughly 1.828801951514065013314743175574 × 10^12673 \n\n\n*^(This action was performed by a bot. Please DM me if you have any questions.)*");
    }

    #[test]
    fn test_get_reply_too_long_from_new_comment_for_multifactorial() {
        let comment = RedditComment::new(
            "This is a test comment with a factorial of 9000!!!",
            "1234",
            "test_author",
            "test_subreddit",
        );

        let reply = comment.get_reply();
        assert_eq!(reply, "If I post the whole number, the comment would get too long, as reddit only allows up to 10k characters. So I had to turn it into scientific notation.\n\nTriple-factorial of 9000 is roughly 9.588379914654826764034139164855 × 10^10561 \n\n\n*^(This action was performed by a bot. Please DM me if you have any questions.)*");
    }

    #[test]
    fn test_get_reply_too_long_from_number_3250() {
        let comment = RedditComment::new(
            "This is a test comment with a factorial of 3250!",
            "1234",
            "test_author",
            "test_subreddit",
        );

        let reply = comment.get_reply();
        assert_eq!(reply, "If I post the whole number, the comment would get too long, as reddit only allows up to 10k characters. So I had to turn it into scientific notation.\n\nThe factorial of 3250 is roughly 2.084009748689879459762331298493 × 10^10004 \n\n\n*^(This action was performed by a bot. Please DM me if you have any questions.)*");
    }

    #[test]
    fn test_get_reply_approximate_from_new_comment() {
        let comment = RedditComment::new(
            "This is a test comment with a factorial of 1489232!",
            "1234",
            "test_author",
            "test_subreddit",
        );

        let reply = comment.get_reply();
        assert_eq!(reply, "Sorry, that is so large, that I can't calculate it, so I'll have to approximate.\n\nThe factorial of 1489232 is approximately 2.120259616630154 × 10^8546211 \n\n\n*^(This action was performed by a bot. Please DM me if you have any questions.)*");
    }

    #[test]
    fn test_get_reply_approximate_from_number_1000002() {
        let comment = RedditComment::new(
            "This is a test comment with a factorial of 1000002!",
            "1234",
            "test_author",
            "test_subreddit",
        );

        let reply = comment.get_reply();
        assert_eq!(reply, "Sorry, that is so large, that I can't calculate it, so I'll have to approximate.\n\nThe factorial of 1000002 is approximately 8.263956480142832 × 10^5565720 \n\n\n*^(This action was performed by a bot. Please DM me if you have any questions.)*");
    }

    #[test]
    fn test_get_reply_approximate_digits_from_new_comment() {
        let comment = RedditComment::new(
            "This is a test comment with a factorial of 67839127837442873498364307437846329874293874384739847347394748012940124093748389701473461687364012630527560276507263724678234685360158032147349867349837403928573587255865587234672880756378340253167320767378467507576450878320574087430274607215697523720397460949849834384772847384738474837484774639847374!",
            "1234",
            "test_author",
            "test_subreddit",
        );

        let reply = comment.get_reply();
        assert_eq!(reply, "That number is so large, that I can't even approximate it well, so I can only give you an approximation on the number of digits.\n\nThe factorial of 67839127837442873498364307437846329874293874384739847347394748012940124093748389701473461687364012630527560276507263724678234685360158032147349867349837403928573587255865587234672880756378340253167320767378467507576450878320574087430274607215697523720397460949849834384772847384738474837484774639847374 has approximately 20446522215564236275041062436291735585615770688497033688635992348006569652526624848770315740147437774149118209115411567314791976403856295878031859754864941032834352021489210979065405760855940731542907166075497068156426030767735126902058810271396007949529366379073139457637180014292606643575007577178264993 digits \n\n\n*^(This action was performed by a bot. Please DM me if you have any questions.)*");
    }

    #[test]
    fn test_get_reply_approximate_digits_from_multifactorial() {
        let comment = RedditComment::new(
            "This is a test comment with a multi-factorial of 8394763!!!!",
            "1234",
            "test_author",
            "test_subreddit",
        );

        let reply = comment.get_reply();
        assert_eq!(reply, "That number is so large, that I can't even approximate it well, so I can only give you an approximation on the number of digits.\n\nQuadruple-factorial of 8394763 has approximately 13619907 digits \n\n\n*^(This action was performed by a bot. Please DM me if you have any questions.)*");
    }

    #[test]
    fn test_get_reply_approximate_digits_from_huge() {
        let comment = RedditComment::new(
            "This is a test comment with a factorial of 10000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000!",
            "1234",
            "test_author",
            "test_subreddit",
        );

        let reply = comment.get_reply();
        assert_eq!(reply, "That number is so large, that I can't even approximate it well, so I can only give you an approximation on the number of digits.\n\nThe factorial of 10000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000 has approximately 3005657055180967481723488710810833949177056029941963334338855462168341353507911292252707750506615682516812938932552336962663583207128410360934307789353371877341478729134313296704066291303411733116688363922615094857155651333231353413914864438517876512346564565642682746164377718604396951353347633904460774 digits \n\n\n*^(This action was performed by a bot. Please DM me if you have any questions.)*");
    }

    #[test]
    fn test_get_reply_factorial_chain() {
        let comment = RedditComment::new(
            "This is a test with a factorial chain 5! (((5!)!)!)!",
            "1234",
            "test_author",
            "test_subreddit",
        );

        let reply = comment.get_reply();
        assert_eq!(reply, "Sorry, some of those are so large, that I can't calculate them, so I'll have to approximate.\n\nThe factorial of 5 is 120 \n\nThe factorial of The factorial of The factorial of 5 is approximately 1.9172992008293117 × 10^1327137837206659786031747299606377028838214110127983264121956821748182259183419110243647989875487282380340365022219190769273781621333865377166444878565902856196867372963998070875391932298781352992969733 \n\n\n*^(This action was performed by a bot. Please DM me if you have any questions.)*");
    }
    #[test]
    fn test_get_reply_factorial_chain_from_approximate() {
        let comment = RedditComment::new(
            "This is a test with a factorial chain (20000000!)!",
            "1234",
            "test_author",
            "test_subreddit",
        );

        let reply = comment.get_reply();
        assert_eq!(reply, "That number is so large, that I can't even approximate it well, so I can only give you an approximation on the number of digits.\n\nThe factorial of The factorial of 20000000 has approximately 2.901348168358672858923433671149 × 10^137334722 digits \n\n\n*^(This action was performed by a bot. Please DM me if you have any questions.)*");
    }

    #[test]
    fn test_get_reply_approximate_digits_from_mixed_types() {
        let comment = RedditComment {
            id: "1234".to_string(),
            factorial_list: vec![
                Factorial {
                    number: 8.into(),
                    levels: vec![2],
                    factorial: CalculatedFactorial::Exact(Integer::from(384)),
                },
                Factorial {
                    number: 10000.into(),
                    levels: vec![1],
                    factorial: CalculatedFactorial::Exact(math::factorial(10000, 1)),
                },
                Factorial {
                    number: 37923648.into(),
                    levels: vec![1],
                    factorial: {
                        let (base, exponent) = math::approximate_factorial(37923648.into());
                        CalculatedFactorial::Approximate(base, exponent)
                    },
                },
                Factorial {
                    number: 283462.into(),
                    levels: vec![2],
                    factorial: CalculatedFactorial::ApproximateDigits(
                        math::approximate_multifactorial_digits(283462.into(), 2),
                    ),
                },
            ],
            author: "test_author".to_string(),
            subreddit: "test_subreddit".to_string(),
            status: Status::REPLY_WOULD_BE_TOO_LONG,
            commands: Default::default(),
        };

        let reply = comment.get_reply();
        assert_eq!(reply, "Some of these are so large, that I can't even approximate them well, so I can only give you an approximation on the number of digits.\n\nDouble-factorial of 8 is 384 \n\nThe factorial of 10000 is roughly 2.84625968091705451890641321212 × 10^35659 \n\nThe factorial of 37923648 is approximately 1.760585629143694 × 10^270949892 \n\nDouble-factorial of 283462 has approximately 711238 digits \n\n\n*^(This action was performed by a bot. Please DM me if you have any questions.)*");
    }
}
