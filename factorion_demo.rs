// Simple demonstration of the factorion detection feature
// This shows how the bot would respond to different inputs

use factorion_lib::comment::*;
use factorion_lib::calculation_results::*;
use factorion_lib::rug::Integer;

fn main() {
    // Initialize the library
    let _ = factorion_lib::init_default();
    
    println!("🤖 Factorion Bot - Demonstrating Unique Messages Feature");
    println!("=".repeat(60));
    
    // Test case 1: Comment that results in calculating 145 (a factorion)
    println!("\n📝 Test Case 1: Result is 145 (factorion)");
    let comment_145 = Comment {
        meta: (),
        calculation_list: vec![
            Calculation {
                value: 12.into(), // Some input
                steps: vec![(1, 0)],
                result: CalculationResult::Exact(Integer::from(145)), // Result is factorion 145
            }
        ],
        notify: None,
        commands: Commands::NONE,
        max_length: 10000,
        status: Status::FACTORIALS_FOUND,
    };
    
    let reply_145 = comment_145.get_reply();
    println!("Bot Reply:");
    println!("{}", reply_145);
    
    // Test case 2: Comment that results in multiple factorions
    println!("\n📝 Test Case 2: Multiple factorions (1, 2, 145)");
    let comment_multiple = Comment {
        meta: (),
        calculation_list: vec![
            Calculation {
                value: 1.into(),
                steps: vec![(1, 0)],
                result: CalculationResult::Exact(Integer::from(1)),
            },
            Calculation {
                value: 2.into(),
                steps: vec![(1, 0)],
                result: CalculationResult::Exact(Integer::from(2)),
            },
            Calculation {
                value: 145.into(),
                steps: vec![(1, 0)],
                result: CalculationResult::Exact(Integer::from(145)),
            }
        ],
        notify: None,
        commands: Commands::NONE,
        max_length: 10000,
        status: Status::FACTORIALS_FOUND,
    };
    
    let reply_multiple = comment_multiple.get_reply();
    println!("Bot Reply:");
    println!("{}", reply_multiple);
    
    // Test case 3: Normal number (not a factorion)
    println!("\n📝 Test Case 3: Normal number (120 - not a factorion)");
    let comment_normal = Comment {
        meta: (),
        calculation_list: vec![
            Calculation {
                value: 5.into(),
                steps: vec![(1, 0)],
                result: CalculationResult::Exact(Integer::from(120)), // 5! = 120, not a factorion
            }
        ],
        notify: None,
        commands: Commands::NONE,
        max_length: 10000,
        status: Status::FACTORIALS_FOUND,
    };
    
    let reply_normal = comment_normal.get_reply();
    println!("Bot Reply:");
    println!("{}", reply_normal);
    
    // Test case 4: The largest known factorion (40585)
    println!("\n📝 Test Case 4: Largest known factorion (40585)");
    let comment_40585 = Comment {
        meta: (),
        calculation_list: vec![
            Calculation {
                value: 40585.into(),
                steps: vec![(1, 0)],
                result: CalculationResult::Exact(Integer::from(40585)),
            }
        ],
        notify: None,
        commands: Commands::NONE,
        max_length: 10000,
        status: Status::FACTORIALS_FOUND,
    };
    
    let reply_40585 = comment_40585.get_reply();
    println!("Bot Reply:");
    println!("{}", reply_40585);
    
    println!("\n✅ Demonstration complete!");
    println!("The bot now detects factorions and provides interesting educational messages!");
}