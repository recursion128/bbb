//! `TrialSpawner`'s persisted state data and server tick state machine.

mod config;
#[cfg(test)]
mod tests;

use std::collections::BTreeSet;

use bbb_protocol::packets::Vec3d;
use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue};
use bbb_server_types::item_stack::RuntimeItemStack;
use uuid::Uuid;

use crate::random::GameRandom;
use crate::world::simulation_tick::trial_spawner::TrialSpawnerDispensingRoll;

use super::base_spawner::{SpawnerSpawnData, WeightedSpawnerSpawnData};
use super::{codec_uuid, entry, unique_field, uuid_int_array};
use config::CONFIG_TAGS;
pub use config::{TrialSpawnerConfig, TrialSpawnerFullConfig, WeightedLootTable};
#[cfg(test)]
use config::{
    DEFAULT_ITEMS_TO_DROP_WHEN_OMINOUS, TAG_ITEMS_TO_DROP_WHEN_OMINOUS, TAG_LOOT_TABLES_TO_EJECT,
    TAG_NORMAL_CONFIG, TAG_OMINOUS_CONFIG, TAG_SPAWN_POTENTIALS, TAG_TARGET_COOLDOWN_LENGTH,
    TAG_TICKS_BETWEEN_SPAWN,
};

/// `TrialSpawnerStateData.Packed.MAP_CODEC` (`TrialSpawnerStateData.java:307-318`).
const TAG_REGISTERED_PLAYERS: &str = "registered_players";
const TAG_CURRENT_MOBS: &str = "current_mobs";
const TAG_COOLDOWN_ENDS_AT: &str = "cooldown_ends_at";
const TAG_NEXT_MOB_SPAWNS_AT: &str = "next_mob_spawns_at";
const TAG_TOTAL_MOBS_SPAWNED: &str = "total_mobs_spawned";
const TAG_SPAWN_DATA: &str = "spawn_data";
const TAG_EJECTING_LOOT_TABLE: &str = "ejecting_loot_table";

/// The `BlockEntity` metadata `saveWithoutMetadata` (`BlockEntity.java:136-139`)
/// and `saveWithFullMetadata` (`:118-121`, `:161-166`) write themselves, so a
/// record read back through them must not also keep them in an unknown-field
/// list.
const METADATA_TAGS: &[&str] = &["id", "x", "y", "z", "components"];

/// `TrialSpawnerStateData.Packed.MAP_CODEC` (`TrialSpawnerStateData.java:307-318`).
const STATE_DATA_TAGS: &[&str] = &[
    TAG_REGISTERED_PLAYERS,
    TAG_CURRENT_MOBS,
    TAG_COOLDOWN_ENDS_AT,
    TAG_NEXT_MOB_SPAWNS_AT,
    TAG_TOTAL_MOBS_SPAWNED,
    TAG_SPAWN_DATA,
    TAG_EJECTING_LOOT_TABLE,
];

/// Every tag one of the three codecs `TrialSpawner.load` reads from the same
/// record owns, which is what the unknown-field lists have to leave out.
fn is_owned_tag(name: &str) -> bool {
    STATE_DATA_TAGS.contains(&name) || CONFIG_TAGS.contains(&name) || METADATA_TAGS.contains(&name)
}

/// `TrialSpawnerState` (`TrialSpawnerState.java`) as the block state's
/// property value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TrialSpawnerState {
    #[default]
    Inactive,
    WaitingForPlayers,
    Active,
    WaitingForRewardEjection,
    EjectingReward,
    Cooldown,
}

impl TrialSpawnerState {
    /// `TrialSpawnerBlockEntity.getState` (`TrialSpawnerBlockEntity.java`):
    /// a state without the property is `INACTIVE`.
    #[must_use]
    pub fn from_block_state(block_state_id: i32) -> Self {
        match bbb_protocol::block_state_types::vanilla_block_state_property(
            block_state_id,
            "trial_spawner_state",
        ) {
            Some("waiting_for_players") => Self::WaitingForPlayers,
            Some("active") => Self::Active,
            Some("waiting_for_reward_ejection") => Self::WaitingForRewardEjection,
            Some("ejecting_reward") => Self::EjectingReward,
            Some("cooldown") => Self::Cooldown,
            _ => Self::Inactive,
        }
    }

    /// The enum's own serialized name (`TrialSpawnerState.java`), which
    /// is what `TrialSpawnerBlock.STATE` holds.
    #[must_use]
    pub const fn property_value(self) -> &'static str {
        match self {
            Self::Inactive => "inactive",
            Self::WaitingForPlayers => "waiting_for_players",
            Self::Active => "active",
            Self::WaitingForRewardEjection => "waiting_for_reward_ejection",
            Self::EjectingReward => "ejecting_reward",
            Self::Cooldown => "cooldown",
        }
    }
}

/// One `WeightedList` entry `TrialSpawnerStateData.getDispensingItems`
/// (`TrialSpawnerStateData.java`) builds out of a loot drop: the drop at
/// count one, weighted by the count it rolled with.
#[derive(Debug, Clone, PartialEq)]
struct DispensingItem {
    item: RuntimeItemStack,
    weight: i32,
}

/// `TrialSpawnerStateData.Packed` (`TrialSpawnerStateData.java`).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct TrialSpawnerStateData {
    registered_players: BTreeSet<Uuid>,
    current_mobs: BTreeSet<Uuid>,
    cooldown_ends_at: i64,
    next_mob_spawns_at: i64,
    total_mobs_spawned: i32,
    spawn_data: Option<SpawnerSpawnData>,
    ejecting_loot_table: Option<String>,
    /// `TrialSpawnerStateData.dispensing` (`:55`): `Packed` does not carry it,
    /// so a fresh state data starts without a cache and a reset keeps it.
    dispensing: Option<Result<Vec<DispensingItem>, String>>,
    remaining: Vec<NbtSummaryEntry>,
}

