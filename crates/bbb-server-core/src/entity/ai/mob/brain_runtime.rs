//! The production half of the Mob brain runtime: where a brain lives, what it
//! reads, and when it ticks.

use crate::world::simulation_level::LevelCtx;

use crate::random::GameRandom;
use bbb_protocol::entity_types::{
    VANILLA_ENTITY_TYPE_CREAKING_ID, VANILLA_ENTITY_TYPE_FROG_ID, VANILLA_ENTITY_TYPE_TADPOLE_ID,
    VANILLA_ENTITY_TYPE_VILLAGER_ID,
};
use bbb_protocol::packets::{BlockPos, Vec3d};
use bbb_server_datapack::damage::DamageTypeFacts;

use crate::entity::ai::{
    Brain, BrainBlockFacts, BrainDamageSource, BrainEntityFacts, BrainPacked, BrainPlayer,
    BrainTickContext, BrainWorld, BrainWorldEffect, EntityTargetingTest, MemoryModuleType,
    MemoryPath, MemoryValue,
};
use crate::entity::level::{NearbyPlayer, ScoreboardTeams};

use super::navigation::malus::PathfindingMalus;
use super::navigation::{GroundPath, MobNavigation, MobPathingSnapshot};
use super::random_pos::RandomPosMob;
use super::sensing::Sensing;
use super::target::{conditions, TargetView};
use super::{creaking, frog, tadpole, villager, MobAiRuntime};

/// The body facts sampled only when a live brain ticks.
#[derive(Debug, Clone)]
pub struct BrainBodyFacts {
    pub entity_id: i32,
    /// `Entity.getType()`, which picks the brain and the activity schedule.
    pub entity_type_id: i32,
    pub in_love: i32,
    /// `Entity.position()`.
    pub position: Vec3d,
    /// `Entity.blockPosition()`.
    pub block_position: BlockPos,
    /// `Entity.getPose().getId()`.
    pub pose_id: i32,
    pub eye_height: f32,
    pub bounding_box: bbb_server_types::geometry::Aabb,
    /// `None` reaches as a rider-less, unarmed body would.
    pub(crate) melee_reach: Option<crate::entity::ai::mob::MeleeReach>,
    pub can_move: bool,
    pub is_active: bool,
    pub in_water: bool,
    pub eye_in_water: bool,
    pub water_fluid_height: f64,
    pub on_fire: bool,
    pub in_lava: bool,
    /// `LivingEntity.getAttributeValue(Attributes.FOLLOW_RANGE)`.
    pub follow_range: f64,
    pub tempt_range: f64,
    /// `LivingEntity.getAttributeValue(Attributes.ATTACK_DAMAGE)`.
    pub attack_damage: f32,
    pub(crate) pathfinding_malus: PathfindingMalus,
    pub inherits_malus: bool,
    pub difficulty_peaceful: bool,
    pub last_damage: Option<BrainDamageSource>,
    pub on_ground: bool,
    pub y_body_rot: f32,
    /// `Entity.getDeltaMovement()`.
    pub delta_movement: Vec3d,
    pub gravity: f64,
    /// `LivingEntity.getAttributeValue(Attributes.JUMP_STRENGTH)`.
    pub jump_strength: f64,
    pub jump_boost_power: f64,
    pub team: Option<u32>,
    pub villager: Option<VillagerBrainFacts>,
}

/// What a villager's job-site behaviours read of the body.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct VillagerBrainFacts {
    /// `VillagerData.profession()`'s resource id.
    pub profession: &'static str,
    pub level: i32,
    pub xp: i32,
    pub assign_profession_when_spawned: bool,
    pub baby: bool,
    /// `AbstractVillager.getTradingPlayer()`'s entity id.
    pub trading_player: Option<i32>,
    /// The schedule's activity now (`Brain.updateActivityFromSchedule`).
    pub scheduled: Option<crate::entity::ai::Activity>,
    /// `LivingEntity.isSleeping()`.
    pub sleeping: bool,
    /// `Villager.canBreed`.
    pub can_breed: bool,
    /// `getInventory().countItem(BONE_MEAL) > 0`.
    pub has_bone_meal: bool,
    /// `Villager.hasFarmSeeds()`.
    pub has_farm_seeds: bool,
}

impl Default for BrainBodyFacts {
    fn default() -> Self {
        Self {
            entity_id: 0,
            entity_type_id: 0,
            in_love: 0,
            position: Vec3d::default(),
            block_position: BlockPos { x: 0, y: 0, z: 0 },
            pose_id: 0,
            eye_height: 0.0,
            bounding_box: bbb_server_types::geometry::Aabb {
                min_x: 0.0,
                min_y: 0.0,
                min_z: 0.0,
                max_x: 0.0,
                max_y: 0.0,
                max_z: 0.0,
            },
            melee_reach: None,
            can_move: true,
            is_active: false,
            in_water: false,
            eye_in_water: false,
            water_fluid_height: 0.0,
            on_fire: false,
            in_lava: false,
            follow_range: 0.0,
            tempt_range: 0.0,
            attack_damage: 0.0,
            pathfinding_malus: PathfindingMalus::default(),
            inherits_malus: false,
            difficulty_peaceful: false,
            last_damage: None,
            on_ground: false,
            y_body_rot: 0.0,
            delta_movement: Vec3d::default(),
            gravity: crate::entity::living::DEFAULT_BASE_GRAVITY,
            jump_strength: 0.0,
            jump_boost_power: 0.0,
            team: None,
            villager: None,
        }
    }
}

impl BrainBodyFacts {
    pub(crate) fn fluid_jump_threshold(&self) -> f64 {
        if self.eye_height < 0.4 {
            0.0
        } else {
            0.4
        }
    }

    pub(crate) fn is_in_love(&self) -> bool {
        self.in_love > 0
    }

    /// Reads the body with the current navigation malus.
    fn of(
        entity: &crate::entity::Entity,
        difficulty_peaceful: bool,
        pathfinding_malus: PathfindingMalus,
        inherits_malus: bool,
        teams: &ScoreboardTeams,
    ) -> Self {
        let crate::entity::EntityData::GeneralLiving(data) = entity.data() else {
            unreachable!("brain body is a general living entity");
        };
        let living = entity.living().expect("brain body has living state");
        Self {
            entity_id: entity.id(),
            entity_type_id: entity.entity_type_id(),
            in_love: data.animal_retained().map_or(
                0,
                crate::entity::general_living::AnimalRetainedState::in_love,
            ),
            position: entity.position(),
            block_position: entity.block_position(),
            pose_id: entity.pose_id(),
            eye_height: entity.eye_height(),
            bounding_box: entity.bounding_box(),
            melee_reach: None,
            can_move: entity.creaking_can_move(),
            is_active: entity.creaking_is_active(),
            in_water: entity.is_in_water(),
            eye_in_water: entity.is_eye_in_fluid(crate::entity::environment::FluidTag::Water),
            water_fluid_height: entity.fluid_height(crate::entity::environment::FluidTag::Water),
            on_fire: entity.is_on_fire(),
            in_lava: entity.is_in_lava(),
            follow_range: living
                .attribute_value(bbb_protocol::attribute_types::VANILLA_ATTRIBUTE_FOLLOW_RANGE_ID),
            tempt_range: living
                .attribute_value(bbb_protocol::attribute_types::VANILLA_ATTRIBUTE_TEMPT_RANGE_ID),
            attack_damage: living
                .attribute_value(bbb_protocol::attribute_types::VANILLA_ATTRIBUTE_ATTACK_DAMAGE_ID)
                as f32,
            pathfinding_malus,
            inherits_malus,
            difficulty_peaceful,
            last_damage: living
                .last_damage_type
                .clone()
                .map(|damage_type| BrainDamageSource {
                    damage_type,
                    source_entity: living.last_damage_source_entity,
                }),
            on_ground: entity.on_ground(),
            y_body_rot: living.y_body_rot,
            delta_movement: entity.delta_movement(),
            gravity: crate::entity::living::gravity(entity),
            jump_strength: living
                .attribute_value(bbb_protocol::attribute_types::VANILLA_ATTRIBUTE_JUMP_STRENGTH_ID),
            jump_boost_power: living
                .active_effect(JUMP_BOOST_EFFECT)
                .map_or(0.0, |effect| 0.1 * f64::from(effect.amplifier() + 1)),
            team: teams.entity_team(entity.uuid()),
            villager: super::villager::brain_facts(entity),
        }
    }
}

