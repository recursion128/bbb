//! `BaseSpawner` (`BaseSpawner.java:31-263`), the state machine
//! `SpawnerBlockEntity` and `MinecartSpawner` share.

use crate::random::GameRandom;
use bbb_protocol::entity_types::VANILLA_ENTITY_TYPES;
pub use bbb_protocol::nbt_summary::compound_field as field;
use bbb_protocol::nbt_summary::{codec_double_value as numeric_f64, entry};
use bbb_protocol::packets::{BlockPos, Vec3d};
use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue};

/// `BaseSpawner.EVENT_SPAWN` (`BaseSpawner.java:34`).
pub const SPAWNER_EVENT_ID: u8 = 1;
/// `BaseSpawner` field defaults (`BaseSpawner.java:35-53`).
pub const DEFAULT_SPAWN_DELAY: i32 = 20;
pub const DEFAULT_MIN_SPAWN_DELAY: i32 = 200;
pub const DEFAULT_MAX_SPAWN_DELAY: i32 = 800;
pub const DEFAULT_SPAWN_COUNT: i32 = 4;
pub const DEFAULT_MAX_NEARBY_ENTITIES: i32 = 6;
pub const DEFAULT_REQUIRED_PLAYER_RANGE: i32 = 16;
pub const DEFAULT_SPAWN_RANGE: i32 = 4;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpawnerSpawnData {
    entity: Vec<NbtSummaryEntry>,
    custom_spawn_rules: Option<NbtSummaryValue>,
    equipment: Option<NbtSummaryValue>,
}

impl SpawnerSpawnData {
    pub fn new(
        entity: Vec<NbtSummaryEntry>,
        custom_spawn_rules: Option<NbtSummaryValue>,
        equipment: Option<NbtSummaryValue>,
    ) -> Self {
        Self {
            entity: canonicalize_entity_id(entity),
            custom_spawn_rules,
            equipment,
        }
    }

    pub fn entity(&self) -> &[NbtSummaryEntry] {
        &self.entity
    }

    pub fn entity_mut(&mut self) -> &mut Vec<NbtSummaryEntry> {
        &mut self.entity
    }

    /// `CompoundTag.putString("id", ...)` on the entity to spawn
    /// (`BaseSpawner.java:56`).
    pub fn set_entity_resource_id(&mut self, resource_id: &str) {
        if let Some(id) = self.entity.iter_mut().find(|field| field.name == "id") {
            id.value = NbtSummaryValue::string(resource_id);
        } else {
            self.entity
                .push(entry("id", NbtSummaryValue::string(resource_id)));
        }
    }

    pub fn custom_spawn_rules(&self) -> Option<&NbtSummaryValue> {
        self.custom_spawn_rules.as_ref()
    }

    pub fn equipment(&self) -> Option<&NbtSummaryValue> {
        self.equipment.as_ref()
    }

    pub(super) fn from_nbt(value: &NbtSummaryValue) -> Option<Self> {
        let NbtSummaryValue::Compound(fields) = value else {
            return None;
        };
        let NbtSummaryValue::Compound(entity) = field(fields, "entity")? else {
            return None;
        };
        let custom_spawn_rules = optional_compound(fields, "custom_spawn_rules")?;
        let equipment = optional_compound(fields, "equipment")?;
        Some(Self::new(entity.clone(), custom_spawn_rules, equipment))
    }

    pub fn to_nbt(&self) -> NbtSummaryValue {
        let mut fields = vec![entry(
            "entity",
            NbtSummaryValue::Compound(self.entity.clone()),
        )];
        if let Some(rules) = &self.custom_spawn_rules {
            fields.push(entry("custom_spawn_rules", rules.clone()));
        }
        if let Some(equipment) = &self.equipment {
            fields.push(entry("equipment", equipment.clone()));
        }
        NbtSummaryValue::Compound(fields)
    }

    pub fn entity_resource_id(&self) -> Option<&bbb_protocol::JavaString> {
        match field(&self.entity, "id") {
            Some(NbtSummaryValue::String(id)) => Some(id),
            _ => None,
        }
    }

