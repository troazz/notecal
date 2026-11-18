//! Debounced file watching (PRD storage rules: "watched with a file watcher,
//! not by polling", so idle CPU stays near zero).

use std::path::{Path, PathBuf};
use std::time::Duration;

use notify_debouncer_full::notify::{RecommendedWatcher, RecursiveMode};
use notify_debouncer_full::{DebounceEventResult, Debouncer, RecommendedCache, new_debouncer};

/// Keeps a watch alive; dropping it stops the watch.
pub type Watch = Debouncer<RecommendedWatcher, RecommendedCache>;

/// Events within this window are delivered together. The debouncer checks
/// for them on a timer of the same length: its default (a quarter of the
/// window) woke each watcher 40 times a second, about 1% CPU at idle
/// across the app's five watches (Gate 2).
const DEBOUNCE: Duration = Duration::from_millis(200);

/// Calls `on_change` with the paths touched in `dir` (not recursive), at
/// most once per debounce window. Watch errors are logged and ignored: a
/// lost event only delays noticing an external change until the next save,
/// which still checks the file before writing.
pub fn watch_dir(
    dir: &Path,
    mut on_change: impl FnMut(Vec<PathBuf>) + Send + 'static,
) -> Result<Watch, String> {
    let mut debouncer = new_debouncer(
        DEBOUNCE,
        Some(DEBOUNCE),
        move |result: DebounceEventResult| match result {
            Ok(events) => {
                let mut paths: Vec<PathBuf> =
                    events.into_iter().flat_map(|e| e.event.paths).collect();
                paths.sort();
                paths.dedup();
                if !paths.is_empty() {
                    on_change(paths);
                }
            }
            Err(errors) => {
                for error in errors {
                    eprintln!("notecal: file watch error: {error}");
                }
            }
        },
    )
    .map_err(|e| format!("Could not start watching {}: {e}", dir.display()))?;
    debouncer
        .watch(dir, RecursiveMode::NonRecursive)
        .map_err(|e| format!("Could not watch {}: {e}", dir.display()))?;
    Ok(debouncer)
}
