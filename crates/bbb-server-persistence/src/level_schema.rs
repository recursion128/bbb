use bbb_protocol::{
    difficulties::{vanilla_difficulty_for_serialized_name, Difficulty},
    normalize_resource_location, JavaString, NbtSummaryEntry, NbtSummaryValue,
};

use super::schema_support::{bool_or, bool_value, field, float_value, int_or, long_or};
use super::{LevelDatSource, LevelDatStore};
use bbb_server_types::save::{NamedCompound, SaveError, SaveResult, JAVA_26_1_DATA_VERSION};

/// The respawn point written by `LevelData.RespawnData.CODEC` in `level.dat`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Java26LevelSpawn {
    dimension: String,
    position: [i32; 3],
    yaw_bits: u32,
    pitch_bits: u32,
}

impl Java26LevelSpawn {
    /// `LevelData.RespawnData.of` (`LevelData.java:54-55`).
    pub fn new(dimension: String, position: [i32; 3], yaw: f32, pitch: f32) -> Self {
        Self {
            dimension,
            position,
            yaw_bits: yaw.to_bits(),
            pitch_bits: pitch.to_bits(),
        }
    }

    pub fn dimension(&self) -> &str {
        &self.dimension
    }

    pub fn position(&self) -> [i32; 3] {
        self.position
    }

    pub fn yaw(&self) -> f32 {
        f32::from_bits(self.yaw_bits)
    }

    pub fn pitch(&self) -> f32 {
        f32::from_bits(self.pitch_bits)
    }
}

/// The difficulty settings written by `LevelSettings.DifficultySettings.CODEC`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Java26DifficultySettings {
    difficulty: String,
    hardcore: bool,
    locked: bool,
}

impl Java26DifficultySettings {
    pub fn difficulty(&self) -> &str {
        &self.difficulty
    }

    pub fn hardcore(&self) -> bool {
        self.hardcore
    }

    pub fn locked(&self) -> bool {
        self.locked
    }
}

pub use bbb_server_types::save::Java26DataPacks;

/// The `level.dat` `Data` fields this server owns at save time.
///
/// Named as one record rather than as four arguments because they are one
/// write in vanilla too: `saveDataTag` re-encodes the whole `PrimaryLevelData`
/// (`LevelStorageSource.java:572-577`) once per `saveAllChunks`
/// (`MinecraftServer.java:599-600`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Java26LevelSaveState {
    /// `PrimaryLevelData.gameTime` (`PrimaryLevelData.java:133`).
    pub game_time: i64,
    /// `Util.getEpochMillis()` (`PrimaryLevelData.java:147-149`).
    pub last_played: i64,
    /// `LevelSettings.gameType().getId()` (`PrimaryLevelData.java:131`).
    pub game_type: i32,
    /// `PrimaryLevelData.initialized` (`PrimaryLevelData.java:138`).
    pub initialized: bool,
    /// `LevelSettings.DifficultySettings.difficulty`
    /// (`LevelSettings.java:58`).
    pub difficulty: Difficulty,
    /// `LevelSettings.DifficultySettings.locked` (`LevelSettings.java:60`).
    pub difficulty_locked: bool,
    /// `PrimaryLevelData.respawnData` written as `spawn`
    /// (`PrimaryLevelData.java:132`).
    pub spawn: Java26LevelSpawn,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Java26LevelData {
    raw: NamedCompound,
    level_name: JavaString,
    game_time: i64,
    last_played: i64,
    server_brands: Vec<JavaString>,
    was_modded: bool,
    game_type: i32,
    spawn: Java26LevelSpawn,
    allow_commands: bool,
    initialized: bool,
    difficulty_settings: Java26DifficultySettings,
    singleplayer_uuid: Option<[i32; 4]>,
    removed_features: Option<Vec<String>>,
    data_packs: Option<Java26DataPacks>,
    enabled_features: Option<Vec<String>>,
}

