use std::time::{Duration, Instant};

use bbb_renderer::terrain::{
    build_terrain_mesh_layers_with_atlas_and_camera, TerrainCardinalLighting, TerrainCell,
    TerrainChunkSnapshot, TerrainFluid, TerrainFluidKind, TerrainLight, TerrainMaterialClass,
    TerrainTint,
};
use bbb_world::{ChunkPos, WorldCardinalLighting, WorldStore};

use crate::biome_tint::{
    block_wants_biome_blend, BiomeBlend, BIOME_BLEND_DIAMETER, BIOME_BLEND_RADIUS,
    BIOME_BLEND_SAMPLES,
};
use crate::camera_pose::camera_pose_from_world;

mod textures;
pub(crate) use textures::{
    load_terrain_textures, BlockRenderPosition, TerrainParticleTintCatalog, TerrainTextureState,
};

const MAX_TERRAIN_UPLOAD_CHUNKS: usize = 49;
const TERRAIN_TEXTURE_ANIMATION_INTERVAL: Duration = Duration::from_millis(50);

#[derive(Debug, Default)]
pub(crate) struct TerrainUploadState {
    decoded_chunks: usize,
    block_updates_applied: usize,
    light_updates_applied: usize,
    biome_updates_applied: usize,
    uploaded_chunks: usize,
    observed_decoded_chunks: usize,
    observed_block_updates_applied: usize,
    observed_light_updates_applied: usize,
    observed_biome_updates_applied: usize,
    last_observed_change: Option<Instant>,
    texture_animation_tick: u64,
    last_texture_animation_at: Option<Instant>,
}

impl TerrainUploadState {
    pub(crate) fn has_uploaded_chunks(&self) -> bool {
        self.uploaded_chunks > 0
    }
}

pub(crate) fn maybe_upload_terrain_texture_animation(
    renderer: &mut bbb_renderer::Renderer,
    upload: &mut TerrainUploadState,
    textures: &TerrainTextureState,
) {
    if !textures.has_texture_animation() {
        return;
    }
    let Some(tick) = advance_texture_animation_tick(upload, Instant::now()) else {
        return;
    };
    match textures.animation_atlas_frame(tick) {
        Ok(Some(atlas)) => {
            let mip_rgba = atlas.rgba_slices();
            if let Err(err) = renderer.update_terrain_texture_atlas_mips(&mip_rgba) {
                tracing::warn!(?err, "failed to update animated terrain texture atlas");
            }
        }
        Ok(None) => {}
        Err(err) => {
            tracing::warn!(
                ?err,
                "failed to stitch animated terrain texture atlas frame"
            );
        }
    }
}

fn advance_texture_animation_tick(upload: &mut TerrainUploadState, now: Instant) -> Option<u64> {
    let Some(last) = upload.last_texture_animation_at else {
        upload.last_texture_animation_at = Some(now);
        return None;
    };
    let elapsed = now.saturating_duration_since(last);
    let ticks = elapsed.as_millis() / TERRAIN_TEXTURE_ANIMATION_INTERVAL.as_millis();
    if ticks == 0 {
        return None;
    }

    let ticks = u64::try_from(ticks).unwrap_or(u64::MAX);
    upload.texture_animation_tick = upload.texture_animation_tick.saturating_add(ticks);
    let advanced = Duration::from_millis(
        ticks.saturating_mul(TERRAIN_TEXTURE_ANIMATION_INTERVAL.as_millis() as u64),
    );
    upload.last_texture_animation_at = last.checked_add(advanced).or(Some(now));
    Some(upload.texture_animation_tick)
}

