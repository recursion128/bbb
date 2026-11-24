use crate::random::GameRandom;
use bbb_facts::activity_types::VANILLA_ACTIVITIES;
use bbb_protocol::packets::{BlockPos, Vec3d};
use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue};
use uuid::Uuid;

use super::behaviors::PANIC_CAUSES_TAG;
use crate::entity::ai::mob::brain_runtime::BrainBodyFacts;
use crate::entity::ai::mob::frog::FROG_FOOD_TAG;
use crate::entity::ai::BrainBreedFacts;

use super::*;

/// The world a [`BrainTickContext`] brain reads: every query answers what the test
/// staged on it, and nothing else. A query the test did not stage answers the
/// refusal its own field defaults to, so a sensor or behaviour that reads it
/// is visible as a missing stage rather than as a silent pass.
pub(crate) struct TestWorld {
    pub(crate) body: BrainBodyFacts,
    pub(crate) passengers: Vec<i32>,
    pub(crate) entity_facts: Option<BrainEntityFacts>,
    /// What the cross-mob `Animal` facts answer, by entity id
    /// (`BrainWorld::entity_breed_facts`). An id absent here is a candidate
    /// this world cannot answer for.
    pub(crate) breed_facts: Vec<(i32, BrainBreedFacts)>,
    pub(crate) panic_causes: bool,
    pub(crate) navigation_stops: u32,
    pub(crate) game_time: i64,
    pub(crate) body_can_attack: bool,
    pub(crate) entity_type_tags: bool,
    /// The one item type id `#minecraft:frog_food` names for this world, and
    /// what the tag answers for it. Every other item is a non-member, as it is
    /// for the real `slime_ball`-only item tag.
    pub(crate) tempting_item_id: Option<i32>,
    /// What `ItemStack.is(#minecraft:frog_food)` answers for
    /// `tempting_item_id`.
    pub(crate) item_tags: bool,
    pub(crate) nearest_living: Vec<i32>,
    pub(crate) nearest_players: Vec<i32>,
    /// What `TemptingSensor`'s own scan answers,
    /// which is the same projection as `nearest_players` with the two hands.
    pub(crate) players_within: Vec<BrainPlayer>,
    pub(crate) line_of_sight: bool,
    pub(crate) targeting: bool,
    /// What `BrainWorld::level_random_next_int` answers.
    pub(crate) level_random_int: i32,
    /// What `targeting_test` answers for a `combat` test, so a sensor's two
    /// conditions can be staged apart.
    pub(crate) combat_targeting: bool,
    /// Every `TargetingConditions` projection a sensor asked this world to
    /// test, in order, which is how a test pins the condition one builds
    /// without re-deriving it.
    pub(crate) targeting_tests: std::cell::RefCell<Vec<EntityTargetingTest>>,
    pub(crate) distance_to_sqr: Option<f64>,
    pub(crate) path_distance: Option<f32>,
    pub(crate) navigation_done: bool,
    pub(crate) navigation_stuck: bool,
    pub(crate) path: Option<MemoryPath>,
    pub(crate) move_to_result: Option<MemoryPath>,
    pub(crate) move_to_calls: Vec<(BlockPos, i32, f32)>,
    pub(crate) prepare_path_results: std::collections::VecDeque<Option<MemoryPath>>,
    pub(crate) prepare_path_calls: Vec<(BlockPos, i32)>,
    pub(crate) prepared_path: Option<MemoryPath>,
    pub(crate) prepared_target: Option<(BlockPos, i32)>,
    pub(crate) towards_result: Option<Option<Vec3d>>,
    pub(crate) towards_calls: Vec<(i32, i32, Vec3d, f64)>,
    pub(crate) look_at: Option<Vec3d>,
    pub(crate) tongue_target: Option<i32>,
    /// `body.getJumpControl().jump()`, counted.
    pub(crate) jump_requests: u32,
    /// What `LandRandomPos.getPos(body, horizontalDist, verticalDist)`
    /// answers: the outer value is the level's own answer, so a test can stage
    /// an unanswerable sample apart from a `null` position.
    pub(crate) land_random_pos: Option<Option<Vec3d>>,
    pub(crate) swim_target_pos: Option<Option<Vec3d>>,
    pub(crate) swim_target_pos_calls: u32,
    /// The `(horizontalDist, verticalDist)` of every sample a behaviour drew,
    /// in order, which is the one seam both `RandomStroll` and `AnimalPanic`
    /// take.
    pub(crate) land_random_pos_calls: Vec<(i32, i32)>,
    /// What `AnimalPanic.lookForWater`'s bounded search answers
    /// and the distances it was asked with.
    pub(crate) closest_water: Option<Option<BlockPos>>,
    pub(crate) closest_water_calls: Vec<(i32, i32, bool)>,
    pub(crate) effects: Vec<BrainWorldEffect>,
    /// Whether the next `HurtEntity` a behaviour queues takes the staged
    /// target's health to zero, which is what the production store does when
    /// it applies the effect.
    pub(crate) dies_on_hurt: bool,
    /// What `Mob.getPathfindingMalus(PathType)` answers, and the types it was
    /// asked for in order.
    pub(crate) pathfinding_malus: f32,
    pub(crate) pathfinding_malus_calls: std::cell::RefCell<Vec<crate::entity::GroundPathType>>,
    /// What `Level.noCollision(Entity, AABB)` answers for every box
    /// `LongJumpUtil` samples, and the boxes asked
    /// about in order.
    pub(crate) can_fit: bool,
    pub(crate) can_fit_calls: std::cell::RefCell<Vec<bbb_server_types::geometry::Aabb>>,
    /// What `body.getNavigation().createPath(pos, reachRange, maxPathLength)`
    /// answers as `Path.canReach()`, and the probes asked for in order.
    /// `None` is Vanilla's `null` path, so a
    /// test stages it apart from an unreached one.
    pub(crate) path_reaches: Option<bool>,
    pub(crate) path_reaches_calls: std::cell::RefCell<Vec<(BlockPos, i32, f32)>>,
    /// What `BlockGetter.getBlockState`/`LevelReader.getFluidState` facts the
    /// samplers read come from, and the positions asked for in order. A
    /// position absent here is one this world cannot answer.
    pub(crate) block_facts: Vec<(BlockPos, crate::entity::ai::BrainBlockFacts)>,
    pub(crate) block_facts_calls: std::cell::RefCell<Vec<BlockPos>>,
    /// What `WalkNodeEvaluator.getPathTypeStatic` answers
    /// and the positions it was asked
    /// about in order.
    pub(crate) path_types: Vec<(BlockPos, crate::entity::GroundPathType)>,
    pub(crate) path_type_calls: std::cell::RefCell<Vec<BlockPos>>,
}

