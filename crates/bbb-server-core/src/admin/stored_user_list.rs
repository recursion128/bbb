//! The four stored user lists and their on-disk JSON documents.
//!
//! Vanilla owners: `StoredUserList` (StoredUserList.java:27-152) holds the map
//! and writes a JSON array of entries; `UserWhiteListEntry` (:5-20),
//! `ServerOpListEntry` (:7-40), `UserBanListEntry` (:8-38) and `IpBanListEntry`
//! (:8-38) each contribute their own fields, with the shared ban fields coming
//! from `BanListEntry` (:12-107). `NameAndId.appendTo` (NameAndId.java:42-44)
//! writes the `uuid`/`name` pair every user-keyed entry starts with. All three
//! user-keyed lists override `getKeyForUser` to the id alone
//! (`UserWhiteList.java:57-59`, `ServerOpList.java:64-66`,
//! `UserBanList.java:27-29`), so a name change never forks an entry.
//!
//! This module owns the documents as bytes only. `ServerWorld` never touches
//! the filesystem — the runtime reads the four files at startup and writes them
//! back atomically whenever the list revision changes, the same split the world
//! save already uses for wall-clock time.

use std::collections::BTreeMap;

use bbb_protocol::JavaString;
use serde_json::value::RawValue;
use serde_json::{Map, Value};
use uuid::Uuid;

use crate::commands::CommandPermission;

use super::identity::NameAndId;

/// The `PlayerList` file names `DedicatedServer` constructs its four
/// `StoredUserList`s with.
pub const ALLOWLIST_FILE: &str = "whitelist.json";
pub const OPERATORS_FILE: &str = "ops.json";
pub const PLAYER_BANS_FILE: &str = "banned-players.json";
pub const IP_BANS_FILE: &str = "banned-ips.json";

/// `BanListEntry.EXPIRES_NEVER` (BanListEntry.java:14).
const EXPIRES_NEVER: &str = "forever";
/// `BanListEntry`'s default source when the document carries none
/// (BanListEntry.java:27).
pub const UNKNOWN_SOURCE: &str = "(Unknown)";
/// The bound this server puts on every free-text management field. Vanilla
/// stores reasons and sources unbounded because only a local operator writes
/// them; a network management surface is an untrusted input boundary, so the
/// fields are bounded before they can be persisted.
pub const MAX_ADMIN_TEXT_CHARS: usize = 256;
/// The bound on how many entries one list may hold. Vanilla has none; a bounded
/// mailbox that can grow an unbounded on-disk document is not bounded.
pub const MAX_LIST_ENTRIES: usize = 4096;

/// Why a stored user list document was rejected.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoredUserListError {
    pub file: &'static str,
    pub detail: String,
}

impl std::fmt::Display for StoredUserListError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}: {}", self.file, self.detail)
    }
}

fn invalid(file: &'static str, detail: impl Into<String>) -> StoredUserListError {
    StoredUserListError {
        file,
        detail: detail.into(),
    }
}

/// `BanListEntry`'s shared fields (BanListEntry.java:15-18).
///
/// Instants are epoch milliseconds: the world crate owns no wall clock, so the
/// current instant arrives with the request that creates the entry and the
/// document format is applied only at this boundary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BanRecord {
    pub created: i64,
    pub source: JavaString,
    pub expires: Option<i64>,
    pub reason: Option<JavaString>,
}

impl BanRecord {
    /// `BanListEntry.hasExpired` (BanListEntry.java:83-85).
    pub fn has_expired(&self, now: i64) -> bool {
        self.expires.is_some_and(|expires| expires < now)
    }
}

/// `ServerOpListEntry` (ServerOpListEntry.java:7-40).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OperatorEntry {
    pub user: NameAndId,
    pub level: CommandPermission,
    pub bypasses_player_limit: bool,
}

/// `UserBanListEntry` (UserBanListEntry.java:8-38).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlayerBanEntry {
    pub user: NameAndId,
    pub ban: BanRecord,
}

/// The four lists as one owned value, each keyed the way vanilla keys it.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct StoredUserLists {
    pub allowlist: BTreeMap<Uuid, NameAndId>,
    pub operators: BTreeMap<Uuid, OperatorEntry>,
    pub player_bans: BTreeMap<Uuid, PlayerBanEntry>,
    pub ip_bans: BTreeMap<String, BanRecord>,
}

