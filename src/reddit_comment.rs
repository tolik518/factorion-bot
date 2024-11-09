use crate::math;
use fancy_regex::Regex;
use num_bigint::BigInt;
use num_traits::{One, ToPrimitive};

pub(crate) const UPPER_CALCULATION_LIMIT: i64 = 100_001;
const PLACEHOLDER: &str = "Factorial of ";
const FOOTER_TEXT: &str = "\n*^(This action was performed by a bot. Please contact u/tolik518 if you have any questions or concerns.)*";
pub(crate) const MAX_COMMENT_LENGTH: i64 = 10_000 - 10 - FOOTER_TEXT.len() as i64;

#[derive(Debug, Clone, PartialEq, Ord, Eq, Hash, PartialOrd)]
pub(crate) struct Factorial {
    pub(crate) number: i64,
    pub(crate) level: i64,
    pub(crate) factorial: BigInt,
}

pub(crate) struct RedditComment {
    pub(crate) id: String,
    pub(crate) factorial_list: Vec<Factorial>,
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

impl RedditComment {
    pub(crate) fn new(body: &str, id: &str) -> Self {
        let factorial_regex =
            Regex::new(r"(?<![.!?\d])\b(\d+)(!+)(?![<\d])").expect("Invalid factorial regex");
        let mut factorial_list: Vec<Factorial> = Vec::new();
        let mut status: Vec<Status> = vec![];

        for regex_capture in factorial_regex.captures_iter(body) {
            let regex_capture = regex_capture.expect("Failed to capture regex");

            let num = regex_capture[1]
                .parse::<BigInt>()
                .expect("Failed to parse number");

            let exclamation_count = regex_capture[2].len();

            // Check if the number is within a reasonable range to compute
            if num > BigInt::from(UPPER_CALCULATION_LIMIT) {
                status.push(Status::NumberTooBig);
            } else if num == BigInt::one() {
                continue;
            } else {
                let num = num.to_i64().expect("Failed to convert BigInt to i64");
                let factorial = math::factorial(num, exclamation_count as i64);
                factorial_list.push(Factorial {
                    number: num,
                    level: exclamation_count as i64,
                    factorial,
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
        if RedditComment::factorials_are_too_long(&factorial_list) {
            status.push(Status::ReplyWouldBeTooLong);
        }

        RedditComment {
            id: id.to_string(),
            factorial_list,
            status,
        }
    }

    fn factorials_are_too_long(factorial_list: &[Factorial]) -> bool {
        factorial_list
            .iter()
            .any(|Factorial { number, level, .. }| {
                *level == 1 && *number > 3249
                    || *level == 2 && *number > 5982
                    || *level == 3 && *number > 8572
                    || *level == 4 && *number > 11077
                    || *level == 5 && *number > 13522
                    || *level == 6 && *number > 15920
                    || *level == 7 && *number > 18282
                    || *level == 8 && *number > 20613
                    || *level == 9 && *number > 22920
                    || *level == 10 && *number > 25208
                    || *level == 11 && *number > 27479
                    || *level == 12 && *number > 29735
                    || *level == 13 && *number > 31977
                    || *level == 14 && *number > 34207
                    || *level == 15 && *number > 36426
                    || *level == 16 && *number > 38635
                    || *level == 17 && *number > 40835
                    || *level == 18 && *number > 43027
                    || *level == 19 && *number > 45212
                    || *level == 20 && *number > 47390
            })
    }

    pub(crate) fn add_status(&mut self, status: Status) {
        self.status.push(status);
    }

    pub(crate) fn get_reply(&self) -> String {
        let mut reply = String::new();
        if self.status.contains(&Status::ReplyWouldBeTooLong) {
            let mut numbers: Vec<i64> = Vec::new();
            for (Factorial { number, .. }) in self.factorial_list.iter() {
                numbers.push(*number);
            }
            reply.push_str(&format!("Sorry bro, but if I calculate the factorial(s) of the number(s) {:?}, the reply would be too long for reddit :(\n\n", numbers));
        } else {
            for (Factorial {
                number,
                level,
                factorial,
            }) in self.factorial_list.iter()
            {
                let factorial_level_string = match level {
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
                    46 => "Sexquadragintuple-",
                    47 => "Septenquadragintuple-",
                    _ => "n-",
                };

                reply.push_str(&format!(
                    "{}{}{} is {} \n\n",
                    factorial_level_string, PLACEHOLDER, number, factorial
                ));
            }
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
        );
        assert_eq!(comment.id, "123");
        assert_eq!(
            comment.factorial_list,
            vec![
                Factorial {
                    number: 5,
                    level: 1,
                    factorial: 120.to_bigint().unwrap(),
                },
                Factorial {
                    number: 6,
                    level: 1,
                    factorial: 720.to_bigint().unwrap(),
                },
            ],
        );
        assert_eq!(comment.status, vec![Status::FactorialsFound]);
    }

    #[test]
    fn test_comment_new_double_factorial() {
        let comment = RedditComment::new("This is a test comment with an n-factorial 6!!", "123");
        assert_eq!(
            comment.factorial_list,
            vec![Factorial {
                number: 6,
                level: 2,
                factorial: 48.to_bigint().unwrap(),
            }]
        );
        assert_eq!(comment.status, vec![Status::FactorialsFound]);
    }

    #[test]
    fn test_comment_new_triple_factorial() {
        let comment = RedditComment::new("This is a test comment with an n-factorial 6!!!", "123");
        assert_eq!(
            comment.factorial_list,
            vec![Factorial {
                number: 6,
                level: 3,
                factorial: 18.to_bigint().unwrap(),
            }]
        );
        assert_eq!(comment.status, vec![Status::FactorialsFound]);
    }

    #[test]
    fn test_comment_new_spoiler() {
        let comment = RedditComment::new(">!This is a spoiler comment 5!<", "123");
        assert_eq!(comment.factorial_list, vec![]);
        assert_eq!(comment.status, vec![Status::NoFactorial]);
    }

    #[test]
    fn test_comment_new_exclamations_one() {
        let comment = RedditComment::new("This is a test with exclamation mark stuff!!!1!", "123");
        assert_eq!(comment.factorial_list, vec![]);
        assert_eq!(comment.status, vec![Status::NoFactorial]);
    }

    #[test]
    fn test_comment_new_exclamations_eleven() {
        let comment = RedditComment::new("This is a test with exclamation mark stuff!!!11!", "123");
        assert_eq!(comment.factorial_list, vec![]);
        assert_eq!(comment.status, vec![Status::NoFactorial]);
    }

    #[test]
    #[ignore]
    fn test_comment_new_decimals() {
        let comment = RedditComment::new("This is a test comment with decimal number 0.5!", "123");
        assert_eq!(comment.factorial_list, vec![]);
        assert_eq!(
            comment.status,
            vec![Status::DecimalFactorial, Status::FactorialsFound]
        );
    }

    #[test]
    fn test_comment_new_big_number_and_normal_number() {
        let comment = RedditComment::new(
            "This is a test comment with a factorial of 555555555555555555555555555555555! and 6!",
            "123",
        );
        assert_eq!(comment.id, "123");
        assert_eq!(
            comment.factorial_list,
            vec![Factorial {
                number: 6,
                level: 1,
                factorial: 720.to_bigint().unwrap()
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
        let comment = RedditComment::new(&very_big_number, "123");
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
        );
        comment.add_status(Status::NotReplied);
        assert_eq!(
            comment.status,
            vec![Status::FactorialsFound, Status::NotReplied]
        );
    }

    #[test]
    fn test_get_reply_for_multifactorial() {
        let comment = RedditComment {
            id: "123".to_string(),
            factorial_list: vec![Factorial {
                number: 10,
                level: 3,
                factorial: 280.to_bigint().unwrap(),
            }],
            status: vec![Status::FactorialsFound],
        };

        let reply = comment.get_reply();
        assert_eq!(reply, "Triple-Factorial of 10 is 280 \n\n\n*^(This action was performed by a bot. Please contact u/tolik518 if you have any questions or concerns.)*");
    }

    #[test]
    fn test_get_reply_for_multiple() {
        let comment = RedditComment {
            id: "123".to_string(),
            factorial_list: vec![
                Factorial {
                    number: 5,
                    level: 1,
                    factorial: 120.to_bigint().unwrap(),
                },
                Factorial {
                    number: 6,
                    level: 1,
                    factorial: 720.to_bigint().unwrap(),
                },
            ],
            status: vec![Status::FactorialsFound],
        };

        let reply = comment.get_reply();
        assert_eq!(reply, "Factorial of 5 is 120 \n\nFactorial of 6 is 720 \n\n\n*^(This action was performed by a bot. Please contact u/tolik518 if you have any questions or concerns.)*");
    }

    #[test]
    fn test_get_reply_too_long() {
        let comment = RedditComment {
            id: "123".to_string(),
            factorial_list: vec![
                Factorial {
                    number: 5,
                    level: 1,
                    factorial: 120.to_bigint().unwrap(),
                },
                Factorial {
                    number: 6,
                    level: 1,
                    factorial: 720.to_bigint().unwrap(),
                },
                Factorial {
                    number: 3249,
                    level: 1,
                    factorial: math::factorial(3249, 1),
                },
            ],
            status: vec![Status::FactorialsFound, Status::ReplyWouldBeTooLong],
        };

        let reply = comment.get_reply();
        assert_eq!(reply, "Sorry bro, but if I calculate the factorial(s) of the number(s) [5, 6, 3249], the reply would be too long for reddit :(\n\n\n*^(This action was performed by a bot. Please contact u/tolik518 if you have any questions or concerns.)*");
    }

    #[test]
    fn test_get_reply_too_long_from_new_comment() {
        let comment =
            RedditComment::new("This is a test comment with a factorial of 4000!", "1234");

        let reply = comment.get_reply();
        assert_eq!(reply, "Sorry bro, but if I calculate the factorial(s) of the number(s) [4000], the reply would be too long for reddit :(\n\n\n*^(This action was performed by a bot. Please contact u/tolik518 if you have any questions or concerns.)*");
    }
}
