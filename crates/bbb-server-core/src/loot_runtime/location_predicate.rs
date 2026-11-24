//! `LocationPredicate.matches` over the one loaded block-state query.

use std::borrow::Cow;

use bbb_protocol::block_state_types::{
    vanilla_block_property_value_rank, vanilla_block_state_identity,
    vanilla_block_state_static_collision_shape, VanillaBlockPropertyKind,
};
use bbb_protocol::packets::BlockPos;
use bbb_protocol::PersistentRegistryTagMembers;
use bbb_registry_codec::PersistentItemStackHolderResolver;
use bbb_server_datapack::loot::{
    BlockPredicate, FluidPredicate, HolderSet, LocationPredicate, StatePropertiesPredicate,
};
use bbb_server_types::fluid::{block_state_fluid_state, FluidState, AMOUNT_FULL};

use super::{
    matches_state_properties, unsupported_field, LootBiomeAt, LootBlockEntityData,
    LootBlockEntityDataAt, LootBlockEntityRead, LootBlockStateAt, LootItemEnvironment, LootLightAt,
    LootRuntimeReason, LootSkyAt, LootStructuresAt,
};

/// Every term this crate cannot answer, refused before a table is drawn: the
/// runtime consults a term only where vanilla does, so one behind a false term
/// would otherwise never be seen.
pub fn check_location_predicate(
    predicate: &LocationPredicate,
) -> std::result::Result<(), LootRuntimeReason> {
    if let Some(block) = &predicate.block {
        super::item_predicate::check_component_predicates(&block.components, &block.predicates)?;
    }
    Ok(())
}

/// `LocationPredicate.matches` at the position `LocationCheck` offsets the
/// origin to, with no biome view.
#[cfg(test)]
pub fn location_predicate_matches(
    predicate: &LocationPredicate,
    x: f64,
    y: f64,
    z: f64,
    dimension: Option<&str>,
    level: Option<&LootBlockStateAt<'_>>,
    resolver: Option<&dyn PersistentItemStackHolderResolver>,
) -> std::result::Result<bool, LootRuntimeReason> {
    location_predicate_matches_in_world(
        predicate,
        [x, y, z],
        dimension,
        level,
        None,
        None,
        None,
        None,
        None,
        resolver,
        None,
    )
}

