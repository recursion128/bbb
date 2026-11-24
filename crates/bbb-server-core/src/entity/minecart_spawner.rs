use crate::random::GameRandom;
use bbb_protocol::NbtSummaryEntry;
use uuid::Uuid;

use crate::entity::minecart::{self, MinecartData};
use crate::entity::save::EntitySaveError;

#[path = "spawn_placement.rs"]
pub mod spawn_placement;
#[path = "spawner_runtime.rs"]
pub mod spawner_runtime;

pub(crate) use spawn_placement::check_spawn_placement_predicate;
pub use spawn_placement::{
    is_spawn_position_ok, spawn_placement, spawn_placement_by_id, SpawnFluid, SpawnHeightmap,
    SpawnPlacementEntry, SpawnPlacementEnvironment, SpawnPlacementProbe, SpawnPlacementType,
    SpawnPredicateKind, SpawnPredicateReason,
};
pub use spawner_runtime::{
    difficulty_special_multiplier, effective_difficulty, equip_spawner_mob, finalize_spawner_mob,
    prepare_spawn_request, AtomicSpawnPlan, CustomSpawnRules, EquipmentPlan,
    EquipmentSlotDropChance, SpawnEntityTree,
};

/// `BaseSpawner`'s own state and decisions, shared with `SpawnerBlockEntity`
/// (`crate::block_entity::base_spawner`).
pub use crate::block_entity::base_spawner::{
    BaseSpawnerData, SpawnAttempt, SpawnRequest, SpawnerRefusal, SpawnerSpawnData,
    SpawnerTickDecision, WeightedSpawnerSpawnData, DEFAULT_MAX_NEARBY_ENTITIES,
    DEFAULT_MAX_SPAWN_DELAY, DEFAULT_MIN_SPAWN_DELAY, DEFAULT_REQUIRED_PLAYER_RANGE,
    DEFAULT_SPAWN_COUNT, DEFAULT_SPAWN_DELAY, DEFAULT_SPAWN_RANGE, SPAWNER_EVENT_ID,
};

pub const DROP_AND_PICK_ITEM_RESOURCE_ID: &str = "minecraft:minecart";
pub const DEFAULT_DISPLAY_BLOCK_RESOURCE_ID: &str = "minecraft:spawner";

#[derive(Debug, Clone, PartialEq)]
pub struct PendingSpawnerMinecartAttempt {
    pub minecart_entity_id: i32,
    pub minecart_uuid: Uuid,
    pub attempt: SpawnAttempt,
}

/// `MinecartSpawner`'s data (`MinecartSpawner.java:16-49`) over the shared
/// `BaseSpawner` state.
#[derive(Debug, Clone, PartialEq)]
pub struct SpawnerMinecartData {
    minecart: MinecartData,
    base: BaseSpawnerData,
}

impl SpawnerMinecartData {
    pub(crate) fn restore_command_transients(&mut self, previous: &Self) {
        self.minecart.restore_command_transients(&previous.minecart);
        self.base.restore_transients(&previous.base);
    }

    pub fn new() -> Self {
        Self {
            minecart: MinecartData::new(),
            base: BaseSpawnerData::new(),
        }
    }

    pub const fn minecart(&self) -> &MinecartData {
        &self.minecart
    }

    pub fn minecart_mut(&mut self) -> &mut MinecartData {
        &mut self.minecart
    }

    pub const fn base(&self) -> &BaseSpawnerData {
        &self.base
    }

    pub fn base_mut(&mut self) -> &mut BaseSpawnerData {
        &mut self.base
    }

    pub const fn spawn_delay(&self) -> i32 {
        self.base.spawn_delay()
    }

    pub fn spawn_potentials(&self) -> &[WeightedSpawnerSpawnData] {
        self.base.spawn_potentials()
    }

    pub fn next_spawn_data(&self) -> Option<&SpawnerSpawnData> {
        self.base.next_spawn_data()
    }

    pub const fn min_spawn_delay(&self) -> i32 {
        self.base.min_spawn_delay()
    }

    pub const fn max_spawn_delay(&self) -> i32 {
        self.base.max_spawn_delay()
    }

    pub const fn spawn_count(&self) -> i32 {
        self.base.spawn_count()
    }

    pub const fn max_nearby_entities(&self) -> i32 {
        self.base.max_nearby_entities()
    }

    pub const fn required_player_range(&self) -> i32 {
        self.base.required_player_range()
    }

    pub const fn spawn_range(&self) -> i32 {
        self.base.spawn_range()
    }

    pub fn set_spawn_delay(&mut self, spawn_delay: i32) {
        self.base.set_spawn_delay(spawn_delay);
    }

    pub fn set_spawn_potentials(&mut self, potentials: Vec<WeightedSpawnerSpawnData>) {
        self.base.set_spawn_potentials(potentials);
    }