    pub fn entity_type_id(&self) -> Option<i32> {
        self.entity_type_id_with_registry(bbb_protocol::content_registry::ContentRegistry::vanilla())
    }

    pub fn entity_type_id_with_registry(
        &self,
        content_registry: &bbb_protocol::content_registry::ContentRegistry,
    ) -> Option<i32> {
        let name = self.entity_resource_id()?;
        VANILLA_ENTITY_TYPES
            .iter()
            .find(|entry| name == entry.resource_id)
            .map(|entry| entry.id)
            .or_else(|| {
                name.with_utf8(|raw| {
                    let normalized =
                        bbb_protocol::normalize_resource_location(raw.to_owned()).ok()?;
                    content_registry
                        .entity_by_resource_id(&normalized)
                        .map(|spec| spec.id)
                })
                .flatten()
            })
    }

    /// The spawn data's own `Pos`, which both spawner bodies read before they
    /// draw one (`BaseSpawner.java:104-110`, `TrialSpawner.java:174-184`).
    pub(crate) fn explicit_position(&self) -> Option<Vec3d> {
        let NbtSummaryValue::List(values) = field(&self.entity, "Pos")? else {
            return None;
        };
        if values.len() != 3 {
            return None;
        }
        Some(Vec3d {
            x: numeric_f64(&values[0])?,
            y: numeric_f64(&values[1])?,
            z: numeric_f64(&values[2])?,
        })
    }
}