/// `MobEffects.JUMP_BOOST`'s resource id, which `getJumpBoostPower` tests the
/// body's effect list for (`LivingEntity.java:2338-2340`).
const JUMP_BOOST_EFFECT: &str = "minecraft:jump_boost";

/// The body, the level, the navigation and the tick's effect buffer one brain
/// tick reaches, which is the `ServerLevel level, E body` pair Vanilla hands
/// every sensor and behaviour split into the pieces a stored brain cannot hold
/// between ticks.
struct MobBrainWorld<'a, 'l, 'w> {
    body: &'a BrainBodyFacts,
    body_entity: &'a crate::entity::Entity,
    passengers: &'a [i32],
    sensing: &'a Sensing,
    mob: MobPathingSnapshot,
    effective_malus: std::cell::Cell<PathfindingMalus>,
    own_malus: std::cell::Cell<PathfindingMalus>,
    entities: &'a crate::entity::EntityStore,
    level: &'a mut LevelCtx<'l, 'w>,
    navigation: &'a mut MobNavigation,
    prepared_path: Option<GroundPath>,
    tongue_target: &'a mut Option<i32>,
    look_request: &'a mut Option<Vec3d>,
    jump_request: &'a mut bool,
    effects: &'a mut Vec<BrainWorldEffect>,
}

impl<'l, 'w> MobBrainWorld<'_, 'l, 'w> {
    fn targets(&self) -> TargetView<'_, 'l, 'w> {
        TargetView::new(self.body_entity, self.entities, self.level, self.sensing)
    }

    fn consume_frog_prepare(&self) {
        if let Some(prepared) = self.navigation.take_amphibious_prepare() {
            let mut own = self.own_malus.get();
            prepared.apply(&mut own);
            self.own_malus.set(own);
            if !self.body.inherits_malus {
                self.effective_malus.set(own);
            }
        }
    }
    /// The facts of one entity of the level, or nothing when the level holds
    /// none — the answer `Sensor`/`TargetingConditions` read as "no such
    /// target" (`BrainEntityFacts`).
    fn facts(&self, entity_id: i32) -> Option<BrainEntityFacts> {
        if let Some(entity) = self.entities.get(entity_id) {
            return Some(entity_facts(entity));
        }
        self.level
            .players()
            .iter()
            .find(|player| player.entity_id == entity_id)
            .map(player_facts)
    }

    /// Every session player of this level inside the body's box inflated by
    /// `range` on all three axes, as `(distanceToSqr, player)`, in the level's
    /// own order.
    fn players_in_box_around(&self, range: f64) -> impl Iterator<Item = (f64, &NearbyPlayer)> + '_ {
        let area = self.body.bounding_box.inflate(range, range, range);
        let body_position = self.body.position;
        self.level
            .players()
            .iter()
            .filter(move |player| player.bounding_box().intersects(area))
            .map(move |player| {
                (
                    position_distance_to_sqr(body_position, player.position),
                    player,
                )
            })
    }
}

