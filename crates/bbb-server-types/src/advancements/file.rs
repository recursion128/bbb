use std::collections::BTreeMap;
use std::path::PathBuf;

use bbb_registry_codec::{decode_loaded_registry_json, RegistryJsonValue};
use thiserror::Error;
use uuid::Uuid;

use crate::save::{WorldSavePaths, JAVA_26_1_DATA_VERSION};

use super::progress::{AdvancementProgress, CriterionProgress};

/// `wrapCodec`'s decode default (`DataFixTypes.java:71`), the 1.12.2 data
/// version every advancement document older than the field carries.
const LEGACY_DATA_VERSION: i64 = 1343;

/// `PlayerAdvancements.Data` (`PlayerAdvancements.java:318-325`): the whole
/// `players/advancements/<uuid>.json` document as a map from advancement id to
/// the progress it holds.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Java26PlayerAdvancements {
    pub progress: BTreeMap<String, AdvancementProgress>,
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum PlayerAdvancementsParseError {
    #[error("advancement progress file is not a JSON object")]
    NotAnObject,
    #[error(
        "advancement progress file DataVersion {0} is not the 26.1 data version {JAVA_26_1_DATA_VERSION}"
    )]
    DataVersion(i64),
}

impl Java26PlayerAdvancements {
    /// The document `save` (`:130-142`) writes: the `Data` map plus the
    /// `DataVersion` `wrapCodec` merges into it (`DataFixTypes.java:64-66`).
    pub fn to_document(&self) -> RegistryJsonValue {
        let mut root = Vec::with_capacity(self.progress.len() + 1);
        root.push((
            "DataVersion".into(),
            RegistryJsonValue::Number(JAVA_26_1_DATA_VERSION.to_string()),
        ));
        for (id, progress) in &self.progress {
            let mut criteria = Vec::new();
            for (name, criterion) in progress.criteria() {
                // The criteria codec writes obtained criteria only (`AdvancementProgress.java:32-35`).
                if let Some(millis) = criterion.obtained_epoch_millis() {
                    criteria.push((
                        name.clone(),
                        RegistryJsonValue::String(format_obtained_time(millis).into()),
                    ));
                }
            }
            let mut entry = Vec::new();
            if !criteria.is_empty() {
                entry.push(("criteria".into(), RegistryJsonValue::Object(criteria)));
            }
            entry.push(("done".into(), RegistryJsonValue::Bool(progress.is_done())));
            root.push((id.clone().into(), RegistryJsonValue::Object(entry)));
        }
        RegistryJsonValue::Object(root)
    }

    /// UTF-8 JSON that retains each criterion's original UTF-16 units.
    pub fn to_file_bytes(&self) -> Vec<u8> {
        serde_json::to_vec(&self.to_document())
            .expect("an advancement progress document is always serializable")
    }

    /// `load` (`:113-128`) for 26.1 documents.
    pub fn parse(document: &RegistryJsonValue) -> Result<Self, PlayerAdvancementsParseError> {
        let RegistryJsonValue::Object(root) = document else {
            return Err(PlayerAdvancementsParseError::NotAnObject);
        };
        let data_version = match document.get("DataVersion") {
            Some(value) => value
                .as_i64()
                .ok_or(PlayerAdvancementsParseError::DataVersion(i64::from(
                    i32::MIN,
                )))?,
            None => LEGACY_DATA_VERSION,
        };
        if data_version != i64::from(JAVA_26_1_DATA_VERSION) {
            return Err(PlayerAdvancementsParseError::DataVersion(data_version));
        }
        let mut progress = BTreeMap::new();
        for (raw_id, value) in root {
            if raw_id == "DataVersion" {
                continue;
            }
            let Some(id) = raw_id
                .try_to_string()
                .ok()
                .and_then(|id| bbb_protocol::normalize_resource_location(id).ok())
            else {
                continue;
            };
            if value.as_object().is_none() {
                continue;
            }
            let criteria = value
                .get("criteria")
                .and_then(RegistryJsonValue::as_object)
                .into_iter()
                .flatten()
                .filter_map(|(name, obtained)| {
                    let millis = parse_obtained_time(&obtained.utf8_string()?)?;
                    Some((name.clone(), CriterionProgress::obtained(millis)))
                })
                .collect();
            progress.insert(id, AdvancementProgress::from_criteria(criteria));
        }
        Ok(Self { progress })
    }

    pub fn parse_bytes(bytes: &[u8]) -> Result<Self, PlayerAdvancementsParseError> {
        let document = decode_loaded_registry_json(bytes, bytes.len())
            .map_err(|_| PlayerAdvancementsParseError::NotAnObject)?;
        Self::parse(
            document
                .json()
                .ok_or(PlayerAdvancementsParseError::NotAnObject)?,
        )
    }
}

/// Vanilla `LevelResource.PLAYER_ADVANCEMENTS_DIR` (`LevelResource.java:4`):
/// `<world>/players/advancements`, beside the playerdata and the statistics.
pub fn player_advancements_dir(paths: &WorldSavePaths) -> PathBuf {
    paths.root().join("players").join("advancements")
}

