mod commands;
mod evaluator;
mod notes;
mod settings;
mod shell;
mod state;
mod theme;
mod watch;

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use commands::{Notes, SettingsState, ThemeSource};
use evaluator::{EvalRequest, EvalResponse, Evaluator};
use tauri::{Manager, State};
use tauri_plugin_window_state::StateFlags;
use watch::Watch;

/// File watches that must live as long as the app.
struct Watches(#[allow(dead_code)] Mutex<Vec<Watch>>);

/// When `main` started, for the cold-start measurement (`mark_ready`).
struct Startup {
    at: Instant,
    reported: AtomicBool,
}

#[tauri::command]
async fn evaluate(
    request: EvalRequest,
    evaluator: State<'_, Arc<Evaluator>>,
) -> Result<Option<EvalResponse>, String> {
    evaluator.announce(request.version);
    let evaluator = Arc::clone(&evaluator);
    tauri::async_runtime::spawn_blocking(move || evaluator.evaluate(&request))
        .await
        .map_err(|e| format!("evaluation task failed: {e}"))
}

/// Called by the UI when the editor first has focus. Logs time since `main`
/// and the wall-clock time, so a launcher script can also measure from
/// process start (see docs/gate1.md).
#[tauri::command]
fn mark_ready(startup: State<'_, Startup>) {
    if startup.reported.swap(true, Ordering::SeqCst) {
        return;
    }
    let since_main = startup.at.elapsed().as_millis();
    let epoch_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_millis());
    eprintln!("notecal: ready {since_main} ms after main (epoch_ms={epoch_ms})");
}

/// WebKitGTK's DMABUF renderer kept the app at ~11% CPU while idle on
/// Hyprland; without it idle CPU is ~1%, and cold start and RAM also drop
/// (docs/gate1.md). Respects an explicit setting, so
/// `WEBKIT_DISABLE_DMABUF_RENDERER=0` turns the renderer back on.
#[cfg(target_os = "linux")]
fn configure_webkit() {
    const VAR: &str = "WEBKIT_DISABLE_DMABUF_RENDERER";
    if std::env::var_os(VAR).is_none() {
        // SAFETY: called first thing in `main`, before Tauri or anything
        // else has started a thread, so nothing reads the environment
        // concurrently.
        #[allow(unsafe_code)]
        unsafe {
            std::env::set_var(VAR, "1");
        }
    }
}

fn main() {
    let startup = Startup {
        at: Instant::now(),
        reported: AtomicBool::new(false),
    };
    #[cfg(target_os = "linux")]
    configure_webkit();
    let mut builder = tauri::Builder::default()
        // Must be the first plugin: a second launch hands its arguments to
        // this instance and exits before anything else starts.
        .plugin(tauri_plugin_single_instance::init(|app, args, _cwd| {
            shell::on_second_instance(app, &args);
        }));
    // On Wayland the compositor owns window geometry, stacking and global
    // keys: the window-state plugin saved wrong sizes and a bogus
    // "maximized" there, and shortcuts cannot be registered. Compositor
    // rules and `notecal --toggle` cover both (docs/linux-setup.md).
    if !shell::is_wayland() {
        builder = builder
            // A5. Visibility is not restored: notecal may have been hidden when it quit.
            .plugin(
                tauri_plugin_window_state::Builder::default()
                    .with_state_flags(StateFlags::all() - StateFlags::VISIBLE)
                    .build(),
            )
            .plugin(tauri_plugin_global_shortcut::Builder::new().build());
    }
    builder
        .manage(startup)
        .manage(Arc::new(Evaluator::default()))
        .manage(shell::ShortcutStatus::default())
        .setup(|app| {
            let paths = app.path();
            let config_dir = paths.config_dir()?;
            let settings = SettingsState::new(commands::settings_path(&config_dir));
            let current = settings.view().settings;
            shell::apply_window_settings(app.handle(), &current);
            if let Err(e) = shell::create_tray(app.handle()) {
                // No tray host or no appindicator library: not fatal.
                eprintln!("notecal: no tray icon: {e}");
            }
            app.state::<Arc<Evaluator>>().set_settings(current.engine());
            let notes = Notes::new(
                commands::resolve_notes_dir(&current, &paths.data_dir()?),
                commands::state_path(&config_dir),
            )?;
            let dir = notes.dir();
            app.manage(notes);
            let handle = app.handle().clone();
            let watch = watch::watch_dir(&dir, move |changed| {
                handle.state::<Notes>().on_paths_changed(&handle, &changed);
            })?;
            let mut watches = vec![watch];

            // Hand edits of settings.toml apply live.
            let settings_dir = settings
                .path()
                .parent()
                .map(std::path::Path::to_path_buf)
                .unwrap_or_default();
            std::fs::create_dir_all(&settings_dir)?;
            app.manage(settings);
            let handle = app.handle().clone();
            watches.push(watch::watch_dir(&settings_dir, move |changed| {
                handle
                    .state::<SettingsState>()
                    .on_paths_changed(&handle, &changed);
            })?);

            let omarchy_current = theme::omarchy_current_dir(&paths.home_dir()?);
            // Managed before the watch starts: its callback reads this state.
            app.manage(ThemeSource {
                omarchy_current: omarchy_current.clone(),
            });
            if omarchy_current.is_dir() {
                let handle = app.handle().clone();
                match watch::watch_dir(&omarchy_current, move |changed| {
                    handle
                        .state::<ThemeSource>()
                        .on_paths_changed(&handle, &changed);
                }) {
                    Ok(watch) => watches.push(watch),
                    // Theme following is a nicety; the app works without it.
                    Err(e) => eprintln!("notecal: {e}"),
                }
            }
            app.manage(Watches(Mutex::new(watches)));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            evaluate,
            mark_ready,
            commands::list_notes,
            commands::open_note,
            commands::save_note,
            commands::create_note,
            commands::rename_note,
            commands::delete_note,
            commands::load_state,
            commands::save_state,
            commands::get_theme,
            commands::get_settings,
            commands::save_settings,
            shell::shortcut_status,
            shell::quit,
        ])
        .run(tauri::generate_context!())
        .expect("failed to start the notecal window");
}
