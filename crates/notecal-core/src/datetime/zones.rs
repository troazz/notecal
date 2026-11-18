//! Time zone names people type: IANA names, cities, countries, abbreviations
//! and fixed offsets (`UTC+7`).

use jiff::tz::{Offset, TimeZone};

/// Cities, countries and abbreviations, lower case, mapped to IANA names.
///
/// Abbreviations that mean more than one zone resolve to the most common
/// meaning: CST is US Central, IST is India, BST is British Summer Time. A
/// summer abbreviation maps to the same zone as its winter one, so `PST` in
/// July shows Pacific Daylight Time.
const ALIASES: &[(&str, &str)] = &[
    // Abbreviations
    ("utc", "UTC"),
    ("gmt", "Etc/GMT"),
    ("wib", "Asia/Jakarta"),
    ("wita", "Asia/Makassar"),
    ("wit", "Asia/Jayapura"),
    ("sgt", "Asia/Singapore"),
    ("myt", "Asia/Kuala_Lumpur"),
    ("ict", "Asia/Bangkok"),
    ("pht", "Asia/Manila"),
    ("hkt", "Asia/Hong_Kong"),
    ("jst", "Asia/Tokyo"),
    ("kst", "Asia/Seoul"),
    ("ist", "Asia/Kolkata"),
    ("pkt", "Asia/Karachi"),
    ("gst", "Asia/Dubai"),
    ("msk", "Europe/Moscow"),
    ("cet", "Europe/Paris"),
    ("cest", "Europe/Paris"),
    ("eet", "Europe/Athens"),
    ("eest", "Europe/Athens"),
    ("wet", "Europe/Lisbon"),
    ("bst", "Europe/London"),
    ("aest", "Australia/Sydney"),
    ("aedt", "Australia/Sydney"),
    ("acst", "Australia/Adelaide"),
    ("awst", "Australia/Perth"),
    ("nzst", "Pacific/Auckland"),
    ("nzdt", "Pacific/Auckland"),
    ("est", "America/New_York"),
    ("edt", "America/New_York"),
    ("et", "America/New_York"),
    ("cst", "America/Chicago"),
    ("cdt", "America/Chicago"),
    ("ct", "America/Chicago"),
    ("mst", "America/Denver"),
    ("mdt", "America/Denver"),
    ("mt", "America/Denver"),
    ("pst", "America/Los_Angeles"),
    ("pdt", "America/Los_Angeles"),
    ("pt", "America/Los_Angeles"),
    ("akst", "America/Anchorage"),
    ("hst", "Pacific/Honolulu"),
    ("brt", "America/Sao_Paulo"),
    ("art", "America/Argentina/Buenos_Aires"),
    // Asia
    ("jakarta", "Asia/Jakarta"),
    ("bandung", "Asia/Jakarta"),
    ("surabaya", "Asia/Jakarta"),
    ("yogyakarta", "Asia/Jakarta"),
    ("medan", "Asia/Jakarta"),
    ("indonesia", "Asia/Jakarta"),
    ("bali", "Asia/Makassar"),
    ("denpasar", "Asia/Makassar"),
    ("makassar", "Asia/Makassar"),
    ("jayapura", "Asia/Jayapura"),
    ("singapore", "Asia/Singapore"),
    ("kuala lumpur", "Asia/Kuala_Lumpur"),
    ("malaysia", "Asia/Kuala_Lumpur"),
    ("bangkok", "Asia/Bangkok"),
    ("thailand", "Asia/Bangkok"),
    ("hanoi", "Asia/Ho_Chi_Minh"),
    ("ho chi minh city", "Asia/Ho_Chi_Minh"),
    ("saigon", "Asia/Ho_Chi_Minh"),
    ("vietnam", "Asia/Ho_Chi_Minh"),
    ("manila", "Asia/Manila"),
    ("philippines", "Asia/Manila"),
    ("hong kong", "Asia/Hong_Kong"),
    ("taipei", "Asia/Taipei"),
    ("taiwan", "Asia/Taipei"),
    ("beijing", "Asia/Shanghai"),
    ("shanghai", "Asia/Shanghai"),
    ("shenzhen", "Asia/Shanghai"),
    ("china", "Asia/Shanghai"),
    ("tokyo", "Asia/Tokyo"),
    ("osaka", "Asia/Tokyo"),
    ("japan", "Asia/Tokyo"),
    ("seoul", "Asia/Seoul"),
    ("korea", "Asia/Seoul"),
    ("south korea", "Asia/Seoul"),
    ("delhi", "Asia/Kolkata"),
    ("new delhi", "Asia/Kolkata"),
    ("mumbai", "Asia/Kolkata"),
    ("bangalore", "Asia/Kolkata"),
    ("bengaluru", "Asia/Kolkata"),
    ("kolkata", "Asia/Kolkata"),
    ("india", "Asia/Kolkata"),
    ("karachi", "Asia/Karachi"),
    ("pakistan", "Asia/Karachi"),
    ("dhaka", "Asia/Dhaka"),
    ("bangladesh", "Asia/Dhaka"),
    ("kathmandu", "Asia/Kathmandu"),
    ("nepal", "Asia/Kathmandu"),
    ("colombo", "Asia/Colombo"),
    ("dubai", "Asia/Dubai"),
    ("abu dhabi", "Asia/Dubai"),
    ("uae", "Asia/Dubai"),
    ("doha", "Asia/Qatar"),
    ("qatar", "Asia/Qatar"),
    ("riyadh", "Asia/Riyadh"),
    ("saudi arabia", "Asia/Riyadh"),
    ("mecca", "Asia/Riyadh"),
    ("tehran", "Asia/Tehran"),
    ("iran", "Asia/Tehran"),
    ("jerusalem", "Asia/Jerusalem"),
    ("tel aviv", "Asia/Jerusalem"),
    ("israel", "Asia/Jerusalem"),
    ("istanbul", "Europe/Istanbul"),
    ("turkey", "Europe/Istanbul"),
    // Europe
    ("london", "Europe/London"),
    ("uk", "Europe/London"),
    ("united kingdom", "Europe/London"),
    ("england", "Europe/London"),
    ("dublin", "Europe/Dublin"),
    ("ireland", "Europe/Dublin"),
    ("lisbon", "Europe/Lisbon"),
    ("portugal", "Europe/Lisbon"),
    ("paris", "Europe/Paris"),
    ("france", "Europe/Paris"),
    ("berlin", "Europe/Berlin"),
    ("munich", "Europe/Berlin"),
    ("germany", "Europe/Berlin"),
    ("amsterdam", "Europe/Amsterdam"),
    ("netherlands", "Europe/Amsterdam"),
    ("brussels", "Europe/Brussels"),
    ("madrid", "Europe/Madrid"),
    ("barcelona", "Europe/Madrid"),
    ("spain", "Europe/Madrid"),
    ("rome", "Europe/Rome"),
    ("milan", "Europe/Rome"),
    ("italy", "Europe/Rome"),
    ("zurich", "Europe/Zurich"),
    ("switzerland", "Europe/Zurich"),
    ("vienna", "Europe/Vienna"),
    ("prague", "Europe/Prague"),
    ("warsaw", "Europe/Warsaw"),
    ("poland", "Europe/Warsaw"),
    ("stockholm", "Europe/Stockholm"),
    ("sweden", "Europe/Stockholm"),
    ("oslo", "Europe/Oslo"),
    ("copenhagen", "Europe/Copenhagen"),
    ("helsinki", "Europe/Helsinki"),
    ("athens", "Europe/Athens"),
    ("greece", "Europe/Athens"),
    ("kyiv", "Europe/Kyiv"),
    ("ukraine", "Europe/Kyiv"),
    ("moscow", "Europe/Moscow"),
    ("russia", "Europe/Moscow"),
    // Africa
    ("cairo", "Africa/Cairo"),
    ("egypt", "Africa/Cairo"),
    ("lagos", "Africa/Lagos"),
    ("nigeria", "Africa/Lagos"),
    ("nairobi", "Africa/Nairobi"),
    ("kenya", "Africa/Nairobi"),
    ("johannesburg", "Africa/Johannesburg"),
    ("cape town", "Africa/Johannesburg"),
    ("south africa", "Africa/Johannesburg"),
    ("casablanca", "Africa/Casablanca"),
    ("morocco", "Africa/Casablanca"),
    // Americas
    ("new york", "America/New_York"),
    ("nyc", "America/New_York"),
    ("boston", "America/New_York"),
    ("washington", "America/New_York"),
    ("miami", "America/New_York"),
    ("atlanta", "America/New_York"),
    ("toronto", "America/Toronto"),
    ("montreal", "America/Toronto"),
    ("chicago", "America/Chicago"),
    ("dallas", "America/Chicago"),
    ("houston", "America/Chicago"),
    ("austin", "America/Chicago"),
    ("mexico city", "America/Mexico_City"),
    ("mexico", "America/Mexico_City"),
    ("denver", "America/Denver"),
    ("phoenix", "America/Phoenix"),
    ("los angeles", "America/Los_Angeles"),
    ("la", "America/Los_Angeles"),
    ("san francisco", "America/Los_Angeles"),
    ("sf", "America/Los_Angeles"),
    ("seattle", "America/Los_Angeles"),
    ("vancouver", "America/Vancouver"),
    ("anchorage", "America/Anchorage"),
    ("honolulu", "Pacific/Honolulu"),
    ("hawaii", "Pacific/Honolulu"),
    ("bogota", "America/Bogota"),
    ("colombia", "America/Bogota"),
    ("lima", "America/Lima"),
    ("peru", "America/Lima"),
    ("santiago", "America/Santiago"),
    ("chile", "America/Santiago"),
    ("buenos aires", "America/Argentina/Buenos_Aires"),
    ("argentina", "America/Argentina/Buenos_Aires"),
    ("sao paulo", "America/Sao_Paulo"),
    ("rio de janeiro", "America/Sao_Paulo"),
    ("brazil", "America/Sao_Paulo"),
    // Oceania
    ("sydney", "Australia/Sydney"),
    ("melbourne", "Australia/Melbourne"),
    ("brisbane", "Australia/Brisbane"),
    ("adelaide", "Australia/Adelaide"),
    ("perth", "Australia/Perth"),
    ("auckland", "Pacific/Auckland"),
    ("wellington", "Pacific/Auckland"),
    ("new zealand", "Pacific/Auckland"),
];

