//! Showing and hiding the window (PRD A2 and key scenario 1). On Wayland a
//! global shortcut cannot be registered, so the compositor binds
//! `notecal --toggle`, which reaches the running instance through the
//! single-instance plugin.

use std::sync::Mutex;

use tauri::{AppHandle, Manager, State, WebviewWindow};

use crate::settings::AppSettings;

pub const TOGGLE_FLAG: &str = "--toggle";

fn main_window(app: &AppHandle) -> Option<WebviewWindow> {
    app.get_webview_window("main")
}

/// Shows, unminimizes and focuses the window.
pub fn show(app: &AppHandle) {
    let Some(window) = main_window(app) else {
        return;
    };
    let steps = [window.show(), window.unminimize(), window.set_focus()];
    for result in steps {
        if let Err(e) = result {
            eprintln!("notecal: could not show the window: {e}");
        }
    }
}

/// Hides the window if it is visible and focused; otherwise shows it.
pub fn toggle(app: &AppHandle) {
    let Some(window) = main_window(app) else {
        return;
    };
    let visible = window.is_visible().unwrap_or(false);
    let focused = window.is_focused().unwrap_or(false);
    if visible && focused {
        if let Err(e) = window.hide() {
            eprintln!("notecal: could not hide the window: {e}");
        }
    } else {
        show(app);
    }
}

/// Global shortcuts cannot be registered by apps on Wayland.
pub fn is_wayland() -> bool {
    cfg!(target_os = "linux") && std::env::var_os("WAYLAND_DISPLAY").is_some()
}

/// What the Settings screen says about the global shortcut.
#[derive(Default)]
pub struct ShortcutStatus(Mutex<String>);

/// Applies the window-related settings: always-on-top (A4) and the global
/// shortcut (A1).
pub fn apply_window_settings(app: &AppHandle, settings: &AppSettings) {
    if let Some(window) = main_window(app)
        && let Err(e) = window.set_always_on_top(settings.always_on_top)
    {
        eprintln!("notecal: could not set always-on-top: {e}");
    }
    let status = register_shortcut(app, &settings.shortcut);
    *app.state::<ShortcutStatus>()
        .0
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner) = status;
}

fn register_shortcut(app: &AppHandle, shortcut: &str) -> String {
    if is_wayland() {
        return "On Wayland, bind `notecal --toggle` to a key in your compositor \
                (see docs/linux-setup.md)."
            .into();
    }
    use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};
    let shortcuts = app.global_shortcut();
    if let Err(e) = shortcuts.unregister_all() {
        eprintln!("notecal: could not clear the old shortcut: {e}");
    }
    match shortcuts.on_shortcut(shortcut, |app, _shortcut, event| {
        if event.state == ShortcutState::Pressed {
            toggle(app);
        }
    }) {
        Ok(()) => format!("{shortcut} shows and hides notecal."),
        Err(e) => format!("Could not register {shortcut}: {e}"),
    }
}

#[tauri::command]
pub fn shortcut_status(status: State<'_, ShortcutStatus>) -> String {
    status
        .0
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .clone()
}

/// What a second `notecal` launch asks the running instance to do.
pub fn on_second_instance(app: &AppHandle, args: &[String]) {
    if args.iter().any(|a| a == TOGGLE_FLAG) {
        toggle(app);
    } else {
        show(app);
    }
}
