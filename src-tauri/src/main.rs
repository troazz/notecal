mod evaluator;

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use evaluator::{EvalRequest, EvalResponse, Evaluator};
use tauri::State;

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

fn main() {
    let startup = Startup {
        at: Instant::now(),
        reported: AtomicBool::new(false),
    };
    tauri::Builder::default()
        .manage(startup)
        .manage(Arc::new(Evaluator::default()))
        .invoke_handler(tauri::generate_handler![evaluate, mark_ready])
        .run(tauri::generate_context!())
        .expect("failed to start the notecal window");
}
