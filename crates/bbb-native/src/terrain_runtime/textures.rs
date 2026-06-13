use std::collections::{BTreeMap, HashMap};

use anyhow::Result;
use bbb_pack::{
    AtlasLayout, AtlasPacker, BiomeColorCatalog, BiomeColorProfile, BlockFaceTextures,
    BlockModelCatalog, BlockModelFace, BlockModelShape, GrassColorModifier, PackRoots,
    TerrainColorMaps,
};
use bbb_renderer::terrain::{
    TerrainCross, TerrainFace, TerrainRenderShape, TerrainTextureAtlas, TerrainTint, TerrainUvRect,
};

use crate::biome_tint::{
    apply_grass_color_modifier, biome_colormap_climate, is_dry_foliage_tinted_block,
    is_foliage_tinted_block, is_grass_tinted_block, terrain_tint_from_rgb,
};

#[derive(Debug, Clone)]
pub(crate) struct TerrainTextureState {
    pub(super) atlas: TerrainTextureAtlas,
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
    pub(crate) y: i32,
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

    pub(super) fn block_render_data(
        &self,
        block_name: Option<&str>,
        properties: &BTreeMap<String, String>,
        material: bbb_world::TerrainMaterialClass,
        biome_id: Option<i32>,
        position: Option<BlockRenderPosition>,
    ) -> ([u32; 6], [TerrainTint; 6], [bool; 6], TerrainRenderShape) {
        let Some(block_name) = block_name else {
            return (
                [self.fallback_index; 6],
                [TerrainTint::WHITE; 6],
                [false; 6],
                TerrainRenderShape::Cube,
            );
        };

        if let Some(model) = self.block_models.as_ref().and_then(|models| {
            models.block_render_model_with_seed(
                block_name,
                properties,
                position.map(block_model_seed),
            )
        }) {
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
                model.face_textures.force_translucent,
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
            [false; 6],
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
            BlockModelShape::Cross {
                shade,
                light_emission,
            } => TerrainRenderShape::Cross {
                shade,
                light_emission,
            },
            BlockModelShape::Crosses(model_crosses) => TerrainRenderShape::Crosses(
                model_crosses
                    .into_iter()
                    .map(|model_cross| TerrainCross {
                        texture_indices: self
                            .model_cross_texture_indices(&model_cross, fallback_texture_indices),
                        tint: self.model_cross_face_tints(
                            block_name,
                            material,
                            &model_cross,
                            fallback_tint,
                            biome_id,
                            position,
                        ),
                        face_force_translucent: model_cross.face_force_translucent,
                        shade: model_cross.shade,
                        light_emission: model_cross.light_emission,
                    })
                    .collect(),
            ),
            BlockModelShape::Box(model_box) => TerrainRenderShape::Box {
                from: model_box.from,
                to: model_box.to,
                face_present: model_box.face_present,
                face_uvs: model_box.face_uvs,
                face_uv_rotations: model_box.face_uv_rotations,
                face_shade: model_box.face_shade,
                face_light_emission: model_box.face_light_emission,
                face_cull: model_box_cull_faces(model_box.face_cull),
                face_force_translucent: model_box.face_force_translucent,
            },
            BlockModelShape::Boxes(model_boxes) => TerrainRenderShape::Boxes(
                model_boxes
                    .into_iter()
                    .map(|model_box| bbb_renderer::terrain::TerrainBox {
                        from: model_box.from,
                        to: model_box.to,
                        face_present: model_box.face_present,
                        face_uvs: model_box.face_uvs,
                        face_uv_rotations: model_box.face_uv_rotations,
                        face_shade: model_box.face_shade,
                        face_light_emission: model_box.face_light_emission,
                        face_cull: model_box_cull_faces(model_box.face_cull),
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
                        face_force_translucent: model_box.face_force_translucent,
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

    fn model_cross_texture_indices(
        &self,
        model_cross: &bbb_pack::BlockModelCross,
        fallback: [u32; 6],
    ) -> [u32; 6] {
        std::array::from_fn(|index| {
            model_cross.face_textures[index]
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

    fn model_cross_face_tints(
        &self,
        block_name: &str,
        material: bbb_world::TerrainMaterialClass,
        model_cross: &bbb_pack::BlockModelCross,
        fallback: [TerrainTint; 6],
        biome_id: Option<i32>,
        position: Option<BlockRenderPosition>,
    ) -> [TerrainTint; 6] {
        std::array::from_fn(|index| {
            if model_cross.face_textures[index].is_some() {
                self.block_tint(
                    block_name,
                    material,
                    model_cross.face_tint_indices[index],
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

fn block_model_seed(position: BlockRenderPosition) -> i64 {
    let seed = i64::from(position.x).wrapping_mul(3_129_871)
        ^ i64::from(position.z).wrapping_mul(116_129_781)
        ^ i64::from(position.y);
    seed.wrapping_mul(seed)
        .wrapping_mul(42_317_861)
        .wrapping_add(seed.wrapping_mul(11))
        >> 16
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
        face_uv_rotations: [0; 6],
        face_shade: [true; 6],
        face_light_emission: [0; 6],
        face_cull: all_terrain_face_cull(),
        face_force_translucent: [false; 6],
    }
}

fn model_box_cull_faces(face_cull: [Option<BlockModelFace>; 6]) -> [Option<TerrainFace>; 6] {
    face_cull.map(|face| face.map(model_face_to_terrain_face))
}

fn model_face_to_terrain_face(face: BlockModelFace) -> TerrainFace {
    match face {
        BlockModelFace::Down => TerrainFace::Down,
        BlockModelFace::Up => TerrainFace::Up,
        BlockModelFace::North => TerrainFace::North,
        BlockModelFace::South => TerrainFace::South,
        BlockModelFace::West => TerrainFace::West,
        BlockModelFace::East => TerrainFace::East,
    }
}

fn all_terrain_face_cull() -> [Option<TerrainFace>; 6] {
    TerrainFace::ALL.map(Some)
}

#[cfg(test)]
mod tests;
