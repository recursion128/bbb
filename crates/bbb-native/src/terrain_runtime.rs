use std::{
    collections::{BTreeMap, HashMap},
    time::{Duration, Instant},
};

use anyhow::Result;
use bbb_control::NetCounters;
use bbb_pack::{
    AtlasLayout, AtlasPacker, BiomeColorCatalog, BiomeColorProfile, BlockFaceTextures,
    BlockModelCatalog, BlockModelShape, GrassColorModifier, PackRoots, TerrainColorMaps,
};
use bbb_renderer::terrain::{
    build_terrain_mesh_layers_with_atlas, TerrainCell, TerrainChunkSnapshot, TerrainLight,
    TerrainMaterialClass, TerrainRenderShape, TerrainTextureAtlas, TerrainTint, TerrainUvRect,
};
use bbb_world::{ChunkPos, WorldStore};

use crate::biome_tint::{
    apply_grass_color_modifier, biome_colormap_climate, is_dry_foliage_tinted_block,
    is_foliage_tinted_block, is_grass_tinted_block, terrain_tint_from_rgb,
};

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

#[derive(Debug, Clone)]
pub(crate) struct TerrainTextureState {
    atlas: TerrainTextureAtlas,
    indices: HashMap<String, u32>,
    block_models: Option<BlockModelCatalog>,
    colormaps: Option<TerrainColorMaps>,
    biome_colors: Option<BiomeColorCatalog>,
    fallback_index: u32,
}

