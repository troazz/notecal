//! Rewrites Numi-style syntax into expressions fend-core understands.
//!
//! fend already handles arithmetic, units, number bases, `20% of 50`,
//! variables and thousands separators (see the Task 2 findings in the M1
//! plan). This module only covers what fend gets wrong or does not know:
//! line kinds, `prev`/`sum`/`total`/`avg`, `A ± B%`, `A as % of B`,
//! reserved variable names, and ambiguous group separators.

/// What a line is, before any evaluation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LineKind {
    /// Empty or whitespace only. Ends the current block.
    Blank,
    /// Starts with `#`. Ends the current block.
    Header,
    /// Starts with `//`. Does not end the block.
    Comment,
    /// Anything else is evaluated.
    Expr,
}

pub(crate) fn classify(line: &str) -> LineKind {
    let t = line.trim();
    if t.is_empty() {
        LineKind::Blank
    } else if t.starts_with('#') {
        LineKind::Header
    } else if t.starts_with("//") {
        LineKind::Comment
    } else {
        LineKind::Expr
    }
}

/// Words with a meaning in notecal or fend that a variable may not take.
/// Built-in units, functions and constants are rejected separately by
/// asking fend whether the name already evaluates.
pub(crate) const KEYWORDS: &[&str] = &[
    "prev", "sum", "total", "avg", "in", "to", "of", "as", "mod", "per",
];

/// Splits `name = expr` into its parts. Returns `None` for anything that is
/// not a plain assignment (no `=`, `==`, or a left side that is not a name).
pub(crate) fn split_assignment(line: &str) -> Option<(&str, &str)> {
    let (lhs, rhs) = line.split_once('=')?;
    if rhs.starts_with('=') {
        return None;
    }
    let name = lhs.trim();
    let mut chars = name.chars();
    let first = chars.next()?;
    let is_name =
        (first.is_alphabetic() || first == '_') && chars.all(|c| c.is_alphanumeric() || c == '_');
    is_name.then(|| (name, rhs.trim()))
}

/// Rejects a group separator that is not followed by exactly three digits.
///
/// fend silently drops such separators (`1,5` becomes `15` in `en`), which
/// would show a confidently wrong result.
pub(crate) fn check_group_separators(expr: &str, group: char) -> Result<(), String> {
    let chars: Vec<char> = expr.chars().collect();
    for (i, &c) in chars.iter().enumerate() {
        if c != group {
            continue;
        }
        let digit_before = i > 0 && chars[i - 1].is_ascii_digit();
        let digit_after = chars.get(i + 1).is_some_and(char::is_ascii_digit);
        if !(digit_before && digit_after) {
            continue;
        }
        let run = chars[i + 1..]
            .iter()
            .take_while(|c| c.is_ascii_digit())
            .count();
        if run != 3 {
            return Err(format!(
                "'{group}' separates thousands here, so it must be followed by exactly three digits"
            ));
        }
    }
    Ok(())
}

/// Rejects a `)` without a matching `(`.
///
/// Each expression is wrapped in parentheses before fend sees it, so a stray
/// `)` would close the wrapper and silently change the meaning. An unclosed
/// `(` is fine: fend closes it, which keeps results live while typing.
pub(crate) fn check_parens(expr: &str) -> Result<(), String> {
    let mut depth = 0usize;
    for c in expr.chars() {
        match c {
            '(' => depth += 1,
            ')' => depth = depth.checked_sub(1).ok_or("Unmatched ')'")?,
            _ => {}
        }
    }
    Ok(())
}

/// Result of replacing line references with hidden result variables.
pub(crate) struct References {
    pub expr: String,
    /// The line uses `sum`, `total` or `avg`, so later totals skip it.
    pub aggregates: bool,
}

/// Name of the hidden fend variable that holds line `index`'s result.
pub(crate) fn result_var(index: usize) -> String {
    format!("_r{index}")
}

