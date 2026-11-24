//! `TrialSpawnerConfig` and `TrialSpawner.FullConfig` persisted codecs and target formulas.

use bbb_protocol::nbt_summary::codec_double_value as numeric_f64;
use bbb_protocol::packets::{BlockPos, Vec3d};
use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue};

use crate::random::GameRandom;

use super::super::base_spawner::{read_potentials, SpawnerSpawnData, WeightedSpawnerSpawnData};
use super::super::{entry, unique_field};
use super::numeric_i64;

/// `TrialSpawner.FullConfig.MAP_CODEC` (`TrialSpawner.java:379-389`).
pub(super) const TAG_NORMAL_CONFIG: &str = "normal_config";
pub(super) const TAG_OMINOUS_CONFIG: &str = "ominous_config";
pub(super) const TAG_TARGET_COOLDOWN_LENGTH: &str = "target_cooldown_length";
const TAG_REQUIRED_PLAYER_RANGE: &str = "required_player_range";

pub(super) const CONFIG_TAGS: &[&str] = &[
    TAG_NORMAL_CONFIG,
    TAG_OMINOUS_CONFIG,
    TAG_TARGET_COOLDOWN_LENGTH,
    TAG_REQUIRED_PLAYER_RANGE,
];

/// `TrialSpawnerConfig.DIRECT_CODEC` (`TrialSpawnerConfig.java:20-53`).
const TAG_SPAWN_RANGE: &str = "spawn_range";
const TAG_TOTAL_MOBS: &str = "total_mobs";
const TAG_SIMULTANEOUS_MOBS: &str = "simultaneous_mobs";
const TAG_TOTAL_MOBS_ADDED_PER_PLAYER: &str = "total_mobs_added_per_player";
const TAG_SIMULTANEOUS_MOBS_ADDED_PER_PLAYER: &str = "simultaneous_mobs_added_per_player";
#[cfg(test)]
pub(super) const TAG_TICKS_BETWEEN_SPAWN: &str = "ticks_between_spawn";
#[cfg(not(test))]
const TAG_TICKS_BETWEEN_SPAWN: &str = "ticks_between_spawn";
#[cfg(test)]
pub(super) const TAG_SPAWN_POTENTIALS: &str = "spawn_potentials";
#[cfg(not(test))]
const TAG_SPAWN_POTENTIALS: &str = "spawn_potentials";
#[cfg(test)]
pub(super) const TAG_LOOT_TABLES_TO_EJECT: &str = "loot_tables_to_eject";
#[cfg(not(test))]
const TAG_LOOT_TABLES_TO_EJECT: &str = "loot_tables_to_eject";
#[cfg(test)]
pub(super) const TAG_ITEMS_TO_DROP_WHEN_OMINOUS: &str = "items_to_drop_when_ominous";
#[cfg(not(test))]
const TAG_ITEMS_TO_DROP_WHEN_OMINOUS: &str = "items_to_drop_when_ominous";

/// `TrialSpawnerConfig.Builder` (`TrialSpawnerConfig.java:101-118`) reads these
/// two built-in loot tables through `BuiltInLootTables` (`:110-111`, `:114`).
const DEFAULT_LOOT_TABLES_TO_EJECT: &[&str] = &[
    "minecraft:spawners/trial_chamber/consumables",
    "minecraft:spawners/trial_chamber/key",
];
#[cfg(test)]
pub(super) const DEFAULT_ITEMS_TO_DROP_WHEN_OMINOUS: &str =
    "minecraft:spawners/trial_chamber/items_to_drop_when_ominous";
#[cfg(not(test))]
const DEFAULT_ITEMS_TO_DROP_WHEN_OMINOUS: &str =
    "minecraft:spawners/trial_chamber/items_to_drop_when_ominous";

