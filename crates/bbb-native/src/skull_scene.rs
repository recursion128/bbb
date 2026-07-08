//! World -> renderer projection for skull/head block-entity models.
//!
//! Vanilla `SkullBlockRenderer` submits skull/head block entities with
//! `OverlayTexture.NO_OVERLAY`, block-position light, a ground or wall
//! transformation from `TRANSFORMATIONS`, and the skull animation progress
//! (`animationTickCount + partial` while powered). bbb projects those source
//! states into the shared entity-model stream as block-sentinel instances.

use bbb_renderer::{
    EntityAttachmentFace, EntityCustomHeadSkull, EntityDefaultPlayerSkin, EntityModelInstance,
    EntityPlayerSkin, SkullBlockModelAttachment,
};
use bbb_world::{
    SkullBlockAttachment as WorldSkullBlockAttachment, SkullBlockKind, SkullModelSourceState,
    SkullWallFacing, TerrainLight, WorldStore,
};

/// Block-entity model sentinel id; no vanilla network entity id is negative.
const SKULL_BLOCK_MODEL_ENTITY_ID: i32 = -1;

pub(crate) fn skull_model_instances_from_world_at_partial_tick(
    world: &WorldStore,
    partial_tick: f32,
) -> Vec<EntityModelInstance> {
    world
        .skull_model_source_states(partial_tick)
        .into_iter()
        .map(|source| skull_model_instance(&source, world))
        .collect()
}

fn skull_model_instance(source: &SkullModelSourceState, world: &WorldStore) -> EntityModelInstance {
    let (attachment, y_rot) = skull_attachment(source.attachment);
    let mut instance = EntityModelInstance::skull_block(
        SKULL_BLOCK_MODEL_ENTITY_ID,
        [
            source.pos.x as f32,
            source.pos.y as f32,
            source.pos.z as f32,
        ],
        y_rot,
        skull_kind(source.kind),
        attachment,
    )
    .with_worn_head_animation_pos(source.animation_progress);
    if let Some(light) = world.sample_block_light(source.pos) {
        instance = instance.with_light_coords(skull_light_coords(light));
    }
    instance
}

fn skull_kind(kind: SkullBlockKind) -> EntityCustomHeadSkull {
    match kind {
        SkullBlockKind::Skeleton => EntityCustomHeadSkull::Skeleton,
        SkullBlockKind::WitherSkeleton => EntityCustomHeadSkull::WitherSkeleton,
        SkullBlockKind::Player => EntityCustomHeadSkull::Player(EntityPlayerSkin::Default(
            EntityDefaultPlayerSkin::WideSteve,
        )),
        SkullBlockKind::Zombie => EntityCustomHeadSkull::Zombie,
        SkullBlockKind::Creeper => EntityCustomHeadSkull::Creeper,
        SkullBlockKind::Dragon => EntityCustomHeadSkull::Dragon,
        SkullBlockKind::Piglin => EntityCustomHeadSkull::Piglin,
    }
}

fn skull_attachment(attachment: WorldSkullBlockAttachment) -> (SkullBlockModelAttachment, f32) {
    match attachment {
        WorldSkullBlockAttachment::Ground { .. } => (
            SkullBlockModelAttachment::Ground,
            -attachment.ground_rotation_degrees(),
        ),
        WorldSkullBlockAttachment::Wall { facing } => (
            SkullBlockModelAttachment::Wall {
                facing: skull_wall_facing(facing),
            },
            0.0,
        ),
    }
}

fn skull_wall_facing(facing: SkullWallFacing) -> EntityAttachmentFace {
    match facing {
        SkullWallFacing::North => EntityAttachmentFace::North,
        SkullWallFacing::South => EntityAttachmentFace::South,
        SkullWallFacing::West => EntityAttachmentFace::West,
        SkullWallFacing::East => EntityAttachmentFace::East,
    }
}

/// Vanilla `LightCoordsUtil.pack(block, sky)`.
fn skull_light_coords(light: TerrainLight) -> u32 {
    u32::from(light.block.min(15)) << 4 | u32::from(light.sky.min(15)) << 20
}

#[cfg(test)]
mod tests {
    use super::*;
    use bbb_protocol::packets::{BlockPos as ProtocolBlockPos, BlockUpdate};
    use bbb_renderer::EntityModelKind;
    use bbb_world::{
        BlockPos, ChunkColumn, ChunkPos, ChunkSection, ChunkState, LightData, PaletteDomain,
        PaletteKind, PalettedContainerData, WorldDimension,
    };
    use std::collections::BTreeMap;

