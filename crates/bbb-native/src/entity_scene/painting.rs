use super::*;

const PAINTING_BACK_SPRITE_ID: &str = "minecraft:back";

/// Expands one synchronized painting into vanilla's independently lit 1x1 geometry segments.
/// `None` leaves non-paintings on normal model selection. A painting whose
/// synchronized variant or atlas is not ready resolves through its catalog
/// owner to `NoRender`, retaining the source for a later frame without drawing
/// raw fallback geometry.
pub(super) fn painting_model_instances(
    source: &EntityModelSourceState,
    world: &WorldStore,
    variant_registries: Option<&dyn EntityVariantRegistryLookup>,
) -> Option<Vec<EntityModelInstance>> {
    if source.entity_type_id != VANILLA_ENTITY_TYPE_PAINTING_ID {
        return None;
    }
    let projection = source.painting_variant_projection.as_ref()?;
    let facing = source.painting_facing?;
    if projection.width <= 0 || projection.height <= 0 {
        return None;
    }
    let registries = variant_registries?;
    let front = registries.ready_painting_texture(&projection.asset_id)?;
    let back = registries.ready_painting_texture(PAINTING_BACK_SPRITE_ID)?;
    let outline_color = if source.outline_color != 0 {
        source.outline_color
    } else if source.appears_glowing {
        ENTITY_DEFAULT_OUTLINE_COLOR
    } else {
        0
    };
    let position = [
        source.position.x as f32,
        source.position.y as f32,
        source.position.z as f32,
    ];
    let mut instances = Vec::with_capacity(
        usize::try_from(projection.width.saturating_mul(projection.height)).ok()?,
    );
    for segment_y in 0..projection.height {
        for segment_x in 0..projection.width {
            let probe = painting_light_probe_block_pos(
                source.position,
                facing,
                projection.width,
                projection.height,
                segment_x,
                segment_y,
            );
            let light_coords = world
                .sample_render_light(probe)
                .unwrap_or(bbb_render_types::VanillaLightCoords::SKY_FULL_BLOCK_DARK)
                .packed();
            instances.push(
                EntityModelInstance::painting_tile(
                    source.entity_id,
                    position,
                    projection.width,
                    projection.height,
                    segment_x,
                    segment_y,
                    painting_attachment_face(facing),
                    front,
                    back,
                )
                .with_light_coords(light_coords)
                .with_invisible(entity_invisible(&source.data_values))
                .with_outline_color(outline_color),
            );
        }
    }
    Some(instances)
}

fn painting_attachment_face(facing: WorldItemFrameFacing) -> EntityAttachmentFace {
    match facing {
        WorldItemFrameFacing::Down => EntityAttachmentFace::Down,
        WorldItemFrameFacing::Up => EntityAttachmentFace::Up,
        WorldItemFrameFacing::North => EntityAttachmentFace::North,
        WorldItemFrameFacing::South => EntityAttachmentFace::South,
        WorldItemFrameFacing::West => EntityAttachmentFace::West,
        WorldItemFrameFacing::East => EntityAttachmentFace::East,
    }
}

fn painting_light_probe_block_pos(
    position: bbb_world::EntityVec3,
    facing: WorldItemFrameFacing,
    width: i32,
    height: i32,
    segment_x: i32,
    segment_y: i32,
) -> bbb_world::BlockPos {
    let offset_x = -(width as f64) / 2.0;
    let offset_y = -(height as f64) / 2.0;
    let segment_offset_x = f64::from(segment_x) + offset_x + 0.5;
    let segment_offset_y = f64::from(segment_y) + offset_y + 0.5;
    let mut x = position.x.floor() as i32;
    let y = (position.y + segment_offset_y).floor() as i32;
    let mut z = position.z.floor() as i32;
    match facing {
        WorldItemFrameFacing::North => x = (position.x + segment_offset_x).floor() as i32,
        WorldItemFrameFacing::West => z = (position.z - segment_offset_x).floor() as i32,
        WorldItemFrameFacing::South => x = (position.x - segment_offset_x).floor() as i32,
        WorldItemFrameFacing::East => z = (position.z + segment_offset_x).floor() as i32,
        WorldItemFrameFacing::Down | WorldItemFrameFacing::Up => {}
    }
    bbb_world::BlockPos { x, y, z }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn painting_light_probe_positions_follow_vanilla_direction_switch() {
        let position = bbb_world::EntityVec3 {
            x: 10.5,
            y: 20.5,
            z: 30.5,
        };
        let north = (0..2)
            .map(|segment_x| {
                painting_light_probe_block_pos(
                    position,
                    WorldItemFrameFacing::North,
                    2,
                    1,
                    segment_x,
                    0,
                )
            })
            .collect::<Vec<_>>();
        assert_eq!(
            north,
            vec![
                bbb_world::BlockPos {
                    x: 10,
                    y: 20,
                    z: 30
                },
                bbb_world::BlockPos {
                    x: 11,
                    y: 20,
                    z: 30
                },
            ]
        );

        let west = (0..2)
            .map(|segment_x| {
                painting_light_probe_block_pos(
                    position,
                    WorldItemFrameFacing::West,
                    2,
                    1,
                    segment_x,
                    0,
                )
            })
            .collect::<Vec<_>>();
        assert_eq!(
            west,
            vec![
                bbb_world::BlockPos {
                    x: 10,
                    y: 20,
                    z: 31
                },
                bbb_world::BlockPos {
                    x: 10,
                    y: 20,
                    z: 30
                },
            ]
        );
    }
}
