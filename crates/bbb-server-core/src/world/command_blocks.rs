//! Block command application over live levels.

use crate::world::LevelId;
use std::collections::VecDeque;

use bbb_protocol::packets::BlockPos;
use bbb_protocol::{component_literal_nbt, NbtSummaryValue};
use bbb_registry_codec::PersistentItemStackHolderResolver;

use crate::block_behavior::{
    collision_shape_full_block_at, default_block_state_id, update_block_shape,
};
use crate::block_entity::BlockEntityCommandDataError;
use crate::commands::{
    block_predicate_matches, CloneBlocksRequest, CommandBlockInput, CommandBlockNeighbourPass,
    CommandBlockPlacement,
};
use crate::neighbor_updater::UPDATE_SHAPE_ORDER;
use crate::redstone;
use bbb_server_types::directions::relative;
use bbb_server_types::fluid::{air_block_state_id, block_state_is_air};

use super::{
    block_affect_neighbors_after_removal, vanilla_block_state_block_type_id, LevelCtx, ServerWorld,
    SET_BLOCK_UPDATE_CLIENTS, SET_BLOCK_UPDATE_KNOWN_SHAPE, UPDATE_LIMIT,
};

/// `Block.UPDATE_SKIP_ALL_SIDEEFFECTS` (Block.java:108): the `16 | 32 | 256 |
/// 512` the `strict` literal adds to the write, which turns off the shape leg,
/// the drops, the block-entity side effects and `onPlace` in one flag.
const SET_BLOCK_UPDATE_SKIP_ALL_SIDEEFFECTS: i32 = 816;

/// `Block.UPDATE_SKIP_BLOCK_ENTITY_SIDEEFFECTS` (Block.java:99), the other half
/// of a non-strict write (SetBlockCommand.java:125, FillCommand.java:181).
const SET_BLOCK_UPDATE_SKIP_BLOCK_ENTITY_SIDEEFFECTS: i32 = 256;

