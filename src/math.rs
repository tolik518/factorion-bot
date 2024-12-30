use num_bigint::BigInt;
use num_traits::One;

pub fn factorial(n: u64, k: u64) -> BigInt {
    if n <= 1 {
        return BigInt::one();
    }
    let i_max = (n - 1) / k;
    multifactorial_recursive(n, k, 0, i_max)
}
fn multifactorial_recursive(n: u64, k: u64, low_i: u64, high_i: u64) -> BigInt {
    if low_i > high_i {
        One::one()
    } else if low_i == high_i {
        BigInt::from(n - k * low_i)
    } else if high_i - low_i == 1 {
        let t_low = n - k * low_i;
        let t_high = n - k * high_i;
        BigInt::from(t_low) * BigInt::from(t_high)
    } else {
        let mid_i = (low_i + high_i) / 2;
        let left = multifactorial_recursive(n, k, low_i, mid_i);
        let right = multifactorial_recursive(n, k, mid_i + 1, high_i);
        left * right
    }
}

/// Rounds a base 10 number string.
/// Uses the last digit to decide the rounding direction.
/// Rounds over 9s. This does **not** keep the length or turn rounded over digits into zeros.
/// If the input is all 9s, this will round to 10.
///
/// # Panic
/// This function may panic if less than two digits are supplied, or if it contains a non-digit of base 10.
pub(crate) fn round(number: &mut String) {
    // Check additional digit if we need to round
    if let Some(digit) = number
        .pop()
        .map(|n| n.to_digit(10).expect("Not a base 10 number"))
    {
        if digit >= 5 {
            let mut last_digit = number
                .pop()
                .and_then(|n| n.to_digit(10))
                .expect("Not a base 10 number");
            // Carry over at 9s
            while last_digit == 9 {
                let Some(digit) = number
                    .pop()
                    .map(|n| n.to_digit(10).expect("Not a base 10 number"))
                else {
                    // If we reached the end we get 10
                    *number = "10".to_string();
                    return;
                };
                last_digit = digit;
            }
            // Round up
            number.push_str(&format!("{}", last_digit + 1));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use num_bigint::ToBigInt;
    use num_traits::Zero;
    use std::str::FromStr;
    #[test]
    fn test_calculate_multi_single_factorial() {
        assert_eq!(factorial(0, 1), 1.to_bigint().unwrap());
        assert_eq!(factorial(1, 1), 1.to_bigint().unwrap());
        assert_eq!(factorial(2, 1), 2.to_bigint().unwrap());
        assert_eq!(factorial(3, 1), 6.to_bigint().unwrap());
        assert_eq!(factorial(4, 1), 24.to_bigint().unwrap());
        assert_eq!(factorial(5, 1), 120.to_bigint().unwrap());
        assert_eq!(factorial(6, 1), 720.to_bigint().unwrap());
        assert_eq!(factorial(7, 1), 5040.to_bigint().unwrap());
        assert_eq!(factorial(8, 1), 40320.to_bigint().unwrap());
        assert_eq!(factorial(9, 1), 362880.to_bigint().unwrap());
        assert_eq!(factorial(10, 1), 3628800.to_bigint().unwrap());
    }

    #[test]
    fn test_calculate_multi_double_factorial() {
        assert_eq!(factorial(0, 2), 1.to_bigint().unwrap());
        assert_eq!(factorial(1, 2), 1.to_bigint().unwrap());
        assert_eq!(factorial(2, 2), 2.to_bigint().unwrap());
        assert_eq!(factorial(3, 2), 3.to_bigint().unwrap());
        assert_eq!(factorial(4, 2), 8.to_bigint().unwrap());
        assert_eq!(factorial(5, 2), 15.to_bigint().unwrap());
        assert_eq!(factorial(6, 2), 48.to_bigint().unwrap());
        assert_eq!(factorial(7, 2), 105.to_bigint().unwrap());
        assert_eq!(factorial(8, 2), 384.to_bigint().unwrap());
        assert_eq!(factorial(9, 2), 945.to_bigint().unwrap());
        assert_eq!(factorial(10, 2), 3840.to_bigint().unwrap());
        assert_eq!(
            factorial(100, 2),
            BigInt::from_str(
                "34243224702511976248246432895208185975118675053719198827915654463488000000000000"
            )
            .unwrap()
        );
    }

    #[test]
    fn test_calculate_triple_factorial() {
        assert_eq!(factorial(0, 3), 1.to_bigint().unwrap());
        assert_eq!(factorial(1, 3), 1.to_bigint().unwrap());
        assert_eq!(factorial(2, 3), 2.to_bigint().unwrap());
        assert_eq!(factorial(3, 3), 3.to_bigint().unwrap());
        assert_eq!(factorial(4, 3), 4.to_bigint().unwrap());
        assert_eq!(factorial(5, 3), 10.to_bigint().unwrap());
        assert_eq!(factorial(6, 3), 18.to_bigint().unwrap());
        assert_eq!(factorial(7, 3), 28.to_bigint().unwrap());
        assert_eq!(factorial(8, 3), 80.to_bigint().unwrap());
        assert_eq!(factorial(9, 3), 162.to_bigint().unwrap());
        assert_eq!(factorial(10, 3), 280.to_bigint().unwrap());
        assert_eq!(factorial(20, 3), 4188800.to_bigint().unwrap());
        assert_eq!(factorial(22, 3), 24344320.to_bigint().unwrap());
        assert_eq!(factorial(25, 3), 608608000.to_bigint().unwrap());
        assert_eq!(
            factorial(100, 3),
            BigInt::from_str("174548867015437739741494347897360069928419328000000000").unwrap()
        );
    }

    #[test]
    fn test_calculate_quadruple_factorial() {
        assert_eq!(factorial(0, 4), 1.to_bigint().unwrap());
        assert_eq!(factorial(1, 4), 1.to_bigint().unwrap());
        assert_eq!(factorial(2, 4), 2.to_bigint().unwrap());
        assert_eq!(factorial(3, 4), 3.to_bigint().unwrap());
        assert_eq!(factorial(4, 4), 4.to_bigint().unwrap());
        assert_eq!(factorial(5, 4), 5.to_bigint().unwrap());
        assert_eq!(factorial(6, 4), 12.to_bigint().unwrap());
        assert_eq!(factorial(7, 4), 21.to_bigint().unwrap());
        assert_eq!(factorial(8, 4), 32.to_bigint().unwrap());
        assert_eq!(factorial(9, 4), 45.to_bigint().unwrap());
        assert_eq!(factorial(10, 4), 120.to_bigint().unwrap());
        assert_eq!(factorial(20, 4), 122880.to_bigint().unwrap());
        assert_eq!(factorial(22, 4), 665280.to_bigint().unwrap());
        assert_eq!(factorial(25, 4), 5221125.to_bigint().unwrap());
        assert_eq!(
            factorial(100, 4),
            BigInt::from_str("17464069942802730897824646237782016000000").unwrap()
        );
    }

    #[test]
    fn test_calculate_quituple_factorial() {
        assert_eq!(factorial(0, 5), 1.to_bigint().unwrap());
        assert_eq!(factorial(1, 5), 1.to_bigint().unwrap());
        assert_eq!(factorial(2, 5), 2.to_bigint().unwrap());
        assert_eq!(factorial(3, 5), 3.to_bigint().unwrap());
        assert_eq!(factorial(4, 5), 4.to_bigint().unwrap());
        assert_eq!(factorial(5, 5), 5.to_bigint().unwrap());
        assert_eq!(factorial(6, 5), 6.to_bigint().unwrap());
        assert_eq!(factorial(7, 5), 14.to_bigint().unwrap());
        assert_eq!(factorial(8, 5), 24.to_bigint().unwrap());
        assert_eq!(factorial(9, 5), 36.to_bigint().unwrap());
        assert_eq!(factorial(10, 5), 50.to_bigint().unwrap());
        assert_eq!(factorial(15, 5), 750.to_bigint().unwrap());
        assert_eq!(factorial(20, 5), 15000.to_bigint().unwrap());
        assert_eq!(factorial(22, 5), 62832.to_bigint().unwrap());
        assert_eq!(factorial(25, 5), 375000.to_bigint().unwrap());
        assert_eq!(
            factorial(100, 5),
            BigInt::from_str("232019615953125000000000000000000").unwrap()
        );
    }

    #[test]
    fn test_calculate_factorials_with_interesting_lengths() {
        let result = factorial(22, 1);
        assert_eq!(22, result.to_string().len());

        let result = factorial(23, 1);
        assert_eq!(23, result.to_string().len());

        let result = factorial(24, 1);
        assert_eq!(24, result.to_string().len());

        let result = factorial(82, 1);
        assert_eq!(123, result.to_string().len());

        let result = factorial(3909, 1);
        assert_eq!(12346, result.to_string().len());

        let result = factorial(574, 1);
        assert_eq!(1337, result.to_string().len());
    }

    #[test]
    fn test_calculate_factorial_with_ten_thousand_digits() {
        let mut num = 0;
        let mut result = BigInt::zero();
        while result.to_string().len() < 10_000 {
            num += 1;
            result = factorial(num, 1);
        }
        assert_eq!(num, 3249);
    }

    #[test]
    fn test_calculate_factorial_hundred_thousand() {
        let num = 100_001;
        let result = factorial(num, 1);
        assert_eq!(result.to_string().len(), 456579);
    }

    #[test]
    fn test_round_down() {
        let mut number = String::from("1929472373");
        round(&mut number);
        assert_eq!(number, "192947237");
    }

    #[test]
    fn test_round_up() {
        let mut number = String::from("74836748625");
        round(&mut number);
        assert_eq!(number, "7483674863");
    }

    #[test]
    fn test_round_carry() {
        let mut number = String::from("24999999995");
        round(&mut number);
        assert_eq!(number, "25");
    }
}
