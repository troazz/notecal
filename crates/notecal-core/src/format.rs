//! Turns fend's result text into what the result column shows and what a
//! bare-number copy puts on the clipboard.

use crate::{Locale, Rates, Settings};

/// Integer parts longer than this switch to scientific notation.
const MAX_INTEGER_DIGITS: usize = 15;

pub(crate) struct Formatted {
    pub display: String,
    pub copy: String,
}

/// Formats fend's main result.
///
/// `verbatim` is set when the user asked for a number base (`in hex`), whose
/// digits must not be grouped or rounded.
pub(crate) fn format_result(
    raw: &str,
    settings: &Settings,
    rates: Option<&Rates>,
    verbatim: bool,
) -> Formatted {
    let text = raw.strip_prefix("approx. ").unwrap_or(raw);
    let as_is = || Formatted {
        display: text.to_string(),
        copy: text.to_string(),
    };
    if verbatim || ["0x", "0b", "0o"].iter().any(|p| text.starts_with(p)) {
        return as_is();
    }

    let (decimal, group) = match settings.locale {
        Locale::En => ('.', ','),
        Locale::Id => (',', '.'),
    };
    let (negative, unsigned) = match text.strip_prefix('-') {
        Some(rest) => (true, rest),
        None => (false, text),
    };
    let int_len = unsigned
        .find(|c: char| !c.is_ascii_digit())
        .unwrap_or(unsigned.len());
    if int_len == 0 {
        return as_is();
    }
    let (int, rest) = unsigned.split_at(int_len);
    let (frac, unit) = match rest.strip_prefix(decimal) {
        Some(after) if after.starts_with(|c: char| c.is_ascii_digit()) => {
            let len = after
                .find(|c: char| !c.is_ascii_digit())
                .unwrap_or(after.len());
            after.split_at(len)
        }
        _ => ("", rest),
    };

    let currency = is_currency(unit.trim(), rates);
    let places = if currency {
        2
    } else {
        usize::from(settings.precision)
    };
    let nonzero = int.bytes().chain(frac.bytes()).any(|b| b != b'0');
    let (r_int, mut r_frac) = round_decimal(int, frac, places);
    if currency {
        if r_frac.bytes().all(|b| b == b'0') {
            r_frac.clear();
        } else {
            while r_frac.len() < places {
                r_frac.push('0');
            }
        }
    } else {
        r_frac.truncate(r_frac.trim_end_matches('0').len());
    }
    let r_int = r_int.trim_start_matches('0');
    let r_int = if r_int.is_empty() { "0" } else { r_int };
    let rounds_to_zero = r_int == "0" && r_frac.is_empty();

    // Tiny non-zero values switch to scientific notation so they do not read
    // as zero; currencies round to 0 like any amount below a cent.
    let scientific_form =
        r_int.len() > MAX_INTEGER_DIGITS || (rounds_to_zero && nonzero && !currency);
    let sign = if negative && (scientific_form || !rounds_to_zero) {
        "-"
    } else {
        ""
    };
    let (display_number, copy_number) = if scientific_form {
        let sci = scientific(int, frac, places.max(1), decimal);
        (sci.clone(), sci)
    } else {
        let join = |int: String| {
            if r_frac.is_empty() {
                int
            } else {
                format!("{int}{decimal}{r_frac}")
            }
        };
        (join(group_digits(r_int, group)), join(r_int.to_string()))
    };
    Formatted {
        display: format!("{sign}{display_number}{unit}"),
        copy: format!("{sign}{copy_number}"),
    }
}

fn is_currency(unit: &str, rates: Option<&Rates>) -> bool {
    unit.len() == 3
        && unit.bytes().all(|b| b.is_ascii_uppercase())
        && rates.is_some_and(|r| r.get(unit).is_some())
}

