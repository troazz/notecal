# Plan: First-run onboarding (M13)

**Source PRD**: `docs/notecal.prd.md` (new requirement A8, milestone 13)
**Selected Milestone**: 13 — First-run onboarding
**Complexity**: Medium (~1.5 w)

## Summary
The first time notecal starts, a welcome screen, in the style of Zed's and VS Code's,
walks through the choices that matter before the first note: look, language and
numbers, keys, and data (rates and the notes folder). Every choice is saved and applied
the moment it's made, so the user sees the effect behind the screen. Any step can be
skipped, and "Skip" keeps the defaults. It ends on a welcome note with live
examples. It can be reopened from Settings and from the cheatsheet.

## How it looks
Like Zed's welcome page (one screen, sections of big clickable choices) crossed with
VS Code's walkthrough (a step list on the left with check marks). It's a full-window
overlay above the editor; the editor stays visible faintly behind it, so theme and
font changes show at once.

```
┌──────────────────────────────────────────────────────────────┐
│  notecal                                         Skip setup  │
│ ┌───────────────┐  ┌───────────────────────────────────────┐ │
│ │ ✓ Welcome     │  │  Pick a look                          │ │
│ │ ● Look        │  │  ┌────────┐ ┌────────┐ ┌────────┐     │ │
│ │ ○ Language    │  │  │ System │ │ Light  │ │ Dark   │ …   │ │
│ │ ○ Keys        │  │  │ 2+2  4 │ │ 2+2  4 │ │ 2+2  4 │     │ │
│ │ ○ Your data   │  │  └────────┘ └────────┘ └────────┘     │ │
│ │ ○ Ready       │  │  Font size  [ 13 | 15 | 17 | 20 ]     │ │
│ └───────────────┘  │                      Back   Next →    │ │
│                    └───────────────────────────────────────┘ │
└──────────────────────────────────────────────────────────────┘
```

## Steps
1. **Welcome:** one sentence on what notecal is, and a live three-line demo, evaluated
   by the engine (`rent = 1500`, `rent * 12`, `100 USD in IDR`). Buttons: "Get
   started", and "Skip setup".
2. **Look:**
   - Theme cards: System (auto, which follows Omarchy), Light, Dark, plus the files in
     the themes folder. Each card is a mini preview in that palette.
   - Font size: four presets.
3. **Language and numbers:**
   - Interface language: System, English or Bahasa Indonesia.
   - Number format: two cards with a sample result, `1,234.5` and `1.234,5`.
   - Decimal places: 2, 4, 6 (the default) or 10.
4. **Keys:**
   - **Show/hide key:**
     - On Wayland/Hyprland: the bind line for the user's config, with a Copy button
       (Lua or classic, detected). Pinning there needs the float rule, so that's
       shown too.
     - On X11, macOS and Windows: the shortcut field, with its status (registered or
       taken).
   - **Editing:** cards for Standard and Vim (`vim_mode`).
   - **Always on top:** a toggle. It's hidden where the desktop can't do it.
5. **Your data:**
   - **Exchange rates:** "Update daily from open.er-api.com" on or off
     (`currency_refresh`), with one line on what is sent: a plain request, no data.
   - **Notes folder:** the default, or a folder of your own, typed as a path, e.g. a
     Syncthing folder. Validated like in Settings.
6. **Ready:**
   - The key tips: Ctrl+P notes, `?` cheatsheet, Esc hide, click to copy.
   - A "Start with a welcome note" option, on by default, which creates a `Welcome`
     note of examples, one per cheatsheet section.

## Requirements
- **A8 (P1): First-run onboarding.**
  - It's shown once, on a fresh install.
  - Each choice is saved to `settings.toml` and applied live.
  - It can be skipped at any step, and Esc counts as skip.
  - It can be reopened later.
- **Fresh install** means no `state.json` at launch.
  - **Existing users** (who have a `state.json`) never get it automatically.
  - **Quitting part-way:** the onboarding comes back at the next launch, until it's
    finished or skipped.
- **Saving:** the same path as the Settings screen (`settings_api.save` → `validate`),
  so problems are reported in the same way. A failed save stays on the step, with the
  error shown.
- **Keyboard and accessibility:** fully usable from the keyboard.
  - Tab and arrow keys move between cards; Enter chooses; Alt+← and Alt+→ go back and
    next.
  - Focus is trapped, as in Settings.
  - The cards are a radio group (`role="radiogroup"`), and the step list is a
    `nav` with `aria-current`.
  - Contrast meets WCAG 2.2 AA in both themes.
- **Translated:** in en and id, and the screen switches language the moment the
  language is chosen.
- **Reopen:**
  - a "Show welcome" button in Settings;
  - a "Welcome / setup" row in the cheatsheet's keyboard section, that opens it.
- **Window size:** fits in the default 720×520 window, and scrolls inside a step if
  the window is smaller.

**Non-goals:**
- importing settings from other apps;
- accounts or telemetry;
- a guided tour drawn over the editor (coach marks);
- animations beyond a short fade;
- a native folder picker (decided: a typed path).

