use super::*;
use crate::entity::level::EntityBlockContext;
use crate::entity::raider::{RaiderRaidFacts, RaiderWaveFacts};

use crate::entity::GroundPathType;
use bbb_protocol::packets::Direction;

pub(super) const fn ground_path_type_to_cache(path_type: GroundPathType) -> i32 {
    match path_type {
        GroundPathType::Blocked => 0,
        GroundPathType::Open => 1,
        GroundPathType::Walkable => 2,
        GroundPathType::WalkableDoor => 3,
        GroundPathType::Trapdoor => 4,
        GroundPathType::PowderSnow => 5,
        GroundPathType::OnTopOfPowderSnow => 6,
        GroundPathType::Fence => 7,
        GroundPathType::Lava => 8,
        GroundPathType::Water => 9,
        GroundPathType::WaterBorder => 10,
        GroundPathType::Rail => 11,
        GroundPathType::UnpassableRail => 12,
        GroundPathType::FireInNeighbor => 13,
        GroundPathType::Fire => 14,
        GroundPathType::DamagingInNeighbor => 15,
        GroundPathType::Damaging => 16,
        GroundPathType::DoorOpen => 17,
        GroundPathType::DoorWoodClosed => 18,
        GroundPathType::DoorIronClosed => 19,
        GroundPathType::Breach => 20,
        GroundPathType::Leaves => 21,
        GroundPathType::StickyHoney => 22,
        GroundPathType::Cocoa => 23,
        GroundPathType::DamageCautious => 24,
        GroundPathType::OnTopOfTrapdoor => 25,
        GroundPathType::BigMobsCloseToDanger => 26,
    }
}

pub(super) const fn ground_path_type_from_cache(path_type: i32) -> Option<GroundPathType> {
    Some(match path_type {
        0 => GroundPathType::Blocked,
        1 => GroundPathType::Open,
        2 => GroundPathType::Walkable,
        3 => GroundPathType::WalkableDoor,
        4 => GroundPathType::Trapdoor,
        5 => GroundPathType::PowderSnow,
        6 => GroundPathType::OnTopOfPowderSnow,
        7 => GroundPathType::Fence,
        8 => GroundPathType::Lava,
        9 => GroundPathType::Water,
        10 => GroundPathType::WaterBorder,
        11 => GroundPathType::Rail,
        12 => GroundPathType::UnpassableRail,
        13 => GroundPathType::FireInNeighbor,
        14 => GroundPathType::Fire,
        15 => GroundPathType::DamagingInNeighbor,
        16 => GroundPathType::Damaging,
        17 => GroundPathType::DoorOpen,
        18 => GroundPathType::DoorWoodClosed,
        19 => GroundPathType::DoorIronClosed,
        20 => GroundPathType::Breach,
        21 => GroundPathType::Leaves,
        22 => GroundPathType::StickyHoney,
        23 => GroundPathType::Cocoa,
        24 => GroundPathType::DamageCautious,
        25 => GroundPathType::OnTopOfTrapdoor,
        26 => GroundPathType::BigMobsCloseToDanger,
        _ => return None,
    })
}

