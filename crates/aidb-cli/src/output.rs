//! Output helpers: data goes to stdout, everything else to stderr.

use serde::Serialize;

/// Prints `value` as pretty JSON on stdout.
pub fn print_json<T: Serialize + ?Sized>(value: &T) {
    match serde_json::to_string_pretty(value) {
        Ok(json) => println!("{json}"),
        // Serialising our own types cannot fail; keep stdout clean if it does
        Err(e) => eprintln!("error: could not encode output as JSON: {e}"),
    }
}

/// Warning on stderr (never mixed into stdout data).
pub fn warn(message: &str) {
    eprintln!("warning: {message}");
}

/// Prints aligned `label  value` rows on stdout.
pub fn print_rows(rows: &[(&str, String)]) {
    let width = rows
        .iter()
        .map(|(label, _)| label.chars().count())
        .max()
        .unwrap_or(0);
    for (label, value) in rows {
        println!("{label:<width$}  {value}");
    }
}

/// `1234567` -> `1,234,567`
pub fn thousands(n: i64) -> String {
    let digits = n.unsigned_abs().to_string();
    let mut out = String::with_capacity(digits.len() + digits.len() / 3 + 1);
    for (i, c) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i) % 3 == 0 {
            out.push(',');
        }
        out.push(c);
    }
    if n < 0 {
        format!("-{out}")
    } else {
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn thousands_groups_digits() {
        assert_eq!(thousands(0), "0");
        assert_eq!(thousands(999), "999");
        assert_eq!(thousands(1000), "1,000");
        assert_eq!(thousands(123_456_789), "123,456,789");
        assert_eq!(thousands(-1_234), "-1,234");
    }
}