impl Default for TestWorld {
    fn default() -> Self {
        Self {
            body: BrainBodyFacts {
                entity_id: 1,
                eye_height: 1.62,
                pose_id: bbb_protocol::entity_poses::VANILLA_ENTITY_POSE_STANDING_ID,
                bounding_box: bbb_server_types::geometry::Aabb::of_entity(
                    Vec3d::default(),
                    0.6,
                    1.8,
                ),
                follow_range: 16.0,
                tempt_range: 10.0,
                on_ground: true,
                gravity: 0.08,
                jump_strength: 0.42,
                ..BrainBodyFacts::default()
            },
            passengers: Vec::new(),
            entity_facts: None,
            breed_facts: Vec::new(),
            panic_causes: false,
            navigation_stops: 0,
            game_time: 0,
            body_can_attack: true,
            entity_type_tags: false,
            tempting_item_id: None,
            item_tags: false,
            nearest_living: Vec::new(),
            nearest_players: Vec::new(),
            players_within: Vec::new(),
            line_of_sight: true,
            level_random_int: 0,
            targeting: true,
            combat_targeting: true,
            targeting_tests: std::cell::RefCell::new(Vec::new()),
            distance_to_sqr: None,
            path_distance: None,
            navigation_done: true,
            navigation_stuck: false,
            path: None,
            move_to_result: None,
            move_to_calls: Vec::new(),
            prepare_path_results: std::collections::VecDeque::new(),
            prepare_path_calls: Vec::new(),
            prepared_path: None,
            prepared_target: None,
            towards_result: Some(None),
            towards_calls: Vec::new(),
            look_at: None,
            tongue_target: None,
            jump_requests: 0,
            land_random_pos: Some(None),
            swim_target_pos: Some(None),
            swim_target_pos_calls: 0,
            land_random_pos_calls: Vec::new(),
            closest_water: Some(None),
            closest_water_calls: Vec::new(),
            effects: Vec::new(),
            dies_on_hurt: false,
            pathfinding_malus: 0.0,
            pathfinding_malus_calls: std::cell::RefCell::new(Vec::new()),
            can_fit: true,
            can_fit_calls: std::cell::RefCell::new(Vec::new()),
            path_reaches: None,
            path_reaches_calls: std::cell::RefCell::new(Vec::new()),
            block_facts: Vec::new(),
            block_facts_calls: std::cell::RefCell::new(Vec::new()),
            path_types: Vec::new(),
            path_type_calls: std::cell::RefCell::new(Vec::new()),
        }
    }
}

