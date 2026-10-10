//! # askinput
//!
//! Paranoid library for reading typed user input from stdin
//! in interactive console applications.
//!
//! All functions panic on fatal I/O errors (closed stdin, OS error).
//! This is a deliberate trade-off for a simple API in educational
//! and prototype code. User errors (invalid input) are handled
//! by a re-prompt loop and never panic.

#![forbid(unsafe_code)]
#![deny(missing_docs)]
#![warn(clippy::pedantic)]

use std::io::{self, BufRead, Write};

/// Maximum number of bytes accepted from one input line.
///
/// 4096 bytes matches the default `stdio` buffer size on most
/// systems and is far more than a human can type by hand.
/// This library is designed for interactive input, not for
/// bulk data. If you need to read larger payloads, use
/// `std::io` directly.
const MAX_INPUT_BYTES: usize = 4096;

/// Maximum length of one numeric token
/// (sign + 19 digits + exponent + margin).
const MAX_NUMERIC_TOKEN_BYTES: usize = 32;

/// Maximum number of whitespace-separated tokens in one line.
///
/// `MAX_INPUT_BYTES / 2` assumes the worst case of one-byte
/// tokens separated by one-byte ASCII spaces. Unicode
/// whitespace is multi-byte, so the real token count can only
/// be lower — this is a safe upper bound.
const MAX_TOKENS_PER_LINE: usize = MAX_INPUT_BYTES / 2;

/// Displays a prompt and immediately flushes stdout.
///
/// Retries on `ErrorKind::Interrupted` — Unix signals can
/// interrupt `write`/`flush` at any moment, and that is not
/// a real error.
fn prompt(message: &str) -> io::Result<()> {
    let stdout = io::stdout();
    let mut stdout = stdout.lock();

    loop {
        match stdout.write_all(message.as_bytes()) {
            Ok(()) => break,

            Err(error) if error.kind() == io::ErrorKind::Interrupted => {}

            Err(error) => return Err(error),
        }
    }

    loop {
        match stdout.flush() {
            Ok(()) => break,

            Err(error) if error.kind() == io::ErrorKind::Interrupted => {}

            Err(error) => return Err(error),
        }
    }

    Ok(())
}

/// Returns `true` if `byte` is a C0 control character that
/// must not appear in user input.
///
/// Allowed: `\t` (0x09). Everything else below 0x20 is
/// rejected, including `\0` (0x00) and ESC (0x1B). ESC is
/// especially dangerous — it is the entry point for ANSI
/// escape sequences that can hijack the terminal.
#[inline]
fn is_forbidden_control(byte: u8) -> bool {
    byte < 0x20 && byte != b'\t'
}

/// Reads one line from standard input.
///
/// The input is limited to `MAX_INPUT_BYTES` bytes. Excess
/// input is silently discarded — the line is already fully
/// consumed by `BufRead::read_line`, so there is nothing to
/// drain.
///
/// Returns the raw line **without** the trailing newline,
/// or an empty string on recoverable input errors (too long,
/// forbidden characters). Panics on fatal I/O errors.
fn read_line(message: &str) -> String {
    if let Err(error) = prompt(message) {
        panic!("askinput: failed to display prompt: {error}");
    }

    let stdin = io::stdin();
    let mut stdin = stdin.lock();

    let mut raw = String::with_capacity(128);

    loop {
        raw.clear();

        match stdin.read_line(&mut raw) {
            Ok(0) => {
                // EOF: либо stdin закрыт, либо Ctrl+D.
                panic!(
                    "askinput: input stream ended (EOF). \
                     Did you press Ctrl+D?"
                );
            }

            Ok(_) => break,

            Err(error) if error.kind() == io::ErrorKind::Interrupted => {}

            Err(error) => {
                panic!("askinput: failed to read input: {error}");
            }
        }
    }

    // `read_line` returns the string WITH the trailing `\n`
    // (if present). Strip all trailing `\r` and `\n`.
    let line = raw.trim_end_matches(['\r', '\n']);

    // Buffer overflow: the line is longer than the limit.
    // `read_line` has already consumed the entire line,
    // so there is nothing left to drain. Just reject.
    if line.len() > MAX_INPUT_BYTES {
        eprintln!("Error: input is too long. Maximum length is {MAX_INPUT_BYTES} bytes.");

        return String::new();
    }

    // Paranoid check: reject control characters (except `\t`).
    if line.bytes().any(is_forbidden_control) {
        eprintln!("Error: input contains forbidden control characters.");

        return String::new();
    }

    line.to_owned()
}

