use bbb_world::{
    BlockPos, ClientParticleOptions, EnvironmentBiomeSampling, TerrainFluidKind, WorldStore,
};

use super::simple_client_particle_options;

const ROOT_SEARCH_LENGTH: i32 = 11;
const DRIP_Y_OFFSET: f64 = 0.25;
const MAX_HORIZONTAL_OFFSET: f32 = 0.125;

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct PointedDripstoneDripContext {
    pub(crate) particle: ClientParticleOptions,
    pub(crate) fluid_kind: Option<TerrainFluidKind>,
}

pub(crate) fn pointed_dripstone_drip_context(
    world: &WorldStore,
    tip_pos: BlockPos,
) -> Option<PointedDripstoneDripContext> {
    let tip = world.probe_block(tip_pos)?;
    if !block_probe_can_drip(&tip) {
        return None;
    }
    let root_pos = pointed_dripstone_root_pos(world, tip_pos)?;
    let above_root_pos = block_pos_above(root_pos)?;
    let above_root = world.probe_block(above_root_pos)?;

    if above_root.block_name.as_deref() == Some("minecraft:mud")
        && !world.water_evaporates_at(above_root_pos)
    {
        return Some(PointedDripstoneDripContext {
            particle: simple_client_particle_options("minecraft:dripping_dripstone_water"),
            fluid_kind: Some(TerrainFluidKind::Water),
        });
    }

    let fluid_kind = above_root.fluid.map(|fluid| fluid.kind);
    let particle = match fluid_kind {
        Some(TerrainFluidKind::Lava) => {
            simple_client_particle_options("minecraft:dripping_dripstone_lava")
        }
        Some(TerrainFluidKind::Water) => {
            simple_client_particle_options("minecraft:dripping_dripstone_water")
        }
        None => default_dripstone_particle(world, above_root_pos),
    };
    Some(PointedDripstoneDripContext {
        particle,
        fluid_kind,
    })
}

pub(crate) fn pointed_dripstone_particle_position(pos: BlockPos) -> [f64; 3] {
    let seed = java_block_position_seed(pos.x, 0, pos.z);
    let x_offset = ((((seed & 15) as f32) / 15.0 - 0.5) * 0.5)
        .clamp(-MAX_HORIZONTAL_OFFSET, MAX_HORIZONTAL_OFFSET);
    let z_offset = ((((seed >> 8 & 15) as f32) / 15.0 - 0.5) * 0.5)
        .clamp(-MAX_HORIZONTAL_OFFSET, MAX_HORIZONTAL_OFFSET);
    [
        f64::from(pos.x) + 0.5 + f64::from(x_offset),
        f64::from(pos.y) + DRIP_Y_OFFSET,
        f64::from(pos.z) + 0.5 + f64::from(z_offset),
    ]
}

fn pointed_dripstone_root_pos(world: &WorldStore, tip_pos: BlockPos) -> Option<BlockPos> {
    for step in 1..ROOT_SEARCH_LENGTH {
        let pos = BlockPos {
            x: tip_pos.x,
            y: tip_pos.y.checked_add(step)?,
            z: tip_pos.z,
        };
        let probe = world.probe_block(pos)?;
        if probe.block_name.as_deref() != Some("minecraft:pointed_dripstone") {
            return Some(pos);
        }
        if !block_probe_is_down_pointed_dripstone(&probe) {
            return None;
        }
    }
    None
}

fn block_probe_can_drip(probe: &bbb_world::BlockProbe) -> bool {
    block_probe_is_down_pointed_dripstone(probe)
        && probe.block_properties.get("thickness").map(String::as_str) == Some("tip")
        && probe
            .block_properties
            .get("waterlogged")
            .map(String::as_str)
            == Some("false")
}

fn block_probe_is_down_pointed_dripstone(probe: &bbb_world::BlockProbe) -> bool {
    probe.block_name.as_deref() == Some("minecraft:pointed_dripstone")
        && probe
            .block_properties
            .get("vertical_direction")
            .map(String::as_str)
            == Some("down")
}

fn block_pos_above(pos: BlockPos) -> Option<BlockPos> {
    Some(BlockPos {
        x: pos.x,
        y: pos.y.checked_add(1)?,
        z: pos.z,
    })
}

fn default_dripstone_particle(world: &WorldStore, position: BlockPos) -> ClientParticleOptions {
    if let Some(particle) = world
        .sampled_client_environment_attributes_at(
            [
                f64::from(position.x),
                f64::from(position.y),
                f64::from(position.z),
            ],
            EnvironmentBiomeSampling::Direct,
        )
        .ok()
        .and_then(|attributes| attributes.default_dripstone_particle)
    {
        return particle;
    }
    if world
        .registry_content(bbb_world::DIMENSION_TYPE_REGISTRY)
        .is_some()
    {
        return simple_client_particle_options("minecraft:dripping_dripstone_water");
    }
    if world.water_evaporates_at(position) {
        simple_client_particle_options("minecraft:dripping_dripstone_lava")
    } else {
        simple_client_particle_options("minecraft:dripping_dripstone_water")
    }
}

fn java_block_position_seed(x: i32, y: i32, z: i32) -> i64 {
    let seed = i64::from(x.wrapping_mul(3_129_871))
        ^ i64::from(z).wrapping_mul(116_129_781)
        ^ i64::from(y);
    seed.wrapping_mul(seed)
        .wrapping_mul(42_317_861)
        .wrapping_add(seed.wrapping_mul(11))
        >> 16
}