impl TrialSpawnerStateData {
    /// `TrialSpawnerConfig.ticksBetweenItemSpawners`
    /// (`TrialSpawnerConfig.java:64-66`), which is a constant rather than a
    /// codec field.
    const TICKS_BETWEEN_ITEM_SPAWNERS: i64 = 160;

    /// Every field is a `lenientOptionalFieldOf`, so an absent one takes the
    /// codec's default and a malformed one fails the whole map -- which
    /// `TrialSpawner.load` (`:92`) answers by keeping the fresh data.
    #[must_use]
    pub fn from_nbt(entries: &[NbtSummaryEntry]) -> Option<Self> {
        let mut data = Self::default();
        if let Some(value) = unique_field(entries, TAG_REGISTERED_PLAYERS) {
            data.registered_players = uuid_set(value)?;
        }
        if let Some(value) = unique_field(entries, TAG_CURRENT_MOBS) {
            data.current_mobs = uuid_set(value)?;
        }
        if let Some(value) = unique_field(entries, TAG_COOLDOWN_ENDS_AT) {
            data.cooldown_ends_at = numeric_i64(value)?;
        }
        if let Some(value) = unique_field(entries, TAG_NEXT_MOB_SPAWNS_AT) {
            data.next_mob_spawns_at = numeric_i64(value)?;
        }
        if let Some(value) = unique_field(entries, TAG_TOTAL_MOBS_SPAWNED) {
            data.total_mobs_spawned = non_negative_int(value)?;
        }
        if let Some(value) = unique_field(entries, TAG_SPAWN_DATA) {
            data.spawn_data = Some(SpawnerSpawnData::from_nbt(value)?);
        }
        if let Some(value) = unique_field(entries, TAG_EJECTING_LOOT_TABLE) {
            let NbtSummaryValue::String(resource_id) = value else {
                return None;
            };
            let resource_id =
                bbb_protocol::normalize_resource_location(resource_id.try_to_string().ok()?)
                    .ok()?;
            data.ejecting_loot_table = Some(resource_id);
        }
        data.remaining = entries
            .iter()
            .filter(|entry| !entry.name.with_utf8(is_owned_tag).unwrap_or(false))
            .cloned()
            .collect();
        Some(data)
    }

    #[must_use]
    pub fn to_nbt(&self) -> Vec<NbtSummaryEntry> {
        let mut entries = Vec::new();
        if !self.registered_players.is_empty() {
            entries.push(entry(
                TAG_REGISTERED_PLAYERS,
                uuid_set_nbt(&self.registered_players),
            ));
        }
        if !self.current_mobs.is_empty() {
            entries.push(entry(TAG_CURRENT_MOBS, uuid_set_nbt(&self.current_mobs)));
        }
        if self.cooldown_ends_at != 0 {
            entries.push(entry(
                TAG_COOLDOWN_ENDS_AT,
                NbtSummaryValue::Long(self.cooldown_ends_at),
            ));
        }
        if self.next_mob_spawns_at != 0 {
            entries.push(entry(
                TAG_NEXT_MOB_SPAWNS_AT,
                NbtSummaryValue::Long(self.next_mob_spawns_at),
            ));
        }
        if self.total_mobs_spawned != 0 {
            entries.push(entry(
                TAG_TOTAL_MOBS_SPAWNED,
                NbtSummaryValue::Int(self.total_mobs_spawned),
            ));
        }
        if let Some(spawn_data) = &self.spawn_data {
            entries.push(entry(TAG_SPAWN_DATA, spawn_data.to_nbt()));
        }
        if let Some(resource_id) = &self.ejecting_loot_table {
            entries.push(entry(
                TAG_EJECTING_LOOT_TABLE,
                NbtSummaryValue::string(resource_id.clone()),
            ));
        }
        entries.extend(self.remaining.iter().cloned());
        entries
    }

    /// `TrialSpawnerStateData.getUpdateTag` (`:252-259`): `next_mob_spawns_at`
    /// rides the tag only while the block state says ACTIVE.
    #[must_use]
    pub fn update_tag(&self, state: TrialSpawnerState) -> Vec<NbtSummaryEntry> {
        let mut entries = Vec::new();
        if state == TrialSpawnerState::Active && self.next_mob_spawns_at != 0 {
            entries.push(entry(
                TAG_NEXT_MOB_SPAWNS_AT,
                NbtSummaryValue::Long(self.next_mob_spawns_at),
            ));
        }
        if let Some(spawn_data) = &self.spawn_data {
            entries.push(entry(TAG_SPAWN_DATA, spawn_data.to_nbt()));
        }
        entries
    }

    #[must_use]
    pub fn registered_players(&self) -> &BTreeSet<Uuid> {
        &self.registered_players
    }

    pub fn registered_players_mut(&mut self) -> &mut BTreeSet<Uuid> {
        &mut self.registered_players
    }

    /// `TrialSpawnerStateData.detectedPlayers` (`TrialSpawnerStateData.java`).
    #[must_use]
    pub fn detected_players(&self) -> &BTreeSet<Uuid> {
        &self.registered_players
    }

    #[must_use]
    pub fn current_mobs(&self) -> &BTreeSet<Uuid> {
        &self.current_mobs
    }