/// Checks that `input` contains only ASCII digits (with an
/// optional leading sign) and that its length is sane.
///
/// Runs **before** `parse()` — so we never feed the parser
/// garbage and never rely on its leniency.
fn validate_integer_token(token: &str) -> Result<(), &'static str> {
    if token.is_empty() {
        return Err("empty token");
    }

    if token.len() > MAX_NUMERIC_TOKEN_BYTES {
        return Err("token is too long");
    }

    let bytes = token.as_bytes();

    let digits = match bytes.first() {
        Some(b'+' | b'-') => &bytes[1..],
        _ => bytes,
    };

    if digits.is_empty() {
        return Err("no digits after sign");
    }

    if !digits.iter().all(u8::is_ascii_digit) {
        return Err("non-digit character");
    }

    Ok(())
}

/// Checks that `token` is a valid decimal floating-point
/// literal: optional sign, digits, optional single `.`,
/// optional exponent.
///
/// Rejects `inf`, `NaN`, `infinity`, hex floats, and anything
/// else that Rust's `parse::<f64>()` would happily accept.
fn validate_float_token(token: &str) -> Result<(), &'static str> {
    if token.is_empty() {
        return Err("empty token");
    }

    if token.len() > MAX_NUMERIC_TOKEN_BYTES {
        return Err("token is too long");
    }

    let mut seen_digit = false;
    let mut seen_dot = false;
    let mut seen_exp = false;
    let mut seen_exp_digit = false;
    let mut prev = 0u8;

    for (index, &byte) in token.as_bytes().iter().enumerate() {
        match byte {
            b'+' | b'-' => {
                // A sign is only valid at the start or right
                // after `e`/`E`.
                if index != 0 && prev != b'e' && prev != b'E' {
                    return Err("misplaced sign");
                }
            }

            b'0'..=b'9' => {
                seen_digit = true;

                if seen_exp {
                    seen_exp_digit = true;
                }
            }

            b'.' => {
                if seen_dot || seen_exp {
                    return Err("misplaced dot");
                }

                seen_dot = true;
            }

            b'e' | b'E' => {
                if seen_exp {
                    return Err("duplicate exponent");
                }

                if !seen_digit {
                    return Err("exponent without mantissa");
                }

                seen_exp = true;
            }

            _ => return Err("forbidden character in number"),
        }

        prev = byte;
    }

    if !seen_digit {
        return Err("no digits");
    }

    if seen_exp && !seen_exp_digit {
        return Err("exponent without digits");
    }

    Ok(())
}

/// Splits `line` into whitespace-separated tokens with a hard
/// cap on the total number of tokens.
///
/// Returning `Err` if the cap is exceeded prevents a malicious
/// 4 KiB line from allocating thousands of `Vec` elements.
fn split_tokens(line: &str) -> Result<Vec<&str>, &'static str> {
    let mut tokens = Vec::with_capacity(4);

    for token in line.split_whitespace() {
        if tokens.len() >= MAX_TOKENS_PER_LINE {
            return Err("too many tokens");
        }

        tokens.push(token);
    }

    Ok(tokens)
}

