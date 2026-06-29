//! Item-frame filled-map surfaces and vanilla `MapRenderer` decoration sprite submissions.

use std::collections::BTreeMap;

use glam::{Mat4, Vec3};

use crate::{
    HudAsciiGlyph, Renderer, HUD_ASCII_FIRST_GLYPH, HUD_ASCII_GLYPH_COUNT, HUD_ASCII_LAST_GLYPH,
};

use super::{ItemModelMesh, ItemModelVertex};

const ITEM_FRAME_MAP_SIZE: u32 = 128;
const ITEM_FRAME_MAP_RGBA_LEN: usize =
    ITEM_FRAME_MAP_SIZE as usize * ITEM_FRAME_MAP_SIZE as usize * 4;
const ITEM_FRAME_MAP_DECORATION_ATLAS_PATH: &str = "minecraft:textures/atlas/map_decorations.png";
const ITEM_FRAME_MAP_TEXT_FONT_PATH: &str = "minecraft:textures/font/ascii.png";
const ITEM_FRAME_MAP_TEXT_REPLACEMENT_GLYPH: u8 = b'?';

/// Decoded RGBA pixels for vanilla's dynamic `minecraft:map/<id>` texture. The renderer packs these
/// 128x128 textures into a per-frame map atlas and draws item-frame maps as textured quads, matching
/// `MapTextureManager.prepareMapTexture` plus `MapRenderer.render`'s base surface submit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ItemFrameMapTexture {
    pub map_id: i32,
    pub rgba: Vec<u8>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ItemFrameMapRenderType {
    Text,
}

