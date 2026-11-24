//! Placement candidates and their eager effects on the live world.

use bbb_protocol::block_types::{
    VanillaGetStateForPlacementOverride, VANILLA_BLOCK_DIRT_ID, VANILLA_BLOCK_POWDER_SNOW_ID,
    VANILLA_BLOCK_SCAFFOLDING_ID,
};
use bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_BLOCK_ENTITY_DATA_TYPE_ID;
use bbb_protocol::entity_synched_data::VANILLA_ENTITY_SHARED_FLAG_SHIFT_KEY_DOWN;
use bbb_protocol::game_types::GameType;
use bbb_protocol::item_types::{vanilla_item_type, VanillaBlockItemKind};
use bbb_protocol::packets::{BlockPos, Direction, InteractionHand, ItemStackSummary};
use bbb_protocol::NbtSummaryValue;
use bbb_server_types::chunk_data::SECTION_SIDE;
use bbb_server_types::chunk_ticks::{BlockTickType, TickPriority};
use bbb_server_types::directions::{horizontal_direction, relative};
use bbb_server_types::fluid::{block_state_fluid_state, FluidFamily};
use bbb_server_types::geometry::{positioned_shapes, Aabb};
use bbb_server_types::item_stack::RuntimeItemStack;

use crate::block_behavior::placement::{
    game_master_state_for_placement, lit_state_can_survive, state_for_placement_with_signals,
    RawBrightnessAt,
};
use crate::block_behavior::{
    self, bed, coral, door, growing_plant, lectern, pitcher_crop, vegetation,
    BlockPlacementContext, BlockPlacementView,
};
use crate::block_behavior::{
    default_block_state_id, get_state_for_placement_override, state_is_block,
};
use crate::block_entity::chest::ChestBlockEntity;
use crate::block_entity::comparator::ComparatorBlockEntity;
use crate::block_entity::BlockEntity;
use crate::block_getter::BlockTags;
use crate::chunk::ChunkStore;
use crate::connection::ConnectionId;
use crate::entity::store::{EntityBodyReadView, EntityStore};
use crate::fluid::{above, with_property};
use crate::redstone::SignalSources;
use crate::world::simulation_level::placement_raw_brightness;
use crate::world::{is_in_world_bounds, LevelId, ServerLevel, ServerWorld, WorldEvent};
use bbb_server_types::block_queries::{block_type_of, is_shulker_box, DIRECTION_VALUES};
use std::cell::RefCell;

fn placement_block_state(chunks: &mut ChunkStore, pos: BlockPos) -> i32 {
    let air = chunks.air_state_id();
    if !is_in_world_bounds(chunks, pos) {
        return air;
    }
    chunks.load_column(pos.x >> 4, pos.z >> 4);
    chunks.get_block_state(pos.x, pos.y, pos.z).unwrap_or(air)
}

struct PlacementSignalSources<'a, 'c>(&'a RefCell<&'c mut ChunkStore>);

impl crate::redstone::SignalSourceReader for PlacementSignalSources<'_, '_> {
    fn comparator_output(&self, pos: BlockPos) -> i32 {
        self.0
            .borrow()
            .block_entities()
            .get(pos)
            .and_then(BlockEntity::comparator)
            .map_or(0, ComparatorBlockEntity::output_signal)
    }

    fn chest_open_count(&self, pos: BlockPos) -> i32 {
        self.0
            .borrow()
            .block_entities()
            .get(pos)
            .and_then(BlockEntity::chest)
            .map_or(0, ChestBlockEntity::open_count)
    }

    fn jukebox_playing(&self, pos: BlockPos) -> bool {
        self.0
            .borrow()
            .block_entities()
            .get(pos)
            .and_then(BlockEntity::jukebox)
            .is_some_and(|jukebox| jukebox.song_player().is_playing())
    }

    fn shulker_open(&self, pos: BlockPos) -> bool {
        self.0
            .borrow()
            .block_entities()
            .get(pos)
            .and_then(BlockEntity::shulker_box)
            .is_some_and(crate::block_entity::shulker_box::ShulkerBoxBlockEntity::lid_moved)
    }

    fn is_redstone_conductor(&self, state: i32) -> bool {
        let chunks = self.0.borrow();
        crate::redstone::is_redstone_conductor_with_registry(state, chunks.content_registry())
    }

