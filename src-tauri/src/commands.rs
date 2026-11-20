//! Note and UI-state commands, plus the bridge from file-watch events to the
//! UI. Storage rules live in `notes.rs`; this layer only locks and routes.

pub use notecal_env::paths::{notes_dir, rates_path, resolve_notes_dir, settings_path, state_path};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard};

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, State};

use crate::evaluator::Evaluator;
use crate::notes::{DiskChange, NoteInfo, NoteStore, SaveOutcome};
use crate::rates::{self, Fetch, RatesStatus, RatesStore};
use crate::settings::{self, AppSettings, SettingsView, UiLanguage};
use crate::state::{self, UiState};
use crate::theme::{self, Theme};
use crate::watch::{self, Watch};

/// Shown in the first note on a fresh install (the PRD's example note).
const SCRATCH_NAME: &str = "Scratch";
const SCRATCH_TEXT: &str = "\
# Monthly budget
rent = 4,500,000
electricity = 350,000
internet = 450,000
total
20% of prev

# Conversions
5 km in miles
3 days 4 hours in minutes
1.5 GB in MB
0xff in binary
";

/// Emitted when the open note's file was changed or deleted by something
/// other than this app.
pub const NOTE_CHANGED_ON_DISK: &str = "note-changed-on-disk";
/// Emitted when any note file was added, removed or changed.
pub const NOTES_LIST_CHANGED: &str = "notes-list-changed";
/// Emitted after the notes folder setting changed; the UI reopens a note.
pub const NOTES_DIR_CHANGED: &str = "notes-dir-changed";

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DiskChangeEvent {
    pub name: String,
    /// `"changed"` or `"deleted"`.
    pub kind: &'static str,
}

struct Inner {
    store: NoteStore,
    /// The note the editor shows, for routing watch events.
    open: Option<String>,
}

pub struct Notes {
    inner: Mutex<Inner>,
    state_path: PathBuf,
    /// `state.json` is read, changed and written back; one writer at a time.
    state_lock: Mutex<()>,
    /// Watch on the notes folder; replaced when the folder changes.
    watch: Mutex<Option<Watch>>,
}

fn notes_watch(app: &AppHandle, dir: &Path) -> Result<Watch, String> {
    let handle = app.clone();
    watch::watch_dir(dir, move |changed| {
        handle.state::<Notes>().on_paths_changed(&handle, &changed);
    })
}

/// Opens a notes folder, seeding `Scratch` when it holds no notes.
fn open_store(dir: PathBuf) -> Result<NoteStore, String> {
    let mut store = NoteStore::new(dir)?;
    if store.list()?.is_empty() {
        store.create(SCRATCH_NAME)?;
        store.save(SCRATCH_NAME, SCRATCH_TEXT, "")?;
    }
    Ok(store)
}

impl Notes {
    pub fn new(notes_dir: PathBuf, state_path: PathBuf) -> Result<Self, String> {
        Ok(Self {
            inner: Mutex::new(Inner {
                store: open_store(notes_dir)?,
                open: None,
            }),
            state_path,
            state_lock: Mutex::new(()),
            watch: Mutex::new(None),
        })
    }

    /// Starts (or restarts) watching the current notes folder.
    pub fn start_watch(&self, app: &AppHandle) -> Result<(), String> {
        let watch = notes_watch(app, &self.dir())?;
        self.set_watch(watch);
        Ok(())
    }

    fn set_watch(&self, watch: Watch) {
        *self
            .watch
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = Some(watch);
    }

    /// Switches to another notes folder (the `notes_dir` setting). Notes are
    /// not moved; the UI saves pending edits before changing the setting.
    pub fn relocate(&self, app: &AppHandle, dir: PathBuf) -> Result<(), String> {
        if dir == self.dir() {
            return Ok(());
        }
        // Everything that can fail happens before the switch.
        let store = open_store(dir.clone())?;
        let watch = notes_watch(app, &dir)?;
        *self.lock() = Inner { store, open: None };
        self.set_watch(watch);
        Ok(())
    }

    pub fn dir(&self) -> PathBuf {
        self.lock().store.dir().to_path_buf()
    }

