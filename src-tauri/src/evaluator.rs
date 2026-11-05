//! Runs note evaluation off the UI thread and skips requests that a newer
//! request has already replaced (PRD "Evaluation flow").

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, MutexGuard};

use notecal_core::{Engine, LineResult, Locale, Settings};
use serde::{Deserialize, Serialize};

/// One `evaluate` call from the editor.
#[derive(Debug, Clone, Deserialize)]
pub struct EvalRequest {
    /// The editor's document version; higher is newer.
    pub version: u64,
    pub text: String,
    pub locale: LocaleDto,
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LocaleDto {
    En,
    Id,
}

impl From<LocaleDto> for Locale {
    fn from(locale: LocaleDto) -> Self {
        match locale {
            LocaleDto::En => Locale::En,
            LocaleDto::Id => Locale::Id,
        }
    }
}

/// Results for one request, one entry per line of its text.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct EvalResponse {
    pub version: u64,
    pub lines: Vec<LineDto>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct LineDto {
    pub display: Option<String>,
    pub copy: Option<String>,
    pub error: Option<String>,
}

impl From<LineResult> for LineDto {
    fn from(line: LineResult) -> Self {
        Self {
            display: line.display,
            copy: line.copy,
            error: line.error,
        }
    }
}

/// Owns the open note's engine (and its cache) and the newest version seen.
#[derive(Default)]
pub struct Evaluator {
    engine: Mutex<Engine>,
    latest: AtomicU64,
}

impl Evaluator {
    /// Records `version` as the newest request. Called as soon as a request
    /// arrives, before it waits for the engine.
    pub fn announce(&self, version: u64) {
        self.latest.fetch_max(version, Ordering::SeqCst);
    }

    /// Evaluates the request, or returns `None` if a newer one was announced
    /// before or while it waited for the engine.
    pub fn evaluate(&self, request: &EvalRequest) -> Option<EvalResponse> {
        if self.superseded(request.version) {
            return None;
        }
        let mut engine = self.lock_engine();
        if self.superseded(request.version) {
            return None;
        }
        let settings = Settings {
            locale: request.locale.into(),
            ..Settings::default()
        };
        let lines = engine.evaluate(&request.text, &settings, None);
        Some(EvalResponse {
            version: request.version,
            lines: lines.into_iter().map(LineDto::from).collect(),
        })
    }

    fn superseded(&self, version: u64) -> bool {
        version < self.latest.load(Ordering::SeqCst)
    }

    /// A panic while holding the lock poisons it. The engine already isolates
    /// panics per line, but if one ever escapes, start from a fresh engine
    /// rather than leaving the app unable to evaluate.
    fn lock_engine(&self) -> MutexGuard<'_, Engine> {
        self.engine.lock().unwrap_or_else(|poisoned| {
            let mut engine = poisoned.into_inner();
            *engine = Engine::default();
            self.engine.clear_poison();
            engine
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use notecal_core::evaluate_note;

    fn request(version: u64, text: &str) -> EvalRequest {
        EvalRequest {
            version,
            text: text.to_string(),
            locale: LocaleDto::En,
        }
    }

    #[test]
    fn evaluates_requests_in_order() {
        let evaluator = Evaluator::default();
        for (version, text) in [(1, "1 + 1"), (2, "1 + 2"), (3, "1 + 2\nprev * 2")] {
            evaluator.announce(version);
            let response = evaluator.evaluate(&request(version, text)).unwrap();
            assert_eq!(response.version, version);
        }
    }

    #[test]
    fn skips_superseded_requests() {
        let evaluator = Evaluator::default();
        evaluator.announce(1);
        evaluator.announce(2);
        assert_eq!(evaluator.evaluate(&request(1, "1 + 1")), None);
        assert!(evaluator.evaluate(&request(2, "1 + 1")).is_some());
    }

    #[test]
    fn matches_core_evaluation() {
        let evaluator = Evaluator::default();
        let text = "rent = 4,500,000\nrent * 2\n# h\n5 km in miles";
        evaluator.announce(7);
        let response = evaluator.evaluate(&request(7, text)).unwrap();
        let expected: Vec<LineDto> = evaluate_note(text, &Settings::default(), None)
            .into_iter()
            .map(LineDto::from)
            .collect();
        assert_eq!(response.lines, expected);
    }

    #[test]
    fn engine_can_live_in_tauri_state() {
        fn assert_send_sync<T: Send + Sync + 'static>() {}
        assert_send_sync::<Evaluator>();
    }
}