    fn support_face(
        &self,
        state: i32,
        pos: BlockPos,
        direction: Direction,
        support_type: bbb_protocol::block_state_types::VanillaBlockSupportType,
    ) -> bool {
        let chunks = self.0.borrow();
        bbb_server_types::block_queries::face_sturdy_at_with_registry(
            state,
            pos,
            direction,
            support_type,
            chunks.content_registry(),
        )
    }
}

/// Placement reads signals through the same block-entity sources as the tick,
/// and the light through the stored light layers.
fn with_placement_signals<R>(
    chunks: &mut ChunkStore,
    body: impl FnOnce(&mut dyn FnMut(BlockPos) -> i32, RawBrightnessAt<'_>, SignalSources<'_>) -> R,
) -> R {
    let chunks = RefCell::new(chunks);
    let brightness =
        |at: BlockPos, sky_darkening| placement_raw_brightness(&chunks.borrow(), at, sky_darkening);
    let sources = PlacementSignalSources(&chunks);
    let mut read = |at: BlockPos| placement_block_state(&mut chunks.borrow_mut(), at);
    body(&mut read, &brightness, SignalSources::new(&sources))
}

fn held_block_entity_data_contains(item: &ItemStackSummary, key: &str) -> bool {
    let patch = &item.component_patch;
    if patch
        .removed_type_ids
        .contains(&VANILLA_DATA_COMPONENT_BLOCK_ENTITY_DATA_TYPE_ID)
        || !patch
            .added_type_ids
            .contains(&VANILLA_DATA_COMPONENT_BLOCK_ENTITY_DATA_TYPE_ID)
    {
        return false;
    }
    match patch.block_entity_data_tag.as_ref() {
        Some(NbtSummaryValue::Compound(entries)) => entries.iter().any(|entry| entry.name == key),
        _ => false,
    }
}

/// `getCollisionShape` under `CollisionContext.placementContext`: empty for
/// scaffolding and powder snow (`ScaffoldingBlock.java:137-139`,
/// `PowderSnowBlock.java:116-131`), and a cube for a shulker box that has no
/// block entity yet (`ShulkerBoxBlock.java:152-156`).
fn placement_collision_boxes(state: i32, pos: BlockPos) -> Option<Vec<Aabb>> {
    match block_type_of(state)? {
        VANILLA_BLOCK_SCAFFOLDING_ID | VANILLA_BLOCK_POWDER_SNOW_ID => Some(Vec::new()),
        _ if is_shulker_box(state) => Some(vec![Aabb::of_block(BlockPos { x: 0, y: 0, z: 0 })]),
        _ => positioned_shapes::collision_boxes(state, pos),
    }
}

/// `CollisionGetter.isUnobstructed` (`CollisionGetter.java:30-33`,
/// `EntityGetter.java:33-48`) with no source entity.
fn placement_is_unobstructed(state: i32, pos: BlockPos, entities: &EntityStore) -> Option<bool> {
    let shape = placement_collision_boxes(state, pos)?;
    let bodies = EntityBodyReadView::new(entities, None, None);
    for unit in shape {
        let placed = Aabb {
            min_x: unit.min_x + f64::from(pos.x),
            min_y: unit.min_y + f64::from(pos.y),
            min_z: unit.min_z + f64::from(pos.z),
            max_x: unit.max_x + f64::from(pos.x),
            max_y: unit.max_y + f64::from(pos.y),
            max_z: unit.max_z + f64::from(pos.z),
        };
        for id in bodies.ids_in_box_sections_with_players(placed) {
            if let Some(player) = entities.player(id) {
                if !player.avatar_removed
                    && player.game_type != GameType::Spectator
                    && player.bounding_box().ok()?.intersects(placed)
                {
                    return Some(false);
                }
            } else if let Some(entity) = bodies.get(id) {
                if entity.removal_reason().is_none()
                    && entity.blocks_building()
                    && entity.bounding_box().intersects(placed)
                {
                    return Some(false);
                }
            }
        }
    }
    Some(true)
}

