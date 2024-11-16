use rug::integer::IntegerExt64;
use rug::{Complete, Integer};

pub fn factorial(n: u64, k: u64) -> Integer  {
    Integer::factorial_m_64(n, k).complete()
}

#[cfg(test)]
mod tests {
    use super::*;
    use rug::Integer;
    use std::str::FromStr;
    #[test]
    fn test_calculate_multi_single_factorial() {
        assert_eq!(factorial(0, 1), Integer::from(1));
        assert_eq!(factorial(1, 1), Integer::from(1));
        assert_eq!(factorial(2, 1), Integer::from(2));
        assert_eq!(factorial(3, 1), Integer::from(6));
        assert_eq!(factorial(4, 1), Integer::from(24));
        assert_eq!(factorial(5, 1), Integer::from(120));
        assert_eq!(factorial(6, 1), Integer::from(720));
        assert_eq!(factorial(7, 1), Integer::from(5040));
        assert_eq!(factorial(8, 1), Integer::from(40320));
        assert_eq!(factorial(9, 1), Integer::from(362880));
        assert_eq!(factorial(10, 1), Integer::from(3628800));
    }

    #[test]
    fn test_calculate_multi_double_factorial() {
        assert_eq!(factorial(0, 2), Integer::from(1));
        assert_eq!(factorial(1, 2), Integer::from(1));
        assert_eq!(factorial(2, 2), Integer::from(2));
        assert_eq!(factorial(3, 2), Integer::from(3));
        assert_eq!(factorial(4, 2), Integer::from(8));
        assert_eq!(factorial(5, 2), Integer::from(15));
        assert_eq!(factorial(6, 2), Integer::from(48));
        assert_eq!(factorial(7, 2), Integer::from(105));
        assert_eq!(factorial(8, 2), Integer::from(384));
        assert_eq!(factorial(9, 2), Integer::from(945));
        assert_eq!(factorial(10, 2), Integer::from(3840));
        assert_eq!(
            factorial(100, 2),
            Integer::from_str(
                "34243224702511976248246432895208185975118675053719198827915654463488000000000000"
            )
            .unwrap()
        );
    }

    #[test]
    fn test_calculate_triple_factorial() {
        assert_eq!(factorial(0, 3), Integer::from(1));
        assert_eq!(factorial(1, 3), Integer::from(1));
        assert_eq!(factorial(2, 3), Integer::from(2));
        assert_eq!(factorial(3, 3), Integer::from(3));
        assert_eq!(factorial(4, 3), Integer::from(4));
        assert_eq!(factorial(5, 3), Integer::from(10));
        assert_eq!(factorial(6, 3), Integer::from(18));
        assert_eq!(factorial(7, 3), Integer::from(28));
        assert_eq!(factorial(8, 3), Integer::from(80));
        assert_eq!(factorial(9, 3), Integer::from(162));
        assert_eq!(factorial(10, 3), Integer::from(280));

        assert_eq!(factorial(20, 3), Integer::from(4188800));
        assert_eq!(factorial(22, 3), Integer::from(24344320));
        assert_eq!(factorial(25, 3), Integer::from(608608000));
        assert_eq!(
            factorial(100, 3),
            Integer::from_str("174548867015437739741494347897360069928419328000000000").unwrap()
        );
    }

    #[test]
    fn test_calculate_quadruple_factorial() {
        assert_eq!(factorial(0, 4), Integer::from(1));
        assert_eq!(factorial(1, 4), Integer::from(1));
        assert_eq!(factorial(2, 4), Integer::from(2));
        assert_eq!(factorial(3, 4), Integer::from(3));
        assert_eq!(factorial(4, 4), Integer::from(4));
        assert_eq!(factorial(5, 4), Integer::from(5));
        assert_eq!(factorial(6, 4), Integer::from(12));
        assert_eq!(factorial(7, 4), Integer::from(21));
        assert_eq!(factorial(8, 4), Integer::from(32));
        assert_eq!(factorial(9, 4), Integer::from(45));
        assert_eq!(factorial(10, 4), Integer::from(120));

        assert_eq!(factorial(20, 4), Integer::from(122880));
        assert_eq!(factorial(22, 4), Integer::from(665280));
        assert_eq!(factorial(25, 4), Integer::from(5221125));
        assert_eq!(
            factorial(100, 4),
            Integer::from_str("17464069942802730897824646237782016000000").unwrap()
        );
    }

    #[test]
    fn test_calculate_quituple_factorial() {
        assert_eq!(factorial(0, 5), Integer::from(1));
        assert_eq!(factorial(1, 5), Integer::from(1));
        assert_eq!(factorial(2, 5), Integer::from(2));
        assert_eq!(factorial(3, 5), Integer::from(3));
        assert_eq!(factorial(4, 5), Integer::from(4));
        assert_eq!(factorial(5, 5), Integer::from(5));
        assert_eq!(factorial(6, 5), Integer::from(6));
        assert_eq!(factorial(7, 5), Integer::from(14));
        assert_eq!(factorial(8, 5), Integer::from(24));
        assert_eq!(factorial(9, 5), Integer::from(36));
        assert_eq!(factorial(10, 5), Integer::from(50));

        assert_eq!(factorial(15, 5), Integer::from(750));
        assert_eq!(factorial(20, 5), Integer::from(15000));
        assert_eq!(factorial(22, 5), Integer::from(62832));
        assert_eq!(factorial(25, 5), Integer::from(375000));
        assert_eq!(
            factorial(100, 5),
            Integer::from_str("232019615953125000000000000000000").unwrap()
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
        let mut result = Integer::new();
        while result.to_string().len() < 10_000 {
            num += 1;
            result = factorial(num, 1);
        }
        assert_eq!(num, 3249);
    }

    #[test]
    fn test_calculate_factorial_with_ten_thousand_digits_for_level_five() {
        let mut num = 0;
        let mut result = Integer::new();
        while result.to_string().len() < 10_000 {
            num += 1;
            result = factorial(num, 5);
        }
        assert_eq!(num, 13522);
    }

    #[test]
    fn test_calculate_factorial_with_ten_thousand_digits_for_level_fourty() {
        let mut num = 0;
        let mut result = Integer::new();
        while result.to_string().len() < 10_000 {
            num += 1;
            result = factorial(num, 40);
        }
        assert_eq!(num, 88602);
    }

    #[test]
    fn test_calculate_factorial_hundred_thousand() {
        let num = 100_001;
        let result = factorial(num, 1);
        assert_eq!(result.to_string().len(), 456579);
    }
}
