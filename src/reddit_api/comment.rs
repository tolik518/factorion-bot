#![allow(unused_imports)]
#![allow(deprecated)]
#![allow(unused_variables)]
use num_bigint::{BigInt, ToBigInt};
use num_traits::One;
use regex::Regex;

pub(crate) const UPPER_CALCULATION_LIMIT: i64 = 100_001;
pub(crate) const FOOTER_TEXT: &str = "\n\n*^(I am a bot, called factorion, and this action was performed automatically. Please contact u/tolik518 if you have any questions or concerns or just visit me on github https://github.com/tolik518/factorion-bot/)*";
pub(crate) const MAX_COMMENT_LENGTH: i64 = 10_000 - 10 - FOOTER_TEXT.len() as i64;

pub(crate) struct Comment {
    pub(crate) body: String,
    pub(crate) id: String,
    pub(crate) factorial_list: Vec<(i64, BigInt)>,
    pub(crate) status: Vec<Status>,
    pub(crate) is_ok: bool,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub(crate) enum Status {
    AlreadyReplied,
    NotReplied,
    NumberTooBig,
    NoFactorial,
    ReplyWouldBeTooLong,
    FactorialsFound
}

impl Comment {
    pub(crate) fn new(body: &str, id:  &str) -> Self {
        let factorial_regex = Regex::new(r"\b(\d+)!\B").expect("Invalid factorial regex");
        let mut factorial_list = Vec::new();
        let mut status: Vec<Status> = vec![];

        for regex_capture in factorial_regex.captures_iter(body)
        {
            let num = regex_capture[1].parse::<i64>().expect("Failed to parse number");

            // Check if the number is within a reasonable range to compute
            if num > UPPER_CALCULATION_LIMIT {
                status.push(Status::NumberTooBig);
            } else {
                let factorial = factorial(num);
                factorial_list.push((num, factorial.clone()));
            }
        }

        if factorial_list.is_empty() {
            status.push(Status::NoFactorial);
        } else {
            status.push(Status::FactorialsFound);
        }

        if factorial_list.iter().any(|(num, _)| *num > 3249) {
            status.push(Status::ReplyWouldBeTooLong);
        }

        Comment {
            body: body.to_string(),
            id: id.to_string(),
            factorial_list,
            status,
            is_ok: true
        }
    }

    fn new_not_ok() -> Self {
        Comment {
            body: "".to_string(),
            id: "".to_string(),
            factorial_list: vec![],
            status: vec![Status::NoFactorial],
            is_ok: false
        }
    }

    fn factorial_list(&self) -> &Vec<(i64, BigInt)> {
        &self.factorial_list
    }

    fn id(&self) -> &str {
        &self.id
    }

    fn status(&self) -> &Vec<Status> {
        &self.status
    }

    pub(crate) fn set_status(&mut self, status: Status) {
        self.status.push(status);
    }

    pub(crate) fn get_reply(&self) -> String {
        let mut reply = String::new();
        for (num, factorial) in self.factorial_list.iter() {
            reply.push_str(&format!("{}! = {}\n", num, factorial));
        }
        reply.push_str(FOOTER_TEXT);
        reply
    }
}

fn factorial(n: i64) -> BigInt {
    if n < 2 {
        return One::one();
    }
    factorial_recursive(1, n)
}

fn factorial_recursive(low: i64, high: i64) -> BigInt {
    if low > high {
        One::one()
    } else if low == high {
        BigInt::from(low)
    } else if high - low == 1 {
        BigInt::from(low) * BigInt::from(high)
    } else {
        let mid = (low + high) / 2;
        let left = factorial_recursive(low, mid);
        let right = factorial_recursive(mid + 1, high);
        left * right
    }
}

#[cfg(test)]
mod tests {
    use num_traits::Zero;
    use super::*;

    #[test]
    fn test_calculate_factorial() {
        assert_eq!(factorial(00),       1.to_bigint().unwrap());
        assert_eq!(factorial(01),       1.to_bigint().unwrap());
        assert_eq!(factorial(02),       2.to_bigint().unwrap());
        assert_eq!(factorial(03),       6.to_bigint().unwrap());
        assert_eq!(factorial(04),      24.to_bigint().unwrap());
        assert_eq!(factorial(05),     120.to_bigint().unwrap());
        assert_eq!(factorial(06),     720.to_bigint().unwrap());
        assert_eq!(factorial(07),    5040.to_bigint().unwrap());
        assert_eq!(factorial(08),   40320.to_bigint().unwrap());
        assert_eq!(factorial(09),  362880.to_bigint().unwrap());
        assert_eq!(factorial(10), 3628800.to_bigint().unwrap());
    }

    #[test]
    fn test_calculate_factorials_with_interesting_lengths(){
        let result = factorial(22);
        assert_eq!(22, result.to_string().len(), "{}", result);

        let result = factorial(23);
        assert_eq!(23, result.to_string().len(), "{}", result);

        let result = factorial(24);
        assert_eq!(24, result.to_string().len(), "{}", result);

        let result = factorial(82);
        assert_eq!(123, result.to_string().len(), "{}", result);

        let result = factorial(3909);
        assert_eq!(12346, result.to_string().len(), "{}", result);

        let result = factorial(574);
        assert_eq!(1337, result.to_string().len(), "{}", result);
    }

    #[test]
    fn test_calculate_factorial_with_ten_thousand_digits() {
        let mut num = 0;
        let mut result = BigInt::zero();
        while result.to_string().len() < 10_000 {
            num += 1;
            result = factorial(num);
        }
        assert_eq!(num, 3249);

    }

    #[test]
    fn test_calculate_factorial_hundred_thousand() {
        let num = 100_001;
        let result = factorial(num);
        assert_eq!(result.to_string().len(), 456579);
    }
}