impl ItemFrameMapRenderType {
    pub fn vanilla_name(self) -> &'static str {
        match self {
            Self::Text => "text",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ItemFrameMapTextureRef {
    pub map_id: i32,
}

impl ItemFrameMapTextureRef {
    pub fn vanilla_path(self) -> String {
        format!("minecraft:map/{}", self.map_id)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ItemFrameMapSubmission {
    pub map_id: i32,
    pub render_type: ItemFrameMapRenderType,
    pub texture: ItemFrameMapTextureRef,
    pub tint: [f32; 4],
    pub transform: Mat4,
    pub light: [f32; 2],
    pub order: u32,
    pub submit_sequence: u32,
}

/// Vanilla `MapDecorationTypes` entry projected from the registry id carried by
/// `ClientboundMapItemDataPacket`. The order mirrors the static registrations in
/// `MapDecorationTypes.java`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ItemFrameMapDecorationType {
    pub type_id: i32,
    pub sprite_id: &'static str,
    pub render_on_frame: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ItemFrameMapDecorationTexture {
    pub sprite_id: String,
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ItemFrameMapDecorationTextureRef {
    pub sprite_id: &'static str,
}

impl ItemFrameMapDecorationTextureRef {
    pub fn vanilla_atlas_path(self) -> &'static str {
        ITEM_FRAME_MAP_DECORATION_ATLAS_PATH
    }

    pub fn vanilla_sprite_id(self) -> &'static str {
        self.sprite_id
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ItemFrameMapDecorationSubmission {
    pub type_id: i32,
    pub render_type: ItemFrameMapRenderType,
    pub texture: ItemFrameMapDecorationTextureRef,
    pub tint: [f32; 4],
    pub transform: Mat4,
    pub light: [f32; 2],
    pub order: u32,
    pub submit_sequence: u32,
    pub decoration_index: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ItemFrameMapTextTextureRef;

impl ItemFrameMapTextTextureRef {
    pub fn vanilla_path(self) -> &'static str {
        ITEM_FRAME_MAP_TEXT_FONT_PATH
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ItemFrameMapTextSubmission {
    pub type_id: i32,
    pub text: String,
    pub render_type: ItemFrameMapRenderType,
    pub texture: ItemFrameMapTextTextureRef,
    pub tint: [f32; 4],
    pub transform: Mat4,
    pub light: [f32; 2],
    pub order: u32,
    pub submit_sequence: u32,
    pub decoration_index: u32,
    pub width: f32,
    pub scale: f32,
}

/// The base map surface submit for an item frame. `mesh` is the single vanilla `MapRenderer` quad in
/// world space with local 0..1 UVs; the renderer remaps those UVs to the dynamic map atlas at draw time.
#[derive(Debug, Clone, PartialEq)]
pub struct ItemFrameMapSurface {
    pub submission: ItemFrameMapSubmission,
    mesh: ItemModelMesh,
}

impl ItemFrameMapSurface {
    pub fn is_empty(&self) -> bool {
        self.mesh.is_empty()
    }

    pub fn vertex_count(&self) -> usize {
        self.mesh.vertices.len()
    }

    pub fn index_count(&self) -> usize {
        self.mesh.indices.len()
    }
}

/// A vanilla `MapRenderer` decoration sprite submit for an item-frame map. `mesh` carries the single
/// local sprite quad with UVs in 0..1; the renderer remaps those UVs to the map-decoration atlas.
#[derive(Debug, Clone, PartialEq)]
pub struct ItemFrameMapDecorationSurface {
    pub submission: ItemFrameMapDecorationSubmission,
    mesh: ItemModelMesh,
}

/// A vanilla `MapRenderer` decoration name text submit for an item-frame map. `mesh` contains one
/// textured quad per glyph in world space and samples the vanilla ASCII font atlas uploaded by native.
#[derive(Debug, Clone, PartialEq)]
pub struct ItemFrameMapTextSurface {
    pub submission: ItemFrameMapTextSubmission,
    mesh: ItemModelMesh,
}

impl ItemFrameMapTextSurface {
    pub fn is_empty(&self) -> bool {
        self.mesh.is_empty()
    }

    pub fn vertex_count(&self) -> usize {
        self.mesh.vertices.len()
    }

    pub fn index_count(&self) -> usize {
        self.mesh.indices.len()
    }
}

impl ItemFrameMapDecorationSurface {
    pub fn is_empty(&self) -> bool {
        self.mesh.is_empty()
    }

    pub fn vertex_count(&self) -> usize {
        self.mesh.vertices.len()
    }

    pub fn index_count(&self) -> usize {
        self.mesh.indices.len()
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct ItemFrameMapUvRect {
    min: [f32; 2],
    max: [f32; 2],
}

impl ItemFrameMapUvRect {
    fn map(self, uv: [f32; 2]) -> [f32; 2] {
        [
            self.min[0] + (self.max[0] - self.min[0]) * uv[0],
            self.min[1] + (self.max[1] - self.min[1]) * uv[1],
        ]
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ItemFrameMapAtlasLayout {
    width: u32,
    height: u32,
    rects: BTreeMap<i32, ItemFrameMapUvRect>,
}

pub(crate) struct ItemFrameMapAtlasGpu {
    _texture: wgpu::Texture,
    _view: wgpu::TextureView,
    _sampler: wgpu::Sampler,
    pub(crate) bind_group: wgpu::BindGroup,
    pub(crate) layout: ItemFrameMapAtlasLayout,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ItemFrameMapDecorationAtlasLayout {
    width: u32,
    height: u32,
    rects: BTreeMap<String, ItemFrameMapUvRect>,
}

pub(crate) struct ItemFrameMapDecorationAtlasGpu {
    _texture: wgpu::Texture,
    _view: wgpu::TextureView,
    _sampler: wgpu::Sampler,
    pub(crate) bind_group: wgpu::BindGroup,
    pub(crate) layout: ItemFrameMapDecorationAtlasLayout,
}

pub(crate) struct ItemFrameMapTextFontAtlasGpu {
    _texture: wgpu::Texture,
    _view: wgpu::TextureView,
    _sampler: wgpu::Sampler,
    pub(crate) bind_group: wgpu::BindGroup,
}

const ITEM_FRAME_MAP_DECORATION_TYPES: &[(i32, &str, bool)] = &[
    (0, "minecraft:player", false),
    (1, "minecraft:frame", true),
    (2, "minecraft:red_marker", false),
    (3, "minecraft:blue_marker", false),
    (4, "minecraft:target_x", true),
    (5, "minecraft:target_point", true),
    (6, "minecraft:player_off_map", false),
    (7, "minecraft:player_off_limits", false),
    (8, "minecraft:woodland_mansion", true),
    (9, "minecraft:ocean_monument", true),
    (10, "minecraft:white_banner", true),
    (11, "minecraft:orange_banner", true),
    (12, "minecraft:magenta_banner", true),
    (13, "minecraft:light_blue_banner", true),
    (14, "minecraft:yellow_banner", true),
    (15, "minecraft:lime_banner", true),
    (16, "minecraft:pink_banner", true),
    (17, "minecraft:gray_banner", true),
    (18, "minecraft:light_gray_banner", true),
    (19, "minecraft:cyan_banner", true),
    (20, "minecraft:purple_banner", true),
    (21, "minecraft:blue_banner", true),
    (22, "minecraft:brown_banner", true),
    (23, "minecraft:green_banner", true),
    (24, "minecraft:red_banner", true),
    (25, "minecraft:black_banner", true),
    (26, "minecraft:red_x", true),
    (27, "minecraft:desert_village", true),
    (28, "minecraft:plains_village", true),
    (29, "minecraft:savanna_village", true),
    (30, "minecraft:snowy_village", true),
    (31, "minecraft:taiga_village", true),
    (32, "minecraft:jungle_temple", true),
    (33, "minecraft:swamp_hut", true),
    (34, "minecraft:trial_chambers", true),
];

pub fn item_frame_map_decoration_type(type_id: i32) -> Option<ItemFrameMapDecorationType> {
    ITEM_FRAME_MAP_DECORATION_TYPES
        .iter()
        .copied()
        .find(|(candidate, _, _)| *candidate == type_id)
        .map(
            |(type_id, sprite_id, render_on_frame)| ItemFrameMapDecorationType {
                type_id,
                sprite_id,
                render_on_frame,
            },
        )
}

/// Bakes vanilla `MapRenderer.render`'s base map surface submit:
/// `RenderTypes.text(minecraft:map/<id>)`, white tint, order 0 / sequence 0, and the four
/// `(0,128,-0.01)..(0,0,-0.01)` vertices with UVs 0..1.
pub fn bake_item_frame_map_surface(
    map_id: i32,
    transform: Mat4,
    light: [f32; 2],
) -> ItemFrameMapSurface {
    let mut mesh = ItemModelMesh::new();
    let corners = [
        transform
            .transform_point3(Vec3::new(0.0, 128.0, -0.01))
            .to_array(),
        transform
            .transform_point3(Vec3::new(128.0, 128.0, -0.01))
            .to_array(),
        transform
            .transform_point3(Vec3::new(128.0, 0.0, -0.01))
            .to_array(),
        transform
            .transform_point3(Vec3::new(0.0, 0.0, -0.01))
            .to_array(),
    ];
    mesh.append_raw_textured_quad(
        corners,
        [[0.0, 1.0], [1.0, 1.0], [1.0, 0.0], [0.0, 0.0]],
        [1.0, 1.0, 1.0, 1.0],
        light,
    );
    ItemFrameMapSurface {
        submission: ItemFrameMapSubmission {
            map_id,
            render_type: ItemFrameMapRenderType::Text,
            texture: ItemFrameMapTextureRef { map_id },
            tint: [1.0, 1.0, 1.0, 1.0],
            transform,
            light,
            order: 0,
            submit_sequence: 0,
        },
        mesh,
    }
}

/// Bakes one item-frame-visible vanilla `MapRenderer` decoration sprite submit. The `map_transform`
/// parameter is the same pose stack transform used for the base map surface; this function appends the
/// decoration-local translate/rotate/scale before writing the `(-1..1)` sprite quad. Player/off-map
/// markers return `None` here because `ItemFrameRenderer` calls `MapRenderer.render(..., true, ...)`.
pub fn bake_item_frame_map_decoration_surface(
    type_id: i32,
    x: i8,
    y: i8,
    rot: u8,
    decoration_index: u32,
    map_transform: Mat4,
    light: [f32; 2],
    submit_sequence: u32,
) -> Option<ItemFrameMapDecorationSurface> {
    let decoration_type = item_frame_map_decoration_type(type_id)?;
    if !decoration_type.render_on_frame {
        return None;
    }
    let transform = map_transform
        * Mat4::from_translation(Vec3::new(
            f32::from(x) / 2.0 + 64.0,
            f32::from(y) / 2.0 + 64.0,
            -0.02,
        ))
        * Mat4::from_rotation_z((f32::from(rot & 15) * 360.0 / 16.0).to_radians())
        * Mat4::from_scale(Vec3::new(4.0, 4.0, 3.0))
        * Mat4::from_translation(Vec3::new(-0.125, 0.125, 0.0));
    let z = decoration_index as f32 * -0.001;
    let corners = [
        transform
            .transform_point3(Vec3::new(-1.0, 1.0, z))
            .to_array(),
        transform
            .transform_point3(Vec3::new(1.0, 1.0, z))
            .to_array(),
        transform
            .transform_point3(Vec3::new(1.0, -1.0, z))
            .to_array(),
        transform
            .transform_point3(Vec3::new(-1.0, -1.0, z))
            .to_array(),
    ];
    let mut mesh = ItemModelMesh::new();
    mesh.append_raw_textured_quad(
        corners,
        [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]],
        [1.0, 1.0, 1.0, 1.0],
        light,
    );
    Some(ItemFrameMapDecorationSurface {
        submission: ItemFrameMapDecorationSubmission {
            type_id,
            render_type: ItemFrameMapRenderType::Text,
            texture: ItemFrameMapDecorationTextureRef {
                sprite_id: decoration_type.sprite_id,
            },
            tint: [1.0, 1.0, 1.0, 1.0],
            transform,
            light,
            order: 0,
            submit_sequence,
            decoration_index,
        },
        mesh,
    })
}

/// Bakes one item-frame-visible vanilla `MapRenderer` decoration name submit. `MapRenderer.render`
/// submits these labels through `submitNodeCollector.order(1).submitText(...)` after the order-0
/// base map and decoration sprite submissions. The width and clamp mirror vanilla `Font.width` plus
/// `Mth.clamp(25.0F / width, 0.0F, 6.0F / 9.0F)` for the currently supported ASCII font atlas.
pub fn bake_item_frame_map_text_surface(
    type_id: i32,
    text: impl Into<String>,
    x: i8,
    y: i8,
    decoration_index: u32,
    map_transform: Mat4,
    light: [f32; 2],
    submit_sequence: u32,
    glyphs: &[HudAsciiGlyph; HUD_ASCII_GLYPH_COUNT],
) -> Option<ItemFrameMapTextSurface> {
    let decoration_type = item_frame_map_decoration_type(type_id)?;
    if !decoration_type.render_on_frame {
        return None;
    }
    let text = text.into();
    let width = item_frame_map_text_width(&text, glyphs)? as f32;
    let scale = (25.0 / width).clamp(0.0, 6.0 / 9.0);
    let transform = map_transform
        * Mat4::from_translation(Vec3::new(
            f32::from(x) / 2.0 + 64.0 - width * scale / 2.0,
            f32::from(y) / 2.0 + 64.0 + 4.0,
            -0.025,
        ))
        * Mat4::from_scale(Vec3::new(scale, scale, -1.0))
        * Mat4::from_translation(Vec3::new(0.0, 0.0, 0.1));

    let mut mesh = ItemModelMesh::new();
    let mut pen_x = 0.0;
    for ch in text.chars() {
        let glyph = item_frame_map_text_glyph(ch, glyphs);
        if glyph.width > 0 && glyph.height > 0 {
            let x0 = pen_x;
            let x1 = pen_x + glyph.width as f32;
            let y0 = 0.0;
            let y1 = glyph.height as f32;
            let corners = [
                transform
                    .transform_point3(Vec3::new(x0, y0, 0.0))
                    .to_array(),
                transform
                    .transform_point3(Vec3::new(x1, y0, 0.0))
                    .to_array(),
                transform
                    .transform_point3(Vec3::new(x1, y1, 0.0))
                    .to_array(),
                transform
                    .transform_point3(Vec3::new(x0, y1, 0.0))
                    .to_array(),
            ];
            mesh.append_raw_textured_quad(
                corners,
                [
                    [glyph.uv.min[0], glyph.uv.min[1]],
                    [glyph.uv.max[0], glyph.uv.min[1]],
                    [glyph.uv.max[0], glyph.uv.max[1]],
                    [glyph.uv.min[0], glyph.uv.max[1]],
                ],
                [1.0, 1.0, 1.0, 1.0],
                light,
            );
        }
        pen_x += glyph.advance as f32;
    }

    Some(ItemFrameMapTextSurface {
        submission: ItemFrameMapTextSubmission {
            type_id,
            text,
            render_type: ItemFrameMapRenderType::Text,
            texture: ItemFrameMapTextTextureRef,
            tint: [1.0, 1.0, 1.0, 1.0],
            transform,
            light,
            order: 1,
            submit_sequence,
            decoration_index,
            width,
            scale,
        },
        mesh,
    })
}

pub fn item_frame_map_text_width(
    text: &str,
    glyphs: &[HudAsciiGlyph; HUD_ASCII_GLYPH_COUNT],
) -> Option<u32> {
    let mut width = 0u32;
    for ch in text.chars() {
        width = width.checked_add(item_frame_map_text_glyph(ch, glyphs).advance)?;
    }
    (width > 0).then_some(width)
}

fn item_frame_map_text_glyph(
    ch: char,
    glyphs: &[HudAsciiGlyph; HUD_ASCII_GLYPH_COUNT],
) -> HudAsciiGlyph {
    let byte = if ch.is_ascii() {
        ch as u8
    } else {
        ITEM_FRAME_MAP_TEXT_REPLACEMENT_GLYPH
    };
    let byte = if (HUD_ASCII_FIRST_GLYPH..=HUD_ASCII_LAST_GLYPH).contains(&byte) {
        byte
    } else {
        ITEM_FRAME_MAP_TEXT_REPLACEMENT_GLYPH
    };
    glyphs[(byte - HUD_ASCII_FIRST_GLYPH) as usize]
}

pub(crate) fn merge_item_frame_map_surfaces(
    surfaces: &[ItemFrameMapSurface],
    atlas: &ItemFrameMapAtlasLayout,
) -> (Vec<ItemModelVertex>, Vec<u32>) {
    let mut vertices = Vec::new();
    let mut indices = Vec::new();
    for surface in surfaces {
        let Some(rect) = atlas.rects.get(&surface.submission.map_id).copied() else {
            continue;
        };
        let base = u32::try_from(vertices.len()).expect("item-frame map vertex count fits in u32");
        vertices.extend(surface.mesh.vertices.iter().copied().map(|mut vertex| {
            vertex.uv = rect.map(vertex.uv);
            vertex
        }));
        indices.extend(surface.mesh.indices.iter().map(|index| index + base));
    }
    (vertices, indices)
}

pub(crate) fn merge_item_frame_map_text_surfaces(
    surfaces: &[ItemFrameMapTextSurface],
) -> (Vec<ItemModelVertex>, Vec<u32>) {
    let mut vertices = Vec::new();
    let mut indices = Vec::new();
    for surface in surfaces {
        let base =
            u32::try_from(vertices.len()).expect("item-frame map text vertex count fits in u32");
        vertices.extend(surface.mesh.vertices.iter().copied());
        indices.extend(surface.mesh.indices.iter().map(|index| index + base));
    }
    (vertices, indices)
}

pub(crate) fn merge_item_frame_map_decoration_surfaces(
    surfaces: &[ItemFrameMapDecorationSurface],
    atlas: &ItemFrameMapDecorationAtlasLayout,
) -> (Vec<ItemModelVertex>, Vec<u32>) {
    let mut vertices = Vec::new();
    let mut indices = Vec::new();
    for surface in surfaces {
        let Some(rect) = atlas
            .rects
            .get(surface.submission.texture.sprite_id)
            .copied()
        else {
            continue;
        };
        let base = u32::try_from(vertices.len())
            .expect("item-frame map decoration vertex count fits in u32");
        vertices.extend(surface.mesh.vertices.iter().copied().map(|mut vertex| {
            vertex.uv = rect.map(vertex.uv);
            vertex
        }));
        indices.extend(surface.mesh.indices.iter().map(|index| index + base));
    }
    (vertices, indices)
}

impl Renderer {
    /// Sets this frame's filled-map item-frame submissions and their dynamic 128x128 map textures. The
    /// textures are packed into a transient atlas for the existing item-model shader; surfaces whose map
    /// texture is absent or malformed are skipped.
    pub fn set_item_frame_map_surfaces(
        &mut self,
        textures: Vec<ItemFrameMapTexture>,
        surfaces: Vec<ItemFrameMapSurface>,
    ) {
        self.item_frame_map_atlas = build_item_frame_map_atlas(&textures).map(|(layout, rgba)| {
            create_item_frame_map_atlas_gpu(
                &self.device,
                &self.queue,
                &self.terrain_bind_group_layout,
                &self.camera_buffer,
                layout,
                &rgba,
            )
        });
        self.item_frame_map_surfaces = if let Some(atlas) = &self.item_frame_map_atlas {
            surfaces
                .into_iter()
                .filter(|surface| {
                    !surface.is_empty()
                        && atlas.layout.rects.contains_key(&surface.submission.map_id)
                })
                .collect()
        } else {
            Vec::new()
        };
    }

    /// Sets this frame's `MapRenderer` decoration sprite submissions for filled-map item frames. The
    /// atlas is the vanilla `textures/atlas/map_decorations.png` sheet, rebuilt here from the decoded
    /// pack sprites supplied by native.
    pub fn set_item_frame_map_decoration_surfaces(
        &mut self,
        textures: Vec<ItemFrameMapDecorationTexture>,
        surfaces: Vec<ItemFrameMapDecorationSurface>,
    ) {
        self.item_frame_map_decoration_atlas = build_item_frame_map_decoration_atlas(&textures)
            .map(|(layout, rgba)| {
                create_item_frame_map_decoration_atlas_gpu(
                    &self.device,
                    &self.queue,
                    &self.terrain_bind_group_layout,
                    &self.camera_buffer,
                    layout,
                    &rgba,
                )
            });
        self.item_frame_map_decoration_surfaces =
            if let Some(atlas) = &self.item_frame_map_decoration_atlas {
                surfaces
                    .into_iter()
                    .filter(|surface| {
                        !surface.is_empty()
                            && atlas
                                .layout
                                .rects
                                .contains_key(surface.submission.texture.sprite_id)
                    })
                    .collect()
            } else {
                Vec::new()
            };
    }

    /// Uploads the vanilla ASCII font atlas for world-space item-frame map label text. Native also uses
    /// the same glyph metrics when baking the text submissions, so width-based label placement and UVs
    /// stay in sync.
    pub fn upload_item_frame_map_text_font(&mut self, width: u32, height: u32, rgba: &[u8]) {
        self.item_frame_map_text_font_atlas = create_item_frame_map_text_font_atlas_gpu(
            &self.device,
            &self.queue,
            &self.terrain_bind_group_layout,
            &self.camera_buffer,
            width,
            height,
            rgba,
        );
    }

    pub fn set_item_frame_map_text_surfaces(&mut self, surfaces: Vec<ItemFrameMapTextSurface>) {
        self.item_frame_map_text_surfaces = if self.item_frame_map_text_font_atlas.is_some() {
            surfaces
                .into_iter()
                .filter(|surface| !surface.is_empty())
                .collect()
        } else {
            Vec::new()
        };
    }

    pub(crate) fn collect_item_frame_map_geometry(&self) -> (Vec<ItemModelVertex>, Vec<u32>) {
        let Some(atlas) = &self.item_frame_map_atlas else {
            return (Vec::new(), Vec::new());
        };
        merge_item_frame_map_surfaces(&self.item_frame_map_surfaces, &atlas.layout)
    }

    pub(crate) fn collect_item_frame_map_decoration_geometry(
        &self,
    ) -> (Vec<ItemModelVertex>, Vec<u32>) {
        let Some(atlas) = &self.item_frame_map_decoration_atlas else {
            return (Vec::new(), Vec::new());
        };
        merge_item_frame_map_decoration_surfaces(
            &self.item_frame_map_decoration_surfaces,
            &atlas.layout,
        )
    }

    pub(crate) fn collect_item_frame_map_text_geometry(&self) -> (Vec<ItemModelVertex>, Vec<u32>) {
        if self.item_frame_map_text_font_atlas.is_none() {
            return (Vec::new(), Vec::new());
        }
        merge_item_frame_map_text_surfaces(&self.item_frame_map_text_surfaces)
    }
}

fn build_item_frame_map_atlas(
    textures: &[ItemFrameMapTexture],
) -> Option<(ItemFrameMapAtlasLayout, Vec<u8>)> {
    let mut by_id: BTreeMap<i32, &[u8]> = BTreeMap::new();
    for texture in textures {
        if texture.rgba.len() == ITEM_FRAME_MAP_RGBA_LEN {
            by_id.insert(texture.map_id, &texture.rgba);
        }
    }
    let map_count = u32::try_from(by_id.len()).ok()?;
    if map_count == 0 {
        return None;
    }
    let width = ITEM_FRAME_MAP_SIZE;
    let height = ITEM_FRAME_MAP_SIZE.checked_mul(map_count)?;
    let mut atlas_rgba = vec![0; width as usize * height as usize * 4];
    let mut rects = BTreeMap::new();
    let per_map_len = ITEM_FRAME_MAP_RGBA_LEN;
    for (slot, (map_id, rgba)) in by_id.into_iter().enumerate() {
        let dst = slot * per_map_len;
        atlas_rgba[dst..dst + per_map_len].copy_from_slice(rgba);
        let v0 = slot as f32 / map_count as f32;
        let v1 = (slot as f32 + 1.0) / map_count as f32;
        rects.insert(
            map_id,
            ItemFrameMapUvRect {
                min: [0.0, v0],
                max: [1.0, v1],
            },
        );
    }
    Some((
        ItemFrameMapAtlasLayout {
            width,
            height,
            rects,
        },
        atlas_rgba,
    ))
}

fn create_item_frame_map_atlas_gpu(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    bind_group_layout: &wgpu::BindGroupLayout,
    camera_buffer: &wgpu::Buffer,
    layout: ItemFrameMapAtlasLayout,
    rgba: &[u8],
) -> ItemFrameMapAtlasGpu {
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("bbb-item-frame-map-atlas-texture"),
        size: wgpu::Extent3d {
            width: layout.width,
            height: layout.height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8UnormSrgb,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });
    queue.write_texture(
        wgpu::ImageCopyTexture {
            texture: &texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        rgba,
        wgpu::ImageDataLayout {
            offset: 0,
            bytes_per_row: Some(layout.width * 4),
            rows_per_image: Some(layout.height),
        },
        wgpu::Extent3d {
            width: layout.width,
            height: layout.height,
            depth_or_array_layers: 1,
        },
    );
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
        label: Some("bbb-item-frame-map-atlas-sampler"),
        address_mode_u: wgpu::AddressMode::ClampToEdge,
        address_mode_v: wgpu::AddressMode::ClampToEdge,
        address_mode_w: wgpu::AddressMode::ClampToEdge,
        mag_filter: wgpu::FilterMode::Nearest,
        min_filter: wgpu::FilterMode::Nearest,
        mipmap_filter: wgpu::FilterMode::Nearest,
        ..Default::default()
    });
    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("bbb-item-frame-map-atlas-bind-group"),
        layout: bind_group_layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: camera_buffer.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::TextureView(&view),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: wgpu::BindingResource::Sampler(&sampler),
            },
        ],
    });
    ItemFrameMapAtlasGpu {
        _texture: texture,
        _view: view,
        _sampler: sampler,
        bind_group,
        layout,
    }
}

fn build_item_frame_map_decoration_atlas(
    textures: &[ItemFrameMapDecorationTexture],
) -> Option<(ItemFrameMapDecorationAtlasLayout, Vec<u8>)> {
    let mut by_id: BTreeMap<&str, &ItemFrameMapDecorationTexture> = BTreeMap::new();
    for texture in textures {
        let expected_len = texture
            .width
            .checked_mul(texture.height)
            .and_then(|pixels| pixels.checked_mul(4))
            .and_then(|len| usize::try_from(len).ok());
        if texture.width > 0 && texture.height > 0 && expected_len == Some(texture.rgba.len()) {
            by_id.insert(texture.sprite_id.as_str(), texture);
        }
    }
    if by_id.is_empty() {
        return None;
    }
    let width = by_id.values().map(|texture| texture.width).max()?;
    let height = by_id
        .values()
        .try_fold(0u32, |height, texture| height.checked_add(texture.height))?;
    let atlas_len = width
        .checked_mul(height)
        .and_then(|pixels| pixels.checked_mul(4))
        .and_then(|len| usize::try_from(len).ok())?;
    let mut atlas_rgba = vec![0; atlas_len];
    let mut rects = BTreeMap::new();
    let mut y_offset = 0u32;
    for (sprite_id, texture) in by_id {
        for y in 0..texture.height {
            let src = usize::try_from((y * texture.width) * 4).ok()?;
            let dst = usize::try_from(((y_offset + y) * width) * 4).ok()?;
            let row_len = usize::try_from(texture.width * 4).ok()?;
            atlas_rgba[dst..dst + row_len].copy_from_slice(&texture.rgba[src..src + row_len]);
        }
        rects.insert(
            sprite_id.to_string(),
            ItemFrameMapUvRect {
                min: [0.0, y_offset as f32 / height as f32],
                max: [
                    texture.width as f32 / width as f32,
                    (y_offset + texture.height) as f32 / height as f32,
                ],
            },
        );
        y_offset += texture.height;
    }
    Some((
        ItemFrameMapDecorationAtlasLayout {
            width,
            height,
            rects,
        },
        atlas_rgba,
    ))
}

fn create_item_frame_map_decoration_atlas_gpu(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    bind_group_layout: &wgpu::BindGroupLayout,
    camera_buffer: &wgpu::Buffer,
    layout: ItemFrameMapDecorationAtlasLayout,
    rgba: &[u8],
) -> ItemFrameMapDecorationAtlasGpu {
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("bbb-item-frame-map-decoration-atlas-texture"),
        size: wgpu::Extent3d {
            width: layout.width,
            height: layout.height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8UnormSrgb,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });
    queue.write_texture(
        wgpu::ImageCopyTexture {
            texture: &texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        rgba,
        wgpu::ImageDataLayout {
            offset: 0,
            bytes_per_row: Some(layout.width * 4),
            rows_per_image: Some(layout.height),
        },
        wgpu::Extent3d {
            width: layout.width,
            height: layout.height,
            depth_or_array_layers: 1,
        },
    );
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
        label: Some("bbb-item-frame-map-decoration-atlas-sampler"),
        address_mode_u: wgpu::AddressMode::ClampToEdge,
        address_mode_v: wgpu::AddressMode::ClampToEdge,
        address_mode_w: wgpu::AddressMode::ClampToEdge,
        mag_filter: wgpu::FilterMode::Nearest,
        min_filter: wgpu::FilterMode::Nearest,
        mipmap_filter: wgpu::FilterMode::Nearest,
        ..Default::default()
    });
    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("bbb-item-frame-map-decoration-atlas-bind-group"),
        layout: bind_group_layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: camera_buffer.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::TextureView(&view),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: wgpu::BindingResource::Sampler(&sampler),
            },
        ],
    });
    ItemFrameMapDecorationAtlasGpu {
        _texture: texture,
        _view: view,
        _sampler: sampler,
        bind_group,
        layout,
    }
}

