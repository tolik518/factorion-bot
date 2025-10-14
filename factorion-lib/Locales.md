# Factorion Locale Format
Factorion uses a versioned Locale format for backwards compatability, it is recommended to use the most up-to-date version.
Locales can theoretically be supplied in any format, the specific bot supports, but preffered and supported by official bots is json.

Here listed is the english locale in all versions, with comments explaining the settings.
Locales contain a few different kinds of settings:
- Simple text
- Templated text, where some "{template}" (like "{factorial}") is replaced by other text
- Character, a text that is only one character long (like "." or ",")
- Toggles: on is true, off is false
- Map, a list of key value pairs for certain settings, used for overrides

## V1
```json
// Version number
{ "V1":
  {
    // The little disclaimer at the end of a comment
    "bot_disclaimer": "This action was performed by a bot.",
    // The notes at the beginning of a comment
    "notes": {
      "tower": "That is so large, that I can't even give the number of digits of it, so I have to make a power of ten tower.",
      "tower_mult": "Some of these are so large, that I can't even give the number of digits of them, so I have to make a power of ten tower.",
      "digits": "That number is so large, that I can't even approximate it well, so I can only give you an approximation on the number of digits.",
      "digits_mult": "Some of these are so large, that I can't even approximate them well, so I can only give you an approximation on the number of digits.",
      "approx": "That is so large, that I can't calculate it, so I'll have to approximate.",
      "approx_mult": "Some of those are so large, that I can't calculate them, so I'll have to approximate.",
      "round": "I can't calculate such a large factorial of decimals. So I had to round at some point.",
      "round_mult": "I can't calculate that large factorials of decimals. So I had to round at some point.",
      "too_big": "If I post the whole number, the comment would get too long. So I had to turn it into scientific notation.",
      "too_big_mult": "If I post the whole numbers, the comment would get too long. So I had to turn them into scientific notation.",
      "remove": "If I posted all numbers, the comment would get too long. So I had to remove some of them.",
      "tetration": "That is so large, I can't even fit it in a comment with a power of 10 tower, so I'll have to use tetration!",
      "no_post": "Sorry, but the reply text for all those number would be _really_ long, so I'd rather not even try posting lmao",
      // How to call out to a user (when mentioning them). "{mention}" is replaced by the user string formatted as a mention
      "mention": "Hey {mention}!"
    },
    // Formatting calculations
    "format": {
      // Formatting numbers
      "number_format": {
        // The number decimal separator (also used when parsing). Must be a single character
        "decimal": "."
      },
      // Whether to capitalize the start of the calculation word (sub, uple, terimal or factorial) (ASCII only)
      "capitalize_calc": false,
      "termial": "termial",
      "factorial": "factorial",
      // What to call tuples. "{factorial}" is replaced by termial or factorial
      "uple": "uple-{factorial}",
      // What to call a subfactorial. "{factorial}" is replaced by termial (not currently) or factorial
      "sub": "sub{factorial}",
      // How to call a negative calculation. "{factorial}" is replaced by sub, uple, termial or factorial
      "negative": "negative {factorial}",
      // Overrides for individual tuples. "{factorial}" is replaced by termial or factorial
      "num_overrides": {
        "2": "double-{factorial}",
        "3": "triple-{factorial}"
      },
      // Turn off automatic tuple names (use overrides and numbers)
      "force_num": false,
      // How to write nested factorials. "{factorial}" is replaced by the outer calculation, "{next}" by the inner (both being negative, sub, uple, terimal or factorial)
      "nest": "{factorial} of {next}",
      // How to write a calculation with an exact result. "{factorial}" is replaced by nest, negative, sub, uple, terimal or factorial,
      // "{number}" by the input to the calcultion or rough_number and "{result}" by the result
      "exact": "{factorial} of {number} is {result}",
      // How to write a calculation with a shortened result. "{factorial}" is replaced by nest, negative, sub, uple, terimal or factorial,
      // "{number}" by the input to the calcultion or rough_number and "{result}" by the result
      "rough": "{factorial} of {number} is roughly {result}",
      // How to write a calculation with an approximate result. "{factorial}" is replaced by nest, negative, sub, uple, terimal or factorial,
      // "{number}" by the input to the calcultion or rough_number and "{result}" by the result
      "approx": "{factorial} of {number} is approximately {result}",
      // How to write a calculation with a result given in approximate number of digits. "{factorial}" is replaced by nest, negative, sub, uple, terimal or factorial,
      // "{number}" by the input to the calcultion or rough_number and "{result}" by the result
      "digits": "{factorial} of {number} has approximately {result} digits",
      // How to write a calculation with a result given in a power-of-ten tower number of digits.
      // "{factorial}" is replaced by nest, negative, sub, uple, terimal or factorial,
      // "{number}" by the input to the calcultion or rough_number and "{result}" by the result
      "order": "{factorial} of {number} has on the order of {result} digits",
      // How to write a calculation with a result given in tetration of ten number of digits, and no calculation steps.
      // "{factorial}" is replaced by nest, negative, sub, uple, terimal or factorial,
      // "{number}" by the input to the calcultion or rough_number and "{result}" by the result
      "all_that": "All that of {number} has on the order of {result} digits",
      // How to write a shortened number. "{number}" is replaced by the input to the calculation
      "rough_number": "roughly {number}"
    }
  }
}
```
