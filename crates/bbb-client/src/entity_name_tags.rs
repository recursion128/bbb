//! Native camera/locale projection for generic entity name tags.

use bbb_client_render::item_model::NativeItemRuntime;
use bbb_client_render::{
    bake_entity_name_tag_surfaces, entity_name_tag_root_position, CameraPose, EntityModelInstance,
    EntityNameTagSurface, WorldTextRun,
};
use bbb_client_world::{
    EntityNameTagLivingVisibilityState, EntityNameTagRenderSourceState,
    EntityNameTagTextSegmentState, EntityNameTagTextState, EntityNameTagVisibilityPolicy,
    ItemFrameFacing,
};
use bbb_protocol::StyledTextRun;

use crate::component_text::{localized_component_runs, localized_or_fallback};

const NAME_TAG_MAX_DISTANCE_SQ: f64 = 64.0 * 64.0;
const DISCRETE_NAME_TAG_MAX_DISTANCE_SQ: f64 = 32.0 * 32.0;
const BELOW_NAME_MAX_DISTANCE_SQ: f64 = 10.0 * 10.0;

#[allow(clippy::too_many_arguments)]
pub(crate) fn entity_name_tag_surfaces(
    sources: &[EntityNameTagRenderSourceState],
    instances: &[EntityModelInstance],
    item_runtime: Option<&NativeItemRuntime>,
    camera_pose: Option<CameraPose>,
    targeted_entity_id: Option<i32>,
    camera_entity_id: Option<i32>,
    render_names: bool,
    obfuscated_seed: u64,
) -> Vec<EntityNameTagSurface> {
    // Name tags bake first in a frame's text extraction, before text displays.
    bbb_client_render::next_text_layout_frame();
    let Some((camera_pose, fonts)) =
        camera_pose.zip(item_runtime.and_then(NativeItemRuntime::map_text_fonts))
    else {
        return Vec::new();
    };
    let eye = [
        f64::from(camera_pose.position[0]),
        f64::from(camera_pose.position[1] + camera_pose.eye_height),
        f64::from(camera_pose.position[2]),
    ];
    let mut surfaces = Vec::new();
    let mut submit_sequence = 0_u64;
    for source in sources {
        let dx = source.distance_position.x - eye[0];
        let dy = source.distance_position.y - eye[1];
        let dz = source.distance_position.z - eye[2];
        let distance_to_camera_sq = dx * dx + dy * dy + dz * dz;
        if !entity_name_tag_is_visible(
            source.visibility,
            source.entity_id,
            distance_to_camera_sq,
            targeted_entity_id,
            camera_entity_id,
            render_names,
        ) {
            continue;
        }
        // Name-visible entities are sparse in ordinary scenes, so resolve only
        // after every visibility gate instead of allocating a second per-frame
        // entity index for all model instances.
        let Some(mut instance) = instances
            .iter()
            .find(|instance| instance.entity_id == source.entity_id)
            .copied()
        else {
            continue;
        };
        // Painting model extraction expands one entity into per-tile instances;
        // generic name display stays at the dispatcher's entity root.
        instance.position = [
            source.position.x as f32,
            source.position.y as f32,
            source.position.z as f32,
        ];
        let name_runs = entity_name_tag_text_runs(&source.name, item_runtime);
        let below_name_runs = below_name_is_within_render_distance(distance_to_camera_sq)
            .then_some(source.below_name.as_ref())
            .flatten()
            .map(|line| entity_name_tag_text_runs(line, item_runtime));
        let baked = bake_entity_name_tag_surfaces(
            source.entity_id,
            submit_sequence,
            entity_name_tag_root_position_for_policy(instance, source.visibility),
            source.attachment_y,
            source.line_offset,
            distance_to_camera_sq as f32,
            source.is_discrete,
            &name_runs,
            below_name_runs.as_deref(),
            instance.render_state.light_coords,
            camera_pose,
            fonts,
            obfuscated_seed,
        );
        submit_sequence = submit_sequence.saturating_add(baked.len() as u64);
        surfaces.extend(baked);
    }
    surfaces
}

fn below_name_is_within_render_distance(distance_to_camera_sq: f64) -> bool {
    distance_to_camera_sq < BELOW_NAME_MAX_DISTANCE_SQ
}