/// Rounds a decimal given as digit strings to `places` fraction digits,
/// half away from zero. Works on digits, so no precision is lost.
fn round_decimal(int: &str, frac: &str, places: usize) -> (String, String) {
    if frac.len() <= places {
        return (int.to_string(), frac.to_string());
    }
    let mut digits: Vec<u8> = int.bytes().chain(frac.bytes().take(places)).collect();
    if frac.as_bytes()[places] >= b'5' {
        let mut i = digits.len();
        loop {
            if i == 0 {
                digits.insert(0, b'1');
                break;
            }
            i -= 1;
            if digits[i] == b'9' {
                digits[i] = b'0';
            } else {
                digits[i] += 1;
                break;
            }
        }
    }
    let split = digits.len() - places;
    let frac = digits.split_off(split);
    (bytes_to_string(digits), bytes_to_string(frac))
}

fn bytes_to_string(bytes: Vec<u8>) -> String {
    bytes.into_iter().map(char::from).collect()
}

fn group_digits(int: &str, group: char) -> String {
    let mut out = String::with_capacity(int.len() + int.len() / 3);
    for (i, c) in int.chars().enumerate() {
        if i > 0 && (int.len() - i) % 3 == 0 {
            out.push(group);
        }
        out.push(c);
    }
    out
}

/// `1.5e-8` style, with up to `places` mantissa decimals.
fn scientific(int: &str, frac: &str, places: usize, decimal: char) -> String {
    let all: String = format!("{int}{frac}");
    let Some(first) = all.find(|c| c != '0') else {
        return "0".into();
    };
    let mut exponent = int.len() as i64 - 1 - first as i64;
    let significant = &all[first..];
    let (mut lead, mut rest) = round_decimal(&significant[..1], &significant[1..], places);
    if lead.len() > 1 {
        // 9.99… rounded up to 10.
        lead = "1".into();
        rest = "0".repeat(places);
        exponent += 1;
    }
    let rest = rest.trim_end_matches('0');
    if rest.is_empty() {
        format!("{lead}e{exponent}")
    } else {
        format!("{lead}{decimal}{rest}e{exponent}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn rates() -> Rates {
        Rates::from_per_usd(HashMap::from([("IDR".to_string(), 16000.0)]))
    }

    fn display(raw: &str) -> String {
        format_result(raw, &Settings::default(), Some(&rates()), false).display
    }

    #[test]
    fn groups_and_trims() {
        assert_eq!(display("4500000 IDR"), "4,500,000 IDR");
        assert_eq!(display("approx. 3.1068559612 miles"), "3.106856 miles");
        assert_eq!(display("2.5"), "2.5");
        assert_eq!(display("-1234.5"), "-1,234.5");
        assert_eq!(display("25%"), "25%");
    }

    #[test]
    fn currency_uses_two_decimals_only_when_needed() {
        assert_eq!(display("approx. 1234.5678 IDR"), "1,234.57 IDR");
        assert_eq!(display("1060000 IDR"), "1,060,000 IDR");
        assert_eq!(display("approx. 0.999 IDR"), "1 IDR");
        assert_eq!(display("1234.5 IDR"), "1,234.50 IDR");
    }

    #[test]
    fn rounding_carries() {
        assert_eq!(display("approx. 0.9999999"), "1");
        assert_eq!(display("approx. 9.9999999"), "10");
    }

    #[test]
    fn extremes_use_scientific() {
        assert_eq!(display("approx. 0.00000001234"), "1.234e-8");
        assert_eq!(display("1000000000000000000000"), "1e21");
    }

    #[test]
    fn id_locale() {
        let settings = Settings {
            locale: Locale::Id,
            precision: 6,
        };
        let f = format_result("approx. 1234567,3333333333", &settings, None, false);
        assert_eq!(f.display, "1.234.567,333333");
        assert_eq!(f.copy, "1234567,333333");
    }

    #[test]
    fn copy_is_bare() {
        let f = format_result("4500000 IDR", &Settings::default(), Some(&rates()), false);
        assert_eq!(f.copy, "4500000");
    }

    #[test]
    fn bases_are_verbatim() {
        assert_eq!(display("0xff"), "0xff");
        let f = format_result("11111111", &Settings::default(), None, true);
        assert_eq!(f.display, "11111111");
    }
}
