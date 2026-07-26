use super::*;
use bbb_protocol::{
    builtin_enum_component_values::VANILLA_DYE_COLOR_RED_ID,
    data_component_types::{
        VANILLA_DATA_COMPONENT_BUCKET_ENTITY_DATA_TYPE_ID, VANILLA_DATA_COMPONENT_DYE_TYPE_ID,
        VANILLA_DATA_COMPONENT_USE_REMAINDER_TYPE_ID,
    },
    entity_data_serializers::{
        VANILLA_ENTITY_DATA_SERIALIZER_BOOLEAN_ID, VANILLA_ENTITY_DATA_SERIALIZER_BYTE_ID,
        VANILLA_ENTITY_DATA_SERIALIZER_FLOAT_ID, VANILLA_ENTITY_DATA_SERIALIZER_INT_ID,
        VANILLA_ENTITY_DATA_SERIALIZER_OPTIONAL_LIVING_ENTITY_REFERENCE_ID,
    },
    entity_types::{
        VANILLA_ENTITY_TYPES, VANILLA_ENTITY_TYPE_ALLAY_ID, VANILLA_ENTITY_TYPE_AXOLOTL_ID,
        VANILLA_ENTITY_TYPE_BEE_ID, VANILLA_ENTITY_TYPE_CAMEL_HUSK_ID,
        VANILLA_ENTITY_TYPE_CAMEL_ID, VANILLA_ENTITY_TYPE_COD_ID, VANILLA_ENTITY_TYPE_DOLPHIN_ID,
        VANILLA_ENTITY_TYPE_GOAT_ID, VANILLA_ENTITY_TYPE_LLAMA_ID,
        VANILLA_ENTITY_TYPE_MOOSHROOM_ID, VANILLA_ENTITY_TYPE_NAUTILUS_ID,
        VANILLA_ENTITY_TYPE_PANDA_ID, VANILLA_ENTITY_TYPE_PARROT_ID, VANILLA_ENTITY_TYPE_PIGLIN_ID,
        VANILLA_ENTITY_TYPE_SKELETON_HORSE_ID, VANILLA_ENTITY_TYPE_STRIDER_ID,
        VANILLA_ENTITY_TYPE_TADPOLE_ID, VANILLA_ENTITY_TYPE_WOLF_ID,
        VANILLA_ENTITY_TYPE_ZOMBIE_VILLAGER_ID,
    },
    equipment_slots::{EquipmentSlot, VANILLA_PLAYER_INVENTORY_OFFHAND_SLOT},
    game_types::VANILLA_GAME_TYPE_SURVIVAL_ID,
    item_types::{
        VANILLA_ITEM_AXOLOTL_BUCKET_ID, VANILLA_ITEM_BOWL_ID, VANILLA_ITEM_CACTUS_ID,
        VANILLA_ITEM_GOLDEN_APPLE_ID, VANILLA_ITEM_GOLDEN_DANDELION_ID, VANILLA_ITEM_POPPY_ID,
        VANILLA_ITEM_RED_DYE_ID, VANILLA_ITEM_STONE_ID, VANILLA_ITEM_SUSPICIOUS_STEW_ID,
        VANILLA_ITEM_TROPICAL_FISH_BUCKET_ID, VANILLA_ITEM_WARPED_FUNGUS_ID,
        VANILLA_ITEM_WATER_BUCKET_ID, VANILLA_ITEM_WHEAT_ID, VANILLA_ITEM_WITHER_ROSE_ID,
    },
    mob_effect_types::{
        VANILLA_MOB_EFFECT_NIGHT_VISION_ID, VANILLA_MOB_EFFECT_POISON_ID,
        VANILLA_MOB_EFFECT_WEAKNESS_ID,
    },
    packets::{
        AddEntity, CommonPlayerSpawnInfo, DataComponentPatchSummary, EntityDataValue,
        EntityDataValueKind, EquipmentSlotUpdate, ItemStackSummary, ItemStackTemplateSummary,
        MobEffectFlags, NbtSummaryValue, PlayLogin, PlayerAbilities, SetEntityData, SetEquipment,
        SetPlayerInventory, TagNetworkPayload, UpdateMobEffect, UpdateTags, Vec3d,
    },
};
use std::collections::{BTreeMap, BTreeSet};
use uuid::Uuid;

fn stack(item_id: i32, count: i32) -> ItemStackSummary {
    ItemStackSummary {
        item_id: Some(item_id),
        count,
        component_patch: DataComponentPatchSummary::default(),
    }
}

fn set_main_hand(world: &mut WorldStore, item: ItemStackSummary) {
    assert!(world.apply_set_player_inventory(SetPlayerInventory { slot: 0, item }));
}

fn add_entity(world: &mut WorldStore, id: i32, entity_type_id: i32) {
    world.apply_add_entity(AddEntity {
        id,
        uuid: Uuid::from_u128(id as u128),
        entity_type_id,
        position: Vec3d::default(),
        delta_movement: Vec3d::default(),
        x_rot: 0.0,
        y_rot: 0.0,
        y_head_rot: 0.0,
        data: 0,
    });
}

fn client_item_success_with_sound(
    sound_action: LocalInteractionSoundAction,
) -> LocalInteractionSuccess {
    LocalInteractionSuccess {
        sound_action: Some(sound_action),
        ..LocalInteractionSuccess::client_item_interaction()
    }
}

fn install_tags(
    world: &mut WorldStore,
    registries: impl IntoIterator<Item = (&'static str, Vec<(&'static str, Vec<i32>)>)>,
) {
    world.apply_update_tags(UpdateTags {
        registries: registries
            .into_iter()
            .map(|(registry, tags)| bbb_protocol::packets::RegistryTags {
                registry: registry.to_string(),
                tags: tags
                    .into_iter()
                    .map(|(tag, entries)| TagNetworkPayload {
                        tag: tag.to_string(),
                        entries,
                    })
                    .collect(),
            })
            .collect(),
    });
}