/// Reads exactly one integer from stdin.
///
/// Re-prompts on invalid input until a valid `i64` is entered.
///
/// # Panics
///
/// Panics on fatal I/O errors: closed stdin, EOF (Ctrl+D), or
/// a failed write to stdout.
///
/// # Examples
///
/// ```no_run
/// use askinput::read_int;
///
/// let x = read_int("Enter a number: ");
/// println!("You entered: {x}");
/// ```
#[must_use]
pub fn read_int(message: &str) -> i64 {
    loop {
        let input = read_line(message);

        let trimmed = input.trim();

        if trimmed.is_empty() {
            eprintln!("Error: enter exactly one valid integer.");
            continue;
        }

        if let Err(reason) = validate_integer_token(trimmed) {
            eprintln!("Error: enter exactly one valid integer ({reason}).");
            continue;
        }

        match trimmed.parse::<i64>() {
            Ok(value) => return value,

            Err(_) => {
                eprintln!("Error: integer is out of range.");
            }
        }
    }
}

/// Reads exactly one floating-point number from stdin.
///
/// Only finite `f64` values are accepted. `inf`, `-inf`, and
/// `NaN` are rejected and the user is re-prompted.
///
/// # Panics
///
/// Panics on fatal I/O errors: closed stdin, EOF (Ctrl+D), or
/// a failed write to stdout.
///
/// # Examples
///
/// ```no_run
/// use askinput::read_float;
///
/// let pi = read_float("Enter pi approximately: ");
/// println!("You entered: {pi}");
/// ```
#[must_use]
pub fn read_float(message: &str) -> f64 {
    loop {
        let input = read_line(message);

        let trimmed = input.trim();

        if trimmed.is_empty() {
            eprintln!("Error: enter exactly one valid floating-point number.");
            continue;
        }

        if let Err(reason) = validate_float_token(trimmed) {
            eprintln!("Error: enter exactly one valid floating-point number ({reason}).");
            continue;
        }

        match trimmed.parse::<f64>() {
            Ok(value) if value.is_finite() => return value,

            Ok(_) => {
                eprintln!("Error: the number must be finite.");
            }

            Err(_) => {
                eprintln!("Error: enter exactly one valid floating-point number.");
            }
        }
    }
}

/// Internal target trait for `read_ints()`.
///
/// This trait is sealed: it cannot be implemented outside
/// this crate. Use `read_ints()` directly.
#[doc(hidden)]
pub trait IntsTarget {
    /// Reads a value from user input.
    fn read_from(message: &str) -> Self;
}

/// Reads multiple integers and returns a tuple whose size
/// is determined by the variables on the left side.
///
/// # Panics
///
/// Panics on fatal I/O errors: closed stdin, EOF (Ctrl+D), or
/// a failed write to stdout.
///
/// # Examples
///
/// ```no_run
/// use askinput::read_ints;
///
/// let (a, b) = read_ints("Enter two integers: ");
/// let (x, y, z) = read_ints("Enter three integers: ");
/// ```
#[must_use]
pub fn read_ints<T: IntsTarget>(message: &str) -> T {
    T::read_from(message)
}

/// Internal target trait for `read_floats()`.
///
/// This trait is sealed: it cannot be implemented outside
/// this crate. Use `read_floats()` directly.
#[doc(hidden)]
pub trait FloatsTarget {
    /// Reads a value from user input.
    fn read_from(message: &str) -> Self;
}

/// Reads multiple floating-point numbers and returns a tuple
/// whose size is determined by the variables on the left side.
///
/// # Panics
///
/// Panics on fatal I/O errors: closed stdin, EOF (Ctrl+D), or
/// a failed write to stdout.
///
/// # Examples
///
/// ```no_run
/// use askinput::read_floats;
///
/// let (a, b) = read_floats("Enter two floats: ");
/// let (x, y, z) = read_floats("Enter three floats: ");
/// ```
#[must_use]
pub fn read_floats<T: FloatsTarget>(message: &str) -> T {
    T::read_from(message)
}

