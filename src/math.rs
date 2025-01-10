use rug::integer::IntegerExt64;
use rug::{Complete, Integer};

pub fn factorial(n: u64, k: u64) -> Integer {
    Integer::factorial_m_64(n, k).complete()
}

/// Calculates Sterling's Approximation of large factorials.
/// Returns a float with the digits, and an int containing the extra base 10 exponent.
///
/// Algorithm adapted from [Wikipedia](https://en.wikipedia.org/wiki/Stirling's_approximation) as cc-by-sa-4.0
pub fn approximate_factorial(n: u64) -> (f64, u64) {
    let n = n as f64;
    let base = n / std::f64::consts::E;
    let ten_in_base = 10.0f64.log(base);
    let extra = (n / ten_in_base) as u64;
    let exponent = n - ten_in_base * extra as f64;
    let factorial = base.powf(exponent) * (std::f64::consts::TAU * n).sqrt();
    // Numerators from https://oeis.org/A001163 (cc-by-sa-4.0)
    let numerators: [f64; 17] = [
        1.0,
        1.0,
        1.0,
        -139.0,
        -571.0,
        163879.0,
        5246819.0,
        -534703531.0,
        -4483131259.0,
        432261921612371.0,
        6232523202521089.0,
        -25834629665134204969.0,
        -1579029138854919086429.0,
        746590869962651602203151.0,
        1511513601028097903631961.0,
        -8849272268392873147705987190261.0,
        -142801712490607530608130701097701.0,
    ];
    // Denominators from https://oeis.org/A001164 (cc-by-sa-4.0)
    let denominators: [f64; 17] = [
        1.0,
        12.0,
        288.0,
        51840.0,
        2488320.0,
        209018880.0,
        75246796800.0,
        902961561600.0,
        86684309913600.0,
        514904800886784000.0,
        86504006548979712000.0,
        13494625021640835072000.0,
        9716130015581401251840000.0,
        116593560186976815022080000.0,
        2798245444487443560529920000.0,
        299692087104605205332754432000000.0,
        57540880724084199423888850944000000.0,
    ];
    let series_sum: f64 = numerators
        .into_iter()
        .zip(denominators)
        .enumerate()
        .map(|(m, (num, den))| num / (den * n.powf(m as f64)))
        .sum();
    let factorial = factorial * series_sum;
    (factorial, extra)
}

/// Calculates the approximate digits of a multifactorial.
/// This is based on the base 10 logarithm of Sterling's Approximation.
///
/// # Panic
/// This function will panic if the output is too large to fit in a u64.
/// It is recommended to only use inputs up to 1 Quintillion.
///
/// Algorithm adapted from [Wikipedia](https://en.wikipedia.org/wiki/Stirling's_approximation) as cc-by-sa-4.0
pub fn approximate_multifactorial_digits(n: u128, k: u64) -> u128 {
    let n = n as f64;
    let k = k as f64;
    let base = n.log(10.0);
    ((0.5 + n / k) * base - n / k / 10.0f64.ln()) as u128 + 1
}

