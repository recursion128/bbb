use std::collections::{BTreeMap, HashMap};

use anyhow::Result;
use bbb_pack::{
    AtlasLayout, AtlasMipImage, AtlasPacker, BiomeColorCatalog, BiomeColorProfile,
    BlockFaceTextures, BlockModelCatalog, BlockModelFace, BlockModelShape, GrassColorModifier,
    PackRoots, SpriteImage, TerrainColorMaps,
};
use bbb_renderer::terrain::{
    TerrainCross, TerrainFace, TerrainFluidKind, TerrainQuad, TerrainRenderShape,
    TerrainTextureAtlas, TerrainTint, TerrainTransparency, TerrainUvRect,
};

use crate::biome_tint::{
    apply_grass_color_modifier, biome_colormap_climate, is_dry_foliage_tinted_block,
    is_foliage_tinted_block, is_grass_tinted_block, terrain_tint_from_rgb,
};

const VANILLA_DEFAULT_MIPMAP_LEVELS: u32 = 4;

#[derive(Debug, Clone)]
pub(crate) struct TerrainTextureState {
    pub(super) atlas: TerrainTextureAtlas,
    indices: HashMap<String, u32>,
    block_models: Option<BlockModelCatalog>,
    colormaps: Option<TerrainColorMaps>,
    biome_colors: Option<BiomeColorCatalog>,
    transparencies: Vec<TerrainTransparency>,
    sprite_alphas: HashMap<String, SpriteAlpha>,
    fallback_index: u32,
    animation: Option<TerrainTextureAnimation>,
}

