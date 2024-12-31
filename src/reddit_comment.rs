use crate::math;
use fancy_regex::Regex;
use num_bigint::BigInt;
use num_traits::{One, ToPrimitive};
use std::fmt::Write;

pub(crate) const UPPER_CALCULATION_LIMIT: i64 = 100_001;
const PLACEHOLDER: &str = "Factorial of ";
const FOOTER_TEXT: &str =
    "\n*^(This action was performed by a bot. Please DM me if you have any questions.)*";
pub(crate) const MAX_COMMENT_LENGTH: i64 = 10_000 - 10 - FOOTER_TEXT.len() as i64;
pub(crate) const NUMBER_DECIMALS_SCIENTIFIC: usize = 100;

#[derive(Debug, Clone, PartialEq, Ord, Eq, Hash, PartialOrd)]
pub(crate) struct Factorial {
    pub(crate) number: u64,
    pub(crate) level: u64,
    pub(crate) factorial: BigInt,
}

#[derive(Debug)]
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

pub trait Unzip3<A, B, C> {
    fn unzip3(self) -> (Vec<A>, Vec<B>, Vec<C>);
}

impl<A, B, C> Unzip3<A, B, C> for std::vec::IntoIter<(A, B, C)> {
    fn unzip3(self) -> (Vec<A>, Vec<B>, Vec<C>) {
        let mut vec_a = Vec::new();
        let mut vec_b = Vec::new();
        let mut vec_c = Vec::new();

        for (a, b, c) in self {
            vec_a.push(a);
            vec_b.push(b);
            vec_c.push(c);
        }

        (vec_a, vec_b, vec_c)
    }
}