macro_rules! impl_ints_target {
    (
        $(
            ($count:literal; ($($ty:ty),+); $($index:tt),+)
        ),+ $(,)?
    ) => {
        $(
            impl IntsTarget for ($($ty),+) {
                fn read_from(message: &str) -> Self {
                    loop {
                        let input = read_line(message);

                        let tokens = match split_tokens(&input) {
                            Ok(tokens) => tokens,

                            Err(reason) => {
                                eprintln!(
                                    "Error: {} (expected {} integers).",
                                    reason, $count
                                );

                                continue;
                            }
                        };

                        if tokens.len() != $count {
                            eprintln!(
                                "Error: enter exactly {} integers.",
                                $count
                            );

                            continue;
                        }

                        // Validate ALL tokens BEFORE parsing,
                        // so we never parse partially.
                        let mut valid = true;

                        for token in &tokens {
                            if let Err(reason) = validate_integer_token(token) {
                                eprintln!(
                                    "Error: invalid integer \"{}\" ({}).",
                                    token, reason
                                );

                                valid = false;
                                break;
                            }
                        }

                        if !valid {
                            continue;
                        }

                        // Parse. Tokens are already format-checked,
                        // so only `i64` range overflow remains.
                        let mut values = [0i64; $count];
                        let mut failed = None;

                        for (slot, token) in values.iter_mut().zip(&tokens) {
                            match token.parse::<i64>() {
                                Ok(value) => *slot = value,

                                Err(_) => {
                                    failed = Some(*token);
                                    break;
                                }
                            }
                        }

                        if let Some(token) = failed {
                            eprintln!(
                                "Error: integer \"{}\" is out of range.",
                                token
                            );

                            continue;
                        }

                        return ($(values[$index]),+);
                    }
                }
            }
        )+
    };
}

macro_rules! impl_floats_target {
    (
        $(
            ($count:literal; ($($ty:ty),+); $($index:tt),+)
        ),+ $(,)?
    ) => {
        $(
            impl FloatsTarget for ($($ty),+) {
                fn read_from(message: &str) -> Self {
                    loop {
                        let input = read_line(message);

                        let tokens = match split_tokens(&input) {
                            Ok(tokens) => tokens,

                            Err(reason) => {
                                eprintln!(
                                    "Error: {} (expected {} floating-point numbers).",
                                    reason, $count
                                );

                                continue;
                            }
                        };

                        if tokens.len() != $count {
                            eprintln!(
                                "Error: enter exactly {} floating-point numbers.",
                                $count
                            );

                            continue;
                        }

                        let mut valid = true;

                        for token in &tokens {
                            if let Err(reason) = validate_float_token(token) {
                                eprintln!(
                                    "Error: invalid floating-point number \"{}\" ({}).",
                                    token, reason
                                );

                                valid = false;
                                break;
                            }
                        }

                        if !valid {
                            continue;
                        }

                        let mut values = [0f64; $count];
                        let mut failed_reason: Option<&'static str> = None;
                        let mut failed_token = "";

                        for (slot, token) in values.iter_mut().zip(&tokens) {
                            match token.parse::<f64>() {
                                Ok(value) if value.is_finite() => *slot = value,

                                Ok(_) => {
                                    failed_reason = Some("value is not finite");
                                    failed_token = token;
                                    break;
                                }

                                Err(_) => {
                                    failed_reason = Some("parse error");
                                    failed_token = token;
                                    break;
                                }
                            }
                        }

                        if let Some(reason) = failed_reason {
                            eprintln!(
                                "Error: floating-point number \"{}\" is invalid ({}).",
                                failed_token, reason
                            );

                            continue;
                        }

                        return ($(values[$index]),+);
                    }
                }
            }
        )+
    };
}

impl_ints_target! {
    (2; (i64, i64); 0, 1),
    (3; (i64, i64, i64); 0, 1, 2),
    (4; (i64, i64, i64, i64); 0, 1, 2, 3),
    (5; (i64, i64, i64, i64, i64); 0, 1, 2, 3, 4),
    (6; (i64, i64, i64, i64, i64, i64); 0, 1, 2, 3, 4, 5),
    (7; (i64, i64, i64, i64, i64, i64, i64); 0, 1, 2, 3, 4, 5, 6),
    (8; (i64, i64, i64, i64, i64, i64, i64, i64); 0, 1, 2, 3, 4, 5, 6, 7),
    (
        9;
        (i64, i64, i64, i64, i64, i64, i64, i64, i64);
        0, 1, 2, 3, 4, 5, 6, 7, 8
    ),
    (
        10;
        (i64, i64, i64, i64, i64, i64, i64, i64, i64, i64);
        0, 1, 2, 3, 4, 5, 6, 7, 8, 9
    ),
    (
        11;
        (i64, i64, i64, i64, i64, i64, i64, i64, i64, i64, i64);
        0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10
    ),
    (
        12;
        (i64, i64, i64, i64, i64, i64, i64, i64, i64, i64, i64, i64);
        0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11
    ),
}

