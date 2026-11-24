//! Live block queries, mutations and scheduled behavior inputs.

use super::*;
#[cfg(test)]
use crate::block_behavior::chiseled_bookshelf as chiseled_bookshelf_block;
use crate::block_behavior::state_is_block;
use crate::fluid::HORIZONTAL;
use crate::level_event::LEVEL_EVENT_PARTICLES_DESTROY_BLOCK;
use crate::redstone::{SignalContext, WireSignal};
use crate::registry_tags::RegistryTagSnapshot;
use crate::world::simulation_level::LevelCtx;
use bbb_facts::environment_attribute_types::VanillaEnvironmentScalarValue;
use bbb_protocol::block_types::VANILLA_BLOCK_COMPARATOR_ID;
use bbb_protocol::registry_data_types::VanillaFluidIdentity;

impl LevelCtx<'_, '_> {
    pub(crate) fn chunks(&self) -> &ChunkStore {
        self.chunks
    }

    pub(crate) fn poi_mut(&mut self) -> &mut crate::chunk::poi::PoiManager {
        self.chunks.poi_mut()
    }

    pub(crate) fn dimension_key(&self) -> &bbb_server_types::save::Java26LevelKey {
        self.dimension
    }

    /// The level's `VILLAGER_ACTIVITY`/`BABY_VILLAGER_ACTIVITY` environment
    /// value now, which a villager brain's schedule follows.
    pub(crate) fn villager_schedule_activity(
        &self,
        baby: bool,
    ) -> Option<crate::entity::ai::Activity> {
        self.structure_services?.villager_activity[usize::from(baby)]
    }

    pub(crate) fn overworld_day(&self) -> i64 {
        self.structure_services
            .map_or(0, |services| services.overworld_day)
    }

    /// The level's datapack generation tags.
    pub(crate) fn tags(&self) -> &RegistryTagSnapshot {
        self.chunks
            .registry_tags()
            .unwrap_or_else(|| RegistryTagSnapshot::builtin())
    }

    pub(crate) fn registry_tag_membership(
        &self,
        registry_id: &str,
        tag_id: &str,
        element_id: &str,
    ) -> bool {
        self.tags().contains(registry_id, tag_id, element_id)
    }

    pub(crate) fn block_state_in_tag(&self, block_state_id: i32, tag_id: &str) -> bool {
        self.tags().block_state_in(tag_id, block_state_id)
    }

    pub(crate) fn place_copper_golem_statue(
        &mut self,
        pos: BlockPos,
        state: i32,
        custom_name: Option<&bbb_protocol::component::RichTextComponent>,
    ) -> bool {
        if !self.set_block(pos, state, SET_BLOCK_UPDATE_ALL, UPDATE_LIMIT) {
            return false;
        }
        let created = self
            .chunks
            .block_entities_mut()
            .get_mut(pos)
            .is_some_and(|entity| entity.create_copper_golem_statue(custom_name));
        if created {
            self.chunks.block_entities_mut().set_changed(pos);
        }
        created
    }

    pub(crate) fn block_entity_present(&self, pos: BlockPos) -> bool {
        self.chunks.block_entities().get(pos).is_some()
    }

