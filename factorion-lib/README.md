# Factorion
A library to create factorion-bots, contains logic for parsing, calculation, and formatting.

Before using any functions, you must initialize constants (float precision, limits, and number of decimals) using the `init` or `init_default` function.

## Usage
You can use the given abstraction for comments in [comment]:
```rust
use factorion_lib::comment::{Comment, CommentConstructed, CommentExtracted, CommentCalculated, Commands, Status};

// You need to initialize first
factorion_lib::init_default().unwrap();
// Construct a comment from the text, metadata (generic), commands and maximum comment length
let comment: CommentConstructed<&str> = Comment::new("Here might be a factorial 5!?", "meta", Commands::TERMIAL, 10_000);
// Here we just checked if it might contain a factorial and put things in the correct form.
// Now to parse and extract any calculations
let comment: CommentExtracted<&str> = comment.extract();
// Do all extracted calculations.
let mut comment: CommentCalculated<&str> = comment.calc();
// Set flag, so a user will be notified (used when summoning on someone else)
comment.notify = Some("@you".to_owned());
// Format the reply
let reply = comment.get_reply();
// Metadata is retained throughout
assert_eq!(comment.meta, "meta");
// Useful status
assert_eq!(comment.status, Status::FACTORIALS_FOUND);
// Good looking reply (reddit markdown formatting).
assert_eq!(reply, "Hey @you! \n\nThe termial of the factorial of 5 is 7260 \n\n\n*^(This action was performed by a bot. Please DM me if you have any questions.)*");
```
Or manually do the steps:
```rust
use factorion_lib::{parse::parse, calculation_tasks::{CalculationJob, CalculationBase}, calculation_results::{Calculation, CalculationResult, Number}};

// You need to initialize first
factorion_lib::init_default().unwrap();
// Parse the text for calculations
let calculations: Vec<CalculationJob> = parse("Some text with factorial 4!", true);
// These are given in an intemediate format for delayed calculation
assert_eq!(calculations, [CalculationJob {
  // The base may be a number or another job
  base: CalculationBase::Num(Number::Exact(4.into())),
  // Type of calculation
  level: 1,
  // how many minus signs were encountered
  negative: 0,
}]);
// Calculate that
let mut results: Vec<Calculation> = calculations.into_iter().flat_map(|job| job.execute(false)).filter_map(|x| x).collect();
// The result is given in another format.
assert_eq!(results, [
  Calculation {
    // The original value (innermost base)
    value: Number::Exact(4.into()),
    // The steps taken to get the result
    steps: vec![(1, 0)],
    // The result in different formats
    result: CalculationResult::Exact(24.into()),
  }
]);
let result = results.remove(0);
let mut formatted = String::new();
// Write the formatted result to a string (for efficiency). We don't want to shorten anything below that huge number
result.format(&mut formatted, false, false, &10000000000000000000u128.into()).unwrap();
assert_eq!(formatted, "The factorial of 4 is 24 \n\n");
```
