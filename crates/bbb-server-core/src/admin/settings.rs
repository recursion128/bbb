//! The mutable server settings the management surface owns.
//!
//! Vanilla owner: `ServerSettingsService` (ServerSettingsService.java:9-169),
//! whose `serversettings/<key>` and `serversettings/<key>/set` pairs read and
//! write `DedicatedServerProperties` through
//! `MinecraftServerSettingsServiceImpl`. Every setter there returns the
//! *effective* value after the write, so a clamped or refused write is visible
//! to the caller rather than silent; this module keeps that contract.
//!
//! Only settings this server actually honors are declared. A management API
//! that answered a `serversettings/<key>` the server ignores would be
//! reporting a value nothing reads. The
//! `scripts/tests/test_admin_settings_alignment.py` guard asserts every key
//! below is a real vanilla `serversettings/<key>` registration, so the subset
//! cannot drift into an invented name.

use bbb_protocol::{
    difficulties::{vanilla_difficulty_for_serialized_name, Difficulty},
    game_types::{vanilla_game_type_for_serialized_name, GameType},
};

use crate::commands::CommandPermission;

use super::stored_user_list::{permission_by_id, MAX_ADMIN_TEXT_CHARS};

/// The server-list description a fresh server starts with. `server.properties`
/// ships `A Minecraft Server`; this product ships its own name, and the CLI
/// default and the world default both read it here so they cannot drift.
pub const DEFAULT_MOTD: &str = "A BBB Server";

/// The chunk-radius floor `ServerWorldConfig::validate` already enforces, and
/// the ceiling vanilla's own `view-distance` property accepts.
pub const MIN_DISTANCE_CHUNKS: i32 = 2;
pub const MAX_DISTANCE_CHUNKS: i32 = 32;

/// The clamp `entity-broadcast-range-percentage` is read through
/// (`DedicatedServerProperties.java:114-116`), default 100.
pub const MIN_ENTITY_BROADCAST_RANGE_PERCENTAGE: i32 = 10;
pub const MAX_ENTITY_BROADCAST_RANGE_PERCENTAGE: i32 = 1000;
pub const DEFAULT_ENTITY_BROADCAST_RANGE_PERCENTAGE: i32 = 100;

/// One settable server setting, carrying its value.
///
/// The variant names are the vanilla `serversettings/<key>` keys; the payload
/// types are the ones the corresponding vanilla method takes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AdminSetting {
    /// `serversettings/difficulty` (`ServerSettingsService.setDifficulty`).
    Difficulty(Difficulty),
    /// `serversettings/game_mode` (`ServerSettingsService.setGameMode`).
    GameMode(GameType),
    /// `serversettings/enforce_allowlist` (`setEnforceAllowlist`). Writing it
    /// also kicks unlisted players (`MinecraftServerSettingsServiceImpl.java:52`).
    EnforceAllowlist(bool),
    /// `serversettings/use_allowlist` (`setUsingAllowlist`), likewise
    /// (`MinecraftServerSettingsServiceImpl.java:65`).
    UseAllowlist(bool),
    /// `serversettings/max_players` (`setMaxPlayers`).
    MaxPlayers(i32),
    /// `serversettings/motd` (`setMotd`).
    Motd(String),
    /// MinecraftServerSettingsServiceImpl.java:118-126.
    SpawnProtectionRadius(i32),
    /// `serversettings/operator_user_permission_level`
    /// (`setOperatorUserPermissionLevel`): the level `PlayerList.op` grants
    /// when the request names none (PlayerList.java:537-543).
    OperatorUserPermissionLevel(CommandPermission),
    /// `serversettings/view_distance` (`setViewDistance`).
    ViewDistance(i32),
    /// `serversettings/simulation_distance` (`setSimulationDistance`).
    SimulationDistance(i32),
    /// `serversettings/hide_online_players` (`setHidesOnlinePlayers`).
    HideOnlinePlayers(bool),
    /// `serversettings/status_replies` (`setRepliesToStatus`).
    StatusReplies(bool),
    AcceptTransfers(bool),
    /// `serversettings/allow_flight` (`setAllowFlight`).
    AllowFlight(bool),
    /// `serversettings/pause_when_empty_seconds` (`setPauseWhenEmptySeconds`).
    PauseWhenEmptySeconds(i32),
    /// `serversettings/entity_broadcast_range`
    /// (`setEntityBroadcastRangePercentage`).
    EntityBroadcastRange(i32),
}