impl ServerWorld {
    pub(crate) fn clone_command_blocks(
        &mut self,
        request: &CloneBlocksRequest,
    ) -> Result<i32, NbtSummaryValue> {
        let resolver = self.item_stack_resolver();
        let source_id = self.require_level_id(&request.from_level);
        let target_id = self.require_level_id(&request.to_level);
        let (solid, entities, other, clear) = self.with_level(source_id, |level| {
            let mut solid = Vec::new();
            let mut entities = Vec::new();
            let mut other = Vec::new();
            let mut clear = VecDeque::new();
            for z in request.area_min.z..=request.area_max.z {
                for y in request.area_min.y..=request.area_max.y {
                    for x in request.area_min.x..=request.area_max.x {
                        let pos = BlockPos { x, y, z };
                        let state = level.block_state(pos);
                        if request.masked && block_state_is_air(state) {
                            continue;
                        }
                        let block_entity = level.chunks.block_entities().get(pos);
                        if let Some(filter) = &request.filter {
                            let data = block_entity
                                .map(|entity| entity.command_data_with_resolver(&resolver))
                                .transpose()
                                .map_err(|error| {
                                    component_literal_nbt(format!(
                                        "clone block entity data at {pos:?} failed: {error}"
                                    ))
                                })?;
                            if !block_predicate_matches(filter, state, data.as_ref(), level.tags())
                            {
                                continue;
                            }
                        }
                        let data = block_entity
                            .map(|entity| entity.save_without_metadata_with_resolver(&resolver))
                            .transpose()
                            .map_err(|error| {
                                component_literal_nbt(format!(
                                    "clone block entity data at {pos:?} failed: {error}"
                                ))
                            })?;
                        let info = ClonedBlock {
                            pos: BlockPos {
                                x: x.wrapping_add(request.offset.x),
                                y: y.wrapping_add(request.offset.y),
                                z: z.wrapping_add(request.offset.z),
                            },
                            state,
                            data,
                            previous_state: 0,
                        };
                        if info.data.is_some() {
                            entities.push(info);
                            clear.push_back(pos);
                        } else if crate::block_behavior::support::solid_render_with_registry(
                            state,
                            level.chunks.content_registry(),
                        ) == Some(true)
                            || collision_shape_full_block_at(state, pos)
                        {
                            solid.push(info);
                            clear.push_back(pos);
                        } else {
                            other.push(info);
                            clear.push_front(pos);
                        }
                    }
                }
            }
            Ok((solid, entities, other, clear))
        })?;
        let mut blocks = solid;
        let entity_begin = blocks.len();
        blocks.extend(entities);
        let entity_end = blocks.len();
        blocks.extend(other);
        self.with_level(target_id, |level| {
            for block in &mut blocks {
                block.previous_state = level.block_state(block.pos);
            }
        });
        let flags = SET_BLOCK_UPDATE_CLIENTS
            | if request.strict {
                SET_BLOCK_UPDATE_SKIP_ALL_SIDEEFFECTS
            } else {
                0
            };
        let barrier = default_block_state_id(bbb_protocol::block_types::VANILLA_BLOCK_BARRIER_ID);
        if request.move_source {
            for &pos in &clear {
                self.with_level(source_id, |level| {
                    level.set_block(
                        pos,
                        barrier,
                        flags | SET_BLOCK_UPDATE_SKIP_ALL_SIDEEFFECTS,
                        UPDATE_LIMIT,
                    );
                });
            }
            for &pos in &clear {
                self.with_level(source_id, |level| {
                    level.set_block(
                        pos,
                        air_block_state_id(),
                        if request.strict { flags } else { 3 },
                        UPDATE_LIMIT,
                    );
                });
            }
        }
        for block in blocks.iter().rev() {
            self.with_level(target_id, |level| {
                level.set_block(
                    block.pos,
                    barrier,
                    flags | SET_BLOCK_UPDATE_SKIP_ALL_SIDEEFFECTS,
                    UPDATE_LIMIT,
                );
            });
        }
        let mut count = 0;
        for block in &blocks {
            count += i32::from(self.with_level(target_id, |level| {
                level.set_block(block.pos, block.state, flags, UPDATE_LIMIT)
            }));
        }
        for block in &blocks[entity_begin..entity_end] {
            let previous_jukebox_item = self.playing_jukebox_item(&target_id, block.pos);
            let loaded: Result<i32, BlockEntityCommandDataError> =
                self.with_level(target_id, |level| {
                    if let Some(NbtSummaryValue::Compound(entries)) = &block.data {
                        load_block_entity_data(level, block.pos, entries, &resolver, true)?;
                    }
                    level.set_block(block.pos, block.state, flags, UPDATE_LIMIT);
                    Ok(level.block_state(block.pos))
                });
            let state = match loaded {
                Ok(state) => state,
                Err(error) => {
                    return Err(component_literal_nbt(format!(
                        "clone block entity data at {:?} failed: {error}",
                        block.pos,
                    )));
                }
            };
            if block.data.is_some() {
                self.finish_loaded_block_entity_side_effects(
                    target_id,
                    block.pos,
                    state,
                    previous_jukebox_item,
                );
            }
        }
        if !request.strict {
            for block in blocks.iter().rev() {
                self.with_level(target_id, |level| {
                    update_neighbours_on_block_set(level, block.pos, block.previous_state);
                });
            }
        }
        let area = bbb_server_types::bounding_box::BoundingBox::from_corners(
            request.area_min,
            request.area_max,
        );
        let source = self.level_by_id(&source_id);
        let ticks = source.block_ticks.ticks_in_area(&source.chunks, area);
        crate::level_ticks::schedule_copied_ticks(
            &mut self.level_mut_by_id(&target_id).chunks,
            ticks,
            request.offset,
        );
        Ok(count)
    }

