//! `StatsCounter` / `ServerStatsCounter` (`net/minecraft/stats/`): the per-player
//! counters, the file they live in, and the packet they are announced with.
//!
//! # Why there is no table here
//!
//! A statistic is a pair: a `StatType` out of `minecraft:stat_type` and a value
//! out of whichever registry that type names (`StatType.java:13`, `:36-38`).
//! Both halves are already generated -- `bbb_protocol::stat_types` carries all
//! nine types with their value registries and all seventy-seven custom stats,
//! derived from `Stats.java` -- so nothing in this module writes an id down.
//! What it writes down is the *shape*: how a pair becomes a JSON key, how a
//! count saturates, and which counters this server can honestly move.
//!
//! # The on-disk form
//!
//! `ServerStatsCounter.toJson` (`ServerStatsCounter.java:115-120`) writes two
//! top-level members: `stats`, a two-level object keyed by the stat type's
//! `Identifier` and then by the value's (`:44-51`, `:55-64`), and
//! `DataVersion`. Both keys are the plain colon form -- `minecraft:mined`,
//! `minecraft:stone` -- and not the dotted `Stat.buildName` form
//! (`Stat.java:27-33`), which is the scoreboard criterion name and appears
//! nowhere in the file.
//!
//! Two differences from vanilla's bytes, neither of which the official server
//! can observe when it reads the file back:
//!
//! * **Key order.** Vanilla's two levels are `HashMap`s (`:48`, and
//!   `Codec.unboundedMap`), so its order is hash order; this writes both levels
//!   sorted, because a save this server produces has to be reproducible for the
//!   differential harness to compare two runs of it.
//! * **Member order.** Gson emits `stats` before `DataVersion` because its
//!   `JsonObject` keeps insertion order; `serde_json` sorts, so `DataVersion`
//!   comes first. A JSON object is unordered and both readers look the members
//!   up by name.
//!
//! # What is deliberately not modelled
//!
//! * The datafixer. `ServerStatsCounter.parse` (`:104-113`) reads
//!   `DataVersion` with the default `1343` and runs the whole `DataFixTypes.STATS`
//!   chain, which is what turns a 1.12 `stat.mineBlock.minecraft.stone` key into
//!   `minecraft:mined`/`minecraft:stone` (`StatsCounterFix.java:191-228`). This
//!   crate has no datafixer at all -- the same gap every other save surface
//!   here has -- so a file older than the current data version is refused
//!   rather than upgraded.
//! * Every counter whose input this server does not compute. The movement
//!   families (`ServerPlayer.java:1420-1490`), crafting, pickup, hand/menu
//!   `Stats.ITEM_DROPPED`, mob `Stats.ENTITY_KILLED`, and the kill counters
//!   all award now. `Stats.ENTITY_KILLED` does not award for a player victim
//!   because `ServerPlayer.die` (`:869`) replaces `LivingEntity.die` without a
//!   `super` call, so the `sourceEntity.killedEntity` leg
//!   (`LivingEntity.java:1446`) that `Player.killedEntity` awards it from
//!   (`Player.java:1486-1487`) never runs for a player victim.

use std::collections::{BTreeMap, BTreeSet};

use bbb_protocol::entity_types::vanilla_entity_type;
use bbb_protocol::stat_types::{
    vanilla_custom_stat_by_id, vanilla_custom_stat_by_resource_id, vanilla_stat_type_by_id,
    vanilla_stat_type_by_resource_id, VANILLA_STAT_TYPE_CUSTOM_ID,
};
use serde_json::{json, Map, Value};

use crate::save::JAVA_26_1_DATA_VERSION;

/// The value registry ids `bbb_protocol::stat_types` names, as the four this
/// server can resolve both ways.
const REGISTRY_BLOCK: &str = "minecraft:block";
const REGISTRY_ITEM: &str = "minecraft:item";
const REGISTRY_ENTITY_TYPE: &str = "minecraft:entity_type";
const REGISTRY_CUSTOM_STAT: &str = "minecraft:custom_stat";

