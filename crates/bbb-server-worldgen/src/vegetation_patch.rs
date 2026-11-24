//! Vanilla `VegetationPatchFeature` (`Feature.VEGETATION_PATCH`,
//! `Feature.java:86-88`) and its `WaterloggedVegetationPatchFeature` subclass
//! (`Feature.java:89-91`): a ground patch under a cave surface with a nested
//! placed feature scattered over it.

use bbb_protocol::block_state_types::{
    vanilla_block_state_block_type_id, vanilla_block_state_identity,
};
use bbb_protocol::packets::{BlockPos, Direction};
use bbb_registry_codec::random_source::RandomSource;
use bbb_server_datapack::worldgen::VegetationPatchConfig;

use super::multiface::is_face_sturdy_at;
use super::placed_feature::place_placed_feature_holder;
use super::state_providers::BlockStateProviderExt;
use super::tree::block_pos_set::BlockPosSet;
use super::value_providers::sample_int;
use super::{
    block_has_property, block_set_bool, catalog_default_state, direction_opposite, is_empty_block,
    offset_block_pos, WorldGenLevel, UPDATE_CLIENTS,
};

/// Which `placeGroundPatch` / `placeVegetation` overrides run: the base class
/// or `WaterloggedVegetationPatchFeature` (`WaterloggedVegetationPatchFeature.java:17`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PatchVariant {
    Plain,
    Waterlogged,
}

/// Vanilla `VegetationPatchFeature.place` (`VegetationPatchFeature.java:22-33`).
pub fn place_vegetation_patch<L: WorldGenLevel>(
    config: &VegetationPatchConfig,
    level: &mut L,
    origin: BlockPos,
    random: &mut dyn RandomSource,
) -> Option<bool> {
    place_patch(config, level, origin, random, PatchVariant::Plain)
}

/// Vanilla `WaterloggedVegetationPatchFeature` through the inherited
/// `VegetationPatchFeature.place` (`VegetationPatchFeature.java:22-33`).
pub fn place_waterlogged_vegetation_patch<L: WorldGenLevel>(
    config: &VegetationPatchConfig,
    level: &mut L,
    origin: BlockPos,
    random: &mut dyn RandomSource,
) -> Option<bool> {
    place_patch(config, level, origin, random, PatchVariant::Waterlogged)
}

fn place_patch<L: WorldGenLevel>(
    config: &VegetationPatchConfig,
    level: &mut L,
    origin: BlockPos,
    random: &mut dyn RandomSource,
    variant: PatchVariant,
) -> Option<bool> {
    let x_radius = sample_int(&config.xz_radius, random) + 1;
    let z_radius = sample_int(&config.xz_radius, random) + 1;
    let surface = place_ground_patch(level, config, random, origin, x_radius, z_radius)?;
    let surface = match variant {
        PatchVariant::Plain => surface,
        PatchVariant::Waterlogged => waterlogged_ground_patch(level, surface),
    };
    distribute_vegetation(level, config, random, &surface, variant)?;
    Some(!surface.is_empty())
}

/// Vanilla `WaterloggedVegetationPatchFeature.placeGroundPatch`
/// (`WaterloggedVegetationPatchFeature.java:23-47`) after the base patch.
fn waterlogged_ground_patch<L: WorldGenLevel>(level: &mut L, surface: BlockPosSet) -> BlockPosSet {
    let mut water_surface = BlockPosSet::new();
    for surface_pos in surface.iter() {
        if !is_exposed(level, surface_pos) {
            water_surface.add(surface_pos);
        }
    }
    let water = catalog_default_state("minecraft:water");
    for surface_pos in water_surface.iter() {
        let _ = level.set_block(surface_pos, water, UPDATE_CLIENTS);
    }
    water_surface
}

/// Vanilla `WaterloggedVegetationPatchFeature.isExposed` (`:49-55`) over
/// `isExposedDirection` (`:57-60`).
fn is_exposed<L: WorldGenLevel>(level: &L, pos: BlockPos) -> bool {
    for direction in [
        Direction::North,
        Direction::East,
        Direction::South,
        Direction::West,
        Direction::Down,
    ] {
        let test_pos = offset_block_pos(pos, direction);
        if !is_face_sturdy_at(
            level,
            test_pos,
            level.get_block(test_pos),
            direction_opposite(direction),
        ) {
            return true;
        }
    }
    false
}