    pub fn current_mobs_mut(&mut self) -> &mut BTreeSet<Uuid> {
        &mut self.current_mobs
    }

    #[must_use]
    pub fn next_spawn_data(&self) -> Option<&SpawnerSpawnData> {
        self.spawn_data.as_ref()
    }

    pub fn set_next_spawn_data(&mut self, spawn_data: Option<SpawnerSpawnData>) {
        self.spawn_data = spawn_data;
    }

    pub fn set_next_mob_spawns_at(&mut self, value: i64) {
        self.next_mob_spawns_at = value;
    }

    pub fn set_cooldown_ends_at(&mut self, value: i64) {
        self.cooldown_ends_at = value;
    }

    pub fn set_total_mobs_spawned(&mut self, value: i32) {
        self.total_mobs_spawned = value;
    }

    pub fn set_ejecting_loot_table(&mut self, resource_id: Option<String>) {
        self.ejecting_loot_table = resource_id;
    }

    /// `TrialSpawnerStateData.ejectingLootTable` (`TrialSpawnerStateData.java`).
    #[must_use]
    pub fn ejecting_loot_table(&self) -> Option<&str> {
        self.ejecting_loot_table.as_deref()
    }

    /// `TrialSpawnerState.java`: `detectedPlayers.remove(iterator().next())`,
    /// which removes from the set `registered_players` packs.
    pub fn remove_one_detected_player(&mut self) -> Option<Uuid> {
        let uuid = self.registered_players.iter().next().copied()?;
        self.registered_players.remove(&uuid);
        Some(uuid)
    }

    /// `TrialSpawnerStateData.reset` (`TrialSpawnerStateData.java`).
    pub fn reset(&mut self) {
        self.current_mobs.clear();
        self.spawn_data = None;
        self.reset_statistics();
    }

    /// `TrialSpawnerStateData.resetStatistics` (`:84-89`).
    pub fn reset_statistics(&mut self) {
        self.registered_players.clear();
        self.total_mobs_spawned = 0;
        self.next_mob_spawns_at = 0;
        self.cooldown_ends_at = 0;
    }

    /// `countAdditionalPlayers` (`:110-116`).
    #[must_use]
    pub fn count_additional_players(&self) -> i32 {
        i32::try_from(self.registered_players.len())
            .unwrap_or(i32::MAX)
            .saturating_sub(1)
            .max(0)
    }

    /// `hasMobToSpawn` (`:91-97`).
    pub fn has_mob_to_spawn(&mut self, config: &TrialSpawnerConfig, random: &GameRandom) -> bool {
        let has_next = self
            .get_or_create_next_spawn_data(config, random)
            .entity_resource_id()
            .is_some();
        has_next || !config.spawn_potentials().is_empty()
    }

    /// `getOrCreateNextSpawnData` (`:225-235`).
    pub fn get_or_create_next_spawn_data(
        &mut self,
        config: &TrialSpawnerConfig,
        random: &GameRandom,
    ) -> &SpawnerSpawnData {
        if self.spawn_data.is_none() {
            let selected = weighted_spawn_data(config.spawn_potentials(), random);
            self.spawn_data = Some(selected.unwrap_or_default());
        }
        self.spawn_data.as_ref().expect("just filled")
    }

    /// `hasFinishedSpawningAllMobs` (`:99-101`).
    #[must_use]
    pub fn has_finished_spawning_all_mobs(
        &self,
        config: &TrialSpawnerConfig,
        additional_players: i32,
    ) -> bool {
        self.total_mobs_spawned >= config.calculate_target_total_mobs(additional_players)
    }

    /// `haveAllCurrentMobsDied` (`:103-105`).
    #[must_use]
    pub fn have_all_current_mobs_died(&self) -> bool {
        self.current_mobs.is_empty()
    }

    /// `isReadyToSpawnNextMob` (`:107-109`).
    #[must_use]
    pub fn is_ready_to_spawn_next_mob(
        &self,
        game_time: i64,
        config: &TrialSpawnerConfig,
        additional_players: i32,
    ) -> bool {
        game_time >= self.next_mob_spawns_at
            && i32::try_from(self.current_mobs.len()).unwrap_or(i32::MAX)
                < config.calculate_target_simultaneous_mobs(additional_players)
    }

    /// `isReadyToOpenShutter` (`:211-214`).
    #[must_use]
    pub fn is_ready_to_open_shutter(
        &self,
        game_time: i64,
        delay_before_open: f32,
        target_cooldown_length: i32,
    ) -> bool {
        let cooldown_started_at = self.cooldown_ends_at - i64::from(target_cooldown_length);
        game_time as f32 >= cooldown_started_at as f32 + delay_before_open
    }

    /// `isReadyToEjectItems` (`:216-219`).
    #[must_use]
    pub fn is_ready_to_eject_items(
        &self,
        game_time: i64,
        time_between_ejections: f32,
        target_cooldown_length: i32,
    ) -> bool {
        let cooldown_started_at = self.cooldown_ends_at - i64::from(target_cooldown_length);
        (game_time - cooldown_started_at) as f32 % time_between_ejections == 0.0
    }

    /// The ACTIVE arm's bookkeeping after a successful `spawnMob`
    /// (`TrialSpawnerState.java`).
    pub fn note_mob_spawned(
        &mut self,
        uuid: Uuid,
        game_time: i64,
        config: &TrialSpawnerConfig,
        random: &GameRandom,
    ) {
        self.current_mobs.insert(uuid);
        self.total_mobs_spawned = self.total_mobs_spawned.saturating_add(1);
        self.next_mob_spawns_at = game_time + i64::from(config.ticks_between_spawn());
        if let Some(next) = weighted_spawn_data(config.spawn_potentials(), random) {
            self.spawn_data = Some(next);
        }
    }