impl Java26LevelData {
    /// `PrimaryLevelData.parse` (`PrimaryLevelData.java:88-120`) and
    /// `LevelSettings.parse` (`LevelSettings.java:11-20`): every field falls
    /// back to its default.
    pub fn parse(raw: NamedCompound) -> SaveResult<Self> {
        let data = match field(raw.entries(), "Data") {
            Some(NbtSummaryValue::Compound(data)) => data,
            _ => return Err(invalid("level.dat root has no Data compound")),
        };
        require_exact_version(data)?;
        let game_type = int_or(data, "GameType", 0);
        // GameType.byId falls back to survival.
        let game_type = if (0..=3).contains(&game_type) {
            game_type
        } else {
            0
        };
        Ok(Self {
            level_name: match field(data, "LevelName") {
                Some(NbtSummaryValue::String(name)) => name.clone(),
                _ => JavaString::default(),
            },
            game_time: long_or(data, "Time", 0),
            last_played: long_or(data, "LastPlayed", 0),
            server_brands: java_string_list(field(data, "ServerBrands")).unwrap_or_default(),
            was_modded: bool_or(data, "WasModded", false),
            game_type,
            spawn: parse_spawn(field(data, "spawn")).unwrap_or_else(|| {
                Java26LevelSpawn::new("minecraft:overworld".to_string(), [0, 0, 0], 0.0, 0.0)
            }),
            allow_commands: bool_or(data, "allowCommands", game_type == 1),
            initialized: bool_or(data, "initialized", true),
            difficulty_settings: parse_difficulty_settings(field(data, "difficulty_settings"))
                .unwrap_or_else(|| Java26DifficultySettings {
                    difficulty: "normal".to_string(),
                    hardcore: false,
                    locked: false,
                }),
            singleplayer_uuid: match field(data, "singleplayer_uuid") {
                Some(NbtSummaryValue::IntArray(uuid)) => uuid.as_slice().try_into().ok(),
                _ => None,
            },
            removed_features: string_list(field(data, "removed_features")),
            data_packs: match field(data, "DataPacks") {
                Some(NbtSummaryValue::Compound(packs)) => Some(Java26DataPacks {
                    enabled: java_string_list(field(packs, "Enabled")).unwrap_or_default(),
                    disabled: java_string_list(field(packs, "Disabled")).unwrap_or_default(),
                }),
                _ => None,
            },
            enabled_features: string_list(field(data, "enabled_features")),
            raw,
        })
    }

    pub fn raw(&self) -> &NamedCompound {
        &self.raw
    }

    pub fn into_raw(self) -> NamedCompound {
        self.raw
    }

    /// Clones the retained raw document and replaces the fields a running
    /// server owns, leaving every other field -- including unknown extras --
    /// verbatim.
    ///
    /// `PrimaryLevelData.setTagData` (`PrimaryLevelData.java:122-145`) writes
    /// the whole document out of live state on every save, so each field it
    /// emits is one the server can have changed since the load. This server
    /// models six of them, and each one has a writer that can move it:
    ///
    /// - `Data.Time` (`:133`) is the game clock.
    /// - `Data.LastPlayed` (`:134`, `PrimaryLevelData.writeLastPlayed`
    ///   `:147-149`) is `Util.getEpochMillis()` at save time.
    /// - `Data.GameType` (`:131`) is `settings.gameType()`, which
    ///   `PrimaryLevelData.setGameType` (`:220-222`) moves. It has five callers
    ///   in the 26.1 tree and a dedicated server reaches three of them:
    ///   `DedicatedServer.initServer` (`DedicatedServer.java:237`) writes
    ///   `properties.gameMode.get()` over the loaded value at every startup,
    ///   `MinecraftServer.setDefaultGameType` (`MinecraftServer.java:1385-1387`)
    ///   is what `/defaultgamemode` calls, and `DedicatedServer.setGameMode`
    ///   (`:809-813`) is the management write of the property. The other two
    ///   change nothing on this path: `ServerLevel`'s constructor re-applies
    ///   `server.getDefaultGameType()` for a non-singleplayer server
    ///   (`ServerLevel.java:276-278`), which is the value `:237` just wrote and
    ///   is a no-op for every level but the overworld anyway
    ///   (`DerivedLevelData.setGameType` is an empty override,
    ///   `DerivedLevelData.java:47-48`), and `setupDebugLevel`
    ///   (`MinecraftServer.java:534-541`) is the debug world type. So the
    ///   startup write is the reason a `/defaultgamemode` is saved here and
    ///   still does not survive a restart -- it is the same shape as
    ///   `forceDifficulty` for the difficulty below.
    /// - `Data.spawn` (`:132`) is `respawnData`, which
    ///   `MinecraftServer.setRespawnData` (`MinecraftServer.java:1699-1706`)
    ///   writes when `/setworldspawn` changes it.
    /// - `Data.difficulty_settings.difficulty` (`:139`) is
    ///   `settings.difficultySettings().difficulty()`, which
    ///   `PrimaryLevelData.setDifficulty` (`:250-252`) moves for
    ///   `MinecraftServer.setDifficulty` (`MinecraftServer.java:1288-1294`).
    /// - `Data.difficulty_settings.locked` is
    ///   `PrimaryLevelData.setDifficultyLocked` (`:260-262`), which
    ///   `MinecraftServer.setDifficultyLocked` (`MinecraftServer.java:1306-1309`)
    ///   owns.
    ///
    /// `difficulty_settings.hardcore` is the third member of
    /// `LevelSettings.DifficultySettings` (`LevelSettings.java:54-64`) and is
    /// deliberately not here: `withDifficulty` and `withDifficultyLock`
    /// (`LevelSettings.java:26-44`) both carry it through unchanged and nothing
    /// in the tree sets it, so the loaded value is the live one.
    ///
    /// A field the loaded document lacked is inserted.
    pub fn raw_with_saved_state(&self, state: &Java26LevelSaveState) -> SaveResult<NamedCompound> {
        self.raw_with_saved_state_and_data_packs(state, None)
    }