/// The four documents, in the order the runtime writes them.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct StoredUserListDocuments {
    pub allowlist: Vec<u8>,
    pub operators: Vec<u8>,
    pub player_bans: Vec<u8>,
    pub ip_bans: Vec<u8>,
}

impl StoredUserListDocuments {
    /// Each document with the vanilla file name it belongs in.
    pub fn files(&self) -> [(&'static str, &[u8]); 4] {
        [
            (ALLOWLIST_FILE, &self.allowlist),
            (OPERATORS_FILE, &self.operators),
            (PLAYER_BANS_FILE, &self.player_bans),
            (IP_BANS_FILE, &self.ip_bans),
        ]
    }
}

impl StoredUserLists {
    /// `StoredUserList.save` for all four lists (StoredUserList.java:127-135).
    pub fn documents(&self) -> StoredUserListDocuments {
        StoredUserListDocuments {
            allowlist: encode(self.allowlist.values().map(|user| {
                serde_json::value::to_raw_value(&user_object(user, Map::new()))
                    .expect("user fields serialize")
            })),
            operators: encode(self.operators.values().map(|entry| {
                let mut object = Map::new();
                object.insert("level".to_string(), Value::from(entry.level.id()));
                object.insert(
                    "bypassesPlayerLimit".to_string(),
                    Value::Bool(entry.bypasses_player_limit),
                );
                serde_json::value::to_raw_value(&user_object(&entry.user, object))
                    .expect("operator fields serialize")
            })),
            player_bans: encode(self.player_bans.values().map(|entry| {
                ban_document(user_object(&entry.user, ban_object(&entry.ban)), &entry.ban)
            })),
            ip_bans: encode(self.ip_bans.iter().map(|(ip, record)| {
                let mut object = ban_object(record);
                object.insert("ip".to_string(), Value::String(ip.clone()));
                ban_document(raw_fields(object), record)
            })),
        }
    }

    /// `StoredUserList.load` for all four lists (StoredUserList.java:137-152),
    /// with a missing file read as the empty list the constructor starts with.
    ///
    /// Vanilla swallows a malformed date and keeps the entry; this server fails
    /// the load instead, because a management document that silently loses a
    /// ban expiry is the same class of failure the save pipeline already
    /// refuses to accept.
    pub fn parse(documents: &StoredUserListDocuments) -> Result<Self, StoredUserListError> {
        let allowlist = parse_entries(
            ALLOWLIST_FILE,
            &documents.allowlist,
            |file, object, name, _, _| {
                let user = user(file, object, name)?;
                Ok((user.id, user))
            },
        )?;
        let operators = parse_entries(
            OPERATORS_FILE,
            &documents.operators,
            |file, object, name, _, _| {
                let user = user(file, object, name)?;
                // `PermissionLevel.ALL` when the field is absent, and a missing
                // `bypassesPlayerLimit` reads as false (ServerOpListEntry.java:19-21).
                let level = match object.get("level") {
                    None => CommandPermission::All,
                    Some(value) => {
                        let id = value
                            .as_i64()
                            .ok_or_else(|| invalid(file, "level must be an integer"))?;
                        permission_by_id(id).ok_or_else(|| {
                            invalid(file, format!("unknown permission level {id}"))
                        })?
                    }
                };
                let bypasses_player_limit = match object.get("bypassesPlayerLimit") {
                    None => false,
                    Some(value) => value
                        .as_bool()
                        .ok_or_else(|| invalid(file, "bypassesPlayerLimit must be a boolean"))?,
                };
                Ok((
                    user.id,
                    OperatorEntry {
                        user,
                        level,
                        bypasses_player_limit,
                    },
                ))
            },
        )?;
        let player_bans = parse_entries(
            PLAYER_BANS_FILE,
            &documents.player_bans,
            |file, object, name, reason, source| {
                let user = user(file, object, name)?;
                let ban = ban_record(file, object, reason, source)?;
                Ok((user.id, PlayerBanEntry { user, ban }))
            },
        )?;
        let ip_bans = parse_entries(
            IP_BANS_FILE,
            &documents.ip_bans,
            |file, object, _, reason, source| {
                let ip = object
                    .get("ip")
                    .and_then(Value::as_str)
                    .ok_or_else(|| invalid(file, "entry has no ip"))?;
                if !valid_ip_literal(ip) {
                    return Err(invalid(file, format!("{ip} is not an IP literal")));
                }
                Ok((ip.to_string(), ban_record(file, object, reason, source)?))
            },
        )?;
        Ok(Self {
            allowlist,
            operators,
            player_bans,
            ip_bans,
        })
    }