    /// `isCooldownFinished` (`:221-223`).
    #[must_use]
    pub fn is_cooldown_finished(&self, game_time: i64) -> bool {
        game_time >= self.cooldown_ends_at
    }

    /// `TrialSpawnerState.timeToSpawnItemSpawner` (`:223-225`).
    #[must_use]
    const fn is_time_to_spawn_item_spawner(&self, game_time: i64) -> bool {
        game_time >= self.cooldown_ends_at
    }

    /// `TrialSpawnerState.spawnOminousOminousItemSpawner`'s cooldown write
    /// (`:170`), which the level reaches only once its position selection has
    /// answered a position.
    pub fn note_ominous_item_spawned(&mut self, game_time: i64) {
        self.cooldown_ends_at = game_time + Self::TICKS_BETWEEN_ITEM_SPAWNERS;
    }

    /// `TrialSpawnerStateData.getDispensingItems` (`:271-295`). An empty roll
    /// is not cached, so a table that answers nothing is rolled every step.
    fn get_dispensing_items(
        &mut self,
        config: &TrialSpawnerConfig,
        loot: &TrialSpawnerDispensingRoll<'_>,
    ) -> bbb_server_types::Result<&[DispensingItem]> {
        if self.dispensing.is_none() {
            let loaded = (|| -> bbb_server_types::Result<Vec<DispensingItem>> {
                let mut items = Vec::new();
                let mut total_weight = 0_i32;
                for drop in loot.dispensing_drops(config.items_to_drop_when_ominous())? {
                    let weight = drop.count();
                    if weight < 0 {
                        return Err(bbb_server_types::ServerWorldError::InvalidConfiguration(
                            "ominous dispensing weight is negative".to_owned(),
                        ));
                    }
                    total_weight = total_weight.checked_add(weight).ok_or_else(|| {
                        bbb_server_types::ServerWorldError::InvalidConfiguration(
                            "ominous dispensing weight sum exceeds int range".to_owned(),
                        )
                    })?;
                    items.push(DispensingItem {
                        weight,
                        item: drop.with_count(1),
                    });
                }
                Ok(items)
            })();
            match loaded {
                Ok(items) if items.is_empty() => return Ok(&[]),
                Ok(items) => self.dispensing = Some(Ok(items)),
                Err(error) => {
                    self.dispensing = Some(Err(error.to_string()));
                    return Err(error);
                }
            }
        }
        match self
            .dispensing
            .as_ref()
            .expect("nonempty roll or terminal cached")
        {
            Ok(items) => Ok(items),
            Err(detail) => Err(bbb_server_types::ServerWorldError::InvalidConfiguration(
                detail.clone(),
            )),
        }
    }

    /// Retains a BBB terminal refusal without committing an undelivered detection.
    pub fn retain_dispensing_cache_from(&mut self, attempt: &Self) {
        if let Some(Err(detail)) = &attempt.dispensing {
            self.dispensing = Some(Err(detail.clone()));
        }
    }

    /// `TrialSpawnerState.java`: `getDispensingItems(...).getRandom(...)`,
    fn pick_dispensing_item(
        &mut self,
        config: &TrialSpawnerConfig,
        loot: &TrialSpawnerDispensingRoll<'_>,
        random: &GameRandom,
    ) -> bbb_server_types::Result<Option<RuntimeItemStack>> {
        let items = self.get_dispensing_items(config, loot)?;
        let Some(index) = weighted_index(items.iter().map(|item| item.weight), random) else {
            return Ok(None);
        };
        Ok(items.get(index).map(|item| item.item.clone()))
    }

    #[must_use]
    pub const fn total_mobs_spawned(&self) -> i32 {
        self.total_mobs_spawned
    }

    #[must_use]
    pub const fn next_mob_spawns_at(&self) -> i64 {
        self.next_mob_spawns_at
    }

    #[must_use]
    pub const fn cooldown_ends_at(&self) -> i64 {
        self.cooldown_ends_at
    }
}

/// `TrialSpawnerBlockEntity` (`:22-33`), whose `loadAdditional`/`saveAdditional`
/// are `TrialSpawner.load`/`store` (`TrialSpawner.java`).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct TrialSpawnerBlockEntity {
    config: TrialSpawnerFullConfig,
    data: TrialSpawnerStateData,
}

impl TrialSpawnerBlockEntity {
    /// `TrialSpawner.getStateData().getUpdateTag(state)`
    /// (`TrialSpawnerBlockEntity.java`), whose `state` argument is the
    /// block state's `trial_spawner_state`.
    pub(super) fn save_items_with_state(&self, block_state_id: i32) -> Vec<NbtSummaryEntry> {
        self.data
            .update_tag(TrialSpawnerState::from_block_state(block_state_id))
    }

    /// `TrialSpawner.store` (`TrialSpawner.java`).
    pub(super) fn save_additional(&self) -> Vec<NbtSummaryEntry> {
        let mut entries = self.data.to_nbt();
        entries.extend(self.config.to_nbt());
        entries
    }

    /// `TrialSpawner.load` (`TrialSpawner.java`): the state data keeps
    /// its fresh defaults when its map fails, while a failed `FullConfig`
    /// resets the whole config to `DEFAULT`.
    pub(super) fn load_additional(&mut self, record: &[NbtSummaryEntry]) {
        self.data = TrialSpawnerStateData::from_nbt(record).unwrap_or_default();
        self.config = TrialSpawnerFullConfig::from_nbt(record).unwrap_or_default();
    }