impl TestWorld {
    /// The calls a behaviour queued, in order (`BrainWorld::queue_effect`).
    pub(crate) fn effect_names(&self) -> Vec<&'static str> {
        self.effects
            .iter()
            .map(|effect| match effect {
                BrainWorldEffect::SetPose { .. } => "SetPose",
                BrainWorldEffect::PlayBodySound { .. } => "PlayBodySound",
                BrainWorldEffect::SetDeltaMovement { .. } => "SetDeltaMovement",
                BrainWorldEffect::HurtEntity { .. } => "HurtEntity",
                BrainWorldEffect::SwingMainHand => "SwingMainHand",
                BrainWorldEffect::MeleeAttack { .. } => "MeleeAttack",
                BrainWorldEffect::SetBodyDeltaMovement { .. } => "SetBodyDeltaMovement",
                BrainWorldEffect::SetBodyYRot { .. } => "SetBodyYRot",
                BrainWorldEffect::SetBodyDiscardFriction { .. } => "SetBodyDiscardFriction",
                BrainWorldEffect::PlaceBlock { .. } => "PlaceBlock",
                BrainWorldEffect::SetMobBrainMemory { .. } => "SetMobBrainMemory",
                BrainWorldEffect::SpawnChildFromBreeding { .. } => "SpawnChildFromBreeding",
                BrainWorldEffect::BodyEntityEvent { .. } => "BodyEntityEvent",
                _ => "Villager",
            })
            .collect()
    }

    /// The sounds the behaviour queued, as `(resource id, volume, pitch)`.
    pub(crate) fn sounds(&self) -> Vec<(&'static str, f32, f32)> {
        self.effects
            .iter()
            .filter_map(|effect| match effect {
                BrainWorldEffect::PlayBodySound {
                    resource_id,
                    volume,
                    pitch,
                    ..
                } => Some((*resource_id, *volume, *pitch)),
                _ => None,
            })
            .collect()
    }

    pub(crate) fn poses(&self) -> Vec<i32> {
        self.effects
            .iter()
            .filter_map(|effect| match effect {
                BrainWorldEffect::SetPose { pose_id } => Some(*pose_id),
                _ => None,
            })
            .collect()
    }
}

/// One `BrainEntityFacts` with every field the sensors do not read left at its
/// neutral value.
pub(crate) fn entity_facts(entity_id: i32, entity_resource_id: &'static str) -> BrainEntityFacts {
    BrainEntityFacts {
        alive: true,
        living: true,
        entity_resource_id,
        uuid: Uuid::from_u128(entity_id as u128 + 1),
        position: Vec3d {
            x: f64::from(entity_id),
            y: 0.0,
            z: 0.0,
        },
        eye_y: 1.0,
        bounding_box: bbb_server_types::geometry::Aabb::of_entity(
            Vec3d {
                x: f64::from(entity_id),
                y: 0.0,
                z: 0.0,
            },
            0.6,
            1.8,
        ),
        slime_size: None,
        spectator: false,
    }
}

impl BrainWorld for TestWorld {
    fn game_time(&self) -> i64 {
        self.game_time
    }

    fn body(&self) -> &BrainBodyFacts {
        &self.body
    }

    fn body_has_passenger(&self, entity_id: i32) -> bool {
        self.passengers.contains(&entity_id)
    }

    fn level_random_next_int(&mut self, bound: i32) -> i32 {
        assert!(bound > 0);
        self.level_random_int.rem_euclid(bound)
    }

    fn entity_breed_facts(&self, entity_id: i32) -> Option<BrainBreedFacts> {
        self.breed_facts
            .iter()
            .find(|(staged, _)| *staged == entity_id)
            .map(|(_, facts)| *facts)
    }

    fn body_can_attack(&self, _entity_id: i32) -> bool {
        self.body_can_attack
    }

    fn damage_type_in_tag(&self, _damage_type: &str, tag_id: &str) -> bool {
        tag_id == PANIC_CAUSES_TAG && self.panic_causes
    }

