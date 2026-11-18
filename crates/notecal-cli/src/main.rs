//! `notecal-cli`: evaluate expressions or whole notes from the terminal
//! (PRD A7), with the app's settings, definitions and cached exchange rates.
//! It never fetches rates; open notecal once with a currency line for that.

#![forbid(unsafe_code)]

use std::io::{self, IsTerminal, Read};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use notecal_core::{Engine, LineResult, NO_RATES, Rates, Settings};
use notecal_env::paths::{self, SystemDirs};
use notecal_env::rates::RatesStore;
use notecal_env::settings;

const USAGE: &str = "\
Usage:
  notecal-cli EXPRESSION...     Evaluate one expression
  notecal-cli -                 Evaluate a note read from standard input
  COMMAND | notecal-cli         (same, when input is piped)
  notecal-cli --note NAME       Evaluate a saved note

Options:
  -c, --copy      Print bare numbers (no units or separators), one per result
  -h, --help      Show this help
  -V, --version   Show the version

Examples:
  notecal-cli 5 km in miles
  notecal-cli '100 USD in IDR'
  notecal-cli --note budget
  printf 'rent = 1500\\nfood = 600\\ntotal\\n' | notecal-cli

Settings, definitions.txt and cached exchange rates are the app's.";

#[derive(Debug, PartialEq, Eq)]
enum Input {
    Expression(String),
    Stdin,
    Note(String),
}

#[derive(Debug, PartialEq, Eq)]
struct Options {
    input: Input,
    copy: bool,
}

#[derive(Debug, PartialEq, Eq)]
enum Parsed {
    Run(Options),
    Help,
    Version,
}

fn parse_args(args: &[String], stdin_is_piped: bool) -> Result<Parsed, String> {
    let mut copy = false;
    let mut note = None;
    let mut stdin = false;
    let mut words = Vec::new();
    let mut iter = args.iter();
    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "-h" | "--help" => return Ok(Parsed::Help),
            "-V" | "--version" => return Ok(Parsed::Version),
            "-c" | "--copy" => copy = true,
            "-n" | "--note" => {
                let name = iter.next().ok_or("--note needs a note name")?;
                note = Some(name.clone());
            }
            "-" => stdin = true,
            // `--` ends options: `notecal-cli -- -5 + 2`.
            "--" => words.extend(iter.by_ref().cloned()),
            flag if flag.starts_with("--") => return Err(format!("Unknown option '{flag}'")),
            _ => words.push(arg.clone()),
        }
    }
    let input = match (note, stdin, words.is_empty()) {
        (Some(name), false, true) => Input::Note(name),
        (None, true, true) => Input::Stdin,
        (None, false, false) => Input::Expression(words.join(" ")),
        (None, false, true) if stdin_is_piped => Input::Stdin,
        (None, false, true) => return Ok(Parsed::Help),
        _ => return Err("Give one of: an expression, '-', or --note NAME".to_string()),
    };
    Ok(Parsed::Run(Options { input, copy }))
}

/// The app's settings, definitions, rates and notes folder.
struct Environment {
    settings: Settings,
    rates: Option<Rates>,
    notes_dir: PathBuf,
}

fn load_environment(dirs: &SystemDirs) -> Environment {
    let view = settings::load(&paths::settings_path(&dirs.config));
    let definitions = settings::load_definitions(&paths::definitions_path(&dirs.config));
    let mut problems = view.problems.clone();
    problems.extend(settings::definition_problems(&view.settings, &definitions));
    for problem in problems {
        eprintln!("notecal-cli: {problem}");
    }
    Environment {
        settings: view.settings.engine(&definitions),
        rates: RatesStore::load(paths::rates_path(&dirs.cache)).rates(),
        notes_dir: paths::resolve_notes_dir(&view.settings, &dirs.data),
    }
}

fn read_note(notes_dir: &Path, name: &str) -> Result<String, String> {
    let bad = name.trim().is_empty() || name.starts_with('.') || name.contains(['/', '\\']);
    if bad {
        return Err(format!("'{name}' is not a note name"));
    }
    let path = notes_dir.join(format!("{name}.txt"));
    std::fs::read_to_string(&path).map_err(|e| match e.kind() {
        io::ErrorKind::NotFound => format!("No note named '{name}' in {}", notes_dir.display()),
        _ => format!("Could not read {}: {e}", path.display()),
    })
}

fn error_message(message: &str) -> String {
    if message == NO_RATES {
        format!("{message}: open notecal once with a currency line to fetch them")
    } else {
        message.to_string()
    }
}

/// One expression: its result, or the error on stderr and exit code 1.
fn print_expression(result: &LineResult, copy: bool) -> ExitCode {
    let value = if copy { &result.copy } else { &result.display };
    match (value, &result.error) {
        (Some(value), _) => {
            println!("{value}");
            ExitCode::SUCCESS
        }
        (None, Some(error)) => {
            eprintln!("notecal-cli: {}", error_message(error));
            ExitCode::FAILURE
        }
        // A comment or blank line has no result and no error.
        (None, None) => ExitCode::SUCCESS,
    }
}