fn install_overworld_level(world: &mut WorldStore) {
    world.apply_login(&PlayLogin {
        player_id: 7,
        hardcore: false,
        levels: vec!["minecraft:overworld".to_string()],
        max_players: 20,
        chunk_radius: 8,
        simulation_distance: 6,
        reduced_debug_info: false,
        show_death_screen: true,
        do_limited_crafting: false,
        common_spawn_info: CommonPlayerSpawnInfo {
            dimension_type_id: 0,
            dimension: "minecraft:overworld".to_string(),
            seed: 0,
            game_type: VANILLA_GAME_TYPE_SURVIVAL_ID as i8,
            previous_game_type: -1,
            is_debug: false,
            is_flat: false,
            last_death_location: None,
            portal_cooldown: 0,
            sea_level: 63,
        },
        enforces_secure_chat: true,
    });
}

#[test]
fn local_mob_interact_generated_owners_are_exhaustively_classified() {
    let mut mob_rows = 0;
    for (entity_type_id, entity_type) in VANILLA_ENTITY_TYPES.iter().enumerate() {
        let entity_type_id = i32::try_from(entity_type_id).unwrap();
        let Some(owner) = vanilla_entity_metadata_mob_interact_owner(entity_type_id) else {
            continue;
        };
        mob_rows += 1;
        assert!(
            generated_mob_interact_owner_is_classified(owner),
            "{} ({entity_type_id}) leaves generated mobInteract owner {owner} on the generic fallback",
            entity_type.resource_id,
        );
    }
    assert_eq!(mob_rows, 89);
}

#[test]
fn local_mob_interact_allay_give_and_take_mutate_equipment_inventory_and_sound() {
    let mut world = WorldStore::new();
    add_entity(&mut world, 1, VANILLA_ENTITY_TYPE_ALLAY_ID);
    set_main_hand(&mut world, stack(VANILLA_ITEM_STONE_ID, 2));

    assert_eq!(
        world.predict_local_entity_interaction(1, InteractionHand::MainHand, false),
        LocalTargetUsePrediction::Result(LocalInteractionResult::Success(
            client_item_success_with_sound(LocalInteractionSoundAction::AllayInteraction {
                entity_id: 1,
                sound_resource_id: "minecraft:entity.allay.item_given",
            })
        ))
    );
    assert_eq!(
        world
            .entities
            .equipment_item(1, EquipmentSlot::MainHand)
            .unwrap()
            .item_id,
        Some(VANILLA_ITEM_STONE_ID)
    );
    assert_eq!(
        world
            .local_item_in_hand(InteractionHand::MainHand)
            .unwrap()
            .count,
        1
    );

    set_main_hand(&mut world, ItemStackSummary::empty());
    assert_eq!(
        world.predict_local_entity_interaction(1, InteractionHand::MainHand, false),
        LocalTargetUsePrediction::Result(LocalInteractionResult::Success(
            client_item_success_with_sound(LocalInteractionSoundAction::AllayInteraction {
                entity_id: 1,
                sound_resource_id: "minecraft:entity.allay.item_taken",
            })
        ))
    );
    assert!(world
        .entities
        .equipment_item(1, EquipmentSlot::MainHand)
        .is_none());
    assert_eq!(
        world
            .local_item_in_hand(InteractionHand::MainHand)
            .unwrap()
            .item_id,
        Some(VANILLA_ITEM_STONE_ID)
    );
}