## Patterns to Mirror
| Category | Source | Pattern |
|---|---|---|
| Overlay dialog | `ui/src/settings/Settings.svelte:108-120` | `role="dialog"`, `onKeydown` with `trapTab`, Esc closes, saves through `onsave` |
| Saving settings | `ui/src/App.svelte` `saveSettings` | flush the session, then `settingsApi.save`; the backend emits `settings-changed`, which applies live |
| UI state file | `src-tauri/src/state.rs:11-35` | `UiState`, camelCase serde with `default`, atomic `save`, tests next to it |
| Commands | `src-tauri/src/commands.rs:538-545` | `load_state` / `save_state` |
| Strings | `ui/src/i18n.svelte.ts` | `onboarding.*` keys in both tables |
| Tests | `ui/src/notes/cheatsearch.test.ts`, `state.rs` tests | pure logic in a `.ts` module with vitest; Rust unit tests |

## Files to Change
| File | Action | Why |
|---|---|---|
| `src-tauri/src/state.rs` | UPDATE | `onboarding: Option<Onboarding>` (`pending` / `done`); missing field = done (existing users) |
| `src-tauri/src/commands.rs` | UPDATE | at startup a missing `state.json` is written as `pending`; `save_state` keeps the stored `onboarding` (today it replaces the whole file); new `onboarding_status`, `finish_onboarding` |
| `src-tauri/src/main.rs` | UPDATE | register the commands |
| `ui/src/onboarding/Onboarding.svelte` | CREATE | the overlay: step list, step body, Back/Next/Skip |
| `ui/src/onboarding/steps.ts` | CREATE | step order, which steps show on this platform (pin, shortcut vs. bind line), the bind-line text per config style |
| `ui/src/onboarding/steps.test.ts` | CREATE | step filtering per platform, bind-line text, welcome-note text |
| `ui/src/onboarding/ThemeCard.svelte` | CREATE | mini preview from a palette (reuses `theme.ts` palette resolution) |
| `ui/src/onboarding/welcome-note.ts` | CREATE | the welcome note's text, built from the cheatsheet sections (one example each) |
| `ui/src/theme.ts` | UPDATE | export the palette resolution for a named theme, for the previews |
| `ui/src/App.svelte` | UPDATE | ask `onboarding_status` on start; show the overlay before the note opens; on finish create the welcome note if asked |
| `ui/src/settings/Settings.svelte` | UPDATE | "Show welcome" button |
| `ui/src/notes/shortcuts.ts`, `Switcher.svelte` | UPDATE | a "Welcome / setup" row that opens it |
| `ui/src/i18n.svelte.ts`, `i18n.test.ts` | UPDATE | strings, en and id |
| `src-tauri/src/pin.rs` / `shell.rs` | — | reused: `pin_support`, `shortcut_status`, `is_wayland` decide what the Keys step shows |
| `docs/notecal.prd.md`, `README.md`, `CHANGELOG.md` | UPDATE | A8, milestone 13, a screenshot in the README |

## Tasks
### Task 1: First-run state ✅
- **Done**: `UiState.onboarding` (`pending`/`done`); no file → pending, an old file without the field → done; `save_state` goes through `save_position`, which keeps the stored value; `finish_onboarding` command; one state writer at a time (mutex). 5 state tests.
- **Action**:
  - Add `onboarding` to `UiState`.
  - At startup: no file → write `{ onboarding: "pending" }`. A file without the field
    reads as done.
  - `save_state` merges: it keeps `onboarding` from disk and only replaces
    `lastNote`/`cursor`.
  - `onboarding_status() -> "pending" | "done"`; `finish_onboarding()` sets done.
- **Validate**: `state.rs` tests:
  - missing file → pending;
  - old file → done;
  - `save_state` keeps the field;
  - finishing sets done.

### Task 2: Step model ✅
- **Done**: `steps.ts`: `STEPS`, presets, `bindLines` (Lua/classic Hyprland, other Wayland, none off Wayland), `welcomeNote`. `desktop_info` command in `pin.rs` (Wayland, Hyprland config style from `hypr/hyprland.lua`, pin support). The welcome layout is also evaluated by the engine (`cheatsheet::welcome_layout_evaluates`).
- **Action**: `steps.ts`, with pure functions:
  - `visibleSteps(platform)`: Keys shows the bind line on Wayland and the shortcut field
    elsewhere; the pin row only when `pin_support != none`;
  - `bindLine(style)`: the Lua and classic Hyprland lines from `docs/linux-setup.md`;
  - `welcomeNote(sheet, language)`: a header and one example per section, in the
    current language.
- **Validate**: `steps.test.ts`.

### Task 3: Overlay and steps ✅
- **Done**: `Onboarding.svelte`: native radio inputs styled as cards (arrow keys and screen readers for free), saves through the app's `saveSettings` like Settings, Esc = Skip, Alt+←/→, focus trap, the first control (or the primary button) focused on every step. Lazy `import()`: a 15 KB chunk, loaded only when shown.
- **Action**: `Onboarding.svelte`.
  - Each choice calls `onsave({...settings, key})`, as Settings does.
  - The step list is clickable, with ✓ on visited steps.
  - Back, Next and Skip.
  - Language changes apply at once.
