//! Dates, times and time zones (PRD C11).
//!
//! fend-core 1.5.9 parses `today` but cannot evaluate it (its current-time
//! setter is a no-op), and it has no time zones. Date lines are therefore
//! recognised and evaluated here, before fend sees them:
//!
//! - `today`, `now`, `time`, `tomorrow`, `yesterday`, ISO dates
//!   (`2026-12-25`), month names (`25 Dec 2026`, `Dec 25`), times (`3pm`,
//!   `15:30`, `noon`), date variables and `prev` after a date line;
//! - `+`/`-` durations in calendar units (`+ 1 month` keeps the day of the
//!   month, clamped at the month's end);
//! - `date - date [in unit]`, handed back to fend as a duration so it is
//!   formatted like any other quantity;
//! - `<time> [zone] in <zone>` conversions.
//!
//! A line that does not start like a date is left to fend untouched. A line
//! that starts with a weak date form (a bare time or month name, which fend
//! might read as `3 pm` picometres or a unit) also falls back to fend when it
//! does not parse as a date; one that starts with a keyword or an ISO date
//! reports the date error instead.

mod zones;

use std::collections::{HashMap, HashSet};

use jiff::civil::{Date, Time};
use jiff::tz::TimeZone;
use jiff::{Span, ZonedDateTime};

/// A date or a moment, in a time zone.
#[derive(Debug, Clone)]
pub(crate) struct DateValue {
    zdt: ZonedDateTime,
    /// False for a day (`today`, `2026-12-25`): shown and subtracted as a date.
    has_time: bool,
    /// The value was converted with `in <zone>`: show the zone's abbreviation.
    zone_shown: bool,
}

/// What a date line evaluates to.
#[derive(Debug)]
pub(crate) enum Outcome {
    /// A date or time, formatted here.
    Value(DateValue),
    /// A duration (`date - date`) for fend to evaluate and format.
    Fend(String),
}

/// A date line's outcome and whether it depends on the clock.
#[derive(Debug)]
pub(crate) struct Evaluated {
    pub(crate) outcome: Outcome,
    /// The result changes as time passes (`now`, `today`, `3pm`, `Dec 25`).
    pub(crate) uses_time: bool,
}

/// Date-related state carried from line to line of a note.
#[derive(Debug, Clone, Default)]
pub(crate) struct DateState {
    vars: HashMap<String, DateValue>,
    prev: Option<DateValue>,
    /// Every name the note has assigned so far. A user variable called `time`
    /// or `now` keeps its meaning instead of becoming a date keyword.
    names: HashSet<String>,
}

impl DateState {
    /// Records the result of a date line, stored in `name` if it assigns one.
    pub(crate) fn set_date(&mut self, name: Option<&str>, value: &DateValue) {
        if let Some(name) = name {
            self.names.insert(name.to_string());
            self.vars.insert(name.to_string(), value.clone());
        }
        self.prev = Some(value.clone());
    }

    /// Records a line with a non-date result: `prev` and `name` are numbers now.
    pub(crate) fn set_other(&mut self, name: Option<&str>) {
        if let Some(name) = name {
            self.names.insert(name.to_string());
            self.vars.remove(name);
        }
        self.prev = None;
    }
}

/// Evaluates `expr` as a date line, or returns `None` if it isn't one.
pub(crate) fn evaluate(
    expr: &str,
    now: &ZonedDateTime,
    state: &DateState,
) -> Option<Result<Evaluated, String>> {
    let tokens = tokenize(expr);
    let mut parser = Parser {
        src: expr,
        tokens: &tokens,
        pos: 0,
        now,
        state,
        uses_time: false,
    };
    let (first, strong) = parser.term()?;
    let result = first.and_then(|value| parser.rest(value));
    match result {
        Ok(outcome) => Some(Ok(Evaluated {
            outcome,
            uses_time: parser.uses_time,
        })),
        Err(message) if strong => Some(Err(message)),
        Err(_) => None,
    }
}

