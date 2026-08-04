//! Vanilla block-entity renderer visibility applied before native submits the
//! shared entity-model stream.
//!
//! `LevelRenderer.extractVisibleBlockEntities` visits ordinary block entities
//! only through visible render sections, then
//! `BlockEntityRenderDispatcher.tryExtractRenderState` applies each renderer's
//! `shouldRender` distance check. That containing 16³ section is the
//! conservative native visibility unit for ordinary renderers.
//!
//! Renderers whose `shouldRenderOffScreen` is true are excluded from section
//! meshes by `SectionCompiler.handleBlockEntity` and extracted instead from
//! `Level.getGloballyRenderedBlockEntities()`, which no section or frustum gate
//! touches. `BeaconRenderer` is one, and it replaces the spherical default with
//! a horizontal-only distance against `renderDistance * 16`.

use bbb_renderer::{
    CameraFrustum, CameraPose, EndPortalModelKind, EntityModelInstance, EntityModelKind,
};
use bbb_world::{BlockPos, WorldStore};

use crate::terrain_runtime::TerrainUploadState;

/// `BlockEntityRenderer.getViewDistance()` default.
const DEFAULT_BLOCK_ENTITY_VIEW_DISTANCE: f64 = 64.0;
/// `TheEndGatewayRenderer.getViewDistance()`.
const END_GATEWAY_VIEW_DISTANCE: f64 = 256.0;
const SECTION_SIZE: i32 = 16;

#[derive(Debug, Clone, Copy)]
struct BlockEntityCameraVisibility {
    position: [f64; 3],
    frustum: CameraFrustum,
}

/// Immutable per-frame visibility inputs shared by block-entity models and
/// their secondary render features (currently sign text).
#[derive(Debug, Clone, Copy)]
pub(crate) struct BlockEntityVisibility {
    camera: Option<BlockEntityCameraVisibility>,
    /// `Minecraft.getInstance().options.getEffectiveRenderDistance() * 16`,
    /// the beacon's `getViewDistance()`.
    render_distance_blocks: f64,
}

impl BlockEntityVisibility {
    pub(crate) fn new(
        camera_pose: Option<CameraPose>,
        surface_size: [u32; 2],
        camera_fov_degrees: f32,
        render_distance_chunks: u32,
    ) -> Self {
        let camera = camera_pose.map(|pose| {
            let aspect = surface_size[0] as f32 / surface_size[1].max(1) as f32;
            BlockEntityCameraVisibility {
                position: [
                    f64::from(pose.position[0]),
                    f64::from(pose.position[1] + pose.eye_height),
                    f64::from(pose.position[2]),
                ],
                frustum: CameraFrustum::from_pose(pose, aspect, camera_fov_degrees),
            }
        });
        Self {
            camera,
            render_distance_blocks: f64::from(render_distance_chunks) * f64::from(SECTION_SIZE),
        }
    }

    /// Returns the block position when the model survives both the renderer
    /// distance check and the ordinary visible-section gate.
    pub(crate) fn visible_block_position(
        self,
        instance: &EntityModelInstance,
        world: &WorldStore,
        terrain_upload: &TerrainUploadState,
    ) -> Option<[i32; 3]> {
        let pos = instance_block_position(instance)?;
        if self.instance_is_visible(
            instance,
            terrain_upload.is_block_section_compiled_and_visible(world, pos),
        ) {
            Some([pos.x, pos.y, pos.z])
        } else {
            None
        }
    }

    pub(crate) fn is_visible(
        self,
        instance: &EntityModelInstance,
        world: &WorldStore,
        terrain_upload: &TerrainUploadState,
    ) -> bool {
        let Some(pos) = instance_block_position(instance) else {
            // Invalid presentation coordinates stay visible, matching the
            // conservative failure behavior of `CameraFrustum`.
            return true;
        };
        self.instance_is_visible(
            instance,
            terrain_upload.is_block_section_compiled_and_visible(world, pos),
        )
    }

    fn instance_is_visible(
        self,
        instance: &EntityModelInstance,
        section_compiled_and_visible: bool,
    ) -> bool {
        let Some(camera) = self.camera else {
            return true;
        };
        let Some(pos) = instance_block_position(instance) else {
            return true;
        };
        let center = [
            f64::from(pos.x) + 0.5,
            f64::from(pos.y) + 0.5,
            f64::from(pos.z) + 0.5,
        ];
        if renders_off_screen(instance) {
            // `BeaconRenderer.shouldRender`: both positions flattened to y = 0,
            // so height above or below the camera never culls the beam.
            let dx = center[0] - camera.position[0];
            let dz = center[2] - camera.position[2];
            return dx * dx + dz * dz < self.render_distance_blocks * self.render_distance_blocks;
        }
        if !section_compiled_and_visible {
            return false;
        }

        let distance = center
            .into_iter()
            .zip(camera.position)
            .map(|(coordinate, camera_coordinate)| {
                let delta = coordinate - camera_coordinate;
                delta * delta
            })
            .sum::<f64>();
        let view_distance = block_entity_view_distance(instance);
        if distance >= view_distance * view_distance {
            return false;
        }

        let section_min = [
            pos.x.div_euclid(SECTION_SIZE) * SECTION_SIZE,
            pos.y.div_euclid(SECTION_SIZE) * SECTION_SIZE,
            pos.z.div_euclid(SECTION_SIZE) * SECTION_SIZE,
        ];
        camera.frustum.intersects_aabb(
            section_min.map(f64::from),
            section_min.map(|coordinate| f64::from(coordinate + SECTION_SIZE)),
        )
    }
}

