//! Item-frame filled-map surfaces and vanilla `MapRenderer` decoration sprite submissions.

use std::{collections::BTreeMap, sync::Arc};

use glam::{Mat4, Vec3};

use crate::{HudAsciiGlyph, HudFontGlyphMap, Renderer};

use super::{ItemModelMesh, ItemModelVertex};

use bbb_render_types::vanilla_map_decoration_type;
pub use bbb_render_types::{
    FirstPersonMapBackgroundKind, FirstPersonMapBackgroundTexture, ItemFrameMapDecorationTexture,
    VanillaMapDecorationType as ItemFrameMapDecorationType,
};

const ITEM_FRAME_MAP_SIZE: u32 = 128;
const ITEM_FRAME_MAP_RGBA_LEN: usize =
    ITEM_FRAME_MAP_SIZE as usize * ITEM_FRAME_MAP_SIZE as usize * 4;
const MAX_RESIDENT_MAP_TEXTURES: usize = 4_096;
const ITEM_FRAME_MAP_DECORATION_ATLAS_PATH: &str = "minecraft:textures/atlas/map_decorations.png";
/// The uploaded map-text font texture is the baked multi-page
/// `minecraft:default` glyph atlas (vanilla `Style.DEFAULT_FONT`), not a
/// single provider page.
const ITEM_FRAME_MAP_TEXT_FONT_PATH: &str = "minecraft:font/default";
const ITEM_FRAME_MAP_TEXT_REPLACEMENT_GLYPH: char = '?';
const FIRST_PERSON_MAP_BACKGROUND_BORDER: f32 = 7.0;
const FIRST_PERSON_MAP_BACKGROUND_SIZE: f32 = 142.0;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ItemFrameMapTexturePatch {
    pub start_x: u8,
    pub start_y: u8,
    pub width: u8,
    pub height: u8,
    pub rgba: Arc<[u8]>,
}

/// Exact immutable source for vanilla's dynamic `minecraft:map/<id>` texture.
/// A repeated owner-generation/revision pair shares the full RGBA payload; a
/// consecutive canonical patch additionally carries only the changed rectangle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ItemFrameMapTexture {
    pub map_id: i32,
    pub owner_generation: u64,
    pub content_revision: u64,
    pub rgba: Arc<[u8]>,
    pub patch: Option<ItemFrameMapTexturePatch>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FirstPersonMapBackgroundTextureRef {
    pub kind: FirstPersonMapBackgroundKind,
}

impl FirstPersonMapBackgroundTextureRef {
    pub fn vanilla_path(self) -> &'static str {
        self.kind.vanilla_path()
    }
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