/// Text for the result column and for copying.
pub(crate) fn format(value: &DateValue, now: &ZonedDateTime) -> (String, String) {
    let zdt = &value.zdt;
    if !value.has_time {
        return (
            zdt.strftime("%a, %-d %b %Y").to_string(),
            zdt.strftime("%Y-%m-%d").to_string(),
        );
    }
    let copy = zdt.strftime("%Y-%m-%d %H:%M").to_string();
    // Compared with the local date: `now in New York` late on the previous
    // evening there shows its weekday.
    let same_day = zdt.date() == now.date();
    let mut display = if same_day {
        zdt.strftime("%H:%M").to_string()
    } else if value.zone_shown {
        zdt.strftime("%a %H:%M").to_string()
    } else {
        zdt.strftime("%a, %-d %b %Y %H:%M").to_string()
    };
    if value.zone_shown {
        let info = zdt.time_zone().to_offset_info(zdt.timestamp());
        display.push(' ');
        display.push_str(info.abbreviation());
    }
    (display, copy)
}

#[derive(Debug, Clone, PartialEq)]
enum Kind {
    Word,
    Number,
    /// `2026-12-25`
    IsoDate,
    /// `15:30`
    Clock,
    /// `3pm`, `3:30am`
    AmPm,
    Op(char),
}

#[derive(Debug, Clone)]
struct Token {
    kind: Kind,
    /// Lower-cased text.
    text: String,
    start: usize,
}

fn tokenize(src: &str) -> Vec<Token> {
    let chars: Vec<(usize, char)> = src.char_indices().collect();
    let mut tokens = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        let (start, c) = chars[i];
        if c.is_whitespace() || c == ',' {
            i += 1;
            continue;
        }
        let take = |i: &mut usize, keep: &dyn Fn(char) -> bool| {
            let from = *i;
            while *i < chars.len() && keep(chars[*i].1) {
                *i += 1;
            }
            chars[from..*i].iter().map(|&(_, c)| c).collect::<String>()
        };
        if c.is_ascii_digit() {
            let mut text = take(&mut i, &|c| c.is_ascii_digit() || c == '.');
            let mut kind = Kind::Number;
            // 2026-12-25
            if text.len() == 4 && !text.contains('.') && iso_tail(&chars, i) {
                text.extend(chars[i..i + 6].iter().map(|&(_, c)| c));
                i += 6;
                kind = Kind::IsoDate;
            } else if i + 2 < chars.len()
                && chars[i].1 == ':'
                && chars[i + 1].1.is_ascii_digit()
                && chars[i + 2].1.is_ascii_digit()
            {
                // 15:30
                text.extend(chars[i..i + 3].iter().map(|&(_, c)| c));
                i += 3;
                kind = Kind::Clock;
            }
            // 3pm, 3:30am (no space before am/pm)
            let rest: String = chars[i..chars.len().min(i + 3)]
                .iter()
                .map(|&(_, c)| c.to_ascii_lowercase())
                .collect();
            let ampm = rest.starts_with("am") || rest.starts_with("pm");
            let word_ends = rest.chars().nth(2).is_none_or(|c| !c.is_alphanumeric());
            if ampm && word_ends && matches!(kind, Kind::Number | Kind::Clock) {
                text.push_str(&rest[..2]);
                i += 2;
                kind = Kind::AmPm;
            }
            tokens.push(Token { kind, text, start });
        } else if c.is_alphabetic() || c == '_' {
            let text = take(&mut i, &|c| c.is_alphanumeric() || c == '_' || c == '/');
            tokens.push(Token {
                kind: Kind::Word,
                text: text.to_lowercase(),
                start,
            });
        } else {
            tokens.push(Token {
                kind: Kind::Op(c),
                text: c.to_string(),
                start,
            });
            i += 1;
        }
    }
    tokens
}

/// `-MM-DD` follows at `i`.
fn iso_tail(chars: &[(usize, char)], i: usize) -> bool {
    let at = |k: usize| chars.get(i + k).map(|&(_, c)| c);
    at(0) == Some('-')
        && at(1).is_some_and(|c| c.is_ascii_digit())
        && at(2).is_some_and(|c| c.is_ascii_digit())
        && at(3) == Some('-')
        && at(4).is_some_and(|c| c.is_ascii_digit())
        && at(5).is_some_and(|c| c.is_ascii_digit())
        && at(6).is_none_or(|c| !c.is_ascii_digit())
}

