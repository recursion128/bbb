//! Native camera/locale projection for generic entity name tags.

use bbb_item_model::NativeItemRuntime;
use bbb_protocol::ComponentStyle;
use bbb_renderer::{
    bake_entity_name_tag_surfaces, entity_name_tag_root_position, CameraPose, EntityModelInstance,
    EntityNameTagSurface, HudTextStyle, WorldTextRun,
};
use bbb_world::{
    EntityNameTagLivingVisibilityState, EntityNameTagRenderSourceState,
    EntityNameTagTextSegmentState, EntityNameTagTextState, EntityNameTagVisibilityPolicy,
    ItemFrameFacing,
};

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
            .then(|| source.below_name.as_ref())
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
    for segment in &state.segments {
        match segment {
            EntityNameTagTextSegmentState::Rich(component) => {
                for run in localized_component_runs(component, &state.inherited_style, item_runtime)
                {
                    push_run(&mut runs, hud_run(&run.text, &run.style));
                }
            }
            EntityNameTagTextSegmentState::Styled(styled) => {
                for run in styled {
                    let style = run.style.apply_to(&state.inherited_style);
                    push_run(&mut runs, hud_run(&run.text, &style));
                }
            }
            EntityNameTagTextSegmentState::Translation { key } => {
                let text = localized_or_fallback(item_runtime, key, key);
                push_run(&mut runs, hud_run(&text, &state.inherited_style));
            }
        }
    }
    runs
}

fn hud_run(text: &str, style: &ComponentStyle) -> WorldTextRun {
    WorldTextRun {
        text: text.to_string(),
        style: HudTextStyle {
            bold: style.bold == Some(true),
            italic: style.italic == Some(true),
            underlined: style.underlined == Some(true),
            strikethrough: style.strikethrough == Some(true),
            obfuscated: style.obfuscated == Some(true),
        },
        color: style.color,
        font: style.font.clone(),
    }
}

fn push_run(runs: &mut Vec<WorldTextRun>, run: WorldTextRun) {
    if run.text.is_empty() {
        return;
    }
    if let Some(previous) = runs.last_mut().filter(|previous| {
        previous.style == run.style && previous.color == run.color && previous.font == run.font
    }) {
        previous.text.push_str(&run.text);
    } else {
        runs.push(run);
    }
}

#[cfg(test)]
mod tests {
    use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue, RichTextComponent};

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
            bbb_renderer::EntityModelKind::NoRender,
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
                name: "translate".to_string(),
                value: NbtSummaryValue::String("missing.test.key".to_string()),
            },
            NbtSummaryEntry {
                name: "fallback".to_string(),
                value: NbtSummaryValue::String("Hello %s %%".to_string()),
            },
            NbtSummaryEntry {
                name: "with".to_string(),
                value: NbtSummaryValue::List(vec![NbtSummaryValue::Compound(vec![
                    NbtSummaryEntry {
                        name: "text".to_string(),
                        value: NbtSummaryValue::String("Alex".to_string()),
                    },
                    NbtSummaryEntry {
                        name: "color".to_string(),
                        value: NbtSummaryValue::String("aqua".to_string()),
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