    /// The `/setblock` and `/fill` writes in `BlockPos.betweenClosed` order:
    /// `placed` counts the positions whose `BlockInput.place` answered true,
    /// `changed` those where it or the destroy affector did
    /// (FillCommand.java:172-191).
    pub(crate) fn place_command_blocks(
        &mut self,
        dimension: LevelId,
        destroy_first: bool,
        strict: bool,
        neighbour_pass: CommandBlockNeighbourPass,
        placements: &[CommandBlockPlacement],
    ) -> Result<CommandBlockWrites, NbtSummaryValue> {
        let flags = SET_BLOCK_UPDATE_CLIENTS
            | if strict {
                SET_BLOCK_UPDATE_SKIP_ALL_SIDEEFFECTS
            } else {
                SET_BLOCK_UPDATE_SKIP_BLOCK_ENTITY_SIDEEFFECTS
            };
        let read_before_destroy = neighbour_pass == CommandBlockNeighbourPass::PositionsThatChanged;
        // The publication a command-supplied block-entity compound loads its
        // stacks through, which is the one `/data merge block …` uses.
        let resolver = self.item_stack_resolver();
        let mut writes = CommandBlockWrites::default();
        let mut updated = Vec::new();
        for placement in placements {
            let has_block_entity_data = placement
                .block
                .as_ref()
                .is_some_and(|block| block.block_entity_data.is_some());
            let previous_jukebox_item = has_block_entity_data
                .then(|| self.playing_jukebox_item(&dimension, placement.pos))
                .flatten();
            let (old_state, new_state, destroyed, placed) = self
                .with_level(dimension, |level| {
                    let before = read_before_destroy.then(|| level.block_state(placement.pos));
                    let destroyed =
                        destroy_first && LevelCtx::destroy_block(level, placement.pos, true);
                    let old_state = before.unwrap_or_else(|| level.block_state(placement.pos));
                    let placed = match placement.block.as_ref() {
                        Some(block) => place(level, placement.pos, block, flags, &resolver)?,
                        None => false,
                    };
                    let new_state = level.block_state(placement.pos);
                    Ok((old_state, new_state, destroyed, placed))
                })
                .map_err(|error: BlockEntityCommandDataError| {
                    component_literal_nbt(format!(
                        "block entity data at {:?} failed: {error}",
                        placement.pos
                    ))
                })?;
            if has_block_entity_data {
                self.finish_loaded_block_entity_side_effects(
                    dimension,
                    placement.pos,
                    new_state,
                    previous_jukebox_item,
                );
            }
            writes.placed += i32::from(placed);
            writes.changed += i32::from(placed || destroyed);
            let pass = match neighbour_pass {
                CommandBlockNeighbourPass::None => false,
                CommandBlockNeighbourPass::EveryPosition => placed || placement.block.is_none(),
                CommandBlockNeighbourPass::PositionsThatChanged => placed,
            };
            if pass {
                updated.push((placement.pos, old_state));
            }
        }
        for (pos, old_state) in updated {
            self.with_level(dimension, |level| {
                update_neighbours_on_block_set(level, pos, old_state);
            });
        }
        Ok(writes)
    }
}

/// What [`ServerWorld::place_command_blocks`] changed.
#[derive(Debug, Default, Clone, Copy)]
pub(crate) struct CommandBlockWrites {
    pub(crate) placed: i32,
    pub(crate) changed: i32,
}

struct ClonedBlock {
    pos: BlockPos,
    state: i32,
    data: Option<NbtSummaryValue>,
    previous_state: i32,
}

fn place(
    level: &mut LevelCtx<'_, '_>,
    pos: BlockPos,
    input: &CommandBlockInput,
    flags: i32,
    resolver: &dyn PersistentItemStackHolderResolver,
) -> Result<bool, BlockEntityCommandDataError> {
    // `:67`: a `strict` write carries `UPDATE_KNOWN_SHAPE` and skips the
    // neighbour-shape read altogether, which is what makes it place exactly
    // what was typed.
    let mut state = if flags & SET_BLOCK_UPDATE_KNOWN_SHAPE != 0 {
        input.state
    } else {
        update_from_neighbour_shapes(level, input.state, pos)
    };
    // `:68-70`: a shape update that answered air -- a torch whose support went
    // away, say -- is discarded, so the command still places the block it was
    // given.
    if block_state_is_air(state) {
        state = input.state;
    }
    state = overwrite_with_defined_properties(state, input);
    // `:74-76`.
    let mut affected = level.set_block(pos, state, flags, UPDATE_LIMIT);
    // `:78-98`: the compound the argument typed is loaded into whatever block
    // entity the write left standing, and a load that moved the object's own
    // record is a change even where the state write was not. `:79` is
    // `getBlockEntity`, whose null answer skips the leg entirely; the state it
    // is validated against is the one standing there, which a block that
    // replaced itself in `onPlace` may have made a different one.
    if let Some(NbtSummaryValue::Compound(entries)) = input.block_entity_data.as_ref() {
        affected |= load_block_entity_data(level, pos, entries, resolver, false)?;
    }
    Ok(affected)
}