/// Replaces `prev`, `sum`, `total` and `avg` (whole words only).
///
/// `prev_line` is the nearest earlier line with a result; `block` lists the
/// lines counted by totals in the current block.
pub(crate) fn rewrite_references(
    expr: &str,
    prev_line: Option<usize>,
    block: &[usize],
) -> Result<References, String> {
    let mut out = String::with_capacity(expr.len());
    let mut aggregates = false;
    let mut word = String::new();
    let mut flush = |word: &mut String, out: &mut String| -> Result<(), String> {
        match word.as_str() {
            "prev" => {
                let line = prev_line.ok_or("There is no earlier result for 'prev'")?;
                out.push_str(&result_var(line));
            }
            "sum" | "total" | "avg" => {
                if block.is_empty() {
                    return Err(format!("There is nothing above to '{word}'"));
                }
                aggregates = true;
                let terms: Vec<String> = block.iter().map(|&i| result_var(i)).collect();
                let sum = terms.join(" + ");
                if word == "avg" {
                    out.push_str(&format!("(({sum}) / {})", block.len()));
                } else {
                    out.push_str(&format!("({sum})"));
                }
            }
            _ => out.push_str(word),
        }
        word.clear();
        Ok(())
    };
    for c in expr.chars() {
        if c.is_alphanumeric() || c == '_' {
            word.push(c);
        } else {
            flush(&mut word, &mut out)?;
            out.push(c);
        }
    }
    flush(&mut word, &mut out)?;
    Ok(References {
        expr: out,
        aggregates,
    })
}

/// Rewrites `A as % of B` to `((A) / (B)) to %`.
pub(crate) fn rewrite_as_percent_of(expr: &str) -> String {
    let lower = expr.to_ascii_lowercase();
    let mut from = 0;
    while let Some(found) = lower[from..].find("as") {
        let start = from + found;
        from = start + 2;
        let word_start = start == 0 || !is_word_char(lower[..start].chars().next_back());
        if !word_start {
            continue;
        }
        let rest = lower[start + 2..].trim_start();
        let Some(rest) = rest.strip_prefix('%') else {
            continue;
        };
        let rest = rest.trim_start();
        let Some(after_of) = rest.strip_prefix("of") else {
            continue;
        };
        if is_word_char(after_of.chars().next()) {
            continue;
        }
        let left = expr[..start].trim();
        let right = expr[expr.len() - after_of.len()..].trim();
        if left.is_empty() || right.is_empty() {
            return expr.to_string();
        }
        return format!("(({left}) / ({right})) to %");
    }
    expr.to_string()
}

fn is_word_char(c: Option<char>) -> bool {
    c.is_some_and(|c| c.is_alphanumeric() || c == '_')
}

#[derive(Debug, PartialEq)]
enum Tok {
    Open,
    Close,
    Op(char),
    Text(String),
}

fn tokenize(expr: &str) -> Vec<Tok> {
    let mut toks = Vec::new();
    let mut text = String::new();
    for c in expr.chars() {
        let tok = match c {
            '(' => Tok::Open,
            ')' => Tok::Close,
            '+' | '-' => Tok::Op(c),
            _ => {
                text.push(c);
                continue;
            }
        };
        if !text.is_empty() {
            toks.push(Tok::Text(std::mem::take(&mut text)));
        }
        toks.push(tok);
    }
    if !text.is_empty() {
        toks.push(Tok::Text(text));
    }
    toks
}

/// Rewrites `A + B%` to `(A) * (1 + B/100)` and `A - B%` likewise, at every
/// parenthesis level. `A` is everything before the operator at that level.
/// The percent must end its level, optionally followed by `in …`/`to …`.
pub(crate) fn rewrite_percent_change(expr: &str) -> String {
    let toks = tokenize(expr);
    let mut pos = 0;
    let mut out = rewrite_level(&toks, &mut pos);
    // Unbalanced `)`: keep the rest verbatim and let fend report it.
    while pos < toks.len() {
        out.push_str(&render(&toks[pos]));
        pos += 1;
        out.push_str(&rewrite_level(&toks, &mut pos));
    }
    out
}

fn render(tok: &Tok) -> String {
    match tok {
        Tok::Open => "(".into(),
        Tok::Close => ")".into(),
        Tok::Op(c) => c.to_string(),
        Tok::Text(t) => t.clone(),
    }
}

fn rewrite_level(toks: &[Tok], pos: &mut usize) -> String {
    let mut out = String::new();
    while let Some(tok) = toks.get(*pos) {
        match tok {
            Tok::Close => break,
            Tok::Open => {
                *pos += 1;
                let inner = rewrite_level(toks, pos);
                out.push('(');
                out.push_str(&inner);
                if toks.get(*pos) == Some(&Tok::Close) {
                    out.push(')');
                    *pos += 1;
                }
            }
            Tok::Op(op) => {
                let percent = match toks.get(*pos + 1) {
                    Some(Tok::Text(t)) if ends_level(toks.get(*pos + 2)) => percent_operand(t),
                    _ => None,
                };
                match percent {
                    Some((value, suffix)) if is_binary(&out) => {
                        out = format!("({}) * (1 {op} {value}/100){suffix}", out.trim());
                        *pos += 2;
                    }
                    _ => {
                        out.push(*op);
                        *pos += 1;
                    }
                }
            }
            Tok::Text(t) => {
                out.push_str(t);
                *pos += 1;
            }
        }
    }
    out
}

