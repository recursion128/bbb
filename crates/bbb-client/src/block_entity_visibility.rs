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
//! touches. Only their own `shouldRender` applies, and this module owns that
//! rule for all three: `BeaconRenderer`'s horizontal `renderDistance * 16`,
//! `BlockEntityWithBoundingBoxRenderer`'s spherical 96, and
//! `TestInstanceRenderer`'s union of the two.

use bbb_client_render::{
    CameraFrustum, CameraPose, EndPortalModelKind, EntityModelInstance, EntityModelKind,
};
use bbb_client_world::{BlockPos, WorldStore};

use crate::occlusion_culling::OpaqueTerrainOccluder;
use crate::terrain_upload::TerrainUploadState;

/// `BlockEntityRenderer.getViewDistance()` default.
pub(crate) const DEFAULT_BLOCK_ENTITY_VIEW_DISTANCE: f64 = 64.0;
/// `TheEndGatewayRenderer.getViewDistance()`.
pub(crate) const END_GATEWAY_VIEW_DISTANCE: f64 = 256.0;
/// `BlockEntityWithBoundingBoxRenderer.getViewDistance()`.
const BOUNDING_BOX_VIEW_DISTANCE: f64 = 96.0;
/// `PistonHeadRenderer.getViewDistance()`.
pub(crate) const PISTON_HEAD_VIEW_DISTANCE: f64 = 68.0;
const SECTION_SIZE: i32 = 16;

/// Which off-screen renderer owns a submission, i.e. whose `shouldRender`
/// applies. `TestInstanceRenderer` delegates to both of the others and ORs the
/// answers, so it is its own case rather than either alone.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum OffScreenBlockEntityRenderer {
    Beacon,
    BoundingBox,
    TestInstance,
}

impl OffScreenBlockEntityRenderer {
    /// The renderer a `BeaconBeamOwner` submission belongs to.
    pub(crate) fn for_beacon_beam_owner(owner_is_test_instance: bool) -> Self {
        if owner_is_test_instance {
            Self::TestInstance
        } else {
            Self::Beacon
        }
    }