    /// `TrialSpawnerBlockEntity.setEntityId` (`TrialSpawnerBlockEntity.java`)
    /// through `TrialSpawner.overrideEntityToSpawn` (`TrialSpawner.java`).
    /// The state write and the block update belong to the level.
    pub fn override_entity_to_spawn(&mut self, resource_id: &str) {
        self.data.reset();
        self.config = self.config.override_entity(resource_id);
    }

    #[must_use]
    pub const fn config(&self) -> &TrialSpawnerFullConfig {
        &self.config
    }

    pub const fn config_mut(&mut self) -> &mut TrialSpawnerFullConfig {
        &mut self.config
    }

    #[must_use]
    pub const fn state_data(&self) -> &TrialSpawnerStateData {
        &self.data
    }

    pub const fn state_data_mut(&mut self) -> &mut TrialSpawnerStateData {
        &mut self.data
    }
}

/// `WeightedList.getRandom` (`WeightedList.java`): one `nextInt` over the
/// checked weight sum, then the cumulative walk both selectors implement
/// (`:161-199`). The weights are an iterator because every caller already
/// holds its own weighted elements.
fn weighted_index(
    weights: impl Iterator<Item = i32> + Clone,
    random: &GameRandom,
) -> Option<usize> {
    let total = weights
        .clone()
        .try_fold(0_i32, |total, weight| total.checked_add(weight))?;
    if total == 0 {
        return None;
    }
    let mut selection = random.next_int(total);
    for (index, weight) in weights.enumerate() {
        if selection < weight {
            return Some(index);
        }
        selection -= weight;
    }
    None
}

/// `WeightedList.getRandom` (`WeightedList.java`) over `SpawnData`.
fn weighted_spawn_data(
    potentials: &[WeightedSpawnerSpawnData],
    random: &GameRandom,
) -> Option<SpawnerSpawnData> {
    let index = weighted_index(potentials.iter().map(|potential| potential.weight), random)?;
    Some(potentials[index].data.clone())
}

/// `WeightedList.getRandom` (`WeightedList.java`) over
/// `loot_tables_to_eject`, drawn once per ejection sequence
/// (`TrialSpawnerState.java`).
fn weighted_loot_table(tables: &[WeightedLootTable], random: &GameRandom) -> Option<String> {
    let index = weighted_index(tables.iter().map(|table| table.weight), random)?;
    Some(tables[index].resource_id.clone())
}

fn numeric_i64(value: &NbtSummaryValue) -> Option<i64> {
    match value {
        NbtSummaryValue::Byte(value) => Some(i64::from(*value)),
        NbtSummaryValue::Short(value) => Some(i64::from(*value)),
        NbtSummaryValue::Int(value) => Some(i64::from(*value)),
        NbtSummaryValue::Long(value) => Some(*value),
        _ => None,
    }
}

fn non_negative_int(value: &NbtSummaryValue) -> Option<i32> {
    let value = numeric_i64(value)?;
    (0..=i64::from(i32::MAX))
        .contains(&value)
        .then_some(value as i32)
}

/// `UUIDUtil.CODEC_SET` (`TrialSpawnerStateData.java`).
fn uuid_set(value: &NbtSummaryValue) -> Option<BTreeSet<Uuid>> {
    let NbtSummaryValue::List(values) = value else {
        return None;
    };
    values.iter().map(codec_uuid).collect()
}

fn uuid_set_nbt(values: &BTreeSet<Uuid>) -> NbtSummaryValue {
    NbtSummaryValue::List(
        values
            .iter()
            .map(|uuid| NbtSummaryValue::IntArray(uuid_int_array(*uuid)))
            .collect(),
    )
}

/// One `PlayerDetector.detect` result with the two answers
/// `TrialSpawnerStateData.findPlayerWithOminousEffect` (`:159-177`) reads off
/// it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TrialSpawnerDetectedPlayer {
    pub uuid: Uuid,
    /// `Entity.getEyePosition` (`Entity.java`), which `:134` floors
    /// into the 3020 event's position.
    pub eye_position: Vec3d,
    /// `Player.hasEffect(MobEffects.TRIAL_OMEN)` (`:166`).
    pub has_trial_omen: bool,
    /// `Player.hasEffect(MobEffects.BAD_OMEN)` (`:170`).
    pub has_bad_omen: bool,
}

/// The level's answers for one `TrialSpawnerState.tickAndGetNext` step
/// (`TrialSpawnerState.java`). The machine stays here; the level owns
/// the entity spawn, the level events and the block state write.
#[derive(Debug, Clone, PartialEq)]
pub struct TrialSpawnerTickFacts {
    pub game_time: i64,
    /// The position half of `TrialSpawnerStateData.tryDetectPlayers`'s throttle,
    /// `pos.asLong() % 20` (`TrialSpawnerStateData.java`): the only part of
    /// the spawner's position that line reads.
    pub detection_phase: i64,
    /// `TrialSpawner.canSpawnInLevel` (`TrialSpawner.java`).
    pub can_spawn_in_level: bool,
    /// `TrialSpawner.isOminous` (`:114`), which the block state carries.
    pub is_ominous: bool,
    /// `PlayerDetector.NO_CREATIVE_PLAYERS` with line of sight
    /// (`PlayerDetector.java`).
    pub players_in_line_of_sight: Vec<TrialSpawnerDetectedPlayer>,
    /// The same detector without the sight requirement, which the state data
    /// asks for once a first player is known (`TrialSpawnerStateData.java`).
    pub players_without_sight: Vec<Uuid>,
}