#[test]
fn local_mob_interact_bucketable_saves_client_data_replaces_hand_and_discards_entity() {
    let mut world = WorldStore::new();
    add_entity(&mut world, 2, VANILLA_ENTITY_TYPE_AXOLOTL_ID);
    set_main_hand(&mut world, stack(VANILLA_ITEM_WATER_BUCKET_ID, 1));

    assert_eq!(
        world.predict_local_entity_interaction(2, InteractionHand::MainHand, false),
        LocalTargetUsePrediction::Result(LocalInteractionResult::Success(
            client_item_success_with_sound(LocalInteractionSoundAction::SuppressedMobInteraction {
                entity_id: 2,
                silent: false,
                random_float_count: 0,
                random_before_silent_check: false,
            })
        ))
    );
    assert!(world.entities.identity(2).is_none());
    let bucket = world
        .local_item_in_hand(InteractionHand::MainHand)
        .expect("filled axolotl bucket");
    assert_eq!(bucket.item_id, Some(VANILLA_ITEM_AXOLOTL_BUCKET_ID));
    assert!(bucket
        .component_patch
        .added_type_ids
        .contains(&VANILLA_DATA_COMPONENT_BUCKET_ENTITY_DATA_TYPE_ID));
    let NbtSummaryValue::Compound(bucket_data) = bucket
        .component_patch
        .codec_backed_nbt
        .get(&VANILLA_DATA_COMPONENT_BUCKET_ENTITY_DATA_TYPE_ID)
        .expect("bucket entity data")
    else {
        panic!("bucket entity data must be a compound");
    };
    assert!(bucket_data.iter().any(|entry| entry.name == "Health"));
    assert!(bucket_data.iter().any(|entry| entry.name == "Age"));
    assert!(bucket_data.iter().any(|entry| entry.name == "AgeLocked"));

    add_entity(&mut world, 20, VANILLA_ENTITY_TYPE_COD_ID);
    let living = crate::entities::vanilla_entity_metadata_living_entity_accessors_for_entity_type(
        VANILLA_ENTITY_TYPE_COD_ID,
    )
    .unwrap();
    assert!(world.apply_set_entity_data(SetEntityData {
        id: 20,
        values: vec![EntityDataValue {
            data_id: living.health_data_id,
            serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_FLOAT_ID,
            value: EntityDataValueKind::Float(0.0),
        }],
    }));
    set_main_hand(&mut world, stack(VANILLA_ITEM_WATER_BUCKET_ID, 1));
    assert_eq!(
        world.predict_local_entity_interaction(20, InteractionHand::MainHand, false),
        LocalTargetUsePrediction::Result(LocalInteractionResult::Pass)
    );
    assert!(world.entities.identity(20).is_some());
    assert_eq!(
        world
            .local_item_in_hand(InteractionHand::MainHand)
            .unwrap()
            .item_id,
        Some(VANILLA_ITEM_WATER_BUCKET_ID)
    );

    let mut full_creative = WorldStore::new();
    add_entity(&mut full_creative, 21, VANILLA_ENTITY_TYPE_AXOLOTL_ID);
    full_creative.apply_player_abilities(PlayerAbilities {
        invulnerable: true,
        flying: false,
        can_fly: true,
        instabuild: true,
        flying_speed: 0.05,
        walking_speed: 0.1,
    });
    for slot in 0..36 {
        assert!(
            full_creative.apply_set_player_inventory(SetPlayerInventory {
                slot,
                item: stack(VANILLA_ITEM_STONE_ID, 64),
            })
        );
    }
    assert!(
        full_creative.apply_set_player_inventory(SetPlayerInventory {
            slot: VANILLA_PLAYER_INVENTORY_OFFHAND_SLOT,
            item: stack(VANILLA_ITEM_STONE_ID, 64),
        })
    );
    set_main_hand(&mut full_creative, stack(VANILLA_ITEM_WATER_BUCKET_ID, 1));
    assert_eq!(
        full_creative.predict_local_entity_interaction(21, InteractionHand::MainHand, false),
        LocalTargetUsePrediction::Result(LocalInteractionResult::Success(
            LocalInteractionSuccess {
                sound_action: Some(LocalInteractionSoundAction::SuppressedMobInteraction {
                    entity_id: 21,
                    silent: false,
                    random_float_count: 0,
                    random_before_silent_check: false,
                }),
                ..LocalInteractionSuccess::client_item_interaction()
            }
        ))
    );
    assert_eq!(
        full_creative
            .local_item_in_hand(InteractionHand::MainHand)
            .unwrap()
            .item_id,
        Some(VANILLA_ITEM_WATER_BUCKET_ID)
    );
}

#[test]
fn local_mob_interact_use_remainder_replaces_consumed_singleton() {
    let mut world = WorldStore::new();
    add_entity(&mut world, 18, VANILLA_ENTITY_TYPE_BEE_ID);
    install_tags(
        &mut world,
        [(
            "minecraft:item",
            vec![("minecraft:bee_food", vec![VANILLA_ITEM_WITHER_ROSE_ID])],
        )],
    );
    let mut flower = stack(VANILLA_ITEM_WITHER_ROSE_ID, 1);
    flower.component_patch.added = 1;
    flower
        .component_patch
        .added_type_ids
        .push(VANILLA_DATA_COMPONENT_USE_REMAINDER_TYPE_ID);
    flower.component_patch.use_remainder = Some(Box::new(ItemStackTemplateSummary {
        item_id: VANILLA_ITEM_STONE_ID,
        count: 1,
        component_patch: DataComponentPatchSummary::default(),
    }));
    set_main_hand(&mut world, flower.clone());

    assert_eq!(
        world.predict_local_entity_interaction(18, InteractionHand::MainHand, false),
        LocalTargetUsePrediction::Result(LocalInteractionResult::Success(
            LocalInteractionSuccess::client_item_interaction()
        ))
    );
    assert_eq!(
        world
            .local_item_in_hand(InteractionHand::MainHand)
            .expect("use remainder replaces the consumed flower"),
        &stack(VANILLA_ITEM_STONE_ID, 1)
    );

    flower.count = 2;
    set_main_hand(&mut world, flower);
    assert!(matches!(
        world.predict_local_entity_interaction(18, InteractionHand::MainHand, false),
        LocalTargetUsePrediction::Result(LocalInteractionResult::Success(_))
    ));
    let remaining = world
        .local_item_in_hand(InteractionHand::MainHand)
        .expect("an extra local remainder is discarded while the used stack remains");
    assert_eq!(remaining.item_id, Some(VANILLA_ITEM_WITHER_ROSE_ID));
    assert_eq!(remaining.count, 1);
    assert!(remaining
        .component_patch
        .added_type_ids
        .contains(&VANILLA_DATA_COMPONENT_USE_REMAINDER_TYPE_ID));
}

#[test]
fn local_mob_interact_axolotl_bucket_food_returns_water_bucket() {
    let mut world = WorldStore::new();
    add_entity(&mut world, 19, VANILLA_ENTITY_TYPE_AXOLOTL_ID);
    let (baby_data_id, _) =
        vanilla_entity_metadata_baby_accessor_for_entity_type(VANILLA_ENTITY_TYPE_AXOLOTL_ID)
            .unwrap();
    assert!(world.apply_set_entity_data(SetEntityData {
        id: 19,
        values: vec![EntityDataValue {
            data_id: baby_data_id,
            serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_BOOLEAN_ID,
            value: EntityDataValueKind::Boolean(true),
        }],
    }));
    install_tags(
        &mut world,
        [(
            "minecraft:item",
            vec![(
                "minecraft:axolotl_food",
                vec![VANILLA_ITEM_TROPICAL_FISH_BUCKET_ID],
            )],
        )],
    );
    set_main_hand(&mut world, stack(VANILLA_ITEM_TROPICAL_FISH_BUCKET_ID, 1));

    assert_eq!(
        world.predict_local_entity_interaction(19, InteractionHand::MainHand, false),
        LocalTargetUsePrediction::Result(LocalInteractionResult::Success(
            LocalInteractionSuccess::client_item_interaction()
        ))
    );
    assert_eq!(
        world
            .local_item_in_hand(InteractionHand::MainHand)
            .expect("bucket food creates a water bucket")
            .item_id,
        Some(VANILLA_ITEM_WATER_BUCKET_ID)
    );
}