    /// The renderer a `BoundingBoxRenderable` submission belongs to.
    pub(crate) fn for_bounding_box_owner(owner_is_test_instance: bool) -> Self {
        if owner_is_test_instance {
            Self::TestInstance
        } else {
            Self::BoundingBox
        }
    }
}

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

    /// `shouldRender` for the off-screen renderers whose submissions do not
    /// ride the entity-model stream. No section or frustum gate applies.
    pub(crate) fn off_screen_owner_is_visible(
        self,
        renderer: OffScreenBlockEntityRenderer,
        pos: BlockPos,
    ) -> bool {
        let Some(camera) = self.camera else {
            return true;
        };
        let center = [
            f64::from(pos.x) + 0.5,
            f64::from(pos.y) + 0.5,
            f64::from(pos.z) + 0.5,
        ];
        // `BeaconRenderer.shouldRender`: both positions flattened to y = 0, so
        // height above or below the camera never culls the beam.
        let horizontal = || {
            let dx = center[0] - camera.position[0];
            let dz = center[2] - camera.position[2];
            dx * dx + dz * dz < self.render_distance_blocks * self.render_distance_blocks
        };
        // The `BlockEntityRenderer.shouldRender` default, at the bounding-box
        // renderer's own view distance.
        let spherical = || {
            let distance_sq = center
                .into_iter()
                .zip(camera.position)
                .map(|(coordinate, camera_coordinate)| {
                    let delta = coordinate - camera_coordinate;
                    delta * delta
                })
                .sum::<f64>();
            distance_sq < BOUNDING_BOX_VIEW_DISTANCE * BOUNDING_BOX_VIEW_DISTANCE
        };
        match renderer {
            OffScreenBlockEntityRenderer::Beacon => horizontal(),
            OffScreenBlockEntityRenderer::BoundingBox => spherical(),
            OffScreenBlockEntityRenderer::TestInstance => horizontal() || spherical(),
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
        ) && !self.instance_occluded_by_terrain(instance, pos, world, terrain_upload)
        {
            Some([pos.x, pos.y, pos.z])
        } else {
            None
        }
    }

    /// `shouldRender` plus the visible-section/frustum gate for an ordinary
    /// block-entity renderer whose submission is not an entity-model instance.
    /// `PistonHeadRenderer`'s moving blocks take this path.
    pub(crate) fn block_position_is_visible(
        self,
        pos: BlockPos,
        view_distance: f64,
        world: &WorldStore,
        terrain_upload: &TerrainUploadState,
    ) -> bool {
        if !self.ordinary_position_is_visible(
            pos,
            view_distance,
            terrain_upload.is_block_section_compiled_and_visible(world, pos),
        ) {
            return false;
        }
        let Some(camera) = self.camera else {
            return true;
        };
        let occluder = OpaqueTerrainOccluder::new(world, terrain_upload, camera.position);
        let (min, max) = ordinary_position_conservative_bounds(pos);
        !occluder.is_aabb_occluded(min, max)
    }

    pub(crate) fn default_block_position_is_visible(
        self,
        pos: BlockPos,
        world: &WorldStore,
        terrain_upload: &TerrainUploadState,
    ) -> bool {
        self.block_position_is_visible(
            pos,
            DEFAULT_BLOCK_ENTITY_VIEW_DISTANCE,
            world,
            terrain_upload,
        )
    }

    /// Rejects whole source sections before scanning their block palettes.
    pub(crate) fn section_may_be_visible(
        self,
        origin: BlockPos,
        world: &WorldStore,
        terrain_upload: &TerrainUploadState,
    ) -> bool {
        self.ordinary_section_may_be_visible(
            origin,
            terrain_upload.is_block_section_compiled_and_visible(world, origin),
        )
    }

    fn ordinary_section_may_be_visible(
        self,
        origin: BlockPos,
        section_compiled_and_visible: bool,
    ) -> bool {
        let Some(camera) = self.camera else {
            return true;
        };
        if !section_compiled_and_visible {
            return false;
        }
        let min = [origin.x, origin.y, origin.z].map(f64::from);
        let max = min.map(|coordinate| coordinate + f64::from(SECTION_SIZE));
        // The nearest point is conservative for every block centre in the section.
        let distance = (0..3)
            .map(|axis| {
                let delta =
                    camera.position[axis].clamp(min[axis], max[axis]) - camera.position[axis];
                delta * delta
            })
            .sum::<f64>();
        distance < DEFAULT_BLOCK_ENTITY_VIEW_DISTANCE * DEFAULT_BLOCK_ENTITY_VIEW_DISTANCE
            && camera.frustum.intersects_aabb(min, max)
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
        ) && !self.instance_occluded_by_terrain(instance, pos, world, terrain_upload)
    }

    fn instance_occluded_by_terrain(
        self,
        instance: &EntityModelInstance,
        pos: BlockPos,
        world: &WorldStore,
        terrain_upload: &TerrainUploadState,
    ) -> bool {
        let Some(camera) = self.camera else {
            return false;
        };
        let occluder = OpaqueTerrainOccluder::new(world, terrain_upload, camera.position);
        instance_occluded_by_occluder(instance, pos, &occluder)
    }

    fn instance_is_visible(
        self,
        instance: &EntityModelInstance,
        section_compiled_and_visible: bool,
    ) -> bool {
        let Some(pos) = instance_block_position(instance) else {
            return true;
        };
        if renders_off_screen(instance) {
            // The projection already applied this renderer's `shouldRender`
            // through `off_screen_owner_is_visible`; the beam's owner is not
            // recoverable from the model kind alone.
            return true;
        }
        self.ordinary_position_is_visible(
            pos,
            block_entity_view_distance(instance),
            section_compiled_and_visible,
        )
    }

    /// The ordinary (non-off-screen) gate: compiled-and-visible section, then
    /// `shouldRender`'s spherical distance, then the containing section against
    /// the camera frustum.
    fn ordinary_position_is_visible(
        self,
        pos: BlockPos,
        view_distance: f64,
        section_compiled_and_visible: bool,
    ) -> bool {
        let Some(camera) = self.camera else {
            return true;
        };
        if !section_compiled_and_visible {
            return false;
        }
        let center = [
            f64::from(pos.x) + 0.5,
            f64::from(pos.y) + 0.5,
            f64::from(pos.z) + 0.5,
        ];
        let distance = center
            .into_iter()
            .zip(camera.position)
            .map(|(coordinate, camera_coordinate)| {
                let delta = coordinate - camera_coordinate;
                delta * delta
            })
            .sum::<f64>();
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
    let occluder = visibility
        .camera
        .map(|camera| OpaqueTerrainOccluder::new(world, terrain_upload, camera.position));
    let mut write = block_entity_start;
    for read in block_entity_start..instances.len() {
        let instance = &instances[read];
        let visible = match instance_block_position(instance) {
            None => true,
            Some(pos) => {
                visibility.instance_is_visible(
                    instance,
                    terrain_upload.is_block_section_compiled_and_visible(world, pos),
                ) && !occluder
                    .as_ref()
                    .is_some_and(|occ| instance_occluded_by_occluder(instance, pos, occ))
            }
        };
        if visible {
            instances.swap(write, read);
            write += 1;
        }
    }
    instances.truncate(write);
}