pub(crate) fn maybe_upload_decoded_terrain(
    world: &WorldStore,
    renderer: &mut bbb_renderer::Renderer,
    upload: &mut TerrainUploadState,
    textures: &TerrainTextureState,
) {
    let world_counters = world.counters();
    let chunk_count = world.chunk_count();
    if chunk_count == 0
        || (upload.decoded_chunks == world_counters.chunks_decoded
            && upload.block_updates_applied == world_counters.block_updates_applied
            && upload.light_updates_applied == world_counters.light_updates_applied
            && upload.biome_updates_applied == world_counters.biome_updates_applied
            && upload.uploaded_chunks == chunk_count)
    {
        return;
    }
    if upload.observed_decoded_chunks != world_counters.chunks_decoded
        || upload.observed_block_updates_applied != world_counters.block_updates_applied
        || upload.observed_light_updates_applied != world_counters.light_updates_applied
        || upload.observed_biome_updates_applied != world_counters.biome_updates_applied
    {
        upload.observed_decoded_chunks = world_counters.chunks_decoded;
        upload.observed_block_updates_applied = world_counters.block_updates_applied;
        upload.observed_light_updates_applied = world_counters.light_updates_applied;
        upload.observed_biome_updates_applied = world_counters.biome_updates_applied;
        upload.last_observed_change = Some(Instant::now());
        return;
    }
    if upload
        .last_observed_change
        .is_some_and(|changed_at| changed_at.elapsed() < Duration::from_millis(250))
    {
        return;
    }

    let center = terrain_upload_center(world);
    let mut positions = world.chunk_positions();
    positions.sort_by_key(|pos| chunk_distance_key(*pos, center));

    let mut snapshots: Vec<_> = positions
        .into_iter()
        .take(MAX_TERRAIN_UPLOAD_CHUNKS)
        .filter_map(|pos| world.extract_terrain_chunk(pos))
        .collect();
    if snapshots.is_empty() {
        return;
    }

    snapshots.sort_by_key(|snapshot| chunk_distance_key(snapshot.pos, center));
    let cardinal_lighting = terrain_cardinal_lighting(world);
    let renderer_snapshots: Vec<_> = snapshots
        .into_iter()
        .map(|snapshot| {
            let biome_sampler = world.chunk_biome_sampler(snapshot.pos);
            convert_terrain_snapshot(snapshot, textures, cardinal_lighting, &biome_sampler)
        })
        .collect();
    let camera_position = camera_pose_from_world(world)
        .map(|pose| {
            [
                pose.position[0],
                pose.position[1] + pose.eye_height,
                pose.position[2],
            ]
        })
        .unwrap_or([0.0, 0.0, 0.0]);
    let meshes = build_terrain_mesh_layers_with_atlas_and_camera(
        &renderer_snapshots,
        &textures.atlas,
        camera_position,
    );

    renderer.upload_terrain_mesh_layers(meshes);
    upload.decoded_chunks = world_counters.chunks_decoded;
    upload.block_updates_applied = world_counters.block_updates_applied;
    upload.light_updates_applied = world_counters.light_updates_applied;
    upload.biome_updates_applied = world_counters.biome_updates_applied;
    upload.uploaded_chunks = chunk_count;
}

fn terrain_upload_center(world: &WorldStore) -> ChunkPos {
    world
        .chunk_cache_center()
        .or(world.first_chunk())
        .unwrap_or_else(|| {
            world
                .chunk_positions()
                .into_iter()
                .next()
                .unwrap_or(ChunkPos { x: 0, z: 0 })
        })
}

fn chunk_distance_key(pos: ChunkPos, center: ChunkPos) -> i64 {
    let dx = i64::from(pos.x - center.x);
    let dz = i64::from(pos.z - center.z);
    dx * dx + dz * dz
}

fn terrain_cardinal_lighting(world: &WorldStore) -> TerrainCardinalLighting {
    match world.level_info().map(|level| level.cardinal_lighting()) {
        Some(WorldCardinalLighting::Nether) => TerrainCardinalLighting::Nether,
        _ => TerrainCardinalLighting::Default,
    }
}