impl AdminSetting {
    /// The vanilla `serversettings/<key>` key this setting is registered under.
    pub fn key(&self) -> &'static str {
        match self {
            Self::Difficulty(_) => "difficulty",
            Self::GameMode(_) => "game_mode",
            Self::EnforceAllowlist(_) => "enforce_allowlist",
            Self::UseAllowlist(_) => "use_allowlist",
            Self::MaxPlayers(_) => "max_players",
            Self::Motd(_) => "motd",
            Self::SpawnProtectionRadius(_) => "spawn_protection_radius",
            Self::OperatorUserPermissionLevel(_) => "operator_user_permission_level",
            Self::ViewDistance(_) => "view_distance",
            Self::SimulationDistance(_) => "simulation_distance",
            Self::HideOnlinePlayers(_) => "hide_online_players",
            Self::StatusReplies(_) => "status_replies",
            Self::AcceptTransfers(_) => "accept_transfers",
            Self::AllowFlight(_) => "allow_flight",
            Self::PauseWhenEmptySeconds(_) => "pause_when_empty_seconds",
            Self::EntityBroadcastRange(_) => "entity_broadcast_range",
        }
    }
}

/// Every key this server owns, in the order the settings readout lists them.
pub const ADMIN_SETTING_KEYS: &[&str] = &[
    "difficulty",
    "game_mode",
    "enforce_allowlist",
    "use_allowlist",
    "max_players",
    "motd",
    "spawn_protection_radius",
    "operator_user_permission_level",
    "view_distance",
    "simulation_distance",
    "hide_online_players",
    "status_replies",
    "accept_transfers",
    "allow_flight",
    "pause_when_empty_seconds",
    "entity_broadcast_range",
];

/// Why a setting write was refused.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SettingRejected {
    pub key: &'static str,
    pub detail: String,
}

/// The live values.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdminSettings {
    pub difficulty: Difficulty,
    pub game_mode: GameType,
    pub enforce_allowlist: bool,
    pub use_allowlist: bool,
    pub max_players: i32,
    pub motd: String,
    pub spawn_protection_radius: i32,
    pub operator_user_permission_level: CommandPermission,
    pub view_distance: i32,
    pub simulation_distance: i32,
    pub hide_online_players: bool,
    pub status_replies: bool,
    pub accepts_transfers: bool,
    pub allow_flight: bool,
    pub pause_when_empty_seconds: i32,
    pub entity_broadcast_range_percentage: i32,
}

impl AdminSettings {
    /// Applies one write and answers with the effective value, the way every
    /// vanilla setter answers by re-reading its getter.
    pub fn set(&mut self, setting: AdminSetting) -> Result<AdminSetting, SettingRejected> {
        let key = setting.key();
        match setting {
            AdminSetting::Difficulty(value) => self.difficulty = value,
            AdminSetting::GameMode(value) => self.game_mode = value,
            AdminSetting::EnforceAllowlist(value) => self.enforce_allowlist = value,
            AdminSetting::UseAllowlist(value) => self.use_allowlist = value,
            AdminSetting::MaxPlayers(value) => {
                if value < 1 {
                    return Err(rejected(key, "max players must be at least 1"));
                }
                self.max_players = value;
            }
            AdminSetting::Motd(value) => {
                if value.chars().count() > MAX_ADMIN_TEXT_CHARS {
                    return Err(rejected(
                        key,
                        format!("motd is longer than {MAX_ADMIN_TEXT_CHARS} characters"),
                    ));
                }
                self.motd = value;
            }
            AdminSetting::SpawnProtectionRadius(value) => self.spawn_protection_radius = value,
            AdminSetting::OperatorUserPermissionLevel(value) => {
                self.operator_user_permission_level = value;
            }
            AdminSetting::ViewDistance(value) => {
                self.view_distance = bounded_distance(key, value)?;
            }
            AdminSetting::SimulationDistance(value) => {
                self.simulation_distance = bounded_distance(key, value)?;
            }
            AdminSetting::HideOnlinePlayers(value) => self.hide_online_players = value,
            AdminSetting::StatusReplies(value) => self.status_replies = value,
            AdminSetting::AcceptTransfers(value) => self.accepts_transfers = value,
            AdminSetting::AllowFlight(value) => self.allow_flight = value,
            AdminSetting::PauseWhenEmptySeconds(value) => self.pause_when_empty_seconds = value,
            // The write reloads the properties, which clamp it.
            AdminSetting::EntityBroadcastRange(value) => {
                self.entity_broadcast_range_percentage = clamp_broadcast_range(value);
            }
        }
        Ok(self.get(key).expect("the key came from a settled variant"))
    }