/// `TrialSpawner.applyOminous` (`TrialSpawner.java`) as the level sees
/// it: the player whose eye the 3020 event floors, whether that player's bad
/// omen must become a trial omen (`TrialSpawnerStateData.java`), and
/// the mobs `resetAfterBecomingOminous` (`:179-199`) retired.
#[derive(Debug, Clone, PartialEq)]
pub struct TrialSpawnerOminousActivation {
    pub player: Uuid,
    pub eye_position: Vec3d,
    pub transform_bad_omen: bool,
    pub removed_mobs: Vec<Uuid>,
}

/// What `TrialSpawnerStateData.tryDetectPlayers`'s detection half asks the
/// level to do (`TrialSpawnerStateData.java`). Vanilla raises the
/// detection broadcast or the activation, never both: `:149` suppresses the
/// event on the tick that turned the spawner ominous.
#[derive(Debug, Clone, PartialEq)]
pub enum TrialSpawnerDetectionEffect {
    /// `:150-151`: 3013 for a normal spawner, 3019 for an ominous one, with
    /// `detectedPlayers.size()` as the data.
    Players {
        event_id: i32,
        detected: i32,
    },
    Ominous(TrialSpawnerOminousActivation),
}

impl TrialSpawnerDetectionEffect {
    /// `TrialSpawner.applyOminous`'s `this.isOminous = true`
    /// (`TrialSpawner.java`).
    #[must_use]
    pub const fn became_ominous(&self) -> bool {
        matches!(self, Self::Ominous(_))
    }
}

/// What one step asks the level to do.
#[derive(Debug, Clone, PartialEq)]
pub enum TrialSpawnerTickOutcome {
    /// The state stays; `spawn_mob` is the ACTIVE arm's `spawnMob` request
    /// (`TrialSpawnerState.java`), whose bookkeeping the level applies
    /// through [`TrialSpawnerStateData::note_mob_spawned`] on success.
    Stay { spawn_mob: bool },
    /// `TrialSpawner.setState` (`TrialSpawner.java`).
    Transition(TrialSpawnerState),
    /// `TrialSpawnerState.java`: the open-shutter sound, then `next`.
    OpenShutter { next: TrialSpawnerState },
    /// `:128-130`: the close-shutter sound, then `next`; the pending eject
    /// table is already cleared.
    CloseShutter { next: TrialSpawnerState },
    /// `:132-138`: roll `loot_table` and eject its drops. The state stays and
    /// one detected player is already consumed, whether or not the roll
    /// produces anything.
    EjectReward { loot_table: String },
    /// `:147-150`: `TrialSpawner.removeOminous` (`TrialSpawner.java`)
    /// before `next`; the state data is already reset.
    RemoveOminous { next: TrialSpawnerState },
}

/// One whole `TrialSpawnerState.tickAndGetNext` answer: the step, the detection
/// effect that step decided inside itself, and `TrialSpawner.isOminous` as it
/// stands when the step returns.
#[derive(Debug, Clone, PartialEq)]
pub struct TrialSpawnerTickResult {
    pub outcome: TrialSpawnerTickOutcome,
    pub detection: Option<TrialSpawnerDetectionEffect>,
    /// `TrialSpawner.activeConfig` (`TrialSpawner.java`) reads the flag
    /// live, so `TrialSpawner.spawnMob` (`:174`, `:228`) and the ACTIVE arm's
    /// `isOminous()` gate (`TrialSpawnerState.java`) see an activation that
    /// happened earlier in the same step.
    pub is_ominous: bool,
    /// `TrialSpawnerState.spawnOminousOminousItemSpawner` (`:158-174`): the
    /// item the ACTIVE step drew for an `OminousItemSpawner`. The level picks
    /// the position, spawns the entity and plays the sound, then answers with
    /// [`TrialSpawnerStateData::note_ominous_item_spawned`].
    pub spawn_ominous_item: Option<RuntimeItemStack>,
}

/// `TrialSpawnerState.DELAY_BEFORE_EJECT_AFTER_KILLING_LAST_MOB`
/// (`TrialSpawnerState.java:41`).
const DELAY_BEFORE_EJECT_AFTER_KILLING_LAST_MOB: f32 = 40.0;

/// `TrialSpawnerState.TIME_BETWEEN_EACH_EJECTION`
/// (`TrialSpawnerState.java:42`).
const TIME_BETWEEN_EACH_EJECTION: f32 = 30.0;

/// `TrialSpawnerState.tickAndGetNext` (`TrialSpawnerState.java`).
///
/// `WAITING_FOR_REWARD_EJECTION` and `EJECTING_REWARD` decide here and only
/// ask the level for their effects; the loot roll itself is the world's.
#[cfg(test)]
pub(crate) fn tick_and_get_next(
    state: TrialSpawnerState,
    data: &mut TrialSpawnerStateData,
    full_config: &TrialSpawnerFullConfig,
    facts: &TrialSpawnerTickFacts,
    random: &GameRandom,
    loot: &TrialSpawnerDispensingRoll<'_>,
) -> bbb_server_types::Result<TrialSpawnerTickResult> {
    tick_and_get_next_with_registry(
        state,
        data,
        full_config,
        facts,
        random,
        loot,
        bbb_protocol::content_registry::ContentRegistry::vanilla(),
    )
}