impl Default for TerrainTextureState {
    fn default() -> Self {
        Self {
            atlas: TerrainTextureAtlas::unit(),
            indices: HashMap::new(),
            block_models: None,
            colormaps: None,
            biome_colors: None,
            fallback_index: 0,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct BlockRenderPosition {
    pub(crate) x: i32,
    pub(crate) z: i32,
}

impl TerrainTextureState {
    fn from_layout(
        layout: &AtlasLayout,
        block_models: Option<BlockModelCatalog>,
        colormaps: Option<TerrainColorMaps>,
        biome_colors: Option<BiomeColorCatalog>,
    ) -> Self {
        let mut indices = HashMap::new();
        let mut rects = Vec::with_capacity(layout.sprites.len());
        for (index, sprite) in layout.sprites.iter().enumerate() {
            indices.insert(sprite.id.clone(), index as u32);
            rects.push(terrain_uv_rect(layout, sprite));
        }
        let fallback_index = indices.get("minecraft:block/stone").copied().unwrap_or(0);
        Self {
            atlas: TerrainTextureAtlas {
                rects,
                fallback_index,
            },
            indices,
            block_models,
            colormaps,
            biome_colors,
            fallback_index,
        }
    }

    fn texture_index(&self, texture_id: &str) -> u32 {
        self.indices
            .get(texture_id)
            .copied()
            .unwrap_or(self.fallback_index)
    }

    fn block_render_data(
        &self,
        block_name: Option<&str>,
        properties: &BTreeMap<String, String>,
        material: bbb_world::TerrainMaterialClass,
        biome_id: Option<i32>,
        position: Option<BlockRenderPosition>,
    ) -> ([u32; 6], [TerrainTint; 6], TerrainRenderShape) {
        let Some(block_name) = block_name else {
            return (
                [self.fallback_index; 6],
                [TerrainTint::WHITE; 6],
                TerrainRenderShape::Cube,
            );
        };

        if let Some(model) = self
            .block_models
            .as_ref()
            .and_then(|models| models.block_render_model(block_name, properties))
        {
            let texture_indices = self.face_texture_indices(&model.face_textures);
            let tint = self.face_tints(
                block_name,
                material,
                &model.face_textures,
                biome_id,
                position,
            );
            return (
                texture_indices,
                tint,
                self.terrain_render_shape_for_block(
                    block_name,
                    properties,
                    material,
                    model.shape,
                    texture_indices,
                    tint,
                    biome_id,
                    position,
                ),
            );
        }

        let all = self.texture_index(&block_fallback_texture_id(block_name));
        let texture_indices = [all; 6];
        let tint = self.fallback_face_tints(block_name, material, biome_id, position);
        (
            texture_indices,
            tint,
            self.terrain_render_shape_for_block(
                block_name,
                properties,
                material,
                BlockModelShape::Cube,
                texture_indices,
                tint,
                biome_id,
                position,
            ),
        )
    }

    fn face_texture_indices(&self, face_textures: &BlockFaceTextures) -> [u32; 6] {
        std::array::from_fn(|index| self.texture_index(&face_textures.textures[index]))
    }

    fn face_tints(
        &self,
        block_name: &str,
        material: bbb_world::TerrainMaterialClass,
        face_textures: &BlockFaceTextures,
        biome_id: Option<i32>,
        position: Option<BlockRenderPosition>,
    ) -> [TerrainTint; 6] {
        std::array::from_fn(|index| {
            self.block_tint(
                block_name,
                material,
                face_textures.tint_indices[index],
                biome_id,
                position,
            )
        })
    }

    fn terrain_render_shape_for_block(
        &self,
        block_name: &str,
        properties: &BTreeMap<String, String>,
        material: bbb_world::TerrainMaterialClass,
        model_shape: BlockModelShape,
        fallback_texture_indices: [u32; 6],
        fallback_tint: [TerrainTint; 6],
        biome_id: Option<i32>,
        position: Option<BlockRenderPosition>,
    ) -> TerrainRenderShape {
        if matches!(material, bbb_world::TerrainMaterialClass::Fluid) {
            if let Some(shape) = fluid_render_shape(block_name, properties) {
                return shape;
            }
        }
        self.terrain_render_shape(
            block_name,
            material,
            model_shape,
            fallback_texture_indices,
            fallback_tint,
            biome_id,
            position,
        )
    }

    fn terrain_render_shape(
        &self,
        block_name: &str,
        material: bbb_world::TerrainMaterialClass,
        shape: BlockModelShape,
        fallback_texture_indices: [u32; 6],
        fallback_tint: [TerrainTint; 6],
        biome_id: Option<i32>,
        position: Option<BlockRenderPosition>,
    ) -> TerrainRenderShape {
        match shape {
            BlockModelShape::Cross => TerrainRenderShape::Cross,
            BlockModelShape::Box(model_box) => TerrainRenderShape::Box {
                from: model_box.from,
                to: model_box.to,
                face_present: model_box.face_present,
                face_uvs: model_box.face_uvs,
                face_cull: model_box.face_cull,
            },
            BlockModelShape::Boxes(model_boxes) => TerrainRenderShape::Boxes(
                model_boxes
                    .into_iter()
                    .map(|model_box| bbb_renderer::terrain::TerrainBox {
                        from: model_box.from,
                        to: model_box.to,
                        face_present: model_box.face_present,
                        face_uvs: model_box.face_uvs,
                        face_cull: model_box.face_cull,
                        texture_indices: self
                            .model_box_texture_indices(&model_box, fallback_texture_indices),
                        tint: self.model_box_face_tints(
                            block_name,
                            material,
                            &model_box,
                            fallback_tint,
                            biome_id,
                            position,
                        ),
                    })
                    .collect(),
            ),
            BlockModelShape::Cube | BlockModelShape::Custom => TerrainRenderShape::Cube,
        }
    }

    fn model_box_texture_indices(
        &self,
        model_box: &bbb_pack::BlockModelBox,
        fallback: [u32; 6],
    ) -> [u32; 6] {
        std::array::from_fn(|index| {
            model_box.face_textures[index]
                .as_deref()
                .map(|texture| self.texture_index(texture))
                .unwrap_or(fallback[index])
        })
    }

    fn fallback_face_tints(
        &self,
        block_name: &str,
        material: bbb_world::TerrainMaterialClass,
        biome_id: Option<i32>,
        position: Option<BlockRenderPosition>,
    ) -> [TerrainTint; 6] {
        [self.block_tint(block_name, material, Some(0), biome_id, position); 6]
    }

    fn model_box_face_tints(
        &self,
        block_name: &str,
        material: bbb_world::TerrainMaterialClass,
        model_box: &bbb_pack::BlockModelBox,
        fallback: [TerrainTint; 6],
        biome_id: Option<i32>,
        position: Option<BlockRenderPosition>,
    ) -> [TerrainTint; 6] {
        std::array::from_fn(|index| {
            if model_box.face_present[index] {
                self.block_tint(
                    block_name,
                    material,
                    model_box.face_tint_indices[index],
                    biome_id,
                    position,
                )
            } else {
                fallback[index]
            }
        })
    }

    fn block_tint(
        &self,
        block_name: &str,
        material: bbb_world::TerrainMaterialClass,
        tint_index: Option<i32>,
        biome_id: Option<i32>,
        position: Option<BlockRenderPosition>,
    ) -> TerrainTint {
        if matches!(block_name, "minecraft:water" | "minecraft:water_cauldron") {
            return self.water_tint(biome_id);
        }
        if tint_index.is_none() {
            return TerrainTint::WHITE;
        }
        if matches!(block_name, "minecraft:spruce_leaves") {
            return TerrainTint::from_rgb_u8(0x61, 0x99, 0x61);
        }
        if matches!(block_name, "minecraft:birch_leaves") {
            return TerrainTint::from_rgb_u8(0x80, 0xa7, 0x55);
        }
        if is_dry_foliage_tinted_block(block_name) {
            return self.dry_foliage_tint(biome_id);
        }
        if is_foliage_tinted_block(block_name) {
            return self.foliage_tint(biome_id);
        }
        if is_grass_tinted_block(block_name) {
            return self.grass_tint(biome_id, position);
        }
        if matches!(material, bbb_world::TerrainMaterialClass::Fluid) {
            return TerrainTint::WHITE;
        }
        TerrainTint::WHITE
    }

    fn grass_tint(
        &self,
        biome_id: Option<i32>,
        position: Option<BlockRenderPosition>,
    ) -> TerrainTint {
        let profile = self.biome_profile(biome_id);
        let base = profile.and_then(|profile| profile.grass_color).or_else(|| {
            self.colormaps.as_ref().map(|colormaps| {
                let (temperature, downfall) = biome_colormap_climate(profile);
                colormaps
                    .grass
                    .sample_temperature_downfall(temperature, downfall)
            })
        });
        let Some(base) = base else {
            return TerrainTint::from_rgb_u8(0x91, 0xbd, 0x59);
        };
        terrain_tint_from_rgb(apply_grass_color_modifier(
            profile.map_or(GrassColorModifier::None, |profile| {
                profile.grass_color_modifier
            }),
            base,
            position,
        ))
    }

    fn foliage_tint(&self, biome_id: Option<i32>) -> TerrainTint {
        let profile = self.biome_profile(biome_id);
        profile
            .and_then(|profile| profile.foliage_color)
            .or_else(|| {
                self.colormaps.as_ref().map(|colormaps| {
                    let (temperature, downfall) = biome_colormap_climate(profile);
                    colormaps
                        .foliage
                        .sample_temperature_downfall(temperature, downfall)
                })
            })
            .map(terrain_tint_from_rgb)
            .unwrap_or_else(|| TerrainTint::from_rgb_u8(0x48, 0xb5, 0x18))
    }

    fn dry_foliage_tint(&self, biome_id: Option<i32>) -> TerrainTint {
        let profile = self.biome_profile(biome_id);
        profile
            .and_then(|profile| profile.dry_foliage_color)
            .or_else(|| {
                self.colormaps
                    .as_ref()
                    .and_then(|colormaps| colormaps.dry_foliage.as_ref())
                    .map(|colormap| {
                        let (temperature, downfall) = biome_colormap_climate(profile);
                        colormap.sample_temperature_downfall(temperature, downfall)
                    })
            })
            .map(terrain_tint_from_rgb)
            .unwrap_or_else(|| TerrainTint::from_rgb_u8(0x5c, 0x3c, 0x32))
    }

    fn water_tint(&self, biome_id: Option<i32>) -> TerrainTint {
        self.biome_profile(biome_id)
            .and_then(|profile| profile.water_color)
            .map(terrain_tint_from_rgb)
            .unwrap_or_else(|| TerrainTint::from_rgb_u8(0x3f, 0x76, 0xe4))
    }

    fn biome_profile(&self, biome_id: Option<i32>) -> Option<&BiomeColorProfile> {
        self.biome_colors.as_ref()?.profile(biome_id?)
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

    let center = counters
        .chunk_cache_center
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

pub(crate) fn load_terrain_textures(renderer: &mut bbb_renderer::Renderer) -> TerrainTextureState {
    match try_load_terrain_textures(renderer) {
        Ok(textures) => textures,
        Err(err) => {
            tracing::warn!(?err, "falling back to default terrain texture atlas");
            TerrainTextureState::default()
        }
    }
}

fn try_load_terrain_textures(renderer: &mut bbb_renderer::Renderer) -> Result<TerrainTextureState> {
    let roots = PackRoots::discover()?;
    let images = roots.load_block_texture_images()?;
    let block_models = roots.load_block_model_catalog()?;
    let colormaps = match roots.load_terrain_colormaps() {
        Ok(colormaps) => Some(colormaps),
        Err(err) => {
            tracing::warn!(?err, "falling back to constant terrain tint colors");
            None
        }
    };
    let biome_colors = match roots.load_biome_color_catalog() {
        Ok(biome_colors) => Some(biome_colors),
        Err(err) => {
            tracing::warn!(?err, "falling back to default terrain biome tint");
            None
        }
    };
    let atlas = AtlasPacker::new(4096, 1)?.stitch(&images)?;
    renderer.upload_terrain_texture_atlas(atlas.layout.width, atlas.layout.height, &atlas.rgba)?;
    tracing::info!(
        width = atlas.layout.width,
        height = atlas.layout.height,
        sprites = atlas.layout.sprites.len(),
        blockstates = block_models.len(),
        colormaps = colormaps.is_some(),
        biome_colors = biome_colors.as_ref().map_or(0, |colors| colors.len()),
        "loaded terrain texture atlas"
    );
    Ok(TerrainTextureState::from_layout(
        &atlas.layout,
        Some(block_models),
        colormaps,
        biome_colors,
    ))
}

fn terrain_uv_rect(layout: &AtlasLayout, sprite: &bbb_pack::AtlasSprite) -> TerrainUvRect {
    let width = layout.width as f32;
    let height = layout.height as f32;
    let x0 = sprite.content.x as f32;
    let y0 = sprite.content.y as f32;
    let x1 = (sprite.content.x + sprite.content.width) as f32;
    let y1 = (sprite.content.y + sprite.content.height) as f32;
    TerrainUvRect {
        min: [(x0 + 0.5) / width, (y0 + 0.5) / height],
        max: [(x1 - 0.5) / width, (y1 - 0.5) / height],
    }
}

fn block_fallback_texture_id(block_name: &str) -> String {
    let stem = block_name.strip_prefix("minecraft:").unwrap_or(block_name);
    format!("minecraft:block/{stem}")
}

fn fluid_render_shape(
    block_name: &str,
    properties: &BTreeMap<String, String>,
) -> Option<TerrainRenderShape> {
    if !matches!(block_name, "minecraft:water" | "minecraft:lava") {
        return None;
    }

    let level = properties
        .get("level")
        .and_then(|value| value.parse::<u8>().ok())
        .unwrap_or(0);
    Some(fluid_box_shape(fluid_height_units(level)))
}

fn fluid_height_units(level: u8) -> u8 {
    let amount = match level {
        0 => 8,
        1..=7 => 8 - level,
        _ => 8,
    };
    ((amount as u16 * 16 + 4) / 9).clamp(1, 16) as u8
}

fn fluid_box_shape(height: u8) -> TerrainRenderShape {
    let height = height.clamp(1, 16);
    let mut face_uvs = [[0, 0, 16, 16]; 6];
    let side_v0 = 16 - height;
    face_uvs[2] = [0, side_v0, 16, 16];
    face_uvs[3] = [0, side_v0, 16, 16];
    face_uvs[4] = [0, side_v0, 16, 16];
    face_uvs[5] = [0, side_v0, 16, 16];
    TerrainRenderShape::Box {
        from: [0, 0, 0],
        to: [16, height, 16],
        face_present: [true; 6],
        face_uvs,
        face_cull: [true; 6],
    }
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
            let local_z = ((index / 16) % 16) as i32;
            let position = BlockRenderPosition {
                x: chunk_origin_x + local_x,
                z: chunk_origin_z + local_z,
            };
            let world_material = cell.material;
            let (texture_indices, tint, render_shape) = textures.block_render_data(
                cell.block_name.as_deref(),
                &cell.block_properties,
                world_material,
                cell.biome_id,
                Some(position),
            );
            TerrainCell {
                block_state_id: cell.block_state_id,
                texture_indices,
                render_shape,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fluid_height_units_follow_vanilla_legacy_level_amounts() {
        assert_eq!(fluid_height_units(0), 14);
        assert_eq!(fluid_height_units(1), 12);
        assert_eq!(fluid_height_units(2), 11);
        assert_eq!(fluid_height_units(3), 9);
        assert_eq!(fluid_height_units(4), 7);
        assert_eq!(fluid_height_units(5), 5);
        assert_eq!(fluid_height_units(6), 4);
        assert_eq!(fluid_height_units(7), 2);
        assert_eq!(fluid_height_units(8), 14);
        assert_eq!(fluid_height_units(15), 14);
    }

    #[test]
    fn water_level_shape_uses_cropped_fluid_box() {
        let shape = fluid_render_shape("minecraft:water", &properties([("level", "3")]))
            .expect("water has a fluid render shape");

        assert_eq!(
            shape,
            TerrainRenderShape::Box {
                from: [0, 0, 0],
                to: [16, 9, 16],
                face_present: [true; 6],
                face_uvs: [
                    [0, 0, 16, 16],
                    [0, 0, 16, 16],
                    [0, 7, 16, 16],
                    [0, 7, 16, 16],
                    [0, 7, 16, 16],
                    [0, 7, 16, 16],
                ],
                face_cull: [true; 6],
            }
        );
    }

    #[test]
    fn fluid_material_overrides_particle_only_model_shape() {
        let textures = TerrainTextureState::default();
        let shape = textures.terrain_render_shape_for_block(
            "minecraft:lava",
            &properties([("level", "8")]),
            bbb_world::TerrainMaterialClass::Fluid,
            BlockModelShape::Custom,
            [0; 6],
            [TerrainTint::WHITE; 6],
            None,
            None,
        );

        assert!(matches!(
            shape,
            TerrainRenderShape::Box {
                to: [16, 14, 16],
                ..
            }
        ));

        let non_fluid_shape = textures.terrain_render_shape_for_block(
            "minecraft:lava",
            &properties([("level", "8")]),
            bbb_world::TerrainMaterialClass::Opaque,
            BlockModelShape::Custom,
            [0; 6],
            [TerrainTint::WHITE; 6],
            None,
            None,
        );
        assert_eq!(non_fluid_shape, TerrainRenderShape::Cube);
    }

    #[test]
    fn model_boxes_preserve_per_element_textures_and_tints() {
        let mut texture_state = TerrainTextureState::default();
        texture_state
            .indices
            .insert("minecraft:block/base".to_string(), 1);
        texture_state
            .indices
            .insert("minecraft:block/overlay".to_string(), 2);
        let base = block_model_box_with_face_texture(
            bbb_pack::BlockModelFace::North,
            "minecraft:block/base",
            None,
        );
        let overlay = block_model_box_with_face_texture(
            bbb_pack::BlockModelFace::North,
            "minecraft:block/overlay",
            Some(0),
        );

        let shape = texture_state.terrain_render_shape_for_block(
            "minecraft:grass_block",
            &BTreeMap::new(),
            bbb_world::TerrainMaterialClass::Opaque,
            BlockModelShape::Boxes(vec![base, overlay]),
            [0; 6],
            [TerrainTint::WHITE; 6],
            Some(4),
            None,
        );

        let TerrainRenderShape::Boxes(boxes) = shape else {
            panic!("expected boxes render shape");
        };
        let north = bbb_pack::BlockModelFace::North.index();
        assert_eq!(boxes[0].texture_indices[north], 1);
        assert_eq!(boxes[0].tint[north], TerrainTint::WHITE);
        assert_eq!(boxes[1].texture_indices[north], 2);
        assert_eq!(
            boxes[1].tint[north],
            TerrainTint::from_rgb_u8(0x91, 0xbd, 0x59)
        );
    }

    #[test]
    fn block_tint_uses_default_vanilla_color_classes() {
        let textures = TerrainTextureState::default();
        assert_eq!(
            textures.block_tint(
                "minecraft:stone",
                bbb_world::TerrainMaterialClass::Opaque,
                None,
                None,
                None
            ),
            TerrainTint::WHITE
        );
        assert_eq!(
            textures.block_tint(
                "minecraft:grass_block",
                bbb_world::TerrainMaterialClass::Opaque,
                Some(0),
                None,
                None
            ),
            TerrainTint::from_rgb_u8(0x91, 0xbd, 0x59)
        );
        assert_eq!(
            textures.block_tint(
                "minecraft:oak_leaves",
                bbb_world::TerrainMaterialClass::Cutout,
                Some(0),
                None,
                None
            ),
            TerrainTint::from_rgb_u8(0x48, 0xb5, 0x18)
        );
        assert_eq!(
            textures.block_tint(
                "minecraft:spruce_leaves",
                bbb_world::TerrainMaterialClass::Cutout,
                Some(0),
                None,
                None
            ),
            TerrainTint::from_rgb_u8(0x61, 0x99, 0x61)
        );
        assert_eq!(
            textures.block_tint(
                "minecraft:birch_leaves",
                bbb_world::TerrainMaterialClass::Cutout,
                Some(0),
                None,
                None
            ),
            TerrainTint::from_rgb_u8(0x80, 0xa7, 0x55)
        );
        assert_eq!(
            textures.block_tint(
                "minecraft:leaf_litter",
                bbb_world::TerrainMaterialClass::Cutout,
                Some(0),
                None,
                None
            ),
            TerrainTint::from_rgb_u8(0x5c, 0x3c, 0x32)
        );
        assert_eq!(
            textures.block_tint(
                "minecraft:water",
                bbb_world::TerrainMaterialClass::Fluid,
                None,
                None,
                None
            ),
            TerrainTint::from_rgb_u8(0x3f, 0x76, 0xe4)
        );
    }

    #[test]
    fn block_tint_samples_loaded_colormaps() {
        let mut textures = TerrainTextureState::default();
        textures.colormaps = Some(TerrainColorMaps {
            grass: flat_colormap([10, 20, 30]),
            foliage: flat_colormap([40, 50, 60]),
            dry_foliage: Some(flat_colormap([70, 80, 90])),
        });

        assert_eq!(
            textures.block_tint(
                "minecraft:grass_block",
                bbb_world::TerrainMaterialClass::Opaque,
                Some(0),
                Some(4),
                None
            ),
            TerrainTint::from_rgb_u8(10, 20, 30)
        );
        assert_eq!(
            textures.block_tint(
                "minecraft:oak_leaves",
                bbb_world::TerrainMaterialClass::Cutout,
                Some(0),
                Some(4),
                None
            ),
            TerrainTint::from_rgb_u8(40, 50, 60)
        );
        assert_eq!(
            textures.block_tint(
                "minecraft:leaf_litter",
                bbb_world::TerrainMaterialClass::Cutout,
                Some(0),
                Some(4),
                None
            ),
            TerrainTint::from_rgb_u8(70, 80, 90)
        );
    }

    #[test]
    fn block_tint_uses_loaded_biome_color_profiles() {
        let mut textures = TerrainTextureState::default();
        textures.colormaps = Some(TerrainColorMaps {
            grass: flat_colormap([10, 20, 30]),
            foliage: flat_colormap([40, 50, 60]),
            dry_foliage: Some(flat_colormap([70, 80, 90])),
        });
        textures.biome_colors = Some(BiomeColorCatalog::new([BiomeColorProfile {
            id: 42,
            name: "minecraft:test_biome".to_string(),
            temperature: 0.2,
            downfall: 0.3,
            grass_color: Some([1, 2, 3]),
            foliage_color: Some([4, 5, 6]),
            dry_foliage_color: Some([7, 8, 9]),
            water_color: Some([10, 11, 12]),
            grass_color_modifier: GrassColorModifier::None,
        }]));

        assert_eq!(
            textures.block_tint(
                "minecraft:grass_block",
                bbb_world::TerrainMaterialClass::Opaque,
                Some(0),
                Some(42),
                Some(BlockRenderPosition { x: 0, z: 0 })
            ),
            TerrainTint::from_rgb_u8(1, 2, 3)
        );
        assert_eq!(
            textures.block_tint(
                "minecraft:oak_leaves",
                bbb_world::TerrainMaterialClass::Cutout,
                Some(0),
                Some(42),
                None
            ),
            TerrainTint::from_rgb_u8(4, 5, 6)
        );
        assert_eq!(
            textures.block_tint(
                "minecraft:leaf_litter",
                bbb_world::TerrainMaterialClass::Cutout,
                Some(0),
                Some(42),
                None
            ),
            TerrainTint::from_rgb_u8(7, 8, 9)
        );
        assert_eq!(
            textures.block_tint(
                "minecraft:water",
                bbb_world::TerrainMaterialClass::Fluid,
                None,
                Some(42),
                None
            ),
            TerrainTint::from_rgb_u8(10, 11, 12)
        );
    }

    #[test]
    fn biome_climate_changes_colormap_sample() {
        let mut textures = TerrainTextureState::default();
        textures.colormaps = Some(TerrainColorMaps {
            grass: coordinate_colormap(),
            foliage: flat_colormap([40, 50, 60]),
            dry_foliage: Some(flat_colormap([70, 80, 90])),
        });
        textures.biome_colors = Some(BiomeColorCatalog::new([BiomeColorProfile {
            id: 7,
            name: "minecraft:dry_biome".to_string(),
            temperature: 0.0,
            downfall: 1.0,
            grass_color: None,
            foliage_color: None,
            dry_foliage_color: None,
            water_color: None,
            grass_color_modifier: GrassColorModifier::None,
        }]));

        assert_eq!(
            textures.block_tint(
                "minecraft:grass_block",
                bbb_world::TerrainMaterialClass::Opaque,
                Some(0),
                Some(7),
                None
            ),
            TerrainTint::from_rgb_u8(30, 60, 6)
        );
    }

    fn properties<const N: usize>(entries: [(&str, &str); N]) -> BTreeMap<String, String> {
        entries
            .into_iter()
            .map(|(key, value)| (key.to_string(), value.to_string()))
            .collect()
    }

    fn flat_colormap(rgb: [u8; 3]) -> bbb_pack::ColorMapImage {
        bbb_pack::ColorMapImage::new(
            2,
            2,
            [rgb, rgb, rgb, rgb]
                .into_iter()
                .flat_map(|[r, g, b]| [r, g, b, 255])
                .collect(),
        )
        .unwrap()
    }

    fn coordinate_colormap() -> bbb_pack::ColorMapImage {
        let mut rgba = Vec::new();
        for y in 0u8..4 {
            for x in 0u8..4 {
                rgba.extend([x * 10, y * 20, x + y, 255]);
            }
        }
        bbb_pack::ColorMapImage::new(4, 4, rgba).unwrap()
    }

    fn block_model_box_with_face_texture(
        face: bbb_pack::BlockModelFace,
        texture: &str,
        tint_index: Option<i32>,
    ) -> bbb_pack::BlockModelBox {
        let mut face_present = [false; 6];
        let mut face_textures: [Option<String>; 6] = std::array::from_fn(|_| None);
        let mut face_tint_indices = [None; 6];
        face_present[face.index()] = true;
        face_textures[face.index()] = Some(texture.to_string());
        face_tint_indices[face.index()] = tint_index;
        bbb_pack::BlockModelBox {
            from: [0, 0, 0],
            to: [16, 16, 16],
            face_present,
            face_uvs: [[0, 0, 16, 16]; 6],
            face_cull: [false; 6],
            face_tint_indices,
            face_textures,
        }
    }
}
