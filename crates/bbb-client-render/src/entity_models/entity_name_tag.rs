//! Vanilla `NameTagFeatureRenderer` line geometry and submission ordering.
//!
//! Each logical line owns the normal layer and, when the entity is not discrete,
//! the matching see-through layer. See-through lines sort far-to-near by entity
//! distance; normal lines retain collector submission order.

use std::{
    cell::RefCell,
    collections::HashMap,
    rc::Rc,
    sync::{Arc, OnceLock},
};

use bbb_render_types::HudFontGlyphBook;
use glam::{Mat4, Vec3};

use crate::entity_models::entity_dispatch_render_offset;
use crate::item_models::{ItemModelMesh, ItemModelVertex};
use crate::item_render::item_models::display_text::{append_mesh, WorldTextBuffers};
use crate::particles::camera_billboard_orientation;
use crate::text_mesh::{
    mesh_world_text_line, world_text_argb, world_text_chars, world_text_visual_order,
    world_text_width, TextLayoutCache, TextMesh, TextPen, VisualGlyph, UNTEXTURED_UVS,
};
use crate::{CameraPose, EntityModelInstance, Renderer, WorldTextRun};

pub const ENTITY_NAME_TAG_SCALE: f32 = 0.025;
pub const ENTITY_NAME_TAG_BACKGROUND_OPACITY: f32 = 0.25;
pub const ENTITY_NAME_TAG_FONT_TEXTURE: &str = crate::item_models::DISPLAY_TEXT_FONT_TEXTURE;

const ENTITY_NAME_TAG_HALF_ALPHA: u8 = 0x80;
const ENTITY_NAME_TAG_FULL_ALPHA: u8 = 0xFF;
const ENTITY_NAME_TAG_HEIGHT: f32 = 9.0;
const ENTITY_NAME_TAG_ATTACHMENT_LIFT: f32 = 0.5;
const ENTITY_NAME_TAG_SCORE_TO_NAME_WORLD_LIFT: f32 =
    ENTITY_NAME_TAG_HEIGHT * 1.15 * ENTITY_NAME_TAG_SCALE;