pub(crate) fn tick_and_get_next_with_registry(
    state: TrialSpawnerState,
    data: &mut TrialSpawnerStateData,
    full_config: &TrialSpawnerFullConfig,
    facts: &TrialSpawnerTickFacts,
    random: &GameRandom,
    loot: &TrialSpawnerDispensingRoll<'_>,
    content_registry: &bbb_protocol::content_registry::ContentRegistry,
) -> bbb_server_types::Result<TrialSpawnerTickResult> {
    // `:65` binds the active config before this step's own detection runs, so
    // the arm bodies below keep the config the step entered with even when the
    // detection turns the spawner ominous.
    let mut is_ominous = facts.is_ominous;
    let config = full_config.active(is_ominous);
    let mut detection = None;
    let mut spawn_ominous_item = None;
    let outcome = 'step: {
        match state {
            TrialSpawnerState::Inactive => {
                // `:69`: the arm waits for the display entity the spawn data names.
                let displayable = data
                    .get_or_create_next_spawn_data(config, random)
                    .entity_type_id_with_registry(content_registry)
                    .is_some();
                if displayable {
                    TrialSpawnerTickOutcome::Transition(TrialSpawnerState::WaitingForPlayers)
                } else {
                    TrialSpawnerTickOutcome::Stay { spawn_mob: false }
                }
            }
            TrialSpawnerState::WaitingForPlayers => {
                if !facts.can_spawn_in_level {
                    data.reset_statistics();
                    break 'step TrialSpawnerTickOutcome::Stay { spawn_mob: false };
                }
                if !data.has_mob_to_spawn(config, random) {
                    break 'step TrialSpawnerTickOutcome::Transition(TrialSpawnerState::Inactive);
                }
                let effect = data.try_detect_players(state, facts, full_config);
                if effect
                    .as_ref()
                    .is_some_and(TrialSpawnerDetectionEffect::became_ominous)
                {
                    is_ominous = true;
                }
                detection = effect;
                if data.detected_players().is_empty() {
                    TrialSpawnerTickOutcome::Stay { spawn_mob: false }
                } else {
                    TrialSpawnerTickOutcome::Transition(TrialSpawnerState::Active)
                }
            }
            TrialSpawnerState::Active => {
                if !facts.can_spawn_in_level {
                    data.reset_statistics();
                    break 'step TrialSpawnerTickOutcome::Transition(
                        TrialSpawnerState::WaitingForPlayers,
                    );
                }
                if !data.has_mob_to_spawn(config, random) {
                    break 'step TrialSpawnerTickOutcome::Transition(TrialSpawnerState::Inactive);
                }
                let additional_players = data.count_additional_players();
                let effect = data.try_detect_players(state, facts, full_config);
                if effect
                    .as_ref()
                    .is_some_and(TrialSpawnerDetectionEffect::became_ominous)
                {
                    is_ominous = true;
                }
                detection = effect;
                // `:90-92` reads the live flag, and the call it makes
                // (`:158-174`) re-reads `activeConfig()` for the table, so both
                // see an activation this step. The draw runs before the
                // cooldown gate, and an empty stack is not a request.
                if is_ominous {
                    let drawn =
                        data.pick_dispensing_item(full_config.active(is_ominous), loot, random)?;
                    if let Some(item) = drawn.filter(|item| !item.is_empty()) {
                        if data.is_time_to_spawn_item_spawner(facts.game_time) {
                            spawn_ominous_item = Some(item);
                        }
                    }
                }
                if data.has_finished_spawning_all_mobs(config, additional_players) {
                    if data.have_all_current_mobs_died() {
                        data.set_cooldown_ends_at(
                            facts.game_time + i64::from(full_config.target_cooldown_length()),
                        );
                        data.set_total_mobs_spawned(0);
                        data.set_next_mob_spawns_at(0);
                        break 'step TrialSpawnerTickOutcome::Transition(
                            TrialSpawnerState::WaitingForRewardEjection,
                        );
                    }
                    break 'step TrialSpawnerTickOutcome::Stay { spawn_mob: false };
                }
                if data.is_ready_to_spawn_next_mob(facts.game_time, config, additional_players) {
                    break 'step TrialSpawnerTickOutcome::Stay { spawn_mob: true };
                }
                TrialSpawnerTickOutcome::Stay { spawn_mob: false }
            }
            TrialSpawnerState::WaitingForRewardEjection => {
                if data.is_ready_to_open_shutter(
                    facts.game_time,
                    DELAY_BEFORE_EJECT_AFTER_KILLING_LAST_MOB,
                    full_config.target_cooldown_length(),
                ) {
                    TrialSpawnerTickOutcome::OpenShutter {
                        next: TrialSpawnerState::EjectingReward,
                    }
                } else {
                    TrialSpawnerTickOutcome::Stay { spawn_mob: false }
                }
            }
            TrialSpawnerState::EjectingReward => {
                if !data.is_ready_to_eject_items(
                    facts.game_time,
                    TIME_BETWEEN_EACH_EJECTION,
                    full_config.target_cooldown_length(),
                ) {
                    break 'step TrialSpawnerTickOutcome::Stay { spawn_mob: false };
                }
                if data.detected_players().is_empty() {
                    data.set_ejecting_loot_table(None);
                    break 'step TrialSpawnerTickOutcome::CloseShutter {
                        next: TrialSpawnerState::Cooldown,
                    };
                }
                if data.ejecting_loot_table().is_none() {
                    data.set_ejecting_loot_table(weighted_loot_table(
                        config.loot_tables_to_eject(),
                        random,
                    ));
                }
                let loot_table = data.ejecting_loot_table().map(str::to_owned);
                match loot_table {
                    Some(loot_table) => TrialSpawnerTickOutcome::EjectReward { loot_table },
                    None => {
                        data.remove_one_detected_player();
                        TrialSpawnerTickOutcome::Stay { spawn_mob: false }
                    }
                }
            }
            TrialSpawnerState::Cooldown => {
                let effect = data.try_detect_players(state, facts, full_config);
                if effect
                    .as_ref()
                    .is_some_and(TrialSpawnerDetectionEffect::became_ominous)
                {
                    is_ominous = true;
                }
                detection = effect;
                if !data.detected_players().is_empty() {
                    data.set_total_mobs_spawned(0);
                    data.set_next_mob_spawns_at(0);
                    break 'step TrialSpawnerTickOutcome::Transition(TrialSpawnerState::Active);
                }
                if data.is_cooldown_finished(facts.game_time) {
                    data.reset();
                    break 'step TrialSpawnerTickOutcome::RemoveOminous {
                        next: TrialSpawnerState::WaitingForPlayers,
                    };
                }
                TrialSpawnerTickOutcome::Stay { spawn_mob: false }
            }
        }
    };
    Ok(TrialSpawnerTickResult {
        outcome,
        detection,
        is_ominous,
        spawn_ominous_item,
    })
}