/// `TrialSpawnerConfig` (`TrialSpawnerConfig.java`).
#[derive(Debug, Clone, PartialEq)]
pub struct TrialSpawnerConfig {
    spawn_range: i32,
    total_mobs: f32,
    simultaneous_mobs: f32,
    total_mobs_added_per_player: f32,
    simultaneous_mobs_added_per_player: f32,
    ticks_between_spawn: i32,
    spawn_potentials: Vec<WeightedSpawnerSpawnData>,
    loot_tables_to_eject: Vec<WeightedLootTable>,
    items_to_drop_when_ominous: String,
    /// Fields no codec in this slice owns, kept for a save round-trip.
    remaining: Vec<NbtSummaryEntry>,
}

/// `WeightedList<ResourceKey<LootTable>>` (`TrialSpawnerConfig.java`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WeightedLootTable {
    pub(super) resource_id: String,
    pub(super) weight: i32,
}

impl WeightedLootTable {
    #[must_use]
    pub fn resource_id(&self) -> &str {
        &self.resource_id
    }

    #[must_use]
    pub const fn weight(&self) -> i32 {
        self.weight
    }
}

impl Default for TrialSpawnerConfig {
    /// `TrialSpawnerConfig.DEFAULT` (`TrialSpawnerConfig.java`).
    fn default() -> Self {
        Self {
            spawn_range: 4,
            total_mobs: 6.0,
            simultaneous_mobs: 2.0,
            total_mobs_added_per_player: 2.0,
            simultaneous_mobs_added_per_player: 1.0,
            ticks_between_spawn: 40,
            spawn_potentials: Vec::new(),
            loot_tables_to_eject: DEFAULT_LOOT_TABLES_TO_EJECT
                .iter()
                .map(|resource_id| WeightedLootTable {
                    resource_id: (*resource_id).to_string(),
                    weight: 1,
                })
                .collect(),
            items_to_drop_when_ominous: DEFAULT_ITEMS_TO_DROP_WHEN_OMINOUS.to_string(),
            remaining: Vec::new(),
        }
    }
}

impl TrialSpawnerConfig {
    /// `TrialSpawnerConfig.DIRECT_CODEC` (`:20-53`): every scalar is an
    /// `optionalFieldOf` with the record's own default, and the two range
    /// codecs reject out-of-range values rather than clamping.
    #[must_use]
    pub fn from_nbt(value: &NbtSummaryValue) -> Option<Self> {
        let NbtSummaryValue::Compound(fields) = value else {
            return None;
        };
        let default = Self::default();
        let mut config = Self {
            spawn_range: optional_bounded_int(
                fields,
                TAG_SPAWN_RANGE,
                1,
                128,
                default.spawn_range,
            )?,
            total_mobs: optional_non_negative_float(fields, TAG_TOTAL_MOBS, default.total_mobs)?,
            simultaneous_mobs: optional_non_negative_float(
                fields,
                TAG_SIMULTANEOUS_MOBS,
                default.simultaneous_mobs,
            )?,
            total_mobs_added_per_player: optional_non_negative_float(
                fields,
                TAG_TOTAL_MOBS_ADDED_PER_PLAYER,
                default.total_mobs_added_per_player,
            )?,
            simultaneous_mobs_added_per_player: optional_non_negative_float(
                fields,
                TAG_SIMULTANEOUS_MOBS_ADDED_PER_PLAYER,
                default.simultaneous_mobs_added_per_player,
            )?,
            ticks_between_spawn: optional_bounded_int(
                fields,
                TAG_TICKS_BETWEEN_SPAWN,
                0,
                i32::MAX,
                default.ticks_between_spawn,
            )?,
            spawn_potentials: match unique_field(fields, TAG_SPAWN_POTENTIALS) {
                Some(value) => read_potentials(value)?,
                None => Vec::new(),
            },
            loot_tables_to_eject: match unique_field(fields, TAG_LOOT_TABLES_TO_EJECT) {
                Some(value) => read_weighted_loot_tables(value)?,
                None => default.loot_tables_to_eject,
            },
            items_to_drop_when_ominous: match unique_field(fields, TAG_ITEMS_TO_DROP_WHEN_OMINOUS) {
                Some(value) => {
                    let NbtSummaryValue::String(resource_id) = value else {
                        return None;
                    };
                    bbb_protocol::normalize_resource_location(resource_id.try_to_string().ok()?)
                        .ok()?
                }
                None => default.items_to_drop_when_ominous,
            },
            remaining: Vec::new(),
        };
        config.remaining = fields
            .iter()
            .filter(|field| {
                !field
                    .name
                    .with_utf8(|name| {
                        matches!(
                            name,
                            TAG_SPAWN_RANGE
                                | TAG_TOTAL_MOBS
                                | TAG_SIMULTANEOUS_MOBS
                                | TAG_TOTAL_MOBS_ADDED_PER_PLAYER
                                | TAG_SIMULTANEOUS_MOBS_ADDED_PER_PLAYER
                                | TAG_TICKS_BETWEEN_SPAWN
                                | TAG_SPAWN_POTENTIALS
                                | TAG_LOOT_TABLES_TO_EJECT
                                | TAG_ITEMS_TO_DROP_WHEN_OMINOUS
                        )
                    })
                    .unwrap_or(false)
            })
            .cloned()
            .collect();
        Some(config)
    }

