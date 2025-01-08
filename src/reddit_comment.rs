use crate::math;
use fancy_regex::Regex;
use num_bigint::BigInt;
use num_traits::{One, ToPrimitive};
use std::fmt::Write;

// Limit for exact calculation, set to limit calculation time
pub(crate) const UPPER_CALCULATION_LIMIT: u64 = 100_001;
// Limit for approximation, set to ensure enough accuracy (aftewards, only single correct decimals have been observed)
pub(crate) const UPPER_APPROXIMATION_LIMIT: u64 = 500_000_000_000;
// Limit for number of digits approximation, set to prevent panics, due to the result overflowing u128
pub(crate) const UPPER_DIGIT_APPROXIMATION_LIMIT: u128 =
    1_000_000_000_000_000_000_000_000_000_000_000_000;
const PLACEHOLDER: &str = "Factorial of ";
const FOOTER_TEXT: &str =
    "\n*^(This action was performed by a bot. Please DM me if you have any questions.)*";
pub(crate) const MAX_COMMENT_LENGTH: i64 = 10_000 - 10 - FOOTER_TEXT.len() as i64;
pub(crate) const NUMBER_DECIMALS_SCIENTIFIC: usize = 100;

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum CalculatedFactorial {
    Exact(BigInt),
    Approximate(f64, u64),
    ApproximateDigits(u128),
}

#[derive(Debug, Clone, PartialEq, Ord, Eq, Hash, PartialOrd)]
pub(crate) struct Factorial {
    pub(crate) number: u128,
    pub(crate) level: u64,
    pub(crate) factorial: CalculatedFactorial,
}

