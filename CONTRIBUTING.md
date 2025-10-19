# Guide for Starters

## Github Intro
If you haven't used GitHub yet, you might be confused about some words. Here are the two most important ones, with links, were you can view and make them:
- [Issues](https://github.com/tolik518/factorion-bot/issues) can be bug reports (when something goes wrong) or feature requests (when you want something added)
- [Pull Request/PR](https://github.com/Aras14HD/factorion-bot/pulls) are where submits changes, can be code, text or locale.

## Ideas
If you have an idea to improve the bot, just request it as a feature in an issue.

Your issue should ideally contain:
1. Your request (what you want to see added)
2. Your reasoning (why factorion should have this, not necessary if obvious)
3. Resources (links to further information of the concept, more for new mathematical constructs, like termials, arcfactorials and similar)
## Code
If you want to work on an issue, just fork the repo and start.
Don't be intimidated by the size. If you are not able to write a part of a feature (be it the parsing, the math, or anything else) or only able to write one part, that's ok.
Someone else can continue your work.

If you are confused, where in the code to work, [Code Structure](#code-structure) has an overview.

When creating a PR (requesting a review), just make sure:
1. That the code compiles (`cargo build`)
2. That your addition is tested (add unit tests and `cargo test`)
3. That Clippy doesn't complain about your code (if possible, `cargo clippy`)
4. To bump the version number (SemVer-ish, if you're unsure, just ask)
## Math
If you are not comfortable writing code but know some math, you can still contribute.
Just write a comment on the issue in question with the formula and your reasoning.
That is, in many cases, already a lot of help.

The math with reasoning and derivation is separately documented in [Math](MATH.md).
## Translation
Alternatively you can provide translations.
While we prefer if you make a PR, you can also just put the locale json text in an issue.
(Please put \`\`\`json \`\`\` around the locale)
If you have any problems making one, just put what you can in an issue, and we will go from there.

The locale file format is documented in [Locale](factorion-lib/Locales.md)

You might want to check first if somebody else is already working on it, by looking in PRs and issues.

If you are confused, on what issues and PRs are, have a look at [Github Intro](#github-intro)
# Code Structure
## Modules
### General server stuff and API (factorion-bot-reddit)
- `main`: Program loop, executing steps, data saving/reading
- `reddit_api`: Interacting with Reddit (getting the comments, sending replies)
- `influxdb`: Sending stats
### Processing of comments (factorion-lib)
- `comment`: Executing steps for individual comments, comment metadata, reply arrangement (notes)
- `parse`: Finding factorials in comments (with skipping URLs and spoilers), parsing numbers
- `calculation_task`: Calculating factorials in different formats (including nested), simple math
- `calculation_result`: Formatting of factorial results (different representations)
- `lib`: Imports/Exports, Combined initializer
- `integration`: Integration tests (take a comment and do the whole pipeline)
### The base math (factorion-math)
- `lib`: The mathematical formulas implemented
