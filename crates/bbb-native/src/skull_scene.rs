//! World -> renderer projection for skull/head block-entity models.
//!
//! Vanilla `SkullBlockRenderer` submits skull/head block entities with
//! `OverlayTexture.NO_OVERLAY`, block-position light, a ground or wall
//! transformation from `TRANSFORMATIONS`, and the skull animation progress
//! (`animationTickCount + partial` while powered). bbb projects those source
//! states into the shared entity-model stream as block-sentinel instances.

use bbb_item_model::{default_player_skin_for_profile, NativeItemRuntime};
use bbb_renderer::{
    EntityAttachmentFace, EntityCustomHeadSkull, EntityDefaultPlayerSkin, EntityModelInstance,
    EntityPlayerSkin, SkullBlockModelAttachment,
};
use bbb_world::{
    SkullBlockAttachment as WorldSkullBlockAttachment, SkullBlockKind, SkullModelSourceState,
    SkullWallFacing, WorldStore,
};

/// Block-entity model sentinel id; no vanilla network entity id is negative.
const SKULL_BLOCK_MODEL_ENTITY_ID: i32 = -1;

pub(crate) fn skull_model_instances_from_world_at_partial_tick(
    world: &WorldStore,
    partial_tick: f32,
    item_runtime: Option<&NativeItemRuntime>,
) -> Vec<EntityModelInstance> {
    world
        .skull_model_source_states(partial_tick)
        .into_iter()
        .map(|source| skull_model_instance(&source, world, item_runtime))
        .collect()
}

fn skull_model_instance(
    source: &SkullModelSourceState,
    world: &WorldStore,
    item_runtime: Option<&NativeItemRuntime>,
) -> EntityModelInstance {
    let (attachment, y_rot) = skull_attachment(source.attachment);
    let mut instance = EntityModelInstance::skull_block(
        SKULL_BLOCK_MODEL_ENTITY_ID,
        [
            source.pos.x as f32,
            source.pos.y as f32,
            source.pos.z as f32,
        ],
        y_rot,
        skull_kind(source, item_runtime),
        attachment,
    )
    .with_worn_head_animation_pos(source.animation_progress);
    if let Some(light) = world.sample_render_light(source.pos) {
        instance = instance.with_light_coords(light.packed());
    }
    instance
}