    /// A panic while holding the lock cannot leave the store inconsistent
    /// (each operation updates its record after the file write), so a
    /// poisoned lock is recovered rather than failing every later command.
    fn lock(&self) -> MutexGuard<'_, Inner> {
        self.inner
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    /// Handles watch events for the notes folder.
    pub fn on_paths_changed(&self, app: &AppHandle, paths: &[PathBuf]) {
        let mut inner = self.lock();
        let names: Vec<String> = paths
            .iter()
            .filter_map(|p| inner.store.name_of(p))
            .collect();
        if names.is_empty() {
            return;
        }
        if let Some(open) = inner.open.clone().filter(|o| names.contains(o)) {
            let kind = match inner.store.disk_changed(&open) {
                Ok(DiskChange::Changed(_)) => Some("changed"),
                Ok(DiskChange::Deleted) => {
                    inner.store.mark_missing(&open);
                    Some("deleted")
                }
                Ok(DiskChange::Unchanged) => None,
                Err(e) => {
                    eprintln!("notecal: {e}");
                    None
                }
            };
            if let Some(kind) = kind {
                emit(
                    app,
                    NOTE_CHANGED_ON_DISK,
                    DiskChangeEvent { name: open, kind },
                );
            }
        }
        drop(inner);
        emit(app, NOTES_LIST_CHANGED, ());
    }
}

fn emit(app: &AppHandle, event: &str, payload: impl Serialize + Clone) {
    if let Err(e) = app.emit(event, payload) {
        eprintln!("notecal: could not send {event}: {e}");
    }
}

/// The current settings, the user's definitions, and where they live.
pub struct SettingsState {
    path: PathBuf,
    definitions_path: PathBuf,
    current: Mutex<SettingsView>,
    /// Text of `definitions.txt`, evaluated before every note.
    definitions: Mutex<String>,
}

impl SettingsState {
    pub fn new(path: PathBuf) -> Self {
        let definitions_path = path.with_file_name(settings::DEFINITIONS_FILE);
        let definitions = settings::load_definitions(&definitions_path);
        let mut view = settings::load(&path);
        view.problems
            .extend(settings::definition_problems(&view.settings, &definitions));
        for problem in &view.problems {
            eprintln!("notecal: {problem}");
        }
        Self {
            path,
            definitions_path,
            current: Mutex::new(view),
            definitions: Mutex::new(definitions),
        }
    }

    fn definitions(&self) -> String {
        self.definitions
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone()
    }

    /// The interface language: the setting, or the system's when `auto`.
    pub fn ui_language(&self) -> UiLanguage {
        self.view()
            .settings
            .language
            .resolve(sys_locale::get_locale().as_deref())
    }

    /// Engine settings: the user's settings plus their definitions.
    pub fn engine(&self) -> notecal_core::Settings {
        self.view()
            .settings
            .engine(&self.definitions(), self.ui_language())
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn view(&self) -> SettingsView {
        self.current
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone()
    }

    /// Stores `view` and applies it; emits `settings-changed` if it differs.
    /// `definitions_changed` applies it even when the view is the same.
    fn apply(&self, app: &AppHandle, mut view: SettingsView, definitions_changed: bool) {
        let definitions = self.definitions();
        view.problems
            .extend(settings::definition_problems(&view.settings, &definitions));
        let old = self.view();
        if old == view && !definitions_changed {
            return;
        }
        // A folder that cannot be used keeps the old one, so the app state
        // never disagrees with the folder actually in use.
        if old.settings.notes_dir != view.settings.notes_dir
            && let Err(e) = relocate_notes(app, &view.settings)
        {
            view.problems
                .push(format!("{e} The notes folder was not changed."));
            view.settings.notes_dir = old.settings.notes_dir.clone();
        }
        *self
            .current
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = view.clone();
        app.state::<Arc<Evaluator>>().set_settings(
            view.settings.engine(
                &definitions,
                view.settings
                    .language
                    .resolve(sys_locale::get_locale().as_deref()),
            ),
        );
        crate::shell::apply_window_settings(app, &view.settings);
        if old.settings.theme != view.settings.theme {
            app.state::<ThemeSource>().emit_current(app);
        }
        if old.settings.language != view.settings.language {
            crate::shell::set_tray_language(app, self.ui_language());
        }
        emit(app, settings::SETTINGS_CHANGED, view);
    }

    /// Records a problem found outside the file (e.g. an unusable notes folder at startup).
    pub fn add_problem(&self, problem: String) {
        eprintln!("notecal: {problem}");
        self.current
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .problems
            .push(problem);
    }

    /// Handles watch events in the config folder (hand edits of
    /// settings.toml and definitions.txt).
    pub fn on_paths_changed(&self, app: &AppHandle, paths: &[PathBuf]) {
        let definitions_changed = paths.iter().any(|p| p == &self.definitions_path);
        if definitions_changed {
            *self
                .definitions
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner) =
                settings::load_definitions(&self.definitions_path);
        }
        if definitions_changed || paths.iter().any(|p| p == &self.path) {
            self.apply(app, settings::load(&self.path), definitions_changed);
        }
    }
}

