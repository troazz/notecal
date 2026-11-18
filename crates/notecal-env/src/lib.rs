//! The files notecal reads and writes, shared by the app and `notecal-cli`:
//! settings, the definitions file, the exchange-rate cache, and where they
//! live.

#![forbid(unsafe_code)]

pub mod fs;
pub mod paths;
pub mod rates;
pub mod settings;
