pub const MC_VERSION: &str = "26.1";
pub const MC_DATA_VERSION: i32 = 4786;
pub const MC_DATA_VERSION_SERIES: &str = "main";
pub const MC_BUILD_TIME: &str = "2026-03-24T12:09:43+00:00";
pub const MC_RESOURCE_PACK_FORMAT: PackFormatVersion = PackFormatVersion::new(84, 0);
pub const MC_DATA_PACK_FORMAT: PackFormatVersion = PackFormatVersion::new(101, 1);
pub const MC_STABLE: bool = true;

/// [`MC_BUILD_TIME`] as `java.util.Date.toString()` renders it, which is the
/// form `/version` puts on the wire.
///
/// `WorldVersion.buildTime()` is a `Date` (WorldVersion.java:19), parsed out of
/// `version.json`'s `build_time` (DetectedVersion.java:41), and
/// `Component.translationArg(Date)` is `literal(date.toString())`
/// (Component.java:204-206) -- so what `commands.version.build_time` carries is
/// the `Date.toString()` text and not an ISO instant.
///
/// **Divergence.** `Date.toString()` renders in `TimeZone.getDefault()`, so an
/// official server started under a zone other than UTC prints that zone's
/// wall-clock reading and abbreviation instead. This workspace keeps no zone
/// database and no clock, so the rendering is UTC.
pub fn mc_build_time_java_date_string() -> String {
    java_date_string(MC_BUILD_TIME).expect("MC_BUILD_TIME is an ISO-8601 instant with an offset")
}

/// `java.util.Date.toString()`'s `dow mon dd hh:mm:ss zzz yyyy`, over an
/// ISO-8601 instant carrying a numeric offset, rendered in UTC.
fn java_date_string(instant: &str) -> Option<String> {
    let bytes = instant.as_bytes();
    if bytes.len() < 19 {
        return None;
    }
    let field = |from: usize, to: usize| instant.get(from..to)?.parse::<i64>().ok();
    let (year, month, day) = (field(0, 4)?, field(5, 7)?, field(8, 10)?);
    let (hour, minute, second) = (field(11, 13)?, field(14, 16)?, field(17, 19)?);
    let offset_minutes = match instant.get(19..) {
        Some("") | Some("Z") => 0,
        Some(offset) if offset.len() == 6 => {
            let sign = match &offset[..1] {
                "+" => 1,
                "-" => -1,
                _ => return None,
            };
            sign * (offset[1..3].parse::<i64>().ok()? * 60 + offset[4..6].parse::<i64>().ok()?)
        }
        _ => return None,
    };
    // Howard Hinnant's `days_from_civil`, which is the same proleptic Gregorian
    // count `Instant` counts in.
    let shifted_year = year - i64::from(month <= 2);
    let era = shifted_year.div_euclid(400);
    let year_of_era = shifted_year - era * 400;
    let day_of_year = (153 * (month + if month > 2 { -3 } else { 9 }) + 2) / 5 + day - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    let days = era * 146_097 + day_of_era - 719_468;
    let utc_minutes = days * 1440 + hour * 60 + minute - offset_minutes;
    let (utc_days, minute_of_day) = (utc_minutes.div_euclid(1440), utc_minutes.rem_euclid(1440));
    // 1970-01-01 was a Thursday, which is index 4 of the array below.
    let weekday =
        ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"][((utc_days + 4).rem_euclid(7)) as usize];
    // Recovering the civil date from the shifted day count is the inverse of
    // the computation above, so it is done by walking back through it.
    let (year, month, day) = civil_from_days(utc_days);
    let month_name = [
        "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
    ][(month - 1) as usize];
    Some(format!(
        "{weekday} {month_name} {day:02} {:02}:{:02}:{second:02} UTC {year}",
        minute_of_day / 60,
        minute_of_day % 60,
    ))
}