impl ServerWorld {
    pub(super) fn placed_block_state_with_events(
        &mut self,
        connection_id: ConnectionId,
        dimension: impl Into<LevelId>,
        context: BlockPlacementContext,
        item_type_id: Option<i32>,
        held_item: Option<&ItemStackSummary>,
        events: &mut Vec<WorldEvent>,
    ) -> Option<i32> {
        let dimension = dimension.into();
        let rotation = self.levels.player(connection_id)?.rotation();
        let view = BlockPlacementView {
            x_rot: rotation.x_rot,
            y_rot: rotation.y_rot,
        };
        if !view.x_rot.is_finite() || !view.y_rot.is_finite() {
            return None;
        }
        let item_type_id = item_type_id?;
        let Some(behavior) =
            vanilla_item_type(item_type_id).and_then(|item| item.block_item_behavior)
        else {
            let content_registry = self.registries.content_registry();
            let item = content_registry.item_by_id(item_type_id)?;
            if !item.is_block_item {
                return None;
            }
            let block = content_registry.block_by_resource_id(item.resource_id())?;
            let state = block.first_state_id;
            return self
                .placement_candidate_is_unobstructed(dimension, state, context.pos)?
                .then_some(state);
        };
        if behavior.kind == VanillaBlockItemKind::GameMasterBlockItem {
            if !self.can_use_game_master_blocks(connection_id) {
                return None;
            }
            let state = game_master_state_for_placement(behavior.block_type_id, context, view)?;
            return self
                .placement_candidate_is_unobstructed(dimension, state, context.pos)?
                .then_some(state);
        }
        if !matches!(
            behavior.kind,
            VanillaBlockItemKind::BedItem
                | VanillaBlockItemKind::BlockItem
                | VanillaBlockItemKind::DoubleHighBlockItem
        ) {
            return self.pure_placement_state(dimension, item_type_id, context, view);
        }
        let owner = get_state_for_placement_override(behavior.block_type_id);
        let tags = self.registries.registry_tags_shared();
        let state = match owner {
            Some(VanillaGetStateForPlacementOverride::DoublePlantBlock) => {
                if !self.upper_cell_takes_the_other_half(dimension, item_type_id, context)? {
                    return None;
                }
                default_block_state_id(behavior.block_type_id)
            }
            Some(VanillaGetStateForPlacementOverride::SmallDripleafBlock) => {
                if !self.upper_cell_takes_the_other_half(dimension, item_type_id, context)? {
                    return None;
                }
                vegetation::small_dripleaf_state_for_placement(
                    behavior.block_type_id,
                    context.pos,
                    context.horizontal_direction,
                    &*tags,
                    &mut |at| self.placement_block_state(dimension, at),
                )?
            }
            Some(VanillaGetStateForPlacementOverride::BedBlock) => {
                let head = relative(context.pos, context.horizontal_direction);
                let head_state = self.placement_block_state(dimension, head);
                block_type_of(head_state)?;
                if !block_behavior::can_replace_for_placement(item_type_id, head_state, context)?
                    || !self
                        .level_by_id(&dimension)
                        .world_border
                        .contains_block_at(head, self.game_time())
                {
                    return None;
                }
                bed::state_for_placement(behavior.block_type_id, context.horizontal_direction)?
            }
            Some(VanillaGetStateForPlacementOverride::DoorBlock) => {
                if !self.upper_cell_takes_the_other_half(dimension, item_type_id, context)? {
                    return None;
                }
                let chunks = &mut self.level_mut_by_id(&dimension).chunks;
                with_placement_signals(chunks, |read, _, sources| {
                    door::state_for_placement(
                        behavior.block_type_id,
                        context.pos,
                        context.horizontal_direction,
                        context.click_x,
                        context.click_z,
                        read,
                        sources,
                    )
                })?
            }
            Some(VanillaGetStateForPlacementOverride::CoralBlock) => {
                self.coral_placement_state(dimension, behavior.block_type_id, context.pos)?
            }
            Some(
                VanillaGetStateForPlacementOverride::GrowingPlantBlock
                | VanillaGetStateForPlacementOverride::KelpBlock,
            ) => {
                self.growing_plant_placement_state(dimension, behavior.block_type_id, context.pos)?
            }
            Some(VanillaGetStateForPlacementOverride::PitcherCropBlock) => {
                pitcher_crop::state_for_placement(behavior.block_type_id)?
            }
            Some(VanillaGetStateForPlacementOverride::LecternBlock) => {
                let has_book = self.can_use_game_master_blocks(connection_id)
                    && held_item.is_some_and(|item| held_block_entity_data_contains(item, "Book"));
                lectern::state_for_placement(
                    behavior.block_type_id,
                    context.horizontal_direction,
                    has_book,
                )?
            }
            Some(VanillaGetStateForPlacementOverride::CreakingHeartBlock) => {
                let creaking_active = {
                    let (chunks, originals) = self.original_mob_inputs(&dimension);
                    originals
                        .creaking_active(chunks, context.pos)
                        .unwrap_or(false)
                };
                crate::block_behavior::creaking_heart::state_for_placement_with_active(
                    behavior.block_type_id,
                    context.pos,
                    context.clicked_face,
                    creaking_active,
                    &*tags,
                    &mut |at| self.placement_block_state(dimension, at),
                )?
            }
            // Lift before a later collision refusal.
            Some(VanillaGetStateForPlacementOverride::DirtPathBlock) => {
                let path = default_block_state_id(behavior.block_type_id);
                if self.placement_state_survives(dimension, path, context.pos)? {
                    path
                } else {
                    let dirt = default_block_state_id(VANILLA_BLOCK_DIRT_ID);
                    self.push_live_entities_up(&dimension, path, dirt, context.pos, events)?;
                    dirt
                }
            }
            _ => return self.pure_placement_state(dimension, item_type_id, context, view),
        };
        if !self.placement_state_survives(dimension, state, context.pos)? {
            return None;
        }
        self.placement_candidate_is_unobstructed(dimension, state, context.pos)?
            .then_some(state)
    }