/// A whole note: each line with its result to the right, like the app.
fn format_note(text: &str, results: &[LineResult], copy: bool) -> String {
    let lines: Vec<&str> = text.split('\n').collect();
    let mut out = String::new();
    if copy {
        for value in results.iter().filter_map(|r| r.copy.as_deref()) {
            out.push_str(value);
            out.push('\n');
        }
        return out;
    }
    let width = lines
        .iter()
        .zip(results)
        .filter(|(_, r)| r.display.is_some() || r.error.is_some())
        .map(|(line, _)| line.trim_end().chars().count())
        .max()
        .unwrap_or(0)
        .min(60);
    // Drop the empty line after a final newline.
    let count = if text.ends_with('\n') {
        lines.len() - 1
    } else {
        lines.len()
    };
    for (line, result) in lines.iter().zip(results).take(count) {
        let line = line.trim_end();
        match (&result.display, &result.error) {
            (Some(display), _) => out.push_str(&format!("{line:<width$}  {display}\n")),
            (None, Some(error)) => out.push_str(&format!(
                "{line:<width$}  (error: {})\n",
                error_message(error)
            )),
            (None, None) => {
                out.push_str(line);
                out.push('\n');
            }
        }
    }
    out
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let options = match parse_args(&args, !io::stdin().is_terminal()) {
        Ok(Parsed::Run(options)) => options,
        Ok(Parsed::Help) => {
            println!("{USAGE}");
            return ExitCode::SUCCESS;
        }
        Ok(Parsed::Version) => {
            println!("notecal-cli {}", env!("CARGO_PKG_VERSION"));
            return ExitCode::SUCCESS;
        }
        Err(message) => {
            eprintln!("notecal-cli: {message}\n\n{USAGE}");
            return ExitCode::from(2);
        }
    };
    let Some(dirs) = paths::system_dirs() else {
        eprintln!("notecal-cli: could not find the home folder");
        return ExitCode::FAILURE;
    };
    let env = load_environment(&dirs);

    let (text, single) = match options.input {
        Input::Expression(expression) => (expression, true),
        Input::Stdin => {
            let mut text = String::new();
            if let Err(e) = io::stdin().read_to_string(&mut text) {
                eprintln!("notecal-cli: could not read standard input: {e}");
                return ExitCode::FAILURE;
            }
            (text, false)
        }
        Input::Note(name) => match read_note(&env.notes_dir, &name) {
            Ok(text) => (text, false),
            Err(message) => {
                eprintln!("notecal-cli: {message}");
                return ExitCode::FAILURE;
            }
        },
    };
    let results = Engine::default().evaluate(&text, &env.settings, env.rates.as_ref());
    if single {
        return results
            .first()
            .map_or(ExitCode::SUCCESS, |r| print_expression(r, options.copy));
    }
    print!("{}", format_note(&text, &results, options.copy));
    ExitCode::SUCCESS
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(list: &[&str]) -> Vec<String> {
        list.iter().map(ToString::to_string).collect()
    }

    fn run(list: &[&str], piped: bool) -> Parsed {
        parse_args(&args(list), piped).unwrap()
    }

    #[test]
    fn parses_the_input_forms() {
        let expr = |e: &str, copy| {
            Parsed::Run(Options {
                input: Input::Expression(e.into()),
                copy,
            })
        };
        assert_eq!(
            run(&["5", "km", "in", "miles"], false),
            expr("5 km in miles", false)
        );
        assert_eq!(run(&["-c", "2 + 2"], false), expr("2 + 2", true));
        assert_eq!(run(&["--", "-5", "+", "2"], false), expr("-5 + 2", false));
        assert_eq!(
            run(&["--note", "budget"], false),
            Parsed::Run(Options {
                input: Input::Note("budget".into()),
                copy: false
            })
        );
        assert_eq!(
            run(&["-"], false),
            Parsed::Run(Options {
                input: Input::Stdin,
                copy: false
            })
        );
        assert_eq!(
            run(&[], true),
            Parsed::Run(Options {
                input: Input::Stdin,
                copy: false
            })
        );
        assert_eq!(run(&[], false), Parsed::Help);
        assert_eq!(run(&["-V"], false), Parsed::Version);
    }

    #[test]
    fn rejects_mixed_and_unknown_input() {
        assert!(parse_args(&args(&["--note", "a", "2 + 2"]), false).is_err());
        assert!(parse_args(&args(&["--note"]), false).is_err());
        assert!(parse_args(&args(&["--frobnicate"]), false).is_err());
    }

    #[test]
    fn formats_a_note_like_the_app() {
        let text = "# Budget\nrent = 1500\nfood = 600\ntotal\nbad = 1 +\n";
        let results = Engine::default().evaluate(text, &Settings::default(), None);
        let out = format_note(text, &results, false);
        assert_eq!(
            out,
            "# Budget\nrent = 1500  1,500\nfood = 600   600\ntotal        2,100\nbad = 1 +    (error: \
             unexpected end of input)\n"
                .replace("unexpected end of input", results[4].error.as_deref().unwrap_or("")),
        );
        assert_eq!(format_note(text, &results, true), "1500\n600\n2100\n");
    }

    #[test]
    fn note_names_cannot_leave_the_notes_folder() {
        let dir = Path::new("/nonexistent");
        for name in ["../x", ".hidden", "a/b", " "] {
            assert!(
                read_note(dir, name)
                    .unwrap_err()
                    .contains("not a note name"),
                "{name}"
            );
        }
    }
}