#[test]
fn local_mob_interact_nautilus_bucket_food_uses_virtual_player_item_override() {
    let mut world = WorldStore::new();
    add_entity(&mut world, 20, VANILLA_ENTITY_TYPE_NAUTILUS_ID);
    let (baby_data_id, _) =
        vanilla_entity_metadata_baby_accessor_for_entity_type(VANILLA_ENTITY_TYPE_NAUTILUS_ID)
            .unwrap();
    assert!(world.apply_set_entity_data(SetEntityData {
        id: 20,
        values: vec![EntityDataValue {
            data_id: baby_data_id,
            serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_BOOLEAN_ID,
            value: EntityDataValueKind::Boolean(true),
        }],
    }));
    install_tags(
        &mut world,
        [(
            "minecraft:item",
            vec![
                (
                    VANILLA_NAUTILUS_FOOD_TAG,
                    vec![VANILLA_ITEM_TROPICAL_FISH_BUCKET_ID],
                ),
                (
                    VANILLA_NAUTILUS_BUCKET_FOOD_TAG,
                    vec![VANILLA_ITEM_TROPICAL_FISH_BUCKET_ID],
                ),
            ],
        )],
    );
    set_main_hand(&mut world, stack(VANILLA_ITEM_TROPICAL_FISH_BUCKET_ID, 1));

    assert_eq!(
        world.predict_local_entity_interaction(20, InteractionHand::MainHand, false),
        LocalTargetUsePrediction::Result(LocalInteractionResult::Success(
            LocalInteractionSuccess::client_item_interaction_with_eating(20, 1)
        ))
    );
    assert_eq!(
        world
            .local_item_in_hand(InteractionHand::MainHand)
            .expect("bucket food creates a water bucket")
            .item_id,
        Some(VANILLA_ITEM_WATER_BUCKET_ID)
    );
}

#[test]
fn local_mob_interact_dolphin_consumes_fish_and_sets_synced_got_fish() {
    let mut world = WorldStore::new();
    add_entity(&mut world, 3, VANILLA_ENTITY_TYPE_DOLPHIN_ID);
    install_tags(
        &mut world,
        [(
            "minecraft:item",
            vec![("minecraft:fishes", vec![VANILLA_ITEM_POPPY_ID])],
        )],
    );
    set_main_hand(&mut world, stack(VANILLA_ITEM_POPPY_ID, 1));

    assert_eq!(
        world.predict_local_entity_interaction(3, InteractionHand::MainHand, false),
        LocalTargetUsePrediction::Result(LocalInteractionResult::Success(
            LocalInteractionSuccess::client_item_interaction()
        ))
    );
    assert!(world
        .local_item_in_hand(InteractionHand::MainHand)
        .is_none());
    let accessors =
        vanilla_entity_metadata_dolphin_accessors_for_entity_type(VANILLA_ENTITY_TYPE_DOLPHIN_ID)
            .unwrap();
    assert_eq!(
        world.entities.metadata_bool_for_entity(
            3,
            accessors.got_fish_data_id,
            accessors.got_fish_default
        ),
        Some(true)
    );
}

#[test]
fn local_mob_interact_brown_mooshroom_tracks_ingredient_particles_and_stew() {
    let mut world = WorldStore::new();
    add_entity(&mut world, 4, VANILLA_ENTITY_TYPE_MOOSHROOM_ID);
    let (variant_data_id, _) =
        vanilla_entity_metadata_mushroom_cow_variant_accessor_for_entity_type(
            VANILLA_ENTITY_TYPE_MOOSHROOM_ID,
        )
        .unwrap();
    assert!(world.apply_set_entity_data(SetEntityData {
        id: 4,
        values: vec![EntityDataValue {
            data_id: variant_data_id,
            serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_INT_ID,
            value: EntityDataValueKind::Int(1),
        }],
    }));
    set_main_hand(&mut world, stack(VANILLA_ITEM_POPPY_ID, 1));

    let result = world.predict_local_entity_interaction(4, InteractionHand::MainHand, false);
    assert!(matches!(
        result,
        LocalTargetUsePrediction::Result(LocalInteractionResult::Success(
            LocalInteractionSuccess {
                particle_action: Some(LocalInteractionParticleAction::MooshroomEffect { .. }),
                ..
            }
        ))
    ));
    assert_eq!(
        world.entities.local_mooshroom_stew_ingredient(4),
        Some(Some(VANILLA_ITEM_POPPY_ID))
    );
    assert!(world
        .local_item_in_hand(InteractionHand::MainHand)
        .is_none());

    set_main_hand(&mut world, stack(VANILLA_ITEM_BOWL_ID, 1));
    let result = world.predict_local_entity_interaction(4, InteractionHand::MainHand, false);
    assert!(matches!(
        result,
        LocalTargetUsePrediction::Result(LocalInteractionResult::Success(
            LocalInteractionSuccess {
                sound_action: Some(LocalInteractionSoundAction::SuppressedMobInteraction {
                    entity_id: 4,
                    ..
                }),
                ..
            }
        ))
    ));
    assert_eq!(
        world.entities.local_mooshroom_stew_ingredient(4),
        Some(None)
    );
    let stew = world
        .local_item_in_hand(InteractionHand::MainHand)
        .expect("suspicious stew");
    assert_eq!(stew.item_id, Some(VANILLA_ITEM_SUSPICIOUS_STEW_ID));
    assert_eq!(
        stew.component_patch.suspicious_stew_effects,
        vec![SuspiciousStewEffectSummary {
            effect_id: VANILLA_MOB_EFFECT_NIGHT_VISION_ID,
            duration: 100,
        }]
    );
}

