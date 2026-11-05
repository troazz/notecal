# Changelog

## 1.0.0 — unreleased

First release.

### Calculation
- Natural math per line with arbitrary precision (fend), units and conversions, hex/binary/octal, math functions and constants.
- Numi-style percentages (`20% of 50`, `50 + 10%`, `5 as % of 20`), variables, `prev`, and block totals with `sum`/`total`/`avg`.
- Currency conversion with daily rates from open.er-api.com, cached for offline use.
- `en` and `id` number formats, adjustable precision; a line that takes too long is stopped without blocking the editor.

### Notes and editor
- Several notes as `.txt` files with autosave; safe with sync tools (conflict copies instead of overwrites).
- Quick switcher (Ctrl+P), syntax highlighting, autocomplete, search, multiple cursors, copy shortcuts, screen-reader announcements.
- Themes: follows Omarchy live, built-in light/dark, or your own palette files.

### App
- `notecal --toggle` for Wayland keybindings; Alt+Space on X11/macOS; tray icon; settings file and screen.