/// Stable-compacts the block-entity suffix without reallocating or changing
/// the order of surviving submissions.
pub(crate) fn retain_visible_block_entity_instances(
    instances: &mut Vec<EntityModelInstance>,
    block_entity_start: usize,
    visibility: BlockEntityVisibility,
    world: &WorldStore,
    terrain_upload: &TerrainUploadState,
) {
    debug_assert!(block_entity_start <= instances.len());
    let mut write = block_entity_start;
    for read in block_entity_start..instances.len() {
        if visibility.is_visible(&instances[read], world, terrain_upload) {
            instances.swap(write, read);
            write += 1;
        }
    }
    instances.truncate(write);
}

fn instance_block_position(instance: &EntityModelInstance) -> Option<BlockPos> {
    instance
        .position
        .into_iter()
        .all(f32::is_finite)
        .then(|| BlockPos {
            x: instance.position[0].floor() as i32,
            y: instance.position[1].floor() as i32,
            z: instance.position[2].floor() as i32,
        })
}

/// `BlockEntityRenderer.shouldRenderOffScreen`: these models are extracted from
/// the level's global collection rather than from visible render sections.
fn renders_off_screen(instance: &EntityModelInstance) -> bool {
    matches!(instance.kind, EntityModelKind::BeaconBeam)
}

fn block_entity_view_distance(instance: &EntityModelInstance) -> f64 {
    match instance.kind {
        EntityModelKind::EndPortalBlock {
            kind: EndPortalModelKind::EndGateway,
            ..
        } => END_GATEWAY_VIEW_DISTANCE,
        _ => DEFAULT_BLOCK_ENTITY_VIEW_DISTANCE,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bbb_renderer::{EndPortalModelFace, EntityModelInstance};

    fn camera_visibility() -> BlockEntityVisibility {
        BlockEntityVisibility::new(
            Some(CameraPose {
                position: [0.0; 3],
                y_rot: 0.0,
                x_rot: 0.0,
                eye_height: 0.0,
            }),
            [1_000, 1_000],
            70.0,
            8,
        )
    }

    fn ordinary(position: [f32; 3]) -> EntityModelInstance {
        EntityModelInstance::bell(-1, position)
    }

    fn gateway(position: [f32; 3]) -> EntityModelInstance {
        EntityModelInstance::new(
            -1,
            EntityModelKind::EndPortalBlock {
                kind: EndPortalModelKind::EndGateway,
                faces: [EndPortalModelFace::Down, EndPortalModelFace::Up],
            },
            position,
            0.0,
        )
    }

    #[test]
    fn default_and_end_gateway_renderers_use_vanilla_view_distances() {
        let visibility = camera_visibility();
        assert!(visibility.instance_is_visible(&ordinary([0.0, 0.0, 63.0]), true));
        assert!(!visibility.instance_is_visible(&ordinary([0.0, 0.0, 64.0]), true));
        assert!(visibility.instance_is_visible(&gateway([0.0, 0.0, 200.0]), true));
        assert!(!visibility.instance_is_visible(&gateway([0.0, 0.0, 256.0]), true));
    }

    #[test]
    fn ordinary_models_use_the_containing_section_as_the_frustum_unit() {
        let visibility = camera_visibility();
        let edge = ordinary([15.0, 0.0, 15.0]);
        let camera = visibility.camera.expect("test camera");
        assert!(!camera
            .frustum
            .intersects_aabb([15.0, 0.0, 15.0], [16.0, 1.0, 16.0]));
        assert!(visibility.instance_is_visible(&edge, true));
        assert!(!visibility.instance_is_visible(&ordinary([0.0, 0.0, -17.0]), true));
    }

    #[test]
    fn compiled_section_gate_precedes_camera_frustum_submission() {
        let visibility = camera_visibility();
        assert!(!visibility.instance_is_visible(&ordinary([0.0, 0.0, 15.0]), false));
        assert!(BlockEntityVisibility::new(None, [1, 1], 70.0, 8)
            .instance_is_visible(&ordinary([0.0, 0.0, 15.0]), false));
    }

    #[test]
    fn beacon_beams_skip_the_section_and_frustum_gates_and_cull_horizontally() {
        let visibility = camera_visibility();
        let beam = |position: [f32; 3]| EntityModelInstance::beacon_beam(-1, position);
        // Behind the camera and in an uncompiled section, but still submitted.
        assert!(visibility.instance_is_visible(&beam([0.0, 0.0, -20.0]), false));
        // Vertical distance never culls the beam.
        assert!(visibility.instance_is_visible(&beam([0.0, 4_000.0, 0.0]), false));
        // `renderDistance * 16` = 128 blocks of horizontal reach.
        assert!(visibility.instance_is_visible(&beam([0.0, 0.0, 127.0]), false));
        assert!(!visibility.instance_is_visible(&beam([0.0, 0.0, 128.0]), false));
    }
}