#[derive(Debug)]
pub(crate) struct RedditComment {
    pub(crate) id: String,
    pub(crate) factorial_list: Vec<Factorial>,
    pub(crate) author: String,
    pub(crate) subreddit: String,
    pub(crate) status: Vec<Status>,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub(crate) enum Status {
    AlreadyReplied,
    NotReplied,
    NumberTooBig,
    NoFactorial,
    ReplyWouldBeTooLong,
    FactorialsFound,
    #[allow(dead_code)]
    DecimalFactorial,
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
    fn format(&self, acc: &mut String, force_shorten: bool) -> Result<(), std::fmt::Error> {
        let factorial_level_string = RedditComment::get_factorial_level_string(self.level);
        match &self.factorial {
            CalculatedFactorial::Exact(factorial) => {
                if self.is_too_long() || force_shorten {
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
                    write!(
                        acc,
                        "{}{}{} is roughly {}e{} \n\n",
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
                    RedditComment::get_factorial_level_string(self.level),
                    PLACEHOLDER,
                    self.number,
                    digits
                )
            }
        }
    }
    fn is_aproximate_digits(&self) -> bool {
        matches!(self.factorial, CalculatedFactorial::ApproximateDigits(_))
    }
    fn is_approximate(&self) -> bool {
        matches!(self.factorial, CalculatedFactorial::Approximate(_, _))
    }
    fn is_too_long(&self) -> bool {
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
}

impl RedditComment {
    pub(crate) fn new(body: &str, id: &str, author: &str, subreddit: &str) -> Self {
        let factorial_regex =
            Regex::new(r"(?<![,.!?\d])\b(\d+)(!+)(?![<\d]|&lt;)").expect("Invalid factorial regex");
        let mut factorial_list: Vec<Factorial> = Vec::new();
        let mut status: Vec<Status> = vec![];

        for regex_capture in factorial_regex.captures_iter(body) {
            let regex_capture = regex_capture.expect("Failed to capture regex");

            let num = regex_capture[1]
                .parse::<BigInt>()
                .expect("Failed to parse number");

            let exclamation_count = regex_capture[2]
                .len()
                .to_u64()
                .expect("Failed to convert exclamation count to u64");
            // Check if we can approximate the number of digits
            if num > BigInt::from(UPPER_DIGIT_APPROXIMATION_LIMIT) {
                status.push(Status::NumberTooBig)
                // Check if we can approximate it
            } else if num > BigInt::from(UPPER_APPROXIMATION_LIMIT)
                || (exclamation_count > 1 && num > BigInt::from(UPPER_CALCULATION_LIMIT))
            {
                let num = num.to_u128().expect("Failed to convert BigInt to i64");
                let factorial = math::approximate_multifactorial_digits(num, exclamation_count);
                factorial_list.push(Factorial {
                    number: num,
                    level: exclamation_count,
                    factorial: CalculatedFactorial::ApproximateDigits(factorial),
                });
            // Check if the number is within a reasonable range to compute
            } else if num > BigInt::from(UPPER_CALCULATION_LIMIT) {
                let num = num.to_u64().expect("Failed to convert BigInt to i64");
                let factorial = math::approximate_factorial(num);
                factorial_list.push(Factorial {
                    number: num as u128,
                    level: exclamation_count,
                    factorial: CalculatedFactorial::Approximate(factorial.0, factorial.1),
                });
            } else if num == BigInt::one() {
                continue;
            } else {
                let num = num.to_u64().expect("Failed to convert BigInt to i64");
                let factorial = math::factorial(num, exclamation_count);
                factorial_list.push(Factorial {
                    number: num as u128,
                    level: exclamation_count,
                    factorial: CalculatedFactorial::Exact(factorial),
                });
            }
        }

        factorial_list.sort();
        factorial_list.dedup();

        if factorial_list.is_empty() {
            status.push(Status::NoFactorial);
        } else {
            status.push(Status::FactorialsFound);
        }

        // rewrite for Factorial struct

        RedditComment {
            id: id.to_string(),
            author: author.to_string(),
            subreddit: subreddit.to_string(),
            factorial_list,
            status,
        }
    }

    fn get_factorial_level_string(level: u64) -> &'static str {
        match level {
            1 => "",
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
            _ => "n-",
        }
    }

    pub(crate) fn add_status(&mut self, status: Status) {
        self.status.push(status);
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
                let _ = factorial.format(&mut acc, false);
                acc
            });

        // If the reply was too long try force shortening all factorials
        if reply.len() > MAX_COMMENT_LENGTH as usize {
            if note.is_empty() {
                let _ = note.write_str("If I post the whole numbers, the comment would get too long, as reddit only allows up to 10k characters. So I had to turn them into scientific notation.\n\n");
            };
            reply = self.factorial_list.iter().fold(note, |mut acc, factorial| {
                let _ = factorial.format(&mut acc, true);
                acc
            });
        }
        if reply.len() > MAX_COMMENT_LENGTH as usize {
            reply = "Sorry, but the reply text for all those number would be _really_ long, so I'd rather not even try posting lmao\n".to_string();
        }

        reply.push_str(FOOTER_TEXT);
        reply
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use num_bigint::ToBigInt;

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
                    number: 5,
                    level: 1,
                    factorial: CalculatedFactorial::Exact(120.to_bigint().unwrap()),
                },
                Factorial {
                    number: 6,
                    level: 1,
                    factorial: CalculatedFactorial::Exact(720.to_bigint().unwrap()),
                },
            ],
        );
        assert_eq!(comment.status, vec![Status::FactorialsFound]);
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
                number: 6,
                level: 2,
                factorial: CalculatedFactorial::Exact(48.to_bigint().unwrap()),
            }]
        );
        assert_eq!(comment.status, vec![Status::FactorialsFound]);
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
                number: 6,
                level: 3,
                factorial: CalculatedFactorial::Exact(18.to_bigint().unwrap()),
            }]
        );
        assert_eq!(comment.status, vec![Status::FactorialsFound]);
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
        assert_eq!(comment.status, vec![Status::NoFactorial]);
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
        assert_eq!(comment.status, vec![Status::NoFactorial]);
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
        assert_eq!(comment.status, vec![Status::NoFactorial]);
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
        assert_eq!(comment.status, vec![Status::NoFactorial]);
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
        assert_eq!(comment.status, vec![Status::NoFactorial]);
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
        assert_eq!(comment.status, vec![Status::NoFactorial]);
    }

    #[test]
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
                number: 6,
                level: 1,
                factorial: CalculatedFactorial::Exact(720.to_bigint().unwrap())
            }]
        );
        assert_eq!(
            comment.status,
            vec![Status::NumberTooBig, Status::FactorialsFound]
        );
    }

    #[test]
    fn test_comment_new_very_big_number() {
        let very_big_number = "9".repeat(10_000) + "!";
        let comment = RedditComment::new(&very_big_number, "123", "test_author", "test_subreddit");
        assert_eq!(comment.id, "123");
        assert_eq!(comment.factorial_list, vec![]);
        assert_eq!(
            comment.status,
            vec![Status::NumberTooBig, Status::NoFactorial]
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
        comment.add_status(Status::NotReplied);
        assert_eq!(
            comment.status,
            vec![Status::FactorialsFound, Status::NotReplied]
        );
    }

    #[test]
    fn test_reply_text_too_long() {
        let comment = RedditComment::new(
            "3500! 3501! 3502! 3503! 3504! 3505! 3506! 3507! 3508! 3509! 3510! 3511! 3512! 3513! 3514! 3515! 3516! 3517! 3518! 3519! 3520! 3521! 3522! 3523! 3524! 3525! 3526! 3527! 3528! 3529! 3530! 3531! 3532! 3533! 3534! 3535! 3536! 3537! 3538! 3539! 3540! 3541! 3542! 3543! 3544! 3545! 3546! 3547! 3548! 3549! 3550! 3551! 3552! 3553! 3554! 3555! 3556! 3557! 3558! 3559! 3560! 3561! 3562! 3563! 3564! 3565! 3566! 3567! 3568! 3569! 3570! 3571! 3572! 3573! 3574! 3575! 3576! 3577! 3578! 3579! 3580! 3581! 3582! 3583! 3584! 3585! 3586! 3587! 3588! 3589! 3590! 3591! 3592! 3593! 3594! 3595! 3596! 3597! 3598! 3599! 3600!",
            "123",
            "test_author",
            "test_subreddit"
        );
        let reply = comment.get_reply();
        assert_eq!(
            reply,
            // over 13k characters
            "Sorry, but the reply text for all those number would be _really_ long, so I'd rather not even try posting lmao\n\n*^(This action was performed by a bot. Please DM me if you have any questions.)*"
        );
    }

    #[test]
    fn test_get_reply_for_multifactorial() {
        let comment = RedditComment {
            id: "123".to_string(),
            factorial_list: vec![Factorial {
                number: 10,
                level: 3,
                factorial: CalculatedFactorial::Exact(280.to_bigint().unwrap()),
            }],
            author: "test_author".to_string(),
            subreddit: "test_subreddit".to_string(),
            status: vec![Status::FactorialsFound],
        };

        let reply = comment.get_reply();
        assert_eq!(reply, "Triple-Factorial of 10 is 280 \n\n\n*^(This action was performed by a bot. Please DM me if you have any questions.)*");
    }

    #[test]
    fn test_get_reply_for_multiple() {
        let comment = RedditComment {
            id: "123".to_string(),
            factorial_list: vec![
                Factorial {
                    number: 5,
                    level: 1,
                    factorial: CalculatedFactorial::Exact(120.to_bigint().unwrap()),
                },
                Factorial {
                    number: 6,
                    level: 1,
                    factorial: CalculatedFactorial::Exact(720.to_bigint().unwrap()),
                },
            ],
            author: "test_author".to_string(),
            subreddit: "test_subreddit".to_string(),
            status: vec![Status::FactorialsFound],
        };

        let reply = comment.get_reply();
        assert_eq!(reply, "Factorial of 5 is 120 \n\nFactorial of 6 is 720 \n\n\n*^(This action was performed by a bot. Please DM me if you have any questions.)*");
    }

    #[test]
    fn test_get_reply_too_long_with_multiple_numbers() {
        let comment = RedditComment {
            id: "123".to_string(),
            factorial_list: vec![
                Factorial {
                    number: 5,
                    level: 2,
                    factorial: CalculatedFactorial::Exact(60.to_bigint().unwrap()),
                },
                Factorial {
                    number: 6,
                    level: 1,
                    factorial: CalculatedFactorial::Exact(720.to_bigint().unwrap()),
                },
                Factorial {
                    number: 3249,
                    level: 1,
                    factorial: CalculatedFactorial::Exact(math::factorial(3249, 1)),
                },
            ],
            author: "test_author".to_string(),
            subreddit: "test_subreddit".to_string(),
            status: vec![Status::FactorialsFound, Status::ReplyWouldBeTooLong],
        };

        let reply = comment.get_reply();
        assert_eq!(reply, "If I post the whole numbers, the comment would get too long, as reddit only allows up to 10k characters. So I had to turn them into scientific notation.\n\nDouble-Factorial of 5 is roughly 6.0e1 \n\nFactorial of 6 is roughly 7.20e2 \n\nFactorial of 3249 is roughly 6.4123376882765521838840963030568127691878727205333658692200854486404915724268122521695176119279253636e10000 \n\n\n*^(This action was performed by a bot. Please DM me if you have any questions.)*");
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
        assert_eq!(reply, "If I post the whole number, the comment would get too long, as reddit only allows up to 10k characters. So I had to turn it into scientific notation.\n\nFactorial of 4000 is roughly 1.8288019515140650133147431755739190442173777107304392197064526954208959797973177364850370286870484107e12673 \n\n\n*^(This action was performed by a bot. Please DM me if you have any questions.)*");
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
        assert_eq!(reply, "If I post the whole number, the comment would get too long, as reddit only allows up to 10k characters. So I had to turn it into scientific notation.\n\nTriple-Factorial of 9000 is roughly 9.5883799146548267640341391648545903348878025438772769707015576436531779580675303393957674423348854753e10561 \n\n\n*^(This action was performed by a bot. Please DM me if you have any questions.)*");
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
        assert_eq!(reply, "If I post the whole number, the comment would get too long, as reddit only allows up to 10k characters. So I had to turn it into scientific notation.\n\nFactorial of 3250 is roughly 2.0840097486898794597623312984934641499860586341733439074965277708081597610387139819550932238765757432e10004 \n\n\n*^(This action was performed by a bot. Please DM me if you have any questions.)*");
    }

    #[test]
    fn test_get_reply_approximate_from_new_comment() {
        let comment = RedditComment::new(
            "This is a test comment with a factorial of 489232!",
            "1234",
            "test_author",
            "test_subreddit",
        );

        let reply = comment.get_reply();
        assert_eq!(reply, "Sorry, that is so large, that I can't calculate it, so I'll have to approximate.\n\nFactorial of 489232 is approximately 1.6514332659579547e2571025 \n\n\n*^(This action was performed by a bot. Please DM me if you have any questions.)*");
    }

    #[test]
    fn test_get_reply_approximate_from_number_100002() {
        let comment = RedditComment::new(
            "This is a test comment with a factorial of 100002!",
            "1234",
            "test_author",
            "test_subreddit",
        );

        let reply = comment.get_reply();
        assert_eq!(reply, "Sorry, that is so large, that I can't calculate it, so I'll have to approximate.\n\nFactorial of 100002 is approximately 2.8243141352927332e456583 \n\n\n*^(This action was performed by a bot. Please DM me if you have any questions.)*");
    }

    #[test]
    fn test_get_reply_approximate_digits_from_new_comment() {
        let comment = RedditComment::new(
            "This is a test comment with a factorial of 67839127837442!",
            "1234",
            "test_author",
            "test_subreddit",
        );

        let reply = comment.get_reply();
        assert_eq!(reply, "That number is so large, that I can't even approximate it well, so I can only give you an approximation on the number of digits.\n\nFactorial of 67839127837442 has approximately 908853398380684 digits \n\n\n*^(This action was performed by a bot. Please DM me if you have any questions.)*");
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
        assert_eq!(reply, "That number is so large, that I can't even approximate it well, so I can only give you an approximation on the number of digits.\n\nQuadruple-Factorial of 8394763 has approximately 13619907 digits \n\n\n*^(This action was performed by a bot. Please DM me if you have any questions.)*");
    }

    #[test]
    fn test_get_reply_approximate_digits_from_huge() {
        let comment = RedditComment::new(
            "This is a test comment with a factorial of 1000000000000000000000000000000000000!",
            "1234",
            "test_autho",
            "test_subreddit",
        );

        let reply = comment.get_reply();
        assert_eq!(reply, "That number is so large, that I can't even approximate it well, so I can only give you an approximation on the number of digits.\n\nFactorial of 1000000000000000000000000000000000000 has approximately 35565705518096741787712172651953782785 digits \n\n\n*^(This action was performed by a bot. Please DM me if you have any questions.)*");
    }

    #[test]
    fn test_get_reply_approximate_digits_from_mixed_types() {
        let comment = RedditComment {
            id: "1234".to_string(),
            factorial_list: vec![
                Factorial {
                    number: 8,
                    level: 2,
                    factorial: CalculatedFactorial::Exact(BigInt::from(384)),
                },
                Factorial {
                    number: 10000,
                    level: 1,
                    factorial: CalculatedFactorial::Exact(math::factorial(10000, 1)),
                },
                Factorial {
                    number: 37923648,
                    level: 1,
                    factorial: {
                        let (base, exponent) = math::approximate_factorial(37923648);
                        CalculatedFactorial::Approximate(base, exponent)
                    },
                },
                Factorial {
                    number: 283462,
                    level: 2,
                    factorial: CalculatedFactorial::ApproximateDigits(
                        math::approximate_multifactorial_digits(283462, 2),
                    ),
                },
            ],
            author: "test_author".to_string(),
            subreddit: "test_subreddit".to_string(),
            status: vec![Status::ReplyWouldBeTooLong],
        };

        let reply = comment.get_reply();
        assert_eq!(reply, "Some of these are so large, that I can't even approximate them well, so I can only give you an approximation on the number of digits.\n\nDouble-Factorial of 8 is 384 \n\nFactorial of 10000 is roughly 2.8462596809170545189064132121198688901480514017027992307941799942744113400037644437729907867577847758e35659 \n\nFactorial of 37923648 is approximately 1.7605854240375498e270949892 \n\nDouble-Factorial of 283462 has approximately 711238 digits \n\n\n*^(This action was performed by a bot. Please DM me if you have any questions.)*");
    }
}
