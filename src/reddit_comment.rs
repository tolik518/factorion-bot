#![allow(unused_parens)]

use crate::math;
use num_bigint::BigInt;
use num_traits::{One, ToPrimitive};
use fancy_regex::Regex;

pub(crate) const UPPER_CALCULATION_LIMIT: i64 = 100_001;
const PLACEHOLDER: &str = "Factorial of ";
const FOOTER_TEXT: &str = "\n*^(This action was performed by a bot. Please contact u/tolik518 if you have any questions or concerns.)*";
pub(crate) const MAX_COMMENT_LENGTH: i64 = 10_000 - 10 - FOOTER_TEXT.len() as i64;

pub(crate) struct RedditComment {
    pub(crate) id: String,
    pub(crate) factorial_list: Vec<(i64, BigInt)>,
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
        let factorial_regex = Regex::new(r"(?<![.!?\d])\b(\d+)(!+)(?![<\d])").expect("Invalid factorial regex");
        let mut factorial_list = Vec::new();
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
                factorial_list.push((num, factorial.clone()));
            }
        }

        factorial_list.sort();
        factorial_list.dedup();

        if factorial_list.is_empty() {
            status.push(Status::NoFactorial);
        } else {
            status.push(Status::FactorialsFound);
        }

        if factorial_list.iter().any(|(num, _)| *num > 3249) {
            status.push(Status::ReplyWouldBeTooLong);
        }

        RedditComment {
            id: id.to_string(),
            factorial_list,
            status,
        }
    }

    pub(crate) fn add_status(&mut self, status: Status) {
        self.status.push(status);
    }

    pub(crate) fn get_reply(&self) -> String {
        let mut reply = String::new();
        if self.status.contains(&Status::ReplyWouldBeTooLong) {
            let mut numbers: Vec<i64> = Vec::new();
            for (num, _) in self.factorial_list.iter() {
                numbers.push(*num);
            }
            reply.push_str(&format!("Sorry bro, but if I calculate the factorial(s) of the number(s) {:?}, the reply would be too long for reddit :(\n\n", numbers));
        } else {
            for (num, factorial) in self.factorial_list.iter() {
                reply.push_str(&format!("{}{} is {} \n\n", PLACEHOLDER, num, factorial));
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
            vec![(5, 120.to_bigint().unwrap()), (6, 720.to_bigint().unwrap())]
        );
        assert_eq!(comment.status, vec![Status::FactorialsFound]);
    }

    #[test]
    fn test_comment_new_double_factorial() {
        let comment = RedditComment::new("This is a test comment with an n-factorial 6!!", "123");
        assert_eq!(comment.factorial_list, vec![(6, 48.to_bigint().unwrap())]);
        assert_eq!(comment.status, vec![Status::FactorialsFound]);
    }

    #[test]
    fn test_comment_new_triple_factorial() {
        let comment = RedditComment::new("This is a test comment with an n-factorial 6!!!", "123");
        assert_eq!(comment.factorial_list, vec![(6, 18.to_bigint().unwrap())]);
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
        assert_eq!(comment.factorial_list, vec![(6, 720.to_bigint().unwrap())]);
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
    fn test_get_reply() {
        let comment = RedditComment {
            id: "123".to_string(),
            factorial_list: vec![(5, 120.to_bigint().unwrap()), (6, 720.to_bigint().unwrap())],
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
                (5, 120.to_bigint().unwrap()),
                (6, 720.to_bigint().unwrap()),
                (3249, math::factorial(3249, 1)),
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