impl<'w> LevelCtx<'_, 'w> {
    pub(crate) fn scoreboard_teams(&self) -> &'w crate::entity::ScoreboardTeams {
        &self
            .loot_level_facts
            .scoreboard
            .expect("entity level requires scoreboard")
            .teams
    }

    pub(crate) fn player_team(
        &self,
        store: &crate::entity::EntityStore,
        uuid: uuid::Uuid,
    ) -> Option<u32> {
        let teams = self.scoreboard_teams();
        if teams.is_empty() {
            return None;
        }
        let player = self.loot_level_facts.player_by_uuid(Some(store), uuid)?;
        player.team(teams)
    }

    pub(crate) fn difficulty(&self) -> bbb_protocol::difficulties::Difficulty {
        self.full_preparation.originals.difficulty()
    }

    /// `getCurrentDifficultyAt(pos).getEffectiveDifficulty()`.
    pub(crate) fn effective_difficulty_at(&self, pos: BlockPos) -> Option<f32> {
        self.full_preparation
            .originals
            .local_effective_difficulty(self.chunks, pos)
    }

    /// `getCurrentDifficultyAt(pos).getSpecialMultiplier()`.
    pub(crate) fn special_multiplier_at(&self, pos: BlockPos) -> Option<f32> {
        self.full_preparation
            .originals
            .local_special_multiplier(self.chunks, pos)
    }

    pub(crate) fn is_thundering(&self) -> bool {
        self.weather.is_thundering()
    }

    pub(crate) fn find_nearest_map_structure(
        &mut self,
        tag: &str,
        origin: BlockPos,
        radius: i32,
        skip_known: bool,
    ) -> Option<BlockPos> {
        let ids = self
            .chunks
            .registry_tags()?
            .members("minecraft:worldgen/structure", tag)
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>();
        self.chunks
            .find_nearest_map_structure(origin, &ids, radius, skip_known)
            .ok()
            .flatten()
            .map(|(pos, _)| pos)
    }

    pub(crate) fn player_may_interact(&self, uuid: uuid::Uuid, pos: BlockPos) -> bool {
        self.block_interaction_policy.player_may_interact(
            uuid,
            pos,
            self.world_border,
            self.game_time,
        )
    }

    pub(crate) fn can_see_sky(&self, pos: BlockPos) -> bool {
        self.brightness(LightLayer::Sky, pos) >= MAX_LIGHT_LEVEL
    }

    pub(crate) fn begin_portal_tick(&mut self, context: Option<(bool, i32, bool)>) {
        self.portal_context = context;
        self.portal_request = None;
    }

    pub(crate) fn take_portal_request(
        &mut self,
    ) -> Option<crate::entity::portal::PortalTransitionRequest> {
        self.portal_context = None;
        self.portal_request.take()
    }

    pub(crate) fn players(&self) -> &[crate::entity::NearbyPlayer] {
        self.players
    }

    pub(crate) fn fast_lava_checked(&self) -> Option<bool> {
        self.structure_services.as_ref()?.fast_lava
    }

    pub(crate) fn set_piston_no_clip(&mut self, direction: Option<Direction>) {
        crate::world::movement_collision::set_piston_no_clip(direction);
    }

    pub(crate) fn trigger_end_gateway_cooldown(&mut self, pos: BlockPos) -> bool {
        let Some(gateway) = self
            .chunks
            .block_entities_mut()
            .get_mut(pos)
            .and_then(BlockEntity::end_gateway_mut)
        else {
            return false;
        };
        if gateway.is_cooling_down() {
            return false;
        }
        gateway.trigger_cooldown();
        self.chunks.block_entities_mut().set_changed(pos);
        self.block_events.block_event(BlockEventData::new(
            pos,
            bbb_protocol::block_types::VANILLA_BLOCK_END_GATEWAY_ID,
            1,
            0,
        ));
        true
    }

    pub(crate) fn can_stop_sleeping(
        &mut self,
        _context: EntityBlockContext<'_>,
        entity: &crate::entity::Entity,
    ) -> bool {
        let Some(pos) = entity.sleeping_pos() else {
            return true;
        };
        let Some(state) = self.block_state_checked(pos) else {
            return false;
        };
        !crate::block_behavior::bed::is_bed(state)
            || crate::world::player_sleep::stand_up::stand_up_position(
                self.chunks,
                self.world_border,
                self.game_time,
                pos,
                entity.entity_type_id(),
                crate::block_behavior::direction_property(state, "facing"),
                entity.y_rot(),
            )
            .is_some()
    }

    pub(crate) fn bed_wake_position(
        &mut self,
        context: EntityBlockContext<'_>,
        pos: BlockPos,
        entity_type: i32,
        yaw: f32,
    ) -> Option<Vec3d> {
        let mut level = self.with_entity_context(context);
        let state = level.block_state_checked(pos)?;
        let facing = crate::block_behavior::direction_property(state, "facing");
        let unoccupied = crate::fluid::with_property(state, "occupied", "false")?;
        crate::world::player_sleep::stand_up::stand_up_position(
            level.chunks,
            level.world_border,
            level.game_time,
            pos,
            entity_type,
            facing,
            yaw,
        )?;
        LevelCtx::set_entity_block(
            &mut level,
            EntityBlockContext::new(None, None),
            pos,
            unoccupied,
            crate::fluid::SET_BLOCK_UPDATE_ALL,
        );
        let landing = crate::world::player_sleep::stand_up::stand_up_position(
            level.chunks,
            level.world_border,
            level.game_time,
            pos,
            entity_type,
            facing,
            yaw,
        )?;
        Some(landing.unwrap_or(Vec3d {
            x: f64::from(pos.x) + 0.5,
            y: f64::from(pos.y) + 1.1,
            z: f64::from(pos.z) + 0.5,
        }))
    }

    pub(crate) fn stop_sleeping(
        &mut self,
        mut context: EntityBlockContext<'_>,
        entity: &mut crate::entity::Entity,
    ) -> Option<()> {
        if let Some(pos) = entity.sleeping_pos() {
            let bed = self
                .block_state_checked(pos)
                .is_some_and(crate::block_behavior::bed::is_bed);
            let position = bed
                .then(|| {
                    self.bed_wake_position(
                        EntityBlockContext {
                            store: context.store.as_deref_mut(),
                            current: Some(entity),
                            ancestors: context.ancestors,
                        },
                        pos,
                        entity.entity_type_id(),
                        entity.y_rot(),
                    )
                })
                .flatten();
            if let Some(position) = position {
                entity.set_pos(position);
                if let Some(store) = context.store.as_deref_mut() {
                    store.refile_detached(entity);
                }
                entity.set_rot(
                    crate::world::player_sleep::look_at_bed_yaw(position, pos),
                    0.0,
                );
            }
        }
        let position = entity.position();
        let _ = entity.set_pose_id_with_level(
            bbb_protocol::entity_poses::VANILLA_ENTITY_POSE_STANDING_ID,
            self,
            context.store.as_deref_mut(),
        );
        if let Some(store) = context.store.as_deref_mut() {
            store.refile_detached(entity);
        }
        entity.set_pos(position);
        entity.set_sleeping_pos(None);
        if let Some(store) = context.store.as_deref_mut() {
            store.refile_detached(entity);
            // `Villager.stopSleeping` (`Villager.java:891-894`), whatever woke it.
            if crate::entity::villager::is_villager(entity) {
                if let Some(ai) = store.mob_ai_mut(entity.id()) {
                    ai.set_brain_memory(
                        crate::entity::ai::MemoryModuleType::LastWoken,
                        crate::entity::ai::MemoryValue::Long(self.game_time()),
                    );
                }
            }
        }
        Some(())
    }

    pub(crate) fn block_state_checked(&self, pos: BlockPos) -> Option<i32> {
        if self.chunks.is_inside_build_height(pos.y) {
            self.chunks.get_block_state(pos.x, pos.y, pos.z)
        } else {
            bbb_protocol::block_state_types::vanilla_block_default_state_id(
                bbb_protocol::block_types::VANILLA_BLOCK_VOID_AIR_ID,
            )
        }
    }

    pub(crate) fn clip_collider_source_only(
        &mut self,
        store: Option<&mut EntityStore>,
        entity: &crate::entity::Entity,
        from: Vec3d,
        to: Vec3d,
    ) -> Option<Option<BlockPos>> {
        let level = self.with_entity_context(EntityBlockContext::new(store, Some(entity)));
        super::falling_clip::collider_source_only(&level, entity, from, to)
    }

    pub(crate) fn clip_outline_for_entity(
        &self,
        entity: &Entity,
        from: Vec3d,
        to: Vec3d,
    ) -> Option<Option<BlockPos>> {
        super::super::pov_clip::clip_entity_outline(self.chunks, entity, from, to)
            .map(|hit| hit.map(|hit| hit.pos))
    }

    pub(crate) fn placement_state_from_neighbors(&mut self, state: i32, pos: BlockPos) -> i32 {
        super::update_from_neighbour_shapes(self, state, pos)
    }

    pub(crate) fn concrete_powder_on_land(
        &mut self,
        context: EntityBlockContext<'_>,
        original_state: i32,
        pos: BlockPos,
        replaced_state: i32,
    ) -> Option<()> {
        let mut level = self.with_entity_context(context);
        crate::block_behavior::concrete_powder::on_land(
            &mut level,
            original_state,
            pos,
            replaced_state,
        )
    }

    pub(crate) fn decrease_turtle_eggs(
        &mut self,
        context: EntityBlockContext<'_>,
        state: i32,
        pos: BlockPos,
    ) -> Option<()> {
        let mut level = self.with_entity_context(context);
        crate::block_behavior::turtle_egg::decrease_eggs(&mut level, pos, state)
    }

    pub(crate) fn interact_redstone_ore(
        &mut self,
        context: EntityBlockContext<'_>,
        state: i32,
        pos: BlockPos,
    ) -> Option<()> {
        let mut level = self.with_entity_context(context);
        crate::block_behavior::redstone_ore::interact(&mut level, pos, state)
    }

    pub(crate) fn step_on_sculk_sensor(
        &mut self,
        context: EntityBlockContext<'_>,
        state: i32,
        pos: BlockPos,
        source: crate::entity::level::SculkSensorStepSource,
    ) -> Option<()> {
        let mut level = self.with_entity_context(context);
        level.sculk_sensor_step(state, pos, source)
    }

    pub(crate) fn step_on_sculk_shrieker(
        &mut self,
        context: EntityBlockContext<'_>,
        pos: BlockPos,
        source: crate::entity::level::SculkShriekerStepSource,
    ) -> Option<()> {
        let mut level = self.with_entity_context(context);
        level.sculk_shrieker_step(pos, source)
    }

    pub(crate) fn raid_at(&self, pos: BlockPos) -> Option<bool> {
        self.loot_level_facts.raids.map(|raids| raids.raid_at(pos))
    }

    pub(crate) fn raid_id_at(&self, pos: BlockPos) -> Option<i32> {
        self.loot_level_facts.raids?.raid_id_at(pos)
    }

    pub(crate) fn raider_raid(&self, id: i32) -> Option<RaiderRaidFacts> {
        self.loot_level_facts.raids?.raider_raid(id)
    }

    pub(crate) fn raider_wave(&self, id: i32, wave: i32) -> Option<RaiderWaveFacts> {
        self.loot_level_facts.raids?.raider_wave(id, wave)
    }

    pub(crate) fn villager_raid_activity(
        &self,
        pos: BlockPos,
    ) -> Option<crate::entity::ai::Activity> {
        self.loot_level_facts
            .raids
            .and_then(|raids| raids.villager_activity_at(pos))
    }

    pub(crate) fn block_collision_boxes(&self, pos: BlockPos) -> Vec<Aabb> {
        crate::world::movement_collision::block_collision_shape(
            self.chunks,
            pos,
            crate::world::movement_collision::current_piston_no_clip(),
        )
    }

    pub(crate) fn push_block_collision_boxes(&self, pos: BlockPos, out: &mut Vec<Aabb>) {
        crate::world::movement_collision::push_block_collision_shape(
            self.chunks,
            pos,
            crate::world::movement_collision::current_piston_no_clip(),
            out,
        )
    }

    pub(crate) fn block_collision_boxes_empty(&self, pos: BlockPos) -> Option<Vec<Aabb>> {
        crate::world::movement_collision::block_collision_shape_checked(
            self.chunks,
            pos,
            crate::world::movement_collision::current_piston_no_clip(),
        )
    }

    pub(crate) fn push_block_collision_boxes_empty(
        &self,
        pos: BlockPos,
        out: &mut Vec<Aabb>,
    ) -> Option<()> {
        crate::world::movement_collision::push_block_collision_shape_checked(
            self.chunks,
            pos,
            crate::world::movement_collision::current_piston_no_clip(),
            out,
        )
    }

    pub(crate) fn merge_falling_block_entity_data(
        &mut self,
        context: EntityBlockContext<'_>,
        pos: BlockPos,
        entries: &[bbb_protocol::NbtSummaryEntry],
    ) -> Option<()> {
        let mut level = self.with_entity_context(context);
        level.merge_falling_block_data(pos, entries)
    }

    pub(crate) fn block_collision_boxes_for_entity(
        &self,
        entity: &crate::entity::Entity,
        pos: BlockPos,
    ) -> Option<Vec<Aabb>> {
        crate::world::movement_collision::stored_body_collision_shape(
            self.chunks,
            entity,
            pos,
            crate::world::movement_collision::current_piston_no_clip(),
        )
    }

    pub(crate) fn push_block_collision_boxes_for_entity(
        &self,
        entity: &crate::entity::Entity,
        pos: BlockPos,
        out: &mut Vec<Aabb>,
    ) -> Option<()> {
        crate::world::movement_collision::push_stored_body_collision_shape(
            self.chunks,
            entity,
            pos,
            crate::world::movement_collision::current_piston_no_clip(),
            out,
        )
    }

    pub(crate) fn block_collision_boxes_for_size_growth(
        &self,
        entity: &crate::entity::Entity,
        pos: BlockPos,
    ) -> Option<Vec<Aabb>> {
        crate::world::movement_collision::stored_growth_collision_shape(
            self.chunks,
            entity,
            pos,
            crate::world::movement_collision::current_piston_no_clip(),
        )
    }

    pub(crate) fn capture_registry_tags(&self) -> EntityRegistryTagView {
        self.chunks.registry_tags_shared().map_or(
            EntityRegistryTagView::Builtin26_1,
            EntityRegistryTagView::Frozen,
        )
    }

    pub(crate) fn set_entity_block(
        &mut self,
        context: EntityBlockContext<'_>,
        pos: BlockPos,
        block_state_id: i32,
        flags: i32,
    ) -> bool {
        let mut level = self.with_entity_context(context);
        LevelCtx::set_block(&mut level, pos, block_state_id, flags, UPDATE_LIMIT)
    }

    pub(crate) fn entity_inside_block(
        &mut self,
        context: EntityBlockContext<'_>,
        pos: BlockPos,
        block_state_id: i32,
        inside: crate::block_behavior::InsideEntity,
    ) -> bool {
        let mut level = self.with_entity_context(context);
        crate::block_behavior::entity_inside(&mut level, pos, block_state_id, inside)
    }

    pub(crate) fn is_entity_ticking(&self, chunk_x: i32, chunk_z: i32) -> bool {
        self.simulation_levels.is_entity_ticking((chunk_x, chunk_z))
    }

    pub(crate) fn world_border(&self) -> Option<bbb_server_types::interaction::WorldBorder> {
        Some(self.world_border)
    }

    pub(crate) fn sea_level(&self) -> Option<i32> {
        Some(self.chunks.sea_level())
    }

    pub(crate) fn ground_path_type_from_state(&self, pos: BlockPos) -> Option<GroundPathType> {
        if let Some(path_type) = self
            .chunks
            .cached_path_type(pos)
            .and_then(ground_path_type_from_cache)
        {
            return Some(path_type);
        }
        let path_type = crate::entity::ground_path_type_from_state(self, pos)?;
        self.chunks
            .cache_path_type(pos, ground_path_type_to_cache(path_type));
        Some(path_type)
    }

    pub(crate) fn player(&self, uuid: Uuid) -> Option<NearbyPlayer> {
        self.players
            .iter()
            .find(|player| player.uuid == uuid)
            .copied()
    }

    fn other_entity_stores(&self) -> impl Iterator<Item = &EntityStore> {
        self.loot_level_facts
            .other_levels
            .into_iter()
            .flat_map(|levels| levels.iter().map(|(_, level)| &level.entities))
    }

    pub(crate) fn stored_entity_in_other_dimension(
        &self,
        uuid: Uuid,
    ) -> Option<(&Entity, &EntityStore)> {
        self.other_entity_stores().find_map(|store| {
            let id = store.visible_entity_id_for_uuid(uuid)?;
            Some((store.get(id)?, store))
        })
    }

    pub(crate) fn stored_entity_by_id_in_other_dimension(
        &self,
        id: i32,
    ) -> Option<(&Entity, &EntityStore)> {
        self.other_entity_stores()
            .find_map(|store| Some((store.get_visible(id)?, store)))
    }

    pub(crate) fn entity_in_other_dimension(
        &self,
        uuid: Uuid,
    ) -> Option<crate::entity::ReferenceTarget> {
        self.stored_entity_in_other_dimension(uuid)
            .and_then(|(entity, _)| crate::entity::ReferenceTarget::of_entity(entity))
            .or_else(|| self.cross_level_entities.get(uuid))
    }

    pub(crate) fn target_input_in_other_dimension(&self, uuid: Uuid) -> Option<&NearbyPlayer> {
        self.cross_level_entities.target_input(uuid)
    }

    pub(crate) fn target_input_by_id_in_other_dimension(
        &self,
        entity_id: i32,
    ) -> Option<&NearbyPlayer> {
        self.cross_level_entities.target_input_by_id(entity_id)
    }

    pub(crate) fn sculk_player_in_other_dimension(&self, uuid: Uuid) -> Option<Option<i32>> {
        self.entity_exists_in_other_dimension(uuid)?;
        let stored = self
            .stored_entity_in_other_dimension(uuid)
            .filter(|(entity, _)| entity.removal_reason().is_none())
            .and_then(|(entity, _)| {
                (entity.entity_type_id()
                    == bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_PLAYER_ID)
                    .then_some(entity.id())
            });
        Some(stored.or_else(|| self.cross_level_entities.player_entity_id(uuid)))
    }

    pub(crate) fn entity_exists_in_other_dimension(&self, uuid: Uuid) -> Option<bool> {
        for store in self.other_entity_stores() {
            let Some(id) = store.visible_entity_id_for_uuid(uuid) else {
                continue;
            };
            if store.is_detached(id) {
                return None;
            }
            if store
                .get(id)
                .is_some_and(|entity| entity.removal_reason().is_none())
                || store
                    .player(id)
                    .is_some_and(|player| !player.avatar_removed)
            {
                return Some(true);
            }
        }
        Some(self.cross_level_entities.target_input(uuid).is_some())
    }

    pub(crate) fn leash_holder_player(&self, entity_id: i32) -> Option<NearbyPlayer> {
        self.players
            .iter()
            .find(|player| player.entity_id == entity_id)
            .copied()
    }

    pub(crate) fn nearest_player(
        &self,
        position: Vec3d,
        max_distance: f64,
    ) -> Option<NearbyPlayer> {
        let mut best: Option<(f64, NearbyPlayer)> = None;
        for player in self.players {
            if player.spectator {
                continue;
            }
            let distance = {
                let (dx, dy, dz) = (
                    player.position.x - position.x,
                    player.position.y - position.y,
                    player.position.z - position.z,
                );
                dx * dx + dy * dy + dz * dz
            };
            if !(max_distance < 0.0 || distance < max_distance * max_distance) {
                continue;
            }
            if best.is_none_or(|(best_distance, _)| distance < best_distance) {
                best = Some((distance, *player));
            }
        }
        best.map(|(_, player)| player)
    }

    pub(crate) fn has_nearby_alive_player(&self, position: Vec3d, max_distance: f64) -> bool {
        self.players.iter().any(|player| {
            if player.spectator || player.dead_or_dying {
                return false;
            }
            let dx = player.position.x - position.x;
            let dy = player.position.y - position.y;
            let dz = player.position.z - position.z;
            max_distance < 0.0 || dx * dx + dy * dy + dz * dz < max_distance * max_distance
        })
    }

    pub(crate) fn min_y(&self) -> i32 {
        self.chunks.min_y()
    }

    pub(crate) fn max_y(&self) -> i32 {
        self.chunks.min_y() + self.chunks.section_count() as i32 * 16 - 1
    }

    pub(crate) fn logical_height(&self) -> Option<i32> {
        Some(self.logical_height)
    }

    pub(crate) fn entity_drops(&self) -> bool {
        self.game_rules.entity_drops
    }

    pub(crate) fn mob_drops(&self) -> bool {
        self.game_rules.mob_drops
    }

    pub(crate) fn mob_loot_drops(
        &mut self,
        bodies: Option<crate::entity::store::EntityBodyReadView<'_>>,
        request: crate::entity::mob_death_loot::MobLootRequest<'_>,
    ) -> crate::entity::mob_death_loot::MobLootRoll {
        use crate::entity::mob_death_loot::bind_body_attacker;
        let bodies = bodies.map(|bodies| bodies.with_ancestors(self.detached_entities));
        let request = match bodies {
            Some(bodies) => crate::entity::mob_death_loot::MobLootRequest {
                attacking_entity: bind_body_attacker(bodies, self, request.attacking_entity),
                direct_attacking_entity: bind_body_attacker(
                    bodies,
                    self,
                    request.direct_attacking_entity,
                ),
                ..request
            },
            None => request,
        };
        super::mob_death_loot::mob_loot_drops(self, bodies, &request)
    }

    pub(crate) fn gift_loot_drops(
        &mut self,
        bodies: crate::entity::store::EntityBodyReadView<'_>,
        entity: &crate::entity::Entity,
        table_id: &str,
    ) -> Vec<crate::loot_runtime::LootStack> {
        let bodies = bodies.with_ancestors(self.detached_entities);
        super::mob_death_loot::gift_loot_drops(self, bodies, entity, table_id)
    }

    pub(crate) fn piglin_barter_loot_drops(
        &mut self,
        bodies: crate::entity::store::EntityBodyReadView<'_>,
        entity: &crate::entity::Entity,
    ) -> Vec<crate::loot_runtime::LootStack> {
        let bodies = bodies.with_ancestors(self.detached_entities);
        super::mob_death_loot::piglin_barter_loot_drops(self, bodies, entity)
    }

    /// `FishingHook.retrieve`'s draw from `gameplay/fishing`.
    pub(crate) fn fishing_loot_drops(
        &mut self,
        bodies: crate::entity::store::EntityBodyReadView<'_>,
        hook: &crate::entity::Entity,
        rod: &crate::loot_runtime::LootTool,
        luck: f32,
        biome_at: &crate::loot_runtime::LootBiomeAt<'_>,
    ) -> Vec<crate::loot_runtime::LootStack> {
        let bodies = bodies.with_ancestors(self.detached_entities);
        super::mob_death_loot::fishing_loot_drops(self, bodies, hook, rod, luck, biome_at)
    }

    pub(crate) fn equipment_drop_chance(
        &mut self,
        request: crate::entity::mob_death_loot::EquipmentDropChanceRequest<'_>,
    ) -> Result<f32, String> {
        let request = crate::entity::mob_death_loot::EquipmentDropChanceRequest {
            bodies: request.bodies.with_ancestors(self.detached_entities),
            ..request
        };
        let LevelCtx {
            drop_tables,
            loot_level_facts,
            random,
            chunks,
            weather,
            ..
        } = self;
        let blocks = LoadedBlocks { chunks };
        super::equipment_drop_chance::equipment_drop_chance(
            drop_tables,
            loot_level_facts,
            random,
            &blocks,
            crate::loot_runtime::LootWeather {
                raining: weather.is_raining(),
                thundering: weather.is_thundering(),
            },
            request,
        )
    }

    pub(crate) fn enchanted_damage_immunity(
        &mut self,
        request: crate::entity::enchanted_damage::EnchantedDamageRequest<'_>,
    ) -> Result<bool, String> {
        use crate::entity::mob_death_loot::bind_body_attacker;
        let bodies = request.bodies.with_ancestors(self.detached_entities);
        let request = crate::entity::enchanted_damage::EnchantedDamageRequest {
            bodies,
            causing: bind_body_attacker(bodies, self, request.causing),
            direct: bind_body_attacker(bodies, self, request.direct),
            ..request
        };
        super::enchanted_damage::immunity(self, request)
    }

    pub(crate) fn enchanted_damage_value(
        &mut self,
        request: crate::entity::enchanted_damage::EnchantedDamageRequest<'_>,
        value: crate::enchantment_combat_effects::CombatValue,
        amount: f32,
    ) -> Result<f32, String> {
        let request = crate::entity::enchanted_damage::EnchantedDamageRequest {
            bodies: request.bodies.with_ancestors(self.detached_entities),
            ..request
        };
        super::enchanted_damage::value(self, request, value, amount)
    }

    pub(crate) fn durability_change(
        &mut self,
        stack: &bbb_server_types::item_stack::RuntimeItemStack,
        amount: i32,
    ) -> Result<i32, String> {
        super::enchanted_damage::durability_change(self, stack, amount)
    }

    pub(crate) fn location_effect_requirements(
        &mut self,
        request: crate::entity::location_effects::LocationRequirementRequest<'_>,
    ) -> Result<bool, String> {
        let LevelCtx {
            chunks,
            drop_tables,
            loot_level_facts,
            random,
            weather,
            ..
        } = self;
        let bodies = match request.this {
            crate::loot_runtime::LootEntity::Stored(stored) => stored.bodies,
            _ => None,
        };
        let weather = crate::loot_runtime::LootWeather {
            raining: weather.is_raining(),
            thundering: weather.is_thundering(),
        };
        super::block_entity_loot::with_predicate_loot_context(
            chunks,
            loot_level_facts,
            drop_tables.tables,
            weather,
            random,
            bodies,
            true,
            None,
            "no loot registries are loaded",
            |loot_refs| {
                crate::world::location_effects::location_condition_matches(
                    &crate::world::location_effects::LocationContextLevel {
                        vehicle_slots: Some(loot_refs.vehicle_slots),
                        dimension: loot_refs.dimension,
                        item_environment: Some(loot_refs.item_environment),
                        loot_tables: drop_tables.tables,
                        blocks: loot_refs.block_at,
                        block_entity_data: loot_refs.block_entity_data,
                        weather,
                    },
                    random,
                    request,
                )
            },
        )
    }

    /// `BlockPredicate.test` over this level, where `unobstructed` is the
    /// caller's entity view for `BlockPredicate.unobstructed()`.
    pub(crate) fn block_predicate_test(
        &self,
        predicate: &bbb_server_datapack::worldgen::BlockPredicate,
        pos: BlockPos,
        unobstructed: &dyn Fn(BlockPos) -> bool,
    ) -> Result<bool, String> {
        use bbb_server_datapack::worldgen::BlockPredicate;
        // The predicate only asks about its own origin, so the caller's answer
        // is bound in before the level evaluates it.
        fn bind(predicate: &mut BlockPredicate, free: &mut dyn FnMut() -> bool) {
            match predicate {
                BlockPredicate::Unobstructed if free() => *predicate = BlockPredicate::True,
                BlockPredicate::Unobstructed => {
                    *predicate = BlockPredicate::Not(Box::new(BlockPredicate::True))
                }
                BlockPredicate::AllOf(all) | BlockPredicate::AnyOf(all) => {
                    all.iter_mut().for_each(|predicate| bind(predicate, free))
                }
                BlockPredicate::Not(predicate) => bind(predicate, free),
                _ => {}
            }
        }
        let mut predicate = predicate.clone();
        let mut answer = None;
        bind(&mut predicate, &mut || {
            *answer.get_or_insert_with(|| unobstructed(pos))
        });
        bbb_server_worldgen::block_predicates::block_predicate_test(&predicate, self, pos)
            .ok_or_else(|| "malformed block predicate".to_string())
    }

    pub(crate) fn block_drops(&self) -> bool {
        self.game_rules.block_drops
    }

    pub(crate) fn crafting_recipes(
        &self,
    ) -> Option<crate::entity::block_drops::RecipeDropFacts<'_>> {
        self.drop_tables.recipes
    }

    pub(crate) fn mob_griefing(&self) -> bool {
        self.game_rules.mob_griefing
    }

    pub(crate) fn projectiles_can_break_blocks(&self) -> bool {
        self.game_rules.projectiles_can_break_blocks
    }

    pub(crate) fn spawner_blocks_work(&self) -> bool {
        self.game_rules.spawner_blocks_work
    }

    pub(crate) fn command_blocks_work(&self) -> bool {
        self.game_rules.command_blocks_work
    }

    pub(crate) fn tnt_explodes(&self) -> bool {
        self.game_rules.tnt_explodes
    }

    pub(crate) fn max_entity_cramming(&self) -> i32 {
        self.game_rules.max_entity_cramming
    }

    pub(crate) fn game_event(
        &mut self,
        game_event: crate::level_event::VanillaGameEvent,
        pos: BlockPos,
        source_entity_id: Option<i32>,
    ) {
        self.event_sink
            .game_event_with_source(game_event, pos, source_entity_id, None);
    }

    pub(crate) fn game_event_at(
        &mut self,
        game_event: crate::level_event::VanillaGameEvent,
        position: Vec3d,
        source_entity_id: Option<i32>,
    ) {
        self.game_event_at_with_state(game_event, position, source_entity_id, None);
    }

    pub(crate) fn game_event_at_with_state(
        &mut self,
        game_event: crate::level_event::VanillaGameEvent,
        position: Vec3d,
        source_entity_id: Option<i32>,
        affected_state: Option<i32>,
    ) {
        self.event_sink.game_event_with_source_at(
            game_event,
            position,
            source_entity_id,
            affected_state,
        );
    }

    pub(crate) fn game_event_with_state(
        &mut self,
        game_event: crate::level_event::VanillaGameEvent,
        pos: BlockPos,
        source_entity_id: Option<i32>,
        affected_state: Option<i32>,
    ) {
        self.event_sink
            .game_event_with_source(game_event, pos, source_entity_id, affected_state);
    }

    pub(crate) fn spawn_runtime_item(
        &mut self,
        position: Vec3d,
        stack: bbb_server_types::item_stack::RuntimeItemStack,
        default_pickup_delay: bool,
    ) -> Option<()> {
        if !stack.is_empty() {
            self.item_spawns
                .items
                .push(crate::entity::block_drops::PendingItemSpawn {
                    position,
                    item: stack,
                    default_pickup_delay,
                });
        }
        Some(())
    }

    pub(crate) fn spawn_item(&mut self, position: Vec3d, stack: &ItemStackSummary) {
        if item_stack_summary_is_empty(stack) {
            return;
        }
        let LevelCtx { item_spawns, .. } = self;
        add_fresh_item_entity(position, stack, item_spawns);
    }

    pub(crate) fn spawn_item_without_pickup_delay(
        &mut self,
        position: Vec3d,
        stack: &ItemStackSummary,
    ) {
        if item_stack_summary_is_empty(stack) {
            return;
        }
        let LevelCtx { item_spawns, .. } = self;
        crate::entity::block_drops::add_fresh_item_entity_without_pickup_delay(
            position,
            stack,
            item_spawns,
        );
    }

    pub(crate) fn unpack_vehicle_container_body(
        &mut self,
        entity: &mut crate::entity::Entity,
        context: crate::entity::level::EntityBlockContext<'_>,
    ) -> Option<()> {
        self.unpack_actual_vehicle_container(entity, context)
    }

    pub(crate) fn item_stack_resolver(
        &self,
    ) -> &'_ dyn bbb_registry_codec::PersistentItemStackHolderResolver {
        self.loot_level_facts.item_stack_publication()
    }

    pub(crate) fn queue_death_experience(
        &mut self,
        _store: &mut EntityStore,
        victim: &Entity,
        position: Vec3d,
        killer_entity_id: Option<i32>,
    ) {
        self.event_sink
            .queue_death_experience(victim.id(), position, killer_entity_id);
    }

    pub(crate) fn entity_animation(
        &mut self,
        entity_id: i32,
        hand: bbb_protocol::packets::InteractionHand,
    ) {
        self.event_sink.entity_animation(entity_id, hand);
    }

    pub(crate) fn destroy_block_progress(&mut self, entity_id: i32, pos: BlockPos, stage: i32) {
        self.event_sink
            .destroy_block_progress(entity_id, pos, stage);
    }

    pub(crate) fn global_level_event(&mut self, event_id: i32, pos: BlockPos, data: i32) {
        self.event_sink.global_level_event(event_id, pos, data);
    }

    pub(crate) fn motion_blocking_height(&self, x: i32, z: i32) -> Option<i32> {
        self.chunks.final_heightmap_first_available(
            x >> 4,
            z >> 4,
            bbb_protocol::heightmap_types::VANILLA_HEIGHTMAP_MOTION_BLOCKING_ID,
            x & 15,
            z & 15,
        )
    }

    pub(crate) fn can_spread_fire_around(&self, pos: BlockPos) -> bool {
        crate::block_behavior::fire::spread_radius_allows(
            self.players,
            self.game_rules.fire_spread_radius_around_player,
            pos,
        )
    }

    /// `ServerLevel.findLightningRod` (`ServerLevel.java:596-606`): the block
    /// above the closest rod within 128 that tops its column.
    pub(crate) fn find_lightning_rod(&mut self, center: BlockPos) -> Option<BlockPos> {
        let rods = self.chunks.poi_mut().records_in_range(
            [center.x, center.y, center.z],
            128,
            crate::chunk::poi::PoiOccupancy::Any,
            |record| record.type_id == "minecraft:lightning_rod",
        );
        rods.into_iter()
            .filter(|record| {
                let [x, y, z] = record.pos;
                crate::world::natural_spawn::world_surface_height(self.chunks, BlockPos { x, y, z })
                    .is_some_and(|surface| y == surface - 1)
            })
            .min_by_key(|record| {
                record
                    .pos
                    .iter()
                    .zip([center.x, center.y, center.z])
                    .map(|(a, b)| (i64::from(*a) - i64::from(b)).pow(2))
                    .sum::<i64>()
            })
            .map(|record| BlockPos {
                x: record.pos[0],
                y: record.pos[1] + 1,
                z: record.pos[2],
            })
    }

    pub(crate) fn motion_blocking_no_leaves_height(&self, x: i32, z: i32) -> Option<i32> {
        self.chunks.final_heightmap_first_available(
            x >> 4,
            z >> 4,
            bbb_protocol::heightmap_types::VANILLA_HEIGHTMAP_MOTION_BLOCKING_NO_LEAVES_ID,
            x & 15,
            z & 15,
        )
    }

    pub(crate) fn player_by_entity_id(&self, entity_id: i32) -> Option<NearbyPlayer> {
        self.players
            .iter()
            .find(|player| player.entity_id == entity_id)
            .copied()
    }

    pub(crate) fn hurt_player(&mut self, hurt: crate::level_event::PendingPlayerHurt) {
        self.event_sink.hurt_player(hurt);
    }

    pub(crate) fn stab_player(&mut self, stab: crate::level_event::PendingPlayerStab) {
        self.event_sink.stab_player(stab);
    }

    pub(crate) fn fall_player(&mut self, fall: crate::level_event::PendingPlayerFall) {
        self.event_sink.fall_player(fall);
    }

    pub(crate) fn post_attack(&mut self, post_attack: crate::level_event::PendingPostAttack) {
        self.event_sink.post_attack(post_attack);
    }

    pub(crate) fn max_local_raw_brightness(&self, pos: BlockPos) -> Option<i32> {
        Some(LevelCtx::max_block_brightness(self, pos))
    }

    pub(crate) fn dimension_ambient_light(&self) -> Option<f32> {
        Some(self.ambient_light)
    }

    pub(crate) fn monsters_burn(&self, pos: Vec3d) -> Option<bool> {
        let services = self.structure_services.as_ref()?;
        services.monsters_burn.as_ref()?.sample(|| {
            self.chunks.biome_id_at_quart_or_source(
                (pos.x.floor() as i32) >> 2,
                (pos.y.floor() as i32) >> 2,
                (pos.z.floor() as i32) >> 2,
            )
        })
    }

    pub(crate) fn piglins_zombify(&self, pos: Vec3d) -> Option<bool> {
        let services = self.structure_services.as_ref()?;
        services.piglins_zombify.as_ref()?.sample(|| {
            self.chunks.biome_id_at_quart_or_source(
                (pos.x.floor() as i32) >> 2,
                (pos.y.floor() as i32) >> 2,
                (pos.z.floor() as i32) >> 2,
            )
        })
    }

    pub(crate) fn cat_waking_up_gift_chance(&self, pos: BlockPos) -> Option<f32> {
        self.full_preparation
            .originals
            .cat_waking_up_gift_chance(self.chunks, pos)
    }

    pub(crate) fn snow_golem_melts(&self, pos: Vec3d) -> Option<bool> {
        self.full_preparation.originals.snow_golem_melts(
            self.chunks,
            BlockPos {
                x: pos.x.floor() as i32,
                y: pos.y.floor() as i32,
                z: pos.z.floor() as i32,
            },
        )
    }

    /// `Block.getFriction` (`Blocks.java` `.friction(`).
    pub(crate) fn block_friction(&self, pos: BlockPos) -> f32 {
        bbb_protocol::block_speed_factors::vanilla_block_state_friction(self.block_state(pos))
            .unwrap_or(0.6)
    }

    /// `Block.getSpeedFactor` (`Blocks.java` `.speedFactor(`).
    pub(crate) fn block_speed_factor(&self, pos: BlockPos) -> f32 {
        bbb_protocol::block_speed_factors::vanilla_block_state_speed_factor(self.block_state(pos))
            .unwrap_or(1.0)
    }

    pub(crate) fn clip_fall_resetting(&self, from: Vec3d, to: Vec3d) -> Option<bool> {
        crate::fall_clip::fall_resetting_clip_with(
            from,
            to,
            crate::fall_clip::FallResettingContext::Stored,
            self.chunks.fluid_shape_cache(),
            |pos| Some(self.block_state(pos)),
            |registry, tag, resource| self.registry_tag_membership(registry, tag, resource),
        )
    }

    pub(crate) fn is_moving_piston_at(&self, pos: BlockPos) -> Option<bool> {
        bbb_protocol::block_state_types::vanilla_block_state_block_type_id(self.block_state(pos))
            .map(|kind| kind == bbb_protocol::block_types::VANILLA_BLOCK_MOVING_PISTON_ID)
    }

    pub(crate) fn player_gaze_shape(
        &self,
        player: &NearbyPlayer,
        pos: BlockPos,
        visual: bool,
    ) -> Option<Vec<Aabb>> {
        let state = self.block_state_checked(pos)?;
        if visual {
            if let Some(shape) =
                bbb_server_types::geometry::positioned_shapes::visual_boxes(state, pos)
            {
                return Some(shape);
            }
        }
        crate::entity::player_collision::collision_boxes(
            state,
            pos,
            player.position.y,
            player.discrete,
            player.collision,
            |registry, tag, element| self.registry_tag_membership(registry, tag, element),
            || self.block_collision_boxes_empty(pos),
        )
    }

    pub(crate) fn player_may_build(&self, uuid: uuid::Uuid) -> bool {
        self.player(uuid).is_some_and(|player| player.may_build)
    }

    pub(crate) fn handle_portal(&mut self, entity: &mut Entity) {
        crate::entity::process_portal_cooldown(entity);
        let Some(context) = self.portal_context else {
            return;
        };
        if !matches!(
            entity.portal_process().map(|portal| portal.kind()),
            Some(
                crate::entity::portal::PortalKind::Nether
                    | crate::entity::portal::PortalKind::End
                    | crate::entity::portal::PortalKind::EndGateway
            )
        ) {
            return;
        }
        let request = entity.process_portal(0, context.0 && entity.is_alive(), context.1);
        if context.2 {
            self.portal_request = request;
        }
    }

    pub(crate) fn record_portal_contact(
        &mut self,
        entity: &mut Entity,
        kind: crate::entity::portal::PortalKind,
        pos: BlockPos,
    ) {
        let (allowed, delay) = self.portal_context.map_or(
            (
                !matches!(entity.data(), crate::entity::EntityData::FishingBobber(_)),
                300,
            ),
            |(allowed, delay, _)| (allowed, delay),
        );
        if allowed
            && entity.is_alive()
            && (kind != crate::entity::portal::PortalKind::EndGateway
                || self.trigger_end_gateway_cooldown(pos))
        {
            entity.set_as_inside_portal(kind, pos, delay);
        }
    }

    pub(crate) fn portal_transition_requested(&self) -> bool {
        self.portal_request.is_some()
    }
}