    #[must_use]
    pub fn to_nbt(&self) -> NbtSummaryValue {
        let default = Self::default();
        let mut fields = Vec::new();
        if self.spawn_range != default.spawn_range {
            fields.push(entry(
                TAG_SPAWN_RANGE,
                NbtSummaryValue::Int(self.spawn_range),
            ));
        }
        for (name, value, default) in [
            (TAG_TOTAL_MOBS, self.total_mobs, default.total_mobs),
            (
                TAG_SIMULTANEOUS_MOBS,
                self.simultaneous_mobs,
                default.simultaneous_mobs,
            ),
            (
                TAG_TOTAL_MOBS_ADDED_PER_PLAYER,
                self.total_mobs_added_per_player,
                default.total_mobs_added_per_player,
            ),
            (
                TAG_SIMULTANEOUS_MOBS_ADDED_PER_PLAYER,
                self.simultaneous_mobs_added_per_player,
                default.simultaneous_mobs_added_per_player,
            ),
        ] {
            if value != default {
                fields.push(entry(name, NbtSummaryValue::Float(value.to_bits())));
            }
        }
        if self.ticks_between_spawn != default.ticks_between_spawn {
            fields.push(entry(
                TAG_TICKS_BETWEEN_SPAWN,
                NbtSummaryValue::Int(self.ticks_between_spawn),
            ));
        }
        if !self.spawn_potentials.is_empty() {
            fields.push(entry(
                TAG_SPAWN_POTENTIALS,
                NbtSummaryValue::List(
                    self.spawn_potentials
                        .iter()
                        .map(|potential| {
                            NbtSummaryValue::Compound(vec![
                                entry("data", potential.data.to_nbt()),
                                entry("weight", NbtSummaryValue::Int(potential.weight)),
                            ])
                        })
                        .collect(),
                ),
            ));
        }
        if self.loot_tables_to_eject != default.loot_tables_to_eject {
            fields.push(entry(
                TAG_LOOT_TABLES_TO_EJECT,
                NbtSummaryValue::List(
                    self.loot_tables_to_eject
                        .iter()
                        .map(|table| {
                            NbtSummaryValue::Compound(vec![
                                entry("data", NbtSummaryValue::string(table.resource_id.clone())),
                                entry("weight", NbtSummaryValue::Int(table.weight)),
                            ])
                        })
                        .collect(),
                ),
            ));
        }
        if self.items_to_drop_when_ominous != default.items_to_drop_when_ominous {
            fields.push(entry(
                TAG_ITEMS_TO_DROP_WHEN_OMINOUS,
                NbtSummaryValue::string(self.items_to_drop_when_ominous.clone()),
            ));
        }
        fields.extend(self.remaining.iter().cloned());
        NbtSummaryValue::Compound(fields)
    }

    #[must_use]
    pub const fn spawn_range(&self) -> i32 {
        self.spawn_range
    }

    #[must_use]
    pub const fn ticks_between_spawn(&self) -> i32 {
        self.ticks_between_spawn
    }