impl<'l, 'w> BrainWorld for MobBrainWorld<'_, 'l, 'w> {
    fn game_time(&self) -> i64 {
        self.level.game_time()
    }

    fn body(&self) -> &BrainBodyFacts {
        self.body
    }

    fn body_has_passenger(&self, entity_id: i32) -> bool {
        self.passengers.contains(&entity_id)
    }

    fn entity_breed_facts(&self, entity_id: i32) -> Option<crate::entity::ai::BrainBreedFacts> {
        let entity = self.entities.get(entity_id)?;
        let crate::entity::EntityData::GeneralLiving(data) = entity.data() else {
            return None;
        };
        let animal = data.animal_retained()?;
        Some(crate::entity::ai::BrainBreedFacts {
            same_class: entity.entity_type_id() == self.body.entity_type_id,
            in_love: animal.is_in_love(),
            panicking: self
                .entities
                .mob_ai(entity_id)
                .is_some_and(|runtime| runtime.is_panicking()),
        })
    }

    fn body_can_attack(&self, entity_id: i32) -> bool {
        let targets = self.targets();
        let Some(target) = targets.facts(entity_id) else {
            return false;
        };
        conditions::can_attack(self.body.difficulty_peaceful, &targets.body(), &target)
    }

    fn damage_type_in_tag(&self, damage_type: &str, tag_id: &str) -> bool {
        self.level
            .registry_tag_membership("minecraft:damage_type", tag_id, damage_type)
    }

    fn entity_type_in_tag(&self, entity_resource_id: &str, tag_id: &str) -> bool {
        self.level
            .registry_tag_membership("minecraft:entity_type", tag_id, entity_resource_id)
    }

    fn item_in_tag(&self, item_type_id: i32, tag_id: &str) -> bool {
        let Some(item) = bbb_protocol::item_types::vanilla_item_type(item_type_id) else {
            return false;
        };
        self.level
            .registry_tag_membership("minecraft:item", tag_id, item.resource_id)
    }

    fn entity_facts(&self, entity_id: i32) -> Option<BrainEntityFacts> {
        self.facts(entity_id)
    }

    fn nearest_living_entities(&self, range: f64) -> Vec<i32> {
        let body = self.body;
        let area = body.bounding_box.inflate(range, range, range);
        let mut nearby: Vec<(f64, i32)> = self
            .entities
            .box_section_ids(area)
            .filter_map(|entity_id| {
                let entity = self.entities.get(entity_id)?;
                if !entity.living().is_some_and(|_| entity.is_alive()) {
                    return None;
                }
                let position = entity.position();
                let dx = (body.position.x - position.x).abs();
                let dy = (body.position.y - position.y).abs();
                let dz = (body.position.z - position.z).abs();
                if dx > range || dy > range || dz > range {
                    return None;
                }
                let distance = dx * dx + dy * dy + dz * dz;
                Some((distance, entity_id))
            })
            .chain(
                self.players_in_box_around(range)
                    .map(|(distance, player)| (distance, player.entity_id)),
            )
            .collect();
        nearby.sort_by(|left, right| left.0.total_cmp(&right.0));
        nearby.into_iter().map(|(_, entity_id)| entity_id).collect()
    }

    fn nearest_players(&self, range: f64) -> Vec<i32> {
        let mut players: Vec<(f64, i32)> = self
            .players_in_box_around(range)
            .filter(|(distance, _)| *distance < range * range)
            .map(|(distance, player)| (distance, player.entity_id))
            .collect();
        players.sort_by(|left, right| left.0.total_cmp(&right.0));
        players
            .into_iter()
            .map(|(_, entity_id)| entity_id)
            .collect()
    }

    fn players_within(&self, range: f64) -> Vec<BrainPlayer> {
        let mut players: Vec<(f64, BrainPlayer)> = self
            .players_in_box_around(range)
            .map(|(distance, player)| {
                (
                    distance,
                    BrainPlayer {
                        entity_id: player.entity_id,
                        main_hand_item_id: player.main_hand_item_id,
                        off_hand_item_id: player.off_hand_item_id,
                    },
                )
            })
            .collect();
        players.sort_by(|left, right| left.0.total_cmp(&right.0));
        players.into_iter().map(|(_, player)| player).collect()
    }

    fn targeting_test(&self, test: EntityTargetingTest) -> bool {
        let targets = self.targets();
        let Some(target) = targets.facts(test.entity_id) else {
            return false;
        };
        let body = targets.body();
        if target.entity_id() == body.entity_id() || !conditions::can_be_seen_by_anyone(&target) {
            return false;
        }
        if test.combat
            && (!conditions::can_attack(self.body.difficulty_peaceful, &body, &target)
                || ScoreboardTeams::allied(self.body.team, target.team()))
        {
            return false;
        }
        conditions::in_targeting_range(&body, &target, test.range, test.ignore_invisibility)
            && (test.ignore_line_of_sight || target.line_of_sight())
    }

    fn entity_distance_to_sqr(&self, entity_id: i32) -> Option<f64> {
        let facts = self.facts(entity_id)?;
        let dx = self.body.position.x - facts.position.x;
        let dy = self.body.position.y - facts.position.y;
        let dz = self.body.position.z - facts.position.z;
        Some(dx * dx + dy * dy + dz * dz)
    }

    fn body_path_distance_to(&self, entity_id: i32, reach_range: i32) -> Option<f32> {
        let facts = self.facts(entity_id)?;
        let path = self.navigation.create_path_to(
            self.level,
            self.pathing_snapshot(),
            self.level.level_random(),
            entity_block_position(&facts),
            reach_range,
            None,
        );
        self.consume_frog_prepare();
        let path = path?;
        Some(path.distance_to_target())
    }

    fn body_prepare_path(&mut self, pos: BlockPos, reach_range: i32) -> Option<MemoryPath> {
        let mob = self.pathing_snapshot();
        self.prepared_path = self.navigation.create_path(
            self.level,
            mob,
            self.level.level_random(),
            pos,
            reach_range,
        );
        self.consume_frog_prepare();
        self.prepared_path.as_ref().map(memory_path)
    }

    fn body_start_prepared_path(&mut self, speed_modifier: f32) {
        if let Some(path) = self.prepared_path.take() {
            let mob = self.pathing_snapshot();
            let _ = self
                .navigation
                .start_path(self.level, path, mob, f64::from(speed_modifier));
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
        let body = RandomPosMob::of(self.body_entity, self.effective_malus.get());
        crate::entity::ai::mob::random_pos::default_random_pos_towards(
            self.level,
            random,
            body,
            horizontal_dist,
            vertical_dist,
            towards,
            max_xz_radians,
        )
    }

    fn body_path(&self) -> Option<MemoryPath> {
        let path = self.navigation.path()?;
        Some(memory_path(path))
    }

    fn body_navigation_is_done(&self) -> bool {
        self.navigation
            .path()
            .is_none_or(crate::entity::ai::mob::navigation::GroundPath::is_done)
    }

    fn body_navigation_is_stuck(&self) -> bool {
        self.navigation.is_stuck()
    }

    fn body_pathfinding_malus(&self, path_type: crate::entity::GroundPathType) -> f32 {
        self.effective_malus.get().get(path_type)
    }

    fn body_can_fit_at(&self, area: bbb_server_types::geometry::Aabb) -> bool {
        !bbb_server_types::geometry::collect_block_collisions_with_moving_pistons(
            area,
            |pos| self.level.is_moving_piston_at(pos).unwrap_or(true),
            |pos| self.level.block_collision_boxes(pos),
        )
        .into_iter()
        .any(|(_, shape)| shape.intersects(area))
    }

    fn body_path_reaches(
        &self,
        pos: BlockPos,
        reach_range: i32,
        max_path_length: f32,
    ) -> Option<bool> {
        let path = self.navigation.create_path_to(
            self.level,
            self.pathing_snapshot(),
            self.level.level_random(),
            pos,
            reach_range,
            Some(max_path_length),
        );
        self.consume_frog_prepare();
        let path = path?;
        Some(path.reached())
    }

    fn body_path_to_pois(
        &self,
        positions: &[BlockPos],
        reach_range: i32,
        max_path_length: f32,
    ) -> Option<(BlockPos, bool)> {
        let MobNavigation::Ground(navigation) = &*self.navigation else {
            return None;
        };
        let path = navigation.create_path_to_targets(
            self.level,
            self.pathing_snapshot(),
            positions,
            reach_range,
            Some(max_path_length),
        )?;
        Some((path.target(), path.reached()))
    }

    fn look_at(&mut self, position: Vec3d) {
        *self.look_request = Some(position);
    }

    fn request_body_jump(&mut self) {
        *self.jump_request = true;
    }

    fn body_land_random_pos(
        &mut self,
        random: &GameRandom,
        horizontal_dist: i32,
        vertical_dist: i32,
    ) -> Option<Option<Vec3d>> {
        let mob = RandomPosMob::of(self.body_entity, self.effective_malus.get());
        crate::entity::ai::mob::random_pos::land_random_pos(
            self.level,
            random,
            mob,
            horizontal_dist,
            vertical_dist,
        )
    }

    fn body_has_no_blocks_above(&mut self) -> bool {
        super::villager::raid::has_no_blocks_above(
            self.level,
            self.body.block_position,
            self.body.position.y,
        )
    }

    fn body_outdoor_position(&mut self, random: &GameRandom) -> Option<Vec3d> {
        let at = self.body.block_position;
        if self.level.can_see_sky(at) {
            return None;
        }
        // `MoveToSkySeeingSpot.getOutdoorPosition`.
        (0..10).find_map(|_| {
            let pos = BlockPos {
                x: at.x + random.next_int(20) - 10,
                y: at.y + random.next_int(6) - 3,
                z: at.z + random.next_int(20) - 10,
            };
            super::villager::raid::has_no_blocks_above(self.level, pos, self.body.position.y)
                .then_some(Vec3d {
                    x: f64::from(pos.x) + 0.5,
                    y: f64::from(pos.y),
                    z: f64::from(pos.z) + 0.5,
                })
        })
    }

    fn body_swim_target_pos(&mut self, random: &GameRandom) -> Option<Option<Vec3d>> {
        let mob = RandomPosMob::of(self.body_entity, self.effective_malus.get());
        crate::entity::ai::mob::random_pos::swim_target_pos(self.level, random, mob)
    }

    fn level_random_next_int(&mut self, bound: i32) -> i32 {
        self.level.level_random().next_int(bound)
    }

    fn body_closest_water(
        &mut self,
        horizontal_dist: i32,
        vertical_dist: i32,
        two_wide: bool,
    ) -> Option<Option<BlockPos>> {
        super::panic::look_for_water(
            self.level,
            self.body.block_position,
            horizontal_dist,
            vertical_dist,
            two_wide,
        )
    }

    fn block_facts(&self, pos: BlockPos) -> Option<BrainBlockFacts> {
        block_facts_at(self.level, pos)
    }

    fn block_path_type(&self, pos: BlockPos) -> Option<crate::entity::GroundPathType> {
        crate::entity::ai::mob::navigation::walk_path_type(self.level, pos)
    }

    fn tongue_target(&self) -> Option<i32> {
        self.tongue_target
            .filter(|entity_id| self.entities.get(*entity_id).is_some())
    }

    fn set_tongue_target(&mut self, entity_id: Option<i32>) {
        *self.tongue_target = entity_id;
    }

    fn stop_body_navigation(&mut self) {
        self.navigation.stop();
    }

    fn queue_effect(&mut self, effect: BrainWorldEffect) {
        self.effects.push(effect);
    }

    fn dimension(&self) -> &str {
        self.level.dimension_key().as_str()
    }

    fn job_site_candidates(
        &mut self,
        center: BlockPos,
        radius: i32,
        poi_type: Option<&str>,
        limit: usize,
        allowed: &dyn Fn(BlockPos) -> bool,
    ) -> Vec<BlockPos> {
        let center = [center.x, center.y, center.z];
        let mut records = self.level.poi_mut().records_in_range(
            center,
            radius,
            crate::chunk::poi::PoiOccupancy::HasSpace,
            |record| {
                poi_type.is_none_or(|poi_type| record.type_id == poi_type)
                    && allowed(BlockPos {
                        x: record.pos[0],
                        y: record.pos[1],
                        z: record.pos[2],
                    })
            },
        );
        if poi_type.is_none() {
            records.retain(|record| self.poi_is_acquirable_job_site(record.type_id));
        }
        records.sort_by_key(|record| {
            record
                .pos
                .iter()
                .zip(center)
                .map(|(a, b)| (i64::from(*a) - i64::from(b)).pow(2))
                .sum::<i64>()
        });
        records
            .into_iter()
            .take(limit)
            .map(|record| BlockPos {
                x: record.pos[0],
                y: record.pos[1],
                z: record.pos[2],
            })
            .collect()
    }

    fn poi_type_at(&mut self, pos: BlockPos) -> Option<&'static str> {
        self.level
            .poi_mut()
            .record([pos.x, pos.y, pos.z])
            .map(|record| record.type_id)
    }

    fn poi_is_acquirable_job_site(&self, type_id: &str) -> bool {
        self.level.registry_tag_membership(
            "minecraft:point_of_interest_type",
            "minecraft:acquirable_job_site",
            type_id,
        )
    }

    fn poi_take(&mut self, pos: BlockPos) -> bool {
        self.level.poi_mut().acquire_ticket([pos.x, pos.y, pos.z])
    }

    fn poi_release(&mut self, pos: BlockPos) {
        self.level.poi_mut().release_ticket([pos.x, pos.y, pos.z]);
    }

    fn bed_occupied(&self, pos: BlockPos) -> Option<bool> {
        let state = self.level.block_state_checked(pos)?;
        if !crate::block_behavior::bed::is_bed(state) {
            return None;
        }
        bbb_server_types::block_queries::bool_property(state, "occupied")
    }

    fn mob_griefing(&self) -> bool {
        self.level.mob_griefing()
    }

    fn villager_can_breed(&self, entity_id: i32) -> bool {
        self.entities
            .get(entity_id)
            .is_some_and(crate::entity::villager_life::can_breed)
    }

    fn farm_cell(&self, pos: BlockPos) -> crate::entity::ai::FarmCell {
        use crate::entity::ai::FarmCell;
        let Some(state) = self.level.block_state_checked(pos) else {
            return FarmCell::Other;
        };
        if crate::block_behavior::crop::is_crop(state) {
            return if crate::block_behavior::crop::is_max_age(state) {
                FarmCell::RipeCrop
            } else {
                FarmCell::GrowingCrop
            };
        }
        let below = self.level.block_state_checked(BlockPos {
            y: pos.y - 1,
            ..pos
        });
        if bbb_server_types::fluid::block_state_is_air(state)
            && below.and_then(bbb_protocol::block_state_types::vanilla_block_state_block_type_id)
                == Some(bbb_protocol::block_types::VANILLA_BLOCK_FARMLAND_ID)
        {
            FarmCell::EmptyFarmland
        } else {
            FarmCell::Other
        }
    }

    fn farmland_around(&self, center: BlockPos) -> Vec<BlockPos> {
        let mut farmland = Vec::new();
        for x in -4..=4 {
            for y in -2..=2 {
                for z in -4..=4 {
                    let pos = BlockPos {
                        x: center.x + x,
                        y: center.y + y,
                        z: center.z + z,
                    };
                    if self.level.block_state_checked(pos).and_then(
                        bbb_protocol::block_state_types::vanilla_block_state_block_type_id,
                    ) == Some(bbb_protocol::block_types::VANILLA_BLOCK_FARMLAND_ID)
                    {
                        farmland.push(pos);
                    }
                }
            }
        }
        farmland
    }

    fn body_land_random_pos_away(
        &mut self,
        random: &GameRandom,
        horizontal_dist: i32,
        vertical_dist: i32,
        avoid: Vec3d,
    ) -> Option<Option<Vec3d>> {
        let mob = RandomPosMob::of(self.body_entity, self.effective_malus.get());
        crate::entity::ai::mob::random_pos::land_random_pos_away(
            self.level,
            random,
            mob,
            0.0,
            f64::from(horizontal_dist),
            vertical_dist,
            avoid,
        )
    }

    fn villager_job_facts(&self, entity_id: i32) -> Option<crate::entity::ai::VillagerJobFacts> {
        let entity = self
            .entities
            .get(entity_id)
            .filter(|entity| entity_is_alive(entity))?;
        let villager = super::villager::brain_facts(entity)?;
        let runtime = self.entities.mob_ai(entity_id);
        let memory = |memory_type| runtime.and_then(|runtime| runtime.brain_memory(memory_type));
        Some(crate::entity::ai::VillagerJobFacts {
            profession: villager.profession,
            xp: villager.xp,
            job_site: match memory(MemoryModuleType::JobSite) {
                Some(MemoryValue::GlobalPos(pos)) => Some(pos.clone()),
                _ => None,
            },
            has_potential_job_site: memory(MemoryModuleType::PotentialJobSite).is_some(),
        })
    }

    fn villager_can_reach_job_site(&self, entity_id: i32, pos: BlockPos, reach: i32) -> bool {
        let Some(entity) = self.entities.get(entity_id) else {
            return false;
        };
        let Some(runtime) = self.entities.mob_ai(entity_id) else {
            return false;
        };
        let Some((malus, inherited)) = self
            .entities
            .effective_pathfinding_malus_with_provenance(entity, self.level)
        else {
            return false;
        };
        let Some(pathing) = MobPathingSnapshot::from_entity(
            entity,
            runtime.has_target(),
            self.entities.vehicle(entity_id).is_some(),
            self.entities.difficulty(),
            malus,
        ) else {
            return false;
        };
        let maximum = (entity
            .living()
            .map(|living| {
                living.attribute_value(
                    bbb_protocol::attribute_types::VANILLA_ATTRIBUTE_FOLLOW_RANGE_ID,
                )
            })
            .unwrap_or_default() as f32)
            .max(super::VILLAGER_REQUIRED_PATH_LENGTH);
        runtime.ground_path_reaches(
            self.level,
            pathing.with_inherited_malus(inherited),
            pos,
            reach,
            maximum,
        ) == Some(true)
    }

    fn villager_play_facts(&self, entity_id: i32) -> Option<crate::entity::ai::VillagerPlayFacts> {
        let entity = self
            .entities
            .get(entity_id)
            .filter(|entity| entity_is_alive(entity))?;
        if !crate::entity::villager::is_villager(entity) {
            return None;
        }
        let crate::entity::EntityData::GeneralLiving(data) = entity.data() else {
            return None;
        };
        let baby = data.ageable_retained().is_some_and(|age| age.is_baby());
        let interaction_target = self
            .entities
            .mob_ai(entity_id)
            .and_then(|runtime| runtime.brain_memory(MemoryModuleType::InteractionTarget))
            .and_then(|memory| match memory {
                MemoryValue::EntityRef(id) => Some(*id),
                _ => None,
            });
        Some(crate::entity::ai::VillagerPlayFacts {
            baby,
            interaction_target,
        })
    }

    // `ServerLevel.isCloseToVillage`.
    fn is_village(&self, pos: BlockPos) -> bool {
        self.level
            .chunks()
            .sections_to_village([pos.x >> 4, pos.y >> 4, pos.z >> 4])
            .is_some_and(|distance| distance <= 1)
    }

    fn closest_village_section(&self, pos: BlockPos) -> Option<BlockPos> {
        self.level.chunks().closest_village_section(pos)
    }

    fn can_see_sky(&self, pos: BlockPos) -> bool {
        self.level.can_see_sky(pos)
    }

    fn home_pois_within(&mut self, center: BlockPos, radius: i32) -> Vec<BlockPos> {
        let center = [center.x, center.y, center.z];
        let mut records = self.level.poi_mut().records_in_range(
            center,
            radius,
            crate::chunk::poi::PoiOccupancy::Any,
            |record| record.type_id == crate::entity::villager::HOME_POI,
        );
        records.sort_by_key(|record| {
            record
                .pos
                .iter()
                .zip(center)
                .map(|(a, b)| (i64::from(*a) - i64::from(b)).pow(2))
                .sum::<i64>()
        });
        records
            .into_iter()
            .map(|record| BlockPos {
                x: record.pos[0],
                y: record.pos[1],
                z: record.pos[2],
            })
            .collect()
    }

    fn villager_sleeps_in(&self, pos: BlockPos) -> bool {
        let area = bbb_server_types::geometry::Aabb {
            min_x: f64::from(pos.x),
            min_y: f64::from(pos.y),
            min_z: f64::from(pos.z),
            max_x: f64::from(pos.x) + 1.0,
            max_y: f64::from(pos.y) + 1.0,
            max_z: f64::from(pos.z) + 1.0,
        };
        self.entities
            .ids_in_box_sections(area)
            .into_iter()
            .filter_map(|id| self.entities.get(id))
            .any(|entity| {
                crate::entity::villager::is_villager(entity)
                    && entity.sleeping_pos().is_some()
                    && entity.bounding_box().intersects(area)
            })
    }
}