    fn entity_type_in_tag(&self, _entity_resource_id: &str, tag_id: &str) -> bool {
        tag_id == FROG_FOOD_TAG && self.entity_type_tags
    }

    fn item_in_tag(&self, item_type_id: i32, tag_id: &str) -> bool {
        if tag_id != crate::entity::ai::mob::frog::FROG_FOOD_TAG {
            return false;
        }
        if self.tempting_item_id == Some(item_type_id) {
            self.item_tags
        } else {
            false
        }
    }

    fn entity_facts(&self, _entity_id: i32) -> Option<BrainEntityFacts> {
        self.entity_facts
    }

    fn nearest_living_entities(&self, _range: f64) -> Vec<i32> {
        self.nearest_living.clone()
    }

    fn nearest_players(&self, _range: f64) -> Vec<i32> {
        self.nearest_players.clone()
    }

    fn players_within(&self, _range: f64) -> Vec<BrainPlayer> {
        self.players_within.clone()
    }

    fn targeting_test(&self, test: EntityTargetingTest) -> bool {
        self.targeting_tests.borrow_mut().push(test);
        (if test.combat {
            self.combat_targeting
        } else {
            self.targeting
        }) && (test.ignore_line_of_sight || self.line_of_sight)
    }

    fn entity_distance_to_sqr(&self, _entity_id: i32) -> Option<f64> {
        self.distance_to_sqr
    }

    fn body_path_distance_to(&self, _entity_id: i32, _reach_range: i32) -> Option<f32> {
        self.path_distance
    }

    fn body_prepare_path(&mut self, pos: BlockPos, reach_range: i32) -> Option<MemoryPath> {
        self.prepare_path_calls.push((pos, reach_range));
        self.prepared_target = Some((pos, reach_range));
        self.prepared_path = self
            .prepare_path_results
            .pop_front()
            .unwrap_or(self.move_to_result);
        self.prepared_path
    }

    fn body_start_prepared_path(&mut self, speed_modifier: f32) {
        if let Some((pos, reach_range)) = self.prepared_target.take() {
            self.move_to_calls.push((pos, reach_range, speed_modifier));
            self.path = self.prepared_path.take();
        }
    }

    fn body_default_random_pos_towards(
        &mut self,
        random: &GameRandom,
        horizontal_dist: i32,
        vertical_dist: i32,
        towards: Vec3d,
        max_xz_radians: f64,
    ) -> Option<Option<Vec3d>> {
        self.towards_calls
            .push((horizontal_dist, vertical_dist, towards, max_xz_radians));
        let _ = random.next_float();
        self.towards_result
    }

    fn body_path(&self) -> Option<MemoryPath> {
        self.path
    }

    fn body_navigation_is_done(&self) -> bool {
        self.navigation_done
    }

    fn body_navigation_is_stuck(&self) -> bool {
        self.navigation_stuck
    }

    fn body_pathfinding_malus(&self, path_type: crate::entity::GroundPathType) -> f32 {
        self.pathfinding_malus_calls.borrow_mut().push(path_type);
        self.pathfinding_malus
    }

    fn body_can_fit_at(&self, area: bbb_server_types::geometry::Aabb) -> bool {
        self.can_fit_calls.borrow_mut().push(area);
        self.can_fit
    }

    fn body_path_reaches(
        &self,
        pos: BlockPos,
        reach_range: i32,
        max_path_length: f32,
    ) -> Option<bool> {
        // The recorders sit behind a `RefCell` because these seams are
        // `&self`, the same shape `targeting_tests` uses.
        self.path_reaches_calls
            .borrow_mut()
            .push((pos, reach_range, max_path_length));
        self.path_reaches
    }

    fn body_path_to_pois(
        &self,
        positions: &[BlockPos],
        reach_range: i32,
        max_path_length: f32,
    ) -> Option<(BlockPos, bool)> {
        let pos = *positions.first()?;
        Some((
            pos,
            self.body_path_reaches(pos, reach_range, max_path_length)?,
        ))
    }

    fn look_at(&mut self, position: Vec3d) {
        self.look_at = Some(position);
    }

    fn request_body_jump(&mut self) {
        self.jump_requests += 1;
    }

    fn body_land_random_pos(
        &mut self,
        _random: &GameRandom,
        horizontal_dist: i32,
        vertical_dist: i32,
    ) -> Option<Option<Vec3d>> {
        self.land_random_pos_calls
            .push((horizontal_dist, vertical_dist));
        self.land_random_pos
    }