const MONTHS: &[&str] = &[
    "jan", "feb", "mar", "apr", "may", "jun", "jul", "aug", "sep", "oct", "nov", "dec",
];

fn month_number(word: &str) -> Option<i8> {
    const FULL: &[&str] = &[
        "january",
        "february",
        "march",
        "april",
        "may",
        "june",
        "july",
        "august",
        "september",
        "october",
        "november",
        "december",
    ];
    let index = MONTHS
        .iter()
        .position(|m| *m == word)
        .or_else(|| FULL.iter().position(|m| *m == word))
        .or_else(|| (word == "sept").then_some(8))?;
    i8::try_from(index + 1).ok()
}

/// One component of a duration.
enum DurationUnit {
    Years,
    Months,
    Weeks,
    Days,
    Seconds(f64),
}

fn duration_unit(word: &str) -> Option<DurationUnit> {
    Some(match word {
        "year" | "years" | "yr" | "yrs" => DurationUnit::Years,
        "month" | "months" | "mo" | "mos" => DurationUnit::Months,
        "week" | "weeks" | "wk" | "wks" => DurationUnit::Weeks,
        "day" | "days" => DurationUnit::Days,
        "hour" | "hours" | "hr" | "hrs" | "h" => DurationUnit::Seconds(3600.0),
        "minute" | "minutes" | "min" | "mins" => DurationUnit::Seconds(60.0),
        "second" | "seconds" | "sec" | "secs" | "s" => DurationUnit::Seconds(1.0),
        _ => return None,
    })
}

struct Parser<'a> {
    src: &'a str,
    tokens: &'a [Token],
    pos: usize,
    now: &'a ZonedDateTime,
    state: &'a DateState,
    uses_time: bool,
}

type TermResult = Result<DateValue, String>;