/// One `Stat<T>` (`net/minecraft/stats/Stat.java:13-25`) as a pair of registry
/// ids: the type's own id in `minecraft:stat_type`, and the value's id in the
/// registry that type names.
///
/// Vanilla interns one `Stat` object per pair (`StatType.java:32-34`) and keys
/// the counter map on object identity through `equals`, which compares the
/// built name (`Stat.java:48-50`); the pair is that identity without the
/// interning.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct StatKey {
    pub stat_type_id: i32,
    pub value_id: i32,
}

impl StatKey {
    /// The pair `StatType.get(T)` (`StatType.java:45-47`) would intern.
    pub const fn new(stat_type_id: i32, value_id: i32) -> Self {
        Self {
            stat_type_id,
            value_id,
        }
    }

    /// `Stats.CUSTOM.get(location)` (`Player.java:1347`), the sugar the two
    /// `Identifier` overloads of `awardStat` go through.
    pub const fn custom(custom_stat_id: i32) -> Self {
        Self::new(VANILLA_STAT_TYPE_CUSTOM_ID, custom_stat_id)
    }

    /// The two `Identifier`s this pair serializes as, outer then inner.
    ///
    /// `None` for a pair naming a registry this crate cannot resolve or an id
    /// outside it, which is refused rather than written: a stats file with a
    /// key the official server cannot look up is a file it drops on load
    /// (`Registry.java:40`, through `ServerStatsCounter.java:110`).
    pub fn resource_ids(&self) -> Option<(&'static str, &'static str)> {
        let stat_type = vanilla_stat_type_by_id(self.stat_type_id)?;
        let value = match stat_type.value_registry {
            REGISTRY_BLOCK => bbb_protocol::block_types::vanilla_block_type(self.value_id)
                .map(|entry| entry.resource_id)?,
            REGISTRY_ITEM => bbb_protocol::item_types::vanilla_item_type(self.value_id)
                .map(|entry| entry.resource_id)?,
            REGISTRY_ENTITY_TYPE => vanilla_entity_type(self.value_id)?.resource_id,
            REGISTRY_CUSTOM_STAT => {
                vanilla_custom_stat_by_id(self.value_id).map(|entry| entry.resource_id)?
            }
            _ => return None,
        };
        Some((stat_type.resource_id, value))
    }

    /// `ContentRegistry`-aware variant of [`Self::resource_ids`].
    pub fn resource_ids_with_registry<'a>(
        &self,
        registry: &'a bbb_protocol::content_registry::ContentRegistry,
    ) -> Option<(&'static str, &'a str)> {
        let stat_type = vanilla_stat_type_by_id(self.stat_type_id)?;
        let value = match stat_type.value_registry {
            REGISTRY_BLOCK => bbb_protocol::block_types::vanilla_block_type(self.value_id)
                .map(|entry| entry.resource_id)
                .or_else(|| {
                    registry
                        .block_by_id(self.value_id)
                        .map(bbb_protocol::content_registry::ModBlockSpec::resource_id)
                })?,
            REGISTRY_ITEM => bbb_protocol::item_types::vanilla_item_type(self.value_id)
                .map(|entry| entry.resource_id)
                .or_else(|| {
                    registry
                        .item_by_id(self.value_id)
                        .map(bbb_protocol::content_registry::ModItemSpec::resource_id)
                })?,
            REGISTRY_ENTITY_TYPE => vanilla_entity_type(self.value_id)
                .map(|entry| entry.resource_id)
                .or_else(|| {
                    registry
                        .entity_by_id(self.value_id)
                        .map(bbb_protocol::content_registry::ModEntityTypeSpec::resource_id)
                })?,
            REGISTRY_CUSTOM_STAT => {
                vanilla_custom_stat_by_id(self.value_id).map(|entry| entry.resource_id)?
            }
            _ => return None,
        };
        Some((stat_type.resource_id, value))
    }

    /// The inverse of [`Self::resource_ids`], which is what the two
    /// `byNameCodec`s do on the way in (`ServerStatsCounter.java:45`, `:56`).
    pub fn from_resource_ids(stat_type: &str, value: &str) -> Option<Self> {
        Self::from_resource_ids_with_registry(
            stat_type,
            value,
            bbb_protocol::content_registry::ContentRegistry::vanilla(),
        )
    }

    /// `ContentRegistry`-aware inverse of [`Self::resource_ids_with_registry`].
    pub fn from_resource_ids_with_registry(
        stat_type: &str,
        value: &str,
        registry: &bbb_protocol::content_registry::ContentRegistry,
    ) -> Option<Self> {
        let stat_type = vanilla_stat_type_by_resource_id(stat_type)?;
        let value_id = match stat_type.value_registry {
            REGISTRY_BLOCK => bbb_protocol::block_types::vanilla_block_type_for_resource_id(value)
                .map(|entry| entry.id)
                .or_else(|| {
                    registry
                        .block_by_resource_id(value)
                        .map(|spec| spec.block_id)
                })?,
            REGISTRY_ITEM => bbb_protocol::item_types::vanilla_item_type_for_resource_id(value)
                .map(|entry| entry.id)
                .or_else(|| registry.item_by_resource_id(value).map(|spec| spec.id))?,
            REGISTRY_ENTITY_TYPE => {
                bbb_protocol::entity_types::vanilla_entity_type_id_for_resource_id(value)
                    .or_else(|| registry.entity_by_resource_id(value).map(|spec| spec.id))?
            }
            REGISTRY_CUSTOM_STAT => vanilla_custom_stat_by_resource_id(value)?.id,
            _ => return None,
        };
        Some(Self::new(stat_type.id, value_id))
    }
}