/// Formats the output of [`approximate_factorial`], by combining the 10 exponents of the number and the extra exponent.
pub fn format_approximate_factorial((x, e): (f64, u64)) -> String {
    let extra = x.log10() as u64;
    let x = x / (10.0f64.powf(extra as f64));
    let total_exponent = extra + e;
    format!("{x}e{total_exponent}")
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
    use crate::math::approximate_factorial;
    use super::*;
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
    fn test_calculate_factorial_hundred_thousand() {
        let num = 100_001;
        let result = factorial(num, 1);
        assert_eq!(result.to_string().len(), 456579);
    }

    #[test]
    fn test_calculate_factorial_multilevel_hundred_thousand() {
        let num = 100_001;
        let result = factorial(num, 10);
        assert_eq!(result.to_string().len(), 45660);
    }

    #[test]
    fn test_calculate_two_hundred_thousand() {
        let num = 200_000;
        let result = factorial(num, 1);
        assert_eq!(result.to_string().len(), 973351);
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

    #[test]
    fn test_approximate_factorial() {
        // NOTE: only the first decimals are correct
        assert_eq!(
            format_approximate_factorial(approximate_factorial(100_001)),
            "2.8242576501182115e456578" // 9 decimals
        );
        assert_eq!(
            format_approximate_factorial(approximate_factorial(2_546_372_899)),
            "7.7547455955465185e22845109185" // 4 decimals
        );
        assert_eq!(
            format_approximate_factorial(approximate_factorial(500_000_000_000)),
            "4.280903142280765e5632337761222" // 2 decimals
        );
        assert_eq!(
            format_approximate_factorial(approximate_factorial(712_460_928_486)),
            "2.982723728493957e8135211294800" // 2 decimals
        );
    }
    #[test]
    #[ignore = "future_improvement"]
    fn test_approximate_factorial_perfect() {
        // NOTE: all decimal are correct
        assert_eq!(
            format_approximate_factorial(approximate_factorial(100_001)),
            "2.8242576502544275e456578"
        );
        assert_eq!(
            format_approximate_factorial(approximate_factorial(2_546_372_899)),
            "7.7547841018050521e22845109185"
        );
        assert_eq!(
            format_approximate_factorial(approximate_factorial(500_000_000_000)),
            "4.2861118869966772e5632337761222"
        );
        assert_eq!(
            format_approximate_factorial(approximate_factorial(712_460_928_486)),
            "2.988979640465335e8135211294800"
        );
    }

    #[test]
    fn test_approximate_digits() {
        assert_eq!(approximate_multifactorial_digits(100_001, 1), 456_579);
        assert_eq!(
            approximate_multifactorial_digits(7_834_436_739, 1),
            74_111_525_394
        );
        assert_eq!(
            approximate_multifactorial_digits(738_247_937_346_920, 1),
            10_655_802_631_914_633
        );
        assert_eq!(
            approximate_multifactorial_digits(827_829_849_020_729_846, 1),
            14_473_484_525_026_752_513 // NOTE: Last 4 digits are wrong
        );
        assert_eq!(
            approximate_multifactorial_digits(1_000_000_000_000_000_000, 1),
            17_565_705_518_096_744_449 // NOTE: Last 4 digits are wrong
        );
        assert_eq!(
            approximate_multifactorial_digits(1_000_000_000_000_000_000_000_000_000_000_000_000, 1),
            35_565_705_518_096_741_787_712_172_651_953_782_785 // NOTE: Last 22 digits are wrong
        );
        assert_eq!(approximate_multifactorial_digits(100_001, 2), 228_291);
        assert_eq!(
            approximate_multifactorial_digits(7_834_436_739, 2),
            37_055_762_699
        );
        assert_eq!(
            approximate_multifactorial_digits(738_247_937_346_920, 2),
            5_327_901_315_957_321
        );
        assert_eq!(
            approximate_multifactorial_digits(827_829_849_020_729_846, 2),
            7_236_742_262_513_376_257 // NOTE: Last 3 digits are wrong
        );
        // TODO(test): test digit approximations for n-factorials (need to find a good reference)
    }

    #[test]
    #[ignore = "future_improvement"]
    fn test_approximate_digits_perfect() {
        // NOTE: All correct
        assert_eq!(approximate_multifactorial_digits(100_001, 1), 456_579);
        assert_eq!(
            approximate_multifactorial_digits(7_834_436_739, 1),
            74_111_525_394
        );
        assert_eq!(
            approximate_multifactorial_digits(738_247_937_346_920, 1),
            10_655_802_631_914_633
        );
        assert_eq!(
            approximate_multifactorial_digits(827_829_849_020_729_846, 1),
            14_473_484_525_026_753_452
        );
        assert_eq!(
            approximate_multifactorial_digits(1_000_000_000_000_000_000, 1),
            17_565_705_518_096_748_182
        );
        assert_eq!(
            approximate_multifactorial_digits(1_000_000_000_000_000_000_000_000_000_000_000_000, 1),
            35_565_705_518_096_748_172_348_871_081_083_394_937
        );
        assert_eq!(approximate_multifactorial_digits(100_001, 2), 228_291);
        assert_eq!(
            approximate_multifactorial_digits(7_834_436_739, 2),
            37_055_762_699
        );
        assert_eq!(
            approximate_multifactorial_digits(738_247_937_346_920, 2),
            5_327_901_315_957_321
        );
        assert_eq!(
            approximate_multifactorial_digits(827_829_849_020_729_846, 2),
            7_236_742_262_513_376_731
        );
    }
}
