use std::time::{Duration, Instant};

use bbb_control::NetCounters;
use bbb_renderer::terrain::{
    build_terrain_mesh_layers_with_atlas, TerrainCell, TerrainChunkSnapshot, TerrainFluid,
    TerrainFluidKind, TerrainLight, TerrainMaterialClass,
};
use bbb_world::{ChunkPos, WorldStore};

mod textures;
pub(crate) use textures::{load_terrain_textures, BlockRenderPosition, TerrainTextureState};

const MAX_TERRAIN_UPLOAD_CHUNKS: usize = 49;

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
}

impl TerrainUploadState {
    pub(crate) fn has_uploaded_chunks(&self) -> bool {
        self.uploaded_chunks > 0
    }
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
            TerrainCell {
                block_state_id: cell.block_state_id,
                fluid: terrain_fluid(cell.block_name.as_deref(), &cell.block_properties),
                texture_indices,
                render_shape,
                ambient_occlusion,
                material: match cell.material {
                    bbb_world::TerrainMaterialClass::Empty => TerrainMaterialClass::Empty,
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

fn terrain_fluid(
    block_name: Option<&str>,
    properties: &std::collections::BTreeMap<String, String>,
) -> Option<TerrainFluid> {
    let kind = match block_name? {
        "minecraft:water" => TerrainFluidKind::Water,
        "minecraft:lava" => TerrainFluidKind::Lava,
        _ => return None,
    };
    let level = properties
        .get("level")
        .and_then(|value| value.parse::<u8>().ok())
        .unwrap_or(0);
    let (amount, falling) = match level {
        0 => (8, false),
        1..=7 => (8 - level, false),
        _ => (8, true),
    };
    Some(TerrainFluid::new(kind, amount, falling))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn terrain_fluid_maps_liquid_block_level_to_vanilla_amount() {
        assert_eq!(
            terrain_fluid(Some("minecraft:water"), &properties([("level", "0")])),
            Some(TerrainFluid::new(TerrainFluidKind::Water, 8, false))
        );
        assert_eq!(
            terrain_fluid(Some("minecraft:water"), &properties([("level", "3")])),
            Some(TerrainFluid::new(TerrainFluidKind::Water, 5, false))
        );
        assert_eq!(
            terrain_fluid(Some("minecraft:lava"), &properties([("level", "8")])),
            Some(TerrainFluid::new(TerrainFluidKind::Lava, 8, true))
        );
        assert_eq!(
            terrain_fluid(Some("minecraft:lava"), &properties([("level", "15")])),
            Some(TerrainFluid::new(TerrainFluidKind::Lava, 8, true))
        );
        assert_eq!(
            terrain_fluid(Some("minecraft:stone"), &properties([("level", "0")])),
            None
        );
    }

    fn properties<const N: usize>(
        entries: [(&str, &str); N],
    ) -> std::collections::BTreeMap<String, String> {
        entries
            .into_iter()
            .map(|(key, value)| (key.to_string(), value.to_string()))
            .collect()
    }
}