    fn upper_cell_takes_the_other_half(
        &mut self,
        dimension: impl Into<LevelId>,
        item_type_id: i32,
        context: BlockPlacementContext,
    ) -> Option<bool> {
        let dimension = dimension.into();
        let upper = above(context.pos);
        let chunks = &self.level_by_id(&dimension).chunks;
        let offset = usize::try_from(i64::from(upper.y) - i64::from(chunks.min_y())).ok();
        if !offset
            .is_some_and(|offset| offset < chunks.section_count().saturating_mul(SECTION_SIDE))
        {
            return Some(false);
        }
        let upper_state = self.placement_block_state(dimension, upper);
        block_type_of(upper_state)?;
        block_behavior::can_replace_for_placement(item_type_id, upper_state, context)
    }

    fn pure_placement_state(
        &mut self,
        dimension: impl Into<LevelId>,
        item_type_id: i32,
        context: BlockPlacementContext,
        view: BlockPlacementView,
    ) -> Option<i32> {
        let dimension = dimension.into();
        let tags = self.registries.registry_tags_shared();
        let ServerLevel {
            chunks, entities, ..
        } = self.levels.get_mut(&dimension)?;
        with_placement_signals(chunks, |read, brightness, sources| {
            state_for_placement_with_signals(
                item_type_id,
                context,
                view,
                &*tags,
                read,
                &mut |state, pos| placement_is_unobstructed(state, pos, entities),
                Some(brightness),
                sources,
            )
        })
    }

    /// `BlockState.canSurvive` over the live level, light included.
    fn placement_state_survives(
        &mut self,
        dimension: LevelId,
        state: i32,
        pos: BlockPos,
    ) -> Option<bool> {
        let tags = self.registries.registry_tags_shared();
        with_placement_signals(
            &mut self.level_mut_by_id(&dimension).chunks,
            |read, brightness, _| lit_state_can_survive(state, pos, &*tags, read, Some(brightness)),
        )
    }

    fn placement_block_state(&mut self, dimension: impl Into<LevelId>, pos: BlockPos) -> i32 {
        let dimension = dimension.into();
        placement_block_state(&mut self.level_mut_by_id(&dimension).chunks, pos)
    }

    pub(in crate::world) fn placement_candidate_is_unobstructed(
        &self,
        dimension: impl Into<LevelId>,
        state: i32,
        pos: BlockPos,
    ) -> Option<bool> {
        let dimension = dimension.into();
        placement_is_unobstructed(state, pos, &self.level_by_id(&dimension).entities)
    }

    fn coral_placement_state(
        &mut self,
        dimension: impl Into<LevelId>,
        block_type_id: i32,
        pos: BlockPos,
    ) -> Option<i32> {
        let dimension = dimension.into();
        let state = default_block_state_id(block_type_id);
        let block = BlockTickType::of_block_state(state)?;
        for direction in DIRECTION_VALUES {
            let neighbour = self.placement_block_state(dimension, relative(pos, direction));
            block_type_of(neighbour)?;
            if self.tags().fluid_in_tag(
                block_state_fluid_state(neighbour).identity(),
                coral::WATER_TAG,
            ) {
                return Some(state);
            }
        }
        let delay = coral::DIE_TICK_MIN_DELAY
            + self
                .level_mut_by_id(&dimension)
                .random
                .next_int(coral::DIE_TICK_DELAY_SPREAD);
        self.schedule_block_tick(&dimension, pos, block, delay, TickPriority::Normal);
        Some(state)
    }