impl RedditComment {
    pub(crate) fn new(body: &str, id: &str) -> Self {
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

            // Check if the number is within a reasonable range to compute
            if num > BigInt::from(UPPER_CALCULATION_LIMIT) {
                status.push(Status::NumberTooBig);
            } else if num == BigInt::one() {
                continue;
            } else {
                let num = num.to_u64().expect("Failed to convert BigInt to i64");
                let factorial = math::factorial(num, exclamation_count);
                factorial_list.push(Factorial {
                    number: num,
                    level: exclamation_count,
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

    fn factorials_are_too_long(factorial_list: &[Factorial]) -> bool {
        factorial_list
            .iter()
            .any(|Factorial { number, level, .. }| match level {
                1 => *number > 3249,
                2 => *number > 5982,
                3 => *number > 8572,
                4 => *number > 11077,
                5 => *number > 13522,
                6 => *number > 15920,
                7 => *number > 18282,
                8 => *number > 20613,
                9 => *number > 22920,
                10 => *number > 25208,
                11 => *number > 27479,
                12 => *number > 29735,
                13 => *number > 31977,
                14 => *number > 34207,
                15 => *number > 36426,
                16 => *number > 38635,
                17 => *number > 40835,
                18 => *number > 43027,
                19 => *number > 45212,
                20 => *number > 47390,
                21 => *number > 49562,
                22 => *number > 51728,
                23 => *number > 53889,
                24 => *number > 56045,
                25 => *number > 58197,
                26 => *number > 60345,
                27 => *number > 62489,
                28 => *number > 64630,
                29 => *number > 66768,
                30 => *number > 68903,
                31 => *number > 71036,
                32 => *number > 73167,
                33 => *number > 75296,
                34 => *number > 77423,
                35 => *number > 79548,
                36 => *number > 81672,
                37 => *number > 83794,
                38 => *number > 85915,
                39 => *number > 88035,
                40 => *number > 90154,
                41 => *number > 92272,
                42 => *number > 94389,
                43 => *number > 96505,
                44 => *number > 98620,
                45 => *number > 100734,
                _ => false,
            })
    }

    pub(crate) fn add_status(&mut self, status: Status) {
        self.status.push(status);
    }

    pub(crate) fn get_reply(&self) -> String {
        let mut reply;

        // Normal case
        if !(self.status.contains(&Status::ReplyWouldBeTooLong)) {
            reply = self
                .factorial_list
                .iter()
                .fold(String::new(), |mut acc, factorial| {
                    let factorial_level_string =
                        RedditComment::get_factorial_level_string(factorial.level);
                    let _ = write!(
                        acc,
                        "{}{}{} is {} \n\n",
                        factorial_level_string, PLACEHOLDER, factorial.number, factorial.factorial
                    );
                    acc
                });

            reply.push_str(FOOTER_TEXT);
            return reply;
        }

        // Too long reply
        let numbers: Vec<u64> = self.factorial_list.iter().map(|f| f.number).collect();

        let (factorial_lengths, factorial_decimals, factorial_level_names): (
            Vec<u64>,
            Vec<String>,
            Vec<&str>,
        ) = self
            .factorial_list
            .iter()
            .map(|f| {
                let mut truncated_number = f.factorial.to_string();
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

                let factorial_level_names = RedditComment::get_factorial_level_string(f.level);

                (length as u64, truncated_number, factorial_level_names)
            })
            .collect::<Vec<_>>() // Collect into a vector of tuples
            .into_iter()
            .unzip3(); // Unzip into three separate vectors

        if numbers.len() == 1 {
            let factorial_level_string =
                RedditComment::get_factorial_level_string(self.factorial_list[0].level);
            reply = format!(
                "If I post the whole number, the comment would get too long, as reddit only allows up to 10k characters.\n\n \
                In scientific notation the {}factorial of {} would be (roughly) {}e{} though :)\n\n",
                factorial_level_string, numbers[0], factorial_decimals[0], factorial_lengths[0]-1 // exponent is one less than the length
            );
        } else {
            let formatted_scientifics = factorial_lengths
                .iter()
                .zip(factorial_decimals)
                .zip(numbers)
                .zip(factorial_level_names)
                .map(|(((length, truncated_number), number), factorial_level)| {
                    format!(
                        "{factorial_level}Factorial of {number} = {truncated_number}e{}",
                        length - 1
                    )
                })
                .fold(String::new(), |a, e| {
                    if !a.is_empty() {
                        format!("{a},\n\n{e}")
                    } else {
                        e
                    }
                });
            reply = format!(
                "If I post the whole numbers, the comment would get too long, as reddit only allows up to 10k characters.\n\n\
                In scientific notation the results would look roughly like that:\n\n{}\n\n:)\n\n",
                formatted_scientifics
            );
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
    fn test_comment_new_spoiler_html_encoded() {
        let comment = RedditComment::new("&gt;!This is a spoiler comment 5!&lt;", "123");
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
    fn test_comment_new_decimals() {
        let comment = RedditComment::new("This is a test comment with decimal number 0.5!", "123");
        assert_eq!(comment.factorial_list, vec![]);
        assert_eq!(comment.status, vec![Status::NoFactorial]);
    }

    #[test]
    fn test_comment_new_comma_decimals() {
        let comment = RedditComment::new("This is a test comment with decimal number 0,5!", "123");
        assert_eq!(comment.factorial_list, vec![]);
        assert_eq!(comment.status, vec![Status::NoFactorial]);
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
    fn test_reply_text_too_long() {
        let comment = RedditComment::new(
            "3500! 3501! 3502! 3503! 3504! 3505! 3506! 3507! 3508! 3509! 3510! 3511! 3512! 3513! 3514! 3515! 3516! 3517! 3518! 3519! 3520! 3521! 3522! 3523! 3524! 3525! 3526! 3527! 3528! 3529! 3530! 3531! 3532! 3533! 3534! 3535! 3536! 3537! 3538! 3539! 3540! 3541! 3542! 3543! 3544! 3545! 3546! 3547! 3548! 3549! 3550! 3551! 3552! 3553! 3554! 3555! 3556! 3557! 3558! 3559! 3560! 3561! 3562! 3563! 3564! 3565! 3566! 3567! 3568! 3569! 3570! 3571! 3572! 3573! 3574! 3575! 3576! 3577! 3578! 3579! 3580! 3581! 3582! 3583! 3584! 3585! 3586! 3587! 3588! 3589! 3590! 3591! 3592! 3593! 3594! 3595! 3596! 3597! 3598! 3599! 3600!",
            "123",
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
                factorial: 280.to_bigint().unwrap(),
            }],
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
                    factorial: 60.to_bigint().unwrap(),
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
        assert_eq!(reply, "If I post the whole numbers, the comment would get too long, as reddit only allows up to 10k characters.\n\nIn scientific notation the results would look roughly like that:\n\nDouble-Factorial of 5 = 6.0e1,\n\nFactorial of 6 = 7.20e2,\n\nFactorial of 3249 = 6.4123376882765521838840963030568127691878727205333658692200854486404915724268122521695176119279253636e10000\n\n:)\n\n\n*^(This action was performed by a bot. Please DM me if you have any questions.)*");
    }

    #[test]
    fn test_get_reply_too_long_from_new_comment() {
        let comment =
            RedditComment::new("This is a test comment with a factorial of 4000!", "1234");

        let reply = comment.get_reply();
        assert_eq!(reply, "If I post the whole number, the comment would get too long, as reddit only allows up to 10k characters.\n\n In scientific notation the factorial of 4000 would be (roughly) 1.8288019515140650133147431755739190442173777107304392197064526954208959797973177364850370286870484107e12673 though :)\n\n\n*^(This action was performed by a bot. Please DM me if you have any questions.)*");
    }

    #[test]
    fn test_get_reply_too_long_from_new_comment_for_multifactorial() {
        let comment =
            RedditComment::new("This is a test comment with a factorial of 9000!!!", "1234");

        let reply = comment.get_reply();
        assert_eq!(reply, "If I post the whole number, the comment would get too long, as reddit only allows up to 10k characters.\n\n In scientific notation the Triple-factorial of 9000 would be (roughly) 9.5883799146548267640341391648545903348878025438772769707015576436531779580675303393957674423348854753e10561 though :)\n\n\n*^(This action was performed by a bot. Please DM me if you have any questions.)*");
    }

    #[test]
    fn test_get_reply_too_long_from_number_3250() {
        let comment =
            RedditComment::new("This is a test comment with a factorial of 3250!", "1234");

        let reply = comment.get_reply();
        assert_eq!(reply, "If I post the whole number, the comment would get too long, as reddit only allows up to 10k characters.\n\n In scientific notation the factorial of 3250 would be (roughly) 2.0840097486898794597623312984934641499860586341733439074965277708081597610387139819550932238765757432e10004 though :)\n\n\n*^(This action was performed by a bot. Please DM me if you have any questions.)*");
    }
}