/// The state-local predicates come from the generated 26.1 support facts,
/// whose probe answers them against `EmptyBlockGetter`. A state whose
/// collision shape is dynamic has no such answer, so this refuses it rather
/// than reporting a stale one.
fn block_facts_at(level: &LevelCtx<'_, '_>, pos: BlockPos) -> Option<BrainBlockFacts> {
    if !level.has_ticking_column_at(pos) {
        return None;
    }
    let state = level.block_state(pos);
    let Some(support) = bbb_protocol::block_state_types::vanilla_block_state_support_facts(state)
    else {
        let (block, _) = level.content_registry().block_by_state_id(state)?;
        let solid = block.full_cube_collision;
        let block_tagged =
            |tag| crate::entity::level::block_state_tag_membership(level, state, tag);
        return Some(BrainBlockFacts {
            is_air: false,
            is_solid_render: solid,
            has_fluid: false,
            fluid_is_water: false,
            fluid_supports_frogspawn: false,
            supports_frogspawn: block_tagged("minecraft:supports_frogspawn"),
            prefers_frog_jump: block_tagged("minecraft:frog_prefer_jump_to"),
            is_water_block: false,
            collision_shape_empty: !solid,
            collision_face_up_empty: !solid,
            face_sturdy_up: solid,
        });
    };
    if support.dynamic_shape() {
        return None;
    }
    let resource_id = bbb_protocol::block_state_types::vanilla_block_state_block_type_id(state)
        .and_then(bbb_protocol::block_types::vanilla_block_type)
        .map(|entry| entry.resource_id)?;
    let fluid = bbb_server_types::fluid::block_state_fluid_state(state);
    let block_tagged = |tag| {
        Some(crate::entity::level::block_state_tag_membership(
            level, state, tag,
        ))
    };
    let fluid_tagged = |tag| {
        Some(crate::entity::level::fluid_state_tag_membership(
            level, fluid, tag,
        ))
    };
    Some(BrainBlockFacts {
        is_air: bbb_server_types::fluid::block_state_is_air(state),
        is_solid_render: support.solid_render(),
        has_fluid: !fluid.is_empty(),
        fluid_is_water: fluid_tagged("minecraft:water")?,
        fluid_supports_frogspawn: fluid_tagged("minecraft:supports_frogspawn")?,
        supports_frogspawn: block_tagged("minecraft:supports_frogspawn")?,
        prefers_frog_jump: block_tagged("minecraft:frog_prefer_jump_to")?,
        is_water_block: resource_id == "minecraft:water",
        collision_shape_empty: bbb_server_types::geometry::static_block_collision_box_iter(state)
            .map_or_else(
                || level.block_collision_boxes(pos).is_empty(),
                |boxes| boxes.len() == 0,
            ),
        collision_face_up_empty: !support.collision_face_nonempty(
            bbb_protocol::block_state_types::VanillaBlockSupportDirection::Up,
        ),
        face_sturdy_up: support.face_sturdy(
            bbb_protocol::block_state_types::VanillaBlockSupportDirection::Up,
            bbb_protocol::block_state_types::VanillaBlockSupportType::Full,
        ),
    })
}