    /// `StoredUserList.removeExpired` (StoredUserList.java:111-123) over both
    /// ban lists, reporting whether anything was dropped.
    pub fn remove_expired(&mut self, now: i64) -> bool {
        let before = self.player_bans.len() + self.ip_bans.len();
        self.player_bans
            .retain(|_, entry| !entry.ban.has_expired(now));
        self.ip_bans.retain(|_, record| !record.has_expired(now));
        before != self.player_bans.len() + self.ip_bans.len()
    }
}

fn encode(entries: impl Iterator<Item = Box<RawValue>>) -> Vec<u8> {
    let array = entries.collect::<Vec<_>>();
    let mut bytes =
        serde_json::to_vec_pretty(&array).expect("stored user list entries are plain JSON objects");
    bytes.push(b'\n');
    bytes
}

fn raw_fields(object: Map<String, Value>) -> BTreeMap<String, Box<RawValue>> {
    object
        .into_iter()
        .map(|(key, value)| {
            (
                key,
                serde_json::value::to_raw_value(&value).expect("stored field serializes"),
            )
        })
        .collect()
}

fn user_object(user: &NameAndId, object: Map<String, Value>) -> BTreeMap<String, Box<RawValue>> {
    let mut object = raw_fields(object);
    object.insert(
        "uuid".into(),
        serde_json::value::to_raw_value(&user.id.to_string()).expect("UUID serializes"),
    );
    object.insert(
        "name".into(),
        serde_json::value::to_raw_value(&user.name).expect("name serializes"),
    );
    object
}

fn ban_object(record: &BanRecord) -> Map<String, Value> {
    // `BanListEntry.serialize` (BanListEntry.java:88-93).
    let mut object = Map::new();
    object.insert(
        "created".to_string(),
        Value::String(format_ban_date(record.created)),
    );
    object.insert(
        "expires".to_string(),
        Value::String(
            record
                .expires
                .map_or_else(|| EXPIRES_NEVER.to_string(), format_ban_date),
        ),
    );
    object
}

// BanListEntry.java:83-88; StoredUserList.java:131-158.
fn ban_document(mut fields: BTreeMap<String, Box<RawValue>>, record: &BanRecord) -> Box<RawValue> {
    fields.insert(
        "source".into(),
        serde_json::value::to_raw_value(&record.source).expect("ban source serializes"),
    );
    fields.insert(
        "reason".into(),
        serde_json::value::to_raw_value(&record.reason).expect("ban reason serializes"),
    );
    serde_json::value::to_raw_value(&fields).expect("ban document serializes")
}

fn parse_entries<K: Ord, V>(
    file: &'static str,
    bytes: &[u8],
    mut entry: impl FnMut(
        &'static str,
        &Map<String, Value>,
        Option<JavaString>,
        Option<JavaString>,
        Option<JavaString>,
    ) -> Result<(K, V), StoredUserListError>,
) -> Result<BTreeMap<K, V>, StoredUserListError> {
    if bytes.is_empty() {
        return Ok(BTreeMap::new());
    }
    let document: &RawValue = serde_json::from_slice(bytes)
        .map_err(|error| invalid(file, format!("document is not JSON: {error}")))?;
    if document.get() == "null" {
        return Ok(BTreeMap::new());
    }
    let array: Vec<&RawValue> = serde_json::from_str(document.get())
        .map_err(|_| invalid(file, "document must be a JSON array"))?;
    if array.len() > MAX_LIST_ENTRIES {
        return Err(invalid(
            file,
            format!(
                "document holds {} entries, maximum is {MAX_LIST_ENTRIES}",
                array.len()
            ),
        ));
    }
    let mut entries = BTreeMap::new();
    for element in array {
        let mut raw: BTreeMap<String, &RawValue> = serde_json::from_str(element.get())
            .map_err(|_| invalid(file, "every entry must be a JSON object"))?;
        let mut optional_text = |field: &str| match raw.remove(field) {
            None => Ok(None),
            Some(value) if value.get() == "null" => Ok(None),
            Some(value) => JavaString::from_json_string(value.get())
                .map(Some)
                .map_err(|_| invalid(file, format!("{field} must be a string"))),
        };
        let name = if file == IP_BANS_FILE {
            None
        } else {
            optional_text("name")?
        };
        let (reason, source) = if file == PLAYER_BANS_FILE || file == IP_BANS_FILE {
            // Management input limits do not bound command names or expanded messages.
            (optional_text("reason")?, optional_text("source")?)
        } else {
            (None, None)
        };
        let object = raw
            .into_iter()
            .map(|(key, value)| {
                serde_json::from_str(value.get())
                    .map(|value| (key, value))
                    .map_err(|error| invalid(file, format!("document is not JSON: {error}")))
            })
            .collect::<Result<Map<String, Value>, _>>()?;
        let (key, value) = entry(file, &object, name, reason, source)?;
        entries.insert(key, value);
    }
    Ok(entries)
}

fn user(
    file: &'static str,
    object: &Map<String, Value>,
    name: Option<JavaString>,
) -> Result<NameAndId, StoredUserListError> {
    // `NameAndId.fromJson` (NameAndId.java:25-40) needs both fields and a
    // parseable uuid; vanilla drops such an entry, this server refuses the
    // document so a hand-edited list cannot silently lose an operator.
    let uuid = object
        .get("uuid")
        .and_then(Value::as_str)
        .ok_or_else(|| invalid(file, "entry has no uuid"))?;
    let name = name.ok_or_else(|| invalid(file, "entry has no name"))?;
    let id = Uuid::parse_str(uuid).map_err(|_| invalid(file, format!("{uuid} is not a UUID")))?;

    Ok(NameAndId { id, name })
}

fn ban_record(
    file: &'static str,
    object: &Map<String, Value>,
    reason: Option<JavaString>,
    source: Option<JavaString>,
) -> Result<BanRecord, StoredUserListError> {
    let created = match object.get("created") {
        None => 0,
        Some(value) => parse_date_field(file, value, "created")?
            .ok_or_else(|| invalid(file, "created must not be \"forever\""))?,
    };
    let expires = match object.get("expires") {
        None | Some(Value::Null) => None,
        Some(value) => parse_date_field(file, value, "expires")?,
    };
    Ok(BanRecord {
        created,
        source: source.unwrap_or_else(|| UNKNOWN_SOURCE.into()),
        expires,
        reason,
    })
}

fn parse_date_field(
    file: &'static str,
    value: &Value,
    field: &str,
) -> Result<Option<i64>, StoredUserListError> {
    let text = value
        .as_str()
        .ok_or_else(|| invalid(file, format!("{field} must be a string")))?;
    if text == EXPIRES_NEVER {
        return Ok(None);
    }
    parse_ban_date(text)
        .map(Some)
        .ok_or_else(|| invalid(file, format!("{field} {text:?} is not a ban date")))
}

/// `PermissionLevel.byId` over the ladder `CommandPermission` already declares.
pub fn permission_by_id(id: i64) -> Option<CommandPermission> {
    [
        CommandPermission::All,
        CommandPermission::Moderator,
        CommandPermission::GameMaster,
        CommandPermission::Admin,
        CommandPermission::Owner,
    ]
    .into_iter()
    .find(|level| i64::from(level.id()) == id)
}

/// `InetAddresses.isInetAddress` as `IpBanlistService` uses it
/// (IpBanlistService.java:75, :107): the stored key must be an IP literal, not
/// a host name, so a ban can never turn into a name lookup.
pub fn valid_ip_literal(text: &str) -> bool {
    text.parse::<std::net::IpAddr>().is_ok()
}

/// `BanListEntry.DATE_FORMAT` (BanListEntry.java:13):
/// `yyyy-MM-dd HH:mm:ss Z` in `Locale.ROOT`. This server writes UTC, so the
/// RFC 822 zone it emits is always `+0000`.
pub fn format_ban_date(epoch_millis: i64) -> String {
    let seconds = epoch_millis.div_euclid(1_000);
    let days = seconds.div_euclid(86_400);
    let time_of_day = seconds.rem_euclid(86_400);
    let (year, month, day) = civil_from_days(days);
    let hour = time_of_day / 3_600;
    let minute = (time_of_day % 3_600) / 60;
    let second = time_of_day % 60;
    format!("{year:04}-{month:02}-{day:02} {hour:02}:{minute:02}:{second:02} +0000")
}

/// `PlayerList.BAN_DATE_FORMAT` (PlayerList.java:113):
/// `yyyy-MM-dd 'at' HH:mm:ss z`, the *login refusal* format, which differs from
/// the document format [`format_ban_date`] writes. This server formats in UTC,
/// so the general time-zone name is always `UTC`.
pub fn format_ban_expiry_date(epoch_millis: i64) -> String {
    let document = format_ban_date(epoch_millis);
    let (date, rest) = document.split_at(10);
    let time = rest
        .trim_start()
        .split(' ')
        .next()
        .expect("the document format always carries a time");
    format!("{date} at {time} UTC")
}

/// `ExtraCodecs.INSTANT_ISO8601`, the codec every management DTO's `expires`
/// field is written with (`BanlistService.UserBanDto`, BanlistService.java:117;
/// `IpBanlistService`, IpBanlistService.java:96, :141). It is
/// `DateTimeFormatter.ISO_INSTANT`, which always writes UTC with a `Z`
/// designator and only writes a fraction when there is one.
///
/// The management *documents* keep [`format_ban_date`]; this is the wire form
/// the request and answer surface uses, and the two must not be confused.
pub fn format_instant_iso8601(epoch_millis: i64) -> String {
    let seconds = epoch_millis.div_euclid(1_000);
    let millis = epoch_millis.rem_euclid(1_000);
    let days = seconds.div_euclid(86_400);
    let time_of_day = seconds.rem_euclid(86_400);
    let (year, month, day) = civil_from_days(days);
    let hour = time_of_day / 3_600;
    let minute = (time_of_day % 3_600) / 60;
    let second = time_of_day % 60;
    let mut text = format!("{year:04}-{month:02}-{day:02}T{hour:02}:{minute:02}:{second:02}");
    if millis != 0 {
        text.push_str(&format!(".{millis:03}"));
    }
    text.push('Z');
    text
}

/// The inverse. `Z` and a numeric `+HH:MM` offset are both accepted, because
/// `DateTimeFormatter.ISO_INSTANT` parses both; a bare local time is not, since
/// it would name an instant that depends on the reader's zone.
pub fn parse_instant_iso8601(text: &str) -> Option<i64> {
    if !text.is_ascii() {
        return None;
    }
    let bytes = text.as_bytes();
    if bytes.len() < 20 || bytes[4] != b'-' || bytes[7] != b'-' || bytes[10] != b'T' {
        return None;
    }
    if bytes[13] != b':' || bytes[16] != b':' {
        return None;
    }
    let year = digits(&text[0..4])?;
    let month = digits(&text[5..7])?;
    let day = digits(&text[8..10])?;
    let hour = digits(&text[11..13])?;
    let minute = digits(&text[14..16])?;
    let second = digits(&text[17..19])?;
    let mut rest = &text[19..];
    let mut fraction_millis = 0;
    if let Some(after_dot) = rest.strip_prefix('.') {
        let fraction_length = after_dot
            .bytes()
            .position(|byte| !byte.is_ascii_digit())
            .unwrap_or(after_dot.len());
        if fraction_length == 0 || fraction_length > 9 {
            return None;
        }
        let mut scaled = String::from(&after_dot[..fraction_length.min(3)]);
        while scaled.len() < 3 {
            scaled.push('0');
        }
        fraction_millis = digits(&scaled)?;
        rest = &after_dot[fraction_length..];
    }
    let offset_seconds = if rest == "Z" || rest == "z" {
        0
    } else {
        let sign = match rest.as_bytes().first() {
            Some(b'+') => 1,
            Some(b'-') => -1,
            _ => return None,
        };
        let zone = &rest[1..];
        let (zone_hours, zone_minutes) = match zone.len() {
            5 if zone.as_bytes()[2] == b':' => (digits(&zone[0..2])?, digits(&zone[3..5])?),
            4 => (digits(&zone[0..2])?, digits(&zone[2..4])?),
            2 => (digits(zone)?, 0),
            _ => return None,
        };
        if zone_hours > 18 || zone_minutes > 59 {
            return None;
        }
        sign * (zone_hours * 3_600 + zone_minutes * 60)
    };
    if !(1..=12).contains(&month)
        || !(1..=31).contains(&day)
        || hour > 23
        || minute > 59
        || second > 59
    {
        return None;
    }
    let days = days_from_civil(year, month as u32, day as u32);
    let seconds = days * 86_400 + hour * 3_600 + minute * 60 + second - offset_seconds;
    seconds.checked_mul(1_000)?.checked_add(fraction_millis)
}

/// The inverse of [`format_ban_date`], accepting any RFC 822 numeric zone.
pub fn parse_ban_date(text: &str) -> Option<i64> {
    if !text.is_ascii() {
        return None;
    }
    let bytes = text.as_bytes();
    if bytes.len() != 25 || bytes[4] != b'-' || bytes[7] != b'-' || bytes[10] != b' ' {
        return None;
    }
    if bytes[13] != b':' || bytes[16] != b':' || bytes[19] != b' ' {
        return None;
    }
    let year = digits(&text[0..4])?;
    let month = digits(&text[5..7])?;
    let day = digits(&text[8..10])?;
    let hour = digits(&text[11..13])?;
    let minute = digits(&text[14..16])?;
    let second = digits(&text[17..19])?;
    let sign = match bytes[20] {
        b'+' => 1,
        b'-' => -1,
        _ => return None,
    };
    let zone_hours = digits(&text[21..23])?;
    let zone_minutes = digits(&text[23..25])?;
    if !(1..=12).contains(&month)
        || !(1..=31).contains(&day)
        || hour > 23
        || minute > 59
        || second > 59
        || zone_hours > 23
        || zone_minutes > 59
    {
        return None;
    }
    let days = days_from_civil(year, month as u32, day as u32);
    let offset = sign * (zone_hours * 3_600 + zone_minutes * 60);
    let seconds = days * 86_400 + hour * 3_600 + minute * 60 + second - offset;
    seconds.checked_mul(1_000)
}

fn digits(text: &str) -> Option<i64> {
    text.bytes()
        .all(|byte| byte.is_ascii_digit())
        .then(|| text.parse::<i64>().ok())
        .flatten()
}

/// Howard Hinnant's `days_from_civil` over the proleptic Gregorian calendar,
/// which is the calendar `SimpleDateFormat` uses for every date this server
/// writes.
fn days_from_civil(year: i64, month: u32, day: u32) -> i64 {
    let year = if month <= 2 { year - 1 } else { year };
    let era = if year >= 0 { year } else { year - 399 } / 400;
    let year_of_era = year - era * 400;
    let month = i64::from(month);
    let day_of_year =
        (153 * (if month > 2 { month - 3 } else { month + 9 }) + 2) / 5 + i64::from(day) - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    era * 146_097 + day_of_era - 719_468
}

/// The inverse, `civil_from_days`.
fn civil_from_days(days: i64) -> (i64, u32, u32) {
    let days = days + 719_468;
    let era = if days >= 0 { days } else { days - 146_096 } / 146_097;
    let day_of_era = days - era * 146_097;
    let year_of_era =
        (day_of_era - day_of_era / 1_460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let year = year_of_era + era * 400;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_index = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * month_index + 2) / 5 + 1;
    let month = if month_index < 10 {
        month_index + 3
    } else {
        month_index - 9
    };
    (
        if month <= 2 { year + 1 } else { year },
        month as u32,
        day as u32,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn epoch(text: &str) -> i64 {
        parse_ban_date(text).expect("fixture date parses")
    }

    #[test]
    fn instants_round_trip_through_the_management_wire_format() {
        for text in [
            "1970-01-01T00:00:00Z",
            "2026-08-17T10:12:27Z",
            "2000-02-29T23:59:59.500Z",
            "1969-12-31T23:59:59Z",
        ] {
            let instant = parse_instant_iso8601(text).unwrap_or_else(|| panic!("{text} parses"));
            assert_eq!(format_instant_iso8601(instant), text, "{text}");
        }
        // An offset names the same instant a `Z` time does.
        assert_eq!(
            parse_instant_iso8601("2026-08-17T12:12:27+02:00"),
            parse_instant_iso8601("2026-08-17T10:12:27Z")
        );
    }

    #[test]
    fn ban_dates_round_trip_through_the_vanilla_format() {
        for text in [
            "1970-01-01 00:00:00 +0000",
            "2026-08-17 10:12:27 +0000",
            "2000-02-29 23:59:59 +0000",
            "1969-12-31 23:59:59 +0000",
            "2100-03-01 12:00:00 +0000",
        ] {
            assert_eq!(format_ban_date(epoch(text)), text, "{text}");
        }
    }

    #[test]
    fn the_login_refusal_uses_the_player_list_date_format() {
        assert_eq!(
            format_ban_expiry_date(epoch("2026-09-17 10:12:27 +0000")),
            "2026-09-17 at 10:12:27 UTC"
        );
    }

    #[test]
    fn a_zoned_date_normalizes_to_utc() {
        assert_eq!(
            epoch("2026-08-17 18:12:27 +0800"),
            epoch("2026-08-17 10:12:27 +0000")
        );
        assert_eq!(
            epoch("2026-08-17 06:12:27 -0400"),
            epoch("2026-08-17 10:12:27 +0000")
        );
    }

    fn empty_documents() -> StoredUserListDocuments {
        StoredUserListDocuments::default()
    }

    fn sample() -> StoredUserLists {
        let user = NameAndId::offline("bbb-client");
        let other = NameAndId::offline("bbb-peer");
        StoredUserLists {
            allowlist: BTreeMap::from([(user.id, user.clone())]),
            operators: BTreeMap::from([(
                user.id,
                OperatorEntry {
                    user,
                    level: CommandPermission::GameMaster,
                    bypasses_player_limit: true,
                },
            )]),
            player_bans: BTreeMap::from([(
                other.id,
                PlayerBanEntry {
                    user: other,
                    ban: BanRecord {
                        created: epoch("2026-08-17 10:12:27 +0000"),
                        source: "Management server".into(),
                        expires: Some(epoch("2026-09-17 10:12:27 +0000")),
                        reason: Some("griefing".into()),
                    },
                },
            )]),
            ip_bans: BTreeMap::from([(
                "203.0.113.7".to_string(),
                BanRecord {
                    created: epoch("2026-08-17 10:12:27 +0000"),
                    source: "Management server".into(),
                    expires: None,
                    reason: None,
                },
            )]),
        }
    }

    #[test]
    fn the_four_documents_round_trip() {
        let lists = sample();
        let parsed = StoredUserLists::parse(&lists.documents()).expect("documents parse");
        assert_eq!(parsed, lists);
    }

    #[test]
    fn documents_carry_the_vanilla_field_names() {
        let documents = sample().documents();
        let operators: Value = serde_json::from_slice(&documents.operators).unwrap();
        let entry = &operators.as_array().unwrap()[0];
        assert_eq!(entry["name"], Value::from("bbb-client"));
        assert_eq!(
            entry["uuid"],
            Value::from(NameAndId::offline("bbb-client").id.to_string())
        );
        assert_eq!(entry["level"], Value::from(2));
        assert_eq!(entry["bypassesPlayerLimit"], Value::from(true));

        let ip_bans: Value = serde_json::from_slice(&documents.ip_bans).unwrap();
        let entry = &ip_bans.as_array().unwrap()[0];
        assert_eq!(entry["ip"], Value::from("203.0.113.7"));
        assert_eq!(entry["expires"], Value::from("forever"));
        assert_eq!(entry["reason"], Value::Null);
        assert_eq!(entry["source"], Value::from("Management server"));
    }

    #[test]
    fn an_absent_or_empty_document_is_the_empty_list() {
        let documents = StoredUserListDocuments {
            allowlist: Vec::new(),
            operators: b"null".to_vec(),
            player_bans: b"[]".to_vec(),
            ip_bans: Vec::new(),
        };
        assert_eq!(
            StoredUserLists::parse(&documents).expect("documents parse"),
            StoredUserLists::default()
        );
    }

    #[test]
    fn an_operator_entry_defaults_the_optional_fields() {
        let documents = StoredUserListDocuments {
            operators: br#"[{"uuid":"00000000-0000-0000-0000-000000000001","name":"a"}]"#.to_vec(),
            ..empty_documents()
        };
        let parsed = StoredUserLists::parse(&documents).expect("documents parse");
        let entry = parsed.operators.values().next().expect("one operator");
        assert_eq!(entry.level, CommandPermission::All);
        assert!(!entry.bypasses_player_limit);
    }

    #[test]
    fn expired_bans_are_dropped_on_demand() {
        let mut lists = sample();
        let now = epoch("2026-10-01 00:00:00 +0000");
        assert!(lists.remove_expired(now));
        assert!(lists.player_bans.is_empty());
        // The permanent ip ban has no expiry and survives.
        assert_eq!(lists.ip_bans.len(), 1);
        assert!(!lists.remove_expired(now));
    }
}
