//! Vanilla `NameTagFeatureRenderer` line geometry and submission ordering.
//!
//! Each logical line owns the normal layer and, when the entity is not discrete,
//! the matching see-through layer. See-through lines sort far-to-near by entity
//! distance; normal lines retain collector submission order.

use bbb_render_types::{HudFontGlyphBook, HudObfuscatedRandom};
use glam::{Mat4, Vec3};

use crate::entity_models::entity_dispatch_render_offset;
use crate::particles::camera_billboard_orientation;
use crate::{CameraPose, EntityModelInstance, Renderer, WorldTextRun};

use super::display_text::{
    append_mesh, append_quad, append_untextured_rect, argb_to_color, display_text_glyph,
    display_text_runs_width, text_argb, visual_world_text_runs,
};
use super::{ItemModelMesh, ItemModelVertex};

pub const ENTITY_NAME_TAG_SCALE: f32 = 0.025;
pub const ENTITY_NAME_TAG_BACKGROUND_OPACITY: f32 = 0.25;
pub const ENTITY_NAME_TAG_FONT_TEXTURE: &str = super::DISPLAY_TEXT_FONT_TEXTURE;

const ENTITY_NAME_TAG_HALF_ALPHA: u8 = 0x80;
const ENTITY_NAME_TAG_FULL_ALPHA: u8 = 0xFF;
const ENTITY_NAME_TAG_HEIGHT: f32 = 9.0;
const ENTITY_NAME_TAG_ATTACHMENT_LIFT: f32 = 0.5;
const ENTITY_NAME_TAG_SCORE_TO_NAME_WORLD_LIFT: f32 =
    ENTITY_NAME_TAG_HEIGHT * 1.15 * ENTITY_NAME_TAG_SCALE;
const ENTITY_NAME_TAG_EFFECT_Z: f32 = 0.01;
const ENTITY_NAME_TAG_BACKGROUND_Z: f32 = -0.01;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
    pub width: u32,
    pub line_offset: i32,
    pub distance_to_camera_sq: f32,
    pub is_discrete: bool,
    pub transform: Mat4,
    pub normal_layer: EntityNameTagLayerSubmission,
    pub see_through_layer: Option<EntityNameTagLayerSubmission>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct EntityNameTagSurface {
    pub submission: EntityNameTagSubmission,
    normal: ItemModelMesh,
    see_through: Option<ItemModelMesh>,
}

impl EntityNameTagSurface {
    pub fn normal_vertex_count(&self) -> usize {
        self.normal.vertex_count()
    }

    pub fn see_through_vertex_count(&self) -> usize {
        self.see_through
            .as_ref()
            .map_or(0, ItemModelMesh::vertex_count)
    }

    pub fn is_empty(&self) -> bool {
        self.normal.is_empty()
            && self
                .see_through
                .as_ref()
                .is_none_or(ItemModelMesh::is_empty)
    }
}

/// The dispatcher-root position shared by an entity model and its name tags.
/// The model source already includes passenger/new-minecart/Shulker offsets;
/// this adds the renderer-owned Enderman jitter and Avatar crouch drop once.
pub fn entity_name_tag_root_position(instance: EntityModelInstance) -> [f32; 3] {
    let offset = entity_dispatch_render_offset(instance);
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
    // NameTagFeatureRenderer centers from `Font.width(Component)` (logical
    // order), then Font draws the Component's visual-order sequence.
    let width = display_text_runs_width(runs, fonts);
    let visual_runs = visual_world_text_runs(runs, fonts);
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
        base_text_argb: text_argb(
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
        base_text_argb: text_argb(None, ENTITY_NAME_TAG_HALF_ALPHA),
        background_argb,
        light: original_light,
    });
    let line_seed = obfuscated_seed.wrapping_add(submit_sequence);
    let normal = bake_layer_mesh(
        transform,
        &visual_runs,
        width,
        line_offset,
        &normal_layer,
        fonts,
        line_seed,
    );
    let see_through = see_through_layer.as_ref().map(|layer| {
        bake_layer_mesh(
            transform,
            &visual_runs,
            width,
            line_offset,
            layer,
            fonts,
            line_seed,
        )
    });
    let surface = EntityNameTagSurface {
        submission: EntityNameTagSubmission {
            entity_id,
            submit_sequence,
            line_kind,
            runs: runs.to_vec(),
            width,
            line_offset,
            distance_to_camera_sq,
            is_discrete,
            transform,
            normal_layer,
            see_through_layer,
        },
        normal,
        see_through,
    };
    (!surface.is_empty()).then_some(surface)
}

