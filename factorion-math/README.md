# Factorion Math
The math functions used by factorion. (Factorials and related functions)
All functions that take [Integer]s, but internally use [Float]s, not only have a float alternative, but also take in a precision.

This crate uses [rug] in its interface. It is re-exported for convenience.

## Features
- k-factorials in exact, float, approximate, approximate digits
- termials in exact, float, approximate, approximate digits
- k-termials in exact, approximate, approximate digits
- subfactorials in exact, approximate, approximate digits

Calculations are split in areas:
- exact: integer calculation (accuracy)
- float: float calculation (decimals)
- approximate: approximation of integer calculation using float as a * 10^b (large numbers)
- approximate digits: approximation of integer calculation using float as 10^b (extremely large numbers)

Formulas and their derivations are available in [MATH.md](https://github.com/tolik518/factorion-bot/blob/master/MATH.md)