/// Exchange rates: the on-disk cache and how to refresh it.
pub struct RatesState {
    store: Mutex<RatesStore>,
    fetcher: Arc<dyn Fetch>,
}

impl RatesState {
    pub fn new(store: RatesStore, fetcher: Arc<dyn Fetch>) -> Self {
        Self {
            store: Mutex::new(store),
            fetcher,
        }
    }

    pub fn initial_rates(&self) -> Option<notecal_core::Rates> {
        self.lock().rates()
    }

    fn lock(&self) -> MutexGuard<'_, RatesStore> {
        self.store
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    /// After an evaluation: fetch in the background if the note uses a
    /// currency and the cache is missing or a day old (PRD C9).
    pub fn maybe_refresh(&self, app: &AppHandle, text: &str) {
        let enabled = app
            .state::<SettingsState>()
            .view()
            .settings
            .currency_refresh;
        if !self.lock().start_refresh(text, enabled, now_secs()) {
            return;
        }
        let fetcher = Arc::clone(&self.fetcher);
        let app = app.clone();
        let spawned = std::thread::Builder::new()
            .name("notecal-rates".into())
            .spawn(move || {
                // A panic must still end the attempt, or refreshes stop for good.
                let result =
                    std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| fetcher.fetch()))
                        .unwrap_or_else(|_| Err("The exchange-rate fetch crashed.".into()));
                let state = app.state::<RatesState>();
                let finished = state.lock().finish(result, now_secs());
                match finished {
                    Ok(new_rates) => {
                        app.state::<Arc<Evaluator>>().set_rates(Some(new_rates));
                        emit(&app, rates::RATES_UPDATED, state.lock().status());
                    }
                    Err(e) => eprintln!("notecal: {e}"),
                }
            });
        if let Err(e) = spawned {
            let _ = self.lock().finish(
                Err(format!("Could not start the rates fetch: {e}")),
                now_secs(),
            );
        }
    }
}

fn now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}

/// Currency codes, for autocomplete.
#[tauri::command]
pub fn currency_codes() -> &'static [&'static str] {
    notecal_core::currency::CODES
}

#[tauri::command]
pub fn rates_status(state: State<'_, RatesState>) -> RatesStatus {
    state.lock().status()
}

/// Points the notes store and its watch at the folder from settings.
fn relocate_notes(app: &AppHandle, settings: &AppSettings) -> Result<(), String> {
    let data_dir = app.path().data_dir().map_err(|e| e.to_string())?;
    let dir = resolve_notes_dir(settings, &data_dir);
    app.state::<Notes>().relocate(app, dir)?;
    emit(app, NOTES_DIR_CHANGED, ());
    Ok(())
}

#[tauri::command]
pub fn get_settings(state: State<'_, SettingsState>) -> SettingsView {
    state.view()
}

/// The interface language to show (`en` or `id`).
#[tauri::command]
pub fn ui_language(state: State<'_, SettingsState>) -> UiLanguage {
    state.ui_language()
}

#[tauri::command]
pub async fn save_settings(
    settings: AppSettings,
    app: AppHandle,
    state: State<'_, SettingsState>,
) -> Result<(), String> {
    settings::validate(&settings)?;
    // Switch folders before saving, so an unusable folder is never written
    // to settings.toml (where it would break the next launch).
    if state.view().settings.notes_dir != settings.notes_dir {
        relocate_notes(&app, &settings)?;
    }
    settings::save(&state.path, &settings)?;
    state.apply(
        &app,
        SettingsView {
            settings,
            problems: Vec::new(),
        },
        false,
    );
    Ok(())
}

// The commands below touch the disk (and `fsync`), so they are `async`:
// Tauri runs sync commands on the main thread, where a slow write would
// stall the window's frames while typing.

#[tauri::command]
pub async fn list_notes(notes: State<'_, Notes>) -> Result<Vec<NoteInfo>, String> {
    notes.lock().store.list()
}

#[tauri::command]
pub async fn open_note(name: String, notes: State<'_, Notes>) -> Result<String, String> {
    let mut inner = notes.lock();
    let text = inner.store.open(&name)?;
    inner.open = Some(name);
    Ok(text)
}