/// The custom statistics this server can honestly move, named the way
/// `Stats.java` names them rather than by a registry index.
///
/// `minecraft:custom_stat` is a registry whose ids are declaration order
/// (`Stats.java:21-97`), and the generated catalog carries all seventy-seven of
/// them; these are the ids a *caller* here needs, so what is written down is
/// the resource id -- the thing `makeCustomStat` builds the registry key from
/// (`Stats.java:100`) -- and the number is looked up.
pub const CUSTOM_STAT_LEAVE_GAME: &str = "minecraft:leave_game";
pub const CUSTOM_STAT_PLAY_TIME: &str = "minecraft:play_time";
pub const CUSTOM_STAT_TOTAL_WORLD_TIME: &str = "minecraft:total_world_time";
pub const CUSTOM_STAT_TIME_SINCE_DEATH: &str = "minecraft:time_since_death";
pub const CUSTOM_STAT_TIME_SINCE_REST: &str = "minecraft:time_since_rest";
/// `Stats.CROUCH_TIME` (`Stats.java:26`), whose field name and registry id
/// disagree: the constant is `CROUCH_TIME` and the id is `sneak_time`.
pub const CUSTOM_STAT_SNEAK_TIME: &str = "minecraft:sneak_time";
pub const CUSTOM_STAT_DEATHS: &str = "minecraft:deaths";

/// `Stats.CUSTOM.get(Identifier)` (`Player.java:1347`) for one of the ids
/// above.
///
/// It panics for an id the generated catalog has no row for, which is a
/// build-time fact rather than a runtime one: the constants above are checked
/// against the catalog by this module's own tests, so a rename fails there
/// first.
pub fn custom_stat(resource_id: &str) -> StatKey {
    let entry = vanilla_custom_stat_by_resource_id(resource_id)
        .unwrap_or_else(|| panic!("{resource_id} is not a built-in 26.1 custom statistic"));
    StatKey::custom(entry.id)
}

/// What went wrong reading a `players/stats/<uuid>.json`.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum StatsParseError {
    #[error("statistics file is not a JSON object")]
    NotAnObject,
    #[error(
        "statistics file DataVersion {0} is not the 26.1 data version {JAVA_26_1_DATA_VERSION}"
    )]
    DataVersion(i64),
}