    pub fn set_next_spawn_data(&mut self, data: Option<SpawnerSpawnData>) {
        self.base.set_next_spawn_data(data);
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
        self.base.set_spawn_parameters(
            min_spawn_delay,
            max_spawn_delay,
            spawn_count,
            max_nearby_entities,
            required_player_range,
            spawn_range,
        );
    }

    // `BaseSpawner.java:82-111`.
    pub fn server_tick(
        &mut self,
        spawner_position: bbb_protocol::packets::BlockPos,
        near_alive_player: bool,
        spawner_enabled: bool,
        random: &GameRandom,
    ) -> SpawnerTickDecision {
        self.base
            .server_tick(spawner_position, near_alive_player, spawner_enabled, random)
    }

    pub fn server_tick_with_registry(
        &mut self,
        spawner_position: bbb_protocol::packets::BlockPos,
        near_alive_player: bool,
        spawner_enabled: bool,
        random: &GameRandom,
        content_registry: &bbb_protocol::content_registry::ContentRegistry,
    ) -> SpawnerTickDecision {
        self.base.server_tick_with_registry(
            spawner_position,
            near_alive_player,
            spawner_enabled,
            random,
            content_registry,
        )
    }

    // `BaseSpawner.java:95-181` leaves every world-dependent check to the caller.
    pub fn resolve_attempt(
        &mut self,
        spawner_position: bbb_protocol::packets::BlockPos,
        result: Result<(), SpawnerRefusal>,
        random: &GameRandom,
    ) -> SpawnerTickDecision {
        self.base.resolve_attempt(spawner_position, result, random)
    }

    pub fn resolve_attempt_with_registry(
        &mut self,
        spawner_position: bbb_protocol::packets::BlockPos,
        result: Result<(), SpawnerRefusal>,
        random: &GameRandom,
        content_registry: &bbb_protocol::content_registry::ContentRegistry,
    ) -> SpawnerTickDecision {
        self.base
            .resolve_attempt_with_registry(spawner_position, result, random, content_registry)
    }

    // `BaseSpawner.java:241-250`.
    pub fn handle_event(&mut self, id: u8, client_side: bool) -> bool {
        self.base.handle_event(id, client_side)
    }

    // `BaseSpawner.java:200-223`.
    pub fn read_save_data(entries: &[NbtSummaryEntry]) -> Self {
        Self::read_save_data_with_registry(
            entries,
            bbb_protocol::content_registry::ContentRegistry::vanilla().as_ref(),
        )
    }

    pub(crate) fn read_save_data_with_registry(
        entries: &[NbtSummaryEntry],
        content_registry: &bbb_protocol::content_registry::ContentRegistry,
    ) -> Self {
        Self {
            minecart: minecart::read_save_data_with_registry(entries, content_registry),
            base: BaseSpawnerData::read_save_data(entries),
        }
    }

    pub fn write_save_data(
        &self,
        entries: &mut Vec<NbtSummaryEntry>,
    ) -> Result<(), EntitySaveError> {
        self.write_save_data_with_registry(
            entries,
            bbb_protocol::content_registry::ContentRegistry::vanilla().as_ref(),
        )
    }

    pub(crate) fn write_save_data_with_registry(
        &self,
        entries: &mut Vec<NbtSummaryEntry>,
        content_registry: &bbb_protocol::content_registry::ContentRegistry,
    ) -> Result<(), EntitySaveError> {
        minecart::write_save_data_with_registry(entries, &self.minecart, content_registry)?;
        self.base.write_save_data(entries);
        Ok(())
    }
}