fn load_block_entity_data(
    level: &mut LevelCtx<'_, '_>,
    pos: BlockPos,
    entries: &[bbb_protocol::NbtSummaryEntry],
    resolver: &dyn PersistentItemStackHolderResolver,
    force_changed: bool,
) -> Result<bool, BlockEntityCommandDataError> {
    let state = level.block_state(pos);
    let previous_automatic = level
        .chunks
        .block_entities()
        .get(pos)
        .and_then(crate::block_entity::BlockEntity::command_block)
        .map(|block| block.automatic());
    let loaded = level
        .chunks
        .block_entities_mut()
        .get_mut(pos)
        .map(|block_entity| {
            block_entity.load_command_data_in_place_with_resolver(state, entries, resolver)
        });
    let Some(loaded) = loaded else {
        return Ok(false);
    };
    let changed = match loaded {
        Ok(changed) => changed,
        Err(BlockEntityCommandDataError::InvalidBlockState { .. }) => return Ok(false),
        Err(error @ BlockEntityCommandDataError::Item(_)) => return Err(error),
    };
    crate::world::game_master_blocks::loaded::after_load(level, pos, previous_automatic);
    if changed || force_changed {
        level.chunks.block_entities_mut().set_changed(pos);
        level.changed_block_broadcasts.insert((pos.x, pos.y, pos.z));
        if !block_state_is_air(state) {
            if let Some(block) = vanilla_block_state_block_type_id(state) {
                level.update_neighbour_for_output_signal(pos, block);
            }
        }
    }
    Ok(changed)
}

fn update_from_neighbour_shapes(level: &mut LevelCtx<'_, '_>, state: i32, pos: BlockPos) -> i32 {
    let mut new_state = state;
    for direction in UPDATE_SHAPE_ORDER {
        let neighbour_pos = relative(pos, direction);
        let neighbour_state = level.block_state(neighbour_pos);
        new_state = update_block_shape(
            &mut crate::block_behavior::shape_context::BlockShapeCtx::Live(level),
            new_state,
            pos,
            direction,
            neighbour_pos,
            neighbour_state,
        );
    }
    new_state
}

///
/// The early return at `:104-106` is an identity check on the state, so a shape
/// update that changed nothing skips the whole loop -- which is why a `/fill`
/// of a block with no shape owner never touches this.
fn overwrite_with_defined_properties(state: i32, input: &CommandBlockInput) -> i32 {
    if state == input.state {
        return state;
    }
    let mut result = state;
    for property in &input.defined_properties {
        result = try_set_value(result, property, input.state);
    }
    result
}

fn try_set_value(state: i32, property: &str, source: i32) -> i32 {
    let Some(wanted) = bbb_protocol::block_state_types::vanilla_block_state_identity(source)
        .and_then(|identity| identity.properties.get(property).cloned())
    else {
        return state;
    };
    let Some(identity) = bbb_protocol::block_state_types::vanilla_block_state_identity(state)
    else {
        return state;
    };
    if !identity.properties.contains_key(property) {
        return state;
    }
    let mut properties = identity.properties.clone();
    properties.insert(property.to_string(), wanted);
    bbb_protocol::block_state_types::vanilla_block_state_id_by_name_and_properties(
        &identity.name,
        &properties,
    )
    .unwrap_or(state)
}

///
/// The three legs are the removal call for a block that actually left, the
/// neighbour fan-out for whatever stands there now, and the comparator leg for
/// a state that answers an analog signal.
fn update_neighbours_on_block_set(level: &mut LevelCtx<'_, '_>, pos: BlockPos, old_state: i32) {
    // `:843-845`.
    let state = level.block_state(pos);
    let new_block = vanilla_block_state_block_type_id(state);
    let block_changed = vanilla_block_state_block_type_id(old_state) != new_block;
    // `:846-848`. `movedByPiston` is false, which is what the call passes.
    if block_changed {
        block_affect_neighbors_after_removal(level, pos, old_state, false);
    }
    // `:850`, with the block that stands there now rather than the one that
    // left.
    if let Some(new_block) = new_block {
        level.update_neighbors_at(pos, new_block);
        // `:851-853`.
        if redstone::has_analog_output_signal(state) {
            level.update_neighbour_for_output_signal(pos, new_block);
        }
    }
}