    const VANILLA_AIR_BLOCK_STATE_ID: i32 = 0;

    fn world_with_air_chunk() -> WorldStore {
        let mut world = WorldStore::with_dimension(WorldDimension {
            min_y: 0,
            height: 16,
        });
        world.insert_decoded_chunk(ChunkColumn {
            pos: ChunkPos { x: 0, z: 0 },
            state: ChunkState::Decoded,
            heightmaps: Vec::new(),
            sections: vec![ChunkSection {
                non_empty_block_count: 0,
                fluid_count: 0,
                block_states: single_value_container(
                    PaletteDomain::BlockStates,
                    4096,
                    VANILLA_AIR_BLOCK_STATE_ID,
                ),
                biomes: single_value_container(PaletteDomain::Biomes, 64, 0),
            }],
            block_entities: Vec::new(),
            light: LightData::default(),
        });
        world
    }

    fn single_value_container(
        domain: PaletteDomain,
        entry_count: usize,
        global_id: i32,
    ) -> PalettedContainerData {
        PalettedContainerData {
            domain,
            bits_per_entry: 0,
            palette_kind: PaletteKind::SingleValue,
            palette_global_ids: vec![global_id],
            packed_data: Vec::new(),
            entry_count,
        }
    }

    fn set_block(world: &mut WorldStore, pos: BlockPos, name: &str, properties: &[(&str, &str)]) {
        let properties: BTreeMap<String, String> = properties
            .iter()
            .map(|(key, value)| (key.to_string(), value.to_string()))
            .collect();
        let state_id = world
            .registries()
            .block_state_id_by_name_and_properties(name, &properties)
            .unwrap_or_else(|| panic!("no registered state for {name} {properties:?}"));
        assert!(world.apply_block_update(BlockUpdate {
            pos: ProtocolBlockPos {
                x: pos.x,
                y: pos.y,
                z: pos.z,
            },
            block_state_id: state_id,
        }));
    }

    #[test]
    fn ground_skull_projects_rotation_and_static_kind() {
        let mut world = world_with_air_chunk();
        set_block(
            &mut world,
            BlockPos { x: 2, y: 3, z: 4 },
            "minecraft:skeleton_skull",
            &[("powered", "false"), ("rotation", "4")],
        );

        let instances = skull_model_instances_from_world_at_partial_tick(&world, 0.5);

        assert_eq!(instances.len(), 1);
        assert_eq!(
            instances[0].kind,
            EntityModelKind::SkullBlock {
                skull: EntityCustomHeadSkull::Skeleton,
                attachment: SkullBlockModelAttachment::Ground,
            }
        );
        assert_eq!(instances[0].render_state.body_rot, -90.0);
        assert_eq!(instances[0].render_state.worn_head_animation_pos, 0.0);
    }

    #[test]
    fn wall_player_head_projects_default_skin_and_facing() {
        let mut world = world_with_air_chunk();
        set_block(
            &mut world,
            BlockPos { x: 2, y: 3, z: 4 },
            "minecraft:player_wall_head",
            &[("powered", "false"), ("facing", "west")],
        );

        let instances = skull_model_instances_from_world_at_partial_tick(&world, 0.5);

        assert_eq!(
            instances[0].kind,
            EntityModelKind::SkullBlock {
                skull: EntityCustomHeadSkull::Player(EntityPlayerSkin::Default(
                    EntityDefaultPlayerSkin::WideSteve,
                )),
                attachment: SkullBlockModelAttachment::Wall {
                    facing: EntityAttachmentFace::West,
                },
            }
        );
    }

    #[test]
    fn powered_dragon_head_projects_animation_progress() {
        let mut world = world_with_air_chunk();
        set_block(
            &mut world,
            BlockPos { x: 2, y: 3, z: 4 },
            "minecraft:dragon_head",
            &[("powered", "true"), ("rotation", "0")],
        );
        world.advance_skull_block_ticks(2);

        let instances = skull_model_instances_from_world_at_partial_tick(&world, 0.25);

        assert_eq!(
            instances[0].kind,
            EntityModelKind::SkullBlock {
                skull: EntityCustomHeadSkull::Dragon,
                attachment: SkullBlockModelAttachment::Ground,
            }
        );
        assert_eq!(instances[0].render_state.worn_head_animation_pos, 2.25);
    }
}
