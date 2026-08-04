//! World -> renderer projection for beacon beam block-entity models.
//!
//! `BeaconRenderer.submit` loops the published `BeaconBeamOwner.Section` list
//! and issues one `submitBeaconBeam` per section; bbb keeps the shared entity
//! model stream, so each section becomes its own block-sentinel instance. The
//! per-frame `beamRadiusScale` comes from `extractRenderState`, which needs the
//! camera position and the local player's scoping state.

use bbb_renderer::{BeaconBeamRenderState, CameraPose, EntityModelInstance};
use bbb_world::{BeaconBeamSourceState, WorldStore};

use crate::block_entity_visibility::{BlockEntityVisibility, OffScreenBlockEntityRenderer};

/// Block-entity model sentinel id; no vanilla network entity id is negative.
const BEACON_BLOCK_MODEL_ENTITY_ID: i32 = -1;
/// `BeaconRenderer.BEAM_SCALE_THRESHOLD`.
const BEAM_SCALE_THRESHOLD: f32 = 96.0;

pub(crate) fn beacon_beam_model_instances_from_world_at_partial_tick(
    world: &WorldStore,
    camera_pose: Option<CameraPose>,
    visibility: BlockEntityVisibility,
    partial_tick: f32,
) -> Vec<EntityModelInstance> {
    let sources = world.beacon_beam_source_states(partial_tick);
    if sources.is_empty() {
        return Vec::new();
    }
    let scoping = world.local_player_is_scoping();
    sources
        .into_iter()
        .filter(|source| {
            // `BeaconRenderer` is an off-screen renderer, so its `shouldRender`
            // replaces the section-based gate the stream applies downstream.
            visibility.off_screen_owner_is_visible(
                OffScreenBlockEntityRenderer::for_beacon_beam_owner(source.owner_is_test_instance),
                source.pos,
            )
        })
        .map(|source| beacon_beam_instance(source, camera_pose, scoping))
        .collect()
}

fn beacon_beam_instance(
    source: BeaconBeamSourceState,
    camera_pose: Option<CameraPose>,
    scoping: bool,
) -> EntityModelInstance {
    let position = [
        source.pos.x as f32,
        source.pos.y as f32,
        source.pos.z as f32,
    ];
    EntityModelInstance::beacon_beam(BEACON_BLOCK_MODEL_ENTITY_ID, position).with_beacon_beam(Some(
        BeaconBeamRenderState {
            beam_start: source.beam_start,
            height: source.height,
            color_argb: source.color_argb,
            radius_scale: beam_radius_scale(source, camera_pose, scoping),
            animation_time: source.animation_time,
        },
    ))
}

/// `player != null && player.isScoping() ? 1.0F : Math.max(1.0F,
/// distanceToBeacon / 96.0F)` over the horizontal camera distance to the
/// beacon block's centre.
fn beam_radius_scale(
    source: BeaconBeamSourceState,
    camera_pose: Option<CameraPose>,
    scoping: bool,
) -> f32 {
    if scoping {
        return 1.0;
    }
    let Some(pose) = camera_pose else {
        return 1.0;
    };
    let dx = source.pos.x as f32 + 0.5 - pose.position[0];
    let dz = source.pos.z as f32 + 0.5 - pose.position[2];
    (dx * dx + dz * dz).sqrt().max(BEAM_SCALE_THRESHOLD) / BEAM_SCALE_THRESHOLD
}

#[cfg(test)]
mod tests {
    use super::*;
    use bbb_renderer::EntityModelKind;
    use bbb_world::{BlockPos, BEACON_MAX_RENDER_Y};

    fn source(beam_start: i32, height: i32) -> BeaconBeamSourceState {
        BeaconBeamSourceState {
            pos: BlockPos { x: 4, y: 5, z: 6 },
            owner_is_test_instance: false,
            beam_start,
            height,
            color_argb: 0xFFF9_FFFE,
            animation_time: 3.5,
        }
    }

    fn pose(position: [f32; 3]) -> CameraPose {
        CameraPose {
            position,
            y_rot: 0.0,
            x_rot: 0.0,
            eye_height: 0.0,
        }
    }

    #[test]
    fn each_section_becomes_one_beam_instance_at_the_block_corner() {
        let instance = beacon_beam_instance(source(2, BEACON_MAX_RENDER_Y), None, false);
        assert_eq!(instance.kind, EntityModelKind::BeaconBeam);
        assert_eq!(instance.position, [4.0, 5.0, 6.0]);
        let beam = instance.render_state.beacon_beam.expect("beam state");
        assert_eq!(beam.beam_start, 2);
        assert_eq!(beam.height, BEACON_MAX_RENDER_Y);
        assert_eq!(beam.animation_time, 3.5);
    }

    #[test]
    fn radius_scale_grows_with_horizontal_distance_only() {
        // Straight above the beacon: horizontal distance is zero, so the beam
        // keeps its unscaled radius.
        let overhead = beacon_beam_instance(source(0, 1), Some(pose([4.5, 500.0, 6.5])), false);
        assert_eq!(
            overhead
                .render_state
                .beacon_beam
                .expect("beam")
                .radius_scale,
            1.0
        );

        let far = beacon_beam_instance(source(0, 1), Some(pose([4.5, 5.0, 198.5])), false);
        assert_eq!(
            far.render_state.beacon_beam.expect("beam").radius_scale,
            2.0
        );
    }

    #[test]
    fn scoping_pins_the_radius_scale() {
        let scoped = beacon_beam_instance(source(0, 1), Some(pose([4.5, 5.0, 198.5])), true);
        assert_eq!(
            scoped.render_state.beacon_beam.expect("beam").radius_scale,
            1.0
        );
    }
}
