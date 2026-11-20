# Changelog

## 1.0.1 — 2026-11-20

- `.deb` package for Ubuntu 22.04+, Linux Mint 21+ and Debian 12+, with `notecal-cli`; the update prompt shows its `apt install` command.
- One-click updates, for copies of 1.0.1 and later: the Windows installer, the `.dmg` app and the AppImage download the signed update in the background and offer "Restart to update"; Homebrew installs get "Upgrade now", which runs `brew upgrade --cask notecal`. pacman installs still show the `makepkg` command.

## 1.0.0 — 2026-11-20

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
- Update check: once a day notecal asks GitHub for the latest release. When there's a newer one, it shows how to upgrade for the way it was installed: `brew upgrade --cask notecal`, the `makepkg` command for the pacman package, or a download link. Turn it off in Settings or during setup.
- First-run setup in the style of Zed's and VS Code's: look (theme cards with previews, font size), language and number format, keys (the Hyprland bind line to copy, or the shortcut; standard or vim editing; always on top) and data (daily rates, notes folder). Choices apply live; Skip keeps the defaults; it ends on a Welcome note with an example from every cheatsheet section. Shown once on a fresh install; reopen from Settings or the cheatsheet.
- Cheatsheet in Ctrl+P: type `?` for examples of every kind of calculation, with live results and a one-line explanation, plus the keyboard shortcuts. Search matches the examples, explanations and section keywords in English or Indonesian (`?discount`, `?diskon`, `?tz tokyo`, `?keys`); Enter inserts an example into the note.
- Vim mode (`vim_mode`, off by default): normal, insert and visual modes, operators, counts, registers, `.`, `/` search; the mode shows in the status bar; `:w` saves, `:q`/`:x`/`:wq` save and hide.
- Always on top works on Hyprland (pins the floating window) and has a pin button in the status bar.
- Status bar at the bottom: the open note, the rates date (when the note uses a currency), a "live" marker for results that update every minute, and the number format and language, each switchable with a click.
- English and Indonesian, chosen in Settings or following the system language (`language = "auto"`); the tray menu, notices, error hover text and the weekday and month names in date results (`Rab, 25 Des 2026`) follow it. Indonesian month names are accepted as input (`17 Agustus 2026`).

### Platforms
- Windows build (NSIS installer), built and tested in CI.
- New note names follow Windows file-name rules so synced folders work everywhere; saves retry when another program briefly holds the file (Windows).
- iCloud Drive notes folders: notes evicted to iCloud are listed and downloaded when opened.
- The macOS app and Windows installer include `notecal-cli`; the Homebrew cask puts it on your `PATH`.

### Command line
- `notecal-cli`: evaluate an expression, a piped note or a saved note (`--note NAME`) from the terminal, with the app's settings, definitions and cached rates; `--copy` prints bare numbers. Installed with the Arch package.
- Launcher scripts: an Omarchy menu entry (`notecal-calc`) and a Raycast script command.

### Notes and editor
- Several notes as `.txt` files with autosave; safe with sync tools (conflict copies instead of overwrites).
- Quick switcher (Ctrl+P), syntax highlighting, autocomplete, search, multiple cursors, copy shortcuts, screen-reader announcements.
- Themes: follows Omarchy live, built-in light/dark, or your own palette files.

### App
- `notecal --toggle` for Wayland keybindings; Alt+Space on X11/macOS; tray icon; settings file and screen.
