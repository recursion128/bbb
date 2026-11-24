use super::*;

pub(super) fn block_destroy_overlays_from_stages(
    world: &WorldStore,
    textures: &TerrainTextureState,
    stages: &[(BlockPos, u8)],
    camera_pose: Option<CameraPose>,
) -> Vec<BlockDestroyOverlay> {
    let Some(camera) = camera_pose
        .map(camera_eye_position)
        .map(|camera| camera.map(f64::from))
    else {
        return Vec::new();
    };
    stages
        .iter()
        .filter(|(pos, _stage)| {
            let center = [
                f64::from(pos.x) + 0.5,
                f64::from(pos.y) + 0.5,
                f64::from(pos.z) + 0.5,
            ];
            center
                .into_iter()
                .zip(camera)
                .map(|(center, camera)| (center - camera).powi(2))
                .sum::<f64>()
                <= 1024.0
        })
        .filter_map(|(pos, stage)| {
            let uv = textures.destroy_stage_uv_rect(*stage)?;
            Some(BlockDestroyOverlay {
                pos: [pos.x, pos.y, pos.z],
                uv,
                shape: block_destroy_render_shape(world, textures, *pos),
            })
        })
        .collect()
}

pub(super) fn block_selection_render_state(
    world: &WorldStore,
    textures: &TerrainTextureState,
    selection: BlockSelectionProjection,
    surface_width: u32,
    high_contrast: bool,
) -> BlockSelectionRenderState {
    let phase = world
        .probe_block(selection.pos)
        .and_then(|probe| {
            probe.block_name.map(|block_name| {
                textures.block_model_has_translucent_material(
                    &block_name,
                    &probe.block_properties,
                    probe.material,
                )
            })
        })
        .filter(|translucent| *translucent)
        .map_or(BlockSelectionPhase::BeforeTranslucent, |_| {
            BlockSelectionPhase::AfterTranslucent
        });
    BlockSelectionRenderState {
        boxes: selection.boxes,
        phase,
        // `Window.getAppropriateLineWidth` (Window.java:543).
        line_width: crate::window_surface::appropriate_line_width(surface_width),
        high_contrast,
    }
}

/// Projects Vanilla `BlockEntityRenderState.breakProgress` onto the block-entity model instances
/// that actually receive a crumbling overlay from their renderer. The `stages` slice is the same
/// sorted, highest-stage-per-position projection used by the terrain block overlay path above.
pub(super) fn apply_block_entity_crumbling(
    instances: &mut [EntityModelInstance],
    stages: &[(BlockPos, u8)],
    textures: &TerrainTextureState,
) {
    for instance in instances {
        instance.block_entity.block_entity_crumbling = None;
        if !block_entity_model_accepts_crumbling(instance.kind) {
            continue;
        }
        let pos = BlockPos {
            x: instance.position[0].floor() as i32,
            y: instance.position[1].floor() as i32,
            z: instance.position[2].floor() as i32,
        };
        let Ok(index) = stages.binary_search_by_key(&(pos.x, pos.y, pos.z), |(pos, _stage)| {
            (pos.x, pos.y, pos.z)
        }) else {
            continue;
        };
        let Some(uv) = textures.destroy_stage_uv_rect(stages[index].1) else {
            continue;
        };
        instance.block_entity.block_entity_crumbling = Some(BlockEntityCrumbling {
            origin: [pos.x as f32, pos.y as f32, pos.z as f32],
            uv,
        });
    }
}

pub(super) fn block_entity_model_accepts_crumbling(kind: EntityModelKind) -> bool {
    matches!(
        kind,
        EntityModelKind::Chest { .. }
            | EntityModelKind::Sign { .. }
            | EntityModelKind::Bed { .. }
            | EntityModelKind::Bell
            | EntityModelKind::ShulkerBox { .. }
            | EntityModelKind::Banner { .. }
            | EntityModelKind::EnchantingBook
            | EntityModelKind::LecternBook
            | EntityModelKind::SkullBlock { .. }
            | EntityModelKind::Conduit {
                part: ConduitModelPart::Shell | ConduitModelPart::Cage
            }
    )
}

/// The render shape of the block being broken at `pos`, reused from the chunk mesher's
/// `TerrainTextureState::block_render_shape` so the crumbling decal follows the block's real
/// geometry (stairs/slab/fence/cross). Falls back to a full cube when the chunk is not loaded.
fn block_destroy_render_shape(
    world: &WorldStore,
    textures: &TerrainTextureState,
    pos: bbb_client_world::BlockPos,
) -> bbb_client_render::terrain::TerrainRenderShape {
    let Some(probe) = world.probe_block(pos) else {
        return bbb_client_render::terrain::TerrainRenderShape::Cube;
    };
    textures.block_render_shape(
        probe.block_name.as_deref(),
        &probe.block_properties,
        probe.material,
        pos,
    )
}
