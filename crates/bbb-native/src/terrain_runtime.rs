use std::time::{Duration, Instant};

use bbb_control::NetCounters;
use bbb_renderer::terrain::{
    build_terrain_mesh_layers_with_atlas, TerrainCell, TerrainChunkSnapshot, TerrainFluid,
    TerrainFluidKind, TerrainLight, TerrainMaterialClass, TerrainTint,
};
use bbb_world::{ChunkPos, WorldStore};

mod textures;
pub(crate) use textures::{load_terrain_textures, BlockRenderPosition, TerrainTextureState};

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
            if let Err(err) = renderer.update_terrain_texture_atlas(&atlas.rgba) {
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
    counters: &NetCounters,
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

    let center = world
        .chunk_cache_center()
        .or(counters.first_chunk)
        .unwrap_or_else(|| {
            world
                .chunk_positions()
                .into_iter()
                .next()
                .unwrap_or(ChunkPos { x: 0, z: 0 })
        });
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
    let renderer_snapshots: Vec<_> = snapshots
        .into_iter()
        .map(|snapshot| convert_terrain_snapshot(snapshot, textures))
        .collect();
    let meshes = build_terrain_mesh_layers_with_atlas(&renderer_snapshots, &textures.atlas);

    renderer.upload_terrain_mesh_layers(meshes);
    upload.decoded_chunks = world_counters.chunks_decoded;
    upload.block_updates_applied = world_counters.block_updates_applied;
    upload.light_updates_applied = world_counters.light_updates_applied;
    upload.biome_updates_applied = world_counters.biome_updates_applied;
    upload.uploaded_chunks = chunk_count;
}

fn chunk_distance_key(pos: ChunkPos, center: ChunkPos) -> i64 {
    let dx = i64::from(pos.x - center.x);
    let dz = i64::from(pos.z - center.z);
    dx * dx + dz * dz
}

fn convert_terrain_snapshot(
    snapshot: bbb_world::TerrainChunkSnapshot,
    textures: &TerrainTextureState,
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
            let (texture_indices, tint, face_transparency, render_shape, ambient_occlusion) =
                textures.block_render_data(
                    cell.block_name.as_deref(),
                    &cell.block_properties,
                    world_material,
                    cell.biome_id,
                    Some(position),
                );
            let fluid = cell.fluid.map(renderer_fluid);
            let (fluid_texture_indices, fluid_tint) = fluid
                .map(|fluid| textures.fluid_render_data(fluid.kind, cell.biome_id, Some(position)))
                .unwrap_or(([0; 6], [TerrainTint::WHITE; 6]));
            TerrainCell {
                block_state_id: cell.block_state_id,
                fluid,
                fluid_texture_indices,
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
}

fn renderer_fluid(fluid: bbb_world::TerrainFluidState) -> TerrainFluid {
    let kind = match fluid.kind {
        bbb_world::TerrainFluidKind::Water => TerrainFluidKind::Water,
        bbb_world::TerrainFluidKind::Lava => TerrainFluidKind::Lava,
    };
    TerrainFluid::new(kind, fluid.amount, fluid.falling)
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