impl Parser<'_> {
    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.pos)
    }

    fn peek_at(&self, offset: usize) -> Option<&Token> {
        self.tokens.get(self.pos + offset)
    }

    fn is_keyword(&self, word: &str) -> bool {
        !self.state.names.contains(word)
            && matches!(word, "today" | "now" | "time" | "tomorrow" | "yesterday")
    }

    /// Parses the first term. `None`: the line doesn't start like a date.
    /// The flag is true for strong starts, whose errors are reported.
    fn term(&mut self) -> Option<(TermResult, bool)> {
        let token = self.peek()?.clone();
        match token.kind {
            Kind::Word if self.state.vars.contains_key(&token.text) => {
                self.pos += 1;
                Some((Ok(self.state.vars[&token.text].clone()), true))
            }
            Kind::Word if token.text == "prev" && self.state.prev.is_some() => {
                self.pos += 1;
                Some((Ok(self.state.prev.clone().expect("checked")), true))
            }
            Kind::Word if self.is_keyword(&token.text) => {
                self.pos += 1;
                Some((self.keyword(&token.text), true))
            }
            Kind::IsoDate => {
                self.pos += 1;
                let date = parse_iso(&token.text);
                Some((date.and_then(|d| self.with_optional_time(d)), true))
            }
            Kind::Word
                if matches!(token.text.as_str(), "noon" | "midnight")
                    && !self.state.names.contains(&token.text) =>
            {
                let time = self.time_literal()?;
                self.uses_time = true;
                let today = self.now.date();
                Some((time.and_then(|t| self.at(today, Some(t))), false))
            }
            Kind::Number | Kind::Word => {
                let date = self.month_date()?;
                Some((date.and_then(|d| self.with_optional_time(d)), false))
            }
            Kind::Clock | Kind::AmPm => {
                let time = self.time_literal()?;
                self.uses_time = true;
                let today = self.now.date();
                Some((time.and_then(|t| self.at(today, Some(t))), false))
            }
            Kind::Op(_) => None,
        }
    }

    fn keyword(&mut self, word: &str) -> TermResult {
        self.uses_time = true;
        let now = self.now.clone();
        let day = |days: i64| -> Result<ZonedDateTime, String> {
            now.checked_add(Span::new().days(days))
                .map_err(|e| e.to_string())
        };
        let (zdt, has_time) = match word {
            "now" | "time" => (now.clone(), true),
            "today" => (now.clone(), false),
            "tomorrow" => (day(1)?, false),
            "yesterday" => (day(-1)?, false),
            _ => unreachable!("is_keyword"),
        };
        let value = DateValue {
            zdt,
            has_time,
            zone_shown: false,
        };
        if has_time {
            return Ok(value);
        }
        // `tomorrow 3pm`
        match self.time_literal() {
            Some(time) => self.at(value.zdt.date(), Some(time?)),
            None => Ok(value),
        }
    }

    /// `25 Dec [2026]`, `Dec 25[,] [2026]`, `December 25`.
    fn month_date(&mut self) -> Option<Result<Date, String>> {
        let first = self.peek()?.clone();
        let second = self.peek_at(1)?.clone();
        let (day, month) = match (&first.kind, &second.kind) {
            (Kind::Number, Kind::Word) => (first.text.clone(), month_number(&second.text)?),
            (Kind::Word, Kind::Number) => (second.text.clone(), month_number(&first.text)?),
            _ => return None,
        };
        let day: i8 = day.parse().ok().filter(|d| (1..=31).contains(d))?;
        self.pos += 2;
        let year = match self.peek() {
            Some(t) if t.kind == Kind::Number && t.text.len() == 4 => {
                let year = t.text.parse().ok();
                self.pos += 1;
                year
            }
            _ => None,
        };
        let year = year.unwrap_or_else(|| {
            self.uses_time = true;
            self.now.year()
        });
        let name = MONTHS[usize::try_from(month - 1).unwrap_or(0)];
        Some(Date::new(year, month, day).map_err(|_| format!("There is no {day} {name} {year}")))
    }

    /// A time after a date: `2026-12-25 15:30`, `tomorrow 3pm`.
    fn with_optional_time(&mut self, date: Date) -> TermResult {
        match self.time_literal() {
            Some(time) => self.at(date, Some(time?)),
            None => self.at(date, None),
        }
    }

    /// `15:30`, `3pm`, `3:30pm`, `noon`, `midnight`.
    fn time_literal(&mut self) -> Option<Result<Time, String>> {
        let token = self.peek()?.clone();
        let time = match token.kind {
            Kind::Clock => {
                let (h, m) = token.text.split_once(':')?;
                clock(h.parse().ok()?, m.parse().ok()?)
            }
            Kind::AmPm => {
                let (digits, suffix) = token.text.split_at(token.text.len() - 2);
                let (h, m) = digits.split_once(':').unwrap_or((digits, "0"));
                let hour: i8 = h.parse().ok()?;
                let minute: i8 = m.parse().ok()?;
                if !(1..=12).contains(&hour) {
                    return None;
                }
                let hour = match (hour, suffix) {
                    (12, "am") => 0,
                    (12, _) => 12,
                    (h, "pm") => h + 12,
                    (h, _) => h,
                };
                clock(hour, minute)
            }
            Kind::Word if token.text == "noon" => clock(12, 0),
            Kind::Word if token.text == "midnight" => clock(0, 0),
            _ => return None,
        };
        self.pos += 1;
        Some(time)
    }

    /// The date (at the time, if given) in the local zone.
    fn at(&self, date: Date, time: Option<Time>) -> TermResult {
        let zone = self.now.time_zone().clone();
        let zdt = date
            .to_datetime(time.unwrap_or(Time::midnight()))
            .to_zoned_datetime(zone)
            .map_err(|e| e.to_string())?;
        Ok(DateValue {
            zdt,
            has_time: time.is_some(),
            zone_shown: false,
        })
    }

    /// Everything after the first term.
    fn rest(&mut self, mut value: DateValue) -> Result<Outcome, String> {
        // `3pm Jakarta in London`: the first term's zone.
        if value.has_time
            && let Some(zone) = self.source_zone()?
        {
            value.zdt = value
                .zdt
                .datetime()
                .to_zoned_datetime(zone)
                .map_err(|e| e.to_string())?;
            value.zone_shown = true;
        }
        loop {
            let Some(token) = self.peek().cloned() else {
                return Ok(Outcome::Value(value));
            };
            match token.kind {
                Kind::Op('+') => {
                    self.pos += 1;
                    let (span, has_time) = self.duration()?;
                    value.zdt = value.zdt.checked_add(span).map_err(|e| e.to_string())?;
                    value.has_time |= has_time;
                }
                Kind::Op('-') => {
                    self.pos += 1;
                    if let Some(duration) = self.try_duration() {
                        let (span, has_time) = duration?;
                        value.zdt = value.zdt.checked_sub(span).map_err(|e| e.to_string())?;
                        value.has_time |= has_time;
                    } else {
                        return self.difference(&value);
                    }
                }
                Kind::Word if token.text == "in" || token.text == "to" => {
                    self.pos += 1;
                    let target = self.remaining_text();
                    let zone = zones::resolve(&target)
                        .ok_or_else(|| format!("Unknown time zone '{}'", target.trim()))?;
                    value.zdt = value.zdt.with_time_zone(zone);
                    value.zone_shown = true;
                    return Ok(Outcome::Value(value));
                }
                _ => return Err(format!("Can't use '{}' with a date", token.text)),
            }
        }
    }

    /// Zone words between the first term and `in`/`to`/an operator.
    fn source_zone(&mut self) -> Result<Option<TimeZone>, String> {
        let from = self.pos;
        let mut end = from;
        while let Some(t) = self.tokens.get(end) {
            if t.kind != Kind::Word || t.text == "in" || t.text == "to" {
                break;
            }
            end += 1;
        }
        if end == from {
            return Ok(None);
        }
        let start = self.tokens[from].start;
        let stop = self.tokens.get(end).map_or(self.src.len(), |t| t.start);
        let name = &self.src[start..stop];
        let zone =
            zones::resolve(name).ok_or_else(|| format!("Unknown time zone '{}'", name.trim()))?;
        self.pos = end;
        Ok(Some(zone))
    }

    /// The source text from the current token to the end.
    fn remaining_text(&mut self) -> String {
        let start = self.peek().map_or(self.src.len(), |t| t.start);
        self.pos = self.tokens.len();
        self.src[start..].to_string()
    }

    /// `2 weeks`, `1 month 3 days`, `1.5 hours`. Errors if none follows.
    fn duration(&mut self) -> Result<(Span, bool), String> {
        self.try_duration()
            .unwrap_or_else(|| Err("Add or subtract a duration, like '+ 2 weeks'".to_string()))
    }

    /// `None` when no duration starts here.
    fn try_duration(&mut self) -> Option<Result<(Span, bool), String>> {
        let mut span = Span::new();
        let mut seconds = 0.0_f64;
        let mut has_time = false;
        let mut count = 0;
        while let (Some(n), Some(u)) = (self.peek(), self.peek_at(1)) {
            if n.kind != Kind::Number || u.kind != Kind::Word {
                break;
            }
            let Some(unit) = duration_unit(&u.text) else {
                break;
            };
            let Ok(amount) = n.text.parse::<f64>() else {
                break;
            };
            let whole = || -> Result<i64, String> {
                if amount.fract() == 0.0 && amount.abs() < 1e7 {
                    #[allow(clippy::cast_possible_truncation)]
                    Ok(amount as i64)
                } else {
                    Err("Use whole numbers of years, months, weeks and days".to_string())
                }
            };
            let next = match unit {
                DurationUnit::Years => {
                    whole().and_then(|n| span.try_years(n).map_err(|_| too_long()))
                }
                DurationUnit::Months => {
                    whole().and_then(|n| span.try_months(n).map_err(|_| too_long()))
                }
                DurationUnit::Weeks => {
                    whole().and_then(|n| span.try_weeks(n).map_err(|_| too_long()))
                }
                DurationUnit::Days => {
                    whole().and_then(|n| span.try_days(n).map_err(|_| too_long()))
                }
                DurationUnit::Seconds(per) => {
                    seconds += amount * per;
                    has_time = true;
                    Ok(span)
                }
            };
            match next {
                Ok(next) => span = next,
                Err(e) => return Some(Err(e)),
            }
            self.pos += 2;
            count += 1;
        }
        if count == 0 {
            return None;
        }
        if !seconds.is_finite() || seconds.abs() > 1e12 {
            return Some(Err(too_long()));
        }
        #[allow(clippy::cast_possible_truncation)]
        let span = span
            .try_seconds(seconds.round() as i64)
            .map_err(|_| too_long());
        Some(span.map(|span| (span, has_time)))
    }

    /// `a - b [in unit]`, as a fend quantity.
    fn difference(&mut self, a: &DateValue) -> Result<Outcome, String> {
        let b = match self.term() {
            Some((b, _)) => b?,
            None => return Err("Subtract a date or a duration".to_string()),
        };
        let quantity = if a.has_time || b.has_time {
            let seconds = a
                .zdt
                .timestamp()
                .duration_since(b.zdt.timestamp())
                .as_secs();
            format!("({seconds} seconds)")
        } else {
            let days = a
                .zdt
                .date()
                .since(b.zdt.date())
                .map_err(|e| e.to_string())?
                .get_days();
            format!("({days} days)")
        };
        let target = match self.peek() {
            None => None,
            Some(t) if t.kind == Kind::Word && (t.text == "in" || t.text == "to") => {
                self.pos += 1;
                Some(self.remaining_text())
            }
            Some(t) => return Err(format!("Can't use '{}' after a date difference", t.text)),
        };
        let default_unit = if a.has_time || b.has_time {
            "hours"
        } else {
            "days"
        };
        let target = target.as_deref().map_or(default_unit, str::trim);
        Ok(Outcome::Fend(format!("{quantity} to {target}")))
    }
}