const ENTITY_NAME_TAG_BACKGROUND_Z: f32 = -0.01;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EntityNameTagLineKind {
    BelowName,
    Name,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntityNameTagRenderType {
    Normal,
    SeeThrough,
}

#[derive(Debug, Clone, PartialEq)]
pub struct EntityNameTagLayerSubmission {
    pub render_type: EntityNameTagRenderType,
    pub base_text_argb: u32,
    pub background_argb: u32,
    pub light: [f32; 2],
}

#[derive(Debug, Clone, PartialEq)]
pub struct EntityNameTagSubmission {
    pub entity_id: i32,
    pub submit_sequence: u64,
    pub line_kind: EntityNameTagLineKind,
    pub runs: Vec<WorldTextRun>,
    pub width: i32,
    pub line_offset: i32,
    pub distance_to_camera_sq: f32,
    pub is_discrete: bool,
    pub transform: Mat4,
    pub normal_layer: EntityNameTagLayerSubmission,
    pub see_through_layer: Option<EntityNameTagLayerSubmission>,
}

#[derive(Debug, Clone)]
pub struct EntityNameTagSurface {
    pub submission: EntityNameTagSubmission,
    dynamic_atlas_dims: Option<(u32, u32)>,
    normal: Arc<TextMesh>,
    see_through: Option<Arc<TextMesh>>,
    world_normal: OnceLock<ItemModelMesh>,
    world_see_through: OnceLock<ItemModelMesh>,
}

impl PartialEq for EntityNameTagSurface {
    fn eq(&self, other: &Self) -> bool {
        self.submission == other.submission
            && self.dynamic_atlas_dims == other.dynamic_atlas_dims
            && self.normal == other.normal
            && self.see_through == other.see_through
    }
}

impl EntityNameTagSurface {
    pub(crate) fn rescale_for_atlas_dims(&mut self, new_dims: (u32, u32)) -> bool {
        let Some((old_w, old_h)) = self.dynamic_atlas_dims else {
            return false;
        };
        if (old_w, old_h) == new_dims || new_dims.0 == 0 || new_dims.1 == 0 {
            return false;
        }
        let scale_x = old_w as f32 / new_dims.0 as f32;
        let scale_y = old_h as f32 / new_dims.1 as f32;
        self.normal = Arc::new(self.normal.rescaled_for_atlas(scale_x, scale_y));
        self.see_through = self
            .see_through
            .as_ref()
            .map(|mesh| Arc::new(mesh.rescaled_for_atlas(scale_x, scale_y)));
        self.world_normal = OnceLock::new();
        self.world_see_through = OnceLock::new();
        self.dynamic_atlas_dims = Some(new_dims);
        true
    }

    pub fn normal_vertex_count(&self) -> usize {
        self.normal.0.vertex_count()
    }

    pub fn see_through_vertex_count(&self) -> usize {
        self.see_through
            .as_ref()
            .map_or(0, |mesh| mesh.0.vertex_count())
    }

    pub fn is_empty(&self) -> bool {
        self.normal.0.is_empty()
            && self
                .see_through
                .as_ref()
                .is_none_or(|mesh| mesh.0.is_empty())
    }

    fn same_geometry(&self, other: &Self) -> bool {
        self.submission.transform == other.submission.transform
            && Arc::ptr_eq(&self.normal, &other.normal)
            && match (&self.see_through, &other.see_through) {
                (Some(left), Some(right)) => Arc::ptr_eq(left, right),
                (None, None) => true,
                _ => false,
            }
    }

    fn line_key(&self) -> (i32, EntityNameTagLineKind) {
        (self.submission.entity_id, self.submission.line_kind)
    }

    fn retain_world_meshes(&mut self, previous: &mut Self) {
        if self.same_geometry(previous) {
            self.world_normal = std::mem::take(&mut previous.world_normal);
            self.world_see_through = std::mem::take(&mut previous.world_see_through);
        }
    }

    fn normal_mesh(&self) -> &ItemModelMesh {
        self.world_normal
            .get_or_init(|| self.normal.transformed(self.submission.transform))
    }

    fn see_through_mesh(&self) -> Option<&ItemModelMesh> {
        self.see_through.as_ref().map(|mesh| {
            self.world_see_through
                .get_or_init(|| mesh.transformed(self.submission.transform))
        })
    }
}

/// The dispatcher-root position shared by an entity model and its name tags.
/// The model source already includes passenger/new-minecart/Shulker offsets;
/// this adds the renderer-owned Enderman jitter and Avatar crouch drop once.
pub fn entity_name_tag_root_position(instance: EntityModelInstance) -> [f32; 3] {
    let offset = entity_dispatch_render_offset(&instance);
    [
        instance.position[0] + offset[0],
        instance.position[1] + offset[1],
        instance.position[2] + offset[2],
    ]
}

/// Bakes score-then-name logical lines exactly in `EntityRenderer.submitNameDisplay`
/// order. The caller supplies already-localized/style-resolved runs.
#[allow(clippy::too_many_arguments)]
pub fn bake_entity_name_tag_surfaces(
    entity_id: i32,
    first_submit_sequence: u64,
    root_position: [f32; 3],
    attachment_y: f32,
    line_offset: i32,
    distance_to_camera_sq: f32,
    is_discrete: bool,
    name_runs: &[WorldTextRun],
    below_name_runs: Option<&[WorldTextRun]>,
    light_coords: u32,
    camera_pose: CameraPose,
    fonts: &HudFontGlyphBook,
    obfuscated_seed: u64,
) -> Vec<EntityNameTagSurface> {
    let mut surfaces = Vec::with_capacity(usize::from(below_name_runs.is_some()) + 1);
    if let Some(runs) = below_name_runs {
        if let Some(surface) = bake_entity_name_tag_line(
            entity_id,
            first_submit_sequence,
            root_position,
            attachment_y,
            line_offset,
            distance_to_camera_sq,
            is_discrete,
            EntityNameTagLineKind::BelowName,
            false,
            runs,
            light_coords,
            camera_pose,
            fonts,
            obfuscated_seed,
        ) {
            surfaces.push(surface);
        }
    }
    if let Some(surface) = bake_entity_name_tag_line(
        entity_id,
        first_submit_sequence + u64::from(below_name_runs.is_some()),
        root_position,
        attachment_y,
        line_offset,
        distance_to_camera_sq,
        is_discrete,
        EntityNameTagLineKind::Name,
        below_name_runs.is_some(),
        name_runs,
        light_coords,
        camera_pose,
        fonts,
        obfuscated_seed,
    ) {
        surfaces.push(surface);
    }
    surfaces
}

/// The font-space meshes of one name-tag line, independent of the camera.
struct EntityNameTagLayout {
    width: i32,
    normal: Arc<TextMesh>,
    see_through: Option<Arc<TextMesh>>,
}

#[derive(PartialEq, Eq, Hash)]
struct EntityNameTagLayoutKey {
    fonts: u64,
    runs: Vec<WorldTextRun>,
    line_offset: i32,
    is_discrete: bool,
    light_coords: u32,
}

thread_local! {
    static NAME_TAG_LAYOUTS: RefCell<TextLayoutCache<EntityNameTagLayoutKey, EntityNameTagLayout>> =
        RefCell::new(TextLayoutCache::default());
}

pub(crate) fn next_name_tag_layout_frame() {
    NAME_TAG_LAYOUTS.with(|cache| cache.borrow_mut().next_frame());
}

#[allow(clippy::too_many_arguments)]
fn bake_entity_name_tag_line(
    entity_id: i32,
    submit_sequence: u64,
    root_position: [f32; 3],
    attachment_y: f32,
    line_offset: i32,
    distance_to_camera_sq: f32,
    is_discrete: bool,
    line_kind: EntityNameTagLineKind,
    lift_for_below_name: bool,
    runs: &[WorldTextRun],
    light_coords: u32,
    camera_pose: CameraPose,
    fonts: &HudFontGlyphBook,
    obfuscated_seed: u64,
) -> Option<EntityNameTagSurface> {
    let name_lift = if lift_for_below_name {
        ENTITY_NAME_TAG_SCORE_TO_NAME_WORLD_LIFT
    } else {
        0.0
    };
    let transform = Mat4::from_translation(
        Vec3::from_array(root_position)
            + Vec3::Y * (attachment_y + ENTITY_NAME_TAG_ATTACHMENT_LIFT + name_lift),
    ) * camera_billboard_orientation(camera_pose)
        * Mat4::from_scale(Vec3::new(
            ENTITY_NAME_TAG_SCALE,
            -ENTITY_NAME_TAG_SCALE,
            ENTITY_NAME_TAG_SCALE,
        ));
    let background_argb = ((ENTITY_NAME_TAG_BACKGROUND_OPACITY * 255.0) as u32) << 24;
    let original_light = shader_light(light_coords);
    let emitted_light = shader_light(light_coords_with_emission(light_coords, 2));

    let normal_layer = EntityNameTagLayerSubmission {
        render_type: EntityNameTagRenderType::Normal,
        base_text_argb: world_text_argb(
            None,
            if is_discrete {
                ENTITY_NAME_TAG_HALF_ALPHA
            } else {
                ENTITY_NAME_TAG_FULL_ALPHA
            },
        ),
        background_argb: if is_discrete { background_argb } else { 0 },
        light: if is_discrete {
            original_light
        } else {
            emitted_light
        },
    };
    let see_through_layer = (!is_discrete).then_some(EntityNameTagLayerSubmission {
        render_type: EntityNameTagRenderType::SeeThrough,
        base_text_argb: world_text_argb(None, ENTITY_NAME_TAG_HALF_ALPHA),
        background_argb,
        light: original_light,
    });
    let build = || {
        // NameTagFeatureRenderer centers from `Font.width(Component)` (logical
        // order), then Font draws the Component's visual-order sequence.
        let width = world_text_width(world_text_chars(runs), fonts);
        let visual = world_text_visual_order(runs, fonts);
        let line_seed = obfuscated_seed.wrapping_add(submit_sequence);
        let layer_mesh = |layer: &EntityNameTagLayerSubmission| {
            layout_layer(&visual, width, line_offset, layer, fonts, line_seed)
        };
        EntityNameTagLayout {
            width,
            normal: Arc::new(layer_mesh(&normal_layer)),
            see_through: see_through_layer
                .as_ref()
                .map(|layer| Arc::new(layer_mesh(layer))),
        }
    };
    let build_consistent = || {
        let dims_before = fonts.dynamic_atlas().map(|atlas| atlas.atlas_dimensions());
        let built = build();
        let dims_after = fonts.dynamic_atlas().map(|atlas| atlas.atlas_dimensions());
        if dims_after != dims_before {
            (build(), dims_after)
        } else {
            (built, dims_after)
        }
    };
    let (layout, dynamic_atlas_dims) = if runs.iter().any(|run| run.style.obfuscated) {
        let (built, dims) = build_consistent();
        (Rc::new(built), dims)
    } else {
        let make_key = |fonts_gen| EntityNameTagLayoutKey {
            fonts: fonts_gen,
            runs: runs.to_vec(),
            line_offset,
            is_discrete,
            light_coords,
        };
        let gen_before = fonts.generation();
        let cached = NAME_TAG_LAYOUTS.with(|cache| {
            cache
                .borrow_mut()
                .get_or_build(make_key(gen_before), &build)
        });
        let gen_after = fonts.generation();
        let dims_after = fonts.dynamic_atlas().map(|atlas| atlas.atlas_dimensions());
        if gen_after != gen_before {
            let rebuilt = NAME_TAG_LAYOUTS
                .with(|cache| cache.borrow_mut().get_or_build(make_key(gen_after), &build));
            (rebuilt, dims_after)
        } else {
            (cached, dims_after)
        }
    };
    let surface = EntityNameTagSurface {
        submission: EntityNameTagSubmission {
            entity_id,
            submit_sequence,
            line_kind,
            runs: runs.to_vec(),
            width: layout.width,
            line_offset,
            distance_to_camera_sq,
            is_discrete,
            transform,
            normal_layer,
            see_through_layer,
        },
        dynamic_atlas_dims,
        normal: Arc::clone(&layout.normal),
        see_through: layout.see_through.clone(),
        world_normal: OnceLock::new(),
        world_see_through: OnceLock::new(),
    };
    (!surface.is_empty()).then_some(surface)
}

fn layout_layer(
    visual: &[VisualGlyph],
    width: i32,
    line_offset: i32,
    layer: &EntityNameTagLayerSubmission,
    fonts: &HudFontGlyphBook,
    obfuscated_seed: u64,
) -> TextMesh {
    let mut mesh = TextMesh::default();
    let x = -(width as f32) / 2.0;
    let y = line_offset as f32;
    if layer.background_argb != 0 {
        mesh.push_rect(
            [
                x - 1.0,
                y - 1.0,
                x + width as f32,
                y + ENTITY_NAME_TAG_HEIGHT,
            ],
            ENTITY_NAME_TAG_BACKGROUND_Z,
            UNTEXTURED_UVS,
            layer.background_argb,
            layer.light,
        );
    }
    let alpha = (layer.base_text_argb >> 24) as u8;
    mesh_world_text_line(
        &mut mesh,
        None,
        visual,
        &mut TextPen::new(x, y, obfuscated_seed),
        fonts,
        |run| world_text_argb(run.color, alpha),
        false,
        layer.light,
        UNTEXTURED_UVS,
    );
    mesh
}

#[derive(Default)]
pub(crate) struct EntityNameTagGeometry {
    pub(crate) see_through: (Vec<ItemModelVertex>, Vec<u32>),
    pub(crate) normal: (Vec<ItemModelVertex>, Vec<u32>),
}

fn sort_name_tag_indices(geometry: &mut EntityNameTagGeometry, surfaces: &[EntityNameTagSurface]) {
    let mut vertex_base = 0_u32;
    let mut see_through = surfaces
        .iter()
        .filter_map(|surface| {
            surface.see_through.as_ref().map(|mesh| {
                let base = vertex_base;
                vertex_base += mesh.0.vertices.len() as u32;
                (
                    surface.submission.distance_to_camera_sq,
                    base,
                    &mesh.0.indices,
                )
            })
        })
        .collect::<Vec<_>>();
    // Equal-distance score/name lines retain collector order.
    see_through.sort_by(|left, right| right.0.total_cmp(&left.0));
    geometry.see_through.1.clear();
    for (_, base, indices) in see_through {
        geometry
            .see_through
            .1
            .extend(indices.iter().map(|index| base + index));
    }
}

pub(crate) fn merge_entity_name_tag_surfaces(
    surfaces: &[EntityNameTagSurface],
) -> EntityNameTagGeometry {
    let mut geometry = EntityNameTagGeometry::default();
    for surface in surfaces {
        if let Some(mesh) = surface.see_through_mesh() {
            geometry.see_through.0.extend_from_slice(&mesh.vertices);
        }
        append_mesh(&mut geometry.normal, surface.normal_mesh());
    }
    sort_name_tag_indices(&mut geometry, surfaces);
    geometry
}

pub(crate) struct EntityNameTagRenderState {
    pub(crate) dirty: bool,
    sort_dirty: bool,
    geometry: EntityNameTagGeometry,
    pub(crate) buffers: [WorldTextBuffers; 2],
}

impl Default for EntityNameTagRenderState {
    fn default() -> Self {
        Self {
            dirty: true,
            sort_dirty: false,
            geometry: EntityNameTagGeometry::default(),
            buffers: std::array::from_fn(|_| WorldTextBuffers::default()),
        }
    }
}

impl EntityNameTagRenderState {
    pub(crate) fn restore_unsubmitted(&mut self) {
        self.geometry = EntityNameTagGeometry::default();
        self.dirty = true;
        self.sort_dirty = false;
    }
}

impl Renderer {
    pub fn set_entity_name_tag_surfaces(&mut self, surfaces: Vec<EntityNameTagSurface>) {
        let mut surfaces: Vec<_> = if self.pack.item_frame_map_text_font_atlas.is_some() {
            surfaces
                .into_iter()
                .filter(|surface| !surface.is_empty())
                .collect()
        } else {
            Vec::new()
        };
        if self.entity_name_tag_surfaces.len() == surfaces.len()
            && self
                .entity_name_tag_surfaces
                .iter()
                .zip(&surfaces)
                .all(|(old, new)| old.line_key() == new.line_key() && old.same_geometry(new))
        {
            for (old, new) in self.entity_name_tag_surfaces.iter_mut().zip(surfaces) {
                self.pack.entity_name_tags.sort_dirty |= old.see_through.is_some()
                    && old.submission.distance_to_camera_sq != new.submission.distance_to_camera_sq;
                old.submission = new.submission;
            }
            return;
        }
        let mut previous = std::mem::take(&mut self.entity_name_tag_surfaces)
            .into_iter()
            .map(|surface| (surface.line_key(), surface))
            .collect::<HashMap<_, _>>();
        for surface in &mut surfaces {
            if let Some(previous) = previous.get_mut(&surface.line_key()) {
                surface.retain_world_meshes(previous);
            }
        }
        self.entity_name_tag_surfaces = surfaces;
        self.pack.entity_name_tags.dirty = true;
    }

    pub(crate) fn prepare_entity_name_tags(&mut self, encoder: &mut wgpu::CommandEncoder) {
        if let Some(current_dims) = self
            .pack
            .hud_font_book
            .dynamic_atlas()
            .map(|atlas| atlas.atlas_dimensions())
        {
            let mut rescaled = false;
            for surface in &mut self.entity_name_tag_surfaces {
                rescaled |= surface.rescale_for_atlas_dims(current_dims);
            }
            if rescaled {
                self.pack.entity_name_tags.dirty = true;
            }
        }
        let state = &mut self.pack.entity_name_tags;
        if state.dirty {
            let geometry = merge_entity_name_tag_surfaces(&self.entity_name_tag_surfaces);
            let belt = self.frame_uploads.belt();
            for (buffers, (next, previous)) in state.buffers.iter_mut().zip([
                (&geometry.see_through, &state.geometry.see_through),
                (&geometry.normal, &state.geometry.normal),
            ]) {
                buffers.vertices.upload_changed_staged_with_encoder(
                    &self.device,
                    encoder,
                    belt,
                    bytemuck::cast_slice(&next.0),
                    bytemuck::cast_slice(&previous.0),
                );
                buffers.indices.upload_staged(
                    &self.device,
                    encoder,
                    belt,
                    bytemuck::cast_slice(&next.1),
                );
                buffers.index_count = next.1.len() as u32;
            }
            state.geometry = geometry;
            state.dirty = false;
            state.sort_dirty = false;
        } else if state.sort_dirty {
            sort_name_tag_indices(&mut state.geometry, &self.entity_name_tag_surfaces);
            let belt = self.frame_uploads.belt();
            state.buffers[0].indices.upload_staged(
                &self.device,
                encoder,
                belt,
                bytemuck::cast_slice(&state.geometry.see_through.1),
            );
            state.buffers[0].index_count = state.geometry.see_through.1.len() as u32;
            state.sort_dirty = false;
        }
    }
}

fn light_coords_with_emission(light_coords: u32, emission: u32) -> u32 {
    let block = ((light_coords >> 4) & 0xF).max(emission.min(15));
    let sky = (light_coords >> 20) & 0xF;
    block << 4 | sky << 20
}

fn shader_light(light_coords: u32) -> [f32; 2] {
    [
        ((light_coords >> 4) & 0xF) as f32 / 15.0,
        ((light_coords >> 20) & 0xF) as f32 / 15.0,
    ]
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::*;
    use crate::{
        EntityDefaultPlayerSkin, EntityModelKind, EntityPlayerSkin, HudAsciiGlyph, HudFontGlyphMap,
        HudTextStyle, HudUvRect, PlayerModelPartVisibility,
    };

    fn fonts() -> HudFontGlyphBook {
        let mut glyphs = HudFontGlyphMap::new();
        for ch in ['A', 'B', '?', ' '] {
            glyphs.insert_first_wins(
                ch,
                HudAsciiGlyph {
                    uv: HudUvRect {
                        min: [0.25, 0.5],
                        max: [0.3, 0.6],
                    },
                    is_empty: ch == ' ',
                    width: if ch == ' ' { 0 } else { 5 },
                    height: if ch == ' ' { 0 } else { 8 },
                    advance: if ch == ' ' { 4.0 } else { 6.0 },
                    bearing_top: 7.0,
                    ..HudAsciiGlyph::default()
                },
            );
        }
        HudFontGlyphBook::from_fonts(
            "minecraft:default",
            BTreeMap::from([("minecraft:default".to_string(), glyphs)]),
        )
    }

    fn bidirectional_fonts() -> HudFontGlyphBook {
        let mut glyphs = HudFontGlyphMap::new();
        for (ch, min_x) in [('1', 0.1), ('2', 0.2), ('3', 0.3), ('-', 0.4), ('?', 0.5)] {
            glyphs.insert_first_wins(
                ch,
                HudAsciiGlyph {
                    uv: HudUvRect {
                        min: [min_x, 0.0],
                        max: [min_x + 0.05, 0.5],
                    },
                    width: 5,
                    height: 8,
                    advance: 6.0,
                    bearing_top: 7.0,
                    ..HudAsciiGlyph::default()
                },
            );
        }
        let mut fonts = HudFontGlyphBook::from_fonts(
            "minecraft:default",
            BTreeMap::from([("minecraft:default".to_string(), glyphs)]),
        );
        fonts.set_default_right_to_left(true);
        fonts
    }

    fn camera() -> CameraPose {
        CameraPose {
            position: [0.0, 0.0, 0.0],
            y_rot: 0.0,
            x_rot: 0.0,
            eye_height: 0.0,
        }
    }

    fn run(text: &str) -> WorldTextRun {
        WorldTextRun::plain(text)
    }

    #[test]
    fn entity_name_tag_non_discrete_emits_vanilla_dual_layers_and_lighting() {
        let surfaces = bake_entity_name_tag_surfaces(
            7,
            10,
            [1.0, 2.0, 3.0],
            1.8,
            0,
            25.0,
            false,
            &[run("A")],
            None,
            (1 << 4) | (9 << 20),
            camera(),
            &fonts(),
            0,
        );
        assert_eq!(surfaces.len(), 1);
        let surface = &surfaces[0];
        assert!(surface.normal_vertex_count() > 0);
        assert!(surface.see_through_vertex_count() > surface.normal_vertex_count());
        assert_eq!(surface.submission.normal_layer.base_text_argb, 0xFFFF_FFFF);
        assert_eq!(surface.submission.normal_layer.background_argb, 0);
        assert_eq!(surface.submission.normal_layer.light, [2.0 / 15.0, 0.6]);
        let see = surface.submission.see_through_layer.as_ref().unwrap();
        assert_eq!(see.base_text_argb, 0x80FF_FFFF);
        assert_eq!(see.background_argb, 0x3F00_0000);
        assert_eq!(see.light, [1.0 / 15.0, 0.6]);
        assert!((surface.submission.transform.w_axis.y - 4.3).abs() < 1.0e-6);
    }

    #[test]
    fn entity_name_tag_centers_logical_width_and_draws_visual_glyph_order() {
        let surfaces = bake_entity_name_tag_surfaces(
            7,
            0,
            [0.0; 3],
            1.0,
            0,
            4.0,
            false,
            &[run("123-")],
            None,
            15 << 20,
            camera(),
            &bidirectional_fonts(),
            0,
        );
        let surface = &surfaces[0];
        assert_eq!(surface.submission.runs[0].text, "123-");
        assert_eq!(surface.submission.width, 24);
        let glyph_uvs = surface
            .normal_mesh()
            .vertices
            .as_chunks::<4>()
            .0
            .iter()
            .map(|vertices| vertices[0].uv[0])
            .collect::<Vec<_>>();
        assert_eq!(glyph_uvs, [0.4, 0.1, 0.2, 0.3]);
    }

    #[test]
    fn entity_name_tag_root_shares_avatar_crouch_dispatch_offset() {
        let player = EntityModelInstance::new(
            9,
            EntityModelKind::Player {
                skin: EntityPlayerSkin::Default(EntityDefaultPlayerSkin::WideSteve),
                parts: PlayerModelPartVisibility::from_vanilla_mask(
                    PlayerModelPartVisibility::ALL_MASK,
                ),
            },
            [1.0, 2.0, 3.0],
            0.0,
        )
        .with(|s| s.is_crouching = true);
        assert_eq!(entity_name_tag_root_position(player), [1.0, 1.875, 3.0]);
        assert_eq!(
            entity_name_tag_root_position(player.with(|s| s.is_crouching = false)),
            [1.0, 2.0, 3.0]
        );
    }

    #[test]
    fn entity_name_tag_discrete_emits_only_half_alpha_normal_layer() {
        let surfaces = bake_entity_name_tag_surfaces(
            7,
            0,
            [0.0; 3],
            1.0,
            -10,
            4.0,
            true,
            &[run("A")],
            None,
            (5 << 4) | (15 << 20),
            camera(),
            &fonts(),
            0,
        );
        let surface = &surfaces[0];
        assert_eq!(surface.see_through_vertex_count(), 0);
        assert_eq!(surface.submission.normal_layer.base_text_argb, 0x80FF_FFFF);
        assert_eq!(surface.submission.normal_layer.background_argb, 0x3F00_0000);
        assert_eq!(surface.submission.normal_layer.light, [1.0 / 3.0, 1.0]);
        assert_eq!(surface.submission.line_offset, -10);
    }

    #[test]
    fn entity_name_tag_score_precedes_name_and_name_gets_world_lift() {
        let surfaces = bake_entity_name_tag_surfaces(
            8,
            30,
            [0.0; 3],
            2.0,
            0,
            9.0,
            false,
            &[run("A")],
            Some(&[run("B")]),
            15 << 20,
            camera(),
            &fonts(),
            0,
        );
        assert_eq!(surfaces.len(), 2);
        assert_eq!(
            surfaces[0].submission.line_kind,
            EntityNameTagLineKind::BelowName
        );
        assert_eq!(surfaces[0].submission.submit_sequence, 30);
        assert_eq!(
            surfaces[1].submission.line_kind,
            EntityNameTagLineKind::Name
        );
        assert_eq!(surfaces[1].submission.submit_sequence, 31);
        let score_y = surfaces[0].submission.transform.w_axis.y;
        let name_y = surfaces[1].submission.transform.w_axis.y;
        assert!((name_y - score_y - ENTITY_NAME_TAG_SCORE_TO_NAME_WORLD_LIFT).abs() < 1.0e-6);
    }

    #[test]
    fn entity_name_tag_merge_sorts_see_through_far_first_and_keeps_normal_order() {
        let bake = |entity_id, sequence, x, distance| {
            bake_entity_name_tag_surfaces(
                entity_id,
                sequence,
                [x, 0.0, 0.0],
                1.0,
                0,
                distance,
                false,
                &[run("A")],
                None,
                15 << 20,
                camera(),
                &fonts(),
                0,
            )
            .remove(0)
        };
        let near = bake(1, 0, 1.0, 1.0);
        let far = bake(2, 1, 9.0, 81.0);
        let geometry = merge_entity_name_tag_surfaces(&[near, far]);
        // See-through starts with the far surface's background; normal has no
        // background and starts with the near surface's glyph.
        assert!(geometry.see_through.0[geometry.see_through.1[0] as usize].position[0] > 8.0);
        assert!(geometry.normal.0[0].position[0] < 2.0);
    }

    #[test]
    fn entity_name_tag_dual_layers_share_the_obfuscated_custom_glyph() {
        let mut custom = HudFontGlyphMap::new();
        for (ch, min_x) in [('A', 0.4), ('B', 0.5), ('C', 0.6)] {
            custom.insert_first_wins(
                ch,
                HudAsciiGlyph {
                    uv: HudUvRect {
                        min: [min_x, 0.0],
                        max: [min_x + 0.05, 0.5],
                    },
                    width: 5,
                    height: 8,
                    advance: 7.0,
                    bearing_top: 7.0,
                    ..HudAsciiGlyph::default()
                },
            );
        }
        let fonts = HudFontGlyphBook::from_fonts(
            "minecraft:default",
            BTreeMap::from([
                ("example:custom".to_string(), custom),
                ("minecraft:default".to_string(), HudFontGlyphMap::new()),
            ]),
        );
        let runs = [WorldTextRun {
            shadow_color: None,
            text: "A".to_string(),
            style: HudTextStyle {
                obfuscated: true,
                ..Default::default()
            },
            color: None,
            font: Some("example:custom".to_string()),
            event_id: None,
        }];
        let surface = bake_entity_name_tag_surfaces(
            7,
            3,
            [0.0; 3],
            1.0,
            0,
            4.0,
            false,
            &runs,
            None,
            15 << 20,
            camera(),
            &fonts,
            9,
        )
        .remove(0);

        assert_eq!(surface.submission.width, 7);
        assert_eq!(
            surface.normal_mesh().vertices[0].uv,
            surface.see_through_mesh().unwrap().vertices[4].uv
        );
        assert!((0.4..=0.6).contains(&surface.normal_mesh().vertices[0].uv[0]));
    }
    #[test]
    fn fractional_name_tag_centers_signed_ceil_and_keeps_float_pen() {
        let mut glyphs = HudFontGlyphMap::new();
        for (ch, advance) in [('a', 0.5), ('b', -3.0)] {
            glyphs.insert_first_wins(
                ch,
                HudAsciiGlyph {
                    advance,
                    width: 1,
                    height: 8,
                    bold_offset: 0.5,
                    ..HudAsciiGlyph::default()
                },
            );
        }
        let fonts = HudFontGlyphBook::from(glyphs);
        for (text, bold, width, delta) in [
            ("aa", false, 1, 0.5),
            ("ba", false, -2, -3.0),
            ("aa", true, 2, 1.0),
        ] {
            let mut run = run(text);
            run.style.bold = bold;
            let surfaces = bake_entity_name_tag_surfaces(
                7,
                0,
                [0.0; 3],
                1.0,
                0,
                4.0,
                false,
                &[run],
                None,
                15 << 20,
                camera(),
                &fonts,
                0,
            );
            let surface = &surfaces[0];
            assert_eq!(surface.submission.width, width);
            let inverse = surface.submission.transform.inverse();
            let first = inverse
                .transform_point3(Vec3::from_array(surface.normal_mesh().vertices[0].position));
            let second = inverse.transform_point3(Vec3::from_array(
                surface.normal_mesh().vertices[if bold { 8 } else { 4 }].position,
            ));
            assert!((first.x + width as f32 / 2.0 - if bold { -0.1 } else { 0.0 }).abs() < 0.00001);
            assert!((second.x - first.x - delta).abs() < 0.00001);
        }
    }
    #[test]
    fn entity_name_tag_zero_bitmap_submits_and_empty_only_advances() {
        for render_scale in [1.0, f32::INFINITY] {
            let mut glyphs = HudFontGlyphMap::new();
            glyphs.insert_first_wins(
                'a',
                HudAsciiGlyph {
                    advance: 3.0,
                    render_scale,
                    ..Default::default()
                },
            );
            glyphs.insert_first_wins(' ', HudAsciiGlyph::empty(3.0));
            let fonts = HudFontGlyphBook::from(glyphs);
            let surfaces = bake_entity_name_tag_surfaces(
                7,
                0,
                [0.0; 3],
                1.0,
                0,
                4.0,
                false,
                &[run("a a")],
                None,
                15 << 20,
                camera(),
                &fonts,
                0,
            );
            let surface = &surfaces[0];
            assert_eq!(surface.submission.width, 9);
            assert_eq!(surface.normal_mesh().vertices.len(), 8);
            if render_scale.is_infinite() {
                assert!(surface.normal_mesh().vertices[2]
                    .position
                    .iter()
                    .any(|x| x.is_nan()));
            } else {
                let inverse = surface.submission.transform.inverse();
                let first = inverse
                    .transform_point3(Vec3::from_array(surface.normal_mesh().vertices[0].position));
                let second = inverse
                    .transform_point3(Vec3::from_array(surface.normal_mesh().vertices[4].position));
                assert!((second.x - first.x - 6.0).abs() < 0.00001);
                assert_eq!(
                    surface.normal_mesh().vertices[0].position,
                    surface.normal_mesh().vertices[2].position
                );
            }
        }
    }

    #[test]
    fn prepare_entity_name_tags_records_into_active_encoder_without_orphan_staged_encoder() {
        let Some(mut renderer) = crate::Renderer::new_offscreen(16, 16) else {
            return;
        };
        let mut glyphs = HudFontGlyphMap::new();
        glyphs.insert_first_wins(
            'A',
            HudAsciiGlyph {
                uv: HudUvRect {
                    min: [0.0, 0.0],
                    max: [0.5, 0.5],
                },
                advance: 6.0,
                width: 6,
                height: 8,
                ..Default::default()
            },
        );
        renderer
            .upload_hud_font_atlas(8, 8, &vec![0xff; 8 * 8 * 4], glyphs)
            .unwrap();
        let surfaces = bake_entity_name_tag_surfaces(
            9,
            0,
            [0.0, 64.0, 0.0],
            1.0,
            0,
            4.0,
            false,
            &[run("A")],
            None,
            15 << 20,
            camera(),
            &renderer.pack.hud_font_book,
            0,
        );
        renderer.set_entity_name_tag_surfaces(surfaces);
        let mut encoder = renderer.frame_uploads.take_encoder(&renderer.device);
        assert!(!renderer.frame_uploads.has_pending_encoder());
        renderer.prepare_entity_name_tags(&mut encoder);
        assert!(
            !renderer.frame_uploads.has_pending_encoder(),
            "prepare_entity_name_tags must record into the passed CommandEncoder"
        );
        renderer.frame_uploads.finish_belt();
        renderer.queue.submit(Some(encoder.finish()));
        renderer.frame_uploads.recall_belt();
    }
}

#[cfg(test)]
#[path = "entity_name_tag/shadow_tests.rs"]
mod shadow_tests;