fn create_item_frame_map_text_font_atlas_gpu(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    bind_group_layout: &wgpu::BindGroupLayout,
    camera_buffer: &wgpu::Buffer,
    width: u32,
    height: u32,
    rgba: &[u8],
) -> Option<ItemFrameMapTextFontAtlasGpu> {
    let expected_len = width
        .checked_mul(height)
        .and_then(|pixels| pixels.checked_mul(4))
        .and_then(|len| usize::try_from(len).ok());
    if width == 0 || height == 0 || expected_len != Some(rgba.len()) {
        return None;
    }
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("bbb-item-frame-map-text-font-atlas-texture"),
        size: wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8UnormSrgb,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });
    queue.write_texture(
        wgpu::ImageCopyTexture {
            texture: &texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        rgba,
        wgpu::ImageDataLayout {
            offset: 0,
            bytes_per_row: Some(width * 4),
            rows_per_image: Some(height),
        },
        wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
    );
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
        label: Some("bbb-item-frame-map-text-font-atlas-sampler"),
        address_mode_u: wgpu::AddressMode::ClampToEdge,
        address_mode_v: wgpu::AddressMode::ClampToEdge,
        address_mode_w: wgpu::AddressMode::ClampToEdge,
        mag_filter: wgpu::FilterMode::Nearest,
        min_filter: wgpu::FilterMode::Nearest,
        mipmap_filter: wgpu::FilterMode::Nearest,
        ..Default::default()
    });
    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("bbb-item-frame-map-text-font-atlas-bind-group"),
        layout: bind_group_layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: camera_buffer.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::TextureView(&view),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: wgpu::BindingResource::Sampler(&sampler),
            },
        ],
    });
    Some(ItemFrameMapTextFontAtlasGpu {
        _texture: texture,
        _view: view,
        _sampler: sampler,
        bind_group,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::item_models::ITEM_MODEL_FULL_BRIGHT_LIGHT;

    #[test]
    fn item_frame_map_surface_uses_dynamic_map_texture_submission() {
        let transform = Mat4::from_translation(Vec3::new(1.0, 2.0, 3.0));
        let light = [13.0 / 15.0, 1.0];
        let surface = bake_item_frame_map_surface(10, transform, light);

        assert_eq!(surface.vertex_count(), 4);
        assert_eq!(surface.index_count(), 6);
        assert_eq!(surface.submission.map_id, 10);
        assert_eq!(surface.submission.render_type, ItemFrameMapRenderType::Text);
        assert_eq!(surface.submission.render_type.vanilla_name(), "text");
        assert_eq!(
            surface.submission.texture.vanilla_path(),
            "minecraft:map/10"
        );
        assert_eq!(surface.submission.tint, [1.0, 1.0, 1.0, 1.0]);
        assert_eq!(surface.submission.transform, transform);
        assert_eq!(surface.submission.light, light);
        assert_eq!(
            (surface.submission.order, surface.submission.submit_sequence),
            (0, 0)
        );

        let first_map_rgba = vec![10; ITEM_FRAME_MAP_RGBA_LEN];
        let second_map_rgba = vec![20; ITEM_FRAME_MAP_RGBA_LEN];
        let (atlas, rgba) = build_item_frame_map_atlas(&[
            ItemFrameMapTexture {
                map_id: 10,
                rgba: second_map_rgba,
            },
            ItemFrameMapTexture {
                map_id: 5,
                rgba: first_map_rgba,
            },
            ItemFrameMapTexture {
                map_id: 99,
                rgba: vec![1, 2, 3],
            },
        ])
        .expect("valid map atlas");

        // The transient atlas is deterministic by map id and ignores malformed RGBA payloads.
        assert_eq!(atlas.width, ITEM_FRAME_MAP_SIZE);
        assert_eq!(atlas.height, ITEM_FRAME_MAP_SIZE * 2);
        assert_eq!(&rgba[..4], &[10, 10, 10, 10]);
        assert_eq!(
            &rgba[ITEM_FRAME_MAP_RGBA_LEN..ITEM_FRAME_MAP_RGBA_LEN + 4],
            &[20, 20, 20, 20]
        );
        assert!(!atlas.rects.contains_key(&99));

        let (vertices, indices) = merge_item_frame_map_surfaces(&[surface], &atlas);
        assert_eq!(indices, vec![0, 1, 2, 0, 2, 3]);
        assert_eq!(vertices.len(), 4);
        assert_eq!(vertices[0].uv, [0.0, 1.0]);
        assert_eq!(vertices[2].uv, [1.0, 0.5]);
        assert!(vertices
            .iter()
            .all(|vertex| vertex.color == [1.0, 1.0, 1.0, 1.0] && vertex.light == light));
    }

    #[test]
    fn item_frame_map_surface_waits_when_dynamic_map_texture_is_absent() {
        let surface = bake_item_frame_map_surface(10, Mat4::IDENTITY, ITEM_MODEL_FULL_BRIGHT_LIGHT);
        let (atlas, _) = build_item_frame_map_atlas(&[ItemFrameMapTexture {
            map_id: 5,
            rgba: vec![20; ITEM_FRAME_MAP_RGBA_LEN],
        }])
        .expect("valid map atlas");

        let (vertices, indices) = merge_item_frame_map_surfaces(&[surface], &atlas);

        assert!(
            vertices.is_empty() && indices.is_empty(),
            "absent dynamic map texture must not fold stale map geometry"
        );
    }

    #[test]
    fn map_decoration_type_mapping_matches_vanilla_registration_order() {
        // Vanilla `MapDecorationTypes` registers these holders in static field order; the packet carries
        // their registry id through `MapDecorationType.STREAM_CODEC`.
        assert_eq!(
            item_frame_map_decoration_type(0),
            Some(ItemFrameMapDecorationType {
                type_id: 0,
                sprite_id: "minecraft:player",
                render_on_frame: false,
            })
        );
        assert_eq!(
            item_frame_map_decoration_type(1),
            Some(ItemFrameMapDecorationType {
                type_id: 1,
                sprite_id: "minecraft:frame",
                render_on_frame: true,
            })
        );
        assert_eq!(
            item_frame_map_decoration_type(26)
                .expect("red_x decoration type")
                .sprite_id,
            "minecraft:red_x"
        );
        assert_eq!(
            item_frame_map_decoration_type(34),
            Some(ItemFrameMapDecorationType {
                type_id: 34,
                sprite_id: "minecraft:trial_chambers",
                render_on_frame: true,
            })
        );
        assert!(item_frame_map_decoration_type(35).is_none());
    }

    #[test]
    fn item_frame_map_decoration_surface_uses_vanilla_text_submission() {
        let map_transform = Mat4::from_translation(Vec3::new(1.0, 2.0, 3.0));
        let light = [7.0 / 15.0, 12.0 / 15.0];
        let surface =
            bake_item_frame_map_decoration_surface(1, -20, 30, 7, 2, map_transform, light, 3)
                .expect("frame decorations render on item-frame maps");

        let expected_transform = map_transform
            * Mat4::from_translation(Vec3::new(-20.0 / 2.0 + 64.0, 30.0 / 2.0 + 64.0, -0.02))
            * Mat4::from_rotation_z((7.0_f32 * 360.0 / 16.0).to_radians())
            * Mat4::from_scale(Vec3::new(4.0, 4.0, 3.0))
            * Mat4::from_translation(Vec3::new(-0.125, 0.125, 0.0));

        assert_eq!(surface.vertex_count(), 4);
        assert_eq!(surface.index_count(), 6);
        assert_eq!(surface.submission.type_id, 1);
        assert_eq!(surface.submission.render_type, ItemFrameMapRenderType::Text);
        assert_eq!(surface.submission.render_type.vanilla_name(), "text");
        assert_eq!(
            surface.submission.texture.vanilla_atlas_path(),
            "minecraft:textures/atlas/map_decorations.png"
        );
        assert_eq!(
            surface.submission.texture.vanilla_sprite_id(),
            "minecraft:frame"
        );
        assert_eq!(surface.submission.tint, [1.0, 1.0, 1.0, 1.0]);
        assert_eq!(surface.submission.transform, expected_transform);
        assert_eq!(surface.submission.light, light);
        assert_eq!(
            (surface.submission.order, surface.submission.submit_sequence),
            (0, 3)
        );
        assert_eq!(surface.submission.decoration_index, 2);
        assert_eq!(
            surface.mesh.vertices[0].position,
            expected_transform
                .transform_point3(Vec3::new(-1.0, 1.0, -0.002))
                .to_array()
        );
        assert!(surface
            .mesh
            .vertices
            .iter()
            .all(|vertex| vertex.color == [1.0, 1.0, 1.0, 1.0] && vertex.light == light));

        assert!(
            bake_item_frame_map_decoration_surface(0, 0, 0, 0, 0, map_transform, light, 1)
                .is_none(),
            "player marker has showOnItemFrame=false in vanilla"
        );
    }

    #[test]
    fn item_frame_map_text_surface_uses_vanilla_order_one_text_submission() {
        let glyphs = test_map_text_glyphs();
        let map_transform = Mat4::from_translation(Vec3::new(1.0, 2.0, 3.0));
        let light = [7.0 / 15.0, 12.0 / 15.0];
        let surface = bake_item_frame_map_text_surface(
            1,
            "Frame",
            -20,
            30,
            0,
            map_transform,
            light,
            0,
            &glyphs,
        )
        .expect("frame-visible decoration name text");

        let width = 30.0;
        let scale = 6.0 / 9.0;
        let expected_transform = map_transform
            * Mat4::from_translation(Vec3::new(
                -20.0 / 2.0 + 64.0 - width * scale / 2.0,
                30.0 / 2.0 + 64.0 + 4.0,
                -0.025,
            ))
            * Mat4::from_scale(Vec3::new(scale, scale, -1.0))
            * Mat4::from_translation(Vec3::new(0.0, 0.0, 0.1));

        assert_eq!(surface.vertex_count(), 20);
        assert_eq!(surface.index_count(), 30);
        assert_eq!(surface.submission.type_id, 1);
        assert_eq!(surface.submission.text, "Frame");
        assert_eq!(surface.submission.render_type, ItemFrameMapRenderType::Text);
        assert_eq!(surface.submission.render_type.vanilla_name(), "text");
        assert_eq!(
            surface.submission.texture.vanilla_path(),
            "minecraft:textures/font/ascii.png"
        );
        assert_eq!(surface.submission.tint, [1.0, 1.0, 1.0, 1.0]);
        assert_eq!(surface.submission.transform, expected_transform);
        assert_eq!(surface.submission.light, light);
        assert_eq!(
            (surface.submission.order, surface.submission.submit_sequence),
            (1, 0)
        );
        assert_eq!(surface.submission.decoration_index, 0);
        assert_eq!(surface.submission.width, width);
        assert_eq!(surface.submission.scale, scale);
        assert_eq!(
            surface.mesh.vertices[0].position,
            expected_transform
                .transform_point3(Vec3::new(0.0, 0.0, 0.0))
                .to_array()
        );
        assert!(surface
            .mesh
            .vertices
            .iter()
            .all(|vertex| vertex.color == [1.0, 1.0, 1.0, 1.0] && vertex.light == light));

        assert!(
            bake_item_frame_map_text_surface(
                0,
                "Hidden",
                0,
                0,
                0,
                map_transform,
                light,
                0,
                &glyphs
            )
            .is_none(),
            "player marker has showOnItemFrame=false in vanilla"
        );
    }

    #[test]
    fn item_frame_map_text_width_uses_ascii_replacement_fallback() {
        let glyphs = test_map_text_glyphs();
        assert_eq!(item_frame_map_text_width("A A", &glyphs), Some(16));
        assert_eq!(item_frame_map_text_width("钻", &glyphs), Some(5));
        assert_eq!(item_frame_map_text_width("", &glyphs), None);
    }

    #[test]
    fn item_frame_map_decoration_atlas_remaps_sprite_uvs() {
        let (atlas, rgba) = build_item_frame_map_decoration_atlas(&[
            ItemFrameMapDecorationTexture {
                sprite_id: "minecraft:target_x".to_string(),
                width: 1,
                height: 1,
                rgba: vec![20, 21, 22, 255],
            },
            ItemFrameMapDecorationTexture {
                sprite_id: "minecraft:frame".to_string(),
                width: 2,
                height: 1,
                rgba: vec![10, 11, 12, 255, 13, 14, 15, 255],
            },
            ItemFrameMapDecorationTexture {
                sprite_id: "minecraft:bad".to_string(),
                width: 2,
                height: 2,
                rgba: vec![1, 2, 3],
            },
        ])
        .expect("valid map decoration atlas");

        assert_eq!(atlas.width, 2);
        assert_eq!(atlas.height, 2);
        assert_eq!(&rgba[0..8], &[10, 11, 12, 255, 13, 14, 15, 255]);
        assert_eq!(&rgba[8..16], &[20, 21, 22, 255, 0, 0, 0, 0]);
        assert!(!atlas.rects.contains_key("minecraft:bad"));

        let surface = bake_item_frame_map_decoration_surface(
            1,
            0,
            0,
            0,
            0,
            Mat4::IDENTITY,
            ITEM_MODEL_FULL_BRIGHT_LIGHT,
            1,
        )
        .expect("frame decoration surface");
        let (vertices, indices) = merge_item_frame_map_decoration_surfaces(&[surface], &atlas);
        assert_eq!(indices, vec![0, 1, 2, 0, 2, 3]);
        assert_eq!(vertices.len(), 4);
        assert_eq!(vertices[0].uv, [0.0, 0.0]);
        assert_eq!(vertices[2].uv, [1.0, 0.5]);
    }

    fn test_map_text_glyphs() -> [HudAsciiGlyph; HUD_ASCII_GLYPH_COUNT] {
        let mut glyphs = [HudAsciiGlyph {
            uv: crate::HudUvRect {
                min: [0.0, 0.0],
                max: [1.0, 1.0],
            },
            width: 6,
            height: 8,
            advance: 6,
        }; HUD_ASCII_GLYPH_COUNT];
        glyphs[(b' ' - HUD_ASCII_FIRST_GLYPH) as usize].advance = 4;
        glyphs[(b'?' - HUD_ASCII_FIRST_GLYPH) as usize].advance = 5;
        glyphs
    }
}