fn bake_layer_mesh(
    transform: Mat4,
    runs: &[WorldTextRun],
    width: u32,
    line_offset: i32,
    layer: &EntityNameTagLayerSubmission,
    fonts: &HudFontGlyphBook,
    obfuscated_seed: u64,
) -> ItemModelMesh {
    let mut mesh = ItemModelMesh::new();
    let mut pen_x = -(width as f32) / 2.0;
    let y = line_offset as f32;
    if layer.background_argb != 0 {
        append_untextured_rect(
            &mut mesh,
            transform,
            pen_x - 1.0,
            y - 1.0,
            pen_x + width as f32,
            y + ENTITY_NAME_TAG_HEIGHT,
            ENTITY_NAME_TAG_BACKGROUND_Z,
            argb_to_color(layer.background_argb),
            layer.light,
        );
    }
    let alpha = (layer.base_text_argb >> 24) as u8;
    let mut first = true;
    let mut effects = Vec::new();
    let mut obfuscated_random = HudObfuscatedRandom::with_seed(obfuscated_seed);
    for run in runs {
        let color = argb_to_color(text_argb(run.color, alpha));
        for ch in run.text.chars() {
            let base_glyph = display_text_glyph(ch, run.font.as_deref(), fonts);
            let glyph = if run.style.obfuscated && ch != ' ' {
                fonts
                    .obfuscated_pool(run.font.as_deref())
                    .random_glyph(base_glyph.advance, &mut obfuscated_random)
                    .unwrap_or(base_glyph)
            } else {
                base_glyph
            };
            if glyph.width > 0 && glyph.height > 0 {
                for quad in glyph.styled_quads(pen_x, y, run.style, false) {
                    append_quad(
                        &mut mesh,
                        transform,
                        quad.corners.map(|[x, y]| [x, y, 0.0]),
                        [
                            [quad.uv.min[0], quad.uv.min[1]],
                            [quad.uv.min[0], quad.uv.max[1]],
                            [quad.uv.max[0], quad.uv.max[1]],
                            [quad.uv.max[0], quad.uv.min[1]],
                        ],
                        color,
                        layer.light,
                    );
                }
            }
            for rect in base_glyph.styled_effect_rects(pen_x, y, run.style, first) {
                effects.push((rect, color));
            }
            pen_x += base_glyph.styled_advance_f32(run.style);
            first = false;
        }
    }
    for (rect, color) in effects {
        append_untextured_rect(
            &mut mesh,
            transform,
            rect.x0,
            rect.y0,
            rect.x1,
            rect.y1,
            ENTITY_NAME_TAG_EFFECT_Z,
            color,
            layer.light,
        );
    }
    mesh
}

#[derive(Default)]
pub(crate) struct EntityNameTagGeometry {
    pub(crate) see_through: (Vec<ItemModelVertex>, Vec<u32>),
    pub(crate) normal: (Vec<ItemModelVertex>, Vec<u32>),
}

pub(crate) fn merge_entity_name_tag_surfaces(
    surfaces: &[EntityNameTagSurface],
) -> EntityNameTagGeometry {
    let mut geometry = EntityNameTagGeometry::default();
    let mut see_through = surfaces
        .iter()
        .filter_map(|surface| {
            surface
                .see_through
                .as_ref()
                .map(|mesh| (surface.submission.distance_to_camera_sq, mesh))
        })
        .collect::<Vec<_>>();
    // Java List.sort is stable; score/name lines for one entity have equal
    // distance and therefore retain collector order.
    see_through.sort_by(|left, right| right.0.total_cmp(&left.0));
    for (_, mesh) in see_through {
        append_mesh(&mut geometry.see_through, mesh);
    }
    for surface in surfaces {
        append_mesh(&mut geometry.normal, &surface.normal);
    }
    geometry
}

impl Renderer {
    pub fn set_entity_name_tag_surfaces(&mut self, surfaces: Vec<EntityNameTagSurface>) {
        self.entity_name_tag_surfaces = if self.item_frame_map_text_font_atlas.is_some() {
            surfaces
                .into_iter()
                .filter(|surface| !surface.is_empty())
                .collect()
        } else {
            Vec::new()
        };
    }

    pub(crate) fn collect_entity_name_tag_geometry(&self) -> EntityNameTagGeometry {
        if self.item_frame_map_text_font_atlas.is_none() {
            return EntityNameTagGeometry::default();
        }
        merge_entity_name_tag_surfaces(&self.entity_name_tag_surfaces)
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
                    width: if ch == ' ' { 0 } else { 5 },
                    height: if ch == ' ' { 0 } else { 8 },
                    advance: if ch == ' ' { 4 } else { 6 },
                    ascent: 7,
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
                    advance: 6,
                    ascent: 7,
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
            .normal
            .vertices
            .chunks_exact(4)
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
        .with_is_crouching(true);
        assert_eq!(entity_name_tag_root_position(player), [1.0, 1.875, 3.0]);
        assert_eq!(
            entity_name_tag_root_position(player.with_is_crouching(false)),
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
        assert!(geometry.see_through.0[0].position[0] > 8.0);
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
                    advance: 7,
                    ascent: 7,
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
            text: "A".to_string(),
            style: HudTextStyle {
                obfuscated: true,
                ..Default::default()
            },
            color: None,
            font: Some("example:custom".to_string()),
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
            surface.normal.vertices[0].uv,
            surface.see_through.as_ref().unwrap().vertices[4].uv
        );
        assert!((0.4..=0.6).contains(&surface.normal.vertices[0].uv[0]));
    }
}
