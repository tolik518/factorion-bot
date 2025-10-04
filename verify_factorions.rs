// Manual verification that our factorion detection logic is correct
// This can be run independently to verify the math

fn factorial_of_digit(digit: u8) -> u64 {
    match digit {
        0 => 1,   // 0! = 1
        1 => 1,   // 1! = 1
        2 => 2,   // 2! = 2
        3 => 6,   // 3! = 6
        4 => 24,  // 4! = 24
        5 => 120, // 5! = 120
        6 => 720, // 6! = 720
        7 => 5040, // 7! = 5040
        8 => 40320, // 8! = 40320
        9 => 362880, // 9! = 362880
        _ => panic!("Invalid digit: {}", digit),
    }
}

fn is_factorion(num: u64) -> bool {
    let num_str = num.to_string();
    let sum: u64 = num_str
        .chars()
        .map(|c| c.to_digit(10).unwrap() as u8)
        .map(factorial_of_digit)
        .sum();
    
    sum == num
}

fn main() {
    println!("🧮 Factorion Verification Tool");
    println!("==============================");
    
    // Test known factorions
    let known_factorions = [1, 2, 145, 40585];
    
    println!("\n✅ Testing known factorions:");
    for &num in &known_factorions {
        let is_fact = is_factorion(num);
        let num_str = num.to_string();
        let calculation: String = num_str
            .chars()
            .map(|c| format!("{}!", c))
            .collect::<Vec<_>>()
            .join(" + ");
        
        let sum: u64 = num_str
            .chars()
            .map(|c| c.to_digit(10).unwrap() as u8)
            .map(factorial_of_digit)
            .sum();
            
        println!("  {} = {} = {} -> {}", num, calculation, sum, 
                if is_fact { "✅ FACTORION" } else { "❌ NOT FACTORION" });
    }
    
    // Test some non-factorions
    let non_factorions = [3, 4, 5, 6, 10, 120, 144, 146];
    
    println!("\n❌ Testing non-factorions:");
    for &num in &non_factorions {
        let is_fact = is_factorion(num);
        let num_str = num.to_string();
        let calculation: String = num_str
            .chars()
            .map(|c| format!("{}!", c))
            .collect::<Vec<_>>()
            .join(" + ");
        
        let sum: u64 = num_str
            .chars()
            .map(|c| c.to_digit(10).unwrap() as u8)
            .map(factorial_of_digit)
            .sum();
            
        println!("  {} = {} = {} -> {}", num, calculation, sum, 
                if is_fact { "✅ FACTORION" } else { "❌ NOT FACTORION" });
    }
    
    println!("\n🎯 Example bot responses:");
    println!("=========================");
    
    println!("\n📝 When user calculates something that results in 145:");
    println!("The factorial of X is 145");
    println!();
    println!("**Interesting!** 145 is a factorion - a number that equals the sum of the factorial of its digits!");
    println!("*This action was performed by a bot.*");
    
    println!("\n📝 When user calculates multiple factorions (1, 2, 145):");
    println!("The factorial of A is 1");
    println!("The factorial of B is 2");  
    println!("The factorial of C is 145");
    println!();
    println!("**Interesting!** 1, 2, 145 are factorions - numbers that equal the sum of the factorial of their digits!");
    println!("*This action was performed by a bot.*");
    
    println!("\n📝 When user calculates normal number (120):");
    println!("The factorial of 5 is 120");
    println!();
    println!("*This action was performed by a bot.*");
    println!("(No special message - 120 is not a factorion)");
}