#[test]
fn local_mob_interact_tadpole_feed_and_age_lock_update_private_and_synced_state() {
    let mut world = WorldStore::new();
    add_entity(&mut world, 5, VANILLA_ENTITY_TYPE_TADPOLE_ID);
    install_tags(
        &mut world,
        [
            (
                "minecraft:item",
                vec![("minecraft:frog_food", vec![VANILLA_ITEM_POPPY_ID])],
            ),
            (
                "minecraft:entity_type",
                vec![("minecraft:cannot_be_age_locked", Vec::new())],
            ),
        ],
    );
    set_main_hand(&mut world, stack(VANILLA_ITEM_POPPY_ID, 1));

    let result = world.predict_local_entity_interaction(5, InteractionHand::MainHand, false);
    assert!(matches!(
        result,
        LocalTargetUsePrediction::Result(LocalInteractionResult::Success(
            LocalInteractionSuccess {
                particle_action: Some(LocalInteractionParticleAction::TadpoleFeeding { .. }),
                ..
            }
        ))
    ));
    assert_eq!(world.entities.local_tadpole_age_ticks(5), Some(2_400));

    set_main_hand(&mut world, stack(VANILLA_ITEM_GOLDEN_DANDELION_ID, 1));
    assert_eq!(
        world.predict_local_entity_interaction(5, InteractionHand::MainHand, false),
        LocalTargetUsePrediction::Result(LocalInteractionResult::Success(
            client_item_success_with_sound(LocalInteractionSoundAction::SuppressedMobInteraction {
                entity_id: 5,
                silent: false,
                random_float_count: 0,
                random_before_silent_check: false,
            })
        ))
    );
    let (age_locked_data_id, age_locked_default) =
        vanilla_entity_metadata_age_locked_accessor_for_entity_type(VANILLA_ENTITY_TYPE_TADPOLE_ID)
            .unwrap();
    assert_eq!(
        world
            .entities
            .metadata_bool_for_entity(5, age_locked_data_id, age_locked_default),
        Some(true)
    );
    assert_eq!(world.entities.local_tadpole_age_ticks(5), Some(0));
    assert_eq!(world.entities.age_lock_particle_timer_ticks(5), Some(40));
}

#[test]
fn local_mob_interact_owned_wolf_dye_and_empty_hand_toggle_mutate_client_state() {
    let mut world = WorldStore::new();
    add_entity(&mut world, 6, VANILLA_ENTITY_TYPE_WOLF_ID);
    let player_id = Uuid::from_u128(0xabc);
    world.set_local_player_profile_id(player_id);
    let tamable = vanilla_entity_metadata_tamable_animal_accessors_for_entity_type(
        VANILLA_ENTITY_TYPE_WOLF_ID,
    )
    .unwrap();
    let wolf = vanilla_entity_metadata_wolf_accessors_for_entity_type(VANILLA_ENTITY_TYPE_WOLF_ID)
        .unwrap();
    assert!(world.apply_set_entity_data(SetEntityData {
        id: 6,
        values: vec![
            EntityDataValue {
                data_id: tamable.flags_data_id,
                serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_BYTE_ID,
                value: EntityDataValueKind::Byte(tamable.tame_mask),
            },
            EntityDataValue {
                data_id: tamable.owner_data_id,
                serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_OPTIONAL_LIVING_ENTITY_REFERENCE_ID,
                value: EntityDataValueKind::OptionalLivingEntityReference(Some(player_id)),
            },
            EntityDataValue {
                data_id: wolf.collar_color_data_id,
                serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_INT_ID,
                value: EntityDataValueKind::Int(0),
            },
        ],
    }));
    install_tags(
        &mut world,
        [(
            "minecraft:item",
            vec![
                ("minecraft:wolf_food", Vec::new()),
                ("minecraft:wolf_collar_dyes", vec![VANILLA_ITEM_RED_DYE_ID]),
            ],
        )],
    );
    world.set_default_item_dye_color_ids(BTreeMap::from([(
        VANILLA_ITEM_RED_DYE_ID,
        VANILLA_DYE_COLOR_RED_ID,
    )]));
    world.set_default_item_component_type_ids(BTreeMap::from([(
        VANILLA_ITEM_RED_DYE_ID,
        BTreeSet::from([VANILLA_DATA_COMPONENT_DYE_TYPE_ID]),
    )]));
    set_main_hand(&mut world, stack(VANILLA_ITEM_RED_DYE_ID, 1));

    assert_eq!(
        world.predict_local_entity_interaction(6, InteractionHand::MainHand, false),
        LocalTargetUsePrediction::Result(LocalInteractionResult::Success(
            LocalInteractionSuccess::client_item_interaction()
        ))
    );
    assert_eq!(
        world.entities.metadata_int_for_entity(
            6,
            wolf.collar_color_data_id,
            wolf.collar_color_default
        ),
        Some(VANILLA_DYE_COLOR_RED_ID)
    );

    set_main_hand(&mut world, ItemStackSummary::empty());
    assert_eq!(
        world.predict_local_entity_interaction(6, InteractionHand::MainHand, false),
        LocalTargetUsePrediction::Result(LocalInteractionResult::Success(
            LocalInteractionSuccess::client_without_item()
        ))
    );
    assert_eq!(world.entities.local_ordered_to_sit(6), Some(true));
}

