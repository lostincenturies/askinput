//! Demo program for the `askinput` crate.
//!
//! Exercises every public function with a variety of inputs.
//!
//! Run with:
//!
//! ```text
//! cargo run --example demo
//! ```

use askinput::{read_float, read_floats, read_int, read_ints, read_str};

fn main() {
    println!("=== askinput demo ===\n");

    // --- read_int ---
    let a = read_int("Enter one integer: ");
    let b = read_int("Enter another integer: ");
    println!("Sum of {a} and {b} is {}\n", a + b);

    // --- read_ints (tuple of two) ---
    let (x, y) = read_ints("Enter two integers separated by whitespace: ");
    println!("Difference of {x} and {y} is {}\n", x - y);

    // --- read_float ---
    let f = read_float("Enter one floating-point number: ");
    println!("Doubled: {}\n", f * 2.0);

    // --- read_floats (tuple of two) ---
    let (c, d) = read_floats("Enter two floating-point numbers separated by whitespace: ");
    println!("{c} + {d} = {}\n", c + d);

    // --- read_str ---
    let name = read_str("Enter your name: ");
    println!("Hello, {name}!\n");

    println!("=== demo finished ===");
}
