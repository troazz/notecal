//! Showing and hiding the window (PRD A2 and key scenario 1). On Wayland a
//! global shortcut cannot be registered, so the compositor binds
//! `notecal --toggle`, which reaches the running instance through the
//! single-instance plugin.

use tauri::{AppHandle, Manager, WebviewWindow};

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

/// What a second `notecal` launch asks the running instance to do.
pub fn on_second_instance(app: &AppHandle, args: &[String]) {
    if args.iter().any(|a| a == TOGGLE_FLAG) {
        toggle(app);
    } else {
        show(app);
    }
}