#[test]
fn local_mob_interact_parrot_poison_food_adds_local_effect() {
    let mut world = WorldStore::new();
    add_entity(&mut world, 7, VANILLA_ENTITY_TYPE_PARROT_ID);
    install_tags(
        &mut world,
        [(
            "minecraft:item",
            vec![
                ("minecraft:parrot_food", Vec::new()),
                (
                    "minecraft:parrot_poisonous_food",
                    vec![VANILLA_ITEM_POPPY_ID],
                ),
            ],
        )],
    );
    set_main_hand(&mut world, stack(VANILLA_ITEM_POPPY_ID, 1));

    assert_eq!(
        world.predict_local_entity_interaction(7, InteractionHand::MainHand, false),
        LocalTargetUsePrediction::Result(LocalInteractionResult::Success(
            LocalInteractionSuccess::client_item_interaction()
        ))
    );
    assert_eq!(
        world
            .entities
            .mob_effect_amplifier(7, VANILLA_MOB_EFFECT_POISON_ID),
        Some(0)
    );

    assert!(world.apply_update_mob_effect(UpdateMobEffect {
        entity_id: 7,
        effect_id: VANILLA_MOB_EFFECT_POISON_ID,
        amplifier: 2,
        duration_ticks: 1_200,
        flags: MobEffectFlags {
            raw: 0x01,
            ambient: true,
            visible: false,
            show_icon: false,
            blend: false,
        },
    }));
    set_main_hand(&mut world, stack(VANILLA_ITEM_POPPY_ID, 1));
    assert!(matches!(
        world.predict_local_entity_interaction(7, InteractionHand::MainHand, false),
        LocalTargetUsePrediction::Result(LocalInteractionResult::Success(_))
    ));
    let effect = world
        .entity_effect(7, VANILLA_MOB_EFFECT_POISON_ID)
        .expect("stronger poison remains active");
    assert_eq!(effect.amplifier, 2);
    assert_eq!(effect.duration_ticks, 1_200);
    assert!(!effect.ambient);
    assert!(effect.visible);
    assert!(effect.show_icon);
}

#[test]
fn local_mob_interact_piglin_dancing_precedes_admiring_offhand_pose() {
    let mut world = WorldStore::new();
    add_entity(&mut world, 17, VANILLA_ENTITY_TYPE_PIGLIN_ID);
    install_tags(
        &mut world,
        [(
            "minecraft:item",
            vec![("minecraft:piglin_loved", vec![VANILLA_ITEM_GOLD_INGOT_ID])],
        )],
    );
    assert!(world.apply_set_equipment(SetEquipment {
        entity_id: 17,
        slots: vec![EquipmentSlotUpdate {
            slot: EquipmentSlot::OffHand,
            item: stack(VANILLA_ITEM_GOLD_INGOT_ID, 1),
        }],
    }));
    set_main_hand(&mut world, stack(VANILLA_ITEM_GOLD_INGOT_ID, 1));
    assert_eq!(
        world.predict_local_entity_interaction(17, InteractionHand::MainHand, false),
        LocalTargetUsePrediction::Result(LocalInteractionResult::Pass)
    );

    let (dancing_data_id, _) = vanilla_entity_metadata_piglin_dancing_accessor_for_entity_type(
        VANILLA_ENTITY_TYPE_PIGLIN_ID,
    )
    .unwrap();
    assert!(world.apply_set_entity_data(SetEntityData {
        id: 17,
        values: vec![EntityDataValue {
            data_id: dancing_data_id,
            serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_BOOLEAN_ID,
            value: EntityDataValueKind::Boolean(true),
        }],
    }));
    assert_eq!(
        world.predict_local_entity_interaction(17, InteractionHand::MainHand, false),
        LocalTargetUsePrediction::Result(LocalInteractionResult::Success(
            LocalInteractionSuccess::client_item_interaction()
        ))
    );
}

#[test]
fn local_mob_interact_strider_food_tracks_direct_eating_sound_randomness() {
    let mut world = WorldStore::new();
    add_entity(&mut world, 8, VANILLA_ENTITY_TYPE_STRIDER_ID);
    install_tags(
        &mut world,
        [(
            "minecraft:item",
            vec![(
                "minecraft:strider_food",
                vec![VANILLA_ITEM_WARPED_FUNGUS_ID],
            )],
        )],
    );
    set_main_hand(&mut world, stack(VANILLA_ITEM_WARPED_FUNGUS_ID, 1));

    assert!(matches!(
        world.predict_local_entity_interaction(8, InteractionHand::MainHand, false),
        LocalTargetUsePrediction::Result(LocalInteractionResult::Success(
            LocalInteractionSuccess {
                sound_action: Some(LocalInteractionSoundAction::SuppressedMobInteraction {
                    entity_id: 8,
                    silent: false,
                    random_float_count: 2,
                    random_before_silent_check: false,
                }),
                ..
            }
        ))
    ));
}