fn skull_kind(
    source: &SkullModelSourceState,
    item_runtime: Option<&NativeItemRuntime>,
) -> EntityCustomHeadSkull {
    match source.kind {
        SkullBlockKind::Skeleton => EntityCustomHeadSkull::Skeleton,
        SkullBlockKind::WitherSkeleton => EntityCustomHeadSkull::WitherSkeleton,
        SkullBlockKind::Player => {
            let skin = source.owner_profile.as_ref().map_or(
                EntityPlayerSkin::Default(EntityDefaultPlayerSkin::SlimSteve),
                |profile| {
                    item_runtime.map_or_else(
                        || {
                            EntityPlayerSkin::ProfiledDefault(default_player_skin_for_profile(
                                profile,
                            ))
                        },
                        |item_runtime| item_runtime.player_skin_for_profile(profile),
                    )
                },
            );
            EntityCustomHeadSkull::Player(skin)
        }
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

#[cfg(test)]
mod tests {
    use super::*;
    use bbb_protocol::block_entity_types::VANILLA_BLOCK_ENTITY_TYPE_SKULL_ID;
    use bbb_protocol::packets::inventory::{
        PlayerModelTypeSummary, PlayerSkinPatchSummary, ProfileSkinTextureSummary,
        ProfileTexturesSummary, ResolvableProfileKindSummary, ResolvableProfileSummary,
    };
    use bbb_protocol::packets::{BlockPos as ProtocolBlockPos, BlockUpdate};
    use bbb_renderer::{
        EntityDynamicPlayerSkin, EntityDynamicPlayerSkinStatus, EntityModelKind,
        EntityPlayerSkinModel,
    };
    use bbb_world::{
        BlockEntityRecord, BlockPos, ChunkColumn, ChunkPos, ChunkSection, ChunkState, LightData,
        PaletteDomain, PaletteKind, PalettedContainerData, WorldDimension,
    };
    use std::collections::BTreeMap;

    fn world_with_air_chunk() -> WorldStore {
        world_with_block_entities(Vec::new())
    }

    fn world_with_block_entities(block_entities: Vec<BlockEntityRecord>) -> WorldStore {
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
                    bbb_world::vanilla_air_block_state_id(),
                ),
                biomes: single_value_container(PaletteDomain::Biomes, 64, 0),
            }],
            block_entities,
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

        let instances = skull_model_instances_from_world_at_partial_tick(&world, 0.5, None);

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

        let instances = skull_model_instances_from_world_at_partial_tick(&world, 0.5, None);

        assert_eq!(
            instances[0].kind,
            EntityModelKind::SkullBlock {
                skull: EntityCustomHeadSkull::Player(EntityPlayerSkin::Default(
                    EntityDefaultPlayerSkin::SlimSteve,
                )),
                attachment: SkullBlockModelAttachment::Wall {
                    facing: EntityAttachmentFace::West,
                },
            }
        );
    }

    #[test]
    fn profiled_player_head_projects_owner_fallback_and_ready_dynamic_skin() {
        let pos = BlockPos { x: 2, y: 3, z: 4 };
        let skin_url = "https://textures.minecraft.net/texture/block-head";
        let profile = ResolvableProfileSummary {
            kind: ResolvableProfileKindSummary::GameProfile,
            uuid: Some(uuid::Uuid::from_u128(0)),
            name: Some("Alex".to_string()),
            properties: vec![bbb_protocol::packets::GameProfilePropertySummary {
                name: "textures".to_string(),
                value: "packed-block-head-textures".to_string(),
                signature: None,
            }],
            profile_textures: Some(ProfileTexturesSummary {
                skin: Some(ProfileSkinTextureSummary {
                    url: skin_url.to_string(),
                    model: PlayerModelTypeSummary::Wide,
                }),
                cape: None,
                elytra: None,
            }),
            skin_patch: PlayerSkinPatchSummary::default(),
        };
        let mut world = world_with_block_entities(vec![BlockEntityRecord {
            local_x: pos.x as u8,
            y: pos.y as i16,
            local_z: pos.z as u8,
            type_id: VANILLA_BLOCK_ENTITY_TYPE_SKULL_ID,
            raw_nbt: vec![10, 0],
            nbt: None,
            sign_text: None,
            vault_shared_data: None,
            campfire: None,
            shelf: None,
            brushable: None,
            conduit_destroy_target: None,
            decorated_pot_sherds: None,
            banner_patterns: None,
            end_gateway: None,
            spawner: None,
            trial_spawner: None,
            command_block: None,
            comparator_output: None,
            structure_block: None,
            jigsaw_block: None,
            test_block: None,
            test_instance_block: None,
            note_block_sound: None,
            skull_profile: Some(profile),
        }]);
        set_block(
            &mut world,
            pos,
            "minecraft:player_head",
            &[("powered", "false"), ("rotation", "0")],
        );

        let fallback = skull_model_instances_from_world_at_partial_tick(&world, 0.5, None);
        assert_eq!(
            fallback[0].kind,
            EntityModelKind::SkullBlock {
                skull: EntityCustomHeadSkull::Player(EntityPlayerSkin::ProfiledDefault(
                    EntityDefaultPlayerSkin::SlimAlex,
                )),
                attachment: SkullBlockModelAttachment::Ground,
            }
        );

        let item_runtime = NativeItemRuntime::empty_for_test();
        let pending =
            skull_model_instances_from_world_at_partial_tick(&world, 0.5, Some(&item_runtime));
        assert_eq!(pending[0].kind, fallback[0].kind);
        item_runtime.mark_profile_skin_resolved(skin_url, 12_345);
        let ready =
            skull_model_instances_from_world_at_partial_tick(&world, 0.5, Some(&item_runtime));
        assert_eq!(
            ready[0].kind,
            EntityModelKind::SkullBlock {
                skull: EntityCustomHeadSkull::Player(EntityPlayerSkin::Dynamic(
                    EntityDynamicPlayerSkin {
                        handle: 12_345,
                        fallback: EntityDefaultPlayerSkin::SlimAlex,
                        model: EntityPlayerSkinModel::Wide,
                        status: EntityDynamicPlayerSkinStatus::Ready,
                    },
                )),
                attachment: SkullBlockModelAttachment::Ground,
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

        let instances = skull_model_instances_from_world_at_partial_tick(&world, 0.25, None);

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