fn entity_name_tag_is_visible(
    policy: EntityNameTagVisibilityPolicy,
    entity_id: i32,
    distance_to_camera_sq: f64,
    targeted_entity_id: Option<i32>,
    camera_entity_id: Option<i32>,
    render_names: bool,
) -> bool {
    if distance_to_camera_sq >= NAME_TAG_MAX_DISTANCE_SQ {
        return false;
    }
    let targeted = targeted_entity_id == Some(entity_id);
    match policy {
        EntityNameTagVisibilityPolicy::Entity {
            entity_should_show_name,
            has_custom_name,
        } => entity_should_show_name || has_custom_name && targeted,
        EntityNameTagVisibilityPolicy::ArmorStand {
            custom_name_visible,
        } => custom_name_visible,
        EntityNameTagVisibilityPolicy::ItemFrame { .. } => render_names && targeted,
        EntityNameTagVisibilityPolicy::MobOrAvatar {
            living,
            entity_should_show_name,
            has_custom_name,
            is_discrete,
        } => {
            if !(entity_should_show_name || has_custom_name && targeted)
                || is_discrete && distance_to_camera_sq >= DISCRETE_NAME_TAG_MAX_DISTANCE_SQ
            {
                return false;
            }
            match living {
                EntityNameTagLivingVisibilityState::TeamOverride { visible } => visible,
                EntityNameTagLivingVisibilityState::General {
                    visible_to_local_player,
                    is_vehicle,
                } => {
                    render_names
                        && camera_entity_id != Some(entity_id)
                        && visible_to_local_player
                        && !is_vehicle
                }
            }
        }
    }
}

fn entity_name_tag_root_position_for_policy(
    instance: EntityModelInstance,
    policy: EntityNameTagVisibilityPolicy,
) -> [f32; 3] {
    let mut root = entity_name_tag_root_position(instance);
    let EntityNameTagVisibilityPolicy::ItemFrame { facing } = policy else {
        return root;
    };
    let [x, z] = match facing {
        ItemFrameFacing::Down | ItemFrameFacing::Up => [0.0, 0.0],
        ItemFrameFacing::North => [0.0, -0.3],
        ItemFrameFacing::South => [0.0, 0.3],
        ItemFrameFacing::West => [-0.3, 0.0],
        ItemFrameFacing::East => [0.3, 0.0],
    };
    root[0] += x;
    root[1] -= 0.25;
    root[2] += z;
    root
}

fn entity_name_tag_text_runs(
    state: &EntityNameTagTextState,
    item_runtime: Option<&NativeItemRuntime>,
) -> Vec<WorldTextRun> {
    let mut runs = Vec::new();
    let mut events = crate::component_text::WorldTextEventIds::default();
    for segment in &state.segments {
        // `EntityRenderer.submitNameDisplay` submits the name Component to the
        // name-tag pass, whose glyphs come from the visual order
        // so each part resolves here with its
        // own inherited style as the `§r` reset.
        let source = match segment {
            EntityNameTagTextSegmentState::Rich(component) => {
                localized_component_runs(component, &state.inherited_style, item_runtime)
            }
            EntityNameTagTextSegmentState::Styled(styled) => styled
                .iter()
                .map(|run| StyledTextRun {
                    text: run.text.clone(),
                    style: run.style.apply_to(&state.inherited_style),
                })
                .collect(),
            EntityNameTagTextSegmentState::Translation { key } => vec![StyledTextRun {
                text: localized_or_fallback(item_runtime, key, key),
                style: state.inherited_style.clone(),
            }],
        };
        crate::component_text::push_world_text_runs(&mut runs, &mut events, &source);
    }
    runs
}