- **Mirror**: Settings dialog (focus trap, Esc).
- **Validate**: `bun run check`; screenshots of every step in light and dark, en and id.

### Task 4: Theme previews ✅
- **Done**: `preview_theme(name)` reuses `theme::resolve` (with the same name rule as the setting); cards draw two sample lines in each palette.
- **Action**: `ThemeCard` draws a 3-line sample with the theme's palette (background,
  text, number, result colours). The themes come from `list_themes` and `get_theme`.
- **Validate**: a preview matches the editor after choosing the theme (screenshot).

### Task 5: App wiring ✅
- **Done**: `startOnce` shows it when the state is pending; Ctrl shortcuts and Esc-hide are off while it's open; finishing (or skipping) marks done and opens the Welcome note, created once and filled through the session like typing. Checked in a fresh XDG profile: shown; quit part-way → shown again; finished → gone after restart; skip → Welcome note, in Indonesian when chosen. Existing profile: not shown.
- **Action**:
  - On mount, if pending: show the overlay first. The note behind it is the usual
    Scratch note, read-only until the onboarding ends.
  - On finish or skip: `finish_onboarding`, then create and open `Welcome` if the
    option was kept.
  - Reopening it from Settings or the cheatsheet doesn't create a second welcome note
    if one exists.
- **Validate**:
  - Fresh profile (`XDG_CONFIG_HOME`, `XDG_DATA_HOME` in the scratchpad): the
    onboarding shows.
  - Quit part-way and restart: it shows again.
  - Finish and restart: it's gone.
  - Existing profile: it never shows.

### Task 6: Reopen points ✅
- **Done**: Settings "Show welcome and setup" button; a "Welcome and setup" row in the cheatsheet's keyboard section (`?welcome`, `?sambutan`) that Enter opens. Checked: reopening and dismissing doesn't duplicate the note or hide the window.
- **Action**: Settings "Show welcome", and the cheatsheet keyboard row.
- **Validate**: both open it; choices made there still save.

### Task 7: Docs ✅
- **Done**: README paragraph and screenshot (`docs/media/onboarding.png`), CHANGELOG, PRD A8 and milestone 13.
- **Action**: PRD A8 and milestone 13; a README line and screenshot; CHANGELOG.
- **Validate**: `scripts/check.sh`.

## Validation
```bash
scripts/check.sh
XDG_CONFIG_HOME=$S/cfg XDG_DATA_HOME=$S/data target/release/notecal   # fresh-install run
```
By hand:
- keyboard only, through every step;
- Indonesian;
- light and dark;
- a 600×400 window;
- a bad notes path (the error shows on the step);
- skip at step 1 → defaults and the welcome note.

## Risks
| Risk | Likelihood | Mitigation |
|---|---|---|
| `save_state` overwriting the onboarding flag | High (it replaces the file today) | merge in `save_state`; test |
| Existing users see it after upgrading | Medium | a missing field means done; only a missing file means fresh |
| Changing the notes folder mid-onboarding switches notes under it | Medium | apply the folder on Next, through the existing `notes-dir-changed` path; the welcome note goes into the final folder |
| Startup time (cold start budget 400 ms) | Low | the overlay is a lazy `import()`, loaded only when pending |
| Too many steps feel like work | Medium | 6 steps; defaults are sensible; Skip on every step; Next works without choosing |

## Decisions (author, 2026-11-20)
1. **Notes folder:** a typed path, validated like Settings; no new dependency.
2. **Welcome note:** created by default (the checkbox on the last step is ticked).
3. **Skip:** keeps the defaults and still creates the welcome note.

## Acceptance
- [ ] Fresh install shows the onboarding; existing installs don't
- [ ] Every choice applies live and is in `settings.toml`
- [ ] Skip anywhere keeps defaults and never shows it again; quitting part-way shows it again
- [ ] Keys step matches the platform (bind line on Wayland, shortcut field elsewhere; pin only where it works)
- [ ] Welcome note with live examples (when kept)
- [ ] Reopen from Settings and the cheatsheet
- [ ] Keyboard-only, en and id, both themes, small window
- [ ] Cold start unchanged for existing users; `scripts/check.sh` passes

## Results (2026-11-20)

- `scripts/check.sh` passes: 104 UI tests, plus the Rust state and welcome-layout tests.
- Tried in the release build with a fresh XDG profile on Hyprland:
  - Light and Indonesian both applied live.
  - One Alt+→ moves one step. An earlier double jump came from nothing being focused on the Welcome step; that step now focuses Get started.
- The fresh Scratch note still holds the PRD's sample note, as before. The Welcome note is added alongside it.
- **Not checked by hand:**
  - the Settings button (the cheatsheet route was checked);
  - a window smaller than 720×520;
  - a bad notes path on the Your data step;
  - the shortcut field off Wayland (X11, macOS, Windows);
  - screen-reader output.