    /// Applies the live datapack selection when one is supplied, while retaining
    /// every other field from the loaded document.
    ///
    /// `PrimaryLevelData.setTagData` stores `WorldDataConfiguration.MAP_CODEC`
    /// (`PrimaryLevelData.java:122-145`), whose datapack member is the ordered
    /// `Enabled` and `Disabled` string lists (`DataPackConfig.java:8-29`).
    pub fn raw_with_saved_state_and_data_packs(
        &self,
        state: &Java26LevelSaveState,
        data_packs: Option<&Java26DataPacks>,
    ) -> SaveResult<NamedCompound> {
        let mut raw = self.raw.clone();
        let data = upsert_compound(raw.entries_mut(), "Data");
        // `PrimaryLevelData.setModdedInfo` (`PrimaryLevelData.java:296-297`)
        // adds the running brand to an insertion-ordered set.
        let brand = bbb_protocol::packets::server_connection::BBB_SERVER_BRAND;
        if !self.server_brands.iter().any(|known| known == brand) {
            let mut brands = self
                .server_brands
                .iter()
                .map(|known| NbtSummaryValue::string(known.clone()))
                .collect::<Vec<_>>();
            brands.push(NbtSummaryValue::string(brand));
            upsert(data, "ServerBrands", NbtSummaryValue::List(brands));
        }
        upsert(data, "Time", NbtSummaryValue::Long(state.game_time));
        upsert(data, "LastPlayed", NbtSummaryValue::Long(state.last_played));
        if !(0..=3).contains(&state.game_type) {
            return Err(invalid("level.dat.Data.GameType must be in 0..=3"));
        }
        upsert(data, "GameType", NbtSummaryValue::Int(state.game_type));
        upsert(
            data,
            "initialized",
            NbtSummaryValue::Byte(i8::from(state.initialized)),
        );
        let settings = upsert_compound(data, "difficulty_settings");
        upsert(
            settings,
            "difficulty",
            NbtSummaryValue::string(state.difficulty.serialized_name()),
        );
        upsert(
            settings,
            "hardcore",
            NbtSummaryValue::Byte(i8::from(self.difficulty_settings.hardcore)),
        );
        upsert(
            settings,
            "locked",
            NbtSummaryValue::Byte(i8::from(state.difficulty_locked)),
        );
        // `PrimaryLevelData.setTagData` (`PrimaryLevelData.java:132`).
        upsert(data, "spawn", spawn_value(&state.spawn));
        if let Some(data_packs) = data_packs {
            let saved_data_packs = upsert_compound(data, "DataPacks");
            upsert(
                saved_data_packs,
                "Enabled",
                string_values(data_packs.enabled()),
            );
            upsert(
                saved_data_packs,
                "Disabled",
                string_values(data_packs.disabled()),
            );
        }
        Ok(raw)
    }

    pub fn level_name(&self) -> &JavaString {
        &self.level_name
    }

    pub fn game_time(&self) -> i64 {
        self.game_time
    }