impl<'l, 'w> MobBrainWorld<'_, 'l, 'w> {
    fn pathing_snapshot(&self) -> MobPathingSnapshot {
        self.mob.with_malus(self.effective_malus.get())
    }
}

fn entity_block_position(facts: &BrainEntityFacts) -> BlockPos {
    entity_block_position_of(facts.position)
}

fn entity_block_position_of(position: Vec3d) -> BlockPos {
    BlockPos {
        x: position.x.floor() as i32,
        y: position.y.floor() as i32,
        z: position.z.floor() as i32,
    }
}

fn memory_path(path: &GroundPath) -> MemoryPath {
    MemoryPath {
        target: path.target(),
        length: path.nodes().len(),
        reached: path.reached(),
    }
}

/// The facts of one stored entity, which `Sensor` and `TargetingConditions`
/// read through [`BrainEntityFacts`].
fn entity_facts(entity: &crate::entity::Entity) -> BrainEntityFacts {
    let position = entity.position();
    BrainEntityFacts {
        alive: entity_is_alive(entity),
        living: entity.living().is_some(),
        entity_resource_id: entity.data().entity_resource_id(),
        uuid: entity.uuid(),
        position,
        eye_y: position.y + f64::from(entity.eye_height()),
        bounding_box: entity.bounding_box(),
        slime_size: match entity.data() {
            crate::entity::EntityData::GeneralLiving(data) => data
                .slime_retained()
                .map(crate::entity::slime::SlimeRetainedState::size),
            _ => None,
        },
        spectator: false,
    }
}