// Multi-arg Vanilla `LocationPredicate.matches` kernel over independent world-query closures.
#[allow(clippy::too_many_arguments)]
pub(crate) fn location_predicate_matches_in_world(
    predicate: &LocationPredicate,
    [x, y, z]: [f64; 3],
    dimension: Option<&str>,
    level: Option<&LootBlockStateAt<'_>>,
    block_entity_data: Option<&LootBlockEntityDataAt<'_>>,
    biome: Option<&LootBiomeAt<'_>>,
    sky: Option<&LootSkyAt<'_>>,
    light: Option<&LootLightAt<'_>>,
    structures: Option<&LootStructuresAt<'_>>,
    resolver: Option<&dyn PersistentItemStackHolderResolver>,
    environment: Option<&dyn LootItemEnvironment>,
) -> std::result::Result<bool, LootRuntimeReason> {
    if let Some([bx, by, bz]) = &predicate.position {
        if !(bx.matches(x) && by.matches(y) && bz.matches(z)) {
            return Ok(false);
        }
    }
    if let Some(expected) = predicate.dimension.as_deref() {
        if dimension != Some(expected) {
            return Ok(false);
        }
    }
    let reads_level = predicate.biomes.is_some()
        || predicate.structures.is_some()
        || predicate.smokey.is_some()
        || predicate.light.is_some()
        || predicate.block.is_some()
        || predicate.fluid.is_some()
        || predicate.can_see_sky.is_some();
    if !reads_level {
        return Ok(true);
    }
    let level = level.ok_or(LootRuntimeReason::NoLevelView)?;
    // The level read folds `isLoaded` into the state it answers, so an absent
    // state is the unloaded position.
    let pos = BlockPos {
        x: x.floor() as i32,
        y: y.floor() as i32,
        z: z.floor() as i32,
    };
    let state_id = level(pos);
    // An unloaded position fails every level term before it is read.
    let Some(state_id) = state_id else {
        return Ok(false);
    };
    if let Some(biomes) = &predicate.biomes {
        let callback_biome;
        let actual = if let Some(read) = biome {
            callback_biome = read(pos);
            callback_biome.as_deref()
        } else {
            environment
                .ok_or_else(|| unreachable("biomes"))?
                .biome_at(pos)?
        };
        let Some(biome) = actual else {
            return Ok(false);
        };
        if !biomes.contains(biome, |tag| {
            resolver
                .and_then(|resolver| resolver.tag_contains("minecraft:worldgen/biome", tag, biome))
                .ok_or_else(|| unreachable("biomes"))
        })? {
            return Ok(false);
        }
    }
    if let Some(expected) = &predicate.structures {
        let matches = match structures {
            Some(read) => read(pos, expected),
            None => environment
                .ok_or_else(|| unreachable("structures"))?
                .structure_matches(pos, expected)?,
        }
        .ok_or_else(|| unreachable("structures"))?;
        if !matches {
            return Ok(false);
        }
    }
    if let Some(smokey) = predicate.smokey {
        if smokey != is_smokey_pos(level, pos, resolver)? {
            return Ok(false);
        }
    }
    if let Some(bounds) = &predicate.light {
        let brightness = match light {
            Some(read) => read(pos),
            None => environment
                .ok_or_else(|| unreachable("light"))?
                .light_at(pos)?,
        }
        .ok_or_else(|| unreachable("light"))?;
        if !bounds.matches(brightness) {
            return Ok(false);
        }
    }
    if let Some(block) = &predicate.block {
        if !block_matches(block, state_id, resolver)? {
            return Ok(false);
        }
        if let Some(expected) = &block.nbt {
            let read = block_entity_data.ok_or_else(|| {
                unsupported_field(
                    "location_check",
                    "predicate.block.nbt needs block-entity data",
                )
            })?;
            let actual = match read(LootBlockEntityRead::Nbt(pos))? {
                Some(LootBlockEntityData::Nbt(value)) => Some(value),
                _ => None,
            };
            if !bbb_server_types::block_state_parse::block_entity_data_matches(
                Some(expected),
                actual.as_ref(),
            ) {
                return Ok(false);
            }
        }
        if !block.components.is_empty() || !block.predicates.is_empty() {
            let read = block_entity_data.ok_or_else(|| {
                unsupported_field(
                    "location_check",
                    "predicate.block needs block-entity components",
                )
            })?;
            let Some(LootBlockEntityData::Components(actual)) =
                read(LootBlockEntityRead::Components(pos))?
            else {
                return Ok(false);
            };
            let resolver = resolver.ok_or_else(|| {
                unsupported_field(
                    "location_check",
                    "block components need their registry publication",
                )
            })?;
            let get = |type_id| {
                let component =
                    bbb_protocol::data_component_types::vanilla_data_component_type(type_id)
                        .ok_or_else(|| unsupported_field("location_check", "unknown component"))?;
                match actual.get(component.resource_id) {
                    None => Ok(None),
                    Some(Err(error)) => Err(LootRuntimeReason::Catalog(error.clone())),
                    Some(Ok(value)) => {
                        bbb_registry_codec::read_item_component(type_id, value, resolver)
                            .map(|value| Some(Cow::Owned(value)))
                            .map_err(|error| LootRuntimeReason::Catalog(error.to_string()))
                    }
                }
            };
            let has = |type_id| {
                Ok(
                    bbb_protocol::data_component_types::vanilla_data_component_type(type_id)
                        .is_some_and(|component| actual.get(component.resource_id).is_some()),
                )
            };
            let reads = super::item_predicate::ComponentReads {
                get: &get,
                has: &has,
            };
            if !super::item_predicate::component_predicates_match(
                &block.components,
                &block.predicates,
                &reads,
                None,
                resolver,
            )? {
                return Ok(false);
            }
        }
    }
    if let Some(fluid) = &predicate.fluid {
        if !fluid_matches(fluid, block_state_fluid_state(state_id), resolver)? {
            return Ok(false);
        }
    }
    if let Some(expected) = predicate.can_see_sky {
        let actual = match sky {
            Some(read) => read(pos),
            None => environment
                .ok_or_else(|| unreachable("can_see_sky"))?
                .sky_at(pos)?,
        };
        if actual != Some(expected) {
            return Ok(false);
        }
    }
    Ok(true)
}

fn unreachable(term: &str) -> LootRuntimeReason {
    let needs = match term {
        "biomes" => "the biome registry that names the biome at a position",
        "structures" => "the level's structure manager",
        "light" => "the level's max local raw brightness",
        _ => "whether the level's sky is visible at a position",
    };
    unsupported_field(
        "location_check",
        format!("predicate.{term}, which needs {needs}"),
    )
}

fn is_smokey_pos(
    level: &LootBlockStateAt<'_>,
    pos: BlockPos,
    resolver: Option<&dyn PersistentItemStackHolderResolver>,
) -> std::result::Result<bool, LootRuntimeReason> {
    crate::block_behavior::campfire::is_smokey_pos(
        pos,
        |pos| column_state(level, pos),
        |state, _| smoke_blocked(state),
        |name| block_tag_contains("minecraft:campfires", name, resolver),
    )
}

/// The walk reads `Level.getBlockState`, which vanilla answers for any position
/// by loading the column; the read here has only the loaded half, so a column
/// it cannot show is a refusal rather than an assumed air.
fn column_state(
    level: &LootBlockStateAt<'_>,
    pos: BlockPos,
) -> std::result::Result<i32, LootRuntimeReason> {
    level(pos).ok_or_else(|| {
        unsupported_field(
            "location_check",
            format!(
                "predicate.smokey reads {} {} {}, which this level does not show",
                pos.x, pos.y, pos.z
            ),
        )
    })
}