/// One session's document, which `PlayerList.getPlayerAdvancements` names after
/// the profile uuid (`PlayerList.java:792`).
pub fn player_advancements_file(paths: &WorldSavePaths, uuid: Uuid) -> PathBuf {
    player_advancements_dir(paths).join(format!("{}.json", uuid.hyphenated()))
}

const SECONDS_PER_DAY: i64 = 86_400;

/// `AdvancementProgress.OBTAINED_TIME_FORMAT` (`AdvancementProgress.java:26-28`):
/// `yyyy-MM-dd HH:mm:ss Z` in UTC.
pub fn format_obtained_time(epoch_millis: i64) -> String {
    let seconds = epoch_millis.div_euclid(1_000);
    let (year, month, day) = civil_from_days(seconds.div_euclid(SECONDS_PER_DAY));
    let second_of_day = seconds.rem_euclid(SECONDS_PER_DAY);
    format!(
        "{year:04}-{month:02}-{day:02} {:02}:{:02}:{:02} +0000",
        second_of_day / 3_600,
        second_of_day / 60 % 60,
        second_of_day % 60,
    )
}

/// The parse half of the same formatter.
pub fn parse_obtained_time(raw: &str) -> Option<i64> {
    let (date, rest) = raw.split_once(' ')?;
    let (time, offset) = rest.split_once(' ')?;
    let mut date = date.splitn(3, '-');
    let year: i64 = date.next()?.parse().ok()?;
    let month: u32 = parse_fixed(date.next()?, 2)?;
    let day: u32 = parse_fixed(date.next()?, 2)?;
    let mut time = time.splitn(3, ':');
    let hour: i64 = i64::from(parse_fixed::<u32>(time.next()?, 2)?);
    let minute: i64 = i64::from(parse_fixed::<u32>(time.next()?, 2)?);
    let second: i64 = i64::from(parse_fixed::<u32>(time.next()?, 2)?);
    if !(1..=12).contains(&month)
        || !(1..=31).contains(&day)
        || hour > 23
        || minute > 59
        || second > 60
    {
        return None;
    }
    let (sign, offset): (i64, &str) = match offset.split_at_checked(1)? {
        ("+", rest) => (1, rest),
        ("-", rest) => (-1, rest),
        _ => return None,
    };
    if offset.len() != 4 {
        return None;
    }
    let offset_hours: i64 = i64::from(parse_fixed::<u32>(&offset[..2], 2)?);
    let offset_minutes: i64 = i64::from(parse_fixed::<u32>(&offset[2..], 2)?);
    let day_seconds = days_from_civil(year, month, day)?.checked_mul(SECONDS_PER_DAY)?;
    let offset_seconds = sign.checked_mul(offset_hours * 3_600 + offset_minutes * 60)?;
    let seconds = day_seconds
        .checked_add(hour * 3_600 + minute * 60 + second)?
        .checked_sub(offset_seconds)?;
    seconds.checked_mul(1_000)
}

fn parse_fixed<T: std::str::FromStr>(raw: &str, width: usize) -> Option<T> {
    (raw.len() == width && raw.bytes().all(|byte| byte.is_ascii_digit()))
        .then(|| raw.parse().ok())
        .flatten()
}

fn civil_from_days(days: i64) -> (i64, u32, u32) {
    let shifted = days + 719_468;
    let era = shifted.div_euclid(146_097);
    let day_of_era = shifted.rem_euclid(146_097);
    let year_of_era =
        (day_of_era - day_of_era / 1_460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_position = (5 * day_of_year + 2) / 153;
    let day = (day_of_year - (153 * month_position + 2) / 5 + 1) as u32;
    let month = if month_position < 10 {
        month_position + 3
    } else {
        month_position - 9
    } as u32;
    let year = year_of_era + era * 400 + i64::from(month <= 2);
    (year, month, day)
}

fn days_from_civil(year: i64, month: u32, day: u32) -> Option<i64> {
    let year = year.checked_sub(i64::from(month <= 2))?;
    let era = year.div_euclid(400);
    let year_of_era = year.rem_euclid(400);
    let month_position = i64::from(if month > 2 { month - 3 } else { month + 9 });
    let day_of_year = (153 * month_position + 2) / 5 + i64::from(day) - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    era.checked_mul(146_097)?
        .checked_add(day_of_era)?
        .checked_sub(719_468)
}

#[cfg(test)]
mod timestamp_overflow_tests {
    use super::*;

    #[test]
    fn parse_obtained_time_rejects_overflowing_years_without_panicking() {
        assert!(parse_obtained_time("999999999999999999-01-01 00:00:00 +0000").is_none());
        assert!(parse_obtained_time("-999999999999999999-01-01 00:00:00 +0000").is_none());
        assert!(parse_obtained_time("2026-04-01 12:34:56 +0000").is_some());
    }
}
