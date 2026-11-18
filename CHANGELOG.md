# Changelog

## 1.0.0 — unreleased

First release.

### Calculation
- Natural math per line with arbitrary precision (fend), units and conversions, hex/binary/octal, math functions and constants.
- Natural-language percentages (`20% of 50`, `50 + 10%`, `5 as % of 20`), variables, `prev`, and block totals with `sum`/`total`/`avg`.
- Currency conversion with daily rates from open.er-api.com, cached for offline use.
- Dates and times: `today`, `now`, `tomorrow`, ISO and month-name dates, `3pm`; calendar arithmetic (`+ 1 month` keeps the day, clamped at month end); differences between dates in any time unit; time zone conversion by IANA name, city, country, abbreviation or `UTC+7`. Lines with `now` refresh every minute.
- CSS units `px`, `pt`, `em`, `rem` (`pt` and `rem` mean the CSS units; pints stay as `pint`), with px per inch and em size in settings.
- `~/.config/notecal/definitions.txt`: your own variables, functions, dates and units (`unit sprint = 2 weeks`) for every note; applies live, and bad lines are listed in Settings.
- `en` and `id` number formats, adjustable precision; a line that takes too long is stopped without blocking the editor.

### Interface
- English and Indonesian, chosen in Settings or following the system language (`language = "auto"`); the tray menu, notices and error hover text follow it. Weekday and month names stay English.

### Command line
- `notecal-cli`: evaluate an expression, a piped note or a saved note (`--note NAME`) from the terminal, with the app's settings, definitions and cached rates; `--copy` prints bare numbers. Installed with the Arch package.
- Launcher scripts: an Omarchy menu entry (`notecal-calc`) and a Raycast script command.

### Notes and editor
- Several notes as `.txt` files with autosave; safe with sync tools (conflict copies instead of overwrites).
- Quick switcher (Ctrl+P), syntax highlighting, autocomplete, search, multiple cursors, copy shortcuts, screen-reader announcements.
- Themes: follows Omarchy live, built-in light/dark, or your own palette files.

### App
- `notecal --toggle` for Wayland keybindings; Alt+Space on X11/macOS; tray icon; settings file and screen.
