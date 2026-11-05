//! Showing and hiding the window (PRD A2 and key scenario 1). On Wayland a
//! global shortcut cannot be registered, so the compositor binds
//! `notecal --toggle`, which reaches the running instance through the
//! single-instance plugin.

use std::sync::Mutex;

use std::time::Duration;

use tauri::menu::{Menu, MenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Emitter, Manager, State, WebviewWindow};

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

/// Asks the UI to save, then quit; the UI calls `quit` when done.
pub const QUIT_REQUESTED: &str = "quit-requested";
/// Quit anyway if the UI does not answer (e.g. it is stuck).
const QUIT_GRACE: Duration = Duration::from_secs(3);

/// Tray icon with Show/Hide and Quit (PRD A3). Without a tray host the icon
/// simply does not appear; `notecal --toggle` still works.
pub fn create_tray(app: &AppHandle) -> tauri::Result<()> {
    let toggle_item = MenuItem::with_id(app, "toggle", "Show / Hide notecal", true, None::<&str>)?;
    let quit_item = MenuItem::with_id(app, "quit", "Quit notecal", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&toggle_item, &quit_item])?;
    let mut tray = TrayIconBuilder::with_id("main")
        .tooltip("notecal")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "toggle" => toggle(app),
            "quit" => request_quit(app),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                toggle(tray.app_handle());
            }
        });
    if let Some(icon) = app.default_window_icon() {
        tray = tray.icon(icon.clone());
    }
    tray.build(app)?;
    Ok(())
}

/// Lets the UI save pending edits before the process exits.
pub fn request_quit(app: &AppHandle) {
    if app.emit(QUIT_REQUESTED, ()).is_err() {
        app.exit(0);
        return;
    }
    let app = app.clone();
    std::thread::spawn(move || {
        std::thread::sleep(QUIT_GRACE);
        app.exit(0);
    });
}

/// Called by the UI once edits are saved.
#[tauri::command]
pub fn quit(app: AppHandle) {
    app.exit(0);
}

/// What a second `notecal` launch asks the running instance to do.
pub fn on_second_instance(app: &AppHandle, args: &[String]) {
    if args.iter().any(|a| a == TOGGLE_FLAG) {
        toggle(app);
    } else {
        show(app);
    }
}
