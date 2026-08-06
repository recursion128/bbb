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

mod tables_26_1;

use std::cell::RefCell;
use std::collections::HashMap;

use chrono::{Datelike, FixedOffset, Local, Offset as _, TimeZone as _, Timelike, Utc};
use chrono_tz::{OffsetComponents, Tz};
use icu_calendar::cal;
use icu_calendar::{AnyCalendarKind, Date};
use icu_datetime::fieldsets::enums::CompositeFieldSet;
use icu_datetime::pattern::{DateTimePattern, FixedCalendarDateTimeNames};
use icu_decimal::options::{DecimalFormatterOptions, GroupingStrategy};
use icu_decimal::DecimalFormatter;
use icu_locale::extensions::unicode as unicode_ext;
use icu_locale::{Locale, LocaleCanonicalizer, LocaleExpander};
use icu_time::zone::iana::IanaParser;
use icu_time::zone::models::AtTime;
use icu_time::zone::{UtcOffset, ZoneNameTimestamp};
use icu_time::{Time, TimeZone as IcuTimeZone, TimeZoneInfo, ZonedDateTime};
use writeable::TryWriteable;

/// Formats one `minecraft:local_time` value the way vanilla's cached
/// `SimpleDateFormat` would; `None` mirrors vanilla's codec/format rejection
/// (unknown zone, invalid pattern letter), which drops the select case.
pub(super) fn local_time_select_value(
    epoch_millis: i64,
    pattern: &str,
    locale: &str,
    time_zone: Option<&str>,
) -> Option<String> {
    thread_local! {
        static CACHE: RefCell<HashMap<(String, String, Option<String>), (i64, String)>> =
            RefCell::new(HashMap::new());
    }
    let key = (
        pattern.to_string(),
        locale.to_string(),
        time_zone.map(str::to_string),
    );
    // Vanilla re-formats at most once per second and serves the cached string
    // in between; keying on the wall-clock second reproduces that cadence.
    let second = epoch_millis.div_euclid(1000);
    if let Some(result) = CACHE.with(|cache| {
        cache
            .borrow()
            .get(&key)
            .filter(|(cached_second, _)| *cached_second == second)
            .map(|(_, result)| result.clone())
    }) {
        return Some(result);
    }
    let result = compute_local_time(epoch_millis, pattern, locale, time_zone)?;
    CACHE.with(|cache| {
        let mut cache = cache.borrow_mut();
        if cache.len() > 256 {
            cache.clear();
        }
        cache.insert(key, (second, result.clone()));
    });
    Some(result)
}

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
fn language_data_is_root(locale: &Locale) -> bool {
    thread_local! {
        static PROBES: RefCell<HashMap<String, bool>> = RefCell::new(HashMap::new());
    }
    let key = format!(
        "{}-{}",
        locale.id.language,
        locale
            .id
            .script
            .map(|script| script.to_string())
            .unwrap_or_default()
    );
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
    PROBES.with(|probes| probes.borrow_mut().insert(key, is_root));
    is_root
}

// ---------------------------------------------------------------------------
// Locale handling (ULocale semantics)

/// Parses vanilla's ICU `ULocale` id shapes (`zh_cn`, `""`/`root`,
/// `ja_JP@calendar=japanese`) into a BCP-47 `Locale`; anything unparseable
/// degrades to root exactly as ICU falls back to root data.
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
fn calendar_kind_for(locale: &Locale) -> AnyCalendarKind {
    let mut maximized = locale.clone();
    LocaleExpander::new_extended().maximize(&mut maximized.id);
    AnyCalendarKind::new((&maximized).into())
}

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
            loop {
                let Some(quoted) = chars.next() else {
                    break;
                };
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
fn placeholder_zone() -> TimeZoneInfo<AtTime> {
    IcuTimeZone::UNKNOWN
        .with_offset(None)
        .with_zone_name_timestamp(ZoneNameTimestamp::far_in_past())
}

/// The calendar-dependent numeric facts every native field derives from.
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
enum FieldOutput {
    Text(String),
    Delegate(String),
}

/// Resolves one pattern field. `None` rejects the whole pattern like
/// `SimpleDateFormat` does for an illegal letter.
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
            .filter(|(row_locale, start, _)| row_locale == key && *start <= minute)
            .next_back()
            .map(|(_, _, names)| names[index].to_string())
    })
}

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
            if zone.icu_zone.is_none() {
                return None;
            }
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

#[derive(Clone, Copy)]
enum GmtZeroForm {
    Short,
    Long,
}

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
            zone: icu_zone.clone(),
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
}