impl Default for SpawnerSpawnData {
    fn default() -> Self {
        Self::new(Vec::new(), None, None)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WeightedSpawnerSpawnData {
    pub data: SpawnerSpawnData,
    pub weight: i32,
}

impl WeightedSpawnerSpawnData {
    pub fn new(data: SpawnerSpawnData, weight: i32) -> Option<Self> {
        (weight >= 0).then_some(Self { data, weight })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct SpawnRequest {
    pub entity_type_id: i32,
    pub position: Vec3d,
    pub spawn_data: SpawnerSpawnData,
    pub spawner_position: BlockPos,
    pub spawn_range: i32,
    pub max_nearby_entities: i32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SpawnAttempt {
    pub ordinal: i32,
    pub request: SpawnRequest,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpawnerRefusal {
    InvalidEntityId,
    Collision,
    PeacefulCustomSpawnRules,
    CustomSpawnRules,
    SpawnPlacement,
    RecursiveEntityLoad,
    MaxNearbyExactClass,
    MobSpawnRules,
    MobObstruction,
    FinalizeSpawn,
    WorldAdd,
}

impl SpawnerRefusal {
    pub const fn resets_delay(self) -> bool {
        matches!(
            self,
            Self::InvalidEntityId
                | Self::RecursiveEntityLoad
                | Self::MaxNearbyExactClass
                | Self::FinalizeSpawn
                | Self::WorldAdd
        )
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum SpawnerTickDecision {
    Dormant,
    Countdown {
        remaining_delay: i32,
        broadcast_event: Option<u8>,
    },
    Attempt(SpawnAttempt),
    Finished {
        spawn_delay: i32,
        broadcast_event: Option<u8>,
        refusal: Option<SpawnerRefusal>,
    },
}

#[derive(Debug, Clone, PartialEq)]
struct SpawnCycle {
    data: SpawnerSpawnData,
    attempted: i32,
    any_success: bool,
    pending_attempt: Option<SpawnAttempt>,
}

/// `BaseSpawner`'s own state (`BaseSpawner.java:42-53`), owned by the block
/// entity that runs it and by the spawner minecart.
#[derive(Debug, Clone, PartialEq)]
pub struct BaseSpawnerData {
    spawn_delay: i32,
    spawn_potentials: Vec<WeightedSpawnerSpawnData>,
    next_spawn_data: Option<SpawnerSpawnData>,
    min_spawn_delay: i32,
    max_spawn_delay: i32,
    spawn_count: i32,
    max_nearby_entities: i32,
    required_player_range: i32,
    spawn_range: i32,
    cycle: Option<SpawnCycle>,
}

impl Default for BaseSpawnerData {
    fn default() -> Self {
        Self::new()
    }
}

impl BaseSpawnerData {
    // `BaseSpawner.java:31-53`.
    pub fn new() -> Self {
        Self {
            spawn_delay: DEFAULT_SPAWN_DELAY,
            spawn_potentials: Vec::new(),
            next_spawn_data: None,
            min_spawn_delay: DEFAULT_MIN_SPAWN_DELAY,
            max_spawn_delay: DEFAULT_MAX_SPAWN_DELAY,
            spawn_count: DEFAULT_SPAWN_COUNT,
            max_nearby_entities: DEFAULT_MAX_NEARBY_ENTITIES,
            required_player_range: DEFAULT_REQUIRED_PLAYER_RANGE,
            spawn_range: DEFAULT_SPAWN_RANGE,
            cycle: None,
        }
    }

    /// The in-flight attempt of a spawn cycle survives a command block edit of
    /// the same object, which is the only state the save codec does not carry.
    pub fn restore_transients(&mut self, previous: &Self) {
        self.cycle = previous.cycle.clone();
    }

    pub const fn spawn_delay(&self) -> i32 {
        self.spawn_delay
    }

    pub fn spawn_potentials(&self) -> &[WeightedSpawnerSpawnData] {
        &self.spawn_potentials
    }

    pub fn next_spawn_data(&self) -> Option<&SpawnerSpawnData> {
        self.next_spawn_data.as_ref()
    }

    pub const fn min_spawn_delay(&self) -> i32 {
        self.min_spawn_delay
    }

    pub const fn max_spawn_delay(&self) -> i32 {
        self.max_spawn_delay
    }

    pub const fn spawn_count(&self) -> i32 {
        self.spawn_count
    }

    pub const fn max_nearby_entities(&self) -> i32 {
        self.max_nearby_entities
    }

    pub const fn required_player_range(&self) -> i32 {
        self.required_player_range
    }

    pub const fn spawn_range(&self) -> i32 {
        self.spawn_range
    }

    pub fn set_spawn_delay(&mut self, spawn_delay: i32) {
        self.spawn_delay = spawn_delay;
    }

    pub fn set_spawn_potentials(&mut self, potentials: Vec<WeightedSpawnerSpawnData>) {
        self.spawn_potentials = potentials;
    }

    pub fn set_next_spawn_data(&mut self, data: Option<SpawnerSpawnData>) {
        self.next_spawn_data = data;
    }

    /// `getOrCreateNextSpawnData` without its draw (`BaseSpawner.java:257-263`),
    /// for the callers that have no random source to spend.
    pub fn next_spawn_data_mut(&mut self) -> &mut SpawnerSpawnData {
        self.next_spawn_data.get_or_insert_with(Default::default)
    }

    /// `BaseSpawner.setEntityId` (`BaseSpawner.java:55-57`): the entity to
    /// spawn is the one `getOrCreateNextSpawnData` answers, so a spawner whose
    /// `SpawnData` is unset still spends the draw.
    pub fn set_entity_id(&mut self, resource_id: &str, random: &GameRandom) {
        // `getOrCreateNextSpawnData` answers the stored object in Java; here it
        // answers a copy, so the mutated one goes back.
        let mut data = self.get_or_create_next_spawn_data(random);
        data.set_entity_resource_id(resource_id);
        self.next_spawn_data = Some(data);
    }

    pub fn set_spawn_parameters(
        &mut self,
        min_spawn_delay: i32,
        max_spawn_delay: i32,
        spawn_count: i32,
        max_nearby_entities: i32,
        required_player_range: i32,
        spawn_range: i32,
    ) {
        self.min_spawn_delay = min_spawn_delay;
        self.max_spawn_delay = max_spawn_delay;
        self.spawn_count = spawn_count;
        self.max_nearby_entities = max_nearby_entities;
        self.required_player_range = required_player_range;
        self.spawn_range = spawn_range;
    }

    /// Fast-path pure countdown (`BaseSpawner.java:88-90`) in place without
    /// cloning or calling `setChanged`.
    pub fn tick_countdown_in_place(&mut self) -> bool {
        if self.cycle.is_none() && self.spawn_delay > 0 {
            self.spawn_delay -= 1;
            true
        } else {
            false
        }
    }

    pub fn server_tick(
        &mut self,
        spawner_position: BlockPos,
        near_alive_player: bool,
        spawner_enabled: bool,
        random: &GameRandom,
    ) -> SpawnerTickDecision {
        self.server_tick_with_registry(
            spawner_position,
            near_alive_player,
            spawner_enabled,
            random,
            bbb_protocol::content_registry::ContentRegistry::vanilla(),
        )
    }

    pub fn server_tick_with_registry(
        &mut self,
        spawner_position: BlockPos,
        near_alive_player: bool,
        spawner_enabled: bool,
        random: &GameRandom,
        content_registry: &bbb_protocol::content_registry::ContentRegistry,
    ) -> SpawnerTickDecision {
        if !near_alive_player || !spawner_enabled {
            return SpawnerTickDecision::Dormant;
        }
        if let Some(ordinal) = self.cycle.as_ref().map(|cycle| cycle.attempted) {
            return self.attempt_decision(spawner_position, ordinal, random, content_registry);
        }

        let mut broadcast_event = None;
        if self.spawn_delay == -1 {
            self.reset_delay(random);
            broadcast_event = Some(SPAWNER_EVENT_ID);
        }
        if self.spawn_delay > 0 {
            self.spawn_delay -= 1;
            return SpawnerTickDecision::Countdown {
                remaining_delay: self.spawn_delay,
                broadcast_event,
            };
        }

        let data = self.get_or_create_next_spawn_data(random);
        if self.spawn_count <= 0 {
            return SpawnerTickDecision::Finished {
                spawn_delay: self.spawn_delay,
                broadcast_event: None,
                refusal: None,
            };
        }
        self.cycle = Some(SpawnCycle {
            data,
            attempted: 0,
            any_success: false,
            pending_attempt: None,
        });
        self.attempt_decision(spawner_position, 0, random, content_registry)
    }

    pub fn resolve_attempt(
        &mut self,
        spawner_position: BlockPos,
        result: Result<(), SpawnerRefusal>,
        random: &GameRandom,
    ) -> SpawnerTickDecision {
        self.resolve_attempt_with_registry(
            spawner_position,
            result,
            random,
            bbb_protocol::content_registry::ContentRegistry::vanilla(),
        )
    }

    pub fn resolve_attempt_with_registry(
        &mut self,
        spawner_position: BlockPos,
        result: Result<(), SpawnerRefusal>,
        random: &GameRandom,
        content_registry: &bbb_protocol::content_registry::ContentRegistry,
    ) -> SpawnerTickDecision {
        let Some(mut cycle) = self.cycle.take() else {
            return SpawnerTickDecision::Finished {
                spawn_delay: self.spawn_delay,
                broadcast_event: None,
                refusal: result.err(),
            };
        };
        cycle.pending_attempt = None;
        cycle.attempted += 1;
        match result {
            Ok(()) => cycle.any_success = true,
            Err(refusal) if refusal.resets_delay() => {
                self.reset_delay(random);
                return SpawnerTickDecision::Finished {
                    spawn_delay: self.spawn_delay,
                    broadcast_event: Some(SPAWNER_EVENT_ID),
                    refusal: Some(refusal),
                };
            }
            Err(_) => {}
        }

        if cycle.attempted < self.spawn_count {
            let ordinal = cycle.attempted;
            self.cycle = Some(cycle);
            return self.attempt_decision(spawner_position, ordinal, random, content_registry);
        }
        if cycle.any_success {
            self.reset_delay(random);
            SpawnerTickDecision::Finished {
                spawn_delay: self.spawn_delay,
                broadcast_event: Some(SPAWNER_EVENT_ID),
                refusal: result.err(),
            }
        } else {
            SpawnerTickDecision::Finished {
                spawn_delay: self.spawn_delay,
                broadcast_event: None,
                refusal: result.err(),
            }
        }
    }

    pub fn handle_event(&mut self, id: u8, client_side: bool) -> bool {
        if id != SPAWNER_EVENT_ID {
            return false;
        }
        if client_side {
            self.spawn_delay = self.min_spawn_delay;
        }
        true
    }

    // `BaseSpawner.java:200-212`.
    pub fn read_save_data(entries: &[NbtSummaryEntry]) -> Self {
        let mut data = Self::new();
        data.spawn_delay = numeric_i32(field(entries, "Delay")).unwrap_or(DEFAULT_SPAWN_DELAY);
        data.next_spawn_data = field(entries, "SpawnData").and_then(SpawnerSpawnData::from_nbt);
        data.spawn_potentials = field(entries, "SpawnPotentials")
            .and_then(read_potentials)
            .unwrap_or_else(|| {
                vec![WeightedSpawnerSpawnData {
                    data: data.next_spawn_data.clone().unwrap_or_default(),
                    weight: 1,
                }]
            });
        data.min_spawn_delay =
            numeric_i32(field(entries, "MinSpawnDelay")).unwrap_or(DEFAULT_MIN_SPAWN_DELAY);
        data.max_spawn_delay =
            numeric_i32(field(entries, "MaxSpawnDelay")).unwrap_or(DEFAULT_MAX_SPAWN_DELAY);
        data.spawn_count = numeric_i32(field(entries, "SpawnCount")).unwrap_or(DEFAULT_SPAWN_COUNT);
        data.max_nearby_entities =
            numeric_i32(field(entries, "MaxNearbyEntities")).unwrap_or(DEFAULT_MAX_NEARBY_ENTITIES);
        data.required_player_range = numeric_i32(field(entries, "RequiredPlayerRange"))
            .unwrap_or(DEFAULT_REQUIRED_PLAYER_RANGE);
        data.spawn_range = numeric_i32(field(entries, "SpawnRange")).unwrap_or(DEFAULT_SPAWN_RANGE);
        data
    }

    // `BaseSpawner.java:214-224`.
    pub fn write_save_data(&self, entries: &mut Vec<NbtSummaryEntry>) {
        entries.extend([
            entry("Delay", NbtSummaryValue::Short(self.spawn_delay as i16)),
            entry(
                "MinSpawnDelay",
                NbtSummaryValue::Short(self.min_spawn_delay as i16),
            ),
            entry(
                "MaxSpawnDelay",
                NbtSummaryValue::Short(self.max_spawn_delay as i16),
            ),
            entry(
                "SpawnCount",
                NbtSummaryValue::Short(self.spawn_count as i16),
            ),
            entry(
                "MaxNearbyEntities",
                NbtSummaryValue::Short(self.max_nearby_entities as i16),
            ),
            entry(
                "RequiredPlayerRange",
                NbtSummaryValue::Short(self.required_player_range as i16),
            ),
            entry(
                "SpawnRange",
                NbtSummaryValue::Short(self.spawn_range as i16),
            ),
        ]);
        if let Some(next) = &self.next_spawn_data {
            entries.push(entry("SpawnData", next.to_nbt()));
        }
        entries.push(entry(
            "SpawnPotentials",
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

    fn attempt_decision(
        &mut self,
        spawner_position: BlockPos,
        ordinal: i32,
        random: &GameRandom,
        content_registry: &bbb_protocol::content_registry::ContentRegistry,
    ) -> SpawnerTickDecision {
        if let Some(attempt) = self
            .cycle
            .as_ref()
            .and_then(|cycle| cycle.pending_attempt.clone())
        {
            return SpawnerTickDecision::Attempt(attempt);
        }
        let data = self
            .cycle
            .as_ref()
            .expect("spawn cycle exists")
            .data
            .clone();
        let Some(entity_type_id) = data.entity_type_id_with_registry(content_registry) else {
            self.cycle = None;
            self.reset_delay(random);
            return SpawnerTickDecision::Finished {
                spawn_delay: self.spawn_delay,
                broadcast_event: Some(SPAWNER_EVENT_ID),
                refusal: Some(SpawnerRefusal::InvalidEntityId),
            };
        };
        let position = data.explicit_position().unwrap_or_else(|| Vec3d {
            x: f64::from(spawner_position.x)
                + (random.next_double() - random.next_double()) * f64::from(self.spawn_range)
                + 0.5,
            y: f64::from(spawner_position.y) + f64::from(random.next_int(3)) - 1.0,
            z: f64::from(spawner_position.z)
                + (random.next_double() - random.next_double()) * f64::from(self.spawn_range)
                + 0.5,
        });
        let attempt = SpawnAttempt {
            ordinal,
            request: SpawnRequest {
                entity_type_id,
                position,
                spawn_data: data,
                spawner_position,
                spawn_range: self.spawn_range,
                max_nearby_entities: self.max_nearby_entities,
            },
        };
        self.cycle
            .as_mut()
            .expect("spawn cycle exists")
            .pending_attempt = Some(attempt.clone());
        SpawnerTickDecision::Attempt(attempt)
    }

    fn reset_delay(&mut self, random: &GameRandom) {
        self.spawn_delay = if self.max_spawn_delay <= self.min_spawn_delay {
            self.min_spawn_delay
        } else {
            self.min_spawn_delay + random.next_int(self.max_spawn_delay - self.min_spawn_delay)
        };
        if let Some(selected) = weighted_choice(&self.spawn_potentials, random) {
            self.next_spawn_data = Some(selected);
        }
    }

    fn get_or_create_next_spawn_data(&mut self, random: &GameRandom) -> SpawnerSpawnData {
        if let Some(data) = &self.next_spawn_data {
            return data.clone();
        }
        let data = weighted_choice(&self.spawn_potentials, random).unwrap_or_default();
        self.next_spawn_data = Some(data.clone());
        data
    }
}

fn weighted_choice(
    potentials: &[WeightedSpawnerSpawnData],
    random: &GameRandom,
) -> Option<SpawnerSpawnData> {
    let total = potentials.iter().try_fold(0_i32, |total, potential| {
        total.checked_add(potential.weight)
    })?;
    if total == 0 {
        return None;
    }
    let mut selection = random.next_int(total);
    for potential in potentials {
        if selection < potential.weight {
            return Some(potential.data.clone());
        }
        selection -= potential.weight;
    }
    None
}

pub(super) fn read_potentials(value: &NbtSummaryValue) -> Option<Vec<WeightedSpawnerSpawnData>> {
    let NbtSummaryValue::List(values) = value else {
        return None;
    };
    let mut result = Vec::with_capacity(values.len());
    for value in values {
        let NbtSummaryValue::Compound(fields) = value else {
            return None;
        };
        let data = SpawnerSpawnData::from_nbt(field(fields, "data")?)?;
        let weight = numeric_i32(field(fields, "weight"))?;
        result.push(WeightedSpawnerSpawnData::new(data, weight)?);
    }
    result
        .iter()
        .try_fold(0_i32, |total, entry| total.checked_add(entry.weight))?;
    Some(result)
}

fn canonicalize_entity_id(mut entity: Vec<NbtSummaryEntry>) -> Vec<NbtSummaryEntry> {
    let Some(index) = entity.iter().position(|entry| entry.name == "id") else {
        return entity;
    };
    let canonical = match &entity[index].value {
        NbtSummaryValue::String(id) => id
            .try_to_string()
            .ok()
            .and_then(|raw| bbb_protocol::normalize_resource_location(raw).ok()),
        _ => None,
    };
    if let Some(id) = canonical {
        entity[index].value = NbtSummaryValue::string(id);
    } else {
        entity.remove(index);
    }
    entity
}

pub fn numeric_i32(value: Option<&NbtSummaryValue>) -> Option<i32> {
    match value? {
        NbtSummaryValue::Byte(value) => Some(i32::from(*value)),
        NbtSummaryValue::Short(value) => Some(i32::from(*value)),
        NbtSummaryValue::Int(value) => Some(*value),
        NbtSummaryValue::Long(value) => i32::try_from(*value).ok(),
        _ => None,
    }
}

fn optional_compound(entries: &[NbtSummaryEntry], name: &str) -> Option<Option<NbtSummaryValue>> {
    match field(entries, name) {
        None => Some(None),
        Some(value @ NbtSummaryValue::Compound(_)) => Some(Some(value.clone())),
        Some(_) => None,
    }
}