    /// The current value of one key, or `None` when this server does not own
    /// that key.
    pub fn get(&self, key: &str) -> Option<AdminSetting> {
        Some(match key {
            "difficulty" => AdminSetting::Difficulty(self.difficulty),
            "game_mode" => AdminSetting::GameMode(self.game_mode),
            "enforce_allowlist" => AdminSetting::EnforceAllowlist(self.enforce_allowlist),
            "use_allowlist" => AdminSetting::UseAllowlist(self.use_allowlist),
            "max_players" => AdminSetting::MaxPlayers(self.max_players),
            "motd" => AdminSetting::Motd(self.motd.clone()),
            "spawn_protection_radius" => {
                AdminSetting::SpawnProtectionRadius(self.spawn_protection_radius)
            }
            "operator_user_permission_level" => {
                AdminSetting::OperatorUserPermissionLevel(self.operator_user_permission_level)
            }
            "view_distance" => AdminSetting::ViewDistance(self.view_distance),
            "simulation_distance" => AdminSetting::SimulationDistance(self.simulation_distance),
            "hide_online_players" => AdminSetting::HideOnlinePlayers(self.hide_online_players),
            "status_replies" => AdminSetting::StatusReplies(self.status_replies),
            "accept_transfers" => AdminSetting::AcceptTransfers(self.accepts_transfers),
            "allow_flight" => AdminSetting::AllowFlight(self.allow_flight),
            "pause_when_empty_seconds" => {
                AdminSetting::PauseWhenEmptySeconds(self.pause_when_empty_seconds)
            }
            "entity_broadcast_range" => {
                AdminSetting::EntityBroadcastRange(self.entity_broadcast_range_percentage)
            }
            _ => return None,
        })
    }

    /// Every owned setting, in `ADMIN_SETTING_KEYS` order.
    pub fn all(&self) -> Vec<AdminSetting> {
        ADMIN_SETTING_KEYS
            .iter()
            .map(|key| self.get(key).expect("every declared key is owned"))
            .collect()
    }
}

fn bounded_distance(key: &'static str, value: i32) -> Result<i32, SettingRejected> {
    if !(MIN_DISTANCE_CHUNKS..=MAX_DISTANCE_CHUNKS).contains(&value) {
        return Err(rejected(
            key,
            format!("distance must be in {MIN_DISTANCE_CHUNKS}..={MAX_DISTANCE_CHUNKS} chunks"),
        ));
    }
    Ok(value)
}

pub fn clamp_broadcast_range(value: i32) -> i32 {
    value.clamp(
        MIN_ENTITY_BROADCAST_RANGE_PERCENTAGE,
        MAX_ENTITY_BROADCAST_RANGE_PERCENTAGE,
    )
}

fn rejected(key: &'static str, detail: impl Into<String>) -> SettingRejected {
    SettingRejected {
        key,
        detail: detail.into(),
    }
}

/// The file in the admin directory that keeps the `server.properties` values a
/// running server rewrites (`DedicatedServerSettings.update`,
/// `DedicatedServerSettings.java:23-26`).
pub const PERSISTED_SETTINGS_FILE: &str = "server-settings.json";

/// The mutable `server.properties` values (`DedicatedServerProperties.java`)
/// that `/whitelist`, `/setidletimeout` or a `serversettings/*/set` write
/// back, under their property names; an absent key keeps the startup value.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PersistedSettings {
    pub white_list: Option<bool>,
    pub enforce_whitelist: Option<bool>,
    pub player_idle_timeout: Option<i32>,
    pub motd: Option<String>,
    pub max_players: Option<i32>,
    pub difficulty: Option<Difficulty>,
    pub gamemode: Option<GameType>,
    pub spawn_protection: Option<i32>,
    pub op_permission_level: Option<CommandPermission>,
    pub view_distance: Option<i32>,
    pub simulation_distance: Option<i32>,
    pub hide_online_players: Option<bool>,
    pub enable_status: Option<bool>,
    pub accepts_transfers: Option<bool>,
    pub allow_flight: Option<bool>,
    pub pause_when_empty_seconds: Option<i32>,
    pub entity_broadcast_range_percentage: Option<i32>,
}