/// The facts of one session player, which `Sensor` and `TargetingConditions`
/// read through [`BrainEntityFacts`] the same way they read a stored entity's.
fn player_facts(player: &NearbyPlayer) -> BrainEntityFacts {
    BrainEntityFacts {
        alive: !player.dead_or_dying,
        living: true,
        entity_resource_id: player_resource_id(),
        uuid: player.uuid,
        position: player.position,
        eye_y: player.position.y + player.eye_height,
        bounding_box: player.bounding_box(),
        slime_size: None,
        spectator: player.spectator,
    }
}

/// `EntityType.PLAYER`'s registry path, which is what `Entity.getType()` names
/// for a session player.
fn player_resource_id() -> &'static str {
    bbb_protocol::entity_types::vanilla_entity_type(
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_PLAYER_ID,
    )
    .map_or("minecraft:player", |entry| entry.resource_id)
}

/// `Vec3.distanceToSqr(Vec3)` between two positions.
fn position_distance_to_sqr(from: Vec3d, to: Vec3d) -> f64 {
    let dx = from.x - to.x;
    let dy = from.y - to.y;
    let dz = from.z - to.z;
    dx * dx + dy * dy + dz * dz
}

fn update_activity_for(
    entity_type_id: i32,
    brain: &mut Brain,
    game_time: i64,
    body: &BrainBodyFacts,
    raid_activity: Option<crate::entity::ai::Activity>,
) {
    match entity_type_id {
        VANILLA_ENTITY_TYPE_FROG_ID => frog::update_activity(brain),
        VANILLA_ENTITY_TYPE_TADPOLE_ID => {
            brain.set_active_activity_to_first_valid(&[crate::entity::ai::Activity::IDLE])
        }
        VANILLA_ENTITY_TYPE_CREAKING_ID => creaking::update_activity(brain, body.can_move),
        VANILLA_ENTITY_TYPE_VILLAGER_ID => {
            villager::update_activity(brain, game_time, body, raid_activity)
        }
        _ => {}
    }
}

/// Whether this runtime builds the entity's brain provider.
pub(crate) const fn has_brain_provider(entity_type_id: i32) -> bool {
    matches!(
        entity_type_id,
        VANILLA_ENTITY_TYPE_FROG_ID
            | VANILLA_ENTITY_TYPE_TADPOLE_ID
            | VANILLA_ENTITY_TYPE_CREAKING_ID
            | VANILLA_ENTITY_TYPE_VILLAGER_ID
    )
}

fn make_brain(entity_type_id: i32, packed: &BrainPacked) -> Option<Brain> {
    match entity_type_id {
        VANILLA_ENTITY_TYPE_FROG_ID => Some(frog::frog_brain(packed)),
        VANILLA_ENTITY_TYPE_TADPOLE_ID => Some(tadpole::tadpole_brain(packed)),
        VANILLA_ENTITY_TYPE_CREAKING_ID => Some(creaking::creaking_brain(packed)),
        VANILLA_ENTITY_TYPE_VILLAGER_ID => Some(villager::villager_brain(packed)),
        _ => None,
    }
}

impl MobAiRuntime {
    /// The mob's brain is built on the first tick rather than at construction
    /// so the spawn-finalize memories this entity already carries are in the
    /// pack it starts from, and it is rebuilt from the pack the entity carries
    /// whenever [`MobAiRuntime::invalidate_brain`] drops the live one.
    pub(crate) fn ensure_brain(&mut self, entity: &crate::entity::Entity) -> bool {
        if self.brain.is_some() && !self.brain_stale {
            return true;
        }
        let entity_type_id = entity.entity_type_id();
        if !has_brain_provider(entity_type_id) {
            return false;
        }
        let Some(packed) = crate::entity::brain_packed(entity) else {
            return false;
        };
        self.attach_brain(entity_type_id, packed)
    }

    /// Marks the live brain stale, so the next tick rebuilds it from the pack
    /// the entity carries now (`/data merge` replaces that pack wholesale,
    /// `EntityDataAccessor.setData`). The brain this drops stays readable until
    /// then, and the rebuild draws the sensor delays again from the mob's own
    /// random, exactly as the first build drew them.
    /// Whether the villager's last raid sample saw an ongoing raid, the
    /// `raid.isActive() && !raid.isOver()` of `Villager`.
    pub(crate) fn villager_in_ongoing_raid(&self) -> bool {
        self.villager_raid_activity.is_some_and(|activity| {
            activity == crate::entity::ai::Activity::RAID
                || activity == crate::entity::ai::Activity::PRE_RAID
        })
    }

    pub(crate) fn invalidate_brain(&mut self) {
        if self.brain.is_some() {
            self.brain_stale = true;
        }
    }

    fn attach_brain(&mut self, entity_type_id: i32, packed: &BrainPacked) -> bool {
        let Some(brain) = make_brain(entity_type_id, packed) else {
            return false;
        };
        self.brain = Some(brain);
        self.brain_stale = false;
        self.villager_raid_activity = None;
        self.tongue_target = None;
        true
    }