fn convert_terrain_snapshot(
    snapshot: bbb_world::TerrainChunkSnapshot,
    textures: &TerrainTextureState,
    cardinal_lighting: TerrainCardinalLighting,
    biome_sampler: &bbb_world::ChunkBiomeSampler,
) -> TerrainChunkSnapshot {
    let chunk_origin_x = snapshot.pos.x * 16;
    let chunk_origin_z = snapshot.pos.z * 16;
    let cells = snapshot
        .cells
        .into_iter()
        .enumerate()
        .map(|(index, cell)| {
            let local_x = (index % 16) as i32;
            let local_y = (index / (16 * 16)) as i32;
            let local_z = ((index / 16) % 16) as i32;
            let position = BlockRenderPosition {
                x: chunk_origin_x + local_x,
                y: snapshot.min_y + local_y,
                z: chunk_origin_z + local_z,
            };
            let world_material = cell.material;
            let has_water_fluid = cell
                .fluid
                .is_some_and(|fluid| matches!(fluid.kind, bbb_world::TerrainFluidKind::Water));
            // Only biome-resolver blocks (grass / foliage / dry-foliage / water)
            // read the blend window, so we skip building the 5x5 sample grid for
            // the interior stone/dirt/air majority of a chunk.
            let wants_blend = cell
                .block_name
                .as_deref()
                .map(|name| block_wants_biome_blend(name, has_water_fluid))
                .unwrap_or(has_water_fluid);
            let blend = wants_blend.then(|| build_biome_blend(biome_sampler, position));
            let (texture_indices, tint, face_transparency, render_shape, ambient_occlusion) =
                textures.block_render_data(
                    cell.block_name.as_deref(),
                    &cell.block_properties,
                    world_material,
                    cell.biome_id,
                    Some(position),
                    blend.as_ref(),
                );
            let fluid = cell.fluid.map(renderer_fluid);
            let (fluid_texture_indices, fluid_tint) = fluid
                .map(|fluid| {
                    textures.fluid_render_data(
                        fluid.kind,
                        cell.biome_id,
                        Some(position),
                        blend.as_ref(),
                    )
                })
                .unwrap_or(([0; 6], [TerrainTint::WHITE; 6]));
            let fluid_overlay_texture_index =
                fluid.and_then(|fluid| textures.fluid_overlay_texture_index(fluid.kind));
            TerrainCell {
                block_state_id: cell.block_state_id,
                fluid,
                fluid_texture_indices,
                fluid_overlay_texture_index,
                fluid_overlay_neighbor: block_uses_fluid_water_overlay(cell.block_name.as_deref()),
                fluid_tint,
                texture_indices,
                render_shape,
                ambient_occlusion,
                material: match cell.material {
                    bbb_world::TerrainMaterialClass::Empty => TerrainMaterialClass::Empty,
                    bbb_world::TerrainMaterialClass::Invisible => TerrainMaterialClass::Empty,
                    bbb_world::TerrainMaterialClass::Opaque => TerrainMaterialClass::Opaque,
                    bbb_world::TerrainMaterialClass::Cutout => TerrainMaterialClass::Cutout,
                    bbb_world::TerrainMaterialClass::Fluid => TerrainMaterialClass::Fluid,
                    bbb_world::TerrainMaterialClass::Translucent => {
                        TerrainMaterialClass::Translucent
                    }
                },
                light: TerrainLight {
                    sky: cell.light.sky,
                    block: cell.light.block,
                },
                tint,
                face_transparency,
            }
        })
        .collect();
    TerrainChunkSnapshot::new(
        snapshot.pos.x,
        snapshot.pos.z,
        snapshot.min_y,
        snapshot.height,
        cells,
    )
    .with_cardinal_lighting(cardinal_lighting)
}

/// Samples the `biomeBlendRadius` window (x/z plane, fixed y) for `center`,
/// pulling neighbour-chunk biomes through `sampler`. Columns whose chunk is not
/// loaded stay `None`, so the averaging step honestly truncates the window at
/// the render-distance edge instead of inventing biome data.
fn build_biome_blend(
    sampler: &bbb_world::ChunkBiomeSampler,
    center: BlockRenderPosition,
) -> BiomeBlend {
    let mut samples = [None; BIOME_BLEND_SAMPLES];
    for row in 0..BIOME_BLEND_DIAMETER {
        for col in 0..BIOME_BLEND_DIAMETER {
            let index = (row * BIOME_BLEND_DIAMETER + col) as usize;
            samples[index] = sampler.biome_id_at(
                center.x - BIOME_BLEND_RADIUS + col,
                center.y,
                center.z - BIOME_BLEND_RADIUS + row,
            );
        }
    }
    BiomeBlend::new(center, samples)
}

fn renderer_fluid(fluid: bbb_world::TerrainFluidState) -> TerrainFluid {
    let kind = match fluid.kind {
        bbb_world::TerrainFluidKind::Water => TerrainFluidKind::Water,
        bbb_world::TerrainFluidKind::Lava => TerrainFluidKind::Lava,
    };
    TerrainFluid::new(kind, fluid.amount, fluid.falling)
}