fn instance_occluded_by_occluder(
    instance: &EntityModelInstance,
    pos: BlockPos,
    occluder: &OpaqueTerrainOccluder<'_>,
) -> bool {
    if renders_off_screen(instance)
        || matches!(
            instance.kind,
            EntityModelKind::EndPortalBlock {
                kind: EndPortalModelKind::EndGateway,
                ..
            }
        )
    {
        return false;
    }
    let (min, max) = instance_conservative_bounds(instance, pos);
    occluder.is_aabb_occluded(min, max)
}

fn instance_conservative_bounds(
    instance: &EntityModelInstance,
    pos: BlockPos,
) -> ([f64; 3], [f64; 3]) {
    let x = f64::from(pos.x);
    let y = f64::from(pos.y);
    let z = f64::from(pos.z);
    match instance.kind {
        EntityModelKind::Chest { .. } => ([x - 1.0, y, z - 1.0], [x + 2.0, y + 1.25, z + 2.0]),
        EntityModelKind::Bed { .. } => ([x - 1.0, y, z - 1.0], [x + 2.0, y + 1.0, z + 2.0]),
        EntityModelKind::Banner { .. } => ([x, y - 1.25, z], [x + 1.0, y + 2.25, z + 1.0]),
        EntityModelKind::EnchantingBook => ([x, y, z], [x + 1.0, y + 1.75, z + 1.0]),
        EntityModelKind::ShulkerBox { .. } => {
            ([x - 0.5, y - 0.5, z - 0.5], [x + 1.5, y + 1.5, z + 1.5])
        }
        _ => ([x, y, z], [x + 1.0, y + 1.25, z + 1.0]),
    }
}