/// `ServerStatsCounter` (`net/minecraft/stats/ServerStatsCounter.java:41-135`)
/// over its base (`StatsCounter.java:8-31`).
///
/// The two are one struct here because the base has one non-test caller in the
/// whole tree -- the client's own counter (`ClientPacketListener.java:520`) --
/// and this crate is the server.
///
/// The map is ordered rather than hashed for the reason this module's own
/// documentation gives: the file it writes has to be reproducible.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ServerStatsCounter {
    /// `StatsCounter.stats` (`StatsCounter.java:9`), whose default return value
    /// is zero (`:12`) -- which is what a missing key means here.
    stats: BTreeMap<StatKey, i32>,
    /// `ServerStatsCounter.dirty` (`:53`).
    dirty: BTreeSet<StatKey>,
}

impl ServerStatsCounter {
    /// A counter with nothing in it, which is what
    /// `new ServerStatsCounter(server, file)` (`:66-78`) leaves behind for a
    /// player the stats directory holds no file for.
    pub fn new() -> Self {
        Self::default()
    }

    /// `StatsCounter.getValue(Stat)` (`StatsCounter.java:28-30`).
    pub fn value(&self, stat: StatKey) -> i32 {
        self.stats.get(&stat).copied().unwrap_or(0)
    }

    /// `StatsCounter.increment(Player, Stat, int)`
    /// (`StatsCounter.java:15-18`): the sum is taken in `long` and saturated at
    /// `Integer.MAX_VALUE`, so a counter never wraps.
    ///
    /// It routes through [`Self::set_value`], which is what makes every award
    /// mark the stat dirty (`ServerStatsCounter.java:93-96`).
    pub fn increment(&mut self, stat: StatKey, count: i32) {
        let result = (i64::from(self.value(stat)) + i64::from(count)).min(i64::from(i32::MAX));
        self.set_value(stat, result as i32);
    }

    /// `ServerStatsCounter.setValue` (`:92-96`) over
    /// `StatsCounter.setValue` (`StatsCounter.java:20-22`).
    pub fn set_value(&mut self, stat: StatKey, count: i32) {
        self.stats.insert(stat, count);
        self.dirty.insert(stat);
    }

    /// `ServerStatsCounter.markAllDirty()` (`:122-124`), which
    /// `PlayerList.placeNewPlayer` runs on every join (`PlayerList.java:189`).
    pub fn mark_all_dirty(&mut self) {
        self.dirty.extend(self.stats.keys().copied());
    }

    /// `ServerStatsCounter.getDirty()` (`:98-102`) followed by the map
    /// `sendStats` builds out of it (`:126-133`): the dirty set is copied and
    /// cleared, so the next call reports nothing until something moves again.
    pub fn take_dirty(&mut self) -> Vec<(StatKey, i32)> {
        std::mem::take(&mut self.dirty)
            .into_iter()
            .map(|stat| (stat, self.value(stat)))
            .collect()
    }

    /// Whether anything at all has been counted, which is what decides whether
    /// this server writes a file for a player.
    pub fn is_empty(&self) -> bool {
        self.stats.is_empty()
    }

    /// `ServerStatsCounter.toJson()` (`:115-120`).
    ///
    /// A pair whose ids this crate cannot name is dropped rather than written,
    /// for the reason [`StatKey::resource_ids`] gives. It cannot occur for a
    /// pair this crate produced -- every award goes through a generated id --
    /// so it only guards a counter parsed from a file, and that path already
    /// dropped the unknown key on the way in.
    pub fn to_json(&self) -> Value {
        self.to_json_with_registry(bbb_protocol::content_registry::ContentRegistry::vanilla())
    }

    /// `ContentRegistry`-aware variant of [`Self::to_json`].
    pub fn to_json_with_registry(
        &self,
        registry: &bbb_protocol::content_registry::ContentRegistry,
    ) -> Value {
        let mut grouped: Map<String, Value> = Map::new();
        for (stat, count) in &self.stats {
            let Some((stat_type, value)) = stat.resource_ids_with_registry(registry) else {
                continue;
            };
            let entry = grouped
                .entry(stat_type.to_string())
                .or_insert_with(|| Value::Object(Map::new()));
            if let Value::Object(values) = entry {
                values.insert(value.to_owned(), json!(count));
            }
        }
        json!({
            "stats": Value::Object(grouped),
            "DataVersion": JAVA_26_1_DATA_VERSION,
        })
    }