    fn body_swim_target_pos(&mut self, _random: &GameRandom) -> Option<Option<Vec3d>> {
        self.swim_target_pos_calls += 1;
        self.swim_target_pos
    }

    fn body_closest_water(
        &mut self,
        horizontal_dist: i32,
        vertical_dist: i32,
        two_wide: bool,
    ) -> Option<Option<BlockPos>> {
        self.closest_water_calls
            .push((horizontal_dist, vertical_dist, two_wide));
        self.closest_water
    }

    fn block_facts(&self, pos: BlockPos) -> Option<crate::entity::ai::BrainBlockFacts> {
        self.block_facts_calls.borrow_mut().push(pos);
        self.block_facts
            .iter()
            .find(|(staged, _)| *staged == pos)
            .map(|(_, facts)| *facts)
    }

    fn block_path_type(&self, pos: BlockPos) -> Option<crate::entity::GroundPathType> {
        self.path_type_calls.borrow_mut().push(pos);
        self.path_types
            .iter()
            .find(|(staged, _)| *staged == pos)
            .map(|(_, path_type)| *path_type)
    }

    fn tongue_target(&self) -> Option<i32> {
        self.tongue_target
    }

    fn set_tongue_target(&mut self, entity_id: Option<i32>) {
        self.tongue_target = entity_id;
    }

    fn stop_body_navigation(&mut self) {
        self.navigation_stops += 1;
    }

    fn queue_effect(&mut self, effect: BrainWorldEffect) {
        match effect {
            BrainWorldEffect::HurtEntity { .. } if self.dies_on_hurt => {
                self.entity_facts = self.entity_facts.map(|facts| BrainEntityFacts {
                    alive: false,
                    ..facts
                });
            }
            _ => {}
        }
        self.effects.push(effect);
    }
}

pub(crate) fn test_world() -> TestWorld {
    TestWorld::default()
}

fn entry(name: &str, value: NbtSummaryValue) -> NbtSummaryEntry {
    NbtSummaryEntry {
        name: name.into(),
        value,
    }
}

fn compound(entries: Vec<NbtSummaryEntry>) -> NbtSummaryValue {
    NbtSummaryValue::Compound(entries)
}

fn expirable(name: &str, value: NbtSummaryValue, ttl: Option<i64>) -> NbtSummaryEntry {
    let mut fields = vec![entry("value", value)];
    if let Some(ttl) = ttl {
        fields.push(entry("ttl", NbtSummaryValue::Long(ttl)));
    }
    entry(name, compound(fields))
}

fn packed_brain(memories: Vec<NbtSummaryEntry>) -> NbtSummaryValue {
    compound(vec![entry("memories", compound(memories))])
}

#[test]
fn brain_activity_constants_match_the_26_1_catalog() {
    assert_eq!(
        Activity::CORE.resource_id(),
        VANILLA_ACTIVITIES[0].resource_id
    );
    assert_eq!(
        Activity::IDLE.resource_id(),
        VANILLA_ACTIVITIES[1].resource_id
    );
    assert_eq!(
        Activity::WORK.resource_id(),
        VANILLA_ACTIVITIES[2].resource_id
    );
    assert_eq!(
        Activity::DIG.resource_id(),
        VANILLA_ACTIVITIES[25].resource_id
    );
    assert_eq!(
        Activity::from_resource_id("minecraft:panic"),
        Some(Activity::PANIC)
    );
}

#[test]
fn brain_memory_module_catalog_is_complete_and_unique() {
    assert_eq!(MemoryModuleType::ALL.len(), 116);
    let mut resources = std::collections::HashSet::new();
    for memory_type in MemoryModuleType::ALL {
        assert!(resources.insert(memory_type.resource_id()), "{memory_type}");
        assert_eq!(
            MemoryModuleType::from_resource_id(memory_type.resource_id()),
            Some(*memory_type)
        );
    }
    assert!(MemoryModuleType::WalkTarget.codec_kind().is_none());
    assert!(MemoryModuleType::IsPanicking.can_serialize());
    assert!(MemoryModuleType::from_resource_id("minecraft:not_a_memory").is_none());
}

