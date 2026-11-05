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

/// Drops a trailing `#` or `//` comment.
///
/// Expressions are spliced into larger fend input, where a `#` would
/// comment out the rest of that input, so comments never reach fend.
pub(crate) fn strip_comment(line: &str) -> &str {
    let end = [line.find('#'), line.find("//")]
        .into_iter()
        .flatten()
        .min()
        .unwrap_or(line.len());
    line[..end].trim_end()
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

/// Rejects a number whose group separators do not form valid thousands
/// groups: `1,234,567` is fine; `1,5`, `1234,567` and `0.5,123` are not.
///
/// fend silently drops such separators (`1,5` becomes `15` in `en`), which
/// would show a confidently wrong result.
pub(crate) fn check_group_separators(expr: &str, group: char, decimal: char) -> Result<(), String> {
    let chars: Vec<char> = expr.chars().collect();
    let digit_at = |i: usize| chars.get(i).is_some_and(char::is_ascii_digit);
    let mut i = 0;
    while i < chars.len() {
        let starts_number =
            digit_at(i) && (i == 0 || !(chars[i - 1].is_alphanumeric() || chars[i - 1] == '_'));
        if !starts_number {
            i += 1;
            continue;
        }
        // A number: digits, plus separators that sit between two digits.
        let mut end = i;
        while digit_at(end) || ((chars[end] == group || chars[end] == decimal) && digit_at(end + 1))
        {
            end += 1;
            if end == chars.len() {
                break;
            }
        }
        let number = &chars[i..end];
        if number.contains(&group) && !is_grouped_number(number, group, decimal) {
            return Err(format!(
                "'{group}' groups thousands, so write numbers like 1{group}234{group}567"
            ));
        }
        i = end.max(i + 1);
    }
    Ok(())
}

/// `\d{1,3}(G\d{3})+(D\d+)?`
fn is_grouped_number(number: &[char], group: char, decimal: char) -> bool {
    let (int, frac) = match number.iter().position(|&c| c == decimal) {
        Some(p) => (&number[..p], Some(&number[p + 1..])),
        None => (number, None),
    };
    let mut groups = int.split(|&c| c == group);
    let first_ok = groups.next().is_some_and(|g| (1..=3).contains(&g.len()));
    let rest_ok = groups.all(|g| g.len() == 3 && g.iter().all(char::is_ascii_digit));
    let frac_ok = frac.is_none_or(|f| !f.is_empty() && f.iter().all(char::is_ascii_digit));
    first_ok && rest_ok && frac_ok
}

/// Rejects `=` anywhere except a single leading `name =`, which the caller
/// has already split off. An embedded assignment would bypass the reserved
/// name check (`1 + (m = 5)` would redefine metres).
pub(crate) fn check_no_assignment(expr: &str) -> Result<(), String> {
    let chars: Vec<char> = expr.chars().collect();
    for (i, &c) in chars.iter().enumerate() {
        let comparison = (i > 0 && "=<>!".contains(chars[i - 1])) || chars.get(i + 1) == Some(&'=');
        if c == '=' && !comparison {
            return Err("Only one assignment per line, at the start: name = …".into());
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

/// Hidden fend variable holding the most recent result, for `prev`.
pub(crate) const PREV_VAR: &str = "_prev";

/// Hidden fend variable holding the running total of the current block.
/// A running total, rather than one variable per line, keeps fend's
/// context (cloned once per line for the cache) the same size however long
/// the block gets.
pub(crate) const SUM_VAR: &str = "_sum";

/// Replaces `prev`, `sum`, `total` and `avg` (whole words only).
///
/// `has_prev` says whether an earlier line has a result; `block_len` is the
/// number of lines counted by totals in the current block, and `sum_valid`
/// is false once those lines turned out to have incompatible units.
pub(crate) fn rewrite_references(
    expr: &str,
    has_prev: bool,
    block_len: usize,
    sum_valid: bool,
) -> Result<References, String> {
    let mut out = String::with_capacity(expr.len());
    let mut aggregates = false;
    let mut word = String::new();
    let mut flush = |word: &mut String, out: &mut String| -> Result<(), String> {
        match word.as_str() {
            "prev" => {
                if !has_prev {
                    return Err("There is no earlier result for 'prev'".into());
                }
                out.push_str(PREV_VAR);
            }
            "sum" | "total" | "avg" => {
                if block_len == 0 {
                    return Err(format!("There is nothing above to '{word}'"));
                }
                if !sum_valid {
                    return Err("The lines above have units that cannot be added".into());
                }
                aggregates = true;
                if word == "avg" {
                    out.push_str(&format!("({SUM_VAR} / {block_len})"));
                } else {
                    out.push_str(SUM_VAR);
                }
            }
            w if w.starts_with('_') => {
                return Err(format!(
                    "'{w}' is reserved: names starting with '_' are internal"
                ));
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

/// True when the line uses `sum`, `total` or `avg` (whole words only).
fn uses_total(line: &str) -> bool {
    strip_comment(line)
        .split(|c: char| !(c.is_alphanumeric() || c == '_'))
        .any(|w| matches!(w, "sum" | "total" | "avg"))
}

/// For each line, whether a later line in the same block uses a total, so
/// this line's result must be added to the block's running total. Blocks
/// without a total skip that work entirely.
pub(crate) fn feeds_total(lines: &[&str]) -> Vec<bool> {
    let mut flags = vec![false; lines.len()];
    let mut total_below = false;
    for (i, line) in lines.iter().enumerate().rev() {
        match classify(line) {
            LineKind::Blank | LineKind::Header => total_below = false,
            LineKind::Comment => {}
            LineKind::Expr => {
                flags[i] = total_below;
                total_below |= uses_total(line);
            }
        }
    }
    flags
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

/// Rewrites `A + B%` to `(A) * (1 + B/100)` and `A - B%` likewise, and
/// `A as % of B` to `((A) / (B)) to %`, at every parenthesis level. `A` is everything before the operator at that level.
/// The percent must end its level, optionally followed by `in …`/`to …`.
pub(crate) fn rewrite_percentages(expr: &str) -> String {
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
    rewrite_as_percent_of(&out)
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
        let en = |e: &str| check_group_separators(e, ',', '.');
        let id = |e: &str| check_group_separators(e, '.', ',');
        assert!(en("4,500,000 IDR").is_ok());
        assert!(en("1,234,567.5").is_ok());
        assert!(en("1,5").is_err());
        assert!(en("1,5000").is_err());
        assert!(en("1234,567").is_err());
        assert!(en("0.5,123").is_err());
        assert!(en("f(a, b)").is_ok());
        assert!(en("x1,000").is_ok());
        assert!(id("4.500.000").is_ok());
        assert!(id("4.500.000,25").is_ok());
        assert!(id("1.5").is_err());
    }

    #[test]
    fn comments_and_embedded_assignments() {
        assert_eq!(strip_comment("2 # d"), "2");
        assert_eq!(strip_comment("3 // e # f"), "3");
        assert_eq!(strip_comment("4 / 2"), "4 / 2");
        assert!(check_no_assignment("1 + (z = 5)").is_err());
        assert!(check_no_assignment("a == b").is_ok());
        assert!(check_no_assignment("1 <= 2").is_ok());
        assert!(rewrite_references("_prev + 1", true, 0, true).is_err());
    }

    #[test]
    fn parens() {
        assert!(check_parens("(1 + 2").is_ok());
        assert!(check_parens("(1) + (2)").is_ok());
        assert!(check_parens("1 + 2) * 3").is_err());
    }

    #[test]
    fn percentage_rewrites() {
        assert_eq!(rewrite_percentages("50 + 10%"), "(50) * (1 + 10/100)");
        assert_eq!(rewrite_percentages("50 - 10%"), "(50) * (1 - 10/100)");
        assert_eq!(
            rewrite_percentages("2 * (50 + 10%)"),
            "2 * ((50) * (1 + 10/100))"
        );
        assert_eq!(
            rewrite_percentages("50 USD + 10% in IDR"),
            "(50 USD) * (1 + 10/100) in IDR"
        );
    }

    #[test]
    fn percentages_leave_other_percent_forms() {
        for expr in ["20% of 50", "10% * 3", "50 + 10% of 20", "-10%", "2 * -10%"] {
            assert_eq!(rewrite_percentages(expr), expr, "{expr}");
        }
    }

    #[test]
    fn as_percent_of() {
        assert_eq!(rewrite_as_percent_of("5 as % of 20"), "((5) / (20)) to %");
        assert_eq!(rewrite_as_percent_of("gas"), "gas");
        assert_eq!(rewrite_as_percent_of("as % of 20"), "as % of 20");
    }

    #[test]
    fn feeds_total_marks_lines_above_a_total_in_the_block() {
        let lines = ["1", "2", "// c", "sum", "3", "", "4", "# h", "5", "avg"];
        let expected = [
            true, true, false, false, false, false, false, false, true, false,
        ];
        assert_eq!(feeds_total(&lines), expected);
    }

    #[test]
    fn references() {
        let r = rewrite_references("20% of prev", true, 0, true).unwrap();
        assert_eq!(r.expr, "20% of _prev");
        assert!(!r.aggregates);
        let r = rewrite_references("avg", false, 2, true).unwrap();
        assert_eq!(r.expr, "(_sum / 2)");
        assert!(r.aggregates);
        assert!(rewrite_references("sum", false, 0, true).is_err());
        assert!(rewrite_references("sum", false, 2, false).is_err());
        assert!(rewrite_references("prev", false, 3, true).is_err());
        assert_eq!(
            rewrite_references("previous", false, 0, true).unwrap().expr,
            "previous"
        );
    }
}