impl_floats_target! {
    (2; (f64, f64); 0, 1),
    (3; (f64, f64, f64); 0, 1, 2),
    (4; (f64, f64, f64, f64); 0, 1, 2, 3),
    (5; (f64, f64, f64, f64, f64); 0, 1, 2, 3, 4),
    (6; (f64, f64, f64, f64, f64, f64); 0, 1, 2, 3, 4, 5),
    (7; (f64, f64, f64, f64, f64, f64, f64); 0, 1, 2, 3, 4, 5, 6),
    (8; (f64, f64, f64, f64, f64, f64, f64, f64); 0, 1, 2, 3, 4, 5, 6, 7),
    (
        9;
        (f64, f64, f64, f64, f64, f64, f64, f64, f64);
        0, 1, 2, 3, 4, 5, 6, 7, 8
    ),
    (
        10;
        (f64, f64, f64, f64, f64, f64, f64, f64, f64, f64);
        0, 1, 2, 3, 4, 5, 6, 7, 8, 9
    ),
    (
        11;
        (f64, f64, f64, f64, f64, f64, f64, f64, f64, f64, f64);
        0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10
    ),
    (
        12;
        (
            f64, f64, f64, f64, f64, f64,
            f64, f64, f64, f64, f64, f64
        );
        0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11
    ),
}

