# askinput

Paranoid typed input from stdin for interactive Rust CLI programs.

`askinput` is a thin, dependency-free wrapper around `std::io::stdin`
that takes the tedium out of interactive console input:

- displays a prompt and guarantees that `stdout` is flushed;
- reads one line with a hard byte limit;
- parses the line into `i64`, `f64`, `String`, or tuples of them;
- rejects control characters, `NaN`, `inf`, and other garbage;
- re-prompts on invalid input until the user gets it right;
- returns values directly into variables — no `match`, no `unwrap`,
  no intermediate `parse` calls in your code.

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
askinput = "0.1"
```

Or with `cargo add`:

```bash
cargo add askinput
```

## Example

```rust
use askinput::{read_float, read_floats, read_int, read_ints, read_str};

fn main() {
    let a = read_int("Enter one integer: ");
    let b = read_int("Enter another integer: ");
    println!("Sum of {a} and {b} is {}", a + b);

    let (x, y) = read_floats("Enter two floats separated by whitespace: ");
    println!("{x} + {y} = {}", x + y);

    let name = read_str("Enter your name: ");
    println!("Hello, {name}!");
}
```

## What it does

`askinput` is designed for ** interactive console programs** where a human types values by hand:

- educational exercises and lab assignments;
- command-line utilities with a dialogue interface;
- calculators, converters, small games;
- scripts and prototypes where concise code matters;
- any program where input is **individual values**, not a stream.

## What it does NOT do

`askinput` is **not** designed for:

- reading files — use `std::fs::read_to_string` or `BufReader`;
- reading large data streams — use `std::io::copy` or `BufRead::lines`;
- line-by-line log or CSV processing — use `csv`, `serde`, or `BufRead`;
- network sockets or pipes — use `tokio::io` or `std::net`;
- non-interactive input (redirection, `echo "5" | ./prog`) — works, but simpler tools exist;
- reading passwords without echo — use `rpassword` or `dialoguer`;
- GUI/TUI input — use `egui`, `ratatui`, or `czosterm`;
- safety-critical systems (avionics, medicine, banking) — this crate has no formal verification, no certification, and no redundancy. For such tasks use Ada, SPARK, or formal methods.

## Applicability boundaries

`askinput` is optimised for the case **"a human types by hand"**. This imposes hard limits:

- **One input line is limited to 4096 bytes.** This is far more than a person can type. If you need more, use `std::io` directly.
- **At most 2048 whitespace-separated tokens per line.** Protects against malicious 4 KiB lines with thousands of tokens.
- **Control characters are rejected** (except `\t`). This includes ESC — the entry point for ANSI escape sequences that can hijack a terminal.
- **`std` only.** No dependencies. No `no_std`. No WASM.

## Philosophy

`askinput` follows three principles:

1. **Paranoia at the input.** Everything that can be checked at the input layer is checked. Everything that cannot is documented as the caller's responsibility.
2. **Brevity at the call site.** One call — one value. No intermediate `match`, `unwrap`, or `expect`, in user code.
3. **Contract clarity.** What the library does is documented. What it does not do is also documented. No surprises.

## Status

Early version. API may change. Used in educational projects. For production use, see "What it does NOT do".

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT license ([LICENSE-MIT](LICENSE-MIT))

at your option.

## Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in the work by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any additional terms or conditions.
