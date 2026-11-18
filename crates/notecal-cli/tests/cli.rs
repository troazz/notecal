//! Runs the real binary against its own config, data and cache folders.
//! Linux only: the folders are redirected with the XDG variables.
#![cfg(target_os = "linux")]

use std::io::Write;
use std::process::{Command, Output, Stdio};

use tempfile::TempDir;

struct Home {
    dir: TempDir,
}

impl Home {
    fn new() -> Self {
        let home = Self {
            dir: TempDir::new().unwrap(),
        };
        for sub in ["config/notecal", "data/notecal/notes", "cache/notecal"] {
            std::fs::create_dir_all(home.dir.path().join(sub)).unwrap();
        }
        home
    }

    fn write(&self, path: &str, text: &str) {
        std::fs::write(self.dir.path().join(path), text).unwrap();
    }

    fn run(&self, args: &[&str], stdin: Option<&str>) -> Output {
        let root = self.dir.path();
        let mut child = Command::new(env!("CARGO_BIN_EXE_notecal-cli"))
            .args(args)
            .env("HOME", root)
            .env("XDG_CONFIG_HOME", root.join("config"))
            .env("XDG_DATA_HOME", root.join("data"))
            .env("XDG_CACHE_HOME", root.join("cache"))
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        let mut input = child.stdin.take().unwrap();
        if let Some(text) = stdin {
            input.write_all(text.as_bytes()).unwrap();
        }
        drop(input);
        child.wait_with_output().unwrap()
    }
}

fn stdout(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

#[test]
fn evaluates_an_expression() {
    let home = Home::new();
    let out = home.run(&["5", "km", "in", "miles"], None);
    assert!(out.status.success());
    assert_eq!(stdout(&out), "3.106856 miles\n");
    assert_eq!(stdout(&home.run(&["-c", "1234.5 * 2"], None)), "2469\n");
}

#[test]
fn uses_the_apps_settings_definitions_and_rates() {
    let home = Home::new();
    home.write(
        "config/notecal/settings.toml",
        "locale = \"id\"\nprecision = 2\n",
    );
    home.write(
        "config/notecal/definitions.txt",
        "vat = 11%\nunit sprint = 2 weeks\n",
    );
    home.write(
        "cache/notecal/rates.json",
        r#"{"fetched_at": 0, "source_date": "Wed, 18 Nov 2026 00:00:01 +0000", "rates": {"USD": 1.0, "IDR": 16000.0}}"#,
    );
    assert_eq!(stdout(&home.run(&["1/3"], None)), "0,33\n");
    assert_eq!(
        stdout(&home.run(&["6 weeks in sprints"], None)),
        "3 sprints\n"
    );
    assert_eq!(stdout(&home.run(&["200 * (1 + vat)"], None)), "222\n");
    assert_eq!(stdout(&home.run(&["10 USD in IDR"], None)), "160.000 IDR\n");
}

#[test]
fn evaluates_saved_notes_and_piped_text() {
    let home = Home::new();
    home.write(
        "data/notecal/notes/budget.txt",
        "rent = 1500\nfood = 600\ntotal\n",
    );
    let out = home.run(&["--note", "budget"], None);
    assert_eq!(
        stdout(&out),
        "rent = 1500  1,500\nfood = 600   600\ntotal        2,100\n"
    );
    let out = home.run(&[], Some("2 + 2\n# note\n"));
    assert_eq!(stdout(&out), "2 + 2  4\n# note\n");
    assert_eq!(
        stdout(&home.run(&["-c", "-"], Some("x = 2\nx * 21\n"))),
        "2\n42\n"
    );
}

#[test]
fn reports_errors_with_exit_codes() {
    let home = Home::new();
    let out = home.run(&["--note", "missing"], None);
    assert_eq!(out.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&out.stderr).contains("No note named 'missing'"));

    let out = home.run(&["100 USD in IDR"], None);
    assert_eq!(out.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&out.stderr).contains("open notecal once"));

    assert_eq!(home.run(&["--bogus"], None).status.code(), Some(2));
}