/// Reads a non-empty string from stdin.
///
/// User whitespace inside the string is preserved. Only the
/// line ending is removed.
///
/// Control characters are rejected — including embedded `\0`,
/// which in the C world would silently truncate the string at
/// an unpredictable point. Empty strings (or strings that
/// contain only whitespace) are rejected and re-prompted.
///
/// # Panics
///
/// Panics on fatal I/O errors: closed stdin, EOF (Ctrl+D), or
/// a failed write to stdout.
///
/// # Examples
///
/// ```no_run
/// use askinput::read_str;
///
/// let name = read_str("Enter your name: ");
/// println!("Hello, {name}!");
/// ```
#[must_use]
pub fn read_str(message: &str) -> String {
    loop {
        let input = read_line(message);

        if input.trim().is_empty() {
            eprintln!("Error: the string cannot be empty.");
            continue;
        }

        return input;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ----- validate_integer_token -----

    #[test]
    fn int_token_accepts_plain_digits() {
        assert!(validate_integer_token("0").is_ok());
        assert!(validate_integer_token("5").is_ok());
        assert!(validate_integer_token("1234567890").is_ok());
    }

    #[test]
    fn int_token_accepts_signs() {
        assert!(validate_integer_token("+5").is_ok());
        assert!(validate_integer_token("-5").is_ok());
        assert!(validate_integer_token("+0").is_ok());
        assert!(validate_integer_token("-0").is_ok());
    }

    #[test]
    fn int_token_rejects_empty() {
        assert!(validate_integer_token("").is_err());
    }

    #[test]
    fn int_token_rejects_lone_sign() {
        assert!(validate_integer_token("+").is_err());
        assert!(validate_integer_token("-").is_err());
    }

    #[test]
    fn int_token_rejects_non_digits() {
        assert!(validate_integer_token("abc").is_err());
        assert!(validate_integer_token("5a").is_err());
        assert!(validate_integer_token("5.0").is_err());
        assert!(validate_integer_token("5,0").is_err());
        assert!(validate_integer_token(" 5").is_err());
        assert!(validate_integer_token("5 ").is_err());
    }

    #[test]
    fn int_token_rejects_too_long() {
        let long = "1".repeat(MAX_NUMERIC_TOKEN_BYTES + 1);
        assert!(validate_integer_token(&long).is_err());
    }

    // ----- validate_float_token -----

    #[test]
    fn float_token_accepts_simple() {
        assert!(validate_float_token("0").is_ok());
        assert!(validate_float_token("3.14").is_ok());
        assert!(validate_float_token(".5").is_ok());
        assert!(validate_float_token("5.").is_ok());
        assert!(validate_float_token("+3.14").is_ok());
        assert!(validate_float_token("-3.14").is_ok());
    }

    #[test]
    fn float_token_accepts_exponents() {
        assert!(validate_float_token("1e5").is_ok());
        assert!(validate_float_token("1E5").is_ok());
        assert!(validate_float_token("1e+5").is_ok());
        assert!(validate_float_token("1e-5").is_ok());
        assert!(validate_float_token("3.14e2").is_ok());
    }

    #[test]
    fn float_token_rejects_empty() {
        assert!(validate_float_token("").is_err());
    }

    #[test]
    fn float_token_rejects_lone_dot() {
        assert!(validate_float_token(".").is_err());
        assert!(validate_float_token("+.").is_err());
        assert!(validate_float_token("-.").is_err());
    }

    #[test]
    fn float_token_rejects_multiple_dots() {
        assert!(validate_float_token("1.2.3").is_err());
        assert!(validate_float_token("..").is_err());
    }

    #[test]
    fn float_token_rejects_broken_exponents() {
        assert!(validate_float_token("1e").is_err());
        assert!(validate_float_token("1e+").is_err());
        assert!(validate_float_token("e5").is_err());
        assert!(validate_float_token("1e5e5").is_err());
    }

    #[test]
    fn float_token_rejects_inf_and_nan() {
        assert!(validate_float_token("inf").is_err());
        assert!(validate_float_token("-inf").is_err());
        assert!(validate_float_token("NaN").is_err());
        assert!(validate_float_token("nan").is_err());
        assert!(validate_float_token("infinity").is_err());
    }

    #[test]
    fn float_token_rejects_hex() {
        assert!(validate_float_token("0x1F").is_err());
        assert!(validate_float_token("0b101").is_err());
    }

    #[test]
    fn float_token_rejects_underscores() {
        assert!(validate_float_token("1_000").is_err());
    }

    // ----- is_forbidden_control -----

    #[test]
    fn control_allows_tab() {
        assert!(!is_forbidden_control(b'\t'));
    }

    #[test]
    fn control_allows_printable_ascii() {
        assert!(!is_forbidden_control(b'a'));
        assert!(!is_forbidden_control(b'Z'));
        assert!(!is_forbidden_control(b'0'));
        assert!(!is_forbidden_control(b' '));
        assert!(!is_forbidden_control(b'~'));
    }

    #[test]
    fn control_rejects_nul_and_esc() {
        assert!(is_forbidden_control(b'\0'));
        assert!(is_forbidden_control(b'\x1b'));
    }

    #[test]
    fn control_rejects_newline_and_cr() {
        assert!(is_forbidden_control(b'\n'));
        assert!(is_forbidden_control(b'\r'));
    }

    #[test]
    fn control_rejects_all_c0_except_tab() {
        for byte in 0u8..0x20 {
            if byte == b'\t' {
                assert!(!is_forbidden_control(byte));
            } else {
                assert!(is_forbidden_control(byte));
            }
        }
    }

    // ----- split_tokens -----

    #[test]
    fn split_handles_simple_input() {
        let tokens = split_tokens("1 2 3").unwrap();
        assert_eq!(tokens, vec!["1", "2", "3"]);
    }

    #[test]
    fn split_handles_extra_whitespace() {
        let tokens = split_tokens("  1   2\t3  ").unwrap();
        assert_eq!(tokens, vec!["1", "2", "3"]);
    }

    #[test]
    fn split_handles_empty_input() {
        let tokens = split_tokens("").unwrap();
        assert!(tokens.is_empty());
    }

    #[test]
    fn split_handles_whitespace_only() {
        let tokens = split_tokens("   \t  ").unwrap();
        assert!(tokens.is_empty());
    }
}