    pub(crate) fn tick_brain(
        &mut self,
        (entity, entities, teams): (
            &crate::entity::Entity,
            &crate::entity::EntityStore,
            &ScoreboardTeams,
        ),
        level: &mut LevelCtx<'_, '_>,
        (mob, inherits_malus): (MobPathingSnapshot, bool),
        random: &GameRandom,
        effects: &mut Vec<BrainWorldEffect>,
    ) -> bool {
        let game_time = level.game_time();
        if entity.entity_type_id() == VANILLA_ENTITY_TYPE_VILLAGER_ID
            && game_time.rem_euclid(20) == i64::from(entity.id().rem_euclid(20))
        {
            self.villager_raid_activity = level.villager_raid_activity(entity.block_position());
        }
        let Some(brain) = self.brain.as_mut() else {
            return false;
        };
        let mut body = BrainBodyFacts::of(
            entity,
            entities.difficulty() == Some(bbb_protocol::difficulties::Difficulty::Peaceful),
            mob.malus(),
            inherits_malus,
            teams,
        );
        body.melee_reach = Some(super::MeleeReach::of(entity, entities));
        if let Some(villager) = &mut body.villager {
            villager.scheduled = level.villager_schedule_activity(villager.baby);
            villager.has_farm_seeds = crate::entity::villager_life::has_farm_seeds(entity, level);
        }
        let body = &body;
        let navigation = &mut self.navigation;
        if body.entity_type_id == VANILLA_ENTITY_TYPE_VILLAGER_ID {
            if let Some(scheduled) = body.villager.and_then(|villager| villager.scheduled) {
                brain.start_from_schedule(game_time, scheduled);
            }
            villager::wanted_item::tick(brain, entity, entities, level);
        }
        let mut context = BrainTickContext::new(level.game_time(), random.clone());
        let mut look_request = None;
        let mut jump_request = false;
        let mut world = MobBrainWorld {
            body,
            body_entity: entity,
            passengers: entities.passengers(entity.id()),
            sensing: &self.sensing,
            mob,
            effective_malus: std::cell::Cell::new(if navigation.frog_enabled() {
                mob.malus()
            } else {
                body.pathfinding_malus
            }),
            own_malus: std::cell::Cell::new(self.pathfinding_malus),
            entities,
            level,
            navigation,
            prepared_path: None,
            tongue_target: &mut self.tongue_target,
            look_request: &mut look_request,
            jump_request: &mut jump_request,
            effects,
        };
        brain.tick(&mut context, &mut world);
        if body
            .villager
            .is_some_and(|villager| villager.assign_profession_when_spawned)
        {
            world
                .effects
                .push(BrainWorldEffect::ClearAssignProfessionWhenSpawned);
        }
        if world.navigation.frog_enabled() {
            self.pathfinding_malus = world.own_malus.get();
        }
        let pending_spawn_finish = world.effects.iter().any(|effect| matches!(effect,
            BrainWorldEffect::SetMobBrainMemory { entity_id, memory_type: MemoryModuleType::IsPregnant, value: None }
            if *entity_id == body.entity_id
        ));
        self.brain_look_request = look_request;
        self.brain_jump_request = jump_request;
        if !pending_spawn_finish {
            update_activity_for(
                body.entity_type_id,
                brain,
                context.game_time(),
                body,
                self.villager_raid_activity,
            );
        }
        true
    }

    pub(crate) fn finish_frog_spawn(&mut self) {
        if let Some(brain) = self.brain.as_mut() {
            frog::update_activity(brain);
        }
    }

    pub(crate) fn take_brain_look_request(&mut self) -> Option<Vec3d> {
        self.brain_look_request.take()
    }

    pub(crate) fn take_brain_jump_request(&mut self) -> bool {
        std::mem::take(&mut self.brain_jump_request)
    }

    pub(crate) fn live_brain(&self) -> Option<&Brain> {
        self.brain.as_ref()
    }

    pub(crate) fn live_brain_pack(&self) -> Option<BrainPacked> {
        Some(self.brain.as_ref()?.pack())
    }
}

pub(crate) fn apply_brain_effects(
    entity: &mut crate::entity::Entity,
    store: &mut crate::entity::EntityStore,
    level: &mut LevelCtx<'_, '_>,
    effects: Vec<BrainWorldEffect>,
) -> bool {
    for effect in effects {
        match effect {
            BrainWorldEffect::SetPose { pose_id } => {
                if entity
                    .set_pose_id_with_level(pose_id, level, Some(&mut *store))
                    .is_none()
                {
                    return false;
                }
            }
            BrainWorldEffect::PlayBodySound {
                resource_id,
                source,
                volume,
                pitch,
            } => {
                store.play_sound_from_entity_at(
                    entity.id(),
                    entity.position(),
                    resource_id,
                    source,
                    volume,
                    pitch,
                );
            }
            BrainWorldEffect::SetDeltaMovement {
                entity_id,
                movement,
            } => {
                if let Some(target) = store.get_mut(entity_id) {
                    if target.delta_movement() != movement {
                        target.set_delta_movement(movement);
                        target.set_needs_sync(true);
                    }
                }
            }
            BrainWorldEffect::HurtEntity {
                target_id,
                damage_type,
                amount,
                remove_if_killed,
            } => {
                let Some(hurted) =
                    hurt_entity_from_body(entity, store, level, target_id, damage_type, amount)
                else {
                    return false;
                };
                if remove_if_killed
                    && store
                        .get(target_id)
                        .is_some_and(|target| !entity_is_alive(target))
                {
                    store.remove(target_id, crate::entity::RemovalReason::Killed);
                }
                if hurted {
                    queue_post_attack(entity, level, target_id, damage_type);
                }
            }
            BrainWorldEffect::SwingMainHand => {
                let hand = bbb_protocol::packets::InteractionHand::MainHand;
                if entity.living_mut().is_some_and(|living| living.swing(hand)) {
                    level.entity_animation(entity.id(), hand);
                }
            }
            BrainWorldEffect::MeleeAttack { target_id } => {
                if !super::melee::do_hurt_target(entity, store, level, target_id) {
                    return false;
                }
            }
            BrainWorldEffect::SetMobBrainMemory {
                entity_id,
                memory_type,
                value,
            } => {
                let own_spawn_finish = entity_id == entity.id()
                    && memory_type == MemoryModuleType::IsPregnant
                    && value.is_none();
                let current = (entity_id == entity.id()).then_some(&*entity);
                store.set_mob_brain_memory(entity_id, current, memory_type, value);
                if own_spawn_finish {
                    if let Some(runtime) = store.mob_ai_mut(entity_id) {
                        runtime.finish_frog_spawn();
                    }
                }
            }
            BrainWorldEffect::SpawnChildFromBreeding { partner_id } => {
                let random = store.random().clone();
                let bred = crate::entity::animal_breeding::spawn_child_from_breeding(
                    entity, store, level, partner_id, &random,
                );
                if !bred {
                    return false;
                }
            }
            BrainWorldEffect::SetBodyDeltaMovement { movement } => {
                entity.set_delta_movement(movement);
            }
            BrainWorldEffect::SetBodyYRot { y_rot } => entity.set_rot(y_rot, entity.x_rot()),
            BrainWorldEffect::SetBodyDiscardFriction { discard } => {
                entity.set_discard_friction(discard);
            }
            BrainWorldEffect::BodyEntityEvent { event_id } => {
                if matches!(event_id, 58 | 59) {
                    crate::entity::goat::set_lowering_head(entity, event_id == 58);
                }
                store.broadcast_to_tracking(
                    entity.id(),
                    crate::connection::ClientboundPlayPacket::EntityEvent(
                        bbb_protocol::packets::EntityEvent {
                            entity_id: entity.id(),
                            event_id,
                        },
                    ),
                );
            }
            BrainWorldEffect::SetVillagerProfession { profession } => {
                crate::entity::general_living::set_villager_profession(entity, profession);
                store.queue_command(crate::entity::EntityCommand::Villager(
                    crate::entity::villager::PendingVillagerAction::GenerateOffers {
                        entity_id: entity.id(),
                    },
                ));
            }
            BrainWorldEffect::ClearAssignProfessionWhenSpawned => {
                crate::entity::general_living::clear_villager_assign_profession_when_spawned(
                    entity,
                );
            }
            BrainWorldEffect::VillagerWorkedAtPoi { job_site } => {
                villager::worked_at_poi(entity, store, level.game_time(), level.overworld_day());
                crate::entity::villager_life::use_composter(entity, store, level, job_site);
            }
            BrainWorldEffect::EntityEvent {
                entity_id,
                event_id,
            } => {
                store.broadcast_to_tracking(
                    entity_id,
                    crate::connection::ClientboundPlayPacket::EntityEvent(
                        bbb_protocol::packets::EntityEvent {
                            entity_id,
                            event_id,
                        },
                    ),
                );
            }
            BrainWorldEffect::VillagerGiveBirth { partner_id } => {
                let random = store.random().clone();
                crate::entity::villager_life::give_birth(entity, store, level, partner_id, &random);
            }
            BrainWorldEffect::VillagerFarmAt { pos } => {
                crate::entity::villager_life::farm_at(entity, level, pos);
            }
            BrainWorldEffect::VillagerBonemealAt { pos } => {
                crate::entity::villager_life::bonemeal_at(entity, level, pos);
            }
            BrainWorldEffect::VillagerHeldItem(held) => {
                crate::entity::villager_life::set_held_item(entity, held);
            }
            BrainWorldEffect::VillagerSpawnGolem { needed } => {
                store.queue_command(crate::entity::EntityCommand::Villager(
                    crate::entity::villager::PendingVillagerAction::SpawnGolemIfNeeded {
                        entity_id: entity.id(),
                        needed,
                    },
                ));
            }
            BrainWorldEffect::TradeWithVillager { target_id } => {
                let random = store.random().clone();
                if crate::entity::villager_life::gossip(
                    entity,
                    store,
                    target_id,
                    level.game_time(),
                    &random,
                ) {
                    store.queue_command(crate::entity::EntityCommand::Villager(
                        crate::entity::villager::PendingVillagerAction::SpawnGolemIfNeeded {
                            entity_id: entity.id(),
                            needed: 5,
                        },
                    ));
                }
                crate::entity::villager_life::trade_with_villager(entity, store, target_id);
            }
            BrainWorldEffect::StartSleeping { pos } => {
                if crate::entity::living::start_sleeping(entity, store, level, pos).is_none() {
                    return false;
                }
            }
            BrainWorldEffect::StopSleeping => {
                if entity.sleeping_pos().is_some()
                    && level
                        .stop_sleeping(
                            crate::entity::level::EntityBlockContext::new(Some(&mut *store), None),
                            entity,
                        )
                        .is_none()
                {
                    return false;
                }
            }
            BrainWorldEffect::PlaceBlock {
                pos,
                block_state_id,
            } => {
                let _ = level.set_entity_block(
                    crate::entity::level::EntityBlockContext::new(Some(&mut *store), Some(entity)),
                    pos,
                    block_state_id,
                    crate::fluid::SET_BLOCK_UPDATE_ALL,
                );
                level.game_event_with_state(
                    crate::level_event::VanillaGameEvent::BLOCK_PLACE,
                    pos,
                    Some(entity.id()),
                    Some(block_state_id),
                );
            }
        }
    }
    true
}

