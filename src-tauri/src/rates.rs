//! Exchange rates (PRD C9): fetched from open.er-api.com at most once a day,
//! only when a note uses a currency, and cached on disk so conversion keeps
//! working offline. The network call sits behind `Fetch` so tests never use
//! the network.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

use notecal_core::Rates;
use notecal_core::currency::mentions_currency;
use serde::{Deserialize, Serialize};

use crate::notes::write_atomic;

/// Emitted with a `RatesStatus` after new rates arrive.
pub const RATES_UPDATED: &str = "rates-updated";

const SOURCE_URL: &str = "https://open.er-api.com/v6/latest/USD";
/// PRD: refresh at most once every 24 h.
const MAX_AGE_SECS: u64 = 24 * 60 * 60;
/// After a failed fetch, wait this long before trying again.
const RETRY_AFTER_SECS: u64 = 60 * 60;
const TIMEOUT: Duration = Duration::from_secs(10);

/// Rates as fetched: units per US dollar, and the provider's update date.
#[derive(Debug, Clone, PartialEq)]
pub struct Fetched {
    pub per_usd: HashMap<String, f64>,
    /// Provider date, e.g. "Thu, 05 Nov 2026 00:02:31 +0000".
    pub source_date: String,
}

pub trait Fetch: Send + Sync {
    fn fetch(&self) -> Result<Fetched, String>;
}

/// The real source.
pub struct ErApi;

#[derive(Deserialize)]
struct ErApiResponse {
    result: String,
    time_last_update_utc: String,
    rates: HashMap<String, f64>,
}

