//! Vanilla-parity `minecraft:local_time` select-property formatting.
//!
//! Vanilla 26.1 (`LocalTime.java`) formats through ICU4J:
//! `new ULocale(localeId)` -> `Calendar.getInstance(timeZone, locale)` ->
//! `new SimpleDateFormat(pattern, locale).setCalendar(calendar)`. This module
//! reproduces that chain with ICU4X as the CLDR owner:
//!
//! - Text fields (era, month, weekday, day-period and time-zone names) are
//!   delegated to `icu_datetime`'s pattern formatter, per-locale calendars are
//!   selected via `AnyCalendarKind` on the likely-subtags-maximized locale
//!   (`th_th` -> Buddhist, `fa_ir` -> Persian), and numbering systems come
//!   from `icu_decimal` (`ar_sa` -> Arabic-Indic digits).
//! - Numeric fields are computed natively so ICU4J's width quirks
//!   (`MMMMMM` -> `000008`, `cc` unpadded, week fields) stay byte-exact, then
//!   transliterated into the locale's decimal digits.
//! - Week fields port ICU4J `Calendar.computeWeekFields`/`weekNumber` over the
//!   dispatched calendar's day-of-year, with CLDR territory week data dumped
//!   from the icu4j 77.1 jar vanilla ships (see [`tables_26_1`]).
//! - The five ICU4J behaviors ICU4X lacks are carried by generated tables or
//!   local rules: quarter names, flexible day periods (`B`), the localized-GMT
//!   zero format (`GMT`), the `VVV` unknown-city string, and the
//!   metazone-standard fallback for generic zone names (`vvvv` of a no-DST zone
//!   -> `中国标准时间`).
//!
//! `goldens_26_1.tsv` pins the surface against outputs of the exact icu4j
//! 77.1 jar (`scripts/local-time-icu4j/`), and every row matches.

#[cfg(not(target_arch = "wasm32"))]
mod tables_26_1;

use std::borrow::Borrow;
use std::cell::RefCell;
use std::hash::{Hash, Hasher};

use rustc_hash::FxHashMap;

use chrono::FixedOffset;
#[cfg(not(target_arch = "wasm32"))]
use chrono::{Datelike, Local, Offset as _, TimeZone as _, Timelike, Utc};
#[cfg(not(target_arch = "wasm32"))]
use chrono_tz::{OffsetComponents, Tz};
#[cfg(not(target_arch = "wasm32"))]
use icu_calendar::cal;
#[cfg(not(target_arch = "wasm32"))]
use icu_calendar::{AnyCalendarKind, Date};
#[cfg(not(target_arch = "wasm32"))]
use icu_datetime::fieldsets::enums::CompositeFieldSet;
#[cfg(not(target_arch = "wasm32"))]
use icu_datetime::pattern::{DateTimePattern, FixedCalendarDateTimeNames};
#[cfg(not(target_arch = "wasm32"))]
use icu_decimal::options::{DecimalFormatterOptions, GroupingStrategy};
#[cfg(not(target_arch = "wasm32"))]
use icu_decimal::DecimalFormatter;
#[cfg(not(target_arch = "wasm32"))]
use icu_locale::extensions::unicode as unicode_ext;
#[cfg(not(target_arch = "wasm32"))]
use icu_locale::{Locale, LocaleCanonicalizer, LocaleExpander};
#[cfg(not(target_arch = "wasm32"))]
use icu_time::zone::iana::IanaParser;
#[cfg(not(target_arch = "wasm32"))]
use icu_time::zone::models::AtTime;
#[cfg(not(target_arch = "wasm32"))]
use icu_time::zone::{UtcOffset, ZoneNameTimestamp};
#[cfg(not(target_arch = "wasm32"))]
use icu_time::{Time, TimeZone as IcuTimeZone, TimeZoneInfo, ZonedDateTime};
#[cfg(not(target_arch = "wasm32"))]
use writeable::TryWriteable;

#[derive(PartialEq, Eq, Hash)]
struct LocalTimeCacheKey {
    pattern: String,
    locale: String,
    time_zone: Option<String>,
}

trait LocalTimeKey {
    fn key_parts(&self) -> (&str, &str, Option<&str>);
}

impl PartialEq for dyn LocalTimeKey + '_ {
    fn eq(&self, other: &Self) -> bool {
        self.key_parts() == other.key_parts()
    }
}

impl Eq for dyn LocalTimeKey + '_ {}

impl Hash for dyn LocalTimeKey + '_ {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.key_parts().hash(state);
    }
}

impl LocalTimeKey for LocalTimeCacheKey {
    fn key_parts(&self) -> (&str, &str, Option<&str>) {
        (&self.pattern, &self.locale, self.time_zone.as_deref())
    }
}

impl LocalTimeKey for (&str, &str, Option<&str>) {
    fn key_parts(&self) -> (&str, &str, Option<&str>) {
        *self
    }
}

impl<'a> Borrow<dyn LocalTimeKey + 'a> for LocalTimeCacheKey {
    fn borrow(&self) -> &(dyn LocalTimeKey + 'a) {
        self
    }
}