fn too_long() -> String {
    "That duration is too long".to_string()
}

fn clock(hour: i8, minute: i8) -> Result<Time, String> {
    Time::new(hour, minute, 0, 0).map_err(|_| format!("{hour}:{minute:02} is not a time"))
}

fn parse_iso(text: &str) -> Result<Date, String> {
    let mut parts = text.split('-').map(str::parse::<i16>);
    let (Some(Ok(y)), Some(Ok(m)), Some(Ok(d))) = (parts.next(), parts.next(), parts.next()) else {
        return Err(format!("'{text}' is not a date"));
    };
    let (m, d) = (i8::try_from(m).ok(), i8::try_from(d).ok());
    m.zip(d)
        .and_then(|(m, d)| Date::new(y, m, d).ok())
        .ok_or_else(|| format!("'{text}' is not a date"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn now() -> ZonedDateTime {
        Date::new(2026, 11, 18)
            .unwrap()
            .to_datetime(Time::new(10, 0, 0, 0).unwrap())
            .to_zoned_datetime(TimeZone::get("Asia/Jakarta").unwrap())
            .unwrap()
    }

    fn eval(expr: &str) -> Option<Result<String, String>> {
        let now = now();
        evaluate(expr, &now, &DateState::default()).map(|r| {
            r.map(|e| match e.outcome {
                Outcome::Value(v) => format(&v, &now).0,
                Outcome::Fend(f) => format!("fend: {f}"),
            })
        })
    }

    fn ok(expr: &str) -> String {
        eval(expr)
            .unwrap_or_else(|| panic!("{expr} not a date"))
            .unwrap_or_else(|e| panic!("{expr}: {e}"))
    }

    #[test]
    fn keywords_and_arithmetic() {
        assert_eq!(ok("today"), "Wed, 18 Nov 2026");
        assert_eq!(ok("now"), "10:00");
        assert_eq!(ok("tomorrow"), "Thu, 19 Nov 2026");
        assert_eq!(ok("yesterday"), "Tue, 17 Nov 2026");
        assert_eq!(ok("today + 2 weeks"), "Wed, 2 Dec 2026");
        assert_eq!(ok("today - 3 days"), "Sun, 15 Nov 2026");
        assert_eq!(ok("now + 90 minutes"), "11:30");
        assert_eq!(ok("now + 1.5 hours"), "11:30");
        assert_eq!(ok("today + 1 month 2 days"), "Sun, 20 Dec 2026");
    }

    #[test]
    fn calendar_months_clamp_to_the_month_end() {
        assert_eq!(ok("2026-01-31 + 1 month"), "Sat, 28 Feb 2026");
        assert_eq!(ok("2028-01-31 + 1 month"), "Tue, 29 Feb 2028");
        assert_eq!(ok("2028-02-29 + 1 year"), "Wed, 28 Feb 2029");
    }

    #[test]
    fn literals() {
        assert_eq!(ok("2026-12-25"), "Fri, 25 Dec 2026");
        assert_eq!(ok("25 Dec"), "Fri, 25 Dec 2026");
        assert_eq!(ok("Dec 25, 2027"), "Sat, 25 Dec 2027");
        assert_eq!(ok("december 25"), "Fri, 25 Dec 2026");
        assert_eq!(ok("3pm"), "15:00");
        assert_eq!(ok("12am"), "00:00");
        assert_eq!(ok("15:30"), "15:30");
        assert_eq!(ok("noon"), "12:00");
        assert_eq!(ok("tomorrow 3pm"), "Thu, 19 Nov 2026 15:00");
        assert_eq!(ok("2026-12-25 09:15"), "Fri, 25 Dec 2026 09:15");
    }

    #[test]
    fn differences_go_to_fend() {
        assert_eq!(ok("2026-12-25 - today"), "fend: (37 days) to days");
        assert_eq!(ok("25 Dec - today in weeks"), "fend: (37 days) to weeks");
        assert_eq!(ok("now - 9am"), "fend: (3600 seconds) to hours");
    }

    #[test]
    fn zones() {
        assert_eq!(ok("now in Tokyo"), "12:00 JST");
        assert_eq!(ok("3pm Jakarta in London"), "08:00 GMT");
        assert_eq!(ok("now in New York"), "Tue 22:00 EST");
        assert_eq!(ok("now in Asia/Kolkata"), "08:30 IST");
        assert_eq!(ok("now in UTC+7"), "10:00 +07");
    }

    #[test]
    fn errors_and_fallbacks() {
        assert_eq!(
            eval("now in Atlantis"),
            Some(Err("Unknown time zone 'Atlantis'".into()))
        );
        assert!(matches!(eval("today + 5 kg"), Some(Err(_))));
        assert!(matches!(eval("2026-02-30"), Some(Err(_))));
        // Weak starts fall back to fend.
        assert!(eval("3pm in nm").is_none());
        assert!(eval("5 km").is_none());
        assert!(eval("2 + 2").is_none());
        assert!(eval("distance / time").is_none());
        assert!(eval("13pm").is_none());
    }

    #[test]
    fn user_variables_keep_their_meaning() {
        let mut state = DateState::default();
        state.set_other(Some("time"));
        let now = now();
        assert!(evaluate("time in hours", &now, &state).is_none());
    }

    #[test]
    fn date_variables_and_prev() {
        let now = now();
        let mut state = DateState::default();
        let Outcome::Value(v) = evaluate("2026-12-25", &now, &state)
            .unwrap()
            .unwrap()
            .outcome
        else {
            panic!()
        };
        state.set_date(Some("deadline"), &v);
        let r = evaluate("deadline - today", &now, &state).unwrap().unwrap();
        assert!(matches!(r.outcome, Outcome::Fend(ref f) if f == "(37 days) to days"));
        let r = evaluate("prev + 1 week", &now, &state).unwrap().unwrap();
        let Outcome::Value(v) = r.outcome else {
            panic!()
        };
        assert_eq!(format(&v, &now).0, "Fri, 1 Jan 2027");
    }

    #[test]
    fn dst_transitions_keep_wall_time() {
        let now = Date::new(2026, 3, 7)
            .unwrap()
            .to_datetime(Time::new(9, 0, 0, 0).unwrap())
            .to_zoned_datetime(TimeZone::get("America/New_York").unwrap())
            .unwrap();
        let r = evaluate("now + 1 day", &now, &DateState::default())
            .unwrap()
            .unwrap();
        let Outcome::Value(v) = r.outcome else {
            panic!()
        };
        assert_eq!(format(&v, &now).0, "Sun, 8 Mar 2026 09:00");
        let r = evaluate("now + 24 hours", &now, &DateState::default())
            .unwrap()
            .unwrap();
        let Outcome::Value(v) = r.outcome else {
            panic!()
        };
        assert_eq!(format(&v, &now).0, "Sun, 8 Mar 2026 10:00");
    }
}
