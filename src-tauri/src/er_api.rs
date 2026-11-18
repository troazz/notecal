//! The real exchange-rate source, open.er-api.com (PRD C9).

use std::collections::HashMap;
use std::time::Duration;

use notecal_env::rates::{Fetch, Fetched};
use serde::Deserialize;

const SOURCE_URL: &str = "https://open.er-api.com/v6/latest/USD";
const TIMEOUT: Duration = Duration::from_secs(10);

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
