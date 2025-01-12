use crate::math;
use fancy_regex::Regex;
use num_traits::ToPrimitive;
use rug::Integer;
use std::fmt::Write;
use crate::factorial::{CalculatedFactorial, Factorial, UPPER_APPROXIMATION_LIMIT, UPPER_CALCULATION_LIMIT, UPPER_DIGIT_APPROXIMATION_LIMIT};

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
    AlreadyRepliedOrRejected,
    NotReplied,
    NumberTooBigToCalculate,
    NoFactorial,
    ReplyWouldBeTooLong,
    FactorialsFound,
}

pub(crate) const PLACEHOLDER: &str = "Factorial of ";
const FOOTER_TEXT: &str =
    "\n*^(This action was performed by a bot. Please DM me if you have any questions.)*";
pub(crate) const MAX_COMMENT_LENGTH: i64 = 10_000 - 10 - FOOTER_TEXT.len() as i64;
pub(crate) const NUMBER_DECIMALS_SCIENTIFIC: usize = 100;


impl RedditComment {
    pub(crate) fn new(body: &str, id: &str, author: &str, subreddit: &str) -> Self {
        let factorial_regex =
            Regex::new(r"(?<![,.!?\d])\b(\d+)(!+)(?![<\d]|&lt;)").expect("Invalid factorial regex");
        let mut factorial_list: Vec<Factorial> = Vec::new();
        let mut status: Vec<Status> = vec![];

        for regex_capture in factorial_regex.captures_iter(body) {
            let regex_capture = regex_capture.expect("Failed to capture regex");

            let num = regex_capture[1]
                .parse::<Integer>()
                .expect("Failed to parse number");

            let factorial_level = regex_capture[2]
                .len()
                .to_u64()
                .expect("Failed to convert exclamation count to u64");
            // Check if we can approximate the number of digits
            if num > UPPER_DIGIT_APPROXIMATION_LIMIT {
                status.push(Status::NumberTooBigToCalculate)
                // Check if we can approximate it
            } else if num > UPPER_APPROXIMATION_LIMIT
                || (factorial_level > 1 && num > UPPER_CALCULATION_LIMIT)
            {
                let num = num.to_u128().expect("Failed to convert BigInt to i64");
                let factorial = math::approximate_multifactorial_digits(num, factorial_level);
                factorial_list.push(Factorial {
                    number: num,
                    level: factorial_level,
                    factorial: CalculatedFactorial::ApproximateDigits(factorial),
                });
            // Check if the number is within a reasonable range to compute
            } else if num > UPPER_CALCULATION_LIMIT {
                let num = num.to_u64().expect("Failed to convert BigInt to i64");
                let factorial = math::approximate_factorial(num);
                factorial_list.push(Factorial {
                    number: num as u128,
                    level: factorial_level,
                    factorial: CalculatedFactorial::Approximate(factorial.0, factorial.1),
                });
            } else if num == 1 {
                continue;
            } else {
                let num = num.to_u64().expect("Failed to convert BigInt to i64");
                let factorial = math::factorial(num, factorial_level);
                factorial_list.push(Factorial {
                    number: num as u128,
                    level: factorial_level,
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

        RedditComment {
            id: id.to_string(),
            author: author.to_string(),
            subreddit: subreddit.to_string(),
            factorial_list,
            status,
        }
    }

    pub(crate) fn new_already_replied(id: &str, author: &str, subreddit: &str) -> Self {
        let factorial_list: Vec<Factorial> = Vec::new();
        let status: Vec<Status> = vec![Status::AlreadyRepliedOrRejected];

        RedditComment {
            id: id.to_string(),
            author: author.to_string(),
            subreddit: subreddit.to_string(),
            factorial_list,
            status,
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
                    factorial: CalculatedFactorial::Exact(Integer::from(120)),
                },
                Factorial {
                    number: 6,
                    level: 1,
                    factorial: CalculatedFactorial::Exact(Integer::from(720)),
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
                factorial: CalculatedFactorial::Exact(Integer::from(48)),
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
                factorial: CalculatedFactorial::Exact(Integer::from(18)),
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
                factorial: CalculatedFactorial::Exact(Integer::from(720))
            }]
        );
        assert_eq!(
            comment.status,
            vec![Status::NumberTooBigToCalculate, Status::FactorialsFound]
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
            vec![Status::NumberTooBigToCalculate, Status::NoFactorial]
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
                factorial: CalculatedFactorial::Exact(Integer::from(280)),
            }],
            author: "test_author".to_string(),
            subreddit: "test_subreddit".to_string(),
            status: vec![Status::FactorialsFound],
        };

        let reply = comment.get_reply();
        assert_eq!(reply, "Triple-Factorial of 10 is 280 \n\n\n*^(This action was performed by a bot. Please DM me if you have any questions.)*");
    }

    #[test]
    fn test_get_reply_for_high_multifactorial() {
        let comment = RedditComment {
            id: "123".to_string(),
            factorial_list: vec![Factorial {
                number: 10,
                level: 46,
                factorial: CalculatedFactorial::Exact(Integer::from(10)),
            }],
            author: "test_author".to_string(),
            subreddit: "test_subreddit".to_string(),
            status: vec![Status::FactorialsFound],
        };

        let reply = comment.get_reply();
        assert_eq!(reply, "46-Factorial of 10 is 10 \n\n\n*^(This action was performed by a bot. Please DM me if you have any questions.)*");
    }

    #[test]
    fn test_get_reply_for_multiple() {
        let comment = RedditComment {
            id: "123".to_string(),
            factorial_list: vec![
                Factorial {
                    number: 5,
                    level: 1,
                    factorial: CalculatedFactorial::Exact(Integer::from(120)),
                },
                Factorial {
                    number: 6,
                    level: 1,
                    factorial: CalculatedFactorial::Exact(Integer::from(720)),
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
                    factorial: CalculatedFactorial::Exact(Integer::from(60)),
                },
                Factorial {
                    number: 6,
                    level: 1,
                    factorial: CalculatedFactorial::Exact(Integer::from(720)),
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
        assert_eq!(reply, "If I post the whole numbers, the comment would get too long, as reddit only allows up to 10k characters. So I had to turn them into scientific notation.\n\nDouble-Factorial of 5 is 60 \n\nFactorial of 6 is 720 \n\nFactorial of 3249 is roughly 6.4123376882765521838840963030568127691878727205333658692200854486404915724268122521695176119279253636e10000 \n\n\n*^(This action was performed by a bot. Please DM me if you have any questions.)*");
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
                    factorial: CalculatedFactorial::Exact(Integer::from(384)),
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
