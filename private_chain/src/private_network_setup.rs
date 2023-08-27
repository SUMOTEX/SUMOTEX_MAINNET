use std::io;

fn main() {
    // Question 1
    println!("Network name?");
    let name = read_input().trim().to_string();

    // Question 2
    println!("How old are you?");
    let age: i32 = read_input().trim().parse().expect("Please enter a valid number.");

    // Question 3
    println!("What is your favorite programming language?");
    let language = read_input().trim().to_string();

    // Display collected information
    println!("\nSummary:");
    println!("Name: {}", name);
    println!("Age: {}", age);
    println!("Favorite Programming Language: {}", language);
}

fn read_input() -> String {
    let mut input = String::new();
    io::stdin().read_line(&mut input).expect("Failed to read line");
    input
}
