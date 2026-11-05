mod commands;
mod evaluator;
mod notes;
mod state;
mod watch;

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use commands::Notes;
use evaluator::{EvalRequest, EvalResponse, Evaluator};
use tauri::{Manager, State};
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
    tauri::Builder::default()
        .manage(startup)
        .manage(Arc::new(Evaluator::default()))
        .setup(|app| {
            let paths = app.path();
            let notes = Notes::new(
                commands::notes_dir(&paths.data_dir()?),
                commands::state_path(&paths.config_dir()?),
            )?;
            let dir = notes.dir();
            app.manage(notes);
            let handle = app.handle().clone();
            let watch = watch::watch_dir(&dir, move |changed| {
                handle.state::<Notes>().on_paths_changed(&handle, &changed);
            })?;
            app.manage(Watches(Mutex::new(vec![watch])));
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
        ])
        .run(tauri::generate_context!())
        .expect("failed to start the notecal window");
}