/// Resolves what a user typed as a time zone, or `None` if it isn't one.
pub(super) fn resolve(name: &str) -> Option<TimeZone> {
    let name = name.trim();
    let lower = name.to_ascii_lowercase();
    // Collapse inner whitespace so "new   york" works too.
    let lower = lower.split_whitespace().collect::<Vec<_>>().join(" ");
    if lower.is_empty() {
        return None;
    }
    if let Some(offset) = fixed_offset(&lower) {
        return Some(TimeZone::fixed(offset));
    }
    if let Some((_, iana)) = ALIASES.iter().find(|(alias, _)| *alias == lower) {
        return TimeZone::get(iana).ok();
    }
    // IANA names such as `Asia/Jakarta` or `america/new_york`; the time zone
    // database lookup is case-insensitive. A bare word like `in` or `kg` is
    // never an IANA name, so this is only tried for names with a slash.
    if name.contains('/') {
        return TimeZone::get(name).ok();
    }
    None
}

/// `utc+7`, `gmt-3`, `utc+5:30`.
fn fixed_offset(lower: &str) -> Option<Offset> {
    let rest = lower
        .strip_prefix("utc")
        .or_else(|| lower.strip_prefix("gmt"))?
        .trim();
    let (sign, rest) = match rest.chars().next()? {
        '+' => (1, &rest[1..]),
        '-' => (-1, &rest[1..]),
        _ => return None,
    };
    let rest = rest.trim();
    let (hours, minutes) = rest.split_once(':').unwrap_or((rest, "0"));
    let hours: i32 = hours.parse().ok()?;
    let minutes: i32 = minutes.parse().ok()?;
    if !(0..=14).contains(&hours) || !(0..60).contains(&minutes) {
        return None;
    }
    Offset::from_seconds(sign * (hours * 3600 + minutes * 60)).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_alias_names_a_known_zone() {
        for (alias, iana) in ALIASES {
            assert!(
                TimeZone::get(iana).is_ok(),
                "{alias} -> {iana} is not in the database"
            );
            assert!(resolve(alias).is_some(), "{alias} does not resolve");
        }
    }

    #[test]
    fn resolves_cities_abbreviations_iana_and_offsets() {
        assert_eq!(resolve("New  York"), TimeZone::get("America/New_York").ok());
        assert_eq!(resolve("WIB"), TimeZone::get("Asia/Jakarta").ok());
        assert_eq!(resolve("asia/tokyo"), TimeZone::get("Asia/Tokyo").ok());
        assert_eq!(
            resolve("UTC+5:30"),
            Some(TimeZone::fixed(Offset::from_seconds(19_800).unwrap()))
        );
        assert_eq!(
            resolve("gmt-3"),
            Some(TimeZone::fixed(Offset::from_seconds(-10_800).unwrap()))
        );
    }

    #[test]
    fn rejects_non_zones() {
        for name in ["", "kg", "miles", "Atlantis", "utc+15", "utc+x"] {
            assert!(resolve(name).is_none(), "{name:?} should not resolve");
        }
    }
}