/// Howard Hinnant's `civil_from_days`, the inverse of the count above.
fn civil_from_days(days: i64) -> (i64, i64, i64) {
    let shifted = days + 719_468;
    let era = shifted.div_euclid(146_097);
    let day_of_era = shifted - era * 146_097;
    let year_of_era =
        (day_of_era - day_of_era / 1460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let year = year_of_era + era * 400;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let shifted_month = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * shifted_month + 2) / 5 + 1;
    let month = shifted_month + if shifted_month < 10 { 3 } else { -9 };
    (year + i64::from(month <= 2), month, day)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PackFormatVersion {
    pub major: u32,
    pub minor: u32,
}

impl PackFormatVersion {
    pub const fn new(major: u32, minor: u32) -> Self {
        Self { major, minor }
    }

    pub fn to_vanilla_string(self) -> String {
        format!("{}.{}", self.major, self.minor)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_constants_match_local_vanilla_version_json() {
        let Some(root) = std::env::var_os("BBB_VANILLA_ROOT") else {
            eprintln!("skipping local vanilla version.json check; BBB_VANILLA_ROOT is unset");
            return;
        };
        let version_path = std::path::PathBuf::from(root)
            .join("sources")
            .join(MC_VERSION)
            .join("version.json");
        if !version_path.exists() {
            eprintln!(
                "skipping local vanilla version.json check; missing {}",
                version_path.display()
            );
            return;
        }

        let raw = std::fs::read_to_string(&version_path).unwrap();
        let value: serde_json::Value = serde_json::from_str(&raw).unwrap();

        assert_eq!(value["id"].as_str(), Some(MC_VERSION));
        assert_eq!(value["name"].as_str(), Some(MC_VERSION));
        assert_eq!(
            value["world_version"].as_i64(),
            Some(MC_DATA_VERSION.into())
        );
        assert_eq!(value["series_id"].as_str(), Some(MC_DATA_VERSION_SERIES));
        assert_eq!(
            value["pack_version"]["resource_major"].as_u64(),
            Some(MC_RESOURCE_PACK_FORMAT.major.into())
        );
        assert_eq!(
            value["pack_version"]["resource_minor"].as_u64(),
            Some(MC_RESOURCE_PACK_FORMAT.minor.into())
        );
        assert_eq!(
            value["pack_version"]["data_major"].as_u64(),
            Some(MC_DATA_PACK_FORMAT.major.into())
        );
        assert_eq!(
            value["pack_version"]["data_minor"].as_u64(),
            Some(MC_DATA_PACK_FORMAT.minor.into())
        );
        assert_eq!(value["build_time"].as_str(), Some(MC_BUILD_TIME));
        assert_eq!(value["stable"].as_bool(), Some(MC_STABLE));
    }

    #[test]
    fn the_build_time_renders_the_way_java_util_date_does() {
        // `Date.toString()`'s documented `dow mon dd hh:mm:ss zzz yyyy`, which
        // `Component.translationArg(Date)` (Component.java:204-206) is what
        // `/version` puts on the wire.
        assert_eq!(
            mc_build_time_java_date_string(),
            "Tue Mar 24 12:09:43 UTC 2026"
        );
        // The calendar arithmetic, over the dates that break a naive one: the
        // epoch itself, a leap day, the day before a century that is not a leap
        // year, the day after one that is, and an offset that carries the
        // instant across midnight in both directions.
        for (instant, rendered) in [
            ("1970-01-01T00:00:00Z", "Thu Jan 01 00:00:00 UTC 1970"),
            ("2024-02-29T23:59:59Z", "Thu Feb 29 23:59:59 UTC 2024"),
            ("1900-02-28T12:00:00Z", "Wed Feb 28 12:00:00 UTC 1900"),
            ("2000-03-01T00:00:00Z", "Wed Mar 01 00:00:00 UTC 2000"),
            ("2026-03-24T01:09:43+02:00", "Mon Mar 23 23:09:43 UTC 2026"),
            ("2026-03-24T23:09:43-02:00", "Wed Mar 25 01:09:43 UTC 2026"),
        ] {
            assert_eq!(java_date_string(instant).unwrap(), rendered, "{instant}");
        }
    }

    #[test]
    fn pack_format_version_string_matches_vanilla_to_string() {
        assert_eq!(MC_RESOURCE_PACK_FORMAT.to_vanilla_string(), "84.0");
        assert_eq!(MC_DATA_PACK_FORMAT.to_vanilla_string(), "101.1");
    }
}