impl Default for SpawnerMinecartData {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bbb_protocol::packets::{BlockPos, Vec3d};
    use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue};

    fn entry(name: &str, value: NbtSummaryValue) -> NbtSummaryEntry {
        NbtSummaryEntry {
            name: name.into(),
            value,
        }
    }

    fn spawn_data(id: &str) -> SpawnerSpawnData {
        SpawnerSpawnData::new(vec![entry("id", NbtSummaryValue::string(id))], None, None)
    }

    fn ready(data: SpawnerSpawnData) -> SpawnerMinecartData {
        let mut spawner = SpawnerMinecartData::new();
        spawner.set_spawn_delay(0);
        spawner.set_next_spawn_data(Some(data));
        spawner
    }

    fn origin() -> BlockPos {
        BlockPos {
            x: 10,
            y: 64,
            z: -3,
        }
    }

    #[test]
    fn defaults_match_base_spawner() {
        let data = SpawnerMinecartData::new();
        assert_eq!(data.spawn_delay(), 20);
        assert_eq!(data.min_spawn_delay(), 200);
        assert_eq!(data.max_spawn_delay(), 800);
        assert_eq!(data.spawn_count(), 4);
        assert_eq!(data.max_nearby_entities(), 6);
        assert_eq!(data.required_player_range(), 16);
        assert_eq!(data.spawn_range(), 4);
        assert_eq!(data.minecart(), &MinecartData::new());
    }

    #[test]
    fn near_player_and_enabled_gate_keep_the_spawner_dormant() {
        let mut data = ready(spawn_data("minecraft:pig"));
        let random = GameRandom::with_seed(0);
        assert_eq!(
            data.server_tick(origin(), false, true, &random),
            SpawnerTickDecision::Dormant
        );
        assert_eq!(
            data.server_tick(origin(), true, false, &random),
            SpawnerTickDecision::Dormant
        );
        assert_eq!(data.spawn_delay(), 0);
    }

    #[test]
    fn minus_one_resets_then_positive_delay_decrements() {
        let mut data = ready(spawn_data("minecraft:pig"));
        data.set_spawn_delay(-1);
        data.set_spawn_parameters(5, 9, 4, 6, 16, 4);
        let random = GameRandom::with_seed(0);
        let SpawnerTickDecision::Countdown {
            remaining_delay,
            broadcast_event: Some(1),
        } = data.server_tick(origin(), true, true, &random)
        else {
            panic!("expected a reset countdown");
        };
        // A reset delay in `5..9`, already counted down once.
        assert!((4..8).contains(&remaining_delay));
        assert_eq!(
            data.server_tick(origin(), true, true, &random),
            SpawnerTickDecision::Countdown {
                remaining_delay: remaining_delay - 1,
                broadcast_event: None,
            }
        );
    }

    #[test]
    fn zero_delay_attempts_inside_the_spawn_box_and_keeps_the_pending_attempt() {
        let mut data = ready(spawn_data("minecraft:pig"));
        let random = GameRandom::with_seed(0);
        let SpawnerTickDecision::Attempt(attempt) = data.server_tick(origin(), true, true, &random)
        else {
            panic!("expected attempt");
        };
        assert_eq!(attempt.ordinal, 0);
        let position = attempt.request.position;
        // A spawn range of 4 around the block centre, one block up or down.
        assert!((position.x - 10.5).abs() <= 4.0);
        assert!([63.0, 64.0, 65.0].contains(&position.y));
        assert!((position.z + 2.5).abs() <= 4.0);
        assert_eq!(
            data.server_tick(origin(), true, true, &random),
            SpawnerTickDecision::Attempt(attempt),
        );
    }

    #[test]
    fn explicit_position_overrides_the_random_spawn_box() {
        let data = SpawnerSpawnData::new(
            vec![
                entry("id", NbtSummaryValue::string("minecraft:pig")),
                entry(
                    "Pos",
                    NbtSummaryValue::List(vec![
                        NbtSummaryValue::Double(1.25_f64.to_bits()),
                        NbtSummaryValue::Int(70),
                        NbtSummaryValue::Double((-2.5_f64).to_bits()),
                    ]),
                ),
            ],
            None,
            None,
        );
        let mut spawner = ready(data);
        let random = GameRandom::with_seed(0);
        let SpawnerTickDecision::Attempt(attempt) =
            spawner.server_tick(origin(), true, true, &random)
        else {
            panic!("expected attempt");
        };
        assert_eq!(
            attempt.request.position,
            Vec3d {
                x: 1.25,
                y: 70.0,
                z: -2.5
            }
        );
    }

    #[test]
    fn weighted_selection_is_used_for_creation_and_delay() {
        let pig = spawn_data("minecraft:pig");
        let cow = spawn_data("minecraft:cow");
        let mut data = SpawnerMinecartData::new();
        data.set_spawn_delay(0);
        data.set_spawn_parameters(8, 8, 1, 6, 16, 4);
        data.set_spawn_potentials(vec![
            WeightedSpawnerSpawnData::new(pig, 1).unwrap(),
            WeightedSpawnerSpawnData::new(cow.clone(), 3).unwrap(),
        ]);
        let random = GameRandom::with_seed(0);
        // Weights 1:3, so 300 of 400 picks are cows on average (sd ~8.7).
        let mut cows = 0;
        for _ in 0..400 {
            let SpawnerTickDecision::Attempt(attempt) =
                data.clone().server_tick(origin(), true, true, &random)
            else {
                panic!("expected attempt");
            };
            cows += usize::from(attempt.request.spawn_data == cow);
        }
        assert!((240..=360).contains(&cows), "{cows} cows");
        let SpawnerTickDecision::Attempt(_) = data.server_tick(origin(), true, true, &random)
        else {
            panic!("expected attempt");
        };
        let finished = data.resolve_attempt(origin(), Ok(()), &random);
        assert!(matches!(
            finished,
            SpawnerTickDecision::Finished {
                broadcast_event: Some(1),
                ..
            }
        ));
    }

    #[test]
    fn invalid_entity_id_resets_and_finishes() {
        let mut data = ready(spawn_data("minecraft:not_a_real_entity"));
        data.set_spawn_parameters(12, 12, 4, 6, 16, 4);
        let random = GameRandom::with_seed(0);
        assert_eq!(
            data.server_tick(origin(), true, true, &random),
            SpawnerTickDecision::Finished {
                spawn_delay: 12,
                broadcast_event: Some(1),
                refusal: Some(SpawnerRefusal::InvalidEntityId),
            }
        );
    }

    #[test]
    fn only_success_or_terminal_refusal_resets_delay() {
        let mut no_success = ready(spawn_data("minecraft:pig"));
        no_success.set_spawn_parameters(9, 9, 1, 6, 16, 4);
        let position_random = GameRandom::with_seed(0);
        assert!(matches!(
            no_success.server_tick(origin(), true, true, &position_random),
            SpawnerTickDecision::Attempt(_)
        ));
        let unused_random = GameRandom::with_seed(0);
        assert_eq!(
            no_success.resolve_attempt(
                origin(),
                Err(SpawnerRefusal::SpawnPlacement),
                &unused_random,
            ),
            SpawnerTickDecision::Finished {
                spawn_delay: 0,
                broadcast_event: None,
                refusal: Some(SpawnerRefusal::SpawnPlacement),
            }
        );

        let mut success = ready(spawn_data("minecraft:pig"));
        success.set_spawn_parameters(9, 9, 1, 6, 16, 4);
        let position_random = GameRandom::with_seed(0);
        success.server_tick(origin(), true, true, &position_random);
        assert_eq!(
            success.resolve_attempt(origin(), Ok(()), &unused_random),
            SpawnerTickDecision::Finished {
                spawn_delay: 9,
                broadcast_event: Some(1),
                refusal: None,
            }
        );
    }

    #[test]
    fn event_one_is_consumed_and_client_resets_delay() {
        let mut data = SpawnerMinecartData::new();
        data.set_spawn_delay(3);
        assert!(!data.handle_event(2, true));
        assert_eq!(data.spawn_delay(), 3);
        assert!(data.handle_event(1, false));
        assert_eq!(data.spawn_delay(), 3);
        assert!(data.handle_event(1, true));
        assert_eq!(data.spawn_delay(), 200);
    }

    #[test]
    fn full_nbt_round_trip_preserves_opaque_payloads() {
        let opaque_rules = NbtSummaryValue::Compound(vec![entry(
            "block_light_limit",
            NbtSummaryValue::Compound(vec![entry("max_inclusive", NbtSummaryValue::Int(7))]),
        )]);
        let opaque_equipment = NbtSummaryValue::Compound(vec![entry(
            "loot_table",
            NbtSummaryValue::string("minecraft:equipment/trial_chamber"),
        )]);
        let spawn = SpawnerSpawnData::new(
            vec![
                entry("id", NbtSummaryValue::string("pig")),
                entry("Health", NbtSummaryValue::Float(12.5_f32.to_bits())),
                entry(
                    "Passengers",
                    NbtSummaryValue::List(vec![NbtSummaryValue::Compound(vec![entry(
                        "id",
                        NbtSummaryValue::string("minecraft:chicken"),
                    )])]),
                ),
            ],
            Some(opaque_rules),
            Some(opaque_equipment),
        );
        let mut data = SpawnerMinecartData::new();
        data.set_spawn_delay(31);
        data.set_spawn_parameters(40, 90, 2, 3, 24, 7);
        data.set_next_spawn_data(Some(spawn.clone()));
        data.set_spawn_potentials(vec![
            WeightedSpawnerSpawnData::new(spawn.clone(), 5).unwrap(),
            WeightedSpawnerSpawnData::new(spawn_data("minecraft:cow"), 2).unwrap(),
        ]);
        let mut entries = Vec::new();
        data.write_save_data(&mut entries).unwrap();
        let restored = SpawnerMinecartData::read_save_data(&entries);
        assert_eq!(restored, data);
        assert_eq!(
            restored.next_spawn_data().unwrap().entity_resource_id(),
            Some(&bbb_protocol::JavaString::from("minecraft:pig"))
        );
    }

    #[test]
    fn missing_potentials_fall_back_to_next_data() {
        let spawn = spawn_data("minecraft:pig");
        let entries = vec![entry("SpawnData", spawn.to_nbt())];
        let restored = SpawnerMinecartData::read_save_data(&entries);
        assert_eq!(
            restored.spawn_potentials(),
            &[WeightedSpawnerSpawnData::new(spawn, 1).unwrap()]
        );
    }
}