#[derive(Debug, Clone, PartialEq)]
pub struct FirstPersonMapBackgroundSubmission {
    pub kind: FirstPersonMapBackgroundKind,
    pub render_type: ItemFrameMapRenderType,
    pub texture: FirstPersonMapBackgroundTextureRef,
    pub tint: [f32; 4],
    pub transform: Mat4,
    pub light: [f32; 2],
    pub order: u32,
    pub submit_sequence: u32,
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

#[derive(Debug, Clone, PartialEq)]
pub struct FirstPersonMapBackgroundSurface {
    pub submission: FirstPersonMapBackgroundSubmission,
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

impl FirstPersonMapBackgroundSurface {
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
    texture: wgpu::Texture,
    _view: wgpu::TextureView,
    _sampler: wgpu::Sampler,
    pub(crate) bind_group: wgpu::BindGroup,
    pub(crate) layout: ItemFrameMapAtlasLayout,
    owners: BTreeMap<i32, ResidentMapTexture>,
    free_slots: Vec<u32>,
    side_slots: u32,
    frame_index: u64,
}

#[derive(Debug, Clone)]
struct ResidentMapTexture {
    map_id: i32,
    slot: u32,
    owner_generation: u64,
    content_revision: u64,
    rgba: Arc<[u8]>,
    last_used_frame: u64,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct FirstPersonMapBackgroundAtlasLayout {
    width: u32,
    height: u32,
    rects: BTreeMap<FirstPersonMapBackgroundKind, ItemFrameMapUvRect>,
}

pub(crate) struct FirstPersonMapBackgroundAtlasGpu {
    _texture: wgpu::Texture,
    _view: wgpu::TextureView,
    _sampler: wgpu::Sampler,
    pub(crate) bind_group: wgpu::BindGroup,
    pub(crate) layout: FirstPersonMapBackgroundAtlasLayout,
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

pub fn item_frame_map_decoration_type(type_id: i32) -> Option<ItemFrameMapDecorationType> {
    vanilla_map_decoration_type(type_id)
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

/// Bakes vanilla `ItemInHandRenderer.renderMap`'s first-person map-background submit:
/// `RenderTypes.text(textures/map/map_background*.png)`, white tint, and the four
/// `(-7,135,0)..(-7,-7,0)` vertices with UVs 0..1 under the same map render transform used by the
/// dynamic 128x128 `MapRenderer` surface.
pub fn bake_first_person_map_background_surface(
    kind: FirstPersonMapBackgroundKind,
    transform: Mat4,
    light: [f32; 2],
) -> FirstPersonMapBackgroundSurface {
    let min = -FIRST_PERSON_MAP_BACKGROUND_BORDER;
    let max = FIRST_PERSON_MAP_BACKGROUND_SIZE - FIRST_PERSON_MAP_BACKGROUND_BORDER;
    let mut mesh = ItemModelMesh::new();
    let corners = [
        transform
            .transform_point3(Vec3::new(min, max, 0.0))
            .to_array(),
        transform
            .transform_point3(Vec3::new(max, max, 0.0))
            .to_array(),
        transform
            .transform_point3(Vec3::new(max, min, 0.0))
            .to_array(),
        transform
            .transform_point3(Vec3::new(min, min, 0.0))
            .to_array(),
    ];
    mesh.append_raw_textured_quad(
        corners,
        [[0.0, 1.0], [1.0, 1.0], [1.0, 0.0], [0.0, 0.0]],
        [1.0, 1.0, 1.0, 1.0],
        light,
    );
    FirstPersonMapBackgroundSurface {
        submission: FirstPersonMapBackgroundSubmission {
            kind,
            render_type: ItemFrameMapRenderType::Text,
            texture: FirstPersonMapBackgroundTextureRef { kind },
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
    bake_map_decoration_surface(
        type_id,
        x,
        y,
        rot,
        decoration_index,
        map_transform,
        light,
        submit_sequence,
        true,
    )
}

pub fn bake_first_person_map_decoration_surface(
    type_id: i32,
    x: i8,
    y: i8,
    rot: u8,
    decoration_index: u32,
    map_transform: Mat4,
    light: [f32; 2],
    submit_sequence: u32,
) -> Option<ItemFrameMapDecorationSurface> {
    bake_map_decoration_surface(
        type_id,
        x,
        y,
        rot,
        decoration_index,
        map_transform,
        light,
        submit_sequence,
        false,
    )
}

fn bake_map_decoration_surface(
    type_id: i32,
    x: i8,
    y: i8,
    rot: u8,
    decoration_index: u32,
    map_transform: Mat4,
    light: [f32; 2],
    submit_sequence: u32,
    show_only_frame: bool,
) -> Option<ItemFrameMapDecorationSurface> {
    let decoration_type = item_frame_map_decoration_type(type_id)?;
    if show_only_frame && !decoration_type.render_on_frame {
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
/// `Mth.clamp(25.0F / width, 0.0F, 6.0F / 9.0F)` over the `minecraft:default` bitmap glyph pages.
pub fn bake_item_frame_map_text_surface(
    type_id: i32,
    text: impl Into<String>,
    x: i8,
    y: i8,
    decoration_index: u32,
    map_transform: Mat4,
    light: [f32; 2],
    submit_sequence: u32,
    glyphs: &HudFontGlyphMap,
) -> Option<ItemFrameMapTextSurface> {
    bake_map_text_surface(
        type_id,
        text,
        x,
        y,
        decoration_index,
        map_transform,
        light,
        submit_sequence,
        glyphs,
        true,
    )
}

pub fn bake_first_person_map_text_surface(
    type_id: i32,
    text: impl Into<String>,
    x: i8,
    y: i8,
    decoration_index: u32,
    map_transform: Mat4,
    light: [f32; 2],
    submit_sequence: u32,
    glyphs: &HudFontGlyphMap,
) -> Option<ItemFrameMapTextSurface> {
    bake_map_text_surface(
        type_id,
        text,
        x,
        y,
        decoration_index,
        map_transform,
        light,
        submit_sequence,
        glyphs,
        false,
    )
}

fn bake_map_text_surface(
    type_id: i32,
    text: impl Into<String>,
    x: i8,
    y: i8,
    decoration_index: u32,
    map_transform: Mat4,
    light: [f32; 2],
    submit_sequence: u32,
    glyphs: &HudFontGlyphMap,
    show_only_frame: bool,
) -> Option<ItemFrameMapTextSurface> {
    let decoration_type = item_frame_map_decoration_type(type_id)?;
    if show_only_frame && !decoration_type.render_on_frame {
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
            // Baseline alignment across font pages (vanilla
            // `GlyphBitmap.getTop()` = `7 - ascent`): ascent-7 pages keep
            // y0 = 0, the accented page rises 3px above the line top.
            let y0 = glyph.baseline_offset();
            let y1 = y0 + glyph.height as f32;
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

pub fn item_frame_map_text_width(text: &str, glyphs: &HudFontGlyphMap) -> Option<u32> {
    let mut width = 0u32;
    for ch in text.chars() {
        width = width.checked_add(item_frame_map_text_glyph(ch, glyphs).advance)?;
    }
    (width > 0).then_some(width)
}

/// Codepoint lookup over the baked first-provider-wins glyph map; codepoints
/// outside the bitmap pages (unihex deferred) degrade to `?`.
fn item_frame_map_text_glyph(ch: char, glyphs: &HudFontGlyphMap) -> HudAsciiGlyph {
    glyphs
        .get(ch)
        .or_else(|| glyphs.get(ITEM_FRAME_MAP_TEXT_REPLACEMENT_GLYPH))
        .unwrap_or_default()
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

pub(crate) fn merge_first_person_map_background_surfaces(
    surfaces: &[FirstPersonMapBackgroundSurface],
    atlas: &FirstPersonMapBackgroundAtlasLayout,
) -> (Vec<ItemModelVertex>, Vec<u32>) {
    let mut vertices = Vec::new();
    let mut indices = Vec::new();
    for surface in surfaces {
        let Some(rect) = atlas.rects.get(&surface.submission.kind).copied() else {
            continue;
        };
        let base =
            u32::try_from(vertices.len()).expect("first-person map background vertex count fits");
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
    pub fn set_first_person_map_background_surfaces(
        &mut self,
        textures: Vec<FirstPersonMapBackgroundTexture>,
        surfaces: Vec<FirstPersonMapBackgroundSurface>,
    ) {
        if self.first_person_map_background_atlas.is_none() {
            self.first_person_map_background_atlas =
                build_first_person_map_background_atlas(&textures).map(|(layout, rgba)| {
                    create_first_person_map_background_atlas_gpu(
                        &self.device,
                        &self.queue,
                        &self.terrain_bind_group_layout,
                        &self.camera_buffer,
                        layout,
                        &rgba,
                    )
                });
        }
        self.first_person_map_background_surfaces = if let Some(atlas) =
            &self.first_person_map_background_atlas
        {
            surfaces
                .into_iter()
                .filter(|surface| {
                    !surface.is_empty() && atlas.layout.rects.contains_key(&surface.submission.kind)
                })
                .collect()
        } else {
            Vec::new()
        };
    }

    /// Sets this frame's filled-map item-frame submissions and updates only
    /// changed owners in the persistent dynamic-map atlas.
    pub fn set_item_frame_map_surfaces(
        &mut self,
        textures: Vec<ItemFrameMapTexture>,
        surfaces: Vec<ItemFrameMapSurface>,
    ) {
        self.set_map_surfaces(textures, Vec::new(), surfaces, Vec::new());
    }

    /// Sets all decoded-map base surfaces. World-space item-frame maps and
    /// depth-cleared first-person maps share stable map-id slots; repeated
    /// revisions do not upload pixels and consecutive patches write only their
    /// changed rectangle.
    pub fn set_map_surfaces(
        &mut self,
        item_frame_textures: Vec<ItemFrameMapTexture>,
        first_person_textures: Vec<ItemFrameMapTexture>,
        item_frame_surfaces: Vec<ItemFrameMapSurface>,
        first_person_surfaces: Vec<ItemFrameMapSurface>,
    ) {
        self.counters.item_map_upload_bytes = 0;
        let mut textures = BTreeMap::new();
        for texture in item_frame_textures.into_iter().chain(first_person_textures) {
            if texture.rgba.len() == ITEM_FRAME_MAP_RGBA_LEN {
                textures.insert(texture.map_id, texture);
            }
        }
        self.prepare_map_atlas_capacity(&textures);
        for texture in textures.into_values() {
            self.update_resident_map_texture(texture);
        }
        self.refresh_map_texture_counters();
        self.item_frame_map_surfaces = if let Some(atlas) = &self.item_frame_map_atlas {
            item_frame_surfaces
                .into_iter()
                .filter(|surface| {
                    !surface.is_empty()
                        && atlas.layout.rects.contains_key(&surface.submission.map_id)
                })
                .collect()
        } else {
            Vec::new()
        };
        self.first_person_map_surfaces = if let Some(atlas) = &self.item_frame_map_atlas {
            first_person_surfaces
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

    fn prepare_map_atlas_capacity(&mut self, textures: &BTreeMap<i32, ItemFrameMapTexture>) {
        if let Some(atlas) = &mut self.item_frame_map_atlas {
            atlas.frame_index = atlas
                .frame_index
                .checked_add(1)
                .expect("map atlas frame overflow");
        }
        let requested = textures
            .keys()
            .copied()
            .collect::<std::collections::BTreeSet<_>>();
        if let Some(atlas) = &mut self.item_frame_map_atlas {
            let mut missing = requested
                .iter()
                .filter(|map_id| !atlas.owners.contains_key(map_id))
                .count();
            while atlas.owners.len().saturating_add(missing) > MAX_RESIDENT_MAP_TEXTURES {
                let candidate = atlas
                    .owners
                    .iter()
                    .filter(|(map_id, _)| !requested.contains(map_id))
                    .min_by_key(|(map_id, owner)| (owner.last_used_frame, **map_id))
                    .map(|(map_id, _)| *map_id);
                let Some(candidate) = candidate else {
                    break;
                };
                remove_resident_map_texture(atlas, candidate);
                missing = requested
                    .iter()
                    .filter(|map_id| !atlas.owners.contains_key(map_id))
                    .count();
            }
        }

        let owner_count = self
            .item_frame_map_atlas
            .as_ref()
            .map_or(0, |atlas| atlas.owners.len());
        let missing = requested
            .iter()
            .filter(|map_id| {
                self.item_frame_map_atlas
                    .as_ref()
                    .is_none_or(|atlas| !atlas.owners.contains_key(map_id))
            })
            .count();
        let required = owner_count
            .saturating_add(missing)
            .min(MAX_RESIDENT_MAP_TEXTURES);
        if required == 0 {
            return;
        }
        let required_side = map_atlas_side_for_capacity(required);
        let current_side = self
            .item_frame_map_atlas
            .as_ref()
            .map_or(0, |atlas| atlas.side_slots);
        if current_side >= required_side {
            return;
        }

        let previous = self.item_frame_map_atlas.take();
        let mut atlas = create_item_frame_map_atlas_gpu(
            &self.device,
            &self.terrain_bind_group_layout,
            &self.camera_buffer,
            required_side,
        );
        if let Some(previous) = previous {
            atlas.frame_index = previous.frame_index;
            let owners = previous.owners.into_values().collect::<Vec<_>>();
            for owner in owners {
                write_full_map_texture(&self.queue, &atlas.texture, atlas.side_slots, &owner);
                self.counters.item_map_upload_bytes = self
                    .counters
                    .item_map_upload_bytes
                    .saturating_add(owner.rgba.len() as u64);
                insert_resident_map_texture(&mut atlas, owner);
            }
            self.counters.item_full_residency_replacements = self
                .counters
                .item_full_residency_replacements
                .saturating_add(1);
            self.counters.map_full_residency_replacements = self
                .counters
                .map_full_residency_replacements
                .saturating_add(1);
        }
        self.item_frame_map_atlas = Some(atlas);
    }

    fn update_resident_map_texture(&mut self, texture: ItemFrameMapTexture) {
        let Some(atlas) = &mut self.item_frame_map_atlas else {
            return;
        };
        if let Some(owner) = atlas.owners.get_mut(&texture.map_id) {
            owner.last_used_frame = atlas.frame_index;
            if owner.owner_generation == texture.owner_generation
                && owner.content_revision == texture.content_revision
            {
                self.counters.map_textures_reused =
                    self.counters.map_textures_reused.saturating_add(1);
                return;
            }
            if owner.owner_generation > texture.owner_generation
                || (owner.owner_generation == texture.owner_generation
                    && owner.content_revision > texture.content_revision)
            {
                self.counters.map_textures_stale =
                    self.counters.map_textures_stale.saturating_add(1);
                return;
            }
            let patched = owner.owner_generation == texture.owner_generation
                && owner.content_revision.checked_add(1) == Some(texture.content_revision)
                && texture.patch.as_ref().is_some_and(|patch| {
                    write_map_texture_patch(
                        &self.queue,
                        &atlas.texture,
                        atlas.side_slots,
                        owner.slot,
                        patch,
                    )
                });
            let uploaded_bytes = if patched {
                texture
                    .patch
                    .as_ref()
                    .map_or(0, |patch| patch.rgba.len() as u64)
            } else {
                let replacement = ResidentMapTexture {
                    map_id: texture.map_id,
                    slot: owner.slot,
                    owner_generation: texture.owner_generation,
                    content_revision: texture.content_revision,
                    rgba: Arc::clone(&texture.rgba),
                    last_used_frame: atlas.frame_index,
                };
                write_full_map_texture(&self.queue, &atlas.texture, atlas.side_slots, &replacement);
                texture.rgba.len() as u64
            };
            owner.owner_generation = texture.owner_generation;
            owner.content_revision = texture.content_revision;
            owner.rgba = texture.rgba;
            self.counters.item_map_upload_bytes = self
                .counters
                .item_map_upload_bytes
                .saturating_add(uploaded_bytes);
            self.counters.map_textures_updated =
                self.counters.map_textures_updated.saturating_add(1);
            return;
        }

        let Some(slot) = atlas.free_slots.pop() else {
            return;
        };
        let owner = ResidentMapTexture {
            map_id: texture.map_id,
            slot,
            owner_generation: texture.owner_generation,
            content_revision: texture.content_revision,
            rgba: texture.rgba,
            last_used_frame: atlas.frame_index,
        };
        write_full_map_texture(&self.queue, &atlas.texture, atlas.side_slots, &owner);
        self.counters.item_map_upload_bytes = self
            .counters
            .item_map_upload_bytes
            .saturating_add(owner.rgba.len() as u64);
        self.counters.map_textures_created = self.counters.map_textures_created.saturating_add(1);
        insert_resident_map_texture(atlas, owner);
    }

    pub fn remove_map_texture(&mut self, map_id: i32) {
        if let Some(atlas) = &mut self.item_frame_map_atlas {
            remove_resident_map_texture(atlas, map_id);
        }
        self.refresh_map_texture_counters();
    }

    pub fn invalidate_map_texture_residency(&mut self) {
        if self.item_frame_map_atlas.is_some() {
            self.counters.map_full_residency_replacements = self
                .counters
                .map_full_residency_replacements
                .saturating_add(1);
        }
        self.item_frame_map_atlas = None;
        self.first_person_map_background_atlas = None;
        self.item_frame_map_decoration_atlas = None;
        self.refresh_map_texture_counters();
    }

    fn refresh_map_texture_counters(&mut self) {
        let bytes = self.item_frame_map_atlas.as_ref().map_or(0, |atlas| {
            (atlas.owners.len() as u64).saturating_mul(ITEM_FRAME_MAP_RGBA_LEN as u64)
        });
        self.counters.map_texture_resident_bytes = bytes;
        self.counters.map_texture_peak_resident_bytes =
            self.counters.map_texture_peak_resident_bytes.max(bytes);
    }

    /// Sets this frame's `MapRenderer` decoration sprite submissions for filled-map item frames. The
    /// atlas is the vanilla `textures/atlas/map_decorations.png` sheet, rebuilt here from the decoded
    /// pack sprites supplied by native.
    pub fn set_item_frame_map_decoration_surfaces(
        &mut self,
        textures: Vec<ItemFrameMapDecorationTexture>,
        surfaces: Vec<ItemFrameMapDecorationSurface>,
    ) {
        self.set_map_decoration_surfaces(textures, Vec::new(), surfaces, Vec::new());
    }

    /// Sets all `MapRenderer` decoration sprite submissions. Item-frame maps use
    /// `showOnlyFrame=true`; first-person maps use `showOnlyFrame=false`, but both sample the same
    /// vanilla map-decoration atlas.
    pub fn set_map_decoration_surfaces(
        &mut self,
        item_frame_textures: Vec<ItemFrameMapDecorationTexture>,
        first_person_textures: Vec<ItemFrameMapDecorationTexture>,
        item_frame_surfaces: Vec<ItemFrameMapDecorationSurface>,
        first_person_surfaces: Vec<ItemFrameMapDecorationSurface>,
    ) {
        if self.item_frame_map_decoration_atlas.is_none() {
            self.item_frame_map_decoration_atlas = build_item_frame_map_decoration_atlas(
                item_frame_textures
                    .iter()
                    .chain(first_person_textures.iter()),
            )
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
        }
        self.item_frame_map_decoration_surfaces =
            if let Some(atlas) = &self.item_frame_map_decoration_atlas {
                item_frame_surfaces
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
        self.first_person_map_decoration_surfaces =
            if let Some(atlas) = &self.item_frame_map_decoration_atlas {
                first_person_surfaces
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
        self.set_map_text_surfaces(surfaces, Vec::new());
    }

    pub fn set_map_text_surfaces(
        &mut self,
        item_frame_surfaces: Vec<ItemFrameMapTextSurface>,
        first_person_surfaces: Vec<ItemFrameMapTextSurface>,
    ) {
        self.item_frame_map_text_surfaces = if self.item_frame_map_text_font_atlas.is_some() {
            item_frame_surfaces
                .into_iter()
                .filter(|surface| !surface.is_empty())
                .collect()
        } else {
            Vec::new()
        };
        self.first_person_map_text_surfaces = if self.item_frame_map_text_font_atlas.is_some() {
            first_person_surfaces
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

    pub(crate) fn collect_first_person_map_background_geometry(
        &self,
    ) -> (Vec<ItemModelVertex>, Vec<u32>) {
        let Some(atlas) = &self.first_person_map_background_atlas else {
            return (Vec::new(), Vec::new());
        };
        merge_first_person_map_background_surfaces(
            &self.first_person_map_background_surfaces,
            &atlas.layout,
        )
    }

    pub(crate) fn collect_first_person_map_geometry(&self) -> (Vec<ItemModelVertex>, Vec<u32>) {
        let Some(atlas) = &self.item_frame_map_atlas else {
            return (Vec::new(), Vec::new());
        };
        merge_item_frame_map_surfaces(&self.first_person_map_surfaces, &atlas.layout)
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

    pub(crate) fn collect_first_person_map_decoration_geometry(
        &self,
    ) -> (Vec<ItemModelVertex>, Vec<u32>) {
        let Some(atlas) = &self.item_frame_map_decoration_atlas else {
            return (Vec::new(), Vec::new());
        };
        merge_item_frame_map_decoration_surfaces(
            &self.first_person_map_decoration_surfaces,
            &atlas.layout,
        )
    }

    pub(crate) fn collect_item_frame_map_text_geometry(&self) -> (Vec<ItemModelVertex>, Vec<u32>) {
        if self.item_frame_map_text_font_atlas.is_none() {
            return (Vec::new(), Vec::new());
        }
        merge_item_frame_map_text_surfaces(&self.item_frame_map_text_surfaces)
    }

    pub(crate) fn collect_first_person_map_text_geometry(
        &self,
    ) -> (Vec<ItemModelVertex>, Vec<u32>) {
        if self.item_frame_map_text_font_atlas.is_none() {
            return (Vec::new(), Vec::new());
        }
        merge_item_frame_map_text_surfaces(&self.first_person_map_text_surfaces)
    }
}

fn build_first_person_map_background_atlas(
    textures: &[FirstPersonMapBackgroundTexture],
) -> Option<(FirstPersonMapBackgroundAtlasLayout, Vec<u8>)> {
    let mut by_kind: BTreeMap<FirstPersonMapBackgroundKind, &FirstPersonMapBackgroundTexture> =
        BTreeMap::new();
    for texture in textures {
        let expected_len = texture
            .width
            .checked_mul(texture.height)
            .and_then(|pixels| pixels.checked_mul(4))
            .and_then(|len| usize::try_from(len).ok());
        if texture.width > 0 && texture.height > 0 && expected_len == Some(texture.rgba.len()) {
            by_kind.insert(texture.kind, texture);
        }
    }
    if by_kind.is_empty() {
        return None;
    }
    let width = by_kind.values().map(|texture| texture.width).max()?;
    let height = by_kind
        .values()
        .try_fold(0u32, |height, texture| height.checked_add(texture.height))?;
    let atlas_len = width
        .checked_mul(height)
        .and_then(|pixels| pixels.checked_mul(4))
        .and_then(|len| usize::try_from(len).ok())?;
    let mut atlas_rgba = vec![0; atlas_len];
    let mut rects = BTreeMap::new();
    let mut y_offset = 0u32;
    for (kind, texture) in by_kind {
        for y in 0..texture.height {
            let src = usize::try_from((y * texture.width) * 4).ok()?;
            let dst = usize::try_from(((y_offset + y) * width) * 4).ok()?;
            let row_len = usize::try_from(texture.width * 4).ok()?;
            atlas_rgba[dst..dst + row_len].copy_from_slice(&texture.rgba[src..src + row_len]);
        }
        rects.insert(
            kind,
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
        FirstPersonMapBackgroundAtlasLayout {
            width,
            height,
            rects,
        },
        atlas_rgba,
    ))
}

fn create_first_person_map_background_atlas_gpu(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    bind_group_layout: &wgpu::BindGroupLayout,
    camera_buffer: &wgpu::Buffer,
    layout: FirstPersonMapBackgroundAtlasLayout,
    rgba: &[u8],
) -> FirstPersonMapBackgroundAtlasGpu {
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("bbb-first-person-map-background-atlas-texture"),
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
        label: Some("bbb-first-person-map-background-atlas-sampler"),
        address_mode_u: wgpu::AddressMode::ClampToEdge,
        address_mode_v: wgpu::AddressMode::ClampToEdge,
        address_mode_w: wgpu::AddressMode::ClampToEdge,
        mag_filter: wgpu::FilterMode::Nearest,
        min_filter: wgpu::FilterMode::Nearest,
        mipmap_filter: wgpu::FilterMode::Nearest,
        ..Default::default()
    });
    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("bbb-first-person-map-background-atlas-bind-group"),
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
    FirstPersonMapBackgroundAtlasGpu {
        _texture: texture,
        _view: view,
        _sampler: sampler,
        bind_group,
        layout,
    }
}

#[cfg(test)]
fn build_item_frame_map_atlas<'a>(
    textures: impl IntoIterator<Item = &'a ItemFrameMapTexture>,
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

fn map_atlas_side_for_capacity(capacity: usize) -> u32 {
    let capacity = u32::try_from(capacity.max(1)).expect("map atlas capacity fits u32");
    let mut side = 1u32;
    while side.saturating_mul(side) < capacity {
        side = side.checked_mul(2).expect("map atlas side overflow");
    }
    side
}

fn map_texture_slot_origin(slot: u32, side_slots: u32) -> wgpu::Origin3d {
    wgpu::Origin3d {
        x: (slot % side_slots) * ITEM_FRAME_MAP_SIZE,
        y: (slot / side_slots) * ITEM_FRAME_MAP_SIZE,
        z: 0,
    }
}

fn map_texture_slot_rect(slot: u32, side_slots: u32) -> ItemFrameMapUvRect {
    let atlas_size = (side_slots * ITEM_FRAME_MAP_SIZE) as f32;
    let origin = map_texture_slot_origin(slot, side_slots);
    ItemFrameMapUvRect {
        min: [
            (origin.x as f32 + 0.5) / atlas_size,
            (origin.y as f32 + 0.5) / atlas_size,
        ],
        max: [
            (origin.x as f32 + ITEM_FRAME_MAP_SIZE as f32 - 0.5) / atlas_size,
            (origin.y as f32 + ITEM_FRAME_MAP_SIZE as f32 - 0.5) / atlas_size,
        ],
    }
}

fn create_item_frame_map_atlas_gpu(
    device: &wgpu::Device,
    bind_group_layout: &wgpu::BindGroupLayout,
    camera_buffer: &wgpu::Buffer,
    side_slots: u32,
) -> ItemFrameMapAtlasGpu {
    let atlas_size = side_slots
        .checked_mul(ITEM_FRAME_MAP_SIZE)
        .expect("map atlas extent overflow");
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("bbb-item-frame-map-atlas-texture"),
        size: wgpu::Extent3d {
            width: atlas_size,
            height: atlas_size,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8UnormSrgb,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });
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
        texture,
        _view: view,
        _sampler: sampler,
        bind_group,
        layout: ItemFrameMapAtlasLayout {
            width: atlas_size,
            height: atlas_size,
            rects: BTreeMap::new(),
        },
        owners: BTreeMap::new(),
        free_slots: (0..side_slots * side_slots).rev().collect(),
        side_slots,
        frame_index: 1,
    }
}

fn write_full_map_texture(
    queue: &wgpu::Queue,
    texture: &wgpu::Texture,
    side_slots: u32,
    owner: &ResidentMapTexture,
) {
    queue.write_texture(
        wgpu::ImageCopyTexture {
            texture,
            mip_level: 0,
            origin: map_texture_slot_origin(owner.slot, side_slots),
            aspect: wgpu::TextureAspect::All,
        },
        &owner.rgba,
        wgpu::ImageDataLayout {
            offset: 0,
            bytes_per_row: Some(ITEM_FRAME_MAP_SIZE * 4),
            rows_per_image: Some(ITEM_FRAME_MAP_SIZE),
        },
        wgpu::Extent3d {
            width: ITEM_FRAME_MAP_SIZE,
            height: ITEM_FRAME_MAP_SIZE,
            depth_or_array_layers: 1,
        },
    );
}

fn write_map_texture_patch(
    queue: &wgpu::Queue,
    texture: &wgpu::Texture,
    side_slots: u32,
    slot: u32,
    patch: &ItemFrameMapTexturePatch,
) -> bool {
    let width = u32::from(patch.width);
    let height = u32::from(patch.height);
    let expected = width
        .checked_mul(height)
        .and_then(|pixels| pixels.checked_mul(4))
        .and_then(|bytes| usize::try_from(bytes).ok());
    if width == 0 || height == 0 || expected != Some(patch.rgba.len()) {
        return false;
    }
    let mut origin = map_texture_slot_origin(slot, side_slots);
    origin.x += u32::from(patch.start_x);
    origin.y += u32::from(patch.start_y);
    if u32::from(patch.start_x) + width > ITEM_FRAME_MAP_SIZE
        || u32::from(patch.start_y) + height > ITEM_FRAME_MAP_SIZE
    {
        return false;
    }
    queue.write_texture(
        wgpu::ImageCopyTexture {
            texture,
            mip_level: 0,
            origin,
            aspect: wgpu::TextureAspect::All,
        },
        &patch.rgba,
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
    true
}

fn insert_resident_map_texture(atlas: &mut ItemFrameMapAtlasGpu, owner: ResidentMapTexture) {
    atlas.free_slots.retain(|slot| *slot != owner.slot);
    let map_id = owner.map_id;
    atlas
        .layout
        .rects
        .insert(map_id, map_texture_slot_rect(owner.slot, atlas.side_slots));
    atlas.owners.insert(map_id, owner);
}

fn remove_resident_map_texture(atlas: &mut ItemFrameMapAtlasGpu, map_id: i32) {
    if let Some(owner) = atlas.owners.remove(&map_id) {
        atlas.free_slots.push(owner.slot);
        atlas.layout.rects.remove(&map_id);
    }
}

fn build_item_frame_map_decoration_atlas<'a>(
    textures: impl IntoIterator<Item = &'a ItemFrameMapDecorationTexture>,
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

    fn map_texture(map_id: i32, rgba: Vec<u8>) -> ItemFrameMapTexture {
        map_texture_at_revision(map_id, 1, 0, rgba, None)
    }

    fn map_texture_at_revision(
        map_id: i32,
        owner_generation: u64,
        content_revision: u64,
        rgba: Vec<u8>,
        patch: Option<ItemFrameMapTexturePatch>,
    ) -> ItemFrameMapTexture {
        ItemFrameMapTexture {
            map_id,
            owner_generation,
            content_revision,
            rgba: rgba.into(),
            patch,
        }
    }

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
            map_texture(10, second_map_rgba),
            map_texture(5, first_map_rgba),
            map_texture(99, vec![1, 2, 3]),
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
        let (atlas, _) =
            build_item_frame_map_atlas(&[map_texture(5, vec![20; ITEM_FRAME_MAP_RGBA_LEN])])
                .expect("valid map atlas");

        let (vertices, indices) = merge_item_frame_map_surfaces(&[surface], &atlas);

        assert!(
            vertices.is_empty() && indices.is_empty(),
            "absent dynamic map texture must not fold stale map geometry"
        );
    }

    #[test]
    fn item_frame_map_surface_waits_when_dynamic_map_texture_is_malformed() {
        let surface = bake_item_frame_map_surface(10, Mat4::IDENTITY, ITEM_MODEL_FULL_BRIGHT_LIGHT);
        let (atlas, _) = build_item_frame_map_atlas(&[
            map_texture(10, vec![1, 2, 3]),
            map_texture(5, vec![20; ITEM_FRAME_MAP_RGBA_LEN]),
        ])
        .expect("valid atlas from the unrelated map");

        let (vertices, indices) = merge_item_frame_map_surfaces(&[surface], &atlas);

        assert!(
            vertices.is_empty() && indices.is_empty(),
            "malformed dynamic map texture must not fold stale map geometry"
        );
    }

    #[test]
    fn persistent_map_owner_reuses_unchanged_pixels_and_uploads_only_one_patch() {
        let Some(mut renderer) = Renderer::new_offscreen(32, 32) else {
            return;
        };
        let surface = bake_item_frame_map_surface(10, Mat4::IDENTITY, [1.0; 2]);
        let second_surface = bake_item_frame_map_surface(11, Mat4::IDENTITY, [1.0; 2]);
        let first_rgba = vec![0; ITEM_FRAME_MAP_RGBA_LEN];
        let second_rgba = vec![9; ITEM_FRAME_MAP_RGBA_LEN];
        renderer.set_map_surfaces(
            vec![
                map_texture_at_revision(10, 7, 0, first_rgba.clone(), None),
                map_texture_at_revision(11, 8, 0, second_rgba.clone(), None),
            ],
            Vec::new(),
            vec![surface.clone(), second_surface.clone()],
            Vec::new(),
        );
        assert_eq!(renderer.counters().item_map_upload_bytes, 131_072);
        assert_eq!(renderer.counters().map_textures_created, 2);
        assert_eq!(renderer.counters().map_texture_resident_bytes, 131_072);
        let slot = renderer
            .item_frame_map_atlas
            .as_ref()
            .unwrap()
            .owners
            .get(&10)
            .unwrap()
            .slot;
        let second_owner = renderer
            .item_frame_map_atlas
            .as_ref()
            .unwrap()
            .owners
            .get(&11)
            .unwrap();
        let second_slot = second_owner.slot;
        let second_owner_rgba = Arc::clone(&second_owner.rgba);

        renderer.set_map_surfaces(
            vec![
                map_texture_at_revision(10, 7, 0, first_rgba, None),
                map_texture_at_revision(11, 8, 0, second_rgba.clone(), None),
            ],
            Vec::new(),
            vec![surface.clone(), second_surface.clone()],
            Vec::new(),
        );
        assert_eq!(renderer.counters().item_map_upload_bytes, 0);
        assert_eq!(renderer.counters().map_textures_reused, 2);

        let pixel = [11, 22, 33, 255];
        let mut updated_rgba = vec![0; ITEM_FRAME_MAP_RGBA_LEN];
        let offset = (2 + 3 * ITEM_FRAME_MAP_SIZE as usize) * 4;
        updated_rgba[offset..offset + 4].copy_from_slice(&pixel);
        let replacements = renderer.counters().item_full_residency_replacements;
        renderer.set_map_surfaces(
            vec![
                map_texture_at_revision(
                    10,
                    7,
                    1,
                    updated_rgba,
                    Some(ItemFrameMapTexturePatch {
                        start_x: 2,
                        start_y: 3,
                        width: 1,
                        height: 1,
                        rgba: Arc::from(pixel),
                    }),
                ),
                map_texture_at_revision(11, 8, 0, second_rgba, None),
            ],
            Vec::new(),
            vec![surface, second_surface],
            Vec::new(),
        );
        assert_eq!(renderer.counters().item_map_upload_bytes, 4);
        assert_eq!(renderer.counters().map_textures_updated, 1);
        assert_eq!(
            renderer.counters().item_full_residency_replacements,
            replacements
        );
        let owner = renderer
            .item_frame_map_atlas
            .as_ref()
            .unwrap()
            .owners
            .get(&10)
            .unwrap();
        assert_eq!(owner.slot, slot);
        assert_eq!(owner.content_revision, 1);
        let second_owner = renderer
            .item_frame_map_atlas
            .as_ref()
            .unwrap()
            .owners
            .get(&11)
            .unwrap();
        assert_eq!(second_owner.slot, second_slot);
        assert_eq!(second_owner.content_revision, 0);
        assert!(Arc::ptr_eq(&second_owner.rgba, &second_owner_rgba));
    }

    #[test]
    fn map_owner_remove_and_resource_invalidation_drop_old_handles() {
        let Some(mut renderer) = Renderer::new_offscreen(16, 16) else {
            return;
        };
        renderer.set_map_surfaces(
            vec![map_texture_at_revision(
                3,
                9,
                2,
                vec![1; ITEM_FRAME_MAP_RGBA_LEN],
                None,
            )],
            Vec::new(),
            vec![bake_item_frame_map_surface(3, Mat4::IDENTITY, [1.0; 2])],
            Vec::new(),
        );
        renderer.set_map_surfaces(
            vec![map_texture_at_revision(
                3,
                9,
                1,
                vec![2; ITEM_FRAME_MAP_RGBA_LEN],
                None,
            )],
            Vec::new(),
            vec![bake_item_frame_map_surface(3, Mat4::IDENTITY, [1.0; 2])],
            Vec::new(),
        );
        assert_eq!(renderer.counters().item_map_upload_bytes, 0);
        assert_eq!(renderer.counters().map_textures_stale, 1);
        assert_eq!(
            renderer
                .item_frame_map_atlas
                .as_ref()
                .unwrap()
                .owners
                .get(&3)
                .unwrap()
                .content_revision,
            2
        );
        renderer.remove_map_texture(3);
        let atlas = renderer.item_frame_map_atlas.as_ref().unwrap();
        assert!(!atlas.owners.contains_key(&3));
        assert!(!atlas.layout.rects.contains_key(&3));

        renderer.invalidate_map_texture_residency();
        assert!(renderer.item_frame_map_atlas.is_none());
        assert!(renderer.first_person_map_background_atlas.is_none());
        assert!(renderer.item_frame_map_decoration_atlas.is_none());
    }

    #[test]
    fn first_person_map_background_surface_uses_vanilla_render_map_geometry() {
        let light = ITEM_MODEL_FULL_BRIGHT_LIGHT;
        let surface = bake_first_person_map_background_surface(
            FirstPersonMapBackgroundKind::Checkerboard,
            Mat4::IDENTITY,
            light,
        );

        assert_eq!(surface.vertex_count(), 4);
        assert_eq!(surface.index_count(), 6);
        assert_eq!(
            surface.submission.kind,
            FirstPersonMapBackgroundKind::Checkerboard
        );
        assert_eq!(surface.submission.render_type, ItemFrameMapRenderType::Text);
        assert_eq!(
            surface.submission.texture.vanilla_path(),
            "minecraft:textures/map/map_background_checkerboard.png"
        );
        assert_eq!(surface.submission.tint, [1.0, 1.0, 1.0, 1.0]);
        assert_eq!(surface.submission.light, light);
        assert_eq!(
            surface
                .mesh
                .vertices
                .iter()
                .map(|vertex| vertex.position)
                .collect::<Vec<_>>(),
            vec![
                [-7.0, 135.0, 0.0],
                [135.0, 135.0, 0.0],
                [135.0, -7.0, 0.0],
                [-7.0, -7.0, 0.0],
            ]
        );
    }

    #[test]
    fn first_person_map_background_atlas_filters_and_remaps_uvs() {
        let (atlas, rgba) = build_first_person_map_background_atlas(&[
            FirstPersonMapBackgroundTexture {
                kind: FirstPersonMapBackgroundKind::Checkerboard,
                width: 1,
                height: 1,
                rgba: vec![40, 41, 42, 255],
            },
            FirstPersonMapBackgroundTexture {
                kind: FirstPersonMapBackgroundKind::Plain,
                width: 2,
                height: 1,
                rgba: vec![10, 11, 12, 255, 13, 14, 15, 255],
            },
            FirstPersonMapBackgroundTexture {
                kind: FirstPersonMapBackgroundKind::Plain,
                width: 1,
                height: 1,
                rgba: vec![1, 2, 3],
            },
        ])
        .expect("valid map background atlas");

        assert_eq!(atlas.width, 2);
        assert_eq!(atlas.height, 2);
        assert_eq!(
            rgba,
            vec![10, 11, 12, 255, 13, 14, 15, 255, 40, 41, 42, 255, 0, 0, 0, 0,]
        );
        assert_eq!(atlas.rects.len(), 2);
        let surface = bake_first_person_map_background_surface(
            FirstPersonMapBackgroundKind::Checkerboard,
            Mat4::IDENTITY,
            ITEM_MODEL_FULL_BRIGHT_LIGHT,
        );
        let (vertices, indices) = merge_first_person_map_background_surfaces(&[surface], &atlas);
        assert_eq!(vertices.len(), 4);
        assert_eq!(indices.len(), 6);
        assert_eq!(vertices[0].uv, [0.0, 1.0]);
        assert_eq!(vertices[2].uv, [0.5, 0.5]);
    }

    #[test]
    fn resource_map_atlases_persist_when_later_frames_carry_only_instances() {
        let Some(mut renderer) = Renderer::new_offscreen(16, 16) else {
            return;
        };
        let background = bake_first_person_map_background_surface(
            FirstPersonMapBackgroundKind::Checkerboard,
            Mat4::IDENTITY,
            [1.0; 2],
        );
        renderer.set_first_person_map_background_surfaces(
            vec![FirstPersonMapBackgroundTexture {
                kind: FirstPersonMapBackgroundKind::Checkerboard,
                width: 1,
                height: 1,
                rgba: vec![1, 2, 3, 255],
            }],
            vec![background.clone()],
        );
        renderer.set_first_person_map_background_surfaces(Vec::new(), vec![background]);
        assert!(renderer.first_person_map_background_atlas.is_some());
        assert_eq!(renderer.first_person_map_background_surfaces.len(), 1);

        let decoration =
            bake_first_person_map_decoration_surface(1, 0, 0, 0, 0, Mat4::IDENTITY, [1.0; 2], 0)
                .unwrap();
        let sprite_id = decoration.submission.texture.sprite_id.to_string();
        renderer.set_map_decoration_surfaces(
            Vec::new(),
            vec![ItemFrameMapDecorationTexture {
                sprite_id,
                width: 1,
                height: 1,
                rgba: vec![1, 2, 3, 255],
            }],
            Vec::new(),
            vec![decoration.clone()],
        );
        renderer.set_map_decoration_surfaces(Vec::new(), Vec::new(), Vec::new(), vec![decoration]);
        assert!(renderer.item_frame_map_decoration_atlas.is_some());
        assert_eq!(renderer.first_person_map_decoration_surfaces.len(), 1);
    }

    #[test]
    fn generated_map_decoration_catalog_drives_all_surface_metadata() {
        let map_transform = Mat4::IDENTITY;
        let light = ITEM_MODEL_FULL_BRIGHT_LIGHT;
        for entry in bbb_render_types::vanilla_map_decoration_types() {
            assert_eq!(item_frame_map_decoration_type(entry.type_id), Some(entry));

            let first_person = bake_first_person_map_decoration_surface(
                entry.type_id,
                0,
                0,
                0,
                0,
                map_transform,
                light,
                0,
            )
            .expect("first-person maps show every registered decoration");
            assert_eq!(
                first_person.submission.texture.vanilla_sprite_id(),
                entry.sprite_id
            );

            let item_frame = bake_item_frame_map_decoration_surface(
                entry.type_id,
                0,
                0,
                0,
                0,
                map_transform,
                light,
                0,
            );
            assert_eq!(item_frame.is_some(), entry.render_on_frame);
            if let Some(surface) = item_frame {
                assert_eq!(
                    surface.submission.texture.vanilla_sprite_id(),
                    entry.sprite_id
                );
            }
        }
        assert!(item_frame_map_decoration_type(-1).is_none());
        let unknown_type_id = bbb_render_types::vanilla_map_decoration_types().len() as i32;
        assert!(item_frame_map_decoration_type(unknown_type_id).is_none());
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
        let first_person =
            bake_first_person_map_decoration_surface(0, 0, 0, 0, 0, map_transform, light, 1)
                .expect("first-person maps render all MapRenderer decorations");
        assert_eq!(first_person.submission.type_id, 0);
        assert_eq!(
            first_person.submission.texture.vanilla_sprite_id(),
            "minecraft:player"
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
            "minecraft:font/default"
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
        let first_person = bake_first_person_map_text_surface(
            0,
            "Player",
            0,
            0,
            0,
            map_transform,
            light,
            0,
            &glyphs,
        )
        .expect("first-person maps render text for non-frame decorations");
        assert_eq!(first_person.submission.type_id, 0);
        assert_eq!(first_person.submission.text, "Player");
    }

    #[test]
    fn item_frame_map_text_width_uses_glyph_map_replacement_fallback() {
        let mut glyphs = test_map_text_glyphs();
        glyphs.insert_first_wins(
            'é',
            HudAsciiGlyph {
                width: 6,
                height: 12,
                advance: 7,
                ascent: 10,
                ..HudAsciiGlyph::default()
            },
        );
        assert_eq!(item_frame_map_text_width("A A", &glyphs), Some(16));
        // Mapped accented codepoints resolve their own glyph now.
        assert_eq!(item_frame_map_text_width("é", &glyphs), Some(7));
        // CJK stays outside the bitmap pages (unihex deferred): still `?`.
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

    fn test_map_text_glyphs() -> HudFontGlyphMap {
        let mut glyphs = HudFontGlyphMap::new();
        for byte in b' '..=b'~' {
            glyphs.insert_first_wins(
                char::from(byte),
                HudAsciiGlyph {
                    uv: crate::HudUvRect {
                        min: [0.0, 0.0],
                        max: [1.0, 1.0],
                    },
                    width: 6,
                    height: 8,
                    advance: match byte {
                        b' ' => 4,
                        b'?' => 5,
                        _ => 6,
                    },
                    ascent: 7,
                },
            );
        }
        glyphs
    }
}