impl<'a, 'b> LevelCtx<'a, 'b> {
    pub(crate) fn players_in_box(
        &self,
        area: bbb_server_types::geometry::Aabb,
    ) -> impl Iterator<Item = NearbyPlayer> + 'a + use<'a> {
        let players: &'a [NearbyPlayer] = self.players;
        players
            .iter()
            .filter(move |player| player.bounding_box().intersects(area))
            .copied()
    }

    pub(crate) fn detached_bodies(
        &self,
    ) -> Option<&'a crate::entity::store::DetachedEntityBodies<'a>> {
        self.detached_entities
    }
}

#[cfg(test)]
mod cross_level_tests {
    use super::*;
    use crate::entity::{EntityData, GeneralLivingData, RemovalReason};
    use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_COW_ID;
    use bbb_server_types::save::Java26BuiltinDimension::{Nether, Overworld};

    #[test]
    fn remote_lookup_distinguishes_hidden_detached_and_removed_entities() {
        let mut world = crate::world::ServerWorld::new(Default::default()).unwrap();
        let (id, uuid) = world.spawn_stored_entity_for_test(
            Nether,
            VANILLA_ENTITY_TYPE_COW_ID,
            Vec3d {
                x: 0.5,
                y: 80.0,
                z: 0.5,
            },
            EntityData::GeneralLiving(GeneralLivingData::new(VANILLA_ENTITY_TYPE_COW_ID).unwrap()),
        );
        world.track_stored_chunk_for_test(Nether, (0, 0));
        let check = |world: &mut crate::world::ServerWorld, present, resolved| {
            world.with_level_for_test(Overworld, |level| {
                assert_eq!(level.entity_exists_in_other_dimension(uuid), present);
                assert_eq!(level.entity_in_other_dimension(uuid).is_some(), resolved);
            });
        };
        check(&mut world, Some(true), true);
        let body = world.level_mut(Nether).entities.detach(id).unwrap();
        check(&mut world, None, false);
        world.level_mut(Nether).entities.attach(body);
        check(&mut world, Some(true), true);
        world
            .level_mut(Nether)
            .entities
            .apply_chunk_visibility((0, 0), crate::entity::store::EntityVisibility::Hidden);
        check(&mut world, Some(false), false);
        world.track_stored_chunk_for_test(Nether, (0, 0));
        world
            .level_mut(Nether)
            .entities
            .remove(id, RemovalReason::UnloadedToChunk)
            .unwrap();
        check(&mut world, Some(false), false);
    }
}