    /// `TrialSpawner.spawnMob`'s position (`TrialSpawner.java`): the
    /// spawn data's own `Pos`, else the two draws around the spawner in
    /// `x`, `y`, `z` order.
    #[must_use]
    pub fn spawn_position(
        &self,
        spawner_position: BlockPos,
        spawn_data: &SpawnerSpawnData,
        random: &GameRandom,
    ) -> Option<Vec3d> {
        if let Some(position) = spawn_data.explicit_position() {
            const MAX_EXPLICIT_SPAWN_OFFSET: f64 = 128.0;
            let dx = position.x - (f64::from(spawner_position.x) + 0.5);
            let dy = position.y - (f64::from(spawner_position.y) + 0.5);
            let dz = position.z - (f64::from(spawner_position.z) + 0.5);
            if !position.x.is_finite()
                || !position.y.is_finite()
                || !position.z.is_finite()
                || dx.abs() > MAX_EXPLICIT_SPAWN_OFFSET
                || dy.abs() > MAX_EXPLICIT_SPAWN_OFFSET
                || dz.abs() > MAX_EXPLICIT_SPAWN_OFFSET
            {
                return None;
            }
            return Some(position);
        }
        Some(Vec3d {
            x: f64::from(spawner_position.x)
                + (random.next_double() - random.next_double()) * f64::from(self.spawn_range)
                + 0.5,
            y: f64::from(spawner_position.y) + f64::from(random.next_int(3)) - 1.0,
            z: f64::from(spawner_position.z)
                + (random.next_double() - random.next_double()) * f64::from(self.spawn_range)
                + 0.5,
        })
    }

    #[must_use]
    pub fn spawn_potentials(&self) -> &[WeightedSpawnerSpawnData] {
        &self.spawn_potentials
    }

    #[must_use]
    pub fn loot_tables_to_eject(&self) -> &[WeightedLootTable] {
        &self.loot_tables_to_eject
    }

    #[must_use]
    pub fn items_to_drop_when_ominous(&self) -> &str {
        &self.items_to_drop_when_ominous
    }

    /// `TrialSpawnerConfig.withSpawning` (`:118-133`): one weighted potential
    /// whose entity tag names the type, every other field untouched.
    #[must_use]
    pub fn with_spawning(&self, resource_id: &str) -> Self {
        let spawn_data = SpawnerSpawnData::new(
            vec![entry("id", NbtSummaryValue::string(resource_id))],
            None,
            None,
        );
        Self {
            spawn_potentials: vec![WeightedSpawnerSpawnData {
                data: spawn_data,
                weight: 1,
            }],
            ..self.clone()
        }
    }

    /// `calculateTargetTotalMobs` (`:57-59`).
    #[must_use]
    pub fn calculate_target_total_mobs(&self, additional_players: i32) -> i32 {
        (self.total_mobs + self.total_mobs_added_per_player * additional_players as f32).floor()
            as i32
    }

    /// `calculateTargetSimultaneousMobs` (`:61-63`).
    #[must_use]
    pub fn calculate_target_simultaneous_mobs(&self, additional_players: i32) -> i32 {
        (self.simultaneous_mobs
            + self.simultaneous_mobs_added_per_player * additional_players as f32)
            .floor() as i32
    }
}

/// `Holder<TrialSpawnerConfig>` as `TrialSpawnerConfig.CODEC`
/// (`TrialSpawnerConfig.java:56`) reads it: an inline config or a
/// `minecraft:trial_spawner` registry id. A reference keeps its id for the
/// save and reads as `DEFAULT` until the level resolves it.
#[derive(Debug, Clone, PartialEq)]
enum TrialSpawnerConfigHolder {
    Direct(TrialSpawnerConfig),
    Reference {
        id: String,
        resolved: bool,
        value: TrialSpawnerConfig,
    },
}

impl Default for TrialSpawnerConfigHolder {
    fn default() -> Self {
        Self::Direct(TrialSpawnerConfig::default())
    }
}