impl Default for TerrainTextureState {
    fn default() -> Self {
        Self {
            atlas: TerrainTextureAtlas::unit(),
            indices: HashMap::new(),
            block_models: None,
            colormaps: None,
            biome_colors: None,
            transparencies: vec![TerrainTransparency::OPAQUE],
            sprite_alphas: HashMap::new(),
            fallback_index: 0,
            animation: None,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct BlockRenderPosition {
    pub(crate) x: i32,
    pub(crate) y: i32,
    pub(crate) z: i32,
}

#[derive(Debug, Clone)]
struct SpriteAlpha {
    width: u32,
    height: u32,
    transparency: TerrainTransparency,
    alpha: Vec<TerrainTransparency>,
}

#[derive(Debug, Clone)]
struct TerrainTextureAnimation {
    packer: AtlasPacker,
    images: Vec<SpriteImage>,
    max_mipmap_levels: u32,
}

impl TerrainTextureAnimation {
    fn new(packer: AtlasPacker, images: Vec<SpriteImage>, max_mipmap_levels: u32) -> Option<Self> {
        images
            .iter()
            .any(|image| image.animation.is_some())
            .then_some(Self {
                packer,
                images,
                max_mipmap_levels,
            })
    }

    fn stitch_frame(&self, tick: u64) -> Result<AtlasMipImage> {
        self.packer.stitch_animation_frame_mips_with_max_level(
            &self.images,
            tick,
            self.max_mipmap_levels,
        )
    }
}

impl SpriteAlpha {
    fn from_image(image: &SpriteImage) -> Self {
        let mut alpha = vec![TerrainTransparency::OPAQUE; (image.width * image.height) as usize];
        if image.animation_frames_rgba.is_empty() {
            accumulate_sprite_alpha(&mut alpha, &image.rgba);
        } else {
            for rgba in &image.animation_frames_rgba {
                accumulate_sprite_alpha(&mut alpha, rgba);
            }
        }
        Self {
            width: image.width,
            height: image.height,
            transparency: terrain_transparency(image.transparency),
            alpha,
        }
    }

    fn model_uv_transparency(&self, uv: [u8; 4]) -> TerrainTransparency {
        if uv == [0, 0, 16, 16] {
            return self.transparency;
        }
        let min_u = u32::from(uv[0].min(uv[2]));
        let min_v = u32::from(uv[1].min(uv[3]));
        let max_u = u32::from(uv[0].max(uv[2]));
        let max_v = u32::from(uv[1].max(uv[3]));
        let x0 = ((min_u * self.width) / 16).min(self.width);
        let y0 = ((min_v * self.height) / 16).min(self.height);
        let x1 = max_u
            .saturating_mul(self.width)
            .div_ceil(16)
            .min(self.width);
        let y1 = max_v
            .saturating_mul(self.height)
            .div_ceil(16)
            .min(self.height);
        if x0 >= x1 || y0 >= y1 {
            return TerrainTransparency::OPAQUE;
        }

        let mut transparency = TerrainTransparency::OPAQUE;
        for y in y0..y1 {
            for x in x0..x1 {
                transparency = transparency.or(self.alpha[(y * self.width + x) as usize]);
            }
        }
        transparency
    }
}

fn accumulate_sprite_alpha(alpha: &mut [TerrainTransparency], rgba: &[u8]) {
    for (slot, pixel) in alpha.iter_mut().zip(rgba.chunks_exact(4)) {
        *slot = slot.or(alpha_transparency(pixel[3]));
    }
}

fn alpha_transparency(alpha: u8) -> TerrainTransparency {
    if alpha == 0 {
        TerrainTransparency {
            has_transparent: true,
            has_translucent: false,
        }
    } else if alpha == 255 {
        TerrainTransparency::OPAQUE
    } else {
        TerrainTransparency::TRANSLUCENT
    }
}

impl TerrainTextureState {
    pub(crate) fn from_layout(
        layout: &AtlasLayout,
        block_models: Option<BlockModelCatalog>,
        colormaps: Option<TerrainColorMaps>,
        biome_colors: Option<BiomeColorCatalog>,
    ) -> Self {
        Self::from_layout_and_images(layout, &[], block_models, colormaps, biome_colors)
    }

    fn from_layout_and_images(
        layout: &AtlasLayout,
        images: &[SpriteImage],
        block_models: Option<BlockModelCatalog>,
        colormaps: Option<TerrainColorMaps>,
        biome_colors: Option<BiomeColorCatalog>,
    ) -> Self {
        let mut indices = HashMap::new();
        let mut rects = Vec::with_capacity(layout.sprites.len());
        let mut transparencies = Vec::with_capacity(layout.sprites.len());
        for (index, sprite) in layout.sprites.iter().enumerate() {
            indices.insert(sprite.id.clone(), index as u32);
            rects.push(terrain_uv_rect(layout, sprite));
            transparencies.push(terrain_transparency(sprite.transparency));
        }
        let sprite_alphas = images
            .iter()
            .map(|image| (image.id.clone(), SpriteAlpha::from_image(image)))
            .collect();
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
            transparencies,
            sprite_alphas,
            fallback_index,
            animation: None,
        }
    }

    pub(super) fn has_texture_animation(&self) -> bool {
        self.animation.is_some()
    }

    pub(super) fn animation_atlas_frame(&self, tick: u64) -> Result<Option<AtlasMipImage>> {
        self.animation
            .as_ref()
            .map(|animation| animation.stitch_frame(tick))
            .transpose()
    }

    fn texture_index(&self, texture_id: &str) -> u32 {
        self.indices
            .get(texture_id)
            .copied()
            .unwrap_or(self.fallback_index)
    }

    pub(crate) fn destroy_stage_uv_rect(&self, stage: u8) -> Option<TerrainUvRect> {
        let texture_id = format!("minecraft:block/destroy_stage_{}", stage.min(9));
        let index = *self.indices.get(&texture_id)?;
        self.atlas.rects.get(index as usize).copied()
    }

    fn texture_transparency(&self, texture_id: &str) -> TerrainTransparency {
        self.texture_index_transparency(self.texture_index(texture_id))
    }

    fn texture_uv_transparency(&self, texture_id: &str, uv: [u8; 4]) -> TerrainTransparency {
        self.sprite_alphas
            .get(texture_id)
            .map(|alpha| alpha.model_uv_transparency(uv))
            .unwrap_or_else(|| self.texture_transparency(texture_id))
    }

    fn texture_index_transparency(&self, texture_index: u32) -> TerrainTransparency {
        self.transparencies
            .get(texture_index as usize)
            .copied()
            .or_else(|| {
                self.transparencies
                    .get(self.fallback_index as usize)
                    .copied()
            })
            .unwrap_or(TerrainTransparency::OPAQUE)
    }

    fn fluid_texture_indices(&self, block_name: &str) -> Option<[u32; 6]> {
        let (still, flowing) = match block_name {
            "minecraft:water" => ("minecraft:block/water_still", "minecraft:block/water_flow"),
            "minecraft:lava" => ("minecraft:block/lava_still", "minecraft:block/lava_flow"),
            _ => return None,
        };
        let still = self.texture_index(still);
        let flowing = self.texture_index(flowing);
        Some([still, still, flowing, flowing, flowing, flowing])
    }

    pub(super) fn fluid_render_data(
        &self,
        kind: TerrainFluidKind,
        biome_id: Option<i32>,
        position: Option<BlockRenderPosition>,
    ) -> ([u32; 6], [TerrainTint; 6]) {
        let block_name = match kind {
            TerrainFluidKind::Water => "minecraft:water",
            TerrainFluidKind::Lava => "minecraft:lava",
        };
        let texture_indices = self
            .fluid_texture_indices(block_name)
            .unwrap_or([self.fallback_index; 6]);
        let tint = [self.block_tint(
            block_name,
            bbb_world::TerrainMaterialClass::Fluid,
            Some(0),
            biome_id,
            position,
        ); 6];
        (texture_indices, tint)
    }

    pub(crate) fn biome_sky_color(&self, biome_id: Option<i32>) -> Option<[u8; 3]> {
        self.biome_profile(biome_id)?.sky_color
    }

    pub(crate) fn biome_fog_color(&self, biome_id: Option<i32>) -> Option<[u8; 3]> {
        self.biome_profile(biome_id)?.fog_color
    }

    pub(crate) fn biome_water_fog_color(&self, biome_id: Option<i32>) -> Option<[u8; 3]> {
        self.biome_profile(biome_id)?.water_fog_color
    }

    #[cfg(test)]
    pub(crate) fn with_biome_colors_for_tests(biome_colors: BiomeColorCatalog) -> Self {
        let mut state = Self::default();
        state.biome_colors = Some(biome_colors);
        state
    }

    pub(super) fn block_render_data(
        &self,
        block_name: Option<&str>,
        properties: &BTreeMap<String, String>,
        material: bbb_world::TerrainMaterialClass,
        biome_id: Option<i32>,
        position: Option<BlockRenderPosition>,
    ) -> (
        [u32; 6],
        [TerrainTint; 6],
        [TerrainTransparency; 6],
        TerrainRenderShape,
        bool,
    ) {
        let Some(block_name) = block_name else {
            return (
                [self.fallback_index; 6],
                [TerrainTint::WHITE; 6],
                [TerrainTransparency::OPAQUE; 6],
                TerrainRenderShape::Cube,
                true,
            );
        };

        if let Some(model) = self.block_models.as_ref().and_then(|models| {
            models.block_render_model_with_seed(
                block_name,
                properties,
                position.map(block_model_seed),
            )
        }) {
            let texture_indices = self
                .fluid_texture_indices(block_name)
                .unwrap_or_else(|| self.face_texture_indices(&model.face_textures));
            let tint = self.face_tints(
                block_name,
                material,
                &model.face_textures,
                biome_id,
                position,
            );
            let face_transparency = self.face_texture_transparencies(&model.face_textures);
            return (
                texture_indices,
                tint,
                face_transparency,
                self.terrain_render_shape_for_block(
                    block_name,
                    properties,
                    material,
                    model.shape,
                    texture_indices,
                    tint,
                    face_transparency,
                    biome_id,
                    position,
                ),
                model.use_ambient_occlusion,
            );
        }

        let all = self.texture_index(&block_fallback_texture_id(block_name));
        let texture_indices = self.fluid_texture_indices(block_name).unwrap_or([all; 6]);
        let tint = self.fallback_face_tints(block_name, material, biome_id, position);
        (
            texture_indices,
            tint,
            [TerrainTransparency::OPAQUE; 6],
            self.terrain_render_shape_for_block(
                block_name,
                properties,
                material,
                BlockModelShape::Cube,
                texture_indices,
                tint,
                [TerrainTransparency::OPAQUE; 6],
                biome_id,
                position,
            ),
            true,
        )
    }

    /// Bakes the **item form** of a block — the block's model rendered as a held / dropped / framed /
    /// inventory item — into item-model quads in vanilla `0..=16` model space, sampling the blocks atlas.
    /// Returns `None` when no real block model exists for `block_name`, so a non-block item (apple, stick)
    /// is left to the flat-item path; a `Cross` foliage block bakes to an empty `Vec` (it renders as a
    /// flat item, not a 3D cross), which the caller treats the same as "not a 3D item". Biome tints use
    /// the default (no-position) climate, matching vanilla's fixed inventory grass/foliage color.
    pub(crate) fn block_item_quads(
        &self,
        block_name: &str,
        properties: &BTreeMap<String, String>,
    ) -> Option<Vec<bbb_renderer::ItemModelQuad>> {
        let models = self.block_models.as_ref()?;
        models.block_render_model_with_seed(block_name, properties, None)?;
        let (texture_indices, tint, _transparency, shape, _ao) = self.block_render_data(
            Some(block_name),
            properties,
            bbb_world::TerrainMaterialClass::Opaque,
            None,
            None,
        );
        Some(bbb_renderer::terrain::bake_block_item_quads(
            &shape,
            texture_indices,
            tint,
            &self.atlas,
        ))
    }

    /// The 3D item-frame border (vanilla `block/item_frame` / `block/glow_item_frame`, or the
    /// `*_map` variants): four `birch_planks` wood bars plus a `back` panel showing the `item_frame` /
    /// `glow_item_frame` texture, baked over the blocks atlas into item-model quads in `0..=16` model
    /// space while preserving the vanilla templates' fractional `15.5` / `15.001` depths.
    pub(crate) fn item_frame_border_quads(
        &self,
        glow: bool,
        map: bool,
    ) -> Vec<bbb_renderer::ItemModelQuad> {
        let wood = self.texture_index("minecraft:block/birch_planks");
        let back = self.texture_index(if glow {
            "minecraft:block/glow_item_frame"
        } else {
            "minecraft:block/item_frame"
        });
        if map {
            return self.item_frame_map_border_quads(wood, back);
        }
        self.item_frame_normal_border_quads(wood, back)
    }

    fn item_frame_normal_border_quads(
        &self,
        wood: u32,
        back: u32,
    ) -> Vec<bbb_renderer::ItemModelQuad> {
        let quads = [
            // Back panel: north + south faces, the `back` texture.
            frame_border_box_quads(
                [3.0, 3.0, 15.5],
                [13.0, 13.0, 16.0],
                [false, false, true, true, false, false],
                [
                    [0, 0, 16, 16],
                    [0, 0, 16, 16],
                    [3, 3, 13, 13],
                    [3, 3, 13, 13],
                    [0, 0, 16, 16],
                    [0, 0, 16, 16],
                ],
                [back; 6],
            ),
            // Bottom wood bar.
            frame_border_box_quads(
                [2.0, 2.0, 15.0],
                [14.0, 3.0, 16.0],
                [true; 6],
                [
                    [2, 0, 14, 1],
                    [2, 15, 14, 16],
                    [2, 13, 14, 14],
                    [2, 13, 14, 14],
                    [15, 13, 16, 14],
                    [0, 13, 1, 14],
                ],
                [wood; 6],
            ),
            // Top wood bar.
            frame_border_box_quads(
                [2.0, 13.0, 15.0],
                [14.0, 14.0, 16.0],
                [true; 6],
                [
                    [2, 0, 14, 1],
                    [2, 15, 14, 16],
                    [2, 2, 14, 3],
                    [2, 2, 14, 3],
                    [15, 2, 16, 3],
                    [0, 2, 1, 3],
                ],
                [wood; 6],
            ),
            // Left wood bar (no up/down faces).
            frame_border_box_quads(
                [2.0, 3.0, 15.0],
                [3.0, 13.0, 16.0],
                [false, false, true, true, true, true],
                [
                    [0, 0, 16, 16],
                    [0, 0, 16, 16],
                    [13, 3, 14, 13],
                    [2, 3, 3, 13],
                    [15, 3, 16, 13],
                    [0, 3, 1, 13],
                ],
                [wood; 6],
            ),
            // Right wood bar (no up/down faces).
            frame_border_box_quads(
                [13.0, 3.0, 15.0],
                [14.0, 13.0, 16.0],
                [false, false, true, true, true, true],
                [
                    [0, 0, 16, 16],
                    [0, 0, 16, 16],
                    [2, 3, 3, 13],
                    [13, 3, 14, 13],
                    [15, 3, 16, 13],
                    [0, 3, 1, 13],
                ],
                [wood; 6],
            ),
        ]
        .into_iter()
        .flatten()
        .collect();
        bbb_renderer::terrain::bake_block_item_quads(
            &TerrainRenderShape::Quads(quads),
            [self.fallback_index; 6],
            [TerrainTint::WHITE; 6],
            &self.atlas,
        )
    }

    fn item_frame_map_border_quads(
        &self,
        wood: u32,
        back: u32,
    ) -> Vec<bbb_renderer::ItemModelQuad> {
        let quads = [
            frame_border_box_quads(
                [1.0, 1.0, 15.001],
                [15.0, 15.0, 16.0],
                [false, false, true, true, false, false],
                [
                    [0, 0, 16, 16],
                    [0, 0, 16, 16],
                    [1, 1, 15, 15],
                    [1, 1, 15, 15],
                    [0, 0, 16, 16],
                    [0, 0, 16, 16],
                ],
                [back; 6],
            ),
            frame_border_box_quads(
                [0.0, 0.0, 15.001],
                [16.0, 1.0, 16.0],
                [true; 6],
                [
                    [0, 0, 16, 1],
                    [0, 15, 16, 16],
                    [0, 15, 16, 16],
                    [0, 15, 16, 16],
                    [15, 15, 16, 16],
                    [0, 15, 1, 16],
                ],
                [wood; 6],
            ),
            frame_border_box_quads(
                [0.0, 15.0, 15.001],
                [16.0, 16.0, 16.0],
                [true; 6],
                [
                    [0, 0, 16, 1],
                    [0, 15, 16, 16],
                    [0, 0, 16, 1],
                    [0, 0, 16, 1],
                    [15, 0, 16, 1],
                    [0, 0, 1, 1],
                ],
                [wood; 6],
            ),
            frame_border_box_quads(
                [0.0, 1.0, 15.001],
                [1.0, 15.0, 16.0],
                [false, false, true, true, true, true],
                [
                    [0, 0, 16, 16],
                    [0, 0, 16, 16],
                    [15, 1, 16, 15],
                    [0, 1, 1, 15],
                    [15, 1, 16, 15],
                    [0, 1, 1, 15],
                ],
                [wood; 6],
            ),
            frame_border_box_quads(
                [15.0, 1.0, 15.001],
                [16.0, 15.0, 16.0],
                [false, false, true, true, true, true],
                [
                    [0, 0, 16, 16],
                    [0, 0, 16, 16],
                    [0, 1, 1, 15],
                    [15, 1, 16, 15],
                    [15, 1, 16, 15],
                    [0, 1, 1, 15],
                ],
                [wood; 6],
            ),
        ]
        .into_iter()
        .flatten()
        .collect();
        bbb_renderer::terrain::bake_block_item_quads(
            &TerrainRenderShape::Quads(quads),
            [self.fallback_index; 6],
            [TerrainTint::WHITE; 6],
            &self.atlas,
        )
    }

    fn face_texture_indices(&self, face_textures: &BlockFaceTextures) -> [u32; 6] {
        std::array::from_fn(|index| self.texture_index(&face_textures.textures[index]))
    }

    fn face_texture_transparencies(
        &self,
        face_textures: &BlockFaceTextures,
    ) -> [TerrainTransparency; 6] {
        std::array::from_fn(|index| {
            self.texture_transparency(&face_textures.textures[index])
                .or(force_translucent(face_textures.force_translucent[index]))
        })
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
        fallback_transparency: [TerrainTransparency; 6],
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
            fallback_transparency,
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
        fallback_transparency: [TerrainTransparency; 6],
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
                        face_transparency: self
                            .model_cross_face_transparencies(&model_cross, fallback_transparency),
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
                face_transparency: self
                    .model_box_face_transparencies(&model_box, fallback_transparency),
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
                        face_transparency: self
                            .model_box_face_transparencies(&model_box, fallback_transparency),
                    })
                    .collect(),
            ),
            BlockModelShape::Quads(model_quads) => TerrainRenderShape::Quads(
                model_quads
                    .into_iter()
                    .map(|model_quad| TerrainQuad {
                        corners: model_quad.corners,
                        normal: model_quad.normal,
                        uvs: model_quad.uvs,
                        cull: model_quad.cull.map(model_face_to_terrain_face),
                        texture_index: model_quad
                            .texture
                            .as_deref()
                            .map(|texture| self.texture_index(texture))
                            .unwrap_or(fallback_texture_indices[model_quad.face.index()]),
                        tint: self.model_quad_tint(
                            block_name,
                            material,
                            &model_quad,
                            fallback_tint,
                            biome_id,
                            position,
                        ),
                        transparency: self
                            .model_quad_transparency(&model_quad, fallback_transparency)
                            .or(force_translucent(model_quad.force_translucent)),
                        shade: model_quad.shade,
                        light_emission: model_quad.light_emission,
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

    fn model_box_face_transparencies(
        &self,
        model_box: &bbb_pack::BlockModelBox,
        fallback: [TerrainTransparency; 6],
    ) -> [TerrainTransparency; 6] {
        std::array::from_fn(|index| {
            model_box.face_textures[index]
                .as_deref()
                .map(|texture| self.texture_uv_transparency(texture, model_box.face_uvs[index]))
                .unwrap_or(fallback[index])
                .or(force_translucent(model_box.face_force_translucent[index]))
        })
    }

    fn model_cross_face_transparencies(
        &self,
        model_cross: &bbb_pack::BlockModelCross,
        fallback: [TerrainTransparency; 6],
    ) -> [TerrainTransparency; 6] {
        std::array::from_fn(|index| {
            model_cross.face_textures[index]
                .as_deref()
                .map(|texture| self.texture_transparency(texture))
                .unwrap_or(fallback[index])
                .or(force_translucent(model_cross.face_force_translucent[index]))
        })
    }

    fn model_quad_transparency(
        &self,
        model_quad: &bbb_pack::BlockModelQuad,
        fallback: [TerrainTransparency; 6],
    ) -> TerrainTransparency {
        model_quad
            .texture
            .as_deref()
            .map(|texture| {
                quad_uv_crop(model_quad.uvs)
                    .map(|uv| self.texture_uv_transparency(texture, uv))
                    .unwrap_or_else(|| self.texture_transparency(texture))
            })
            .unwrap_or(fallback[model_quad.face.index()])
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

    fn model_quad_tint(
        &self,
        block_name: &str,
        material: bbb_world::TerrainMaterialClass,
        model_quad: &bbb_pack::BlockModelQuad,
        fallback: [TerrainTint; 6],
        biome_id: Option<i32>,
        position: Option<BlockRenderPosition>,
    ) -> TerrainTint {
        if model_quad.texture.is_some() {
            self.block_tint(
                block_name,
                material,
                model_quad.tint_index,
                biome_id,
                position,
            )
        } else {
            fallback[model_quad.face.index()]
        }
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

pub(crate) fn load_terrain_textures(
    renderer: &mut bbb_renderer::Renderer,
    roots: Option<&PackRoots>,
) -> TerrainTextureState {
    let Some(roots) = roots else {
        tracing::warn!("falling back to default terrain texture atlas without pack roots");
        return TerrainTextureState::default();
    };
    match try_load_terrain_textures(renderer, roots) {
        Ok(textures) => textures,
        Err(err) => {
            tracing::warn!(?err, "falling back to default terrain texture atlas");
            TerrainTextureState::default()
        }
    }
}

fn try_load_terrain_textures(
    renderer: &mut bbb_renderer::Renderer,
    roots: &PackRoots,
) -> Result<TerrainTextureState> {
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
    let packer = AtlasPacker::new(4096, 1)?;
    let atlas = packer.stitch_mips_with_max_level(&images, VANILLA_DEFAULT_MIPMAP_LEVELS)?;
    let mip_rgba = atlas.rgba_slices();
    renderer.upload_terrain_texture_atlas_mips(
        atlas.layout.width,
        atlas.layout.height,
        &mip_rgba,
    )?;
    let animated_sprites = images
        .iter()
        .filter(|image| image.animation.is_some())
        .count();
    tracing::info!(
        width = atlas.layout.width,
        height = atlas.layout.height,
        mip_level = atlas.mip_level(),
        sprites = atlas.layout.sprites.len(),
        animated_sprites,
        blockstates = block_models.len(),
        colormaps = colormaps.is_some(),
        biome_colors = biome_colors.as_ref().map_or(0, |colors| colors.len()),
        "loaded terrain texture atlas"
    );
    let mut textures = TerrainTextureState::from_layout_and_images(
        &atlas.layout,
        &images,
        Some(block_models),
        colormaps,
        biome_colors,
    );
    textures.animation =
        TerrainTextureAnimation::new(packer, images, VANILLA_DEFAULT_MIPMAP_LEVELS);
    Ok(textures)
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

fn terrain_transparency(transparency: bbb_pack::SpriteTransparency) -> TerrainTransparency {
    TerrainTransparency {
        has_transparent: transparency.has_transparent,
        has_translucent: transparency.has_translucent,
    }
}

fn force_translucent(force: bool) -> TerrainTransparency {
    if force {
        TerrainTransparency::TRANSLUCENT
    } else {
        TerrainTransparency::OPAQUE
    }
}

fn quad_uv_crop(uvs: [[f32; 2]; 4]) -> Option<[u8; 4]> {
    let min_u = uvs
        .iter()
        .map(|uv| uv[0])
        .try_fold(f32::INFINITY, finite_min)?;
    let min_v = uvs
        .iter()
        .map(|uv| uv[1])
        .try_fold(f32::INFINITY, finite_min)?;
    let max_u = uvs
        .iter()
        .map(|uv| uv[0])
        .try_fold(f32::NEG_INFINITY, finite_max)?;
    let max_v = uvs
        .iter()
        .map(|uv| uv[1])
        .try_fold(f32::NEG_INFINITY, finite_max)?;
    Some([
        quantize_model_uv(min_u)?,
        quantize_model_uv(min_v)?,
        quantize_model_uv(max_u)?,
        quantize_model_uv(max_v)?,
    ])
}

fn finite_min(current: f32, value: f32) -> Option<f32> {
    value.is_finite().then_some(current.min(value))
}

fn finite_max(current: f32, value: f32) -> Option<f32> {
    value.is_finite().then_some(current.max(value))
}

fn quantize_model_uv(value: f32) -> Option<u8> {
    let scaled = value * 16.0;
    let rounded = scaled.round();
    ((0.0..=16.0).contains(&rounded) && (scaled - rounded).abs() < 0.001).then_some(rounded as u8)
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
        face_transparency: [TerrainTransparency::OPAQUE; 6],
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

/// One element of the item-frame border model: opaque, shaded, unculled quads whose present faces map
/// their `0..=16` UV crop into the named atlas texture. This path keeps the vanilla model templates'
/// fractional Z depths without broadening the integer `TerrainBox` representation used by normal blocks.
fn frame_border_box_quads(
    from: [f32; 3],
    to: [f32; 3],
    face_present: [bool; 6],
    face_uvs: [[u8; 4]; 6],
    texture_indices: [u32; 6],
) -> Vec<TerrainQuad> {
    TerrainFace::ALL
        .into_iter()
        .filter_map(|face| {
            let index = terrain_face_index(face);
            face_present[index].then_some(TerrainQuad {
                corners: frame_border_face_corners(face, from, to),
                normal: terrain_face_normal(face),
                uvs: frame_border_face_uvs(face_uvs[index]),
                cull: Some(face),
                texture_index: texture_indices[index],
                tint: TerrainTint::WHITE,
                transparency: TerrainTransparency::OPAQUE,
                shade: true,
                light_emission: 0,
            })
        })
        .collect()
}

fn frame_border_face_corners(face: TerrainFace, min: [f32; 3], max: [f32; 3]) -> [[f32; 3]; 4] {
    match face {
        TerrainFace::Down => [
            [min[0], min[1], max[2]],
            [max[0], min[1], max[2]],
            [max[0], min[1], min[2]],
            [min[0], min[1], min[2]],
        ],
        TerrainFace::Up => [
            [min[0], max[1], min[2]],
            [max[0], max[1], min[2]],
            [max[0], max[1], max[2]],
            [min[0], max[1], max[2]],
        ],
        TerrainFace::North => [
            [max[0], min[1], min[2]],
            [max[0], max[1], min[2]],
            [min[0], max[1], min[2]],
            [min[0], min[1], min[2]],
        ],
        TerrainFace::South => [
            [min[0], min[1], max[2]],
            [min[0], max[1], max[2]],
            [max[0], max[1], max[2]],
            [max[0], min[1], max[2]],
        ],
        TerrainFace::West => [
            [min[0], min[1], min[2]],
            [min[0], max[1], min[2]],
            [min[0], max[1], max[2]],
            [min[0], min[1], max[2]],
        ],
        TerrainFace::East => [
            [max[0], min[1], max[2]],
            [max[0], max[1], max[2]],
            [max[0], max[1], min[2]],
            [max[0], min[1], min[2]],
        ],
    }
}

fn frame_border_face_uvs(uv: [u8; 4]) -> [[f32; 2]; 4] {
    [
        [uv[0] as f32 / 16.0, uv[1] as f32 / 16.0],
        [uv[2] as f32 / 16.0, uv[1] as f32 / 16.0],
        [uv[2] as f32 / 16.0, uv[3] as f32 / 16.0],
        [uv[0] as f32 / 16.0, uv[3] as f32 / 16.0],
    ]
}

fn terrain_face_normal(face: TerrainFace) -> [f32; 3] {
    match face {
        TerrainFace::Down => [0.0, -1.0, 0.0],
        TerrainFace::Up => [0.0, 1.0, 0.0],
        TerrainFace::North => [0.0, 0.0, -1.0],
        TerrainFace::South => [0.0, 0.0, 1.0],
        TerrainFace::West => [-1.0, 0.0, 0.0],
        TerrainFace::East => [1.0, 0.0, 0.0],
    }
}

fn terrain_face_index(face: TerrainFace) -> usize {
    match face {
        TerrainFace::Down => 0,
        TerrainFace::Up => 1,
        TerrainFace::North => 2,
        TerrainFace::South => 3,
        TerrainFace::West => 4,
        TerrainFace::East => 5,
    }
}

#[cfg(test)]
mod tests;