#[test]
fn local_mob_interact_full_health_baby_llama_only_emits_client_particle() {
    let mut world = WorldStore::new();
    add_entity(&mut world, 9, VANILLA_ENTITY_TYPE_LLAMA_ID);
    install_tags(
        &mut world,
        [(
            "minecraft:item",
            vec![("minecraft:llama_food", vec![VANILLA_ITEM_WHEAT_ID])],
        )],
    );
    let horse = vanilla_entity_metadata_abstract_horse_accessors_for_entity_type(
        VANILLA_ENTITY_TYPE_LLAMA_ID,
    )
    .unwrap();
    let (baby_data_id, _) = crate::entities::vanilla_entity_metadata_baby_accessor_for_entity_type(
        VANILLA_ENTITY_TYPE_LLAMA_ID,
    )
    .unwrap();
    let living = crate::entities::vanilla_entity_metadata_living_entity_accessors_for_entity_type(
        VANILLA_ENTITY_TYPE_LLAMA_ID,
    )
    .unwrap();
    let full_health = world
        .entities
        .attribute_value(9, VANILLA_ATTRIBUTE_MAX_HEALTH_ID)
        .unwrap() as f32;
    assert!(world.apply_set_entity_data(SetEntityData {
        id: 9,
        values: vec![
            EntityDataValue {
                data_id: horse.flags_data_id,
                serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_BYTE_ID,
                value: EntityDataValueKind::Byte(horse.tame_mask),
            },
            EntityDataValue {
                data_id: baby_data_id,
                serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_BOOLEAN_ID,
                value: EntityDataValueKind::Boolean(true),
            },
            EntityDataValue {
                data_id: living.health_data_id,
                serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_FLOAT_ID,
                value: EntityDataValueKind::Float(full_health),
            },
        ],
    }));
    set_main_hand(&mut world, stack(VANILLA_ITEM_WHEAT_ID, 2));

    let prediction = world.predict_local_entity_interaction(9, InteractionHand::MainHand, false);
    assert!(
        matches!(
            prediction,
            LocalTargetUsePrediction::Result(LocalInteractionResult::Success(
                LocalInteractionSuccess {
                    sound_action: None,
                    particle_action: Some(LocalInteractionParticleAction::EquineBabyFeeding { .. }),
                    ..
                }
            ))
        ),
        "{prediction:?}"
    );
    assert_eq!(
        world
            .local_item_in_hand(InteractionHand::MainHand)
            .unwrap()
            .count,
        2,
        "Llama.handleEating leaves baby age-up and consumption to the server"
    );
}

#[test]
fn local_mob_interact_camel_age_up_consumes_but_husk_ignores_baby_metadata() {
    let mut world = WorldStore::new();
    add_entity(&mut world, 10, VANILLA_ENTITY_TYPE_CAMEL_ID);
    add_entity(&mut world, 11, VANILLA_ENTITY_TYPE_CAMEL_HUSK_ID);
    install_tags(
        &mut world,
        [(
            "minecraft:item",
            vec![
                ("minecraft:camel_food", vec![VANILLA_ITEM_CACTUS_ID]),
                ("minecraft:camel_husk_food", vec![VANILLA_ITEM_CACTUS_ID]),
            ],
        )],
    );
    for (entity_id, entity_type_id) in [
        (10, VANILLA_ENTITY_TYPE_CAMEL_ID),
        (11, VANILLA_ENTITY_TYPE_CAMEL_HUSK_ID),
    ] {
        let (baby_data_id, _) =
            crate::entities::vanilla_entity_metadata_baby_accessor_for_entity_type(entity_type_id)
                .unwrap();
        let living =
            crate::entities::vanilla_entity_metadata_living_entity_accessors_for_entity_type(
                entity_type_id,
            )
            .unwrap();
        let full_health = world
            .entities
            .attribute_value(entity_id, VANILLA_ATTRIBUTE_MAX_HEALTH_ID)
            .unwrap() as f32;
        assert!(world.apply_set_entity_data(SetEntityData {
            id: entity_id,
            values: vec![
                EntityDataValue {
                    data_id: baby_data_id,
                    serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_BOOLEAN_ID,
                    value: EntityDataValueKind::Boolean(true),
                },
                EntityDataValue {
                    data_id: living.health_data_id,
                    serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_FLOAT_ID,
                    value: EntityDataValueKind::Float(full_health),
                },
            ],
        }));
    }

    set_main_hand(&mut world, stack(VANILLA_ITEM_CACTUS_ID, 2));
    let prediction = world.predict_local_entity_interaction(10, InteractionHand::MainHand, false);
    assert!(
        matches!(
            prediction,
            LocalTargetUsePrediction::Result(LocalInteractionResult::Success(
                LocalInteractionSuccess {
                    sound_action: Some(LocalInteractionSoundAction::SuppressedEquineEating {
                        entity_id: 10
                    }),
                    particle_action: Some(LocalInteractionParticleAction::EquineBabyFeeding { .. }),
                    ..
                }
            ))
        ),
        "{prediction:?}"
    );
    assert_eq!(
        world
            .local_item_in_hand(InteractionHand::MainHand)
            .unwrap()
            .count,
        1
    );

    set_main_hand(&mut world, stack(VANILLA_ITEM_CACTUS_ID, 2));
    assert_eq!(
        world.predict_local_entity_interaction(11, InteractionHand::MainHand, false),
        LocalTargetUsePrediction::Result(LocalInteractionResult::Success(
            LocalInteractionSuccess::server_equine_food(11, false, None)
        ))
    );
    assert_eq!(
        world
            .local_item_in_hand(InteractionHand::MainHand)
            .unwrap()
            .count,
        2
    );
}