#[test]
fn brain_tick_expires_memories_after_ttl_reaches_zero() {
    let mut brain = Brain::new();
    brain.register_memory(MemoryModuleType::IsPanicking);
    brain.register_memory(MemoryModuleType::LastWoken);
    brain.set_memory_with_expiry(MemoryModuleType::IsPanicking, MemoryValue::Bool(true), 1);
    brain.set_memory(MemoryModuleType::LastWoken, Some(MemoryValue::Long(40)));
    let mut context = BrainTickContext::default();
    let mut world = test_world();

    brain.tick(&mut context, &mut world);
    assert_eq!(
        brain.memory(MemoryModuleType::IsPanicking),
        Some(&MemoryValue::Bool(true))
    );
    assert_eq!(brain.time_until_expiry(MemoryModuleType::IsPanicking), 0);
    assert_eq!(
        brain.memory(MemoryModuleType::LastWoken),
        Some(&MemoryValue::Long(40))
    );

    context.game_time = 1;
    brain.tick(&mut context, &mut world);
    assert!(brain.memory(MemoryModuleType::IsPanicking).is_none());
    assert!(!brain.has_memory_value(MemoryModuleType::IsPanicking));
    assert_eq!(
        brain.memory(MemoryModuleType::LastWoken),
        Some(&MemoryValue::Long(40))
    );
    assert!(brain
        .pack()
        .memories
        .iter()
        .all(|memory| memory.memory_type != MemoryModuleType::IsPanicking));
}

#[test]
fn brain_packed_codec_round_trips_serializable_expirable_memories() {
    let uuid = Uuid::from_u128(0x0123_4567_89ab_cdef_fedc_ba98_7654_3210);
    let source = packed_brain(vec![
        expirable("minecraft:is_panicking", NbtSummaryValue::Byte(1), Some(8)),
        expirable("minecraft:last_woken", NbtSummaryValue::Long(40), None),
        expirable(
            "minecraft:is_in_water",
            NbtSummaryValue::Compound(Vec::new()),
            Some(3),
        ),
        expirable(
            "minecraft:liked_player",
            NbtSummaryValue::IntArray(vec![
                (uuid.as_u128() >> 96) as u32 as i32,
                (uuid.as_u128() >> 64) as u32 as i32,
                (uuid.as_u128() >> 32) as u32 as i32,
                uuid.as_u128() as u32 as i32,
            ]),
            None,
        ),
        expirable(
            "minecraft:breeze_jump_target",
            NbtSummaryValue::IntArray(vec![1, 2, 3]),
            None,
        ),
        expirable(
            "minecraft:home",
            compound(vec![
                entry("dimension", NbtSummaryValue::string("minecraft:overworld")),
                entry("pos", NbtSummaryValue::IntArray(vec![4, 5, 6])),
            ]),
            None,
        ),
        expirable(
            "minecraft:sniffer_explored_positions",
            NbtSummaryValue::List(vec![compound(vec![
                entry("dimension", NbtSummaryValue::string("minecraft:the_nether")),
                entry("pos", NbtSummaryValue::IntArray(vec![7, 8, 9])),
            ])]),
            Some(12),
        ),
    ]);
    let packed = BrainPacked::decode(&source).unwrap();
    assert_eq!(packed.memories.len(), 7);

    let mut brain = Brain::new();
    for memory in MemoryModuleType::ALL {
        if memory.can_serialize() {
            brain.register_memory(*memory);
        }
    }
    brain.apply_packed(&packed);
    assert_eq!(
        brain.memory(MemoryModuleType::IsPanicking),
        Some(&MemoryValue::Bool(true))
    );
    assert_eq!(brain.time_until_expiry(MemoryModuleType::IsPanicking), 8);
    assert_eq!(
        brain.memory(MemoryModuleType::BreezeJumpTarget),
        Some(&MemoryValue::BlockPos(BlockPos { x: 1, y: 2, z: 3 }))
    );
    assert_eq!(
        brain.memory(MemoryModuleType::LikedPlayer),
        Some(&MemoryValue::Uuid(uuid))
    );

    let encoded = brain.pack().encode();
    let round_trip = BrainPacked::decode(&encoded).unwrap();
    assert_eq!(round_trip.memories.len(), packed.memories.len());
}

#[test]
fn brain_sensor_registration_adds_the_memories_it_requires() {
    let mut brain = Brain::new();
    brain.register_sensor(SensorType::HurtBy);
    // `Brain`'s constructor registers `newSensor.requires()`.
    assert!(brain.check_memory(MemoryModuleType::HurtBy, MemoryStatus::Registered));
    assert!(brain.check_memory(MemoryModuleType::HurtByEntity, MemoryStatus::Registered));
    assert!(!brain.check_memory(MemoryModuleType::IsInWater, MemoryStatus::Registered));
}