    pub fn last_played(&self) -> i64 {
        self.last_played
    }

    pub fn server_brands(&self) -> &[JavaString] {
        &self.server_brands
    }

    pub fn was_modded(&self) -> bool {
        self.was_modded
    }

    pub fn game_type(&self) -> i32 {
        self.game_type
    }

    pub fn spawn(&self) -> &Java26LevelSpawn {
        &self.spawn
    }

    pub fn allow_commands(&self) -> bool {
        self.allow_commands
    }

    pub fn initialized(&self) -> bool {
        self.initialized
    }

    pub fn difficulty_settings(&self) -> &Java26DifficultySettings {
        &self.difficulty_settings
    }

    pub fn singleplayer_uuid(&self) -> Option<[i32; 4]> {
        self.singleplayer_uuid
    }

    pub fn removed_features(&self) -> Option<&[String]> {
        self.removed_features.as_deref()
    }

    pub fn data_packs(&self) -> Option<&Java26DataPacks> {
        self.data_packs.as_ref()
    }

    pub fn enabled_features(&self) -> Option<&[String]> {
        self.enabled_features.as_deref()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoadedJava26LevelData {
    pub source: LevelDatSource,
    pub data: Java26LevelData,
    pub decompressed_bytes: usize,
}

impl LevelDatStore {
    pub fn load_java_26_1(&self) -> SaveResult<LoadedJava26LevelData> {
        let loaded = self.load()?;
        Ok(LoadedJava26LevelData {
            source: loaded.source,
            data: Java26LevelData::parse(loaded.document)?,
            decompressed_bytes: loaded.decompressed_bytes,
        })
    }

    pub fn save_java_26_1(&self, data: &Java26LevelData) -> SaveResult<()> {
        self.save(data.raw())
    }
}

/// The field `name`, inserted with `value` when the loaded document lacked it.
fn upsert(entries: &mut Vec<NbtSummaryEntry>, name: &str, value: NbtSummaryValue) {
    match entries.iter_mut().find(|entry| entry.name == name) {
        Some(entry) => entry.value = value,
        None => entries.push(NbtSummaryEntry {
            name: name.into(),
            value,
        }),
    }
}

/// The compound under `name`, created (or replacing another tag) for writing.
fn upsert_compound<'a>(
    entries: &'a mut Vec<NbtSummaryEntry>,
    name: &str,
) -> &'a mut Vec<NbtSummaryEntry> {
    if !matches!(field(entries, name), Some(NbtSummaryValue::Compound(_))) {
        upsert(entries, name, NbtSummaryValue::Compound(Vec::new()));
    }
    let Some(NbtSummaryValue::Compound(entries)) = entries
        .iter_mut()
        .find(|entry| entry.name == name)
        .map(|entry| &mut entry.value)
    else {
        unreachable!("the field was just made a compound");
    };
    entries
}

fn string_values(values: &[JavaString]) -> NbtSummaryValue {
    NbtSummaryValue::List(
        values
            .iter()
            .cloned()
            .map(NbtSummaryValue::string)
            .collect(),
    )
}

fn spawn_value(spawn: &Java26LevelSpawn) -> NbtSummaryValue {
    NbtSummaryValue::Compound(vec![
        NbtSummaryEntry {
            name: "dimension".into(),
            value: NbtSummaryValue::string(spawn.dimension.clone()),
        },
        NbtSummaryEntry {
            name: "pos".into(),
            value: NbtSummaryValue::IntArray(spawn.position.to_vec()),
        },
        NbtSummaryEntry {
            name: "yaw".into(),
            value: NbtSummaryValue::Float(spawn.yaw_bits),
        },
        NbtSummaryEntry {
            name: "pitch".into(),
            value: NbtSummaryValue::Float(spawn.pitch_bits),
        },
    ])
}

/// Saves from other game versions require DataFixerUpper.
fn require_exact_version(data: &[NbtSummaryEntry]) -> SaveResult<()> {
    let data_version = int_or(data, "DataVersion", 0);
    if data_version != JAVA_26_1_DATA_VERSION {
        return Err(SaveError::UnsupportedDataVersion {
            expected: JAVA_26_1_DATA_VERSION,
            actual: data_version,
        });
    }
    Ok(())
}