fn ordinary_position_conservative_bounds(pos: BlockPos) -> ([f64; 3], [f64; 3]) {
    let x = f64::from(pos.x);
    let y = f64::from(pos.y);
    let z = f64::from(pos.z);
    ([x - 1.0, y - 1.25, z - 1.0], [x + 2.0, y + 2.25, z + 2.0])
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
    use bbb_client_render::{EndPortalModelFaces, EntityModelInstance};

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
                faces: EndPortalModelFaces::ALL,
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
    fn source_section_gate_keeps_the_near_corner_inside_the_render_distance() {
        let visibility = camera_visibility();
        let origin = BlockPos { x: 32, y: 0, z: 48 };
        assert!(visibility.ordinary_section_may_be_visible(origin, true));
        assert!(visibility.ordinary_position_is_visible(
            origin,
            DEFAULT_BLOCK_ENTITY_VIEW_DISTANCE,
            true,
        ));
        assert!(!visibility.ordinary_position_is_visible(
            BlockPos { x: 47, y: 0, z: 63 },
            DEFAULT_BLOCK_ENTITY_VIEW_DISTANCE,
            true,
        ));
        assert!(!visibility.ordinary_section_may_be_visible(BlockPos { x: 0, y: 0, z: 64 }, true,));
        assert!(!visibility.ordinary_section_may_be_visible(origin, false));
    }

    #[test]
    fn beacon_beams_skip_the_section_and_frustum_gates() {
        let visibility = camera_visibility();
        let beam = |position: [f32; 3]| EntityModelInstance::beacon_beam(-1, position);
        // Behind the camera and in an uncompiled section, but still submitted;
        // the projection owns the distance check for off-screen renderers.
        assert!(visibility.instance_is_visible(&beam([0.0, 0.0, -20.0]), false));
        assert!(visibility.instance_is_visible(&beam([0.0, 4_000.0, 0.0]), false));
    }

    #[test]
    fn the_piston_head_renderer_uses_its_own_sixty_eight_block_view_distance() {
        let visibility = camera_visibility();
        let visible = |z: i32, compiled| {
            visibility.ordinary_position_is_visible(
                BlockPos { x: 0, y: 0, z },
                PISTON_HEAD_VIEW_DISTANCE,
                compiled,
            )
        };
        assert!(visible(67, true));
        assert!(!visible(68, true));
        // The visible-section gate still precedes the distance check.
        assert!(!visible(1, false));
    }

    #[test]
    fn off_screen_renderers_use_their_own_should_render_rules() {
        let visibility = camera_visibility();
        let pos = |x: i32, y: i32, z: i32| BlockPos { x, y, z };
        let visible = |renderer, pos| visibility.off_screen_owner_is_visible(renderer, pos);

        // `renderDistance * 16` = 128 blocks of horizontal reach, and height
        // never enters the beacon's check.
        assert!(visible(
            OffScreenBlockEntityRenderer::Beacon,
            pos(0, 4_000, 0)
        ));
        assert!(visible(
            OffScreenBlockEntityRenderer::Beacon,
            pos(0, 0, 126)
        ));
        assert!(!visible(
            OffScreenBlockEntityRenderer::Beacon,
            pos(0, 0, 128)
        ));

        // The bounding box uses the spherical default at 96.
        assert!(visible(
            OffScreenBlockEntityRenderer::BoundingBox,
            pos(0, 0, 94)
        ));
        assert!(!visible(
            OffScreenBlockEntityRenderer::BoundingBox,
            pos(0, 0, 96)
        ));
        assert!(!visible(
            OffScreenBlockEntityRenderer::BoundingBox,
            pos(0, 4_000, 0)
        ));

        // `TestInstanceRenderer.shouldRender` ORs both, so it survives wherever
        // either does.
        assert!(visible(
            OffScreenBlockEntityRenderer::TestInstance,
            pos(0, 4_000, 0)
        ));
        assert!(visible(
            OffScreenBlockEntityRenderer::TestInstance,
            pos(0, 0, 126)
        ));
        assert!(!visible(
            OffScreenBlockEntityRenderer::TestInstance,
            pos(0, 0, 128)
        ));
    }

    #[test]
    fn a_short_render_distance_leaves_the_bounding_box_reach_intact() {
        // Two chunks of render distance is below the bounding box's own 96, so
        // only the union keeps a distant test instance alive.
        let visibility = BlockEntityVisibility::new(
            Some(CameraPose {
                position: [0.0; 3],
                y_rot: 0.0,
                x_rot: 0.0,
                eye_height: 0.0,
            }),
            [1_000, 1_000],
            70.0,
            2,
        );
        let pos = BlockPos { x: 0, y: 0, z: 50 };
        assert!(!visibility.off_screen_owner_is_visible(OffScreenBlockEntityRenderer::Beacon, pos));
        assert!(
            visibility.off_screen_owner_is_visible(OffScreenBlockEntityRenderer::BoundingBox, pos)
        );
        assert!(
            visibility.off_screen_owner_is_visible(OffScreenBlockEntityRenderer::TestInstance, pos)
        );
    }

    #[test]
    fn ordinary_block_entities_behind_solid_wall_cull_while_beacon_beams_stay_visible() {
        use crate::occlusion_culling::tests::{stone_state_id, test_world_with_blocks};
        let stone = stone_state_id();
        let mut blocks = Vec::new();
        for y in 63..=67 {
            for x in -2..=2 {
                blocks.push((x, y, 5, stone));
            }
        }
        let world = test_world_with_blocks(&blocks);
        let mut upload = TerrainUploadState::default();
        upload.mark_section_occluder_ready_for_test(&world, 0, 3, 0, 1.0);
        upload.mark_section_occluder_ready_for_test(&world, 0, 4, 0, 1.0);
        upload.mark_section_occluder_ready_for_test(&world, -1, 3, 0, 1.0);
        upload.mark_section_occluder_ready_for_test(&world, -1, 4, 0, 1.0);

        let visibility = BlockEntityVisibility::new(
            Some(CameraPose {
                position: [0.5, 65.5, 1.0],
                y_rot: 0.0,
                x_rot: 0.0,
                eye_height: 0.0,
            }),
            [1_000, 1_000],
            70.0,
            8,
        );
        let behind_bell = ordinary([0.5, 65.0, 8.0]);
        let front_bell = ordinary([0.5, 65.0, 3.0]);
        let behind_beam = EntityModelInstance::beacon_beam(-1, [0.5, 65.0, 8.0]);

        assert!(!visibility.is_visible(&behind_bell, &world, &upload));
        assert!(visibility.is_visible(&front_bell, &world, &upload));
        assert!(visibility.is_visible(&behind_beam, &world, &upload));
    }
}