#[tauri::command]
pub async fn save_note(
    name: String,
    text: String,
    stamp: String,
    notes: State<'_, Notes>,
) -> Result<SaveOutcome, String> {
    notes.lock().store.save(&name, &text, &stamp)
}

#[tauri::command]
pub async fn create_note(name: String, notes: State<'_, Notes>) -> Result<(), String> {
    notes.lock().store.create(&name)
}

#[tauri::command]
pub async fn rename_note(from: String, to: String, notes: State<'_, Notes>) -> Result<(), String> {
    let mut inner = notes.lock();
    inner.store.rename(&from, &to)?;
    if inner.open.as_deref() == Some(from.as_str()) {
        inner.open = Some(to);
    }
    Ok(())
}

#[tauri::command]
pub async fn delete_note(name: String, notes: State<'_, Notes>) -> Result<(), String> {
    let mut inner = notes.lock();
    inner.store.delete(&name)?;
    if inner.open.as_deref() == Some(name.as_str()) {
        inner.open = None;
    }
    Ok(())
}

/// Where "auto" looks for Omarchy's current theme.
pub struct ThemeSource {
    pub omarchy_current: PathBuf,
    /// `~/.config/notecal/themes`, the user's palette files.
    pub themes_dir: PathBuf,
}

impl ThemeSource {
    /// The palette for the current `theme` setting; a broken user theme is
    /// reported and falls back to "auto".
    pub fn current(&self, app: &AppHandle) -> Option<Theme> {
        let choice = app.state::<SettingsState>().view().settings.theme;
        theme::resolve(&choice, &self.omarchy_current, &self.themes_dir).unwrap_or_else(|e| {
            eprintln!("notecal: {e}");
            theme::load_auto(&self.omarchy_current)
        })
    }

    pub fn emit_current(&self, app: &AppHandle) {
        emit(app, theme::THEME_CHANGED, self.current(app));
    }

    /// Handles watch events in Omarchy's current folder.
    pub fn on_omarchy_changed(&self, app: &AppHandle, paths: &[PathBuf]) {
        let follows_omarchy = app.state::<SettingsState>().view().settings.theme == "auto";
        if follows_omarchy
            && paths
                .iter()
                .any(|p| theme::is_theme_swap(&self.omarchy_current, p))
        {
            self.emit_current(app);
        }
    }

    /// Handles watch events in the user's themes folder (editing the chosen theme repaints).
    pub fn on_user_themes_changed(&self, app: &AppHandle, paths: &[PathBuf]) {
        let choice = app.state::<SettingsState>().view().settings.theme;
        let chosen = theme::user_theme_path(&self.themes_dir, &choice);
        if paths.contains(&chosen) {
            self.emit_current(app);
        }
    }
}

/// The palette to use, or `None` for the built-in one of the system mode.
#[tauri::command]
pub fn get_theme(app: AppHandle, source: State<'_, ThemeSource>) -> Option<Theme> {
    source.current(&app)
}

/// Names for the theme picker: user theme files (built-ins are added by the UI).
#[tauri::command]
pub fn list_themes(source: State<'_, ThemeSource>) -> Vec<String> {
    theme::list_user_themes(&source.themes_dir)
}

/// A theme's palette for a preview card in the onboarding (A8), without
/// choosing it. `None` for `auto` without Omarchy (the built-in palette).
#[tauri::command]
pub fn preview_theme(
    name: String,
    source: State<'_, ThemeSource>,
) -> Result<Option<Theme>, String> {
    // Same rule as the `theme` setting: a plain file name, never a path.
    if name.is_empty() || name.starts_with('.') || name.contains(['/', '\\']) {
        return Err(format!("'{name}' is not a theme name"));
    }
    theme::resolve(&name, &source.omarchy_current, &source.themes_dir)
}

#[tauri::command]
pub fn load_state(notes: State<'_, Notes>) -> UiState {
    state::load(&notes.state_path)
}

#[tauri::command]
pub async fn save_state(ui_state: UiState, notes: State<'_, Notes>) -> Result<(), String> {
    let _guard = notes
        .state_lock
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    state::save_position(&notes.state_path, &ui_state)
}

/// The first-run onboarding was finished or skipped (A8).
#[tauri::command]
pub async fn finish_onboarding(notes: State<'_, Notes>) -> Result<(), String> {
    let _guard = notes
        .state_lock
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    state::finish_onboarding(&notes.state_path)
}