/// `LevelData.RespawnData.CODEC` (`LevelData.java:33-42`).
fn parse_spawn(value: Option<&NbtSummaryValue>) -> Option<Java26LevelSpawn> {
    let Some(NbtSummaryValue::Compound(spawn)) = value else {
        return None;
    };
    let Some(NbtSummaryValue::String(dimension)) = field(spawn, "dimension") else {
        return None;
    };
    let dimension = dimension
        .try_to_string()
        .ok()
        .and_then(|raw| normalize_resource_location(raw).ok())?;
    let Some(NbtSummaryValue::IntArray(position)) = field(spawn, "pos") else {
        return None;
    };
    let position: [i32; 3] = position.as_slice().try_into().ok()?;
    let yaw = field(spawn, "yaw").and_then(float_value)?;
    let pitch = field(spawn, "pitch").and_then(float_value)?;
    if !(-180.0..=180.0).contains(&yaw) || !(-90.0..=90.0).contains(&pitch) {
        return None;
    }
    Some(Java26LevelSpawn::new(dimension, position, yaw, pitch))
}

/// `LevelSettings.DifficultySettings.CODEC` (`LevelSettings.java:54-64`).
fn parse_difficulty_settings(value: Option<&NbtSummaryValue>) -> Option<Java26DifficultySettings> {
    let Some(NbtSummaryValue::Compound(settings)) = value else {
        return None;
    };
    let Some(NbtSummaryValue::String(difficulty)) = field(settings, "difficulty") else {
        return None;
    };
    let difficulty = difficulty.try_to_string().ok()?;
    vanilla_difficulty_for_serialized_name(&difficulty)?;
    Some(Java26DifficultySettings {
        difficulty: difficulty.clone(),
        hardcore: field(settings, "hardcore").and_then(bool_value)?,
        locked: field(settings, "locked").and_then(bool_value)?,
    })
}

/// A list's string members, as `asStream().flatMap(asString)` reads them.
fn java_string_list(value: Option<&NbtSummaryValue>) -> Option<Vec<JavaString>> {
    let NbtSummaryValue::List(values) = value? else {
        return None;
    };
    Some(
        values
            .iter()
            .filter_map(|value| match value {
                NbtSummaryValue::String(value) => Some(value.clone()),
                _ => None,
            })
            .collect(),
    )
}

fn string_list(value: Option<&NbtSummaryValue>) -> Option<Vec<String>> {
    let Some(NbtSummaryValue::List(values)) = value else {
        return None;
    };
    Some(
        values
            .iter()
            .filter_map(|value| match value {
                NbtSummaryValue::String(value) => value.try_to_string().ok(),
                _ => None,
            })
            .collect(),
    )
}