    fn occupancy_bodies(&self) -> Option<crate::entity::store::EntityBodyReadView<'_>> {
        let context = self.structure_entities.as_ref()?;
        Some(
            crate::entity::store::EntityBodyReadView::new(
                context.store,
                context.current,
                context.ancestors,
            )
            .with_ancestors(self.detached_entities),
        )
    }

    pub(crate) fn entity_count_in_box(&self, area: Aabb, query: BlockEntityQuery) -> usize {
        if let Some(bodies) = self.occupancy_bodies() {
            occupancy::count_in_box(bodies, self.players, area, query)
        } else {
            match query {
                BlockEntityQuery::BlockTriggerAll | BlockEntityQuery::BlockTriggerLiving => self
                    .players
                    .iter()
                    .filter(|player| !player.spectator && player.bounding_box().intersects(area))
                    .count(),
                BlockEntityQuery::Minecart => 0,
            }
        }
    }

    pub(crate) fn cat_sitting_on_chest(&self, pos: BlockPos) -> bool {
        let Some(context) = self.structure_entities.as_ref() else {
            return false;
        };
        let bodies = crate::entity::store::EntityBodyReadView::new(
            context.store,
            context.current,
            context.ancestors,
        )
        .with_ancestors(self.detached_entities);
        crate::block_behavior::chest::is_cat_sitting_on_chest(pos, bodies)
    }

    pub(crate) fn first_arrow_in_box(&self, area: Aabb) -> Option<Option<i32>> {
        let context = self.structure_entities.as_ref()?;
        let bodies = crate::entity::store::EntityBodyReadView::new(
            context.store,
            context.current,
            context.ancestors,
        );
        Some(bodies.ids_in_box_sections(area).into_iter().find(|id| {
            bodies.get(*id).is_some_and(|entity| {
                entity.is_alive()
                    && crate::entity::abstract_arrow::state(entity.data()).is_some()
                    && entity.bounding_box().intersects(area)
            })
        }))
    }

    pub(crate) fn piston_moving_block_entity(
        &self,
        pos: BlockPos,
    ) -> Option<crate::block_entity::piston::PistonMovingSnapshot> {
        self.chunks
            .block_entities()
            .get(pos)
            .and_then(BlockEntity::piston_snapshot)
    }

    pub(crate) fn piston_is_handling_tick(&self) -> bool {
        self.block_events.is_handling_tick()
            || self.block_ticks.is_handling_tick()
            || self.fluid_ticks.is_handling_tick()
    }

    pub(crate) fn piston_apply_movement(&mut self, plan: &PistonMovementPlan) {
        // Entity pushes belong to the moving block entity, not the block plan.
        let moving_state = moving_piston_state(plan.direction, false);
        let source_moving_state = moving_piston_state(plan.direction, plan.sticky);
        let head_state = piston_head_state(plan.direction, plan.sticky);
        let push_direction = if plan.extending {
            plan.direction
        } else {
            bbb_server_types::directions::opposite(plan.direction)
        };
        let destinations = plan
            .moved
            .iter()
            .map(|(pos, _)| bbb_server_types::directions::relative(*pos, push_direction))
            .collect::<Vec<_>>();
        let sources = source_positions(plan);
        let arm = bbb_server_types::directions::relative(plan.piston_pos, plan.direction);

        let moving_entity =
            |pos: BlockPos, state: i32, moved: i32, extending: bool, source: bool| {
                let mut entity = new_block_entity(pos, state)?;
                entity
                    .configure_piston(moved, plan.direction, extending, source)
                    .then_some(entity)
            };

        let finalized_states = plan
            .finalize
            .iter()
            .map(|(pos, snapshot)| {
                let state = if snapshot.is_source {
                    self.chunks.air_state_id()
                } else {
                    update_from_neighbour_shapes(self, snapshot.moved_state, *pos)
                };
                (*pos, state, snapshot.movement_direction())
            })
            .collect::<Vec<_>>();

        for (pos, state, finalized_push_direction) in finalized_states {
            self.chunks.block_entities_mut().remove(pos);
            let _ = LevelCtx::set_block(self, pos, state, 3, UPDATE_LIMIT);
            if let Some(block) = vanilla_block_state_block_type_id(state) {
                let orientation =
                    crate::block_behavior::redstone_wire::experimental::initial_orientation(
                        self,
                        Some(finalized_push_direction),
                        None,
                    );
                self.dispatch_neighbors(|neighbours| {
                    neighbours.neighbor_changed_with_orientation(pos, block, orientation)
                });
            }
        }

        if !plan.extending {
            let _ = LevelCtx::set_block(
                self,
                plan.piston_pos,
                source_moving_state,
                276,
                UPDATE_LIMIT,
            );
            // `PistonBaseBlock.java:186-191`: install the configured BE before neighbor/shape updates.
            if let Some(entity) = moving_entity(
                plan.piston_pos,
                source_moving_state,
                plan.source_base_state.unwrap_or(plan.piston_state),
                false,
                true,
            ) {
                self.chunks.block_entities_mut().add_and_register(entity);
            }
            if let Some(block) = vanilla_block_state_block_type_id(source_moving_state) {
                self.dispatch_neighbors(|neighbours| {
                    neighbours.update_neighbors_at(plan.piston_pos, block)
                });
            }
            // `PistonBaseBlock.java:191`: flag 276 skips the shape pass, so it runs here.
            self.update_neighbor_shapes(plan.piston_pos, source_moving_state, 2, UPDATE_LIMIT);
        }

        for (pos, state) in plan.broken.iter().rev() {
            self.drop_resources_at(*pos, *state);
            let air = self.chunks.air_state_id();
            let _ = LevelCtx::set_block(self, *pos, air, 18, UPDATE_LIMIT);
            self.event_sink
                .game_event(VanillaGameEvent::BLOCK_DESTROY, *pos, Some(*state));
        }
        for (pos, state) in plan.moved.iter().rev() {
            let destination = bbb_server_types::directions::relative(*pos, push_direction);
            let _ = LevelCtx::set_block(self, destination, moving_state, 324, UPDATE_LIMIT);
            if let Some(entity) =
                moving_entity(destination, moving_state, *state, plan.extending, false)
            {
                self.chunks.block_entities_mut().add_and_register(entity);
            }
        }
        if plan.extending {
            let _ = LevelCtx::set_block(self, arm, source_moving_state, 324, UPDATE_LIMIT);
            if let Some(entity) = moving_entity(arm, source_moving_state, head_state, true, true) {
                self.chunks.block_entities_mut().add_and_register(entity);
            }
        }

        for source in sources {
            if destinations.contains(&source) || (plan.extending && source == arm) {
                continue;
            }
            let air = self.chunks.air_state_id();
            let _ = LevelCtx::set_block(self, source, air, 82, UPDATE_LIMIT);
        }
        if !plan.extending && plan.clear_arm {
            let air = self.chunks.air_state_id();
            let _ = LevelCtx::set_block(self, arm, air, 67, UPDATE_LIMIT);
        }

        for (pos, state) in &plan.moved {
            if destinations.contains(pos) || (plan.extending && *pos == arm) {
                continue;
            }
            let air = self.chunks.air_state_id();
            block_update_indirect_neighbour_shapes(self, *state, *pos, 2, UPDATE_LIMIT);
            self.update_neighbor_shapes(*pos, air, 2, UPDATE_LIMIT);
            block_update_indirect_neighbour_shapes(self, air, *pos, 2, UPDATE_LIMIT);
        }

        // `PistonBaseBlock.java:350,361,365,369`: propagate push-direction orientation.
        let orientation = crate::block_behavior::redstone_wire::experimental::initial_orientation(
            self,
            Some(push_direction),
            None,
        );
        for (pos, state) in plan.broken.iter().rev() {
            block_affect_neighbors_after_removal(self, *pos, *state, false);
            block_update_indirect_neighbour_shapes(self, *state, *pos, 2, UPDATE_LIMIT);
            if let Some(block) = vanilla_block_state_block_type_id(*state) {
                self.dispatch_neighbors(|neighbours| {
                    neighbours.update_neighbors_at_with_orientation(*pos, block, None, orientation)
                });
            }
        }
        for (pos, state) in plan.moved.iter().rev() {
            if let Some(block) = vanilla_block_state_block_type_id(*state) {
                self.dispatch_neighbors(|neighbours| {
                    neighbours.update_neighbors_at_with_orientation(*pos, block, None, orientation)
                });
            }
        }
        if plan.extending {
            if let Some(extended_state) =
                crate::fluid::with_property(plan.piston_state, "extended", "true")
            {
                let _ =
                    LevelCtx::set_block(self, plan.piston_pos, extended_state, 67, UPDATE_LIMIT);
            }
            self.dispatch_neighbors(|neighbours| {
                neighbours.update_neighbors_at_with_orientation(
                    arm,
                    VANILLA_BLOCK_PISTON_HEAD_ID,
                    None,
                    orientation,
                )
            });
        }
    }

    pub(crate) fn drop_resources(&mut self, pos: BlockPos, block_state_id: i32) {
        self.drop_resources_at(pos, block_state_id);
    }

    pub(crate) fn beehive_emergency_release(&mut self, pos: BlockPos, state: i32) {
        self.event_sink.beehive_emergency_release(pos, state);
    }

    pub(crate) fn level_event(&mut self, event_type: i32, pos: BlockPos, data: i32) {
        self.event_sink.level_event(event_type, pos, data);
    }

    pub(crate) fn play_block_sound(&mut self, pos: BlockPos, sound_resource_id: &str) {
        self.event_sink.play_block_sound(pos, sound_resource_id);
    }

    pub(crate) fn play_block_sound_with_volume_pitch(
        &mut self,
        pos: BlockPos,
        sound_resource_id: &str,
        volume: f32,
        pitch: f32,
    ) {
        self.event_sink
            .play_block_sound_with_volume_pitch(pos, sound_resource_id, volume, pitch);
    }

    pub(crate) fn play_block_sound_except(
        &mut self,
        pos: BlockPos,
        sound_resource_id: &str,
        volume: f32,
        pitch: f32,
        except_entity_id: Option<i32>,
    ) {
        self.event_sink.play_block_sound_except(
            pos,
            sound_resource_id,
            volume,
            pitch,
            except_entity_id,
        );
    }

    /// `EnvironmentAttributes.TURTLE_EGG_HATCH_CHANCE` at `position`.
    pub(crate) fn turtle_egg_hatch_chance(&self, position: BlockPos) -> f32 {
        self.full_preparation
            .originals
            .turtle_egg_hatch_chance(self.chunks, position)
            .unwrap_or(0.002)
    }

    /// `EnvironmentAttributes.CREAKING_ACTIVE` at `position`.
    pub(crate) fn creaking_active(&self, position: BlockPos) -> bool {
        self.full_preparation
            .originals
            .creaking_active(self.chunks, position)
            .unwrap_or(false)
    }

    pub(crate) fn eyeblossom_open(
        &self,
        position: BlockPos,
    ) -> Option<bbb_server_types::environment::TriState> {
        self.full_preparation
            .originals
            .eyeblossom_open(self.chunks, position)
    }

    pub(crate) fn send_trail_particle(
        &mut self,
        from: [f64; 3],
        to: [f64; 3],
        color: i32,
        duration: i32,
    ) {
        self.event_sink
            .send_trail_particle(from, to, color, duration);
    }

    pub(crate) fn block_game_event(
        &mut self,
        game_event: VanillaGameEvent,
        pos: BlockPos,
        affected_state: Option<i32>,
    ) {
        self.event_sink.game_event(game_event, pos, affected_state);
    }

    pub(crate) fn game_event_from_entity(
        &mut self,
        game_event: VanillaGameEvent,
        pos: BlockPos,
        source_entity_id: Option<i32>,
        affected_state: Option<i32>,
    ) {
        self.event_sink
            .game_event_with_source(game_event, pos, source_entity_id, affected_state);
    }

    pub(crate) fn send_block_updated_with_flags(
        &mut self,
        pos: BlockPos,
        old_state: i32,
        new_state: i32,
        flags: i32,
    ) {
        if flags & SET_BLOCK_UPDATE_CLIENTS != 0 {
            self.send_block_updated(pos, old_state, new_state);
        }
    }

    pub(crate) fn neighbor_changed(&mut self, pos: BlockPos, source_block_type_id: i32) {
        self.dispatch_neighbors(|neighbours| {
            neighbours.neighbor_changed(pos, source_block_type_id)
        });
    }

    pub(crate) fn neighbor_changed_with_orientation(
        &mut self,
        pos: BlockPos,
        source_block_type_id: i32,
        orientation: Option<
            crate::block_behavior::redstone_wire::experimental::RedstoneOrientation,
        >,
    ) {
        if orientation.is_none() {
            self.neighbor_changed(pos, source_block_type_id);
            return;
        }
        self.dispatch_neighbors(|neighbours| {
            neighbours.neighbor_changed_with_orientation(pos, source_block_type_id, orientation)
        });
    }

    pub(crate) fn neighbor_changed_with_state(
        &mut self,
        state: i32,
        pos: BlockPos,
        source_block_type_id: i32,
    ) {
        self.neighbor_changed_with_state_and_orientation(state, pos, source_block_type_id, None);
    }

    pub(crate) fn neighbor_changed_with_state_and_orientation(
        &mut self,
        state: i32,
        pos: BlockPos,
        source_block_type_id: i32,
        orientation: Option<
            crate::block_behavior::redstone_wire::experimental::RedstoneOrientation,
        >,
    ) {
        self.dispatch_neighbors(|neighbours| {
            neighbours.neighbor_changed_with_state_and_orientation(
                state,
                pos,
                source_block_type_id,
                orientation,
            )
        });
    }

    pub(crate) fn update_neighbors_at(&mut self, pos: BlockPos, source_block_type_id: i32) {
        let orientation = crate::block_behavior::redstone_wire::experimental::initial_orientation(
            self, None, None,
        );
        self.dispatch_neighbors(|neighbours| {
            neighbours.update_neighbors_at_with_orientation(
                pos,
                source_block_type_id,
                None,
                orientation,
            )
        });
    }

    pub(crate) fn update_neighbors_at_except_from_facing(
        &mut self,
        pos: BlockPos,
        source_block_type_id: i32,
        skip_direction: Direction,
    ) {
        let orientation = crate::block_behavior::redstone_wire::experimental::initial_orientation(
            self,
            Some(bbb_server_types::directions::opposite(skip_direction)),
            None,
        );
        self.update_neighbors_at_except_from_facing_with_orientation(
            pos,
            source_block_type_id,
            skip_direction,
            orientation,
        );
    }

    pub(crate) fn update_neighbors_at_except_from_facing_with_orientation(
        &mut self,
        pos: BlockPos,
        source_block_type_id: i32,
        skip_direction: Direction,
        orientation: Option<
            crate::block_behavior::redstone_wire::experimental::RedstoneOrientation,
        >,
    ) {
        self.dispatch_neighbors(|neighbours| {
            neighbours.update_neighbors_at_with_orientation(
                pos,
                source_block_type_id,
                Some(skip_direction),
                orientation,
            )
        });
    }

    pub(crate) fn neighbor_shape_changed(
        &mut self,
        direction: Direction,
        pos: BlockPos,
        neighbor_pos: BlockPos,
        neighbor_state: i32,
        update_flags: i32,
        update_limit: i32,
    ) {
        self.dispatch_neighbors(|neighbours| {
            neighbours.shape_update(ShapeNeighborUpdate {
                direction,
                pos,
                neighbor_pos,
                neighbor_state,
                update_flags,
                update_limit,
            })
        });
    }

    pub(crate) fn get_max_y(&self) -> i32 {
        self.max_y()
    }

    pub(crate) fn get_min_y(&self) -> i32 {
        self.chunks.min_y()
    }

    pub(crate) fn in_nether_portal_dimension(&self) -> bool {
        self.nether_portal_dimension
    }

    pub(crate) fn spread_vines_enabled(&self) -> bool {
        self.game_rules.spread_vines
    }

    pub(crate) fn game_time(&self) -> i64 {
        self.game_time
    }

    pub(crate) fn recent_torch_toggles(&mut self) -> &mut Vec<RedstoneTorchToggle> {
        self.recent_torch_toggles
    }

    pub(crate) fn has_scheduled_tick(&self, pos: BlockPos, block: BlockTickType) -> bool {
        self.chunks.has_scheduled_tick(pos, block)
    }

    pub(crate) fn will_tick_this_tick(&mut self, pos: BlockPos, block: BlockTickType) -> bool {
        self.block_ticks.will_tick_this_tick(pos, block)
    }

    pub(crate) fn block_event(
        &mut self,
        pos: BlockPos,
        block_type_id: i32,
        param_a: i32,
        param_b: i32,
    ) {
        self.block_events
            .block_event(BlockEventData::new(pos, block_type_id, param_a, param_b));
    }

    pub(crate) fn brightness(&self, layer: LightLayer, pos: BlockPos) -> i32 {
        simulation_brightness(self.chunks, layer, pos)
    }

    pub(crate) fn raw_brightness(&self, pos: BlockPos, sky_darkening: i32) -> i32 {
        placement_raw_brightness(self.chunks, pos, sky_darkening)
    }

    pub(crate) fn sky_darken(&self) -> i32 {
        self.sky_darkening
    }

    pub(crate) fn is_bright_outside(&self) -> Option<bool> {
        self.full_preparation
            .originals
            .is_bright_outside(self.sky_darkening)
    }

    pub(crate) fn is_dark_outside(&self) -> Option<bool> {
        self.full_preparation
            .originals
            .is_dark_outside(self.sky_darkening)
    }

    pub(crate) fn sun_angle_degrees(&self, pos: BlockPos) -> Option<f32> {
        match self
            .block_environment_scalar(crate::world::environment_attributes::ATTR_SUN_ANGLE, pos)?
        {
            VanillaEnvironmentScalarValue::FloatBits(bits) => Some(f32::from_bits(bits)),
            _ => None,
        }
    }

    pub(crate) fn increased_fire_burnout(&self, pos: BlockPos) -> Option<bool> {
        match self.block_environment_scalar(
            crate::world::environment_attributes::ATTR_INCREASED_FIRE_BURNOUT,
            pos,
        )? {
            VanillaEnvironmentScalarValue::Boolean(value) => Some(value),
            _ => None,
        }
    }

    pub(crate) fn nether_portal_spawns_piglins(&self, pos: BlockPos) -> Option<bool> {
        match self.block_environment_scalar(
            crate::world::environment_attributes::ATTR_NETHER_PORTAL_SPAWNS_PIGLIN,
            pos,
        )? {
            VanillaEnvironmentScalarValue::Boolean(value) => Some(value),
            _ => None,
        }
    }

    fn block_environment_scalar(
        &self,
        attribute_index: usize,
        pos: BlockPos,
    ) -> Option<VanillaEnvironmentScalarValue> {
        self.loot_level_facts
            .environment_attributes
            .as_ref()?
            .block_scalar(attribute_index, self.chunks, pos)
    }

    /// `Entity.isInClouds` (`Entity.java:1530-1542`).
    pub(crate) fn is_in_clouds(&self, pos: Vec3d, bb_height: f32) -> bool {
        let cloud_bottom = if let Some(environment) = self.loot_level_facts.environment_attributes {
            static CLOUD_COLOR: std::sync::OnceLock<
                &'static bbb_facts::environment_attribute_types::VanillaEnvironmentAttribute,
            > = std::sync::OnceLock::new();
            static CLOUD_HEIGHT: std::sync::OnceLock<
                &'static bbb_facts::environment_attribute_types::VanillaEnvironmentAttribute,
            > = std::sync::OnceLock::new();
            let origin = Some(crate::loot_runtime::LootOrigin {
                x: pos.x,
                y: pos.y,
                z: pos.z,
            });
            let color_attr = *CLOUD_COLOR.get_or_init(|| {
                bbb_facts::environment_attribute_types::vanilla_environment_attribute(
                    "minecraft:visual/cloud_color",
                )
                .expect("built-in visual/cloud_color attribute")
            });
            let Some(crate::loot_runtime::LootEnvironmentAttributeValue::Color(argb)) =
                environment.sample(color_attr, self.chunks, origin)
            else {
                return false;
            };
            if (argb >> 24) == 0 {
                return false;
            }
            let height_attr = *CLOUD_HEIGHT.get_or_init(|| {
                bbb_facts::environment_attribute_types::vanilla_environment_attribute(
                    "minecraft:visual/cloud_height",
                )
                .expect("built-in visual/cloud_height attribute")
            });
            let Some(crate::loot_runtime::LootEnvironmentAttributeValue::Scalar(
                VanillaEnvironmentScalarValue::FloatBits(bits),
            )) = environment.sample(height_attr, self.chunks, origin)
            else {
                return false;
            };
            f32::from_bits(bits)
        } else {
            if self.dimension_key().builtin()
                != Some(bbb_server_types::save::Java26BuiltinDimension::Overworld)
            {
                return false;
            }
            192.33_f32
        };
        pos.y + f64::from(bb_height) >= f64::from(cloud_bottom)
            && pos.y <= f64::from(cloud_bottom + 4.0_f32)
    }

    pub(crate) fn light_block_into(
        &self,
        from_state: i32,
        to_state: i32,
        direction: Direction,
        simple_opacity: i32,
    ) -> i32 {
        vanilla_light_block_into(from_state, to_state, direction, simple_opacity)
    }

    pub(crate) fn bees_stay_in_hive(&self, pos: BlockPos) -> bool {
        self.structure_services
            .as_ref()
            .and_then(|services| services.bees_stay_in_hive.as_ref())
            .and_then(|value| {
                value.sample(|| {
                    self.chunks
                        .biome_id_at_quart_or_source(pos.x >> 2, pos.y >> 2, pos.z >> 2)
                })
            })
            .unwrap_or(false)
    }

    pub(crate) fn is_raining(&self) -> bool {
        self.weather.is_raining()
    }

    pub(crate) fn is_raining_at(&self, pos: BlockPos) -> bool {
        self.weather.is_raining_at(self.chunks, pos)
    }

    /// `Level.precipitationAt(pos) != NONE`: rain or snow falls on `pos`.
    pub(crate) fn has_precipitation_at(&self, pos: BlockPos) -> bool {
        self.weather.precipitation_at(self.chunks, pos)
            != crate::world::weather::Precipitation::None
    }

    pub(crate) fn biome_resource_id(&self, pos: BlockPos) -> Option<&str> {
        use bbb_server_worldgen::noise::biome::{biome_zoom_seed, fuzzy_biome_quart_position};
        let (x, y, z) = fuzzy_biome_quart_position(
            biome_zoom_seed(self.loot_level_facts.world_seed),
            pos.x,
            pos.y,
            pos.z,
        );
        let id = self.chunks.biome_id_at_quart(x, y, z)?;
        self.loot_level_facts
            .datapacks?
            .registries
            .registry("minecraft:worldgen/biome")?
            .elements
            .get(usize::try_from(id).ok()?)
            .map(|element| element.id.as_str())
    }

    /// `EnvironmentAttributes.WATER_EVAPORATES` at `pos`.
    pub(crate) fn water_evaporates(&self, pos: BlockPos) -> bool {
        self.structure_services
            .as_ref()
            .and_then(|services| services.water_evaporates.as_ref())
            .and_then(|value| {
                value.sample(|| {
                    self.chunks
                        .biome_id_at_quart_or_source(pos.x >> 2, pos.y >> 2, pos.z >> 2)
                })
            })
            .unwrap_or(false)
    }

    pub(crate) fn command_block(
        &self,
        pos: BlockPos,
    ) -> Option<&command_block::CommandBlockEntity> {
        self.chunks.block_entities().get(pos)?.command_block()
    }

    pub(crate) fn command_block_mut(
        &mut self,
        pos: BlockPos,
    ) -> Option<&mut command_block::CommandBlockEntity> {
        self.chunks.block_entities_mut().set_changed(pos);
        self.chunks
            .block_entities_mut()
            .get_mut(pos)?
            .command_block_mut()
    }

    pub(crate) fn crafter_mut(
        &mut self,
        pos: BlockPos,
    ) -> Option<&mut crate::block_entity::crafter::CrafterBlockEntity> {
        self.chunks.block_entities_mut().get_mut(pos)?.crafter_mut()
    }

    pub(crate) fn block_entity_output_signal(&self, pos: BlockPos) -> i32 {
        self.chunks
            .block_entities()
            .get(pos)
            .and_then(crate::block_entity::BlockEntity::comparator)
            .map_or(
                0,
                crate::block_entity::comparator::ComparatorBlockEntity::output_signal,
            )
    }

    pub(crate) fn comparator_block_entity_output_signal(&self, pos: BlockPos) -> Option<i32> {
        self.chunks
            .block_entities()
            .get(pos)
            .and_then(crate::block_entity::BlockEntity::comparator)
            .map(crate::block_entity::comparator::ComparatorBlockEntity::output_signal)
    }

    pub(crate) fn creaking_heart_analog(
        &self,
        pos: BlockPos,
    ) -> Option<crate::block_entity::creaking_heart::CreakingHeartAnalog> {
        self.chunks
            .block_entities()
            .get(pos)
            .and_then(crate::block_entity::BlockEntity::creaking_heart)
            .map(crate::block_entity::creaking_heart::CreakingHeartBlockEntity::analog)
    }

    pub(crate) fn brushable_check_reset(
        &mut self,
        pos: BlockPos,
    ) -> Option<crate::block_entity::brushable::BrushReset> {
        let game_time = self.game_time;
        self.chunks
            .block_entities_mut()
            .get_mut(pos)
            .and_then(crate::block_entity::BlockEntity::brushable_mut)
            .map(|brushable| brushable.check_reset(game_time))
    }

    pub(crate) fn detector_rail_minecarts(
        &self,
        area: Aabb,
    ) -> crate::block_behavior::level::DetectorRailMinecarts {
        self.occupancy_bodies()
            .map(|bodies| occupancy::detector_rail_minecarts(bodies, area))
            .unwrap_or_default()
    }

    pub(crate) fn comparator_item_frame_output(
        &self,
        pos: BlockPos,
        facing: Direction,
    ) -> Option<i32> {
        self.occupancy_bodies()
            .and_then(|bodies| occupancy::comparator_item_frame_output(bodies, pos, facing))
    }

    pub(crate) fn jukebox_playing(&self, pos: BlockPos) -> bool {
        self.chunks
            .block_entities()
            .get(pos)
            .and_then(crate::block_entity::BlockEntity::jukebox)
            .is_some_and(|jukebox| jukebox.song_player().is_playing())
    }

    /// Whether a shulker box lid has left the closed full-block shape.
    /// `BlockState.isRedstoneConductor(level, pos)` (`BlockBehaviour.java:1003`):
    /// a shulker box whose lid has moved is not a full collision block.
    pub(crate) fn is_redstone_conductor_at(&self, pos: BlockPos) -> bool {
        let state = self.block_state(pos);
        crate::redstone::is_redstone_conductor_with_registry(state, self.chunks.content_registry())
            && !(bbb_server_types::block_queries::is_shulker_box(state)
                && self.shulker_box_open(pos))
    }

    pub(crate) fn shulker_box_open(&self, pos: BlockPos) -> bool {
        self.chunks
            .block_entities()
            .get(pos)
            .and_then(crate::block_entity::BlockEntity::shulker_box)
            .is_some_and(crate::block_entity::shulker_box::ShulkerBoxBlockEntity::lid_moved)
    }

    pub(crate) fn chest_open_count(&self, pos: BlockPos) -> i32 {
        self.chunks
            .block_entities()
            .get(pos)
            .and_then(crate::block_entity::BlockEntity::chest)
            .map_or(0, crate::block_entity::chest::ChestBlockEntity::open_count)
    }

    pub(crate) fn container_openers_count(&self, pos: BlockPos) -> Option<i32> {
        let block_entity = self.chunks.block_entities().get(pos)?;
        if let Some(chest) = crate::block_entity::BlockEntity::chest(block_entity) {
            return Some(chest.open_count());
        }
        if let Some(barrel) = crate::block_entity::BlockEntity::barrel(block_entity) {
            return Some(barrel.open_count());
        }
        crate::block_entity::BlockEntity::ender_chest(block_entity)
            .map(crate::block_entity::ender_chest::EnderChestBlockEntity::open_count)
    }

    pub(crate) fn container_openers_range(&self, pos: BlockPos) -> f64 {
        let Some(entity) = self.chunks.block_entities().get(pos) else {
            return 0.0;
        };
        if let Some(chest) = entity.chest() {
            return chest.max_interaction_range();
        }
        if let Some(barrel) = entity.barrel() {
            return barrel.max_interaction_range();
        }
        entity
            .ender_chest()
            .map_or(0.0, |chest| chest.max_interaction_range())
    }

    pub(crate) fn set_container_openers_range(&mut self, pos: BlockPos, range: f64) {
        let Some(entity) = self.chunks.block_entities_mut().get_mut(pos) else {
            return;
        };
        if let Some(chest) = entity.chest_mut() {
            chest.set_max_interaction_range(range);
        } else if let Some(barrel) = entity.barrel_mut() {
            barrel.set_max_interaction_range(range);
        } else if let Some(chest) = entity.ender_chest_mut() {
            chest.set_max_interaction_range(range);
        }
    }

    // ChestBlockEntity.java:103-117: double chests sound at the joined lid's centre.
    pub(crate) fn play_chest_sound(&mut self, pos: BlockPos, sound: &str) {
        let state = self.block_state(pos);
        if crate::block_behavior::chest::chest_type_is_left(state) {
            return;
        }
        let mut position = Vec3d {
            x: f64::from(pos.x) + 0.5,
            y: f64::from(pos.y) + 0.5,
            z: f64::from(pos.z) + 0.5,
        };
        if crate::block_behavior::chest::chest_type_is_right(state) {
            if let Some(direction) = crate::block_behavior::chest::connected_direction(state) {
                let step = bbb_server_types::directions::relative(
                    BlockPos { x: 0, y: 0, z: 0 },
                    direction,
                );
                position.x += f64::from(step.x) * 0.5;
                position.z += f64::from(step.z) * 0.5;
            }
        }
        let pitch = self.level_random().next_float() * 0.1 + 0.9;
        if let Some(context) = self.structure_entities.as_mut() {
            context
                .store
                .play_entity_sound(crate::entity::store::PendingEntitySound {
                    position,
                    resource_id: sound.into(),
                    source: bbb_protocol::packets::SoundSource::Blocks,
                    volume: 0.5,
                    pitch,
                });
        } else {
            self.play_block_sound_with_volume_pitch(pos, sound, 0.5, pitch);
        }
    }

    pub(crate) fn container_openers_users(&self, pos: BlockPos) -> (i32, f64) {
        let Some(chest) = self.chunks.block_entities().get(pos).filter(|chest| {
            chest.ender_chest().is_some() || chest.chest().is_some() || chest.barrel().is_some()
        }) else {
            return (0, 0.0);
        };
        let facts = &self.loot_level_facts;
        let (Some(context), Some(dimension)) =
            (self.structure_entities.as_ref(), facts.dimension.as_ref())
        else {
            return (0, 0.0);
        };
        let range = self.container_openers_range(pos) + 4.0;
        let bounds = bbb_server_types::geometry::Aabb {
            min_x: f64::from(pos.x),
            min_y: f64::from(pos.y),
            min_z: f64::from(pos.z),
            max_x: f64::from(pos.x) + 1.0,
            max_y: f64::from(pos.y) + 1.0,
            max_z: f64::from(pos.z) + 1.0,
        }
        .inflate(range, range, range);
        let mut count = 0;
        let mut max_range = 0.0_f64;
        for player in context.store.players() {
            let owns_container = if chest.ender_chest().is_none() {
                player.dimension == *dimension
                    && player
                        .inventory
                        .open_container_anchor()
                        .is_some_and(|anchor| anchor.positions.contains(&pos))
                    && player
                        .inventory
                        .container_instances()
                        .iter()
                        .any(|active| active.same_position_and_type(chest))
            } else {
                player
                    .inventory
                    .active_ender_chest()
                    .is_some_and(|(level, active)| {
                        facts
                            .entity_nbt
                            .as_ref()
                            .is_some_and(|snapshot| level == *snapshot.level_key(*dimension))
                            && active.same_position_and_type(chest)
                    })
            };
            if !player.avatar_removed
                && player.game_type != bbb_protocol::game_types::GameType::Spectator
                && owns_container
                && player
                    .bounding_box()
                    .is_ok_and(|bbox| bounds.intersects(bbox))
            {
                count += 1;
                max_range = max_range.max(player.block_interaction_range());
            }
        }
        if chest.chest().is_some() {
            let bodies = crate::entity::store::EntityBodyReadView::new(
                context.store,
                context.current,
                context.ancestors.or(self.detached_entities),
            );
            for id in bodies.ids_in_box_sections(bounds) {
                let Some(body) = bodies.get(id).filter(|body| {
                    body.removal_reason().is_none() && body.bounding_box().intersects(bounds)
                }) else {
                    continue;
                };
                let Some(opened) = context
                    .store
                    .mob_ai(body.id())
                    .and_then(|runtime| runtime.copper_golem_opened_chest())
                else {
                    continue;
                };
                // CopperGolem.java:411-425: a golem opens both halves with a range of three.
                let state = self.block_state(opened);
                let connected = (crate::block_behavior::chest::is_chest_block(state)
                    && (crate::block_behavior::chest::chest_type_is_left(state)
                        || crate::block_behavior::chest::chest_type_is_right(state)))
                .then(|| crate::block_behavior::chest::connected_direction(state))
                .flatten()
                .map(|direction| bbb_server_types::directions::relative(opened, direction));
                if opened == pos || connected == Some(pos) {
                    count += 1;
                    max_range = max_range.max(3.0);
                }
            }
        }
        (count, max_range)
    }

    pub(crate) fn set_container_openers_count(&mut self, pos: BlockPos, count: i32) {
        let Some(block_entity) = self.chunks.block_entities_mut().get_mut(pos) else {
            return;
        };
        if let Some(chest) = crate::block_entity::BlockEntity::chest_mut(block_entity) {
            chest.set_open_count(count);
        } else if let Some(barrel) = crate::block_entity::BlockEntity::barrel_mut(block_entity) {
            barrel.set_open_count(count);
        } else if let Some(ender_chest) =
            crate::block_entity::BlockEntity::ender_chest_mut(block_entity)
        {
            ender_chest.set_open_count(count);
        }
    }

    pub(crate) fn set_comparator_block_entity_output_signal(
        &mut self,
        pos: BlockPos,
        value: i32,
    ) -> bool {
        let Some(comparator) = self
            .chunks
            .block_entities_mut()
            .get_mut(pos)
            .and_then(crate::block_entity::BlockEntity::comparator_mut)
        else {
            return false;
        };
        comparator.set_output_signal(value);
        true
    }

    pub(crate) fn analog_output_signal(&mut self, pos: BlockPos, direction: Direction) -> i32 {
        analog_output::analog_output_signal(self, pos, direction)
    }

    pub(crate) fn has_container(&self, pos: BlockPos) -> bool {
        self.chunks
            .block_entities()
            .get(pos)
            .is_some_and(|entity| entity.container_slots().is_some())
    }

    #[cfg(test)]
    pub(crate) fn set_container_items(
        &mut self,
        pos: BlockPos,
        items: Vec<bbb_server_types::item_stack::RuntimeItemStack>,
    ) -> bbb_server_types::Result<()> {
        let item_tags = self.chunks.registry_tags_shared().map_or(
            crate::entity::EntityRegistryTagView::Builtin26_1,
            crate::entity::EntityRegistryTagView::Frozen,
        );
        let occupancy = {
            let Some(block_entity) = self.chunks.block_entities_mut().get_mut(pos) else {
                return Ok(());
            };
            if let Some(chest) = block_entity.chest_mut() {
                let dest = chest.items_mut();
                if dest.len() == items.len() {
                    for (dest, item) in dest.iter_mut().zip(&items) {
                        if *dest != *item {
                            *dest = item.clone();
                        }
                    }
                }
                return Ok(());
            }
            if let Some(hopper) = block_entity.hopper_mut() {
                let dest = hopper.items_mut();
                if dest.len() == items.len() {
                    for (dest, item) in dest.iter_mut().zip(&items) {
                        if *dest != *item {
                            *dest = item.clone();
                        }
                    }
                }
                return Ok(());
            }
            if let Some(barrel) = block_entity.barrel_mut() {
                let dest = barrel.items_mut();
                if dest.len() == items.len() {
                    for (dest, item) in dest.iter_mut().zip(&items) {
                        if *dest != *item {
                            *dest = item.clone();
                        }
                    }
                }
                return Ok(());
            }
            if let Some(dispenser) = block_entity.dispenser_mut() {
                let dest = dispenser.items_mut();
                if dest.len() == items.len() {
                    for (dest, item) in dest.iter_mut().zip(&items) {
                        if *dest != *item {
                            *dest = item.clone();
                        }
                    }
                }
                return Ok(());
            }
            if let Some(shulker) = block_entity.shulker_box_mut() {
                let dest = shulker.items_mut();
                if dest.len() == items.len() {
                    for (dest, item) in dest.iter_mut().zip(&items) {
                        if *dest != *item {
                            *dest = item.clone();
                        }
                    }
                }
                return Ok(());
            }
            if let Some(pot) = block_entity.decorated_pot_mut() {
                pot.overwrite_items(&items);
                return Ok(());
            }
            if let Some(shelf) = block_entity.shelf_mut() {
                shelf.overwrite_items(&items);
                return Ok(());
            }
            if let Some(bookshelf) = block_entity.chiseled_bookshelf_mut() {
                bookshelf
                    .overwrite_items(&items, |item| {
                        crate::block_entity::chiseled_bookshelf::item_is_bookshelf_book_with(
                            item,
                            |registry, tag, id| item_tags.contains(registry, tag, id),
                        )
                    })
                    .map(|_| bookshelf.items().to_vec())
            } else {
                None
            }
        };
        if let Some(items) = occupancy {
            let Some(state) = self.chunks.get_block_state(pos.x, pos.y, pos.z) else {
                return Ok(());
            };
            if let Some(next) = chiseled_bookshelf_block::with_occupancy(state, &items) {
                if next != state {
                    let _ =
                        LevelCtx::set_block(self, pos, next, SET_BLOCK_UPDATE_ALL, UPDATE_LIMIT);
                }
            }
        }
        Ok(())
    }

    pub(crate) fn spawn_block_item(
        &mut self,
        position: Vec3d,
        stack: bbb_server_types::item_stack::RuntimeItemStack,
    ) -> bbb_server_types::Result<()> {
        if stack.is_empty() {
            return Ok(());
        }
        self.item_spawns
            .items
            .push(crate::entity::block_drops::PendingItemSpawn {
                position,
                item: stack,
                default_pickup_delay: true,
            });
        Ok(())
    }

    /// `Block.popResource` (`Block.java:417-424`).
    pub(crate) fn pop_resource(
        &mut self,
        pos: BlockPos,
        stack: &bbb_protocol::packets::ItemStackSummary,
    ) {
        crate::entity::block_drops::pop_resource(
            pos,
            stack,
            self.random,
            self.game_rules.block_drops,
            &mut self.item_spawns.items,
        );
    }

    pub(crate) fn record_item_failure(
        &mut self,
        _pos: BlockPos,
        error: bbb_server_types::ServerWorldError,
    ) {
        tracing::warn!("block item operation: {error}");
    }

    pub(crate) fn request_block_mob_spawn(&mut self, request: BlockMobSpawnRequest) -> bool {
        self.item_spawns.block_mobs.push(request);
        true
    }

    pub(crate) fn request_golem_spawn(&mut self, request: GolemSpawnRequest) -> Option<bool> {
        self.item_spawns.pattern_mobs.push(request);
        Some(true)
    }

    pub(crate) fn request_falling_block_fall(
        &mut self,
        request: FallingBlockFallRequest,
    ) -> Option<bool> {
        self.set_block(
            request.pos,
            fallen_block_replacement(request.block_state_id),
            SET_BLOCK_UPDATE_ALL,
            UPDATE_LIMIT,
        );
        self.item_spawns.falling_blocks.push(request);
        Some(true)
    }

    pub(crate) fn is_valid_spawn(&self, block_state_id: i32, entity_type_id: i32) -> Option<bool> {
        Some(crate::world::spawner_minecarts::valid_spawn_floor_state(
            self.chunks,
            block_state_id,
            entity_type_id,
        ))
    }

    pub(crate) fn prime_tnt_from_fire(&mut self, pos: BlockPos) -> Option<bool> {
        if !self.game_rules.tnt_explodes {
            return Some(false);
        }
        let entities = self.structure_entities.as_mut()?;
        crate::entity::spawn_tnt(
            entities.store,
            self.random,
            Vec3d {
                x: f64::from(pos.x) + 0.5,
                y: f64::from(pos.y),
                z: f64::from(pos.z) + 0.5,
            },
            None,
        )?;
        Some(true)
    }

    pub(crate) fn prime_tnt_and_remove(&mut self, pos: BlockPos) -> Option<bool> {
        if !self.game_rules.tnt_explodes {
            return Some(false);
        }
        if !self.item_spawns.tnt_primes.contains(&pos) {
            self.item_spawns.tnt_primes.push(pos);
        }
        Some(true)
    }

    pub(crate) fn fluid_identity_in_tag(&self, fluid: VanillaFluidIdentity, tag_id: &str) -> bool {
        self.registry_tag_membership("minecraft:fluid", tag_id, fluid.resource_id())
    }

    pub(crate) fn max_block_brightness(&self, pos: BlockPos) -> i32 {
        max_local_raw_brightness(self.chunks, pos, self.sky_darken())
    }

    pub(crate) fn has_neighbor_signal(&self, pos: BlockPos) -> bool {
        with_signal_context(self, WireSignal::Emitting, |context| {
            crate::redstone::has_neighbor_signal(pos, context)
        })
    }

    pub(crate) fn has_signal(&self, pos: BlockPos, direction: Direction) -> bool {
        with_signal_context(self, WireSignal::Emitting, |context| {
            crate::redstone::has_signal(pos, direction, context)
        })
    }

    pub(crate) fn get_signal(&self, pos: BlockPos, direction: Direction) -> i32 {
        with_signal_context(self, WireSignal::Emitting, |context| {
            crate::redstone::get_signal(pos, direction, context)
        })
    }

    pub(crate) fn get_control_input_signal(
        &self,
        pos: BlockPos,
        direction: Direction,
        only_diodes: bool,
    ) -> i32 {
        with_signal_context(self, WireSignal::Emitting, |context| {
            crate::redstone::get_control_input_signal(pos, direction, only_diodes, context)
        })
    }

    pub(crate) fn update_neighbour_for_output_signal(
        &mut self,
        pos: BlockPos,
        changed_block_type_id: i32,
    ) {
        for direction in HORIZONTAL {
            let mut relative_pos = bbb_server_types::directions::relative(pos, direction);
            if !self.has_chunk_at(relative_pos) {
                continue;
            }
            let mut state = self.block_state(relative_pos);
            if state_is_block(state, VANILLA_BLOCK_COMPARATOR_ID) {
                self.neighbor_changed_with_state(state, relative_pos, changed_block_type_id);
            } else if self.is_redstone_conductor_at(relative_pos) {
                relative_pos = bbb_server_types::directions::relative(relative_pos, direction);
                state = self.block_state(relative_pos);
                if state_is_block(state, VANILLA_BLOCK_COMPARATOR_ID) {
                    self.neighbor_changed_with_state(state, relative_pos, changed_block_type_id);
                }
            }
        }
    }

    pub(crate) fn get_best_neighbor_signal_with(
        &self,
        pos: BlockPos,
        wire_signal: WireSignal,
    ) -> i32 {
        with_signal_context(self, wire_signal, |context| {
            crate::redstone::get_best_neighbor_signal(pos, context)
        })
    }

    pub(crate) fn destroy_block_with_limit(
        &mut self,
        pos: BlockPos,
        drop_resources: bool,
        update_limit: i32,
    ) -> bool {
        self.destroy_block_with_source(pos, drop_resources, update_limit, None, LootEntity::Absent)
    }

    pub(crate) fn destroy_block_by_entity(
        &mut self,
        pos: BlockPos,
        drop_resources: bool,
        entity: &crate::entity::Entity,
        store: Option<&crate::entity::EntityStore>,
    ) -> bool {
        let breaker = LootEntity::Stored(crate::loot_runtime::StoredLootEntity {
            entity,
            bodies: store.map(|store| {
                crate::entity::store::EntityBodyReadView::new(
                    store,
                    Some(entity),
                    self.detached_entities,
                )
            }),
            raid_at_position: None,
        });
        self.destroy_block_with_source(
            pos,
            drop_resources,
            UPDATE_LIMIT,
            Some(entity.id()),
            breaker,
        )
    }

    fn destroy_block_with_source(
        &mut self,
        pos: BlockPos,
        drop_resources: bool,
        update_limit: i32,
        source_entity_id: Option<i32>,
        breaker: LootEntity<'_>,
    ) -> bool {
        let state = self.block_state(pos);
        if block_state_is_air(state) {
            return false;
        }
        let fluid = self.fluid_state(pos);
        // The fixed base-fire family controls particles, independently of live tags.
        if !bbb_protocol::block_state_types::vanilla_block_state_block_type_id(state).is_some_and(
            |id| {
                matches!(
                    id,
                    bbb_protocol::block_types::VANILLA_BLOCK_FIRE_ID
                        | bbb_protocol::block_types::VANILLA_BLOCK_SOUL_FIRE_ID
                )
            },
        ) {
            self.level_event(LEVEL_EVENT_PARTICLES_DESTROY_BLOCK, pos, state);
        }
        if drop_resources {
            self.drop_resources_with_tool_at(pos, state, &LootTool::empty(), breaker);
        }
        let destroyed = self.set_block(
            pos,
            fluid.create_legacy_block(),
            SET_BLOCK_UPDATE_ALL,
            update_limit,
        );
        if destroyed {
            self.game_event_from_entity(
                VanillaGameEvent::BLOCK_DESTROY,
                pos,
                source_entity_id,
                Some(state),
            );
        }
        destroyed
    }

    pub(crate) fn remove_block(&mut self, pos: BlockPos) -> bool {
        let fluid = self.fluid_state(pos);
        self.set_block(
            pos,
            fluid.create_legacy_block(),
            SET_BLOCK_UPDATE_ALL,
            UPDATE_LIMIT,
        )
    }

    pub(crate) fn destroy_block(&mut self, pos: BlockPos, drop_resources: bool) -> bool {
        self.destroy_block_with_limit(pos, drop_resources, UPDATE_LIMIT)
    }
}

