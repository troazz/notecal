# notecal on Linux

## Show and hide notecal with a key

Wayland compositors do not let apps register global shortcuts, so on Wayland you bind a key in your compositor to:

```sh
notecal --toggle
```

That shows notecal (focused) or hides it if it is already in front. If notecal is not running yet, it starts. Esc also hides the window. On X11 and macOS the shortcut from Settings (default Alt+Space) works without this.

Until notecal is installed on your `PATH`, use the full path to the binary, for example `~/Work/notecal/target/release/notecal --toggle`.

### Hyprland (Lua config, e.g. Omarchy)

In `~/.config/hypr/bindings.lua`:

```lua
hl.bind("ALT + SPACE", hl.dsp.exec_cmd("notecal --toggle"), { description = "notecal" })
```

If Alt+Space is already bound, `hl.unbind("ALT + SPACE")` first, or pick another key.

### Hyprland (classic `hyprland.conf`)

```ini
bind = ALT, SPACE, exec, notecal --toggle
```

### Sway

In `~/.config/sway/config`:

```
bindsym Mod1+space exec notecal --toggle
```

### GNOME

Settings → Keyboard → View and Customise Shortcuts → Custom Shortcuts → add one with the command `notecal --toggle`.

## Window size, position and always on top (Wayland)

On Wayland the compositor decides where windows go and how big they are, and apps cannot keep themselves on top, so notecal's "always on top" setting and its remembered window size only apply on X11 and macOS. On Hyprland, a window rule does the same job:

```lua
-- ~/.config/hypr/hyprland.lua (Lua config)
hl.window_rule({ match = { class = "^notecal$" }, float = true, center = true, size = { 720, 520 }, pin = true })
```

```ini
# hyprland.conf (classic config)
windowrule = float, pin, size 720 520, center, class:^(notecal)$
```

`pin` keeps notecal above other windows on every workspace; drop it if you do not want that. The Lua form matches the rules Omarchy itself uses (`/usr/share/omarchy/default/hypr`).

## Files

| What | Where |
|---|---|
| Notes | `~/.local/share/notecal/notes/*.txt` (change with `notes_dir` in settings) |
| Settings | `~/.config/notecal/settings.toml` |
| Window and last-note state | `~/.config/notecal/state.json` |
| Exchange rates cache | `~/.cache/notecal/rates.json` |

Notes are plain text. Pointing `notes_dir` at a Syncthing or git folder is supported: notecal never overwrites a note that changed on disk while you were editing it; your version is saved as `<name> (conflict <date>).txt` instead.

## Settings

`~/.config/notecal/settings.toml`. Every key is optional; changes apply as soon as the file is saved. A bad value is reported and replaced by its default.

```toml
shortcut = "Alt+Space"   # X11/macOS only; on Wayland bind `notecal --toggle`
theme = "auto"           # auto (Omarchy, else system) | light | dark | a file in ~/.config/notecal/themes/
font_family = "auto"     # auto = system monospace font (follows Omarchy's font)
font_size = 15           # 8–40
precision = 6            # decimal places, 0–20
locale = "en"            # en: 1,234.5 | id: 1.234,5
notes_dir = ""           # empty = default folder
currency_refresh = true  # false = use cached exchange rates only
always_on_top = false
background = true        # closing the window hides it; quit from the tray
css_ppi = 96             # px per inch, 1–1000
css_em_px = 16           # px per em and rem, 1–200
```

## Definitions

`~/.config/notecal/definitions.txt` holds your own names for every note. It is optional, and edits apply as soon as the file is saved. Each line is a definition, a comment (`#`, `//`) or blank:

```
vat = 11%
rate = 15 USD / hour
sq = x: x^2              # a function: sq(12)
payday = 2026-11-25      # a date: payday - today
unit sprint = 2 weeks    # a unit that converts: 6 weeks in sprints
```

A note cannot reassign a defined name. A line that doesn't work is listed in Settings (Ctrl+,) with its line number; the other lines still load.

## Command line

`notecal-cli` evaluates without opening the app, with the same settings, definitions and cached exchange rates (it never fetches rates itself):

```sh
notecal-cli 5 km in miles          # 3.106856 miles
notecal-cli -c '1500 * 12'         # 18000 (bare number, for scripts)
notecal-cli --note budget          # a saved note, each line with its result
printf 'a = 2\na * 21\n' | notecal-cli
```

It exits with 1 when an expression has no result (the reason goes to stderr) and 2 for a usage error.

## Launchers

`packaging/launchers/notecal-calc` asks for an expression in the Omarchy menu, shows the result as a notification and copies it. The Arch package installs it as `notecal-calc`; otherwise copy it somewhere on your `PATH`. Then add a row to `~/.config/omarchy/extensions/omarchy-menu.jsonc`:

```jsonc
"notecal": {"icon":"󰃬","label":"Calculate","aliases":["calc"],"description":"notecal: evaluate an expression and copy the result","action":"notecal-calc"},
```

Bind it to a key in Hyprland the same way as `notecal --toggle`. On macOS, `packaging/launchers/raycast-notecal.sh` is the same thing as a Raycast script command.

## WebKitGTK

notecal sets `WEBKIT_DISABLE_DMABUF_RENDERER=1` at startup, which cut idle CPU from ~11% to ~1% on Hyprland (docs/gate1.md). Set `WEBKIT_DISABLE_DMABUF_RENDERER=0` to turn the DMABUF renderer back on.