    /// The bytes `ServerStatsCounter.save()` writes (`:80-90`): the pretty
    /// printer Gson is built with (`:42`), UTF-8, no trailing newline.
    pub fn to_file_bytes(&self) -> Vec<u8> {
        self.to_file_bytes_with_registry(bbb_protocol::content_registry::ContentRegistry::vanilla())
    }

    /// `ContentRegistry`-aware variant of [`Self::to_file_bytes`].
    pub fn to_file_bytes_with_registry(
        &self,
        registry: &bbb_protocol::content_registry::ContentRegistry,
    ) -> Vec<u8> {
        serde_json::to_vec_pretty(&self.to_json_with_registry(registry))
            .expect("a stats document is always serializable")
    }

    /// `ServerStatsCounter.parse` (`:104-113`), minus the datafixer this crate
    /// has none of. An entry the codec cannot decode -- an id outside the
    /// built-in registries or a non-number count -- is dropped
    /// (`resultOrPartial`), and a `stats` member that is not an object is empty.
    pub fn parse(document: &Value) -> Result<Self, StatsParseError> {
        Self::parse_with_registry(
            document,
            bbb_protocol::content_registry::ContentRegistry::vanilla(),
        )
    }

    /// `ContentRegistry`-aware variant of [`Self::parse`].
    pub fn parse_with_registry(
        document: &Value,
        registry: &bbb_protocol::content_registry::ContentRegistry,
    ) -> Result<Self, StatsParseError> {
        let Value::Object(root) = document else {
            return Err(StatsParseError::NotAnObject);
        };
        // `:106` reads the version with the default `1343`, the 1.12.2 one, and
        // hands it to the fixer chain. With no fixer, anything but the current
        // version is a file this crate must not pretend to understand.
        let data_version = match root.get("DataVersion") {
            Some(value) => value
                .as_i64()
                .ok_or(StatsParseError::DataVersion(i64::from(i32::MIN)))?,
            None => 1343,
        };
        if data_version != i64::from(JAVA_26_1_DATA_VERSION) {
            return Err(StatsParseError::DataVersion(data_version));
        }
        let mut stats = BTreeMap::new();
        if let Some(Value::Object(groups)) = root.get("stats") {
            for (stat_type, values) in groups {
                let Value::Object(values) = values else {
                    continue;
                };
                for (value, count) in values {
                    // `Codec.INT` through `JsonOps.getNumberValue`.
                    let count = count
                        .as_i64()
                        .map(|count| count as i32)
                        .or_else(|| count.as_f64().map(|count| count as i32));
                    if let (Some(stat), Some(count)) = (
                        StatKey::from_resource_ids_with_registry(stat_type, value, registry),
                        count,
                    ) {
                        stats.insert(stat, count);
                    }
                }
            }
        }
        Ok(Self {
            stats,
            dirty: BTreeSet::new(),
        })
    }

    /// The same read over the file's bytes.
    pub fn parse_bytes(bytes: &[u8]) -> Result<Self, StatsParseError> {
        Self::parse_bytes_with_registry(
            bytes,
            bbb_protocol::content_registry::ContentRegistry::vanilla(),
        )
    }

    /// `ContentRegistry`-aware variant of [`Self::parse_bytes`].
    pub fn parse_bytes_with_registry(
        bytes: &[u8],
        registry: &bbb_protocol::content_registry::ContentRegistry,
    ) -> Result<Self, StatsParseError> {
        let document: Value =
            serde_json::from_slice(bytes).map_err(|_| StatsParseError::NotAnObject)?;
        Self::parse_with_registry(&document, registry)
    }
}

#[cfg(test)]
mod tests;