impl TrialSpawnerConfigHolder {
    fn from_nbt(value: &NbtSummaryValue) -> Option<Self> {
        match value {
            NbtSummaryValue::String(id) => Some(Self::Reference {
                id: id
                    .try_to_string()
                    .ok()
                    .and_then(|raw| bbb_protocol::normalize_resource_location(raw).ok())?,
                resolved: false,
                value: TrialSpawnerConfig::default(),
            }),
            value => TrialSpawnerConfig::from_nbt(value).map(Self::Direct),
        }
    }

    /// `optionalFieldOf(..., Holder.direct(DEFAULT))` omits only the default
    /// direct holder; a reference always writes its id.
    fn to_nbt(&self) -> Option<NbtSummaryValue> {
        match self {
            Self::Direct(config) => {
                (*config != TrialSpawnerConfig::default()).then(|| config.to_nbt())
            }
            Self::Reference { id, .. } => Some(NbtSummaryValue::string(id.clone())),
        }
    }

    const fn value(&self) -> &TrialSpawnerConfig {
        match self {
            Self::Direct(value) | Self::Reference { value, .. } => value,
        }
    }

    fn resolve(&mut self, lookup: &impl Fn(&str) -> Option<TrialSpawnerConfig>) {
        if let Self::Reference {
            id,
            resolved: resolved @ false,
            value,
        } = self
        {
            if let Some(config) = lookup(id) {
                *value = config;
            }
            *resolved = true;
        }
    }

    const fn is_unresolved(&self) -> bool {
        matches!(
            self,
            Self::Reference {
                resolved: false,
                ..
            }
        )
    }
}

/// `TrialSpawner.FullConfig` (`TrialSpawner.java`).
#[derive(Debug, Clone, PartialEq)]
pub struct TrialSpawnerFullConfig {
    normal: TrialSpawnerConfigHolder,
    ominous: TrialSpawnerConfigHolder,
    target_cooldown_length: i32,
    required_player_range: i32,
}

impl Default for TrialSpawnerFullConfig {
    /// `TrialSpawner.FullConfig.DEFAULT` (`:393-395`).
    fn default() -> Self {
        Self {
            normal: TrialSpawnerConfigHolder::default(),
            ominous: TrialSpawnerConfigHolder::default(),
            target_cooldown_length: Self::DEFAULT_TARGET_COOLDOWN_LENGTH,
            required_player_range: Self::DEFAULT_REQUIRED_PLAYER_RANGE,
        }
    }
}

impl TrialSpawnerFullConfig {
    pub const DEFAULT_TARGET_COOLDOWN_LENGTH: i32 = 36_000;
    pub const DEFAULT_REQUIRED_PLAYER_RANGE: i32 = 14;

    /// `TrialSpawner.FullConfig.MAP_CODEC` (`:380-388`): a config that fails its
    /// own codec fails the whole map, which `TrialSpawner.load` (`:93`) answers
    /// with `DEFAULT`.
    #[must_use]
    pub fn from_nbt(fields: &[NbtSummaryEntry]) -> Option<Self> {
        let normal = match unique_field(fields, TAG_NORMAL_CONFIG) {
            Some(value) => TrialSpawnerConfigHolder::from_nbt(value)?,
            None => TrialSpawnerConfigHolder::default(),
        };
        let ominous = match unique_field(fields, TAG_OMINOUS_CONFIG) {
            Some(value) => TrialSpawnerConfigHolder::from_nbt(value)?,
            None => TrialSpawnerConfigHolder::default(),
        };
        let target_cooldown_length = optional_bounded_int(
            fields,
            TAG_TARGET_COOLDOWN_LENGTH,
            0,
            i32::MAX,
            Self::DEFAULT_TARGET_COOLDOWN_LENGTH,
        )?;
        let required_player_range = optional_bounded_int(
            fields,
            TAG_REQUIRED_PLAYER_RANGE,
            1,
            128,
            Self::DEFAULT_REQUIRED_PLAYER_RANGE,
        )?;
        Some(Self {
            normal,
            ominous,
            target_cooldown_length,
            required_player_range,
        })
    }