/// Vanilla `VegetationPatchFeature.placeGroundPatch` (`VegetationPatchFeature.java:35-85`).
pub fn place_ground_patch<L: WorldGenLevel>(
    level: &mut L,
    config: &VegetationPatchConfig,
    random: &mut dyn RandomSource,
    origin: BlockPos,
    x_radius: i32,
    z_radius: i32,
) -> Option<BlockPosSet> {
    let inwards = config.surface;
    let outwards = direction_opposite(inwards);
    let mut surface = BlockPosSet::new();
    for dx in -x_radius..=x_radius {
        let is_x_edge = dx == -x_radius || dx == x_radius;
        for dz in -z_radius..=z_radius {
            let is_z_edge = dz == -z_radius || dz == z_radius;
            let is_edge = is_x_edge || is_z_edge;
            let is_corner = is_x_edge && is_z_edge;
            let is_edge_but_not_corner = is_edge && !is_corner;
            if !is_corner
                && (!is_edge_but_not_corner
                    || (config.extra_edge_column_chance != 0.0
                        && random.next_float() <= config.extra_edge_column_chance))
            {
                let mut pos = BlockPos {
                    x: origin.x + dx,
                    y: origin.y,
                    z: origin.z + dz,
                };
                let mut offset = 0;
                while is_empty_block(level.get_block(pos)) && offset < config.vertical_range {
                    pos = offset_block_pos(pos, inwards);
                    offset += 1;
                }
                let mut steps = 0;
                while !is_empty_block(level.get_block(pos)) && steps < config.vertical_range {
                    pos = offset_block_pos(pos, outwards);
                    steps += 1;
                }
                let below_pos = offset_block_pos(pos, config.surface);
                let below_state = level.get_block(below_pos);
                if is_empty_block(level.get_block(pos))
                    && is_face_sturdy_at(
                        level,
                        below_pos,
                        below_state,
                        direction_opposite(config.surface),
                    )
                {
                    let depth = sample_int(&config.depth, random)
                        + if config.extra_bottom_block_chance > 0.0
                            && random.next_float() < config.extra_bottom_block_chance
                        {
                            1
                        } else {
                            0
                        };
                    let ground_pos = below_pos;
                    let ground_placed = place_ground(level, config, random, below_pos, depth)?;
                    if ground_placed {
                        surface.add(ground_pos);
                    }
                }
            }
        }
    }
    Some(surface)
}

/// Vanilla `VegetationPatchFeature.distributeVegetation`
/// (`VegetationPatchFeature.java:87-101`) with `placeVegetation` (`:103-111`).
fn distribute_vegetation<L: WorldGenLevel>(
    level: &mut L,
    config: &VegetationPatchConfig,
    random: &mut dyn RandomSource,
    surface: &BlockPosSet,
    variant: PatchVariant,
) -> Option<()> {
    for surface_pos in surface.iter() {
        if config.vegetation_chance > 0.0 && random.next_float() < config.vegetation_chance {
            match variant {
                PatchVariant::Plain => {
                    place_vegetation(level, config, random, surface_pos);
                }
                PatchVariant::Waterlogged => {
                    waterlogged_place_vegetation(level, config, random, surface_pos)?;
                }
            }
        }
    }
    Some(())
}

/// Vanilla `WaterloggedVegetationPatchFeature.placeVegetation`
/// (`WaterloggedVegetationPatchFeature.java:63-80`).
fn waterlogged_place_vegetation<L: WorldGenLevel>(
    level: &mut L,
    config: &VegetationPatchConfig,
    random: &mut dyn RandomSource,
    placement_pos: BlockPos,
) -> Option<bool> {
    if place_vegetation(
        level,
        config,
        random,
        offset_block_pos(placement_pos, Direction::Down),
    ) {
        let placed = level.get_block(placement_pos);
        if block_has_property(placed, "waterlogged") && !is_waterlogged(placed) {
            let _ = level.set_block(
                placement_pos,
                block_set_bool(placed, "waterlogged", true),
                UPDATE_CLIENTS,
            );
        }
        Some(true)
    } else {
        Some(false)
    }
}

/// Vanilla `VegetationPatchFeature.placeVegetation` (`VegetationPatchFeature.java:103-111`).
pub fn place_vegetation<L: WorldGenLevel>(
    level: &mut L,
    config: &VegetationPatchConfig,
    random: &mut dyn RandomSource,
    vegetation_pos: BlockPos,
) -> bool {
    place_placed_feature_holder(
        &config.vegetation_feature,
        level,
        random,
        offset_block_pos(vegetation_pos, direction_opposite(config.surface)),
    )
}

/// Vanilla `VegetationPatchFeature.placeGround` (`VegetationPatchFeature.java:113-135`).
fn place_ground<L: WorldGenLevel>(
    level: &mut L,
    config: &VegetationPatchConfig,
    random: &mut dyn RandomSource,
    mut below_pos: BlockPos,
    depth: i32,
) -> Option<bool> {
    for i in 0..depth {
        let state_to_place = config.ground_state.get_state(level, random, below_pos)?;
        let below_state = level.get_block(below_pos);
        if vanilla_block_state_block_type_id(state_to_place)
            != vanilla_block_state_block_type_id(below_state)
        {
            if !level.block_state_in_tag(below_state, &config.replaceable) {
                return Some(i != 0);
            }
            let _ = level.set_block(below_pos, state_to_place, UPDATE_CLIENTS);
            below_pos = offset_block_pos(below_pos, config.surface);
        }
    }
    Some(true)
}

fn is_waterlogged(state: i32) -> bool {
    vanilla_block_state_identity(state).is_some_and(|identity| {
        identity
            .properties
            .get("waterlogged")
            .is_some_and(|value| value == "true")
    })
}