fn invalid(detail: impl Into<String>) -> SaveError {
    SaveError::InvalidNbt {
        path: None,
        detail: detail.into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::JAVA_MAIN_SERIES;
    use bbb_server_types::save::JAVA_26_1_NAME;
    use bbb_server_types::save::JAVA_26_1_STORAGE_VERSION;

    fn entry(name: &str, value: NbtSummaryValue) -> NbtSummaryEntry {
        NbtSummaryEntry {
            name: name.into(),
            value,
        }
    }

    fn strings(values: &[&str]) -> NbtSummaryValue {
        NbtSummaryValue::List(
            values
                .iter()
                .map(|value| NbtSummaryValue::string(*value))
                .collect(),
        )
    }

    fn raw_level() -> NamedCompound {
        NamedCompound::new(
            "",
            vec![entry(
                "Data",
                NbtSummaryValue::Compound(vec![
                    entry(
                        "unknown_bbb_fixture",
                        NbtSummaryValue::string("preserve me"),
                    ),
                    entry("ServerBrands", strings(&["vanilla"])),
                    entry("WasModded", NbtSummaryValue::Byte(0)),
                    entry("DataVersion", NbtSummaryValue::Int(JAVA_26_1_DATA_VERSION)),
                    entry("GameType", NbtSummaryValue::Int(1)),
                    entry(
                        "spawn",
                        NbtSummaryValue::Compound(vec![
                            entry("dimension", NbtSummaryValue::string("minecraft:overworld")),
                            entry("pos", NbtSummaryValue::IntArray(vec![0, 64, 0])),
                            entry("yaw", NbtSummaryValue::Float(0.0_f32.to_bits())),
                            entry("pitch", NbtSummaryValue::Float(0.0_f32.to_bits())),
                        ]),
                    ),
                    entry("Time", NbtSummaryValue::Long(42)),
                    entry("LastPlayed", NbtSummaryValue::Long(1_786_000_000_000)),
                    entry("LevelName", NbtSummaryValue::string("BBB fixture")),
                    entry("version", NbtSummaryValue::Int(JAVA_26_1_STORAGE_VERSION)),
                    entry("allowCommands", NbtSummaryValue::Byte(1)),
                    entry("initialized", NbtSummaryValue::Byte(1)),
                    entry(
                        "difficulty_settings",
                        NbtSummaryValue::Compound(vec![
                            entry("difficulty", NbtSummaryValue::string("normal")),
                            entry("hardcore", NbtSummaryValue::Byte(0)),
                            entry("locked", NbtSummaryValue::Byte(0)),
                        ]),
                    ),
                    entry(
                        "singleplayer_uuid",
                        NbtSummaryValue::IntArray(vec![1, 2, 3, 4]),
                    ),
                    entry("removed_features", strings(&["minecraft:trade_rebalance"])),
                    entry(
                        "DataPacks",
                        NbtSummaryValue::Compound(vec![
                            entry("Enabled", strings(&["vanilla"])),
                            entry("Disabled", strings(&[])),
                        ]),
                    ),
                    entry("enabled_features", strings(&["minecraft:vanilla"])),
                    entry(
                        "Version",
                        NbtSummaryValue::Compound(vec![
                            entry("Name", NbtSummaryValue::string(JAVA_26_1_NAME)),
                            entry("Id", NbtSummaryValue::Int(JAVA_26_1_DATA_VERSION)),
                            entry("Snapshot", NbtSummaryValue::Byte(0)),
                            entry("Series", NbtSummaryValue::string(JAVA_MAIN_SERIES)),
                        ]),
                    ),
                ]),
            )],
        )
    }

    fn data_entries(raw: &mut NamedCompound) -> &mut Vec<NbtSummaryEntry> {
        let NbtSummaryValue::Compound(entries) = &mut raw.entries_mut()[0].value else {
            unreachable!()
        };
        entries
    }

    fn data_packs_entries(raw: &mut NamedCompound) -> &mut Vec<NbtSummaryEntry> {
        let data_packs = data_entries(raw)
            .iter_mut()
            .find(|entry| entry.name == "DataPacks")
            .unwrap();
        let NbtSummaryValue::Compound(entries) = &mut data_packs.value else {
            unreachable!()
        };
        entries
    }

    #[test]
    fn exact_level_dto_parses_and_preserves_unknown_fields() {
        let raw = raw_level();
        let data = Java26LevelData::parse(raw.clone()).unwrap();
        assert_eq!(data.level_name(), "BBB fixture");
        assert_eq!(data.game_time(), 42);
        assert_eq!(data.last_played(), 1_786_000_000_000);
        assert_eq!(data.server_brands(), ["vanilla"]);
        assert!(!data.was_modded());
        assert_eq!(data.game_type(), 1);
        assert_eq!(data.spawn().dimension(), "minecraft:overworld");
        assert_eq!(data.spawn().position(), [0, 64, 0]);
        assert!(data.allow_commands());
        assert!(data.initialized());
        assert_eq!(data.difficulty_settings().difficulty(), "normal");
        assert_eq!(data.singleplayer_uuid(), Some([1, 2, 3, 4]));
        assert_eq!(
            data.removed_features(),
            Some(["minecraft:trade_rebalance".to_string()].as_slice())
        );
        assert_eq!(data.data_packs().unwrap().enabled(), ["vanilla"]);
        assert_eq!(
            data.enabled_features(),
            Some(["minecraft:vanilla".to_string()].as_slice())
        );
        assert_eq!(data.into_raw(), raw);
    }

    /// The live fields `setTagData` re-emits, and nothing else.
    fn save_state() -> Java26LevelSaveState {
        Java26LevelSaveState {
            game_time: 100,
            last_played: 1_800_000_000_000,
            game_type: 0,
            initialized: true,
            difficulty: Difficulty::Hard,
            difficulty_locked: true,
            spawn: Java26LevelSpawn::new(
                "minecraft:the_nether".to_string(),
                [8, 80, -4],
                90.0,
                -10.0,
            ),
        }
    }

    #[test]
    fn raw_with_saved_state_replaces_only_the_fields_the_server_owns() {
        let raw = raw_level();
        let data = Java26LevelData::parse(raw.clone()).unwrap();
        let patched = data.raw_with_saved_state(&save_state()).unwrap();
        assert_eq!(
            patched,
            data.raw_with_saved_state_and_data_packs(&save_state(), None)
                .unwrap()
        );

        let mut expected = raw;
        for field in data_entries(&mut expected) {
            match field.name.to_jdk_string().as_str() {
                "Time" => field.value = NbtSummaryValue::Long(100),
                "LastPlayed" => field.value = NbtSummaryValue::Long(1_800_000_000_000),
                "GameType" => field.value = NbtSummaryValue::Int(0),
                "initialized" => field.value = NbtSummaryValue::Byte(1),
                "ServerBrands" => field.value = strings(&["vanilla", "bbb-server"]),
                "spawn" => {
                    field.value = NbtSummaryValue::Compound(vec![
                        entry("dimension", NbtSummaryValue::string("minecraft:the_nether")),
                        entry("pos", NbtSummaryValue::IntArray(vec![8, 80, -4])),
                        entry("yaw", NbtSummaryValue::Float(90.0_f32.to_bits())),
                        entry("pitch", NbtSummaryValue::Float((-10.0_f32).to_bits())),
                    ]);
                }
                "difficulty_settings" => {
                    let NbtSummaryValue::Compound(settings) = &mut field.value else {
                        panic!("the fixture writes difficulty_settings as a compound");
                    };
                    for setting in settings.iter_mut() {
                        match setting.name.to_jdk_string().as_str() {
                            "difficulty" => {
                                setting.value = NbtSummaryValue::string("hard");
                            }
                            "locked" => setting.value = NbtSummaryValue::Byte(1),
                            // `hardcore` has no setter anywhere in the tree
                            // (`LevelSettings.java:26-44` carry it through), so
                            // the loaded value has to survive untouched.
                            _ => {}
                        }
                    }
                }
                _ => {}
            }
        }
        assert_eq!(patched, expected);
        assert_eq!(data.raw(), &raw_level());
    }

    #[test]
    fn the_running_brand_is_appended_to_server_brands_once() {
        let data = Java26LevelData::parse(raw_level()).unwrap();
        let saved = data.raw_with_saved_state(&save_state()).unwrap();
        let reloaded = Java26LevelData::parse(saved).unwrap();
        assert_eq!(reloaded.server_brands(), ["vanilla", "bbb-server"]);
        let resaved = reloaded.raw_with_saved_state(&save_state()).unwrap();
        assert_eq!(
            Java26LevelData::parse(resaved).unwrap().server_brands(),
            ["vanilla", "bbb-server"]
        );
    }

    #[test]
    fn raw_with_saved_state_persists_initialization_without_changing_other_fields() {
        let mut raw = raw_level();
        data_entries(&mut raw)
            .iter_mut()
            .find(|entry| entry.name == "initialized")
            .unwrap()
            .value = NbtSummaryValue::Byte(0);
        let data = Java26LevelData::parse(raw.clone()).unwrap();
        assert!(!data.initialized());
        let state = Java26LevelSaveState {
            game_time: data.game_time(),
            last_played: data.last_played(),
            game_type: data.game_type(),
            initialized: true,
            difficulty: Difficulty::Normal,
            difficulty_locked: data.difficulty_settings().locked(),
            spawn: data.spawn().clone(),
        };

        let patched = data.raw_with_saved_state(&state).unwrap();
        let mut expected = raw;
        data_entries(&mut expected)
            .iter_mut()
            .find(|entry| entry.name == "initialized")
            .unwrap()
            .value = NbtSummaryValue::Byte(1);
        data_entries(&mut expected)
            .iter_mut()
            .find(|entry| entry.name == "ServerBrands")
            .unwrap()
            .value = strings(&["vanilla", "bbb-server"]);
        assert_eq!(patched, expected);
    }

    #[test]
    fn saved_data_pack_lists_replace_in_order_and_preserve_unknown_fields() {
        let mut raw = raw_level();
        data_packs_entries(&mut raw).push(entry(
            "unknown_pack_field",
            NbtSummaryValue::string("preserve pack metadata"),
        ));
        let data = Java26LevelData::parse(raw).unwrap();
        let selection = Java26DataPacks::new(
            vec![
                "file/zeta".to_string(),
                "arbitrary pack name".to_string(),
                "file/zeta".to_string(),
            ],
            vec!["file/disabled".to_string(), "file/disabled".to_string()],
        );

        let mut patched = data
            .raw_with_saved_state_and_data_packs(&save_state(), Some(&selection))
            .unwrap();
        let reparsed = Java26LevelData::parse(patched.clone()).unwrap();
        assert_eq!(reparsed.data_packs(), Some(&selection));
        assert!(data_entries(&mut patched)
            .iter()
            .any(|entry| entry.name == "unknown_bbb_fixture"));
        assert!(data_packs_entries(&mut patched).iter().any(|entry| {
            entry.name == "unknown_pack_field"
                && entry.value == NbtSummaryValue::string("preserve pack metadata")
        }));
    }

    #[test]
    fn optional_configuration_fields_accept_absence() {
        let mut raw = raw_level();
        data_entries(&mut raw).retain(|entry| {
            entry.name != "singleplayer_uuid"
                && entry.name != "removed_features"
                && entry.name != "DataPacks"
                && entry.name != "enabled_features"
        });
        let data = Java26LevelData::parse(raw).unwrap();
        assert_eq!(data.singleplayer_uuid(), None);
        assert_eq!(data.removed_features(), None);
        assert_eq!(data.data_packs(), None);
        assert_eq!(data.enabled_features(), None);
    }

    #[test]
    fn missing_fields_read_as_defaults_and_the_writer_inserts_them() {
        let mut raw = raw_level();
        data_entries(&mut raw).retain(|entry| {
            !matches!(
                entry.name.to_jdk_string().as_str(),
                "Time" | "GameType" | "spawn" | "difficulty_settings" | "DataPacks" | "Version"
            )
        });
        let data = Java26LevelData::parse(raw).unwrap();
        assert_eq!((data.game_time(), data.game_type()), (0, 0));
        assert_eq!(data.spawn().position(), [0, 0, 0]);
        assert_eq!(data.difficulty_settings().difficulty(), "normal");

        let packs = Java26DataPacks {
            enabled: vec!["vanilla".into()],
            disabled: Vec::new(),
        };
        let saved = Java26LevelData::parse(
            data.raw_with_saved_state_and_data_packs(&save_state(), Some(&packs))
                .unwrap(),
        )
        .unwrap();
        assert_eq!((saved.game_time(), saved.game_type()), (100, 0));
        assert_eq!(saved.spawn(), &save_state().spawn);
        assert_eq!(saved.difficulty_settings().difficulty(), "hard");
        assert!(saved.difficulty_settings().locked());
        assert_eq!(saved.data_packs(), Some(&packs));
    }

    #[test]
    fn old_and_new_data_versions_fail_before_level_publication() {
        for version in [JAVA_26_1_DATA_VERSION - 1, JAVA_26_1_DATA_VERSION + 1] {
            let mut raw = raw_level();
            data_entries(&mut raw)
                .iter_mut()
                .find(|entry| entry.name == "DataVersion")
                .unwrap()
                .value = NbtSummaryValue::Int(version);
            assert!(matches!(
                Java26LevelData::parse(raw),
                Err(SaveError::UnsupportedDataVersion { actual, .. }) if actual == version
            ));
        }
    }

    #[test]
    #[ignore = "requires BBB_TEST_JAVA_26_1_WORLD from an authorized official-server run"]
    fn verified_official_java_26_1_level_dat_matches_frozen_dto() {
        let root = std::env::var_os("BBB_TEST_JAVA_26_1_WORLD")
            .expect("BBB_TEST_JAVA_26_1_WORLD must name an official 26.1 world");
        let paths = bbb_server_types::save::WorldSavePaths::new(root).unwrap();
        let store =
            LevelDatStore::new(paths, bbb_server_types::save::WorldSaveLimits::default()).unwrap();

        let loaded = store.load_java_26_1().unwrap();

        assert_eq!(loaded.source, LevelDatSource::Primary);
        assert!(!loaded.data.level_name().is_empty());
        assert!(!loaded.data.server_brands().is_empty());
        assert_eq!(loaded.data.spawn().dimension(), "minecraft:overworld");
        assert!(loaded.data.data_packs().is_some());
    }
}