    #[must_use]
    pub fn to_nbt(&self) -> Vec<NbtSummaryEntry> {
        let mut fields = Vec::new();
        if let Some(normal) = self.normal.to_nbt() {
            fields.push(entry(TAG_NORMAL_CONFIG, normal));
        }
        if let Some(ominous) = self.ominous.to_nbt() {
            fields.push(entry(TAG_OMINOUS_CONFIG, ominous));
        }
        if self.target_cooldown_length != Self::DEFAULT_TARGET_COOLDOWN_LENGTH {
            fields.push(entry(
                TAG_TARGET_COOLDOWN_LENGTH,
                NbtSummaryValue::Int(self.target_cooldown_length),
            ));
        }
        if self.required_player_range != Self::DEFAULT_REQUIRED_PLAYER_RANGE {
            fields.push(entry(
                TAG_REQUIRED_PLAYER_RANGE,
                NbtSummaryValue::Int(self.required_player_range),
            ));
        }
        fields
    }

    /// `TrialSpawner.FullConfig.overrideEntity` (`:397-404`).
    #[must_use]
    pub fn override_entity(&self, resource_id: &str) -> Self {
        Self {
            normal: TrialSpawnerConfigHolder::Direct(
                self.normal.value().with_spawning(resource_id),
            ),
            ominous: TrialSpawnerConfigHolder::Direct(
                self.ominous.value().with_spawning(resource_id),
            ),
            ..self.clone()
        }
    }

    /// Whether a registry reference still waits for [`Self::resolve_references`].
    #[must_use]
    pub const fn has_unresolved_reference(&self) -> bool {
        self.normal.is_unresolved() || self.ominous.is_unresolved()
    }

    /// Resolves registry references once; an id the lookup misses keeps
    /// `DEFAULT` but still saves as written.
    pub fn resolve_references(&mut self, lookup: impl Fn(&str) -> Option<TrialSpawnerConfig>) {
        self.normal.resolve(&lookup);
        self.ominous.resolve(&lookup);
    }

    /// `TrialSpawner.activeConfig` (`:84-86`).
    #[must_use]
    pub const fn active(&self, ominous: bool) -> &TrialSpawnerConfig {
        if ominous {
            self.ominous.value()
        } else {
            self.normal.value()
        }
    }

    /// `TrialSpawner.ominousConfig` (`TrialSpawner.java`).
    #[must_use]
    pub const fn ominous(&self) -> &TrialSpawnerConfig {
        self.ominous.value()
    }

    #[must_use]
    pub const fn target_cooldown_length(&self) -> i32 {
        self.target_cooldown_length
    }

    #[must_use]
    pub const fn required_player_range(&self) -> i32 {
        self.required_player_range
    }
}

fn optional_bounded_int(
    fields: &[NbtSummaryEntry],
    name: &str,
    min: i32,
    max: i32,
    default: i32,
) -> Option<i32> {
    let Some(value) = unique_field(fields, name) else {
        return Some(default);
    };
    let value = numeric_i64(value)?;
    (value >= i64::from(min) && value <= i64::from(max)).then_some(value as i32)
}

fn optional_non_negative_float(
    fields: &[NbtSummaryEntry],
    name: &str,
    default: f32,
) -> Option<f32> {
    let Some(value) = unique_field(fields, name) else {
        return Some(default);
    };
    let value = numeric_f64(value)?;
    (value >= 0.0).then_some(value as f32)
}

/// `WeightedList.codec(LootTable.KEY_CODEC)` (`TrialSpawnerConfig.java`).
fn read_weighted_loot_tables(value: &NbtSummaryValue) -> Option<Vec<WeightedLootTable>> {
    let NbtSummaryValue::List(values) = value else {
        return None;
    };
    values
        .iter()
        .map(|value| {
            let NbtSummaryValue::Compound(fields) = value else {
                return None;
            };
            let NbtSummaryValue::String(resource_id) = unique_field(fields, "data")? else {
                return None;
            };
            let resource_id =
                bbb_protocol::normalize_resource_location(resource_id.try_to_string().ok()?)
                    .ok()?;
            Some(WeightedLootTable {
                resource_id: resource_id.clone(),
                weight: numeric_i64(unique_field(fields, "weight")?)?
                    .try_into()
                    .ok()?,
            })
        })
        .collect()
}