fn with_signal_context<T>(
    level: &LevelCtx<'_, '_>,
    wire_signal: WireSignal,
    question: impl FnOnce(&mut SignalContext<'_>) -> T,
) -> T {
    let mut read = |at: BlockPos| level.block_state(at);
    let mut context = match wire_signal {
        WireSignal::Emitting => SignalContext::emitting(&mut read),
        WireSignal::Suppressed => SignalContext::suppressed(&mut read),
    }
    .with_reader(level);
    question(&mut context)
}

impl crate::block_getter::BlockGetter for LevelCtx<'_, '_> {
    fn block_state(&self, pos: BlockPos) -> i32 {
        LevelCtx::block_state(self, pos)
    }

    fn raw_brightness(&self, pos: BlockPos, sky_darkening: i32) -> Option<i32> {
        Some(LevelCtx::raw_brightness(self, pos, sky_darkening))
    }

    fn tags(&self) -> &dyn crate::block_getter::BlockTags {
        LevelCtx::tags(self)
    }

    fn content_registry(&self) -> &bbb_protocol::content_registry::ContentRegistry {
        self.chunks.content_registry()
    }
}

impl<'a> LevelCtx<'a, '_> {
    pub(crate) fn content_registry(&self) -> &bbb_protocol::content_registry::ContentRegistry {
        self.chunks.content_registry()
    }

    /// The level's gameplay stream, borrowed for the context's whole life so a
    /// draw can sit beside a mutable use of the context.
    pub(crate) fn level_random(&self) -> &'a GameRandom {
        self.random
    }

    #[cfg(test)]
    pub(crate) fn save_dirty_epoch_for_test(&self) -> u64 {
        self.chunks
            .save_dirty_chunks()
            .map(|(_, epoch)| epoch)
            .max()
            .unwrap_or(0)
    }
}