pub(super) fn queue_post_attack(
    entity: &crate::entity::Entity,
    level: &mut LevelCtx<'_, '_>,
    target_id: i32,
    damage_type: &'static str,
) {
    let attacker = crate::level_event::PendingHurtEntity::Stored {
        entity_id: entity.id(),
    };
    level.post_attack(crate::level_event::PendingPostAttack {
        victim_entity_id: target_id,
        damage_type,
        causing: Some(attacker.clone()),
        direct: Some(attacker),
    });
}

pub(crate) fn hurt_entity_from_body(
    entity: &crate::entity::Entity,
    store: &mut crate::entity::EntityStore,
    level: &mut LevelCtx<'_, '_>,
    target_id: i32,
    damage_type: &str,
    amount: f32,
) -> Option<bool> {
    level.with_detached_entity(entity, |level| {
        hurt_entity_from_body_inner(entity, store, level, target_id, damage_type, amount)
    })
}

fn hurt_entity_from_body_inner(
    entity: &crate::entity::Entity,
    store: &mut crate::entity::EntityStore,
    level: &mut LevelCtx<'_, '_>,
    target_id: i32,
    damage_type: &str,
    amount: f32,
) -> Option<bool> {
    let Some(facts) = damage_type_facts(damage_type) else {
        return Some(false);
    };
    let source = crate::entity::damage::StoredEntityDamageSource {
        damage_type: facts,
        direct_entity_id: Some(entity.id()),
        causing_entity: Some(crate::entity::StoredDamageCausingEntity::Mob {
            entity_id: entity.id(),
        }),
        attacker_views: Default::default(),
    };
    let rules = crate::entity::StoredEntityDamageRules {
        mob_griefing: level.mob_griefing(),
        entity_drops: level.entity_drops(),
        tnt_explodes: level.tnt_explodes(),
        fall_damage_immune: false,
    };
    let Some(mut target) = store.detach(target_id) else {
        return Some(false);
    };
    if matches!(target.data(), crate::entity::EntityData::ArmorStand(_)) {
        let Some(decision) = crate::entity::armor_stand::hurt_decision_in_level(
            &target,
            source,
            rules,
            crate::entity::damage::StoredEntityDamageContext {
                block_drops: level.block_drops(),
                ..Default::default()
            },
            level,
        ) else {
            store.attach(target);
            return None;
        };
        let Some(effects) = crate::entity::armor_stand::hurt_server_with_decision(
            &mut target,
            store,
            decision,
            level.block_drops(),
        ) else {
            store.attach(target);
            return None;
        };
        if effects.emits_entity_damage {
            level.game_event_at(
                crate::level_event::VanillaGameEvent::ENTITY_DAMAGE,
                target.position(),
                Some(entity.id()),
            );
        }
        if effects.emits_entity_die {
            crate::entity::kill::raise_entity_die(&target, level);
        }
        store.attach(target);
        return Some(effects.outcome.succeeded);
    }
    if !matches!(target.data(), crate::entity::EntityData::GeneralLiving(_)) {
        store.attach(target);
        return Some(false);
    }
    let position = target.position();
    let effects = crate::entity::general_living::hurt_live_entity(
        &mut target,
        store,
        level,
        None,
        source,
        None,
        None,
        None,
        amount,
        rules,
        Some(entity.position()),
        None,
        None,
    );
    if effects.emits_entity_damage {
        level.game_event_at(
            crate::level_event::VanillaGameEvent::ENTITY_DAMAGE,
            position,
            effects.game_event_source_id,
        );
    }
    let finished =
        crate::entity::general_living::emit_entity_die(store, level, &mut target, source, effects);
    store.attach(target);
    finished?;
    Some(effects.outcome.succeeded)
}

fn entity_is_alive(entity: &crate::entity::Entity) -> bool {
    entity.is_alive()
        && entity
            .living()
            .is_none_or(|living| !living.is_dead_or_dying())
}

/// One `minecraft:damage_type` from the built-in registry, which is where
/// every damage type this runtime names comes from.
fn damage_type_facts(resource_id: &str) -> Option<&'static DamageTypeFacts> {
    bbb_server_datapack::damage::builtin_damage_type_registry()
        .iter()
        .find(|facts| facts.resource_id == resource_id)
}

#[cfg(test)]
mod tests;