#[cfg(test)]
mod tests {
    #[test]
    fn entity_name_tag_component_shadow_keeps_distinct_adjacent_runs() {
        use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue, RichTextComponent};
        let entry = |name: &str, value| NbtSummaryEntry {
            name: name.into(),
            value,
        };
        let component = RichTextComponent::try_from_component_nbt(NbtSummaryValue::Compound(vec![
            entry("text", NbtSummaryValue::String("A".into())),
            entry("shadow_color", NbtSummaryValue::Int(0x8012_3456_u32 as i32)),
            entry(
                "extra",
                NbtSummaryValue::List(vec![
                    NbtSummaryValue::Compound(vec![
                        entry("text", NbtSummaryValue::String("B".into())),
                        entry("shadow_color", NbtSummaryValue::Int(0)),
                    ]),
                    NbtSummaryValue::String("C".into()),
                ]),
            ),
        ]))
        .unwrap();
        let runs = crate::component_text::world_text_runs(component.styled());
        assert_eq!(
            runs.iter().map(|run| run.shadow_color).collect::<Vec<_>>(),
            [Some(0x8012_3456), Some(0), Some(0x8012_3456)]
        );
    }

    use bbb_protocol::{ComponentStyle, NbtSummaryEntry, NbtSummaryValue, RichTextComponent};

    use super::*;

    fn general_policy() -> EntityNameTagVisibilityPolicy {
        EntityNameTagVisibilityPolicy::MobOrAvatar {
            living: EntityNameTagLivingVisibilityState::General {
                visible_to_local_player: true,
                is_vehicle: false,
            },
            entity_should_show_name: true,
            has_custom_name: false,
            is_discrete: false,
        }
    }

    #[test]
    fn entity_name_tag_consumes_legacy_codes_before_width_and_baked_glyphs() {
        use bbb_client_render::{HudAsciiGlyph, HudFontGlyphBook, HudFontGlyphMap};
        use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue};
        use std::collections::BTreeMap;

        let component = RichTextComponent::try_from_component_nbt(NbtSummaryValue::Compound(vec![
            NbtSummaryEntry {
                name: "text".into(),
                value: NbtSummaryValue::String("ab§ccd§ref§?gh§".into()),
            },
        ]))
        .unwrap();
        let state = EntityNameTagTextState {
            segments: vec![EntityNameTagTextSegmentState::Rich(component)],
            inherited_style: ComponentStyle::default(),
        };
        let runs = entity_name_tag_text_runs(&state, None);
        assert_eq!(
            runs.iter().map(|run| run.text.as_str()).collect::<Vec<_>>(),
            ["ab", "cd", "efgh"]
        );
        assert_eq!(runs[1].color, Some(0xff_55_55));
        assert_eq!(runs[0].color, None);
        assert_eq!(runs[2].color, None);

        // The centre offset, the logical width and the baked quads all read
        // those runs: `EntityRenderer.renderNameTag` centres on the Component
        // width, and an unconsumed `§` would resolve to the `?` missing glyph.
        let mut glyphs = HudFontGlyphMap::new();
        let glyph = |advance: f32, min_x: f32| HudAsciiGlyph {
            uv: bbb_client_render::HudUvRect {
                min: [min_x, 0.0],
                max: [min_x + 0.05, 1.0],
            },
            advance,
            width: 5,
            height: 8,
            ..HudAsciiGlyph::default()
        };
        for (ch, min_x) in [('a', 0.1), ('b', 0.2), ('c', 0.3), ('d', 0.4), ('e', 0.5)] {
            glyphs.insert_first_wins(ch, glyph(6.0, min_x));
        }
        glyphs.insert_first_wins('f', glyph(6.0, 0.6));
        glyphs.insert_first_wins('g', glyph(6.0, 0.7));
        glyphs.insert_first_wins('h', glyph(6.0, 0.8));
        glyphs.insert_first_wins('?', glyph(6.0, 0.9));
        let fonts = HudFontGlyphBook::from_fonts(
            "minecraft:default",
            BTreeMap::from([("minecraft:default".to_string(), glyphs)]),
        );
        let surfaces = bbb_client_render::bake_entity_name_tag_surfaces(
            7,
            0,
            [0.0; 3],
            1.0,
            0,
            4.0,
            false,
            &runs,
            None,
            15 << 20,
            bbb_client_render::CameraPose {
                position: [0.0; 3],
                y_rot: 0.0,
                x_rot: 0.0,
                eye_height: 0.0,
            },
            &fonts,
            0,
        );
        let surface = &surfaces[0];
        assert_eq!(surface.submission.width, 48);
        assert_eq!(
            surface
                .submission
                .runs
                .iter()
                .map(|run| run.text.as_str())
                .collect::<String>(),
            "abcdefgh"
        );
        // Eight drawn glyphs; the two codes and the trailing `§` bake nothing.
        assert_eq!(surface.normal_vertex_count(), 8 * 4);
    }

    #[test]
    fn entity_name_visibility_applies_vanilla_distance_camera_and_team_order() {
        assert!(entity_name_tag_is_visible(
            general_policy(),
            7,
            4095.0,
            None,
            None,
            true
        ));
        assert!(!entity_name_tag_is_visible(
            general_policy(),
            7,
            4096.0,
            None,
            None,
            true
        ));
        assert!(!entity_name_tag_is_visible(
            general_policy(),
            7,
            1.0,
            None,
            Some(7),
            true
        ));
        let team_override = EntityNameTagVisibilityPolicy::MobOrAvatar {
            living: EntityNameTagLivingVisibilityState::TeamOverride { visible: true },
            entity_should_show_name: true,
            has_custom_name: false,
            is_discrete: false,
        };
        assert!(entity_name_tag_is_visible(
            team_override,
            7,
            1.0,
            None,
            Some(7),
            false
        ));
    }

    #[test]
    fn entity_name_visibility_applies_crosshair_and_discrete_gates() {
        let base = EntityNameTagVisibilityPolicy::Entity {
            entity_should_show_name: false,
            has_custom_name: true,
        };
        assert!(!entity_name_tag_is_visible(base, 4, 1.0, None, None, true));
        assert!(entity_name_tag_is_visible(
            base,
            4,
            1.0,
            Some(4),
            None,
            true
        ));
        let discrete = EntityNameTagVisibilityPolicy::MobOrAvatar {
            living: EntityNameTagLivingVisibilityState::General {
                visible_to_local_player: true,
                is_vehicle: false,
            },
            entity_should_show_name: true,
            has_custom_name: false,
            is_discrete: true,
        };
        assert!(entity_name_tag_is_visible(
            discrete, 4, 1023.0, None, None, true
        ));
        assert!(!entity_name_tag_is_visible(
            discrete, 4, 1024.0, None, None, true
        ));
        assert!(below_name_is_within_render_distance(99.999));
        assert!(!below_name_is_within_render_distance(100.0));
    }

    #[test]
    fn item_frame_name_visibility_and_root_offset_follow_renderer_override() {
        let policy = EntityNameTagVisibilityPolicy::ItemFrame {
            facing: ItemFrameFacing::North,
        };
        assert!(!entity_name_tag_is_visible(
            policy,
            12,
            1.0,
            Some(12),
            None,
            false
        ));
        assert!(!entity_name_tag_is_visible(
            policy, 12, 1.0, None, None, true
        ));
        assert!(entity_name_tag_is_visible(
            policy,
            12,
            4095.0,
            Some(12),
            None,
            true
        ));
        assert!(!entity_name_tag_is_visible(
            policy,
            12,
            4096.0,
            Some(12),
            None,
            true
        ));

        let instance = EntityModelInstance::new(
            12,
            bbb_client_render::EntityModelKind::NoRender,
            [4.0, 5.0, 6.0],
            0.0,
        );
        assert_eq!(
            entity_name_tag_root_position_for_policy(instance, policy),
            [4.0, 4.75, 5.7]
        );
        assert_eq!(
            entity_name_tag_root_position_for_policy(
                instance,
                EntityNameTagVisibilityPolicy::ItemFrame {
                    facing: ItemFrameFacing::East,
                },
            ),
            [4.3, 4.75, 6.0]
        );
    }

    #[test]
    fn localized_component_fallback_formats_arguments_and_inherited_style() {
        let component = RichTextComponent::try_from_component_nbt(NbtSummaryValue::Compound(vec![
            NbtSummaryEntry {
                name: "translate".into(),
                value: NbtSummaryValue::string("missing.test.key"),
            },
            NbtSummaryEntry {
                name: "fallback".into(),
                value: NbtSummaryValue::string("Hello %s %%"),
            },
            NbtSummaryEntry {
                name: "with".into(),
                value: NbtSummaryValue::List(vec![NbtSummaryValue::Compound(vec![
                    NbtSummaryEntry {
                        name: "text".into(),
                        value: NbtSummaryValue::string("Alex"),
                    },
                    NbtSummaryEntry {
                        name: "color".into(),
                        value: NbtSummaryValue::string("aqua"),
                    },
                ])]),
            },
        ]))
        .unwrap();
        let state = EntityNameTagTextState {
            segments: vec![EntityNameTagTextSegmentState::Rich(component)],
            inherited_style: ComponentStyle {
                bold: Some(true),
                font: Some("example:names".to_string()),
                ..ComponentStyle::default()
            },
        };
        let runs = entity_name_tag_text_runs(&state, None);
        assert_eq!(
            runs.iter().map(|run| run.text.as_str()).collect::<String>(),
            "Hello Alex %"
        );
        assert!(runs.iter().all(|run| run.style.bold));
        assert!(runs
            .iter()
            .all(|run| run.font.as_deref() == Some("example:names")));
        assert_eq!(runs[1].color, Some(0x55_FF_FF));
    }
}