    fn growing_plant_placement_state(
        &mut self,
        dimension: impl Into<LevelId>,
        block_type_id: i32,
        pos: BlockPos,
    ) -> Option<i32> {
        let dimension = dimension.into();
        let owner = get_state_for_placement_override(block_type_id)?;
        let default = default_block_state_id(block_type_id);
        let kind = growing_plant::growing_plant_kind(default)?;
        if owner == VanillaGetStateForPlacementOverride::KelpBlock {
            let replaced = self.placement_block_state(dimension, pos);
            block_type_of(replaced)?;
            let fluid = block_state_fluid_state(replaced);
            if !fluid.is_same_family(FluidFamily::Water) || !fluid.is_full() {
                return None;
            }
        }
        let growth_state =
            self.placement_block_state(dimension, relative(pos, kind.growth_direction()));
        block_type_of(growth_state)?;
        if state_is_block(growth_state, kind.head_block_type_id())
            || state_is_block(growth_state, kind.body_block_type_id())
        {
            return Some(default_block_state_id(kind.body_block_type_id()));
        }
        if !growing_plant::is_head(default) {
            return Some(default);
        }
        let age = self
            .level_mut_by_id(&dimension)
            .random
            .next_int(growing_plant::PLACEMENT_AGE_BOUND);
        with_property(default, "age", &age.to_string())
    }
}

impl ServerWorld {
    /// `BlockItem.place` (`BlockItem.java:48-91`) for the `BlockItem`
    /// subclasses whose block takes its default state
    /// (`SolidBucketItem`, `PlaceOnWaterBlockItem`): the clicked cell when it
    /// is replaceable, else the one against the clicked face
    /// (`BlockPlaceContext.java:30`, `:52-57`), if the state survives there
    /// and nothing collides with it (`BlockItem.java:134-138`).
    #[allow(clippy::too_many_arguments)]
    pub(in crate::world) fn place_default_block_from_item(
        &mut self,
        connection_id: ConnectionId,
        dimension: impl Into<LevelId>,
        clicked: BlockPos,
        face: Direction,
        block_type_id: i32,
        hand: InteractionHand,
        held: &RuntimeItemStack,
        place_sound: Option<&str>,
        events: &mut Vec<WorldEvent>,
    ) -> bool {
        let dimension = dimension.into();
        let (horizontal_direction, secondary_use_active) = self
            .levels
            .player(connection_id)
            .map_or((Direction::North, false), |player| {
                (
                    horizontal_direction(player.rotation().y_rot).unwrap_or(Direction::North),
                    player.shared_flag(VANILLA_ENTITY_SHARED_FLAG_SHIFT_KEY_DOWN),
                )
            });
        let replaceable = |pos: BlockPos, state: i32, replacing_clicked: bool| {
            let Some(item_id) = held.item_id() else {
                return false;
            };
            let context = BlockPlacementContext {
                pos,
                clicked_face: face,
                click_x: 0.5,
                click_y: 0.5,
                click_z: 0.5,
                replacing_clicked,
                horizontal_direction,
                secondary_use_active,
                inside: false,
            };
            block_behavior::can_replace_for_placement(item_id, state, context) == Some(true)
        };
        let pos = if replaceable(clicked, self.block_state_or_air(dimension, clicked), true) {
            clicked
        } else {
            relative(clicked, face)
        };
        if !self.is_in_world_bounds_by_key(&dimension, pos)
            || pos != clicked && !replaceable(pos, self.block_state_or_air(dimension, pos), false)
        {
            return false;
        }
        let state = default_block_state_id(block_type_id);
        if self.placement_state_survives(dimension, state, pos) == Some(false)
            || self.placement_candidate_is_unobstructed(dimension, state, pos) != Some(true)
        {
            return false;
        }
        self.place_block_from_item(
            connection_id,
            dimension,
            pos,
            state,
            hand,
            held,
            place_sound,
            events,
        )
    }
}