/// The `Shapes.joinIsNotEmpty(SHAPE_VIRTUAL_POST, .., AND)` half of the walk
/// (`CampfireBlock.java:268`): the join is non-empty exactly when some
/// collision box overlaps the post on all three axes.
///
/// Vanilla evaluates the shape of the state below against the *original*
/// position, so the six dynamic-shape blocks would need a context this read has
/// no view of and are refused rather than sampled in an empty world.
fn smoke_blocked(state_id: i32) -> std::result::Result<bool, LootRuntimeReason> {
    let Some(shape) = vanilla_block_state_static_collision_shape(state_id) else {
        return Err(unsupported_field(
            "location_check",
            format!("predicate.smokey reads the dynamic collision shape of block state {state_id}"),
        ));
    };
    Ok(shape.boxes().any(|bounds| {
        crate::block_behavior::campfire::smoke_post_intersects(bbb_server_types::geometry::Aabb {
            min_x: bounds.min_x,
            min_y: bounds.min_y,
            min_z: bounds.min_z,
            max_x: bounds.max_x,
            max_y: bounds.max_y,
            max_z: bounds.max_z,
        })
    }))
}

/// The holder and state terms of `BlockPredicate.matches`.
fn block_matches(
    block: &BlockPredicate,
    state_id: i32,
    resolver: Option<&dyn PersistentItemStackHolderResolver>,
) -> std::result::Result<bool, LootRuntimeReason> {
    let state = vanilla_block_state_identity(state_id)
        .ok_or(LootRuntimeReason::UnknownBlockState(state_id))?;
    if let Some(blocks) = &block.blocks {
        if !block_holder_set_contains(blocks, &state.name, resolver)? {
            return Ok(false);
        }
    }
    match &block.state {
        None => Ok(true),
        Some(properties) => matches_state_properties(properties, state),
    }
}

pub(super) fn block_holder_set_contains(
    holder_set: &HolderSet,
    block: &str,
    resolver: Option<&dyn PersistentItemStackHolderResolver>,
) -> std::result::Result<bool, LootRuntimeReason> {
    holder_set.contains(block, |tag| block_tag_contains(tag, block, resolver))
}

fn block_tag_contains(
    tag_id: &str,
    block: &str,
    resolver: Option<&dyn PersistentItemStackHolderResolver>,
) -> std::result::Result<bool, LootRuntimeReason> {
    resolver
        .and_then(|resolver| resolver.tag_contains("minecraft:block", tag_id, block))
        .ok_or_else(|| {
            unsupported_field(
                "block predicate",
                format!("block tag #{tag_id} needs the current registry publication"),
            )
        })
}

/// `FluidPredicate.matches` once the position is loaded: the fluid state is
/// the block state's own.
fn fluid_matches(
    fluid: &FluidPredicate,
    state: FluidState,
    resolver: Option<&dyn PersistentItemStackHolderResolver>,
) -> std::result::Result<bool, LootRuntimeReason> {
    let id = state.identity().resource_id();
    if let Some(fluids) = &fluid.fluids {
        if !fluids.contains(id, |tag| fluid_tag_contains(tag, id, resolver))? {
            return Ok(false);
        }
    }
    match &fluid.state {
        None => Ok(true),
        Some(properties) => fluid_properties_match(properties, state),
    }
}

fn fluid_tag_contains(
    tag_id: &str,
    fluid: &str,
    resolver: Option<&dyn PersistentItemStackHolderResolver>,
) -> std::result::Result<bool, LootRuntimeReason> {
    let members = resolver.map_or(PersistentRegistryTagMembers::Unavailable, |resolver| {
        resolver.tag_members("minecraft:fluid", tag_id)
    });
    match members {
        PersistentRegistryTagMembers::Present(members) => {
            Ok(members.iter().any(|member| member == fluid))
        }
        PersistentRegistryTagMembers::Absent => Ok(false),
        PersistentRegistryTagMembers::Unavailable => Err(unsupported_field(
            "fluid predicate",
            format!("fluid tag #{tag_id} needs the current registry publication"),
        )),
    }
}

/// `StatePropertiesPredicate.matches` against a fluid state: flowing fluids
/// declare `falling` and `level`; any other name is a null property that
/// fails.
fn fluid_properties_match(
    properties: &StatePropertiesPredicate,
    state: FluidState,
) -> std::result::Result<bool, LootRuntimeReason> {
    for (name, matcher) in &properties.0 {
        let (actual, kind) = match (name, state) {
            (name, FluidState::Source { falling, .. } | FluidState::Flowing { falling, .. })
                if name == "falling" =>
            {
                (i32::from(falling), VanillaBlockPropertyKind::Boolean)
            }
            (name, FluidState::Flowing { amount, .. }) if name == "level" => (
                i32::from(amount),
                // LEVEL_FLOWING (BlockStateProperties.java:113).
                VanillaBlockPropertyKind::Integer {
                    min: 1,
                    max: AMOUNT_FULL,
                },
            ),
            _ => return Ok(false),
        };
        if !super::state_properties::matches_property_value(matcher, actual, |value| {
            vanilla_block_property_value_rank(&kind, value)
        }) {
            return Ok(false);
        }
    }
    Ok(true)
}

#[cfg(test)]
mod tests;