impl Fetch for ErApi {
    fn fetch(&self) -> Result<Fetched, String> {
        let agent: ureq::Agent = ureq::Agent::config_builder()
            .timeout_global(Some(TIMEOUT))
            .build()
            .into();
        let response: ErApiResponse = agent
            .get(SOURCE_URL)
            .call()
            .map_err(|e| format!("Could not fetch exchange rates: {e}"))?
            .body_mut()
            .read_json()
            .map_err(|e| format!("Unexpected exchange-rate response: {e}"))?;
        if response.result != "success" || response.rates.is_empty() {
            return Err(format!(
                "The exchange-rate service answered '{}'.",
                response.result
            ));
        }
        Ok(Fetched {
            per_usd: response.rates,
            source_date: response.time_last_update_utc,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct Cache {
    /// When notecal fetched them, in seconds since the Unix epoch.
    fetched_at: u64,
    source_date: String,
    rates: HashMap<String, f64>,
}

/// What the status line shows.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RatesStatus {
    /// Short provider date ("5 Nov 2026"), or `None` before the first fetch.
    pub date: Option<String>,
}

pub struct RatesStore {
    path: PathBuf,
    cache: Option<Cache>,
    rates: Option<Rates>,
    last_attempt: Option<u64>,
    in_flight: bool,
}

impl RatesStore {
    /// Loads the cache; a missing or unreadable one means "no rates yet".
    pub fn load(path: PathBuf) -> Self {
        let cache: Option<Cache> = fs::read_to_string(&path)
            .ok()
            .and_then(|text| serde_json::from_str(&text).ok());
        let rates = cache.as_ref().map(|c| Rates::from_per_usd(c.rates.clone()));
        Self {
            path,
            cache,
            rates,
            last_attempt: None,
            in_flight: false,
        }
    }

    pub fn rates(&self) -> Option<Rates> {
        self.rates.clone()
    }

    pub fn status(&self) -> RatesStatus {
        RatesStatus {
            date: self.cache.as_ref().map(|c| short_date(&c.source_date)),
        }
    }

    /// Whether to fetch now for a note with `text`. Starting a fetch marks
    /// it in flight, so only one runs at a time; call `finish` afterwards.
    pub fn start_refresh(&mut self, text: &str, enabled: bool, now: u64) -> bool {
        let stale = self
            .cache
            .as_ref()
            // A timestamp from the future (clock changed) counts as stale.
            .is_none_or(|c| c.fetched_at > now || now - c.fetched_at >= MAX_AGE_SECS);
        let may_retry = self
            .last_attempt
            .is_none_or(|t| t > now || now - t >= RETRY_AFTER_SECS);
        let go = enabled && !self.in_flight && stale && may_retry && mentions_currency(text);
        if go {
            self.in_flight = true;
            self.last_attempt = Some(now);
        }
        go
    }

    /// Records a fetch result. On success the cache is saved and the new
    /// rates are returned; on failure the old cache stays in use.
    pub fn finish(&mut self, result: Result<Fetched, String>, now: u64) -> Result<Rates, String> {
        self.in_flight = false;
        let fetched = result?;
        let rates = Rates::from_per_usd(fetched.per_usd.clone());
        let cache = Cache {
            fetched_at: now,
            source_date: fetched.source_date,
            rates: fetched.per_usd,
        };
        save(&self.path, &cache)?;
        self.cache = Some(cache);
        self.rates = Some(rates.clone());
        Ok(rates)
    }
}

fn save(path: &Path, cache: &Cache) -> Result<(), String> {
    let dir = path
        .parent()
        .ok_or_else(|| format!("{} has no parent folder", path.display()))?;
    fs::create_dir_all(dir).map_err(|e| format!("Could not create {}: {e}", dir.display()))?;
    let text = serde_json::to_string(cache).map_err(|e| e.to_string())?;
    write_atomic(dir, path, &text).map_err(|e| format!("Could not save exchange rates: {e}"))
}

/// "Thu, 05 Nov 2026 00:02:31 +0000" → "5 Nov 2026"; anything else as is.
fn short_date(source: &str) -> String {
    let words: Vec<&str> = source.split_whitespace().collect();
    match words.as_slice() {
        [_, day, month, year, ..] if day.parse::<u8>().is_ok() => {
            format!("{} {month} {year}", day.trim_start_matches('0'))
        }
        _ => source.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    const DAY: u64 = 24 * 60 * 60;

    fn fetched() -> Fetched {
        Fetched {
            per_usd: HashMap::from([("USD".into(), 1.0), ("IDR".into(), 16000.0)]),
            source_date: "Thu, 05 Nov 2026 00:02:31 +0000".into(),
        }
    }

    fn store() -> (TempDir, RatesStore) {
        let dir = TempDir::new().unwrap();
        let store = RatesStore::load(dir.path().join("cache/rates.json"));
        (dir, store)
    }

    #[test]
    fn fetches_once_when_a_currency_is_used_and_there_is_no_cache() {
        let (_dir, mut store) = store();
        assert!(!store.start_refresh("5 km in miles", true, 1000));
        assert!(store.start_refresh("100 USD in IDR", true, 1000));
        // Only one fetch at a time.
        assert!(!store.start_refresh("100 USD in IDR", true, 1001));
        store.finish(Ok(fetched()), 1002).unwrap();
        assert_eq!(store.rates().unwrap().get("IDR"), Some(16000.0));
        assert_eq!(store.status().date.as_deref(), Some("5 Nov 2026"));
    }

    #[test]
    fn fresh_cache_is_not_refetched_until_a_day_has_passed() {
        let (_dir, mut store) = store();
        assert!(store.start_refresh("1 EUR", true, 0));
        store.finish(Ok(fetched()), 0).unwrap();
        assert!(!store.start_refresh("1 EUR", true, DAY - 1));
        assert!(store.start_refresh("1 EUR", true, DAY));
    }

    #[test]
    fn cache_from_the_future_is_stale() {
        let (_dir, mut store) = store();
        assert!(store.start_refresh("1 EUR", true, 10 * DAY));
        store.finish(Ok(fetched()), 10 * DAY).unwrap();
        assert!(store.start_refresh("1 EUR", true, DAY));
    }

    #[test]
    fn disabled_refresh_never_fetches() {
        let (_dir, mut store) = store();
        assert!(!store.start_refresh("1 EUR", false, 0));
    }

    #[test]
    fn failure_keeps_the_old_rates_and_backs_off() {
        let (_dir, mut store) = store();
        assert!(store.start_refresh("1 EUR", true, 0));
        store.finish(Ok(fetched()), 0).unwrap();
        assert!(store.start_refresh("1 EUR", true, DAY));
        assert!(store.finish(Err("offline".into()), DAY).is_err());
        assert_eq!(store.rates().unwrap().get("IDR"), Some(16000.0));
        assert!(!store.start_refresh("1 EUR", true, DAY + 60));
        assert!(store.start_refresh("1 EUR", true, DAY + RETRY_AFTER_SECS));
    }

    #[test]
    fn cache_survives_a_restart_and_works_offline() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("rates.json");
        let mut first = RatesStore::load(path.clone());
        assert!(first.start_refresh("1 EUR", true, 0));
        first.finish(Ok(fetched()), 0).unwrap();
        let second = RatesStore::load(path);
        assert_eq!(second.rates().unwrap().get("IDR"), Some(16000.0));
    }

    #[test]
    fn corrupt_cache_is_ignored() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("rates.json");
        fs::write(&path, "{ not json").unwrap();
        let mut store = RatesStore::load(path);
        assert!(store.rates().is_none());
        assert_eq!(store.status().date, None);
        assert!(store.start_refresh("1 EUR", true, 0));
    }

    #[test]
    fn short_date_formats_the_provider_date() {
        assert_eq!(short_date("Thu, 05 Nov 2026 00:02:31 +0000"), "5 Nov 2026");
        assert_eq!(short_date("odd"), "odd");
    }
}