impl PersistedSettings {
    /// An empty document is a server that never saved one.
    pub fn parse(bytes: &[u8]) -> Result<Self, String> {
        if bytes.iter().all(u8::is_ascii_whitespace) {
            return Ok(Self::default());
        }
        let value: serde_json::Value =
            serde_json::from_slice(bytes).map_err(|error| error.to_string())?;
        let object = value
            .as_object()
            .ok_or_else(|| "the document is not a JSON object".to_string())?;
        fn read<T>(
            object: &serde_json::Map<String, serde_json::Value>,
            key: &str,
            convert: impl FnOnce(&serde_json::Value) -> Option<T>,
        ) -> Result<Option<T>, String> {
            object
                .get(key)
                .map(|value| convert(value).ok_or_else(|| format!("{key} has an invalid value")))
                .transpose()
        }
        let int = |value: &serde_json::Value| value.as_i64().and_then(|n| i32::try_from(n).ok());
        let distance = |value: &serde_json::Value| {
            int(value).filter(|n| (MIN_DISTANCE_CHUNKS..=MAX_DISTANCE_CHUNKS).contains(n))
        };
        Ok(Self {
            white_list: read(object, "white-list", serde_json::Value::as_bool)?,
            enforce_whitelist: read(object, "enforce-whitelist", serde_json::Value::as_bool)?,
            player_idle_timeout: read(object, "player-idle-timeout", int)?,
            motd: read(object, "motd", |value| value.as_str().map(str::to_string))?,
            max_players: read(object, "max-players", |value| {
                int(value).filter(|n| *n >= 1)
            })?,
            difficulty: read(object, "difficulty", |value| {
                value.as_str().and_then(difficulty_by_name)
            })?,
            gamemode: read(object, "gamemode", |value| {
                value.as_str().and_then(game_type_by_name)
            })?,
            spawn_protection: read(object, "spawn-protection", int)?,
            op_permission_level: read(object, "op-permission-level", |value| {
                value.as_i64().and_then(permission_by_id)
            })?,
            view_distance: read(object, "view-distance", distance)?,
            simulation_distance: read(object, "simulation-distance", distance)?,
            hide_online_players: read(object, "hide-online-players", serde_json::Value::as_bool)?,
            enable_status: read(object, "enable-status", serde_json::Value::as_bool)?,
            accepts_transfers: read(object, "accepts-transfers", serde_json::Value::as_bool)?,
            allow_flight: read(object, "allow-flight", serde_json::Value::as_bool)?,
            pause_when_empty_seconds: read(object, "pause-when-empty-seconds", int)?,
            entity_broadcast_range_percentage: read(
                object,
                "entity-broadcast-range-percentage",
                |value| int(value).map(clamp_broadcast_range),
            )?,
        })
    }

    pub fn to_json(&self) -> Vec<u8> {
        let mut object = serde_json::Map::new();
        let mut put = |key: &str, value: Option<serde_json::Value>| {
            if let Some(value) = value {
                object.insert(key.into(), value);
            }
        };
        put("white-list", self.white_list.map(Into::into));
        put("enforce-whitelist", self.enforce_whitelist.map(Into::into));
        put(
            "player-idle-timeout",
            self.player_idle_timeout.map(Into::into),
        );
        put("motd", self.motd.clone().map(Into::into));
        put("max-players", self.max_players.map(Into::into));
        put(
            "difficulty",
            self.difficulty.map(|value| value.serialized_name().into()),
        );
        put(
            "gamemode",
            self.gamemode.map(|value| value.serialized_name().into()),
        );
        put("spawn-protection", self.spawn_protection.map(Into::into));
        put(
            "op-permission-level",
            self.op_permission_level.map(|value| value.id().into()),
        );
        put("view-distance", self.view_distance.map(Into::into));
        put(
            "simulation-distance",
            self.simulation_distance.map(Into::into),
        );
        put(
            "hide-online-players",
            self.hide_online_players.map(Into::into),
        );
        put("enable-status", self.enable_status.map(Into::into));
        put("accepts-transfers", self.accepts_transfers.map(Into::into));
        put("allow-flight", self.allow_flight.map(Into::into));
        put(
            "pause-when-empty-seconds",
            self.pause_when_empty_seconds.map(Into::into),
        );
        put(
            "entity-broadcast-range-percentage",
            self.entity_broadcast_range_percentage.map(Into::into),
        );
        let mut bytes = serde_json::to_vec_pretty(&serde_json::Value::Object(object))
            .expect("a JSON object always serializes");
        bytes.push(b'\n');
        bytes
    }
}