fn ends_level(next: Option<&Tok>) -> bool {
    matches!(next, None | Some(Tok::Close | Tok::Op(_)))
}

/// Matches ` 10%` or ` 10 % in IDR`; returns the number and any suffix.
fn percent_operand(text: &str) -> Option<(&str, &str)> {
    let t = text.trim_start();
    let digits = t
        .find(|c: char| !(c.is_ascii_digit() || c == '.' || c == ','))
        .unwrap_or(t.len());
    if digits == 0 || !t.as_bytes()[0].is_ascii_digit() {
        return None;
    }
    let (number, rest) = t.split_at(digits);
    let rest = rest.trim_start().strip_prefix('%')?;
    let suffix = rest.trim();
    let is_conversion = suffix.is_empty()
        || suffix
            .split_whitespace()
            .next()
            .is_some_and(|w| w.eq_ignore_ascii_case("in") || w.eq_ignore_ascii_case("to"));
    is_conversion.then_some((number, rest))
}

/// True when the text so far ends in an operand, so a following `+`/`-` is
/// binary rather than a sign.
fn is_binary(left: &str) -> bool {
    left.trim_end()
        .chars()
        .next_back()
        .is_some_and(|c| !"+-*/^(=,×÷".contains(c))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_lines() {
        assert_eq!(classify("   "), LineKind::Blank);
        assert_eq!(classify("# Budget"), LineKind::Header);
        assert_eq!(classify("  // note"), LineKind::Comment);
        assert_eq!(classify("1 + 1"), LineKind::Expr);
    }

    #[test]
    fn splits_assignments() {
        assert_eq!(split_assignment("rent = 5 IDR"), Some(("rent", "5 IDR")));
        assert_eq!(split_assignment("a == b"), None);
        assert_eq!(split_assignment("2 * a = 3"), None);
        assert_eq!(split_assignment("5 km"), None);
    }

    #[test]
    fn group_separators_need_three_digits() {
        assert!(check_group_separators("4,500,000 IDR", ',').is_ok());
        assert!(check_group_separators("1,5", ',').is_err());
        assert!(check_group_separators("1,5000", ',').is_err());
        assert!(check_group_separators("f(a, b)", ',').is_ok());
        assert!(check_group_separators("4.500.000", '.').is_ok());
        assert!(check_group_separators("1.5", '.').is_err());
    }

    #[test]
    fn parens() {
        assert!(check_parens("(1 + 2").is_ok());
        assert!(check_parens("(1) + (2)").is_ok());
        assert!(check_parens("1 + 2) * 3").is_err());
    }

    #[test]
    fn percent_change_rewrites() {
        assert_eq!(rewrite_percent_change("50 + 10%"), "(50) * (1 + 10/100)");
        assert_eq!(rewrite_percent_change("50 - 10%"), "(50) * (1 - 10/100)");
        assert_eq!(
            rewrite_percent_change("2 * (50 + 10%)"),
            "2 * ((50) * (1 + 10/100))"
        );
        assert_eq!(
            rewrite_percent_change("50 USD + 10% in IDR"),
            "(50 USD) * (1 + 10/100) in IDR"
        );
    }

    #[test]
    fn percent_change_leaves_other_percent_forms() {
        for expr in ["20% of 50", "10% * 3", "50 + 10% of 20", "-10%", "2 * -10%"] {
            assert_eq!(rewrite_percent_change(expr), expr, "{expr}");
        }
    }

    #[test]
    fn as_percent_of() {
        assert_eq!(rewrite_as_percent_of("5 as % of 20"), "((5) / (20)) to %");
        assert_eq!(rewrite_as_percent_of("gas"), "gas");
        assert_eq!(rewrite_as_percent_of("as % of 20"), "as % of 20");
    }

    #[test]
    fn references() {
        let r = rewrite_references("20% of prev", Some(3), &[]).unwrap();
        assert_eq!(r.expr, "20% of _r3");
        assert!(!r.aggregates);
        let r = rewrite_references("avg", None, &[1, 2]).unwrap();
        assert_eq!(r.expr, "((_r1 + _r2) / 2)");
        assert!(r.aggregates);
        assert!(rewrite_references("sum", None, &[]).is_err());
        assert_eq!(
            rewrite_references("previous", None, &[]).unwrap().expr,
            "previous"
        );
    }
}