#[test]
fn local_mob_interact_baby_goat_runs_parent_and_specialized_eating_sounds() {
    let mut world = WorldStore::new();
    add_entity(&mut world, 12, VANILLA_ENTITY_TYPE_GOAT_ID);
    install_tags(
        &mut world,
        [(
            "minecraft:item",
            vec![("minecraft:goat_food", vec![VANILLA_ITEM_WHEAT_ID])],
        )],
    );
    let (baby_data_id, _) = crate::entities::vanilla_entity_metadata_baby_accessor_for_entity_type(
        VANILLA_ENTITY_TYPE_GOAT_ID,
    )
    .unwrap();
    assert!(world.apply_set_entity_data(SetEntityData {
        id: 12,
        values: vec![EntityDataValue {
            data_id: baby_data_id,
            serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_BOOLEAN_ID,
            value: EntityDataValueKind::Boolean(true),
        }],
    }));
    set_main_hand(&mut world, stack(VANILLA_ITEM_WHEAT_ID, 1));

    assert!(matches!(
        world.predict_local_entity_interaction(12, InteractionHand::MainHand, false),
        LocalTargetUsePrediction::Result(LocalInteractionResult::Success(
            LocalInteractionSuccess {
                sound_action: Some(LocalInteractionSoundAction::SuppressedAnimalEating {
                    entity_id: 12,
                    repetitions: 2,
                }),
                ..
            }
        ))
    ));
}

#[test]
fn local_mob_interact_panda_and_zombie_villager_preserve_server_swing_results() {
    let mut world = WorldStore::new();
    install_overworld_level(&mut world);
    add_entity(&mut world, 13, VANILLA_ENTITY_TYPE_PANDA_ID);
    install_tags(
        &mut world,
        [(
            "minecraft:item",
            vec![("minecraft:panda_food", vec![VANILLA_ITEM_WHEAT_ID])],
        )],
    );
    let (baby_data_id, _) = crate::entities::vanilla_entity_metadata_baby_accessor_for_entity_type(
        VANILLA_ENTITY_TYPE_PANDA_ID,
    )
    .unwrap();
    let panda =
        vanilla_entity_metadata_panda_accessors_for_entity_type(VANILLA_ENTITY_TYPE_PANDA_ID)
            .unwrap();
    assert!(world.apply_set_entity_data(SetEntityData {
        id: 13,
        values: vec![
            EntityDataValue {
                data_id: baby_data_id,
                serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_BOOLEAN_ID,
                value: EntityDataValueKind::Boolean(true),
            },
            EntityDataValue {
                data_id: panda.main_gene_data_id,
                serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_BYTE_ID,
                value: EntityDataValueKind::Byte(2),
            },
        ],
    }));
    set_main_hand(&mut world, stack(VANILLA_ITEM_WHEAT_ID, 1));
    world.weather.thunder_level = 1.0;
    world.weather.rain_level = 0.0;
    assert_eq!(
        world.predict_local_entity_interaction(13, InteractionHand::MainHand, false),
        LocalTargetUsePrediction::Result(LocalInteractionResult::Success(
            LocalInteractionSuccess::server_item_interaction()
        ))
    );
    set_main_hand(&mut world, stack(VANILLA_ITEM_WHEAT_ID, 1));
    world.weather.rain_level = 1.0;
    assert_eq!(
        world.predict_local_entity_interaction(13, InteractionHand::MainHand, false),
        LocalTargetUsePrediction::Result(LocalInteractionResult::Pass),
        "Level.getThunderLevel multiplies thunder by rain before Panda.isScared"
    );

    add_entity(&mut world, 14, VANILLA_ENTITY_TYPE_ZOMBIE_VILLAGER_ID);
    assert!(world.apply_update_mob_effect(UpdateMobEffect {
        entity_id: 14,
        effect_id: VANILLA_MOB_EFFECT_WEAKNESS_ID,
        amplifier: 0,
        duration_ticks: 100,
        flags: MobEffectFlags {
            raw: 0,
            ambient: false,
            visible: false,
            show_icon: false,
            blend: false,
        },
    }));
    set_main_hand(&mut world, stack(VANILLA_ITEM_GOLDEN_APPLE_ID, 1));
    assert_eq!(
        world.predict_local_entity_interaction(14, InteractionHand::MainHand, false),
        LocalTargetUsePrediction::Result(LocalInteractionResult::Success(
            LocalInteractionSuccess::server_item_interaction()
        ))
    );
}

#[test]
fn local_mob_interact_baby_skeleton_horse_cannot_age_up() {
    let mut world = WorldStore::new();
    add_entity(&mut world, 15, VANILLA_ENTITY_TYPE_SKELETON_HORSE_ID);
    install_tags(
        &mut world,
        [(
            "minecraft:item",
            vec![("minecraft:horse_food", vec![VANILLA_ITEM_WHEAT_ID])],
        )],
    );
    let horse = vanilla_entity_metadata_abstract_horse_accessors_for_entity_type(
        VANILLA_ENTITY_TYPE_SKELETON_HORSE_ID,
    )
    .unwrap();
    let (baby_data_id, _) = crate::entities::vanilla_entity_metadata_baby_accessor_for_entity_type(
        VANILLA_ENTITY_TYPE_SKELETON_HORSE_ID,
    )
    .unwrap();
    assert!(world.apply_set_entity_data(SetEntityData {
        id: 15,
        values: vec![
            EntityDataValue {
                data_id: horse.flags_data_id,
                serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_BYTE_ID,
                value: EntityDataValueKind::Byte(horse.tame_mask),
            },
            EntityDataValue {
                data_id: baby_data_id,
                serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_BOOLEAN_ID,
                value: EntityDataValueKind::Boolean(true),
            },
        ],
    }));
    set_main_hand(&mut world, stack(VANILLA_ITEM_WHEAT_ID, 2));

    assert_eq!(
        world.predict_local_entity_interaction(15, InteractionHand::MainHand, false),
        LocalTargetUsePrediction::Result(LocalInteractionResult::Success(
            LocalInteractionSuccess::consume(false)
        ))
    );
    assert_eq!(
        world
            .local_item_in_hand(InteractionHand::MainHand)
            .unwrap()
            .count,
        2
    );
}