fn block_uses_fluid_water_overlay(block_name: Option<&str>) -> bool {
    let Some(block_name) = block_name.and_then(|name| name.strip_prefix("minecraft:")) else {
        return false;
    };

    matches!(
        block_name,
        "glass"
            | "tinted_glass"
            | "ice"
            | "frosted_ice"
            | "blue_ice"
            | "slime_block"
            | "honey_block"
    ) || block_name.ends_with("_stained_glass")
        || block_name.ends_with("copper_grate")
        || block_name.ends_with("_leaves")
}

#[cfg(test)]
mod tests {
    use super::*;
    use bbb_protocol::packets::SetChunkCacheCenter;
    use bbb_world::{ChunkColumn, ChunkState, LightData};

    #[test]
    fn renderer_fluid_preserves_world_fluid_state() {
        assert_eq!(
            renderer_fluid(bbb_world::TerrainFluidState::new(
                bbb_world::TerrainFluidKind::Water,
                5,
                false,
            )),
            TerrainFluid::new(TerrainFluidKind::Water, 5, false)
        );
        assert_eq!(
            renderer_fluid(bbb_world::TerrainFluidState::new(
                bbb_world::TerrainFluidKind::Lava,
                8,
                true,
            )),
            TerrainFluid::new(TerrainFluidKind::Lava, 8, true)
        );
    }

    #[test]
    fn fluid_water_overlay_neighbors_follow_vanilla_half_transparent_and_leaves_blocks() {
        for block_name in [
            "minecraft:glass",
            "minecraft:tinted_glass",
            "minecraft:white_stained_glass",
            "minecraft:ice",
            "minecraft:frosted_ice",
            "minecraft:blue_ice",
            "minecraft:slime_block",
            "minecraft:honey_block",
            "minecraft:copper_grate",
            "minecraft:weathered_copper_grate",
            "minecraft:waxed_oxidized_copper_grate",
            "minecraft:oak_leaves",
            "minecraft:flowering_azalea_leaves",
        ] {
            assert!(
                block_uses_fluid_water_overlay(Some(block_name)),
                "{block_name} should use water overlay"
            );
        }

        for block_name in [
            None,
            Some("stone"),
            Some("minecraft:stone"),
            Some("minecraft:packed_ice"),
            Some("minecraft:glass_pane"),
            Some("minecraft:white_stained_glass_pane"),
        ] {
            assert!(
                !block_uses_fluid_water_overlay(block_name),
                "{block_name:?} should not use water overlay"
            );
        }
    }

    #[test]
    fn texture_animation_tick_advances_at_client_tick_interval() {
        let mut upload = TerrainUploadState::default();
        let start = Instant::now();

        assert_eq!(advance_texture_animation_tick(&mut upload, start), None);
        assert_eq!(
            advance_texture_animation_tick(&mut upload, start + Duration::from_millis(49)),
            None
        );
        assert_eq!(
            advance_texture_animation_tick(&mut upload, start + Duration::from_millis(50)),
            Some(1)
        );
        assert_eq!(
            advance_texture_animation_tick(&mut upload, start + Duration::from_millis(149)),
            Some(2)
        );
        assert_eq!(
            advance_texture_animation_tick(&mut upload, start + Duration::from_millis(250)),
            Some(5)
        );
        assert_eq!(
            advance_texture_animation_tick(&mut upload, start + Duration::from_millis(299)),
            None
        );
    }

    #[test]
    fn terrain_upload_center_is_derived_from_world_chunk_state() {
        let mut world = WorldStore::new();
        assert_eq!(terrain_upload_center(&world), ChunkPos { x: 0, z: 0 });

        world.insert_decoded_chunk(test_chunk(ChunkPos { x: 5, z: 6 }));
        assert_eq!(terrain_upload_center(&world), ChunkPos { x: 5, z: 6 });

        world.insert_decoded_chunk(test_chunk(ChunkPos { x: -1, z: 2 }));
        assert_eq!(terrain_upload_center(&world), ChunkPos { x: 5, z: 6 });

        world.apply_set_chunk_cache_center(SetChunkCacheCenter {
            chunk_x: -4,
            chunk_z: 7,
        });
        assert_eq!(terrain_upload_center(&world), ChunkPos { x: -4, z: 7 });
    }

    fn test_chunk(pos: ChunkPos) -> ChunkColumn {
        ChunkColumn {
            pos,
            state: ChunkState::Decoded,
            heightmaps: Vec::new(),
            sections: Vec::new(),
            block_entities: Vec::new(),
            light: LightData::default(),
        }
    }
}