/// `Difficulty.byName` as `server.properties` and the management API both take
/// it (`vanilla_difficulty_for_serialized_name`).
pub fn difficulty_by_name(name: &str) -> Option<Difficulty> {
    vanilla_difficulty_for_serialized_name(name).map(|entry| entry.difficulty)
}

/// `GameType.CODEC` over serialized names (`serversettings/game_mode`).
pub fn game_type_by_name(name: &str) -> Option<GameType> {
    vanilla_game_type_for_serialized_name(name).map(|entry| entry.game_type)
}

/// `PermissionLevel.INT_CODEC` as `OperatorDto` and
/// `serversettings/operator_user_permission_level` both use it.
pub fn permission_level_by_id(id: i64) -> Option<CommandPermission> {
    permission_by_id(id)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn settings() -> AdminSettings {
        AdminSettings {
            difficulty: Difficulty::Easy,
            game_mode: GameType::Survival,
            enforce_allowlist: false,
            use_allowlist: false,
            max_players: 20,
            motd: "A BBB Server".to_string(),
            spawn_protection_radius: 16,
            operator_user_permission_level: CommandPermission::Owner,
            view_distance: 2,
            simulation_distance: 2,
            hide_online_players: false,
            status_replies: true,
            accepts_transfers: false,
            allow_flight: false,
            pause_when_empty_seconds: 60,
            entity_broadcast_range_percentage: 100,
        }
    }

    #[test]
    fn persisted_settings_round_trip_and_reject_wrong_types() {
        let saved = PersistedSettings {
            white_list: Some(true),
            enforce_whitelist: Some(false),
            player_idle_timeout: Some(7),
            motd: Some("saved".to_string()),
            max_players: Some(3),
            difficulty: Some(Difficulty::Hard),
            gamemode: Some(GameType::Creative),
            spawn_protection: Some(-1),
            op_permission_level: Some(CommandPermission::Admin),
            view_distance: Some(12),
            simulation_distance: Some(5),
            hide_online_players: Some(true),
            enable_status: Some(false),
            accepts_transfers: Some(true),
            allow_flight: Some(true),
            pause_when_empty_seconds: Some(0),
            entity_broadcast_range_percentage: Some(250),
        };
        assert_eq!(PersistedSettings::parse(&saved.to_json()), Ok(saved));
        assert_eq!(
            PersistedSettings::parse(b""),
            Ok(PersistedSettings::default())
        );
        assert!(PersistedSettings::parse(br#"{"white-list":1}"#).is_err());
        assert!(PersistedSettings::parse(br#"{"view-distance":40}"#).is_err());
    }

    #[test]
    fn every_declared_key_is_readable_and_writable() {
        let mut live = settings();
        for key in ADMIN_SETTING_KEYS {
            let current = live.get(key).expect("declared key is owned");
            assert_eq!(current.key(), *key);
            assert_eq!(live.set(current.clone()), Ok(current));
        }
        assert_eq!(live, settings());
        assert_eq!(live.all().len(), ADMIN_SETTING_KEYS.len());
    }

    #[test]
    fn a_write_answers_with_the_effective_value() {
        let mut live = settings();
        assert_eq!(
            live.set(AdminSetting::Difficulty(Difficulty::Hard)),
            Ok(AdminSetting::Difficulty(Difficulty::Hard))
        );
        assert_eq!(live.difficulty, Difficulty::Hard);
        assert_eq!(
            live.set(AdminSetting::EntityBroadcastRange(5000)),
            Ok(AdminSetting::EntityBroadcastRange(1000))
        );
    }

    #[test]
    fn out_of_range_writes_are_refused_and_change_nothing() {
        let mut live = settings();
        for setting in [
            AdminSetting::MaxPlayers(0),
            AdminSetting::ViewDistance(1),
            AdminSetting::ViewDistance(MAX_DISTANCE_CHUNKS + 1),
            AdminSetting::SimulationDistance(0),
            AdminSetting::Motd("m".repeat(MAX_ADMIN_TEXT_CHARS + 1)),
        ] {
            let key = setting.key();
            assert!(live.set(setting).is_err(), "{key} must be refused");
        }
        assert_eq!(live, settings());
    }

    #[test]
    fn an_unknown_key_is_not_owned() {
        assert_eq!(settings().get("unknown_setting"), None);
    }
}