/// `TrialSpawnerStateData` (`TrialSpawnerStateData.java`).
impl TrialSpawnerStateData {
    /// `TrialSpawnerStateData.tryDetectPlayers` (`:118-155`). The bad-omen
    /// transformation, the 3020 pair, the block write and the per-mob removals
    /// are the level's; this decides and mutates the state data.
    fn try_detect_players(
        &mut self,
        state: TrialSpawnerState,
        facts: &TrialSpawnerTickFacts,
        full_config: &TrialSpawnerFullConfig,
    ) -> Option<TrialSpawnerDetectionEffect> {
        if (facts.detection_phase + facts.game_time) % 20 != 0 {
            return None;
        }
        // `:122`: an already-ominous spawner in COOLDOWN skips the scan.
        if state == TrialSpawnerState::Cooldown && facts.is_ominous {
            return None;
        }
        let in_line_of_sight = facts.players_in_line_of_sight.as_slice();
        let mut became_ominous = None;
        // `:126-140`: the scan that can turn the spawner ominous runs only
        // while it is normal.
        if !facts.is_ominous && !in_line_of_sight.is_empty() {
            if let Some((player, is_bad_omen)) = find_player_with_ominous_effect(in_line_of_sight) {
                became_ominous = Some(TrialSpawnerOminousActivation {
                    player: player.uuid,
                    eye_position: player.eye_position,
                    transform_bad_omen: is_bad_omen,
                    removed_mobs: self.reset_after_becoming_ominous(facts, full_config),
                });
            }
        }
        // `:142-153`.
        if state != TrialSpawnerState::Cooldown || became_ominous.is_some() {
            let found = if self.registered_players.is_empty() {
                in_line_of_sight.iter().map(|player| player.uuid).collect()
            } else {
                facts.players_without_sight.clone()
            };
            let mut added = false;
            for uuid in found {
                added |= self.registered_players.insert(uuid);
            }
            if added {
                self.next_mob_spawns_at = self.next_mob_spawns_at.max(facts.game_time + 40);
                // `:149-151`: the tick that turned the spawner ominous
                // suppresses the event even though `:150` would already read
                // the new flag.
                if became_ominous.is_none() {
                    let event_id = if facts.is_ominous { 3019 } else { 3013 };
                    return Some(TrialSpawnerDetectionEffect::Players {
                        event_id,
                        detected: i32::try_from(self.registered_players.len()).unwrap_or(i32::MAX),
                    });
                }
            }
        }
        became_ominous.map(TrialSpawnerDetectionEffect::Ominous)
    }

    /// `TrialSpawnerStateData.resetAfterBecomingOminous` (`:179-199`) without
    /// its per-entity half, which needs the level's entity store: the retired
    /// mobs come back for the level to emit 3012, drop preserved equipment and
    /// discard. `markUpdated` is the level's write-back.
    fn reset_after_becoming_ominous(
        &mut self,
        facts: &TrialSpawnerTickFacts,
        full_config: &TrialSpawnerFullConfig,
    ) -> Vec<Uuid> {
        let removed = self.current_mobs.iter().copied().collect();
        let ominous = full_config.ominous();
        if !ominous.spawn_potentials().is_empty() {
            self.spawn_data = None;
        }
        self.total_mobs_spawned = 0;
        self.current_mobs.clear();
        self.next_mob_spawns_at = facts.game_time + i64::from(ominous.ticks_between_spawn());
        self.cooldown_ends_at = facts.game_time + Self::TICKS_BETWEEN_ITEM_SPAWNERS;
        removed
    }
}

/// `TrialSpawnerStateData.findPlayerWithOminousEffect` (`:159-177`): the first
/// player holding TRIAL_OMEN wins, else the last holding BAD_OMEN, and the
/// second field is which of the two won -- `:130` transforms only a bad omen,
/// so a player holding both is left alone.
fn find_player_with_ominous_effect(
    players: &[TrialSpawnerDetectedPlayer],
) -> Option<(&TrialSpawnerDetectedPlayer, bool)> {
    let mut bad_omen = None;
    for player in players {
        if player.has_trial_omen {
            return Some((player, false));
        }
        if player.has_bad_omen {
            bad_omen = Some(player);
        }
    }
    bad_omen.map(|player| (player, true))
}