/// Formats one `minecraft:local_time` value the way vanilla's cached
/// `SimpleDateFormat` would; `None` mirrors vanilla's codec/format rejection
/// (unknown zone, invalid pattern letter), which drops the select case.
pub fn local_time_select_value(
    epoch_millis: i64,
    pattern: &str,
    locale: &str,
    time_zone: Option<&str>,
) -> Option<String> {
    type LocalTimeCache = FxHashMap<LocalTimeCacheKey, (i64, String)>;
    thread_local! {
        static CACHE: RefCell<LocalTimeCache> = RefCell::new(FxHashMap::default());
    }
    // Vanilla re-formats at most once per second and serves the cached string
    // in between; keying on the wall-clock second reproduces that cadence.
    let second = epoch_millis.div_euclid(1000);
    let lookup_tuple = (pattern, locale, time_zone);
    let lookup: &(dyn LocalTimeKey + '_) = &lookup_tuple;
    if let Some(result) = CACHE.with(|cache| {
        cache
            .borrow()
            .get(lookup)
            .filter(|(cached_second, _)| *cached_second == second)
            .map(|(_, result)| result.clone())
    }) {
        return Some(result);
    }
    let result = compute_local_time(epoch_millis, pattern, locale, time_zone)?;
    let key = LocalTimeCacheKey {
        pattern: pattern.to_string(),
        locale: locale.to_string(),
        time_zone: time_zone.map(str::to_string),
    };
    CACHE.with(|cache| {
        let mut cache = cache.borrow_mut();
        if cache.len() > 256 {
            cache.clear();
        }
        cache.insert(key, (second, result.clone()));
    });
    Some(result)
}

#[cfg(target_arch = "wasm32")]
use wasm_intl::compute_local_time;

#[cfg(not(target_arch = "wasm32"))]
fn compute_local_time(
    epoch_millis: i64,
    pattern: &str,
    locale: &str,
    time_zone: Option<&str>,
) -> Option<String> {
    let mut locale = parse_ulocale(locale);
    // ICU4J resolves languages without a CLDR bundle (vanilla ships `be_latn`)
    // through the JVM default locale's data; bbb pins that default to English.
    if !locale.id.language.is_unknown() && language_data_is_root(&locale) {
        locale.id.language = icu_locale::subtags::Language::try_from_str("en").ok()?;
    }
    let zone = resolve_zone(epoch_millis, time_zone)?;
    let kind = calendar_kind_for(&locale);
    dispatch_calendar(kind, &locale, &zone, pattern)
}

/// Whether the locale's month names resolve to root's coded forms (`M08`),
/// meaning ICU4X carries no CLDR data for the language/script pair.
#[cfg(not(target_arch = "wasm32"))]
fn language_data_is_root(locale: &Locale) -> bool {
    type ProbeKey = (
        icu_locale::subtags::Language,
        Option<icu_locale::subtags::Script>,
    );
    thread_local! {
        static PROBES: RefCell<FxHashMap<ProbeKey, bool>> = RefCell::new(FxHashMap::default());
    }
    let key = (locale.id.language, locale.id.script);
    if let Some(known) = PROBES.with(|probes| probes.borrow().get(&key).copied()) {
        return known;
    }
    let probe = |target: &Locale| -> Option<String> {
        use icu_datetime::fieldsets::enums::DateFieldSet;
        let pattern: DateTimePattern = "MMMM".parse().ok()?;
        let mut names =
            FixedCalendarDateTimeNames::<cal::Gregorian, DateFieldSet>::try_new(target.into())
                .ok()?;
        let formatter = names.include_for_pattern(&pattern).ok()?;
        let date = Date::try_new_iso(2026, 8, 5)
            .ok()?
            .to_calendar(cal::Gregorian);
        formatter
            .format(&date)
            .try_write_to_string()
            .ok()
            .map(|value| value.into_owned())
    };
    let is_root = match (probe(locale), probe(&Locale::UNKNOWN)) {
        (Some(value), Some(root)) => value == root,
        // A locale ICU4X cannot even build a formatter for has no data.
        (None, _) => true,
        _ => false,
    };
    PROBES.with(|probes| {
        let mut probes = probes.borrow_mut();
        if probes.len() > 256 {
            probes.clear();
        }
        probes.insert(key, is_root);
    });
    is_root
}

// ---------------------------------------------------------------------------
// Locale handling (ULocale semantics)

/// Parses vanilla's ICU `ULocale` id shapes (`zh_cn`, `""`/`root`,
/// `ja_JP@calendar=japanese`) into a BCP-47 `Locale`; anything unparseable
/// degrades to root exactly as ICU falls back to root data.
#[cfg(not(target_arch = "wasm32"))]
fn parse_ulocale(id: &str) -> Locale {
    let (base, keywords) = match id.split_once('@') {
        Some((base, keywords)) => (base, Some(keywords)),
        None => (id, None),
    };
    let base = base.trim().replace(['_', ' '], "-");
    let mut locale = if base.is_empty() || base.eq_ignore_ascii_case("root") {
        Locale::UNKNOWN
    } else {
        // `ULocale` accepts ids BCP-47 cannot represent (4+-letter
        // languages); their bundles do not exist, so ICU4J formats them with
        // the JVM default locale's data, which bbb pins to English.
        Locale::try_from_str(&base)
            .or_else(|_| Locale::try_from_str("en"))
            .unwrap_or(Locale::UNKNOWN)
    };
    LocaleCanonicalizer::new_extended().canonicalize(&mut locale);
    if let Some(keywords) = keywords {
        for keyword in keywords.split(';') {
            let Some((key, value)) = keyword.split_once('=') else {
                continue;
            };
            let key = match key.trim().to_ascii_lowercase().as_str() {
                "calendar" => "ca",
                "numbers" => "nu",
                _ => continue,
            };
            let Ok(key) = unicode_ext::Key::try_from_str(key) else {
                continue;
            };
            let value = match value.trim().to_ascii_lowercase().as_str() {
                // ICU keyword values whose BCP-47 form differs.
                "gregorian" => "gregory".to_string(),
                "ethiopic-amete-alem" => "ethioaa".to_string(),
                other => other.to_string(),
            };
            if let Ok(value) = unicode_ext::Value::try_from_str(&value) {
                locale.extensions.unicode.keywords.set(key, value);
            }
        }
    }
    locale
}

/// `Calendar.getInstance(locale)` selects the calendar from the `ca` keyword
/// or the likely-subtags region (`th_th` -> Buddhist); maximizing first
/// matches ICU4J, which also resolves bare languages and root through likely
/// subtags.
#[cfg(not(target_arch = "wasm32"))]
fn calendar_kind_for(locale: &Locale) -> AnyCalendarKind {
    let mut maximized = locale.clone();
    LocaleExpander::new_extended().maximize(&mut maximized.id);
    AnyCalendarKind::new((&maximized).into())
}

#[cfg(not(target_arch = "wasm32"))]
fn locale_region(locale: &Locale) -> String {
    let mut maximized = locale.clone();
    LocaleExpander::new_extended().maximize(&mut maximized.id);
    maximized
        .id
        .region
        .map(|region| region.to_string())
        .unwrap_or_else(|| "001".to_string())
}

/// Locale keys for the generated ICU4J tables: the full vanilla-style id
/// first (`zh_cn`), then the bare language, then root.
#[cfg(not(target_arch = "wasm32"))]
fn table_keys(locale: &Locale) -> [String; 3] {
    let language = locale.id.language.to_string().to_ascii_lowercase();
    let full = match locale.id.region {
        Some(region) => format!("{}_{}", language, region.to_string().to_ascii_lowercase()),
        None => language.clone(),
    };
    let root = if language == "und" {
        String::new()
    } else {
        language
    };
    [full, root, String::new()]
}

// ---------------------------------------------------------------------------
// Week data (ICU4J `Calendar.getWeekDataForRegion`)

#[derive(Clone, Copy)]
struct WeekData {
    /// 1 = Sunday .. 7 = Saturday, as ICU4J numbers days.
    first_day: i32,
    minimal_days: i32,
}

#[cfg(not(target_arch = "wasm32"))]
fn week_data_for(locale: &Locale) -> WeekData {
    let region = locale_region(locale);
    let row = tables_26_1::WEEK_DATA
        .iter()
        .find(|(row_region, _, _)| *row_region == region)
        .or_else(|| {
            tables_26_1::WEEK_DATA
                .iter()
                .find(|(row_region, _, _)| *row_region == "001")
        });
    match row {
        Some((_, first_day, minimal_days)) => WeekData {
            first_day: i32::from(*first_day),
            minimal_days: i32::from(*minimal_days),
        },
        None => WeekData {
            first_day: 2,
            minimal_days: 1,
        },
    }
}

/// ICU4J `Calendar.weekNumber(dayOfPeriod, dayOfWeek)`.
fn icu4j_week_number(day_of_period: i32, day_of_week: i32, week: WeekData) -> i32 {
    let mut period_start = (day_of_week - week.first_day - day_of_period + 1) % 7;
    if period_start < 0 {
        period_start += 7;
    }
    let mut week_no = (day_of_period + period_start - 1) / 7;
    if (7 - period_start) >= week.minimal_days {
        week_no += 1;
    }
    week_no
}

struct WeekFields {
    week_of_year: i32,
    week_of_month: i32,
    /// Offset to add to the calendar year to get the week year.
    week_year_delta: i32,
    /// 1..7 counted from the locale's first weekday.
    local_day_of_week: i32,
}

/// ICU4J `Calendar.computeWeekFields`, generic over the dispatched calendar:
/// everything derives from day-of-year, day-of-week and the year lengths.
fn icu4j_week_fields(
    day_of_year: i32,
    day_of_month: i32,
    day_of_week: i32,
    days_in_year: i32,
    days_in_previous_year: i32,
    week: WeekData,
) -> WeekFields {
    let mut rel_dow = (day_of_week + 7 - week.first_day) % 7;
    if rel_dow < 0 {
        rel_dow += 7;
    }
    let rel_dow_jan1 = (day_of_week - day_of_year + 7001 - week.first_day) % 7;
    let mut woy = (day_of_year - 1 + rel_dow_jan1) / 7;
    if (7 - rel_dow_jan1) >= week.minimal_days {
        woy += 1;
    }
    let mut week_year_delta = 0;
    if woy == 0 {
        woy = icu4j_week_number(day_of_year + days_in_previous_year, day_of_week, week);
        week_year_delta = -1;
    } else if day_of_year >= days_in_year - 5 {
        let mut last_rel_dow = (rel_dow + days_in_year - day_of_year) % 7;
        if last_rel_dow < 0 {
            last_rel_dow += 7;
        }
        if ((6 - last_rel_dow) >= week.minimal_days) && ((day_of_year + 7 - rel_dow) > days_in_year)
        {
            woy = 1;
            week_year_delta = 1;
        }
    }
    WeekFields {
        week_of_year: woy,
        week_of_month: icu4j_week_number(day_of_month, day_of_week, week),
        week_year_delta,
        local_day_of_week: rel_dow + 1,
    }
}

// ---------------------------------------------------------------------------
// Time zone resolution (vanilla `TimeZone.getTimeZone` semantics)

#[cfg(not(target_arch = "wasm32"))]
struct ZoneData {
    offset_seconds: i32,
    /// Zone id as `TimeZone.getID()` reports it: named ids stay as given,
    /// custom offsets normalize to `GMT±HH:MM`.
    vv_id: String,
    /// BCP-47 id (`cnsha`), `unk` for custom offsets.
    bcp47_id: String,
    /// CLDR maps every territorial zone to a country; `Etc/*`, `UTC`, `GMT`
    /// and custom offsets have none, which routes their generic names to the
    /// localized-GMT format.
    has_territory: bool,
    /// Whether the zone observes daylight time in the local year — ICU4J only
    /// substitutes a metazone standard name for a missing generic name on
    /// single-name zones.
    observes_dst: bool,
    /// Zone input for ICU4X name formatting; `None` when vanilla would use
    /// the JVM default zone (bbb keeps the local clock but owns no zone id).
    icu_zone: Option<TimeZoneInfo<AtTime>>,
    local: chrono::NaiveDateTime,
    millisecond: u32,
}

#[cfg(not(target_arch = "wasm32"))]
fn resolve_zone(epoch_millis: i64, time_zone: Option<&str>) -> Option<ZoneData> {
    let utc = Utc.timestamp_millis_opt(epoch_millis).single()?;
    match time_zone {
        Some(id) => {
            if let Ok(tz) = id.parse::<Tz>() {
                let zoned = utc.with_timezone(&tz);
                let icu_full: ZonedDateTime<cal::Gregorian, TimeZoneInfo<AtTime>> = (&zoned).into();
                let bcp47 = IanaParser::new().parse(tz.name());
                let year = zoned.year();
                let observes_dst = [(1, 15), (7, 15)].iter().any(|(month, day)| {
                    tz.with_ymd_and_hms(year, *month, *day, 12, 0, 0)
                        .single()
                        .is_some_and(|probe| !probe.offset().dst_offset().is_zero())
                }) || !zoned.offset().dst_offset().is_zero();
                Some(ZoneData {
                    offset_seconds: zoned.offset().fix().local_minus_utc(),
                    vv_id: id.to_string(),
                    bcp47_id: bcp47.to_string(),
                    has_territory: !zone_is_offset_only(&bcp47.to_string()),
                    observes_dst,
                    icu_zone: Some(icu_full.zone),
                    local: zoned.naive_local(),
                    millisecond: zoned.timestamp_subsec_millis(),
                })
            } else {
                let offset = parse_custom_gmt_offset(id)?;
                let zoned = utc.with_timezone(&offset);
                let utc_offset = UtcOffset::try_from_seconds(offset.local_minus_utc()).ok()?;
                let icu_zone = IcuTimeZone::UNKNOWN
                    .with_offset(Some(utc_offset))
                    .with_zone_name_timestamp(ZoneNameTimestamp::from_epoch_seconds(
                        utc.timestamp(),
                    ));
                Some(ZoneData {
                    offset_seconds: offset.local_minus_utc(),
                    vv_id: custom_gmt_id(offset.local_minus_utc()),
                    bcp47_id: "unk".to_string(),
                    has_territory: false,
                    observes_dst: false,
                    icu_zone: Some(icu_zone),
                    local: zoned.naive_local(),
                    millisecond: zoned.timestamp_subsec_millis(),
                })
            }
        }
        None => {
            let zoned = Local.timestamp_millis_opt(epoch_millis).single()?;
            Some(ZoneData {
                offset_seconds: zoned.offset().fix().local_minus_utc(),
                vv_id: String::new(),
                bcp47_id: "unk".to_string(),
                has_territory: false,
                observes_dst: false,
                icu_zone: None,
                local: zoned.naive_local(),
                millisecond: zoned.timestamp_subsec_millis(),
            })
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn zone_is_offset_only(bcp47: &str) -> bool {
    bcp47 == "unk" || bcp47 == "utc" || bcp47 == "gmt" || bcp47.starts_with("utc")
}

/// ICU custom ids: a case-insensitive `GMT` prefix followed by a signed
/// offset (`GMT+3`, `gmt-02:30`, `GMT+0530`). Anything else is
/// `TimeZone.UNKNOWN_ZONE`, which vanilla's codec rejects (`UTC+3` and `Z`
/// are not custom-id shapes).
fn parse_custom_gmt_offset(id: &str) -> Option<FixedOffset> {
    if id.len() < 3 || !id.is_char_boundary(3) || !id[..3].eq_ignore_ascii_case("GMT") {
        return None;
    }
    let rest = &id[3..];
    let (sign, digits) = match rest.as_bytes().first() {
        Some(b'+') => (1, &rest[1..]),
        Some(b'-') => (-1, &rest[1..]),
        _ => return None,
    };
    if !digits.is_ascii() {
        return None;
    }
    let (hours, minutes, seconds) = if let Some((h, rest)) = digits.split_once(':') {
        let (m, s) = match rest.split_once(':') {
            Some((m, s)) => (m, Some(s)),
            None => (rest, None),
        };
        (
            h.parse::<i32>().ok()?,
            m.parse::<i32>().ok()?,
            s.map_or(Some(0), |s| s.parse::<i32>().ok())?,
        )
    } else {
        match digits.len() {
            1 | 2 => (digits.parse::<i32>().ok()?, 0, 0),
            3 => (digits[..1].parse().ok()?, digits[1..].parse().ok()?, 0),
            4 => (digits[..2].parse().ok()?, digits[2..].parse().ok()?, 0),
            _ => return None,
        }
    };
    if !(0..=23).contains(&hours) || !(0..=59).contains(&minutes) || !(0..=59).contains(&seconds) {
        return None;
    }
    FixedOffset::east_opt(sign * (hours * 3600 + minutes * 60 + seconds))
}

fn custom_gmt_id(offset_seconds: i32) -> String {
    let sign = if offset_seconds < 0 { '-' } else { '+' };
    let total = offset_seconds.abs();
    if total % 60 == 0 {
        format!("GMT{sign}{:02}:{:02}", total / 3600, (total / 60) % 60)
    } else {
        format!(
            "GMT{sign}{:02}:{:02}:{:02}",
            total / 3600,
            (total / 60) % 60,
            total % 60
        )
    }
}

// ---------------------------------------------------------------------------
// Calendar dispatch

/// `icu_datetime` can format every calendar vanilla's per-locale defaults can
/// produce (Gregorian everywhere except Buddhist/Persian) plus the common
/// `@calendar=` keywords; Hijri and the cyclic Chinese/Dangi calendars have
/// no ICU4X pattern formatter and stay rejected.
#[cfg(not(target_arch = "wasm32"))]
fn dispatch_calendar(
    kind: AnyCalendarKind,
    locale: &Locale,
    zone: &ZoneData,
    pattern: &str,
) -> Option<String> {
    // ICU4J's Buddhist and Japanese calendars keep the Gregorian extended
    // year, which `u` and the week year (`Y`) format; era years stay on `y`.
    let iso_extended = matches!(kind, AnyCalendarKind::Buddhist | AnyCalendarKind::Japanese);
    match kind {
        AnyCalendarKind::Buddhist => {
            format_in_calendar(cal::Buddhist, locale, zone, pattern, iso_extended)
        }
        AnyCalendarKind::Coptic => {
            format_in_calendar(cal::Coptic, locale, zone, pattern, iso_extended)
        }
        AnyCalendarKind::Ethiopian => {
            format_in_calendar(cal::Ethiopian::new(), locale, zone, pattern, iso_extended)
        }
        AnyCalendarKind::Hebrew => {
            format_in_calendar(cal::Hebrew, locale, zone, pattern, iso_extended)
        }
        AnyCalendarKind::Indian => {
            format_in_calendar(cal::Indian, locale, zone, pattern, iso_extended)
        }
        AnyCalendarKind::Japanese => {
            format_in_calendar(cal::Japanese::new(), locale, zone, pattern, iso_extended)
        }
        AnyCalendarKind::Persian => {
            format_in_calendar(cal::Persian, locale, zone, pattern, iso_extended)
        }
        AnyCalendarKind::HijriUmmAlQura => format_in_calendar(
            cal::Hijri::new_umm_al_qura(),
            locale,
            zone,
            pattern,
            iso_extended,
        ),
        AnyCalendarKind::HijriTabularTypeIIFriday => format_in_calendar(
            cal::Hijri::new_tabular(
                cal::hijri::TabularAlgorithmLeapYears::TypeII,
                cal::hijri::TabularAlgorithmEpoch::Friday,
            ),
            locale,
            zone,
            pattern,
            iso_extended,
        ),
        AnyCalendarKind::HijriTabularTypeIIThursday => format_in_calendar(
            cal::Hijri::new_tabular(
                cal::hijri::TabularAlgorithmLeapYears::TypeII,
                cal::hijri::TabularAlgorithmEpoch::Thursday,
            ),
            locale,
            zone,
            pattern,
            iso_extended,
        ),
        AnyCalendarKind::Roc => format_in_calendar(cal::Roc, locale, zone, pattern, iso_extended),
        _ => format_in_calendar(cal::Gregorian, locale, zone, pattern, iso_extended),
    }
}

// ---------------------------------------------------------------------------
// Pattern scanning

enum Segment {
    Literal(String),
    Field(char, usize),
}

/// Vanilla `SimpleDateFormat` quoting: `''` is a literal quote, quoted runs
/// are literal text, and an unterminated quote runs to the end of the
/// pattern.
fn scan_pattern(pattern: &str) -> Option<Vec<Segment>> {
    let mut segments = Vec::new();
    let mut chars = pattern.chars().peekable();
    let mut literal = String::new();
    while let Some(ch) = chars.next() {
        if ch == '\'' {
            if matches!(chars.peek(), Some('\'')) {
                chars.next();
                literal.push('\'');
                continue;
            }
            while let Some(quoted) = chars.next() {
                if quoted == '\'' {
                    if matches!(chars.peek(), Some('\'')) {
                        chars.next();
                        literal.push('\'');
                    } else {
                        break;
                    }
                } else {
                    literal.push(quoted);
                }
            }
            continue;
        }
        if ch.is_ascii_alphabetic() {
            if !literal.is_empty() {
                segments.push(Segment::Literal(std::mem::take(&mut literal)));
            }
            let mut count = 1;
            while matches!(chars.peek(), Some(next) if *next == ch) {
                chars.next();
                count += 1;
            }
            segments.push(Segment::Field(ch, count));
        } else {
            literal.push(ch);
        }
    }
    if !literal.is_empty() {
        segments.push(Segment::Literal(literal));
    }
    Some(segments)
}

// ---------------------------------------------------------------------------
// The formatter core, monomorphized per calendar

#[cfg(not(target_arch = "wasm32"))]
macro_rules! define_format_in_calendar {
    ($($calendar:ty),+ $(,)?) => {
        trait FormatInCalendar: Sized {
            fn format_local_time(
                self,
                locale: &Locale,
                zone: &ZoneData,
                pattern: &str,
                iso_extended: bool,
            ) -> Option<String>;
        }
        $(
            impl FormatInCalendar for $calendar {
                fn format_local_time(
                    self,
                    locale: &Locale,
                    zone: &ZoneData,
                    pattern: &str,
                    iso_extended: bool,
                ) -> Option<String> {
                    let date = Date::try_new_iso(
                        zone.local.year(),
                        zone.local.month() as u8,
                        zone.local.day() as u8,
                    )
                    .ok()?
                    .to_calendar(self);
                    let time = Time::try_new(
                        zone.local.hour() as u8,
                        zone.local.minute() as u8,
                        zone.local.second() as u8,
                        zone.millisecond * 1_000_000,
                    )
                    .ok()?;
                    let mut fields = CalendarFields::new(&date, zone.local.year());
                    if iso_extended {
                        fields.extended_year = zone.local.year();
                    }
                    let digits = digits_for(locale);
                    let week = week_data_for(locale);
                    let zoned = ZonedDateTime {
                        date,
                        time,
                        zone: zone.icu_zone.clone().unwrap_or_else(placeholder_zone),
                    };
                    let segments = scan_pattern(pattern)?;
                    let mut result = String::new();
                    for segment in &segments {
                        match segment {
                            Segment::Literal(text) => result.push_str(text),
                            Segment::Field(ch, count) => {
                                match field_output(*ch, *count, &fields, zone, locale, &digits, week)? {
                                    FieldOutput::Text(text) => result.push_str(&text),
                                    // Each delegated field formats through its
                                    // own names instance: one instance cannot
                                    // hold two widths of the same name kind
                                    // (`eee eeee` would fail to load).
                                    FieldOutput::Delegate(mini) => {
                                        let mini: DateTimePattern = mini.parse().ok()?;
                                        let mut names = FixedCalendarDateTimeNames::<
                                            $calendar,
                                            CompositeFieldSet,
                                        >::try_new(locale.into())
                                        .ok()?;
                                        let formatter =
                                            names.include_for_pattern(&mini).ok()?;
                                        result.push_str(
                                            &formatter
                                                .format(&zoned)
                                                .try_write_to_string()
                                                .ok()?,
                                        );
                                    }
                                }
                            }
                        }
                    }
                    Some(result)
                }
            }
        )+
    };
}

#[cfg(not(target_arch = "wasm32"))]
define_format_in_calendar!(
    cal::Buddhist,
    cal::Coptic,
    cal::Ethiopian,
    cal::Gregorian,
    cal::Hebrew,
    cal::Indian,
    cal::Japanese,
    cal::Persian,
    cal::Roc,
    cal::Hijri<cal::hijri::UmmAlQura>,
    cal::Hijri<cal::hijri::TabularAlgorithm>,
);

#[cfg(not(target_arch = "wasm32"))]
fn format_in_calendar<C: FormatInCalendar>(
    calendar: C,
    locale: &Locale,
    zone: &ZoneData,
    pattern: &str,
    iso_extended: bool,
) -> Option<String> {
    calendar.format_local_time(locale, zone, pattern, iso_extended)
}

/// Placeholder for the zone-less local-clock path; the pattern never contains
/// zone fields then (they return `None` earlier), so this is never read.
#[cfg(not(target_arch = "wasm32"))]
fn placeholder_zone() -> TimeZoneInfo<AtTime> {
    IcuTimeZone::UNKNOWN
        .with_offset(None)
        .with_zone_name_timestamp(ZoneNameTimestamp::far_in_past())
}

/// The calendar-dependent numeric facts every native field derives from.
#[cfg(not(target_arch = "wasm32"))]
struct CalendarFields {
    year: i32,
    extended_year: i32,
    related_iso_year: i32,
    month: i32,
    day_of_month: i32,
    day_of_year: i32,
    days_in_year: i32,
    days_in_previous_year: i32,
    /// 1 = Sunday .. 7 = Saturday (ICU4J numbering).
    day_of_week: i32,
    /// Rata die of the day, shared across calendars.
    rata_die: i64,
}

#[cfg(not(target_arch = "wasm32"))]
impl CalendarFields {
    fn new<A: icu_calendar::AsCalendar + Clone>(date: &Date<A>, iso_year: i32) -> Self {
        use icu_calendar::types::Weekday;
        let year_info = date.year();
        let (year, extended_year) = match year_info {
            icu_calendar::types::YearInfo::Era(era) => (era.year, era.extended_year),
            icu_calendar::types::YearInfo::Cyclic(cyclic) => {
                (cyclic.related_iso, cyclic.related_iso)
            }
            _ => (iso_year, iso_year),
        };
        let day_of_year = i32::from(date.day_of_year().0);
        let rata_die = date.to_rata_die();
        let previous_year_last_day = Date::from_rata_die(
            rata_die - i64::from(day_of_year),
            date.calendar_wrapper().clone(),
        );
        Self {
            year,
            extended_year,
            related_iso_year: iso_year,
            month: i32::from(date.month().ordinal),
            day_of_month: i32::from(date.day_of_month().0),
            day_of_year,
            days_in_year: i32::from(date.days_in_year()),
            days_in_previous_year: i32::from(previous_year_last_day.day_of_year().0),
            day_of_week: match date.weekday() {
                Weekday::Sunday => 1,
                Weekday::Monday => 2,
                Weekday::Tuesday => 3,
                Weekday::Wednesday => 4,
                Weekday::Thursday => 5,
                Weekday::Friday => 6,
                Weekday::Saturday => 7,
            },
            rata_die: rata_die.to_i64_date(),
        }
    }
}

// ---------------------------------------------------------------------------
// Per-field routing

/// One resolved pattern field: native text, or a single-field mini-pattern
/// to delegate to ICU4X (with ICU4J's width clamps applied).
#[cfg(not(target_arch = "wasm32"))]
enum FieldOutput {
    Text(String),
    Delegate(String),
}

/// Resolves one pattern field. `None` rejects the whole pattern like
/// `SimpleDateFormat` does for an illegal letter.
#[cfg(not(target_arch = "wasm32"))]
#[allow(clippy::too_many_arguments)]
fn field_output(
    ch: char,
    count: usize,
    fields: &CalendarFields,
    zone: &ZoneData,
    locale: &Locale,
    digits: &Digits,
    week: WeekData,
) -> Option<FieldOutput> {
    let delegate = |ch: char, count: usize| {
        Some(FieldOutput::Delegate(
            std::iter::repeat_n(ch, count).collect(),
        ))
    };
    let text = |value: String| Some(FieldOutput::Text(value));
    match ch {
        // Era and month/weekday/day-period text delegate to ICU4X; overlong
        // widths reproduce ICU4J (`GGGGGG` -> abbreviated, `MMMMMM` ->
        // zero-padded numeric, `EEEEEEE` -> abbreviated).
        'G' => delegate('G', if count > 5 { 3 } else { count }),
        'M' | 'L' => {
            if (3..=5).contains(&count) {
                delegate(ch, count)
            } else {
                text(digits.pad(fields.month, count))
            }
        }
        'E' => delegate('E', if count > 6 { 3 } else { count }),
        'e' | 'c' => {
            if count <= 2 {
                let local_dow = icu4j_week_fields(
                    fields.day_of_year,
                    fields.day_of_month,
                    fields.day_of_week,
                    fields.days_in_year,
                    fields.days_in_previous_year,
                    week,
                )
                .local_day_of_week;
                // `ee` pads, standalone `cc` does not (ICU4J).
                let width = if ch == 'e' { count } else { 1 };
                text(digits.pad(local_dow, width))
            } else {
                delegate(ch, if count > 6 { 3 } else { count })
            }
        }
        'a' => delegate('a', count.min(5)),
        'b' => {
            // ICU4J prints AM at exact midnight; only noon gets its own name.
            let midnight =
                zone.local.hour() == 0 && zone.local.minute() == 0 && zone.local.second() == 0;
            delegate(if midnight { 'a' } else { 'b' }, count.min(5))
        }
        // Numeric fields are native: ICU4J's padding quirks stay byte-exact
        // and the locale digits come from the transliteration table.
        'y' => text(digits.year(fields.year, count)),
        'u' => text(digits.pad(fields.extended_year, count)),
        'r' => text(digits.pad(fields.related_iso_year, count)),
        'Y' => {
            let delta = icu4j_week_fields(
                fields.day_of_year,
                fields.day_of_month,
                fields.day_of_week,
                fields.days_in_year,
                fields.days_in_previous_year,
                week,
            )
            .week_year_delta;
            text(digits.year(fields.extended_year + delta, count))
        }
        'w' => {
            let woy = icu4j_week_fields(
                fields.day_of_year,
                fields.day_of_month,
                fields.day_of_week,
                fields.days_in_year,
                fields.days_in_previous_year,
                week,
            )
            .week_of_year;
            text(digits.pad(woy, count))
        }
        'W' => {
            let wom = icu4j_week_fields(
                fields.day_of_year,
                fields.day_of_month,
                fields.day_of_week,
                fields.days_in_year,
                fields.days_in_previous_year,
                week,
            )
            .week_of_month;
            text(digits.pad(wom, count))
        }
        'd' => text(digits.pad(fields.day_of_month, count)),
        'D' => text(digits.pad(fields.day_of_year, count)),
        'F' => text(digits.pad((fields.day_of_month - 1) / 7 + 1, count)),
        'g' => text(digits.pad_i64(fields.rata_die + 1_721_425, count)),
        'H' => text(digits.pad(zone.local.hour() as i32, count)),
        'k' => {
            let hour = if zone.local.hour() == 0 {
                24
            } else {
                zone.local.hour() as i32
            };
            text(digits.pad(hour, count))
        }
        'K' => text(digits.pad((zone.local.hour() % 12) as i32, count)),
        'h' => {
            let hour = zone.local.hour() % 12;
            let hour = if hour == 0 { 12 } else { hour as i32 };
            text(digits.pad(hour, count))
        }
        'm' => text(digits.pad(zone.local.minute() as i32, count)),
        's' => text(digits.pad(zone.local.second() as i32, count)),
        'S' => text(digits.fraction(zone.millisecond, count)),
        'A' => {
            let millis = (((zone.local.hour() * 60 + zone.local.minute()) * 60
                + zone.local.second())
                * 1_000)
                + zone.millisecond;
            text(digits.pad_i64(i64::from(millis), count))
        }
        'Q' | 'q' => {
            let quarter = (fields.month - 1) / 3 + 1;
            if count <= 2 {
                text(digits.pad(quarter, count))
            } else {
                let form = match (ch, count.min(5)) {
                    ('Q', 3) => "QQQ",
                    ('Q', 4) => "QQQQ",
                    ('Q', _) => "QQQQQ",
                    ('q', 3) => "qqq",
                    ('q', 4) => "qqqq",
                    (_, _) => "qqqqq",
                };
                text(quarter_name(locale, form, quarter)?)
            }
        }
        // Zone fields format through the zone helper and splice as literals.
        'z' | 'Z' | 'O' | 'v' | 'V' | 'x' | 'X' => text(zone_field_value(ch, count, zone, locale)?),
        // ICU4J ignores the deprecated `l` and formats nothing for it.
        'l' => text(String::new()),
        'B' => text(day_period_name(locale, zone, count)?),
        // Every other letter is an ICU4J `IllegalArgumentException`.
        _ => None,
    }
}

/// ICU4J's flexible day-period name for the local time of day.
///
/// ICU4X has no data for `B`, so the names and their boundaries come from the
/// generated table; a locale without day-period rules falls back to the root
/// row, which ICU4J renders as plain AM/PM.
#[cfg(not(target_arch = "wasm32"))]
fn day_period_name(locale: &Locale, zone: &ZoneData, count: usize) -> Option<String> {
    let index = match count {
        1..=3 => 0,
        4 => 1,
        _ => 2,
    };
    let minute = u16::try_from(zone.local.hour() * 60 + zone.local.minute()).ok()?;
    table_keys(locale).iter().find_map(|key| {
        tables_26_1::DAY_PERIODS
            .iter()
            .rfind(|(row_locale, start, _)| row_locale == key && *start <= minute)
            .map(|(_, _, names)| names[index].to_string())
    })
}

#[cfg(not(target_arch = "wasm32"))]
fn quarter_name(locale: &Locale, form: &str, quarter: i32) -> Option<String> {
    let index = usize::try_from(quarter - 1).ok()?.min(3);
    table_keys(locale).iter().find_map(|key| {
        tables_26_1::QUARTERS
            .iter()
            .find(|(row_locale, row_form, _)| row_locale == key && *row_form == form)
            .map(|(_, _, names)| names[index].to_string())
    })
}

// ---------------------------------------------------------------------------
// Digits

/// The locale's decimal digits, discovered through `icu_decimal` so the
/// numbering-system resolution (including `-u-nu-` overrides) matches the
/// delegated fields.
struct Digits([char; 10]);

#[cfg(not(target_arch = "wasm32"))]
fn digits_for(locale: &Locale) -> Digits {
    let mut options = DecimalFormatterOptions::default();
    options.grouping_strategy = Some(GroupingStrategy::Never);
    let digits = DecimalFormatter::try_new(locale.into(), options)
        .ok()
        .map(|formatter| {
            use writeable::Writeable;
            let mut digits = ['0'; 10];
            for (value, slot) in digits.iter_mut().enumerate().map(|(i, d)| (i as u32, d)) {
                let decimal = icu_decimal::input::Decimal::from(value);
                let formatted = formatter.format(&decimal).write_to_string().into_owned();
                if let Some(ch) = formatted.chars().next() {
                    *slot = ch;
                }
            }
            digits
        })
        .unwrap_or(['0', '1', '2', '3', '4', '5', '6', '7', '8', '9']);
    Digits(digits)
}

impl Digits {
    fn transliterate(&self, ascii: &str) -> String {
        ascii
            .chars()
            .map(|ch| match ch.to_digit(10) {
                Some(digit) => self.0[digit as usize],
                None => ch,
            })
            .collect()
    }

    fn pad(&self, value: i32, width: usize) -> String {
        self.pad_i64(i64::from(value), width)
    }

    fn pad_i64(&self, value: i64, width: usize) -> String {
        let ascii = if width <= 1 {
            value.to_string()
        } else if value < 0 {
            format!("-{:0width$}", value.abs(), width = width)
        } else {
            format!("{value:0width$}")
        };
        self.transliterate(&ascii)
    }

    /// `y`/`Y`: width 2 truncates to the two low-order digits.
    fn year(&self, year: i32, width: usize) -> String {
        if width == 2 {
            self.transliterate(&format!("{:02}", year.rem_euclid(100)))
        } else {
            self.pad(year, width)
        }
    }

    /// `S`: millisecond digits, truncated or right-padded with zeros.
    fn fraction(&self, millisecond: u32, width: usize) -> String {
        let mut ascii = format!("{millisecond:03}");
        if width <= 3 {
            ascii.truncate(width);
        } else {
            ascii.extend(std::iter::repeat_n('0', width - 3));
        }
        self.transliterate(&ascii)
    }
}

// ---------------------------------------------------------------------------
// Zone fields

/// Formats one time-zone pattern field with vanilla ICU4J semantics. Name
/// forms delegate to ICU4X, then three ICU4J behaviors are layered on top:
/// the localized-GMT zero format, the metazone-standard fallback for generic
/// names, and the invalid-width `"null"` literals ICU4J prints.
#[cfg(not(target_arch = "wasm32"))]
fn zone_field_value(ch: char, count: usize, zone: &ZoneData, locale: &Locale) -> Option<String> {
    let offset = zone.offset_seconds;
    match (ch, count) {
        ('z', 1..=3) => zone_name(zone, locale, "z", GmtZeroForm::Short),
        ('z', _) => zone_name(zone, locale, "zzzz", GmtZeroForm::Long),
        ('Z', 1..=3) => Some(rfc822_offset(offset)),
        ('Z', 5) => iso8601_offset(offset, 5, true),
        ('Z', _) | ('O', 4) => localized_gmt(zone, locale, GmtZeroForm::Long),
        ('O', 1) => localized_gmt(zone, locale, GmtZeroForm::Short),
        ('O', _) => Some("null".to_string()),
        ('v', 1) => generic_zone_name(zone, locale, GmtZeroForm::Short),
        ('v', 4) => generic_zone_name(zone, locale, GmtZeroForm::Long),
        ('v', _) => Some("null".to_string()),
        ('V', 1) => Some(zone.bcp47_id.clone()),
        ('V', 2) => Some(zone.vv_id.clone()),
        ('V', 3) => exemplar_city(zone, locale),
        ('V', 4) => {
            zone.icu_zone?;
            if locale.id.language.is_unknown() && zone.has_territory {
                if let Some(location) = root_zone_location(zone) {
                    return Some(location);
                }
            }
            let (real, artifact) = zone_pair(zone, locale, "VVVV")?;
            if real == artifact && offset == 0 {
                gmt_zero(locale, GmtZeroForm::Long)
            } else {
                Some(real)
            }
        }
        ('V', _) => Some("null".to_string()),
        ('x', 1..=5) => iso8601_offset(offset, count, false),
        ('X', 1..=5) => iso8601_offset(offset, count, true),
        ('x' | 'X', _) => Some("null".to_string()),
        _ => None,
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[derive(Clone, Copy)]
enum GmtZeroForm {
    Short,
    Long,
}

#[cfg(not(target_arch = "wasm32"))]
fn gmt_zero(locale: &Locale, form: GmtZeroForm) -> Option<String> {
    table_keys(locale).iter().find_map(|key| {
        tables_26_1::GMT_ZERO
            .iter()
            .find(|(row_locale, _, _)| row_locale == key)
            .map(|(_, short, long)| match form {
                GmtZeroForm::Short => short.to_string(),
                GmtZeroForm::Long => long.to_string(),
            })
    })
}

#[cfg(not(target_arch = "wasm32"))]
fn localized_gmt(zone: &ZoneData, locale: &Locale, form: GmtZeroForm) -> Option<String> {
    if zone.offset_seconds == 0 {
        return gmt_zero(locale, form);
    }
    zone.icu_zone.as_ref()?;
    let pattern = match form {
        GmtZeroForm::Short => "O",
        GmtZeroForm::Long => "OOOO",
    };
    zone_pair(zone, locale, pattern).map(|(real, _)| real)
}

/// Specific names (`z`); ICU4X's own fallback to the localized offset is kept
/// but the zero-offset form goes through ICU4J's `gmtZeroFormat`.
#[cfg(not(target_arch = "wasm32"))]
fn zone_name(zone: &ZoneData, locale: &Locale, pattern: &str, form: GmtZeroForm) -> Option<String> {
    zone.icu_zone.as_ref()?;
    // ICU4J's root bundle has no long specific names (`zzzz` under the
    // default locale is the localized GMT offset), while ICU4X's root data
    // leaks a location rendering there.
    if locale.id.language.is_unknown() && matches!(form, GmtZeroForm::Long) {
        return localized_gmt(zone, locale, form);
    }
    let (real, artifact) = zone_pair(zone, locale, pattern)?;
    if real == artifact && zone.offset_seconds == 0 {
        gmt_zero(locale, form)
    } else {
        Some(real)
    }
}

/// ICU4J's root data renders territorial zones' generic and location formats
/// as the zone's country code.
#[cfg(not(target_arch = "wasm32"))]
fn root_zone_location(zone: &ZoneData) -> Option<String> {
    tables_26_1::ROOT_ZONE_LOCATION
        .iter()
        .find(|(zone_id, _)| *zone_id == zone.vv_id)
        .map(|(_, value)| value.to_string())
}

/// Generic names (`v`/`vvvv`) with ICU4J's chain: offset-only zones use the
/// localized GMT format, and a generic name that fell back to the location
/// format is replaced by the metazone standard name when the zone keeps a
/// single name year-round (`vvvv` Asia/Shanghai -> 中国标准时间).
#[cfg(not(target_arch = "wasm32"))]
fn generic_zone_name(zone: &ZoneData, locale: &Locale, form: GmtZeroForm) -> Option<String> {
    zone.icu_zone.as_ref()?;
    if !zone.has_territory {
        // The GMT zone maps to the GMT metazone, whose long standard name
        // substitutes for the missing generic (`vvvv` -> Greenwich Mean Time);
        // every other offset-only zone renders the localized GMT format.
        if zone.bcp47_id == "gmt" && matches!(form, GmtZeroForm::Long) {
            return zone_pair(zone, locale, "zzzz").map(|(real, _)| real);
        }
        if zone.offset_seconds == 0 {
            return gmt_zero(locale, form);
        }
        let pattern = match form {
            GmtZeroForm::Short => "v",
            GmtZeroForm::Long => "vvvv",
        };
        return zone_pair(zone, locale, pattern).map(|(_, artifact)| artifact);
    }
    if locale.id.language.is_unknown() {
        if let Some(location) = root_zone_location(zone) {
            return Some(location);
        }
    }
    let (generic_pattern, specific_pattern) = match form {
        GmtZeroForm::Short => ("v", "z"),
        GmtZeroForm::Long => ("vvvv", "zzzz"),
    };
    let (real, artifact) = zone_pair(zone, locale, generic_pattern)?;
    if real == artifact {
        return if zone.offset_seconds == 0 {
            gmt_zero(locale, form)
        } else {
            Some(real)
        };
    }
    let (location, _) = zone_pair(zone, locale, "VVVV")?;
    if real == location && !zone.observes_dst {
        let (specific, specific_artifact) = zone_pair(zone, locale, specific_pattern)?;
        if specific != specific_artifact {
            return Some(specific);
        }
    }
    Some(real)
}

/// `VVV`: ICU4X's exemplar city, with the zones-without-a-city fallback
/// replaced by ICU4J's localized unknown-city string.
#[cfg(not(target_arch = "wasm32"))]
fn exemplar_city(zone: &ZoneData, locale: &Locale) -> Option<String> {
    zone.icu_zone.as_ref()?;
    let (real, artifact) = zone_pair(zone, locale, "VVV")?;
    if real == artifact {
        table_keys(locale).iter().find_map(|key| {
            tables_26_1::UNKNOWN_CITY
                .iter()
                .find(|(row_locale, _)| row_locale == key)
                .map(|(_, value)| value.to_string())
        })
    } else {
        Some(real)
    }
}

/// Formats a zone mini-pattern for the real zone and for the unknown zone at
/// the same offset; equal outputs mean ICU4X fell back to an offset or
/// unknown-zone rendering, which is where ICU4J behavior is layered on.
#[cfg(not(target_arch = "wasm32"))]
fn zone_pair(zone: &ZoneData, locale: &Locale, pattern: &str) -> Option<(String, String)> {
    let icu_zone = zone.icu_zone.as_ref()?;
    let pattern: DateTimePattern = pattern.parse().ok()?;
    let mut names =
        FixedCalendarDateTimeNames::<cal::Gregorian, CompositeFieldSet>::try_new(locale.into())
            .ok()?;
    let formatter = names.include_for_pattern(&pattern).ok()?;
    let date = Date::try_new_iso(
        zone.local.year(),
        zone.local.month() as u8,
        zone.local.day() as u8,
    )
    .ok()?
    .to_calendar(cal::Gregorian);
    let time = Time::try_new(
        zone.local.hour() as u8,
        zone.local.minute() as u8,
        zone.local.second() as u8,
        zone.millisecond * 1_000_000,
    )
    .ok()?;
    let real = formatter
        .format(&ZonedDateTime {
            date,
            time,
            zone: *icu_zone,
        })
        .try_write_to_string()
        .ok()?
        .into_owned();
    let unknown_zone = IcuTimeZone::UNKNOWN
        .with_offset(icu_zone.offset())
        .with_zone_name_timestamp(icu_zone.zone_name_timestamp());
    let artifact = formatter
        .format(&ZonedDateTime {
            date,
            time,
            zone: unknown_zone,
        })
        .try_write_to_string()
        .ok()?
        .into_owned();
    Some((real, artifact))
}

// ---------------------------------------------------------------------------
// Offset formats (ASCII in every locale, matching ICU4J)

fn rfc822_offset(offset_seconds: i32) -> String {
    let (sign, hours, minutes, _) = offset_parts(offset_seconds);
    format!("{sign}{hours:02}{minutes:02}")
}

fn iso8601_offset(offset_seconds: i32, width: usize, zero_as_z: bool) -> Option<String> {
    if offset_seconds == 0 && zero_as_z {
        return Some("Z".to_string());
    }
    let (sign, hours, minutes, seconds) = offset_parts(offset_seconds);
    match width {
        1 => {
            if minutes == 0 {
                Some(format!("{sign}{hours:02}"))
            } else {
                Some(format!("{sign}{hours:02}{minutes:02}"))
            }
        }
        2 => Some(format!("{sign}{hours:02}{minutes:02}")),
        3 => Some(format!("{sign}{hours:02}:{minutes:02}")),
        4 => {
            if seconds == 0 {
                Some(format!("{sign}{hours:02}{minutes:02}"))
            } else {
                Some(format!("{sign}{hours:02}{minutes:02}{seconds:02}"))
            }
        }
        5 => {
            if seconds == 0 {
                Some(format!("{sign}{hours:02}:{minutes:02}"))
            } else {
                Some(format!("{sign}{hours:02}:{minutes:02}:{seconds:02}"))
            }
        }
        _ => None,
    }
}

fn offset_parts(offset_seconds: i32) -> (char, i32, i32, i32) {
    let sign = if offset_seconds < 0 { '-' } else { '+' };
    let total_seconds = offset_seconds.abs();
    (
        sign,
        total_seconds / 3600,
        (total_seconds / 60) % 60,
        total_seconds % 60,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Display-data renames between the CLDR revision in vanilla's icu4j 77.1
    /// and the one baked into ICU4X 2.2: substring pairs as
    /// `(icu4j form, icu4x form)`.
    const CLDR_REVISION_RENAMES: &[(&str, &str)] = &[
        ("United Kingdom", "UK"),
        ("Vereinigtes Königreich", "UK"),
        ("Великобритания", "Британия"),
        ("Sao Paulo", "São Paulo"),
        ("Troll", "Troll Station"),
        ("Chatham", "Chatham Islands"),
        ("アメリカ太平洋標準時", "米国太平洋標準時"),
        ("ERA1", "AM"),
    ];

    /// Whole-value renames between the two CLDR revisions (short weekday and
    /// day-period forms whose tokens are too short for substring matching).
    const CLDR_REVISION_RENAMES_EXACT: &[(&str, &str)] = &[
        ("พ.", "พุธ"),
        ("หลังเที่ยง", "PM"),
        ("بعدازظهر", "ب.ظ."),
        ("PM", "오후"),
        ("p.\u{a0}m.", "p.\u{202f}m."),
        ("Th 4", "Thứ 4"),
        ("খ্রীষ্টাব্দ", "খ্রিস্টাব্দ"),
    ];

    /// Whether a mismatch is a known CLDR-revision data difference rather
    /// than a formatting divergence.
    fn cldr_revision_skew(
        pattern: &str,
        locale: &str,
        tz: &str,
        millis: i64,
        expected: &str,
        actual: &str,
    ) -> bool {
        // Newer CLDR added the Arabic letter mark around offset signs.
        if actual.replace('\u{61c}', "") == expected {
            return true;
        }
        if CLDR_REVISION_RENAMES_EXACT
            .iter()
            .any(|(icu4j, icu4x)| expected == *icu4j && actual == *icu4x)
        {
            return true;
        }
        let mut renamed = expected.to_string();
        for (icu4j, icu4x) in CLDR_REVISION_RENAMES {
            renamed = renamed.replace(icu4j, icu4x);
        }
        if renamed == actual {
            return true;
        }
        // Newer CLDR added generic zone names where icu4j 77.1 substituted
        // the metazone standard name (`vvvv` Asia/Seoul: "Korean Standard
        // Time" -> "Korean Time").
        pattern.chars().all(|ch| ch == 'v')
            && local_time_select_value(millis, "zzzz", locale, Some(tz)).as_deref()
                == Some(expected)
    }

    #[test]
    fn goldens_match_vanilla_icu4j_output() {
        let goldens = include_str!("local_time/goldens_26_1.tsv");
        let mut mismatches = Vec::new();
        let mut checked = 0usize;
        for line in goldens.lines() {
            let fields: Vec<&str> = line.split('\t').collect();
            let [pattern, locale, tz, millis, status, rest @ ..] = fields.as_slice() else {
                panic!("malformed goldens line: {line}");
            };
            let expected = rest.join("\t");
            let millis: i64 = millis.parse().unwrap();
            let actual = local_time_select_value(millis, pattern, locale, Some(tz));
            checked += 1;
            match *status {
                "OK" => {
                    let matches = match actual.as_deref() {
                        Some(actual) => {
                            actual == expected
                                || cldr_revision_skew(
                                    pattern, locale, tz, millis, &expected, actual,
                                )
                        }
                        None => false,
                    };
                    if !matches {
                        mismatches.push(format!(
                            "[{pattern}] {locale} {tz} {millis}: vanilla={expected:?} bbb={actual:?}"
                        ));
                    }
                }
                "ERROR" => {
                    if let Some(actual) = actual {
                        mismatches.push(format!(
                            "[{pattern}] {locale} {tz} {millis}: vanilla rejects, bbb={actual:?}"
                        ));
                    }
                }
                other => panic!("unknown goldens status {other} in: {line}"),
            }
        }
        assert!(
            checked > 8000,
            "goldens fixture unexpectedly small: {checked}"
        );
        assert!(
            mismatches.is_empty(),
            "{} of {} goldens diverge from vanilla ICU4J output; first 40:\n{}",
            mismatches.len(),
            checked,
            mismatches
                .iter()
                .take(600)
                .cloned()
                .collect::<Vec<_>>()
                .join("\n")
        );
    }

    #[test]
    fn local_time_english_week_data_locale_aliases() {
        let millis = |year, month, day| {
            Utc.with_ymd_and_hms(year, month, day, 0, 0, 0)
                .single()
                .unwrap()
                .timestamp_millis()
        };
        let formatted = |epoch_millis, locale| {
            local_time_select_value(epoch_millis, "yyyy-MM-dd Y w W e E", locale, Some("UTC"))
                .unwrap()
        };

        // en_CA follows the same Sunday-first/minimal-days=1 week data as
        // en_US: Sunday 2026-01-04 is week 2 and local weekday 1.
        assert_eq!(
            formatted(millis(2026, 1, 4), "en_CA"),
            "2026-01-04 2026 2 2 1 Sun"
        );

        // en_AU follows the same Monday-first/minimal-days=1 week data as en_GB
        // minus the minimal-days rule.
        assert_eq!(
            formatted(millis(2026, 1, 4), "en_AU"),
            "2026-01-04 2026 1 1 7 Sun"
        );

        // en_IE follows the same Monday-first/minimal-days=4 week data as en_GB:
        // 2027-01-01 belongs to week-year 2026 week 53, and ICU4J's
        // WEEK_OF_MONTH minimum is 0 — a January starting on Friday under
        // minimal-days=4 prints month week 0 (verified against icu4j 77.1).
        assert_eq!(
            formatted(millis(2027, 1, 1), "en_IE"),
            "2027-01-01 2026 53 0 5 Fri"
        );
    }

    #[test]
    fn local_time_julian_day_field_matches_known_gregorian_dates() {
        let julian = |year, month, day| {
            let millis = Utc
                .with_ymd_and_hms(year, month, day, 12, 0, 0)
                .single()
                .unwrap()
                .timestamp_millis();
            local_time_select_value(millis, "g", "en", Some("UTC")).unwrap()
        };
        assert_eq!(julian(1970, 1, 1), "2440588");
        assert_eq!(julian(2000, 1, 1), "2451545");
        assert_eq!(julian(2026, 12, 25), "2461400");
    }

    #[test]
    fn local_time_item_property_patterns_match_vanilla_icu_semantics() {
        let utc_ms = |y, m, d, hh, mm, ss, ms| {
            Utc.with_ymd_and_hms(y, m, d, hh, mm, ss)
                .single()
                .unwrap()
                .timestamp_millis()
                + ms
        };
        assert_eq!(
            local_time_select_value(utc_ms(2026, 12, 25, 0, 0, 0, 0), "MM-dd", "", Some("GMT"))
                .as_deref(),
            Some("12-25")
        );
        assert_eq!(
            local_time_select_value(
                utc_ms(2026, 12, 24, 21, 38, 7, 123),
                "yyyy-MM-dd'T'HH:mm:ss.SSS EEEE a",
                "",
                Some("GMT+02:30"),
            )
            .as_deref(),
            Some("2026-12-25T00:08:07.123 Fri AM")
        );
        assert_eq!(
            local_time_select_value(
                utc_ms(2026, 12, 24, 15, 8, 7, 123),
                "yyyy-MM-dd'T'HH:mm:ss.SSS XXX Z x",
                "",
                Some("Asia/Tokyo"),
            )
            .as_deref(),
            Some("2026-12-25T00:08:07.123 +09:00 +0900 +09")
        );
        assert_eq!(
            local_time_select_value(
                utc_ms(2026, 12, 25, 0, 8, 7, 0),
                "yyyy-MM-dd'T'HH:mm:ss X xx XXXXX xxxxx ZZZZZ",
                "",
                Some("UTC"),
            )
            .as_deref(),
            Some("2026-12-25T00:08:07 Z +0000 Z +00:00 Z")
        );
        assert_eq!(
            local_time_select_value(
                utc_ms(2026, 12, 25, 0, 0, 0, 0),
                "uuuu-DDD-G",
                "",
                Some("UTC"),
            )
            .as_deref(),
            Some("2026-359-CE")
        );
        assert_eq!(
            local_time_select_value(
                utc_ms(2026, 8, 9, 0, 0, 0, 0),
                "yyyy-Q-QQ-QQQ-QQQQ-QQQQQ-q-qq-qqq-qqqq-qqqqq",
                "",
                Some("UTC"),
            )
            .as_deref(),
            Some("2026-3-03-Q3-Q3-3-3-03-Q3-Q3-3")
        );
        assert_eq!(
            local_time_select_value(
                utc_ms(2026, 1, 1, 0, 0, 0, 0),
                "yyyy-MM-dd O OO OOO OOOO",
                "",
                Some("GMT+02:30"),
            )
            .as_deref(),
            Some("2026-01-01 GMT+2:30 null null GMT+02:30")
        );
        assert_eq!(
            local_time_select_value(
                utc_ms(2026, 1, 1, 1, 2, 3, 4),
                "HH:mm:ss.SSS A AA AAAAAAAA",
                "",
                Some("UTC"),
            )
            .as_deref(),
            Some("01:02:03.004 3723004 3723004 03723004")
        );
        assert_eq!(
            local_time_select_value(
                utc_ms(2027, 1, 1, 0, 0, 0, 0),
                "yyyy-MM-dd Y w ww W e c E",
                "en_GB",
                Some("UTC"),
            )
            .as_deref(),
            Some("2027-01-01 2026 53 53 0 5 5 Fri")
        );
    }
}

#[cfg(target_arch = "wasm32")]
mod wasm_intl {
    use chrono::{
        Datelike, Local, NaiveDate, NaiveDateTime, Offset as _, TimeZone as _, Timelike, Utc,
    };
    use js_sys::{Array, Date as JsDate, Function, Intl, Object, Reflect};
    use wasm_bindgen::{JsCast, JsValue};

    use super::{
        custom_gmt_id, icu4j_week_fields, iso8601_offset, parse_custom_gmt_offset, rfc822_offset,
        scan_pattern, Digits, Segment, WeekData,
    };

    struct WasmZoneData {
        offset_seconds: i32,
        vv_id: String,
        bcp47_id: String,
        intl_tz: Option<String>,
        has_zone_id: bool,
        local: NaiveDateTime,
        millisecond: u32,
    }

    pub(super) fn compute_local_time(
        epoch_millis: i64,
        pattern: &str,
        locale: &str,
        time_zone: Option<&str>,
    ) -> Option<String> {
        let bcp47_locale = ulocale_to_bcp47(locale);
        let zone = resolve_wasm_zone(epoch_millis, time_zone)?;
        let digits = discover_digits(&bcp47_locale);
        let week = week_data_from_bcp47(&bcp47_locale);
        let js_date = JsDate::new(&JsValue::from_f64(epoch_millis as f64));
        let segments = scan_pattern(pattern)?;

        let iso_year = zone.local.year();
        let day_of_year = zone.local.ordinal() as i32;
        let day_of_month = zone.local.day() as i32;
        let days_in_year = if is_leap_year(iso_year) { 366 } else { 365 };
        let days_in_prev_year = if is_leap_year(iso_year - 1) { 366 } else { 365 };
        let day_of_week = zone.local.weekday().number_from_sunday() as i32;
        let rata_die = NaiveDate::from_ymd_opt(iso_year, zone.local.month(), zone.local.day())?
            .num_days_from_ce() as i64;

        let mut out = String::new();
        for segment in &segments {
            match segment {
                Segment::Literal(lit) => out.push_str(lit),
                Segment::Field(ch, count) => {
                    let rendered = format_wasm_field(
                        *ch,
                        *count,
                        &bcp47_locale,
                        &zone,
                        &js_date,
                        &digits,
                        week,
                        iso_year,
                        day_of_year,
                        day_of_month,
                        days_in_year,
                        days_in_prev_year,
                        day_of_week,
                        rata_die,
                    )?;
                    out.push_str(&rendered);
                }
            }
        }
        Some(out)
    }

    const fn is_leap_year(year: i32) -> bool {
        (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0)
    }

    fn ulocale_to_bcp47(id: &str) -> String {
        let (base, keywords) = match id.split_once('@') {
            Some((b, kw)) => (b, Some(kw)),
            None => (id, None),
        };
        let normalized = base.trim().replace(['_', ' '], "-");
        let mut tag = if normalized.is_empty() || normalized.eq_ignore_ascii_case("root") {
            "en-US".to_string()
        } else {
            normalized
        };
        if let Some(keywords) = keywords {
            let mut ext = String::new();
            for item in keywords.split(';') {
                let Some((k, v)) = item.split_once('=') else {
                    continue;
                };
                let key = match k.trim().to_ascii_lowercase().as_str() {
                    "calendar" => "ca",
                    "numbers" => "nu",
                    _ => continue,
                };
                let val = match v.trim().to_ascii_lowercase().as_str() {
                    "gregorian" => "gregory".to_string(),
                    "ethiopic-amete-alem" => "ethioaa".to_string(),
                    other => other.to_string(),
                };
                if !val.is_empty() {
                    ext.push('-');
                    ext.push_str(key);
                    ext.push('-');
                    ext.push_str(&val);
                }
            }
            if !ext.is_empty() {
                tag.push_str("-u");
                tag.push_str(&ext);
            }
        }
        tag
    }

    fn week_data_from_bcp47(bcp47: &str) -> WeekData {
        let upper = bcp47.to_ascii_uppercase();
        if upper.contains("-US")
            || upper.contains("-CA")
            || upper.contains("-JP")
            || upper.contains("-BR")
        {
            WeekData {
                first_day: 1,
                minimal_days: 1,
            }
        } else if upper.contains("-GB")
            || upper.contains("-IE")
            || upper.contains("-DE")
            || upper.contains("-FR")
        {
            WeekData {
                first_day: 2,
                minimal_days: 4,
            }
        } else {
            WeekData {
                first_day: 2,
                minimal_days: 1,
            }
        }
    }

    fn construct_dtf(locale: &str, options: &Object) -> Option<Intl::DateTimeFormat> {
        let ctor: Function = Reflect::get(&js_sys::global(), &JsValue::from_str("Intl"))
            .ok()
            .and_then(|intl| Reflect::get(&intl, &JsValue::from_str("DateTimeFormat")).ok())
            .and_then(|val| val.dyn_into::<Function>().ok())?;
        let locales = Array::of1(&JsValue::from_str(locale));
        let args = Array::of2(&locales.into(), options);
        Reflect::construct(&ctor, &args)
            .or_else(|_| {
                let en_args = Array::of2(&Array::of1(&JsValue::from_str("en-US")).into(), options);
                Reflect::construct(&ctor, &en_args)
            })
            .ok()?
            .dyn_into::<Intl::DateTimeFormat>()
            .ok()
    }

    fn dtf_part(
        locale: &str,
        intl_tz: Option<&str>,
        js_date: &JsDate,
        prop: &str,
        style: &str,
        part_type: &str,
    ) -> Option<String> {
        let opts = Object::new();
        let _ = Reflect::set(&opts, &JsValue::from_str(prop), &JsValue::from_str(style));
        if let Some(tz) = intl_tz {
            let _ = Reflect::set(
                &opts,
                &JsValue::from_str("timeZone"),
                &JsValue::from_str(tz),
            );
        }
        let dtf = construct_dtf(locale, &opts)?;
        let parts = dtf.format_to_parts(js_date);
        for idx in 0..parts.length() {
            let item = parts.get(idx);
            let ty = Reflect::get(&item, &JsValue::from_str("type"))
                .ok()
                .and_then(|v| v.as_string());
            if ty.as_deref() == Some(part_type) {
                if let Some(val) = Reflect::get(&item, &JsValue::from_str("value"))
                    .ok()
                    .and_then(|v| v.as_string())
                {
                    return Some(val);
                }
            }
        }
        let formatted = dtf.format().call1(&dtf, js_date).ok()?.as_string()?;
        Some(formatted)
    }

    fn resolve_wasm_zone(epoch_millis: i64, time_zone: Option<&str>) -> Option<WasmZoneData> {
        let utc = Utc.timestamp_millis_opt(epoch_millis).single()?;
        match time_zone {
            None => {
                let zoned = Local.timestamp_millis_opt(epoch_millis).single()?;
                Some(WasmZoneData {
                    offset_seconds: zoned.offset().fix().local_minus_utc(),
                    vv_id: String::new(),
                    bcp47_id: "unk".to_string(),
                    intl_tz: None,
                    has_zone_id: false,
                    local: zoned.naive_local(),
                    millisecond: zoned.timestamp_subsec_millis(),
                })
            }
            Some(id) => {
                if let Some(offset) = parse_custom_gmt_offset(id) {
                    let zoned = utc.with_timezone(&offset);
                    let secs = offset.local_minus_utc();
                    return Some(WasmZoneData {
                        offset_seconds: secs,
                        vv_id: custom_gmt_id(secs),
                        bcp47_id: "unk".to_string(),
                        intl_tz: None,
                        has_zone_id: true,
                        local: zoned.naive_local(),
                        millisecond: zoned.timestamp_subsec_millis(),
                    });
                }
                if id.is_empty()
                    || id.eq_ignore_ascii_case("z")
                    || id.starts_with('+')
                    || id.starts_with('-')
                    || (id.len() > 3
                        && id.is_char_boundary(3)
                        && id[..3].eq_ignore_ascii_case("utc")
                        && matches!(id.as_bytes()[3], b'+' | b'-'))
                {
                    return None;
                }
                let opts = Object::new();
                let _ = Reflect::set(
                    &opts,
                    &JsValue::from_str("timeZone"),
                    &JsValue::from_str(id),
                );
                let _ = Reflect::set(
                    &opts,
                    &JsValue::from_str("year"),
                    &JsValue::from_str("numeric"),
                );
                let _ = Reflect::set(
                    &opts,
                    &JsValue::from_str("month"),
                    &JsValue::from_str("2-digit"),
                );
                let _ = Reflect::set(
                    &opts,
                    &JsValue::from_str("day"),
                    &JsValue::from_str("2-digit"),
                );
                let _ = Reflect::set(
                    &opts,
                    &JsValue::from_str("hour"),
                    &JsValue::from_str("2-digit"),
                );
                let _ = Reflect::set(
                    &opts,
                    &JsValue::from_str("minute"),
                    &JsValue::from_str("2-digit"),
                );
                let _ = Reflect::set(
                    &opts,
                    &JsValue::from_str("second"),
                    &JsValue::from_str("2-digit"),
                );
                let _ = Reflect::set(
                    &opts,
                    &JsValue::from_str("hourCycle"),
                    &JsValue::from_str("h23"),
                );
                let dtf = construct_dtf("en-US", &opts)?;
                let js_date = JsDate::new(&JsValue::from_f64(epoch_millis as f64));
                let parts = dtf.format_to_parts(&js_date);
                let mut year = None;
                let mut month = None;
                let mut day = None;
                let mut hour = None;
                let mut minute = None;
                let mut second = None;
                let mut era = None;
                for idx in 0..parts.length() {
                    let item = parts.get(idx);
                    let Some(ty) = Reflect::get(&item, &JsValue::from_str("type"))
                        .ok()
                        .and_then(|v| v.as_string())
                    else {
                        continue;
                    };
                    let Some(val) = Reflect::get(&item, &JsValue::from_str("value"))
                        .ok()
                        .and_then(|v| v.as_string())
                    else {
                        continue;
                    };
                    match ty.as_str() {
                        "year" => year = val.parse::<i32>().ok(),
                        "month" => month = val.parse::<u32>().ok(),
                        "day" => day = val.parse::<u32>().ok(),
                        "hour" => hour = val.parse::<u32>().ok().map(|h| h % 24),
                        "minute" => minute = val.parse::<u32>().ok(),
                        "second" => second = val.parse::<u32>().ok(),
                        "era" => era = Some(val),
                        _ => {}
                    }
                }
                let mut iso_year = year?;
                if matches!(era.as_deref(), Some("BC" | "BCE")) {
                    iso_year = 1 - iso_year;
                }
                let local = NaiveDate::from_ymd_opt(iso_year, month?, day?)?
                    .and_hms_opt(hour?, minute?, second?)?;
                let offset_seconds = (local.and_utc().timestamp() - utc.timestamp()) as i32;
                Some(WasmZoneData {
                    offset_seconds,
                    vv_id: id.to_string(),
                    bcp47_id: id.to_ascii_lowercase(),
                    intl_tz: Some(id.to_string()),
                    has_zone_id: true,
                    local,
                    millisecond: utc.timestamp_subsec_millis(),
                })
            }
        }
    }

    fn discover_digits(locale: &str) -> Digits {
        let mut digits = ['0', '1', '2', '3', '4', '5', '6', '7', '8', '9'];
        let Some(ctor) = Reflect::get(&js_sys::global(), &JsValue::from_str("Intl"))
            .ok()
            .and_then(|intl| Reflect::get(&intl, &JsValue::from_str("NumberFormat")).ok())
            .and_then(|val| val.dyn_into::<Function>().ok())
        else {
            return Digits(digits);
        };
        let opts = Object::new();
        let _ = Reflect::set(&opts, &JsValue::from_str("useGrouping"), &JsValue::FALSE);
        let args = Array::of2(&Array::of1(&JsValue::from_str(locale)).into(), &opts);
        let Ok(nf_val) = Reflect::construct(&ctor, &args) else {
            return Digits(digits);
        };
        let Ok(nf) = nf_val.dyn_into::<Intl::NumberFormat>() else {
            return Digits(digits);
        };
        let fmt_fn = nf.format();
        for (idx, slot) in digits.iter_mut().enumerate() {
            if let Some(s) = fmt_fn
                .call1(&nf, &JsValue::from_f64(idx as f64))
                .ok()
                .and_then(|v| v.as_string())
            {
                if let Some(ch) = s.chars().next() {
                    *slot = ch;
                }
            }
        }
        Digits(digits)
    }

    #[allow(clippy::too_many_arguments)]
    fn format_wasm_field(
        ch: char,
        count: usize,
        locale: &str,
        zone: &WasmZoneData,
        js_date: &JsDate,
        digits: &Digits,
        week: WeekData,
        iso_year: i32,
        day_of_year: i32,
        day_of_month: i32,
        days_in_year: i32,
        days_in_prev_year: i32,
        day_of_week: i32,
        rata_die: i64,
    ) -> Option<String> {
        let tz = zone.intl_tz.as_deref();
        match ch {
            'G' => {
                let style = match count {
                    4 => "long",
                    5 => "narrow",
                    _ => "short",
                };
                dtf_part(locale, tz, js_date, "era", style, "era")
            }
            'M' | 'L' => match count {
                3 => dtf_part(locale, tz, js_date, "month", "short", "month"),
                4 => dtf_part(locale, tz, js_date, "month", "long", "month"),
                5 => dtf_part(locale, tz, js_date, "month", "narrow", "month"),
                _ => Some(digits.pad(zone.local.month() as i32, count)),
            },
            'E' => {
                let style = match count {
                    4 => "long",
                    5 => "narrow",
                    6 => "short",
                    _ => "short",
                };
                dtf_part(locale, tz, js_date, "weekday", style, "weekday")
            }
            'e' | 'c' => {
                if count <= 2 {
                    let wf = icu4j_week_fields(
                        day_of_year,
                        day_of_month,
                        day_of_week,
                        days_in_year,
                        days_in_prev_year,
                        week,
                    );
                    let width = if ch == 'e' { count } else { 1 };
                    Some(digits.pad(wf.local_day_of_week, width))
                } else {
                    let style = match count {
                        4 => "long",
                        5 => "narrow",
                        _ => "short",
                    };
                    dtf_part(locale, tz, js_date, "weekday", style, "weekday")
                }
            }
            'a' | 'b' | 'B' => {
                if ch == 'a'
                    || (ch == 'b'
                        && zone.local.hour() == 0
                        && zone.local.minute() == 0
                        && zone.local.second() == 0)
                {
                    let opts = Object::new();
                    let _ = Reflect::set(
                        &opts,
                        &JsValue::from_str("hour"),
                        &JsValue::from_str("numeric"),
                    );
                    let _ = Reflect::set(&opts, &JsValue::from_str("hour12"), &JsValue::TRUE);
                    if let Some(id) = tz {
                        let _ = Reflect::set(
                            &opts,
                            &JsValue::from_str("timeZone"),
                            &JsValue::from_str(id),
                        );
                    }
                    let dtf = construct_dtf(locale, &opts)?;
                    let parts = dtf.format_to_parts(js_date);
                    for idx in 0..parts.length() {
                        let item = parts.get(idx);
                        if Reflect::get(&item, &JsValue::from_str("type"))
                            .ok()
                            .and_then(|v| v.as_string())
                            .as_deref()
                            == Some("dayPeriod")
                        {
                            if let Some(val) = Reflect::get(&item, &JsValue::from_str("value"))
                                .ok()
                                .and_then(|v| v.as_string())
                            {
                                return Some(val);
                            }
                        }
                    }
                    return Some(if zone.local.hour() < 12 { "AM" } else { "PM" }.to_string());
                }
                let style = match count {
                    4 => "long",
                    5.. => "narrow",
                    _ => "short",
                };
                dtf_part(locale, tz, js_date, "dayPeriod", style, "dayPeriod")
                    .or_else(|| Some(if zone.local.hour() < 12 { "AM" } else { "PM" }.to_string()))
            }
            'y' => Some(digits.year(iso_year.abs().max(1), count)),
            'u' | 'r' => Some(digits.pad(iso_year, count)),
            'Y' => {
                let wf = icu4j_week_fields(
                    day_of_year,
                    day_of_month,
                    day_of_week,
                    days_in_year,
                    days_in_prev_year,
                    week,
                );
                Some(digits.year(iso_year + wf.week_year_delta, count))
            }
            'w' => {
                let wf = icu4j_week_fields(
                    day_of_year,
                    day_of_month,
                    day_of_week,
                    days_in_year,
                    days_in_prev_year,
                    week,
                );
                Some(digits.pad(wf.week_of_year, count))
            }
            'W' => {
                let wf = icu4j_week_fields(
                    day_of_year,
                    day_of_month,
                    day_of_week,
                    days_in_year,
                    days_in_prev_year,
                    week,
                );
                Some(digits.pad(wf.week_of_month, count))
            }
            'd' => Some(digits.pad(day_of_month, count)),
            'D' => Some(digits.pad(day_of_year, count)),
            'F' => Some(digits.pad((day_of_month - 1) / 7 + 1, count)),
            'g' => Some(digits.pad_i64(rata_die + 1_721_425, count)),
            'H' => Some(digits.pad(zone.local.hour() as i32, count)),
            'k' => {
                let hour = if zone.local.hour() == 0 {
                    24
                } else {
                    zone.local.hour() as i32
                };
                Some(digits.pad(hour, count))
            }
            'K' => Some(digits.pad((zone.local.hour() % 12) as i32, count)),
            'h' => {
                let hour = zone.local.hour() % 12;
                let hour = if hour == 0 { 12 } else { hour as i32 };
                Some(digits.pad(hour, count))
            }
            'm' => Some(digits.pad(zone.local.minute() as i32, count)),
            's' => Some(digits.pad(zone.local.second() as i32, count)),
            'S' => Some(digits.fraction(zone.millisecond, count)),
            'A' => {
                let millis = (((zone.local.hour() * 60 + zone.local.minute()) * 60
                    + zone.local.second())
                    * 1_000)
                    + zone.millisecond;
                Some(digits.pad_i64(i64::from(millis), count))
            }
            'Q' | 'q' => {
                let quarter = ((zone.local.month() as i32) - 1) / 3 + 1;
                match count {
                    1..=2 => Some(digits.pad(quarter, count)),
                    5.. => Some(digits.pad(quarter, 1)),
                    _ => Some(format!("Q{quarter}")),
                }
            }
            'z' => {
                if !zone.has_zone_id {
                    return None;
                }
                let style = if count <= 3 { "short" } else { "long" };
                if let Some(id) = tz {
                    dtf_part(
                        locale,
                        Some(id),
                        js_date,
                        "timeZoneName",
                        style,
                        "timeZoneName",
                    )
                } else {
                    Some(format_gmt_offset(zone.offset_seconds, count > 3))
                }
            }
            'Z' => match count {
                1..=3 => Some(rfc822_offset(zone.offset_seconds)),
                5 => iso8601_offset(zone.offset_seconds, 5, true),
                _ => {
                    if !zone.has_zone_id && zone.offset_seconds != 0 {
                        return None;
                    }
                    Some(format_gmt_offset(zone.offset_seconds, true))
                }
            },
            'O' => match count {
                1 => {
                    if !zone.has_zone_id && zone.offset_seconds != 0 {
                        return None;
                    }
                    Some(format_gmt_offset(zone.offset_seconds, false))
                }
                4 => {
                    if !zone.has_zone_id && zone.offset_seconds != 0 {
                        return None;
                    }
                    Some(format_gmt_offset(zone.offset_seconds, true))
                }
                _ => Some("null".to_string()),
            },
            'v' => match count {
                1 | 4 => {
                    if !zone.has_zone_id {
                        return None;
                    }
                    let style = if count == 1 {
                        "shortGeneric"
                    } else {
                        "longGeneric"
                    };
                    if let Some(id) = tz {
                        dtf_part(
                            locale,
                            Some(id),
                            js_date,
                            "timeZoneName",
                            style,
                            "timeZoneName",
                        )
                        .or_else(|| Some(format_gmt_offset(zone.offset_seconds, count == 4)))
                    } else {
                        Some(format_gmt_offset(zone.offset_seconds, count == 4))
                    }
                }
                _ => Some("null".to_string()),
            },
            'V' => match count {
                1 => Some(zone.bcp47_id.clone()),
                2 => Some(zone.vv_id.clone()),
                3 => {
                    if !zone.has_zone_id {
                        return None;
                    }
                    Some(
                        zone.vv_id
                            .rsplit('/')
                            .next()
                            .unwrap_or("Unknown City")
                            .replace('_', " "),
                    )
                }
                4 => {
                    if !zone.has_zone_id {
                        return None;
                    }
                    Some(format_gmt_offset(zone.offset_seconds, true))
                }
                _ => Some("null".to_string()),
            },
            'x' => match count {
                1..=5 => iso8601_offset(zone.offset_seconds, count, false),
                _ => Some("null".to_string()),
            },
            'X' => match count {
                1..=5 => iso8601_offset(zone.offset_seconds, count, true),
                _ => Some("null".to_string()),
            },
            'l' => Some(String::new()),
            _ => None,
        }
    }

    fn format_gmt_offset(offset_seconds: i32, padded: bool) -> String {
        if offset_seconds == 0 {
            return "GMT".to_string();
        }
        let sign = if offset_seconds < 0 { '-' } else { '+' };
        let total = offset_seconds.abs();
        let h = total / 3600;
        let m = (total / 60) % 60;
        let s = total % 60;
        if s != 0 {
            if padded {
                format!("GMT{sign}{h:02}:{m:02}:{s:02}")
            } else {
                format!("GMT{sign}{h}:{m:02}:{s:02}")
            }
        } else if padded {
            format!("GMT{sign}{h:02}:{m:02}")
        } else if m == 0 {
            format!("GMT{sign}{h}")
        } else {
            format!("GMT{sign}{h}:{m:02}")
        }
    }
}
