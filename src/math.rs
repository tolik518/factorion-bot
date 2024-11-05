use num_bigint::BigInt;
use num_traits::One;

pub(crate) fn factorial(n: i64) -> BigInt {
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
    use super::*;
    use num_bigint::ToBigInt;
    use num_traits::Zero;

    #[test]
    fn test_calculate_factorial() {
        assert_eq!(factorial(0), 1.to_bigint().unwrap());
        assert_eq!(factorial(1), 1.to_bigint().unwrap());
        assert_eq!(factorial(2), 2.to_bigint().unwrap());
        assert_eq!(factorial(3), 6.to_bigint().unwrap());
        assert_eq!(factorial(4), 24.to_bigint().unwrap());
        assert_eq!(factorial(5), 120.to_bigint().unwrap());
        assert_eq!(factorial(6), 720.to_bigint().unwrap());
        assert_eq!(factorial(7), 5040.to_bigint().unwrap());
        assert_eq!(factorial(8), 40320.to_bigint().unwrap());
        assert_eq!(factorial(9), 362880.to_bigint().unwrap());
        assert_eq!(factorial(10), 3628800.to_bigint().unwrap());
    }

    #[test]
    fn test_calculate_factorials_with_interesting_lengths() {
        let result = factorial(22);
        assert_eq!(22, result.to_string().len());

        let result = factorial(23);
        assert_eq!(23, result.to_string().len());

        let result = factorial(24);
        assert_eq!(24, result.to_string().len());

        let result = factorial(82);
        assert_eq!(123, result.to_string().len());

        let result = factorial(3909);
        assert_eq!(12346, result.to_string().len());

        let result = factorial(574);
        assert_eq!(1337, result.to_string().len());
    }

    #[test]
    #[ignore]
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
    #[ignore]
    fn test_calculate_factorial_hundred_thousand() {
        let num = 100_001;
        let result = factorial(num);
        assert_eq!(result.to_string().len(), 456579);
    }
}
