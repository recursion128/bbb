use super::*;

#[test]
fn hanging_entity_generated_owners_and_face_height_gates_are_exact() {
    let owner_ids = VANILLA_ITEM_TYPES
        .iter()
        .filter(|item| item.use_on_override == Some(VanillaItemUseOnOverride::HangingEntityItem))
        .map(|item| item.id)
        .collect::<Vec<_>>();
    assert_eq!(
        owner_ids,
        vec![
            VANILLA_ITEM_PAINTING_ID,
            VANILLA_ITEM_ITEM_FRAME_ID,
            VANILLA_ITEM_GLOW_ITEM_FRAME_ID,
        ]
    );

    let clicked = BlockPos { x: 8, y: 7, z: 8 };
    let stone = block_state_id("minecraft:stone", []);
    let mut world = tool_use_world(block_state_id("minecraft:air", []));
    set_block(&mut world, clicked, stone);
    set_main_hand(&mut world, VANILLA_ITEM_PAINTING_ID);
    assert_eq!(
        world.predict_local_block_use(
            clicked,
            ProtocolDirection::Up,
            InteractionHand::MainHand,
            true,
            65,
        ),
        LocalTargetUsePrediction::Result(LocalInteractionResult::Fail)
    );

    let mut held_frame = stack(VANILLA_ITEM_ITEM_FRAME_ID);
    held_frame.count = 2;
    world.apply_set_player_inventory(SetPlayerInventory {
        slot: 0,
        item: held_frame,
    });
    assert_eq!(
        world.predict_local_block_use(
            clicked,
            ProtocolDirection::Up,
            InteractionHand::MainHand,
            true,
            66,
        ),
        LocalTargetUsePrediction::Result(LocalInteractionResult::Success(
            LocalInteractionSuccess::client_item_interaction()
        ))
    );
    assert_eq!(player_inventory_item(&world, 0).count, 1);
    assert!(world.local_block_predictions().is_empty());

    let top = BlockPos {
        y: world.dimension().min_y + world.dimension().height - 1,
        ..clicked
    };
    set_block(&mut world, top, stone);
    set_main_hand(&mut world, VANILLA_ITEM_GLOW_ITEM_FRAME_ID);
    assert_eq!(
        world.predict_local_block_use(
            top,
            ProtocolDirection::Up,
            InteractionHand::MainHand,
            true,
            67,
        ),
        LocalTargetUsePrediction::Result(LocalInteractionResult::Fail)
    );
    assert_eq!(
        player_inventory_item(&world, 0).item_id,
        Some(VANILLA_ITEM_GLOW_ITEM_FRAME_ID)
    );
}

#[test]
fn armor_stand_creative_use_on_restores_the_pre_call_count() {
    let clicked = BlockPos { x: 8, y: 7, z: 8 };
    let stone = block_state_id("minecraft:stone", []);
    let mut world = tool_use_world(block_state_id("minecraft:air", []));
    set_block(&mut world, clicked, stone);
    world.apply_player_abilities(bbb_protocol::packets::PlayerAbilities {
        invulnerable: true,
        flying: false,
        can_fly: true,
        instabuild: true,
        flying_speed: 0.05,
        walking_speed: 0.1,
    });
    let mut held = stack(VANILLA_ITEM_ARMOR_STAND_ID);
    held.count = 2;
    world.apply_set_player_inventory(SetPlayerInventory {
        slot: 0,
        item: held,
    });

    assert!(matches!(
        world.predict_local_block_use(
            clicked,
            ProtocolDirection::Up,
            InteractionHand::MainHand,
            true,
            79,
        ),
        LocalTargetUsePrediction::Result(LocalInteractionResult::Success(_))
    ));
    assert_eq!(player_inventory_item(&world, 0).count, 2);
}

#[test]
fn armor_stand_generated_owner_and_down_face_gate_are_exact() {
    let owner_ids = VANILLA_ITEM_TYPES
        .iter()
        .filter(|item| item.use_on_override == Some(VanillaItemUseOnOverride::ArmorStandItem))
        .map(|item| item.id)
        .collect::<Vec<_>>();
    assert_eq!(owner_ids, vec![VANILLA_ITEM_ARMOR_STAND_ID]);

    let clicked = BlockPos { x: 8, y: 7, z: 8 };
    let stone = block_state_id("minecraft:stone", []);
    let mut world = tool_use_world(block_state_id("minecraft:air", []));
    set_block(&mut world, clicked, stone);
    let mut held = stack(VANILLA_ITEM_ARMOR_STAND_ID);
    held.count = 2;
    world.apply_set_player_inventory(SetPlayerInventory {
        slot: 0,
        item: held,
    });

    assert_eq!(
        world.predict_local_block_use(
            clicked,
            ProtocolDirection::Down,
            InteractionHand::MainHand,
            true,
            49,
        ),
        LocalTargetUsePrediction::Result(LocalInteractionResult::Fail)
    );
    assert_eq!(player_inventory_item(&world, 0).count, 2);
    assert_eq!(
        world.predict_local_block_use(
            clicked,
            ProtocolDirection::Up,
            InteractionHand::MainHand,
            true,
            50,
        ),
        LocalTargetUsePrediction::Result(LocalInteractionResult::Success(
            LocalInteractionSuccess::client_item_interaction()
        ))
    );
    assert_eq!(player_inventory_item(&world, 0).count, 1);
    assert!(world.local_block_predictions().is_empty());
}

#[test]
fn armor_stand_requires_every_non_spectator_entity_box_to_be_empty() {
    let clicked = BlockPos { x: 8, y: 7, z: 8 };
    let target = BlockPos {
        y: clicked.y + 1,
        ..clicked
    };
    let air = block_state_id("minecraft:air", []);
    let stone = block_state_id("minecraft:stone", []);
    let prepare = || {
        let mut world = tool_use_world(air);
        set_block(&mut world, clicked, stone);
        set_main_hand(&mut world, VANILLA_ITEM_ARMOR_STAND_ID);
        world
    };
    let position = Vec3d {
        x: f64::from(target.x) + 0.5,
        y: f64::from(target.y) + 0.5,
        z: f64::from(target.z) + 0.5,
    };

    let mut dropped_item = prepare();
    add_entity_at(&mut dropped_item, 94, VANILLA_ENTITY_TYPE_ITEM_ID, position);
    assert_eq!(
        dropped_item.predict_local_block_use(
            clicked,
            ProtocolDirection::Up,
            InteractionHand::MainHand,
            true,
            54,
        ),
        LocalTargetUsePrediction::Result(LocalInteractionResult::Fail)
    );

    let mut local_player = prepare();
    local_player.set_local_player_pose(crate::LocalPlayerPoseState {
        position,
        ..crate::LocalPlayerPoseState::default()
    });
    assert_eq!(
        local_player.predict_local_block_use(
            clicked,
            ProtocolDirection::Up,
            InteractionHand::MainHand,
            true,
            55,
        ),
        LocalTargetUsePrediction::Result(LocalInteractionResult::Fail)
    );

    let mut armor_stand = prepare();
    add_entity_at(
        &mut armor_stand,
        95,
        VANILLA_ENTITY_TYPE_ARMOR_STAND_ID,
        position,
    );
    assert_eq!(
        armor_stand.predict_local_block_use(
            clicked,
            ProtocolDirection::Up,
            InteractionHand::MainHand,
            true,
            56,
        ),
        LocalTargetUsePrediction::Result(LocalInteractionResult::Fail)
    );
    let accessors = vanilla_entity_metadata_armor_stand_accessors_for_entity_type(
        VANILLA_ENTITY_TYPE_ARMOR_STAND_ID,
    )
    .unwrap();
    assert!(armor_stand.apply_set_entity_data(SetEntityData {
        id: 95,
        values: vec![EntityDataValue {
            data_id: accessors.client_flags_data_id,
            serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_BYTE_ID,
            value: EntityDataValueKind::Byte(accessors.marker_mask),
        }],
    }));
    assert_eq!(
        armor_stand.predict_local_block_use(
            clicked,
            ProtocolDirection::Up,
            InteractionHand::MainHand,
            true,
            57,
        ),
        LocalTargetUsePrediction::Result(LocalInteractionResult::Fail)
    );

    let mut spectator = prepare();
    add_entity_at(&mut spectator, 96, VANILLA_ENTITY_TYPE_PLAYER_ID, position);
    let spectator_uuid = Uuid::from_u128(96);
    assert_eq!(
        spectator.apply_player_info_update(PlayerInfoUpdate {
            actions: vec![
                PlayerInfoAction::AddPlayer,
                PlayerInfoAction::UpdateGameMode,
            ],
            entries: vec![PlayerInfoEntry {
                profile_id: spectator_uuid,
                profile: Some(GameProfile {
                    uuid: spectator_uuid,
                    name: "spectator".to_string(),
                    properties: Vec::new(),
                }),
                listed: true,
                latency: 0,
                game_mode: GameType::Spectator,
                display_name: None,
                show_hat: true,
                list_order: 0,
                chat_session: None,
            }],
        }),
        1
    );
    assert!(matches!(
        spectator.predict_local_block_use(
            clicked,
            ProtocolDirection::Up,
            InteractionHand::MainHand,
            true,
            58,
        ),
        LocalTargetUsePrediction::Result(LocalInteractionResult::Success(_))
    ));
}

#[test]
fn boat_use_checks_eye_obstruction_and_initial_hull_collision_without_local_shrink() {
    let hit = |location: Vec3d| LocalItemUsePovResult::Block {
        hit: BlockHitResult {
            pos: ProtocolBlockPos {
                x: location.x.floor() as i32,
                y: location.y.floor() as i32,
                z: location.z.floor() as i32,
            },
            direction: ProtocolDirection::Up,
            cursor_x: location.x.fract() as f32,
            cursor_y: location.y.fract() as f32,
            cursor_z: location.z.fract() as f32,
            inside: false,
            world_border_hit: false,
        },
        location,
    };
    let clear_location = Vec3d {
        x: 0.0,
        y: 1.0,
        z: 2.5,
    };
    let mut clear = tool_use_world(crate::vanilla_air_block_state_id());
    clear.set_local_player_pose(crate::LocalPlayerPoseState::default());
    set_main_hand(&mut clear, VANILLA_ITEM_OAK_BOAT_ID);
    let held_before = clear.local_item_in_hand(InteractionHand::MainHand).cloned();
    assert_eq!(
        clear.predict_local_item_use(InteractionHand::MainHand),
        LocalItemUsePrediction::UnsupportedOverride(VanillaItemUseOverride::BoatItem)
    );
    assert_eq!(
        clear.predict_local_item_use_with_pov(
            InteractionHand::MainHand,
            Some(LocalItemUsePovResult::Miss),
        ),
        LocalItemUsePrediction::Result(LocalInteractionResult::Pass)
    );
    assert_eq!(
        clear
            .predict_local_item_use_with_pov(InteractionHand::MainHand, Some(hit(clear_location)),),
        LocalItemUsePrediction::Result(LocalInteractionResult::Success(
            LocalInteractionSuccess::client_item_interaction()
        ))
    );
    assert_eq!(
        clear.local_item_in_hand(InteractionHand::MainHand),
        held_before.as_ref()
    );

    let mut obstructed = clear.clone();
    add_entity_at(
        &mut obstructed,
        20,
        VANILLA_ENTITY_TYPE_ZOMBIE_ID,
        Vec3d::default(),
    );
    assert_eq!(
        obstructed
            .predict_local_item_use_with_pov(InteractionHand::MainHand, Some(hit(clear_location)),),
        LocalItemUsePrediction::Result(LocalInteractionResult::Pass)
    );

    let mut entity_collision = clear.clone();
    add_entity_at(
        &mut entity_collision,
        21,
        VANILLA_ENTITY_TYPE_ZOMBIE_ID,
        clear_location,
    );
    assert_eq!(
        entity_collision
            .predict_local_item_use_with_pov(InteractionHand::MainHand, Some(hit(clear_location)),),
        LocalItemUsePrediction::Result(LocalInteractionResult::Fail)
    );

    let mut border_collision = clear.clone();
    border_collision.apply_set_border_size(bbb_protocol::packets::SetBorderSize { size: 4.0 });
    assert_eq!(
        border_collision.predict_local_item_use_with_pov(
            InteractionHand::MainHand,
            Some(hit(Vec3d {
                x: 1.75,
                y: 1.0,
                z: 0.0,
            })),
        ),
        LocalItemUsePrediction::Result(LocalInteractionResult::Fail)
    );

    let mut block_collision = clear;
    set_block(
        &mut block_collision,
        BlockPos { x: 0, y: 1, z: 2 },
        block_state_id("minecraft:stone", []),
    );
    assert_eq!(
        block_collision.predict_local_item_use_with_pov(
            InteractionHand::MainHand,
            Some(hit(Vec3d {
                x: 0.5,
                y: 1.5,
                z: 2.5,
            })),
        ),
        LocalItemUsePrediction::Result(LocalInteractionResult::Fail)
    );
}

#[test]
fn generated_boat_items_resolve_to_same_resource_boat_entities() {
    let boat_items = VANILLA_ITEM_TYPES
        .iter()
        .filter(|item| item.use_override == Some(VanillaItemUseOverride::BoatItem))
        .collect::<Vec<_>>();
    assert_eq!(boat_items.len(), 20);
    for item in boat_items {
        let entity =
            bbb_protocol::entity_types::vanilla_entity_type_for_resource_id(item.resource_id)
                .expect("boat item resource must have an entity type");
        assert!(crate::entities::is_vanilla_boat_type(entity.id));
    }
}

#[test]
fn bottle_use_does_not_infer_dragon_cloud_owner_from_add_entity_data() {
    let pos = ProtocolBlockPos { x: 0, y: 0, z: 0 };
    let mut world = tool_use_world(block_state_id("minecraft:water", [("level", "0")]));
    add_entity_at(
        &mut world,
        41,
        VANILLA_ENTITY_TYPE_ENDER_DRAGON_ID,
        Vec3d::default(),
    );
    world.apply_add_entity(AddEntity {
        id: 42,
        uuid: Uuid::from_u128(42),
        entity_type_id: VANILLA_ENTITY_TYPE_AREA_EFFECT_CLOUD_ID,
        position: Vec3d::default(),
        delta_movement: Vec3d::default(),
        x_rot: 0.0,
        y_rot: 0.0,
        y_head_rot: 0.0,
        data: 41,
    });
    let cloud_bounds = |world: &WorldStore| {
        world
            .entity_debug_hitbox_targets_at_partial_tick(1.0)
            .into_iter()
            .find(|target| target.entity_id == 42)
            .map(|target| target.bounds)
    };
    let cloud_bounds_before = cloud_bounds(&world).expect("area effect cloud bounds");
    set_main_hand(&mut world, VANILLA_ITEM_GLASS_BOTTLE_ID);

    assert_eq!(
        world.predict_local_item_use_with_pov(
            InteractionHand::MainHand,
            Some(LocalItemUsePovResult::Block {
                hit: BlockHitResult {
                    pos,
                    direction: ProtocolDirection::Up,
                    cursor_x: 0.5,
                    cursor_y: 0.5,
                    cursor_z: 0.5,
                    inside: false,
                    world_border_hit: false,
                },
                location: Vec3d {
                    x: 0.5,
                    y: 0.5,
                    z: 0.5,
                },
            }),
        ),
        LocalItemUsePrediction::Result(LocalInteractionResult::Success(
            LocalInteractionSuccess::bottle_fill(false)
        ))
    );
    assert_eq!(cloud_bounds(&world), Some(cloud_bounds_before));
    assert_eq!(
        player_inventory_item(&world, 0).item_id,
        Some(VANILLA_ITEM_POTION_ID)
    );
}

#[test]
fn wax_and_shears_use_on_predict_client_mutations_and_inventory() {
    let copper_bulb = block_state_id(
        "minecraft:copper_bulb",
        [("lit", "true"), ("powered", "false")],
    );
    let mut world = tool_use_world(copper_bulb);
    world.apply_set_player_inventory(SetPlayerInventory {
        slot: 0,
        item: ItemStackSummary {
            count: 2,
            ..stack(VANILLA_ITEM_HONEYCOMB_ID)
        },
    });
    let pos = BlockPos { x: 0, y: 0, z: 0 };
    let wax_success = LocalTargetUsePrediction::Result(LocalInteractionResult::Success(
        LocalInteractionSuccess {
            sound_action: Some(LocalInteractionSoundAction::BlockPositioned {
                pos,
                sound_resource_id: "minecraft:item.honeycomb.wax_on",
                volume_bits: 1.0_f32.to_bits(),
                pitch_bits: 1.0_f32.to_bits(),
            }),
            ..LocalInteractionSuccess::client_item_interaction()
        },
    ));
    assert_eq!(
        world.predict_local_block_use(
            pos,
            ProtocolDirection::Up,
            InteractionHand::MainHand,
            true,
            3,
        ),
        wax_success
    );
    let waxed = world.probe_block(pos).unwrap();
    assert_eq!(
        waxed.block_name.as_deref(),
        Some("minecraft:waxed_copper_bulb")
    );
    assert_eq!(
        waxed.block_properties,
        BTreeMap::from([
            ("lit".to_string(), "true".to_string()),
            ("powered".to_string(), "false".to_string()),
        ])
    );
    assert_eq!(
        world
            .local_item_in_hand(InteractionHand::MainHand)
            .unwrap()
            .count,
        1
    );

    let cave_vines = block_state_id("minecraft:cave_vines", [("age", "3"), ("berries", "true")]);
    let mut world = tool_use_world(cave_vines);
    set_main_hand(&mut world, VANILLA_ITEM_SHEARS_ID);
    assert_eq!(
        world.predict_local_block_use(
            pos,
            ProtocolDirection::North,
            InteractionHand::MainHand,
            true,
            4,
        ),
        LocalTargetUsePrediction::Result(LocalInteractionResult::Success(
            LocalInteractionSuccess {
                sound_action: Some(LocalInteractionSoundAction::BlockPositioned {
                    pos,
                    sound_resource_id: "minecraft:block.growing_plant.crop",
                    volume_bits: 1.0_f32.to_bits(),
                    pitch_bits: 1.0_f32.to_bits(),
                }),
                ..LocalInteractionSuccess::client_item_interaction()
            }
        ))
    );
    let max_age = world.probe_block(pos).unwrap();
    assert_eq!(max_age.block_name.as_deref(), Some("minecraft:cave_vines"));
    assert_eq!(
        max_age.block_properties.get("age").map(String::as_str),
        Some("25")
    );
    assert_eq!(
        max_age.block_properties.get("berries").map(String::as_str),
        Some("true")
    );
    assert_eq!(
        world.predict_local_block_use(
            pos,
            ProtocolDirection::North,
            InteractionHand::MainHand,
            true,
            5,
        ),
        LocalTargetUsePrediction::Result(LocalInteractionResult::Pass)
    );

    let mut creative_world = tool_use_world(copper_bulb);
    creative_world.apply_game_event(bbb_protocol::packets::GameEvent {
        event_id: bbb_protocol::game_event_types::VANILLA_CLIENT_GAME_EVENT_CHANGE_GAME_MODE_ID,
        param: bbb_protocol::game_types::VANILLA_GAME_TYPE_CREATIVE_ID as f32,
    });
    creative_world.apply_player_abilities(bbb_protocol::packets::PlayerAbilities {
        invulnerable: true,
        flying: false,
        can_fly: true,
        instabuild: true,
        flying_speed: 0.05,
        walking_speed: 0.1,
    });
    creative_world.apply_set_player_inventory(SetPlayerInventory {
        slot: 0,
        item: ItemStackSummary {
            count: 2,
            ..stack(VANILLA_ITEM_HONEYCOMB_ID)
        },
    });
    assert_eq!(
        creative_world.predict_local_block_use(
            pos,
            ProtocolDirection::Up,
            InteractionHand::MainHand,
            true,
            6,
        ),
        wax_success
    );
    assert_eq!(
        creative_world
            .local_item_in_hand(InteractionHand::MainHand)
            .unwrap()
            .count,
        2
    );

    let mut offhand_world = tool_use_world(copper_bulb);
    set_offhand(
        &mut offhand_world,
        ItemStackSummary {
            count: 2,
            ..stack(VANILLA_ITEM_HONEYCOMB_ID)
        },
    );
    assert_eq!(
        offhand_world.predict_local_block_use(
            pos,
            ProtocolDirection::Up,
            InteractionHand::OffHand,
            true,
            7,
        ),
        wax_success
    );
    assert_eq!(
        offhand_world
            .local_item_in_hand(InteractionHand::OffHand)
            .unwrap()
            .count,
        1
    );
}

#[test]
fn end_crystal_and_minecart_use_on_predict_client_gates_and_shrink() {
    let pos = BlockPos { x: 0, y: 0, z: 0 };
    let client_success = LocalTargetUsePrediction::Result(LocalInteractionResult::Success(
        LocalInteractionSuccess::client_item_interaction(),
    ));
    let fail = LocalTargetUsePrediction::Result(LocalInteractionResult::Fail);

    let mut crystal_world = tool_use_world(block_state_id("minecraft:obsidian", []));
    crystal_world.apply_set_player_inventory(SetPlayerInventory {
        slot: 0,
        item: ItemStackSummary {
            count: 2,
            ..stack(VANILLA_ITEM_END_CRYSTAL_ID)
        },
    });
    assert_eq!(
        crystal_world.predict_local_block_use(
            pos,
            ProtocolDirection::Up,
            InteractionHand::MainHand,
            false,
            1,
        ),
        client_success
    );
    assert_eq!(
        crystal_world
            .local_item_in_hand(InteractionHand::MainHand)
            .unwrap()
            .count,
        1
    );
    assert!(crystal_world.apply_block_update(BlockUpdate {
        pos: ProtocolBlockPos { x: 0, y: 1, z: 0 },
        block_state_id: block_state_id("minecraft:stone", []),
    }));
    assert_eq!(
        crystal_world.predict_local_block_use(
            pos,
            ProtocolDirection::Up,
            InteractionHand::MainHand,
            false,
            2,
        ),
        fail
    );

    let mut occupied_crystal_world = tool_use_world(block_state_id("minecraft:obsidian", []));
    add_entity_at(
        &mut occupied_crystal_world,
        8,
        VANILLA_ENTITY_TYPE_AREA_EFFECT_CLOUD_ID,
        Vec3d {
            x: 0.5,
            y: 1.0,
            z: 0.5,
        },
    );
    set_main_hand(&mut occupied_crystal_world, VANILLA_ITEM_END_CRYSTAL_ID);
    assert_eq!(
        occupied_crystal_world.predict_local_block_use(
            pos,
            ProtocolDirection::Up,
            InteractionHand::MainHand,
            false,
            3,
        ),
        fail
    );
    let stone = block_state_id("minecraft:stone", []);
    assert!(occupied_crystal_world.apply_block_update(BlockUpdate {
        pos: ProtocolBlockPos { x: 0, y: 0, z: 0 },
        block_state_id: stone,
    }));
    assert_eq!(
        occupied_crystal_world.predict_local_block_use(
            pos,
            ProtocolDirection::Up,
            InteractionHand::MainHand,
            false,
            4,
        ),
        fail
    );

    let ascending_rail = block_state_id(
        "minecraft:rail",
        [("shape", "ascending_east"), ("waterlogged", "false")],
    );
    let mut minecart_world = tool_use_world(ascending_rail);
    minecart_world.apply_update_tags(UpdateTags {
        registries: vec![RegistryTags {
            registry: "minecraft:block".to_string(),
            tags: vec![TagNetworkPayload {
                tag: "minecraft:rails".to_string(),
                entries: vec![VANILLA_BLOCK_RAIL_ID],
            }],
        }],
    });
    minecart_world.apply_set_player_inventory(SetPlayerInventory {
        slot: 0,
        item: ItemStackSummary {
            count: 2,
            ..stack(VANILLA_ITEM_MINECART_ID)
        },
    });
    assert_eq!(
        minecart_world.predict_local_block_use(
            pos,
            ProtocolDirection::Up,
            InteractionHand::MainHand,
            false,
            5,
        ),
        client_success
    );
    assert_eq!(
        minecart_world
            .local_item_in_hand(InteractionHand::MainHand)
            .unwrap()
            .count,
        1
    );

    minecart_world.apply_update_enabled_features(UpdateEnabledFeatures {
        features: vec![
            "minecraft:vanilla".to_string(),
            "minecraft:minecart_improvements".to_string(),
        ],
    });
    add_entity_at(
        &mut minecart_world,
        9,
        VANILLA_ENTITY_TYPE_AREA_EFFECT_CLOUD_ID,
        Vec3d {
            x: 0.5,
            y: 1.0,
            z: 0.5,
        },
    );
    assert_eq!(
        minecart_world.predict_local_block_use(
            pos,
            ProtocolDirection::Up,
            InteractionHand::MainHand,
            false,
            6,
        ),
        client_success
    );
    set_main_hand(&mut minecart_world, VANILLA_ITEM_MINECART_ID);
    add_entity_at(
        &mut minecart_world,
        10,
        VANILLA_ENTITY_TYPE_MINECART_ID,
        Vec3d {
            x: 0.5,
            y: 1.0,
            z: 0.5,
        },
    );
    assert_eq!(
        minecart_world.predict_local_block_use(
            pos,
            ProtocolDirection::Up,
            InteractionHand::MainHand,
            false,
            7,
        ),
        fail
    );
    assert!(minecart_world.apply_block_update(BlockUpdate {
        pos: ProtocolBlockPos { x: 0, y: 0, z: 0 },
        block_state_id: stone,
    }));
    assert_eq!(
        minecart_world.predict_local_block_use(
            pos,
            ProtocolDirection::Up,
            InteractionHand::MainHand,
            false,
            8,
        ),
        fail
    );
}

#[test]
fn local_mob_interact_predicts_shared_animal_and_ageable_client_branches() {
    let mut world = WorldStore::new();
    add_entity(&mut world, 40, VANILLA_ENTITY_TYPE_CHICKEN_ID);
    add_entity(&mut world, 41, VANILLA_ENTITY_TYPE_SQUID_ID);
    world.apply_update_tags(UpdateTags {
        registries: vec![
            RegistryTags {
                registry: "minecraft:item".to_string(),
                tags: vec![TagNetworkPayload {
                    tag: "minecraft:chicken_food".to_string(),
                    entries: vec![VANILLA_ITEM_WHEAT_SEEDS_ID],
                }],
            },
            RegistryTags {
                registry: "minecraft:entity_type".to_string(),
                tags: vec![TagNetworkPayload {
                    tag: VANILLA_CANNOT_BE_AGE_LOCKED_TAG.to_string(),
                    entries: Vec::new(),
                }],
            },
        ],
    });

    set_main_hand(&mut world, VANILLA_ITEM_WHEAT_SEEDS_ID);
    assert_eq!(
        world.predict_local_entity_interaction(40, InteractionHand::MainHand, false),
        LocalTargetUsePrediction::Result(LocalInteractionResult::Success(
            LocalInteractionSuccess::consume(false)
        ))
    );
    assert_eq!(
        world
            .local_item_in_hand(InteractionHand::MainHand)
            .unwrap()
            .count,
        1
    );

    let (chicken_baby_data_id, _) =
        crate::entities::vanilla_entity_metadata_baby_accessor_for_entity_type(
            VANILLA_ENTITY_TYPE_CHICKEN_ID,
        )
        .unwrap();
    assert!(world.apply_set_entity_data(SetEntityData {
        id: 40,
        values: vec![EntityDataValue {
            data_id: chicken_baby_data_id,
            serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_BOOLEAN_ID,
            value: EntityDataValueKind::Boolean(true),
        }],
    }));
    assert_eq!(
        world.predict_local_entity_interaction(40, InteractionHand::MainHand, false),
        LocalTargetUsePrediction::Result(LocalInteractionResult::Success(
            LocalInteractionSuccess::client_item_interaction()
        ))
    );
    assert!(world
        .local_item_in_hand(InteractionHand::MainHand)
        .is_none());

    let (squid_baby_data_id, _) =
        crate::entities::vanilla_entity_metadata_baby_accessor_for_entity_type(
            VANILLA_ENTITY_TYPE_SQUID_ID,
        )
        .unwrap();
    assert!(world.apply_set_entity_data(SetEntityData {
        id: 41,
        values: vec![EntityDataValue {
            data_id: squid_baby_data_id,
            serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_BOOLEAN_ID,
            value: EntityDataValueKind::Boolean(true),
        }],
    }));
    set_main_hand(&mut world, VANILLA_ITEM_GOLDEN_DANDELION_ID);
    assert_eq!(
        world.predict_local_entity_interaction(41, InteractionHand::MainHand, false),
        age_lock_success()
    );
    let (age_locked_data_id, age_locked_default) =
        vanilla_entity_metadata_age_locked_accessor_for_entity_type(VANILLA_ENTITY_TYPE_SQUID_ID)
            .unwrap();
    assert_eq!(
        world
            .entities
            .metadata_bool_for_entity(41, age_locked_data_id, age_locked_default),
        Some(true)
    );
    assert_eq!(world.entities.age_lock_particle_timer_ticks(41), Some(40));

    set_main_hand(&mut world, VANILLA_ITEM_GOLDEN_DANDELION_ID);
    assert_eq!(
        world.predict_local_entity_interaction(41, InteractionHand::MainHand, false),
        LocalTargetUsePrediction::Result(LocalInteractionResult::Pass)
    );
    assert_eq!(
        world
            .local_item_in_hand(InteractionHand::MainHand)
            .unwrap()
            .count,
        1
    );
}

#[test]
fn local_mob_interact_predicts_villager_and_wandering_trader_client_branches() {
    let mut world = WorldStore::new();
    add_entity(&mut world, 45, VANILLA_ENTITY_TYPE_VILLAGER_ID);
    add_entity(&mut world, 46, VANILLA_ENTITY_TYPE_WANDERING_TRADER_ID);
    add_entity(&mut world, 47, VANILLA_ENTITY_TYPE_VILLAGER_ID);
    add_entity(&mut world, 48, VANILLA_ENTITY_TYPE_WANDERING_TRADER_ID);
    world.apply_update_tags(UpdateTags {
        registries: vec![RegistryTags {
            registry: "minecraft:entity_type".to_string(),
            tags: vec![TagNetworkPayload {
                tag: VANILLA_CANNOT_BE_AGE_LOCKED_TAG.to_string(),
                entries: Vec::new(),
            }],
        }],
    });

    for (entity_id, entity_type_id) in [
        (45, VANILLA_ENTITY_TYPE_VILLAGER_ID),
        (46, VANILLA_ENTITY_TYPE_WANDERING_TRADER_ID),
        (47, VANILLA_ENTITY_TYPE_VILLAGER_ID),
    ] {
        let (baby_data_id, _) =
            crate::entities::vanilla_entity_metadata_baby_accessor_for_entity_type(entity_type_id)
                .unwrap();
        assert!(world.apply_set_entity_data(SetEntityData {
            id: entity_id,
            values: vec![EntityDataValue {
                data_id: baby_data_id,
                serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_BOOLEAN_ID,
                value: EntityDataValueKind::Boolean(true),
            }],
        }));
    }

    let client_success = LocalTargetUsePrediction::Result(LocalInteractionResult::Success(
        LocalInteractionSuccess::client_item_interaction(),
    ));
    assert_eq!(
        world.predict_local_entity_interaction(45, InteractionHand::MainHand, false),
        client_success
    );
    let (unhappy_data_id, unhappy_default) =
        crate::entities::vanilla_entity_metadata_abstract_villager_unhappy_accessor_for_entity_type(
            VANILLA_ENTITY_TYPE_VILLAGER_ID,
        )
        .unwrap();
    assert_eq!(
        world
            .entities
            .metadata_int_for_entity(45, unhappy_data_id, unhappy_default),
        Some(40)
    );
    world.advance_entity_client_animations(1);
    assert_eq!(
        world
            .entities
            .metadata_int_for_entity(45, unhappy_data_id, unhappy_default),
        Some(39),
        "Villager.tick decrements the client-local unhappy counter"
    );
    assert_eq!(
        world.predict_local_entity_interaction(46, InteractionHand::MainHand, false),
        LocalTargetUsePrediction::Result(LocalInteractionResult::Pass)
    );

    let living_accessors =
        crate::entities::vanilla_entity_metadata_living_entity_accessors_for_entity_type(
            VANILLA_ENTITY_TYPE_VILLAGER_ID,
        )
        .unwrap();
    assert!(world.apply_set_entity_data(SetEntityData {
        id: 47,
        values: vec![EntityDataValue {
            data_id: living_accessors.sleeping_pos_data_id,
            serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_OPTIONAL_BLOCK_POS_ID,
            value: EntityDataValueKind::OptionalBlockPos(Some(ProtocolBlockPos {
                x: 1,
                y: 2,
                z: 3,
            })),
        }],
    }));
    set_main_hand(&mut world, VANILLA_ITEM_GOLDEN_DANDELION_ID);
    assert_eq!(
        world.predict_local_entity_interaction(47, InteractionHand::MainHand, false),
        age_lock_success()
    );
    let (age_locked_data_id, age_locked_default) =
        vanilla_entity_metadata_age_locked_accessor_for_entity_type(
            VANILLA_ENTITY_TYPE_VILLAGER_ID,
        )
        .unwrap();
    assert_eq!(
        world
            .entities
            .metadata_bool_for_entity(47, age_locked_data_id, age_locked_default),
        Some(true)
    );
    assert_eq!(
        world
            .entities
            .metadata_int_for_entity(47, unhappy_data_id, unhappy_default),
        Some(0),
        "sleeping villager delegates to AgeableMob before its baby branch"
    );

    assert_eq!(
        world.predict_local_entity_interaction(48, InteractionHand::MainHand, false),
        client_success
    );
    assert!(world.apply_set_entity_data(SetEntityData {
        id: 45,
        values: vec![EntityDataValue {
            data_id: unhappy_data_id,
            serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_INT_ID,
            value: EntityDataValueKind::Int(3),
        }],
    }));
    assert_eq!(
        world
            .entities
            .metadata_int_for_entity(45, unhappy_data_id, unhappy_default),
        Some(3),
        "server metadata correction remains authoritative"
    );
}

#[test]
fn shears_clear_local_leash_connections_inside_vanilla_search_box() {
    let mut world = WorldStore::new();
    add_entity(&mut world, 100, VANILLA_ENTITY_TYPE_COW_ID);
    add_entity_at(
        &mut world,
        101,
        VANILLA_ENTITY_TYPE_COW_ID,
        Vec3d {
            x: 15.0,
            y: 0.0,
            z: 0.0,
        },
    );
    add_entity_at(
        &mut world,
        102,
        VANILLA_ENTITY_TYPE_COW_ID,
        Vec3d {
            x: 17.0,
            y: 0.0,
            z: 0.0,
        },
    );
    for (source_id, dest_id) in [(100, 999), (101, 100), (102, 100)] {
        assert!(world.apply_set_entity_link(SetEntityLink { source_id, dest_id }));
    }
    set_main_hand(&mut world, VANILLA_ITEM_SHEARS_ID);

    assert_eq!(
        world.predict_local_entity_interaction(100, InteractionHand::MainHand, true),
        LocalTargetUsePrediction::Result(LocalInteractionResult::Success(
            LocalInteractionSuccess::client_item_interaction()
        ))
    );
    assert_eq!(world.entities.leash_holder_id(100), None);
    assert_eq!(world.entities.leash_holder_id(101), None);
    assert_eq!(world.entities.leash_holder_id(102), Some(100));
}

#[test]
fn shears_clear_first_eligible_mob_equipment_in_vanilla_slot_order() {
    let mut world = WorldStore::new();
    add_entity(&mut world, 110, VANILLA_ENTITY_TYPE_COW_ID);
    install_test_enchantment_registry(&mut world);
    world.set_test_equippable_defaults(BTreeSet::from([VANILLA_ITEM_SADDLE_ID]).into_iter().map(
        |id| {
            (
                id,
                bbb_protocol::packets::EquippableSummary {
                    can_be_sheared: true,
                    ..crate::item_profiles::test_equippable(
                        bbb_protocol::equipment_slots::EquipmentSlot::Saddle,
                    )
                },
            )
        },
    ));
    let binding_curse = world
        .registry_holder_id_by_resource_id("minecraft:enchantment", "minecraft:binding_curse")
        .unwrap();
    let patched_item = |can_be_sheared, enchantments| ItemStackSummary {
        item_id: Some(VANILLA_ITEM_STONE_ID),
        count: 1,
        component_patch: DataComponentPatchSummary {
            added_type_ids: vec![
                VANILLA_DATA_COMPONENT_EQUIPPABLE_TYPE_ID,
                VANILLA_DATA_COMPONENT_ENCHANTMENTS_TYPE_ID,
            ],
            equippable: Some(equippable(can_be_sheared)),
            enchantments,
            ..DataComponentPatchSummary::default()
        }
        .into(),
    };
    let removed_default = ItemStackSummary {
        component_patch: DataComponentPatchSummary {
            removed_type_ids: vec![VANILLA_DATA_COMPONENT_EQUIPPABLE_TYPE_ID],
            ..DataComponentPatchSummary::default()
        }
        .into(),
        ..stack(VANILLA_ITEM_SADDLE_ID)
    };
    assert!(world.apply_set_equipment(SetEquipment {
        entity_id: 110,
        slots: vec![
            EquipmentSlotUpdate {
                slot: EquipmentSlot::MainHand,
                item: removed_default,
            },
            EquipmentSlotUpdate {
                slot: EquipmentSlot::OffHand,
                item: patched_item(false, Vec::new()),
            },
            EquipmentSlotUpdate {
                slot: EquipmentSlot::Feet,
                item: patched_item(
                    true,
                    vec![ItemEnchantmentSummary {
                        holder_id: binding_curse,
                        level: 1,
                    }],
                ),
            },
            EquipmentSlotUpdate {
                slot: EquipmentSlot::Legs,
                item: stack(VANILLA_ITEM_SADDLE_ID),
            },
        ],
    }));
    set_main_hand(&mut world, VANILLA_ITEM_SHEARS_ID);

    assert_eq!(
        world.predict_local_entity_interaction(110, InteractionHand::MainHand, false),
        LocalTargetUsePrediction::Result(LocalInteractionResult::Success(
            LocalInteractionSuccess::client_item_interaction()
        ))
    );
    assert!(world.equipment_item(110, EquipmentSlot::MainHand).is_some());
    assert!(world.equipment_item(110, EquipmentSlot::OffHand).is_some());
    assert!(world.equipment_item(110, EquipmentSlot::Feet).is_some());
    assert!(world.equipment_item(110, EquipmentSlot::Legs).is_none());
}

#[test]
fn shearing_mob_equipment_honors_vehicle_secondary_and_creative_guards() {
    let mut world = WorldStore::new();
    install_test_enchantment_registry(&mut world);
    for id in [120, 121, 122] {
        add_entity(&mut world, id, VANILLA_ENTITY_TYPE_ZOMBIE_ID);
        assert!(world.apply_set_equipment(SetEquipment {
            entity_id: id,
            slots: vec![EquipmentSlotUpdate {
                slot: EquipmentSlot::Body,
                item: stack(VANILLA_ITEM_SADDLE_ID),
            }],
        }));
    }
    add_entity(&mut world, 123, VANILLA_ENTITY_TYPE_AREA_EFFECT_CLOUD_ID);
    world.set_test_equippable_defaults(BTreeSet::from([VANILLA_ITEM_SADDLE_ID]).into_iter().map(
        |id| {
            (
                id,
                bbb_protocol::packets::EquippableSummary {
                    can_be_sheared: true,
                    ..crate::item_profiles::test_equippable(
                        bbb_protocol::equipment_slots::EquipmentSlot::Saddle,
                    )
                },
            )
        },
    ));
    set_main_hand(&mut world, VANILLA_ITEM_SHEARS_ID);
    assert!(world.apply_set_passengers(SetPassengers {
        vehicle_id: 120,
        passenger_ids: vec![123],
    }));

    for (entity_id, secondary_use_active) in [(120, false), (121, true)] {
        assert_eq!(
            world.predict_local_entity_interaction(
                entity_id,
                InteractionHand::MainHand,
                secondary_use_active,
            ),
            LocalTargetUsePrediction::Result(LocalInteractionResult::Pass)
        );
        assert!(world
            .equipment_item(entity_id, EquipmentSlot::Body)
            .is_some());
    }

    let binding_curse = world
        .registry_holder_id_by_resource_id("minecraft:enchantment", "minecraft:binding_curse")
        .unwrap();
    assert!(world.apply_set_equipment(SetEquipment {
        entity_id: 122,
        slots: vec![EquipmentSlotUpdate {
            slot: EquipmentSlot::Body,
            item: ItemStackSummary {
                component_patch: DataComponentPatchSummary {
                    added_type_ids: vec![VANILLA_DATA_COMPONENT_ENCHANTMENTS_TYPE_ID],
                    enchantments: vec![ItemEnchantmentSummary {
                        holder_id: binding_curse,
                        level: 1,
                    }],
                    ..DataComponentPatchSummary::default()
                }
                .into(),
                ..stack(VANILLA_ITEM_SADDLE_ID)
            },
        }],
    }));
    assert_eq!(
        world.predict_local_entity_interaction(122, InteractionHand::MainHand, false),
        LocalTargetUsePrediction::Result(LocalInteractionResult::Pass)
    );
    world.apply_game_event(bbb_protocol::packets::GameEvent {
        event_id: bbb_protocol::game_event_types::VANILLA_CLIENT_GAME_EVENT_CHANGE_GAME_MODE_ID,
        param: bbb_protocol::game_types::VANILLA_GAME_TYPE_CREATIVE_ID as f32,
    });
    assert_eq!(
        world.predict_local_entity_interaction(122, InteractionHand::MainHand, false),
        LocalTargetUsePrediction::Result(LocalInteractionResult::Success(
            LocalInteractionSuccess::client_item_interaction()
        ))
    );
    assert!(world.equipment_item(122, EquipmentSlot::Body).is_none());
}

#[test]
fn wolf_equipment_shearing_requires_the_synced_owner() {
    let mut world = WorldStore::new();
    add_entity(&mut world, 130, VANILLA_ENTITY_TYPE_WOLF_ID);
    add_entity(&mut world, 131, VANILLA_ENTITY_TYPE_AREA_EFFECT_CLOUD_ID);
    world.set_test_equippable_defaults(BTreeSet::from([VANILLA_ITEM_SADDLE_ID]).into_iter().map(
        |id| {
            (
                id,
                bbb_protocol::packets::EquippableSummary {
                    can_be_sheared: true,
                    ..crate::item_profiles::test_equippable(
                        bbb_protocol::equipment_slots::EquipmentSlot::Saddle,
                    )
                },
            )
        },
    ));
    assert!(world.apply_set_equipment(SetEquipment {
        entity_id: 130,
        slots: vec![EquipmentSlotUpdate {
            slot: EquipmentSlot::Body,
            item: stack(VANILLA_ITEM_SADDLE_ID),
        }],
    }));
    assert!(world.apply_set_passengers(SetPassengers {
        vehicle_id: 130,
        passenger_ids: vec![131],
    }));
    set_main_hand(&mut world, VANILLA_ITEM_SHEARS_ID);

    assert!(matches!(
        world.predict_local_entity_interaction(130, InteractionHand::MainHand, false),
        LocalTargetUsePrediction::UnsupportedEntityInteract(_)
    ));
    assert!(world.equipment_item(130, EquipmentSlot::Body).is_some());

    let local_player = Uuid::from_u128(0xabc);
    world.set_local_player_profile_id(local_player);
    let accessors = vanilla_entity_metadata_tamable_animal_accessors_for_entity_type(
        VANILLA_ENTITY_TYPE_WOLF_ID,
    )
    .unwrap();
    assert!(world.apply_set_entity_data(SetEntityData {
        id: 130,
        values: vec![EntityDataValue {
            data_id: accessors.owner_data_id,
            serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_OPTIONAL_LIVING_ENTITY_REFERENCE_ID,
            value: EntityDataValueKind::OptionalLivingEntityReference(Some(local_player)),
        }],
    }));
    assert_eq!(
        world.predict_local_entity_interaction(130, InteractionHand::MainHand, false),
        LocalTargetUsePrediction::Result(LocalInteractionResult::Success(
            LocalInteractionSuccess::client_item_interaction()
        ))
    );
    assert!(world.equipment_item(130, EquipmentSlot::Body).is_none());
}

#[test]
fn default_mob_interaction_predicts_pass_spawn_egg_and_local_unleash() {
    let mut world = WorldStore::new();
    add_entity(&mut world, 4, VANILLA_ENTITY_TYPE_ZOMBIE_ID);

    assert_eq!(
        world.predict_local_entity_interaction(4, InteractionHand::MainHand, false),
        LocalTargetUsePrediction::Result(LocalInteractionResult::Pass)
    );
    set_main_hand(&mut world, VANILLA_ITEM_STONE_ID);
    assert_eq!(
        world.predict_local_entity_interaction(4, InteractionHand::MainHand, false),
        LocalTargetUsePrediction::Result(LocalInteractionResult::Pass)
    );

    set_main_hand(&mut world, VANILLA_ITEM_CHICKEN_SPAWN_EGG_ID);
    assert_eq!(
        world.predict_local_entity_interaction(4, InteractionHand::MainHand, false),
        LocalTargetUsePrediction::Result(LocalInteractionResult::Success(
            LocalInteractionSuccess::server_item_interaction()
        ))
    );
    add_entity(&mut world, 5, VANILLA_ENTITY_TYPE_ZOMBIE_ID);
    let living_accessors =
        crate::entities::vanilla_entity_metadata_living_entity_accessors_for_entity_type(
            VANILLA_ENTITY_TYPE_ZOMBIE_ID,
        )
        .unwrap();
    assert!(world.apply_set_entity_data(SetEntityData {
        id: 5,
        values: vec![EntityDataValue {
            data_id: living_accessors.health_data_id,
            serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_FLOAT_ID,
            value: EntityDataValueKind::Float(f32::NAN),
        }],
    }));
    assert_eq!(
        world.predict_local_entity_interaction(5, InteractionHand::MainHand, false),
        LocalTargetUsePrediction::Result(LocalInteractionResult::Pass),
        "LivingEntity.isAlive uses `health > 0`, so NaN must fail closed"
    );

    set_main_hand(&mut world, VANILLA_ITEM_LEAD_ID);
    assert_eq!(
        world.predict_local_entity_interaction(4, InteractionHand::MainHand, false),
        LocalTargetUsePrediction::UnsupportedEntityInteract(VANILLA_DEFAULT_ENTITY_INTERACT_OWNER)
    );

    world.local_player_id = Some(7);
    assert!(world.apply_set_entity_link(SetEntityLink {
        source_id: 4,
        dest_id: 7,
    }));
    world.apply_set_player_inventory(SetPlayerInventory {
        slot: 0,
        item: ItemStackSummary::empty(),
    });
    assert_eq!(
        world.predict_local_entity_interaction(4, InteractionHand::MainHand, false),
        LocalTargetUsePrediction::Result(LocalInteractionResult::Success(
            LocalInteractionSuccess::client_without_item()
        ))
    );
}

#[test]
fn minecart_interaction_predicts_client_results_from_local_guards() {
    let mut world = WorldStore::new();
    add_entity(&mut world, 10, VANILLA_ENTITY_TYPE_MINECART_ID);
    add_entity(&mut world, 11, VANILLA_ENTITY_TYPE_CHEST_MINECART_ID);
    add_entity(&mut world, 12, VANILLA_ENTITY_TYPE_HOPPER_MINECART_ID);
    add_entity(&mut world, 13, VANILLA_ENTITY_TYPE_FURNACE_MINECART_ID);
    add_entity(
        &mut world,
        14,
        VANILLA_ENTITY_TYPE_COMMAND_BLOCK_MINECART_ID,
    );
    add_entity(&mut world, 99, VANILLA_ENTITY_TYPE_AREA_EFFECT_CLOUD_ID);

    let success = LocalTargetUsePrediction::Result(LocalInteractionResult::Success(
        LocalInteractionSuccess::client_item_interaction(),
    ));
    assert_eq!(
        world.predict_local_entity_interaction(10, InteractionHand::MainHand, false),
        success
    );
    assert_eq!(
        world.predict_local_entity_interaction(10, InteractionHand::MainHand, true),
        LocalTargetUsePrediction::Result(LocalInteractionResult::Pass)
    );
    assert!(world.apply_set_passengers(SetPassengers {
        vehicle_id: 10,
        passenger_ids: vec![99],
    }));
    assert_eq!(
        world.predict_local_entity_interaction(10, InteractionHand::MainHand, false),
        LocalTargetUsePrediction::Result(LocalInteractionResult::Pass)
    );

    for entity_id in [11, 12, 13] {
        assert_eq!(
            world.predict_local_entity_interaction(entity_id, InteractionHand::MainHand, false),
            success
        );
    }

    world.set_local_player_pose(crate::LocalPlayerPoseState {
        position: Vec3d {
            x: -2.0,
            y: 5.0,
            z: 3.0,
        },
        ..crate::LocalPlayerPoseState::default()
    });
    world.apply_update_tags(UpdateTags {
        registries: vec![RegistryTags {
            registry: "minecraft:item".to_string(),
            tags: vec![TagNetworkPayload {
                tag: "minecraft:furnace_minecart_fuel".to_string(),
                entries: vec![VANILLA_ITEM_COAL_ID],
            }],
        }],
    });
    world.apply_set_player_inventory(SetPlayerInventory {
        slot: 0,
        item: ItemStackSummary {
            count: 9,
            ..stack(VANILLA_ITEM_COAL_ID)
        },
    });
    for expected_fuel in (3_600..=28_800).step_by(3_600) {
        assert_eq!(
            world.predict_local_entity_interaction(13, InteractionHand::MainHand, false),
            success
        );
        let furnace = world
            .probe_entity(13)
            .and_then(|entity| entity.minecart_furnace)
            .expect("furnace minecart state");
        assert_eq!(furnace.fuel, expected_fuel);
        assert_eq!(
            furnace.push,
            crate::EntityVec3 {
                x: 2.0,
                y: 0.0,
                z: -3.0,
            }
        );
    }
    assert_eq!(player_inventory_item(&world, 0).count, 1);
    assert_eq!(
        world.predict_local_entity_interaction(13, InteractionHand::MainHand, false),
        success
    );
    assert_eq!(
        world
            .probe_entity(13)
            .and_then(|entity| entity.minecart_furnace)
            .expect("furnace minecart state")
            .fuel,
        28_800
    );
    assert_eq!(player_inventory_item(&world, 0).count, 1);

    let serialized = serde_json::to_value(&world).expect("serialize furnace minecart state");
    let mut restored: WorldStore =
        serde_json::from_value(serialized).expect("restore furnace minecart state");
    assert_eq!(
        restored
            .probe_entity(13)
            .and_then(|entity| entity.minecart_furnace)
            .expect("restored furnace minecart state")
            .fuel,
        28_800
    );
    add_entity(&mut restored, 13, VANILLA_ENTITY_TYPE_FURNACE_MINECART_ID);
    assert_eq!(
        restored
            .probe_entity(13)
            .and_then(|entity| entity.minecart_furnace),
        Some(crate::MinecartFurnaceState::default())
    );

    assert_eq!(
        world.predict_local_entity_interaction(14, InteractionHand::MainHand, false),
        LocalTargetUsePrediction::Result(LocalInteractionResult::Pass)
    );
    world.local_player_id = Some(7);
    assert!(world.apply_entity_event(EntityEvent {
        entity_id: 7,
        event_id: 26,
    }));
    world.apply_player_abilities(bbb_protocol::packets::PlayerAbilities {
        invulnerable: true,
        flying: false,
        can_fly: true,
        instabuild: true,
        flying_speed: 0.05,
        walking_speed: 0.1,
    });
    assert_eq!(
        world.predict_local_entity_interaction(14, InteractionHand::MainHand, false),
        success
    );
}

#[test]
fn boat_interaction_predicts_riding_container_and_submerged_guards() {
    let mut world = WorldStore::new();
    add_entity(&mut world, 20, VANILLA_ENTITY_TYPE_OAK_BOAT_ID);
    add_entity(&mut world, 21, VANILLA_ENTITY_TYPE_OAK_CHEST_BOAT_ID);

    let success = LocalTargetUsePrediction::Result(LocalInteractionResult::Success(
        LocalInteractionSuccess::client_item_interaction(),
    ));
    assert_eq!(
        world.predict_local_entity_interaction(20, InteractionHand::MainHand, false),
        success
    );
    assert_eq!(
        world.predict_local_entity_interaction(20, InteractionHand::MainHand, true),
        LocalTargetUsePrediction::Result(LocalInteractionResult::Pass)
    );
    world
        .entities
        .with_boat_movement_transform_and_support_mut(20, |movement, _, _| {
            movement.out_of_control_ticks = 60.0;
        })
        .unwrap();
    assert_eq!(
        world.predict_local_entity_interaction(20, InteractionHand::MainHand, false),
        LocalTargetUsePrediction::Result(LocalInteractionResult::Pass)
    );

    for secondary_use_active in [false, true] {
        assert_eq!(
            world.predict_local_entity_interaction(
                21,
                InteractionHand::MainHand,
                secondary_use_active
            ),
            success
        );
    }
    world
        .entities
        .with_boat_movement_transform_and_support_mut(21, |movement, _, _| {
            movement.out_of_control_ticks = 60.0;
        })
        .unwrap();
    assert_eq!(
        world.predict_local_entity_interaction(21, InteractionHand::MainHand, false),
        success
    );

    set_main_hand(&mut world, VANILLA_ITEM_LEAD_ID);
    assert_eq!(
        world.predict_local_entity_interaction(20, InteractionHand::MainHand, false),
        LocalTargetUsePrediction::UnsupportedEntityInteract(VANILLA_DEFAULT_ENTITY_INTERACT_OWNER)
    );
}

#[test]
fn interaction_entity_response_selects_consume_or_client_success() {
    let mut world = WorldStore::new();
    add_entity(&mut world, 30, VANILLA_ENTITY_TYPE_INTERACTION_ID);
    let accessors = vanilla_entity_metadata_interaction_accessors_for_entity_type(
        VANILLA_ENTITY_TYPE_INTERACTION_ID,
    )
    .unwrap();

    assert_eq!(
        world.predict_local_entity_interaction(30, InteractionHand::MainHand, false),
        LocalTargetUsePrediction::Result(LocalInteractionResult::Success(
            LocalInteractionSuccess::consume(false)
        ))
    );
    assert!(world.apply_set_entity_data(SetEntityData {
        id: 30,
        values: vec![EntityDataValue {
            data_id: accessors.response_data_id,
            serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_BOOLEAN_ID,
            value: EntityDataValueKind::Boolean(true),
        }],
    }));
    assert_eq!(
        world.predict_local_entity_interaction(30, InteractionHand::MainHand, false),
        LocalTargetUsePrediction::Result(LocalInteractionResult::Success(
            LocalInteractionSuccess::client_item_interaction()
        ))
    );
}

#[test]
fn item_frame_interaction_uses_synced_frame_and_held_item_presence() {
    let mut world = WorldStore::new();
    add_entity(&mut world, 40, VANILLA_ENTITY_TYPE_ITEM_FRAME_ID);
    add_entity(&mut world, 41, VANILLA_ENTITY_TYPE_ITEM_FRAME_ID);
    let accessors = crate::entities::vanilla_entity_metadata_item_frame_accessors_for_entity_type(
        VANILLA_ENTITY_TYPE_ITEM_FRAME_ID,
    )
    .unwrap();
    assert!(world.apply_set_entity_data(SetEntityData {
        id: 41,
        values: vec![EntityDataValue {
            data_id: accessors.item_data_id,
            serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_ITEM_STACK_ID,
            value: EntityDataValueKind::ItemStack(stack(VANILLA_ITEM_STONE_ID)),
        }],
    }));

    assert_eq!(
        world.predict_local_entity_interaction(40, InteractionHand::MainHand, false),
        LocalTargetUsePrediction::Result(LocalInteractionResult::Pass)
    );
    assert_eq!(
        world.predict_local_entity_interaction(41, InteractionHand::MainHand, false),
        LocalTargetUsePrediction::Result(LocalInteractionResult::Success(
            LocalInteractionSuccess::client_item_interaction()
        ))
    );
    set_main_hand(&mut world, VANILLA_ITEM_STONE_ID);
    assert_eq!(
        world.predict_local_entity_interaction(40, InteractionHand::MainHand, false),
        LocalTargetUsePrediction::Result(LocalInteractionResult::Success(
            LocalInteractionSuccess::client_item_interaction()
        ))
    );
}

#[test]
fn armor_stand_interaction_predicts_marker_and_name_tag_routing() {
    let mut world = WorldStore::new();
    add_entity(&mut world, 50, VANILLA_ENTITY_TYPE_ARMOR_STAND_ID);
    let accessors = vanilla_entity_metadata_armor_stand_accessors_for_entity_type(
        VANILLA_ENTITY_TYPE_ARMOR_STAND_ID,
    )
    .unwrap();

    assert_eq!(
        world.predict_local_entity_interaction(50, InteractionHand::MainHand, false),
        LocalTargetUsePrediction::Result(LocalInteractionResult::Success(
            LocalInteractionSuccess::server_item_interaction()
        ))
    );
    set_main_hand(&mut world, VANILLA_ITEM_NAME_TAG_ID);
    assert_eq!(
        world.predict_local_entity_interaction(50, InteractionHand::MainHand, false),
        LocalTargetUsePrediction::Result(LocalInteractionResult::Pass)
    );

    world.apply_set_player_inventory(SetPlayerInventory {
        slot: 0,
        item: ItemStackSummary::empty(),
    });
    assert!(world.apply_set_entity_data(SetEntityData {
        id: 50,
        values: vec![EntityDataValue {
            data_id: accessors.client_flags_data_id,
            serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_BYTE_ID,
            value: EntityDataValueKind::Byte(accessors.marker_mask),
        }],
    }));
    assert_eq!(
        world.predict_local_entity_interaction(50, InteractionHand::MainHand, false),
        LocalTargetUsePrediction::Result(LocalInteractionResult::Pass)
    );
}

#[test]
fn leash_knot_interaction_is_unconditional_client_success() {
    let mut world = WorldStore::new();
    add_entity(&mut world, 60, VANILLA_ENTITY_TYPE_LEASH_KNOT_ID);

    for secondary_use_active in [false, true] {
        assert_eq!(
            world.predict_local_entity_interaction(
                60,
                InteractionHand::MainHand,
                secondary_use_active
            ),
            LocalTargetUsePrediction::Result(LocalInteractionResult::Success(
                LocalInteractionSuccess::client_item_interaction()
            ))
        );
    }
    set_main_hand(&mut world, VANILLA_ITEM_SHEARS_ID);
    assert_eq!(
        world.predict_local_entity_interaction(60, InteractionHand::MainHand, false),
        LocalTargetUsePrediction::Result(LocalInteractionResult::Success(
            LocalInteractionSuccess::client_item_interaction()
        ))
    );
}

#[test]
fn persistence_only_mob_interact_overrides_delegate_to_mob_flow() {
    let mut world = WorldStore::new();
    for (entity_id, entity_type_id) in [
        (70, VANILLA_ENTITY_TYPE_CAMEL_HUSK_ID),
        (71, VANILLA_ENTITY_TYPE_ZOMBIE_HORSE_ID),
        (72, VANILLA_ENTITY_TYPE_NAUTILUS_ID),
        (73, VANILLA_ENTITY_TYPE_ZOMBIE_NAUTILUS_ID),
    ] {
        add_entity(&mut world, entity_id, entity_type_id);
    }

    assert_eq!(
        world.predict_local_entity_interaction(71, InteractionHand::MainHand, false),
        LocalTargetUsePrediction::Result(LocalInteractionResult::Success(
            LocalInteractionSuccess::client_item_interaction()
        ))
    );
    for entity_id in [72, 73] {
        assert_eq!(
            world.predict_local_entity_interaction(entity_id, InteractionHand::MainHand, false),
            LocalTargetUsePrediction::Result(LocalInteractionResult::Pass)
        );
    }

    set_main_hand(&mut world, VANILLA_ITEM_CHICKEN_SPAWN_EGG_ID);
    assert_eq!(
        world.predict_local_entity_interaction(70, InteractionHand::MainHand, false),
        LocalTargetUsePrediction::Result(LocalInteractionResult::Success(
            LocalInteractionSuccess::server_item_interaction()
        ))
    );
}

#[test]
fn nautilus_mob_interaction_uses_synced_tame_age_health_and_food_state() {
    let mut world = WorldStore::new();
    add_entity(&mut world, 90, VANILLA_ENTITY_TYPE_NAUTILUS_ID);
    add_entity(&mut world, 91, VANILLA_ENTITY_TYPE_ZOMBIE_NAUTILUS_ID);
    let tame_accessors = vanilla_entity_metadata_tamable_animal_accessors_for_entity_type(
        VANILLA_ENTITY_TYPE_NAUTILUS_ID,
    )
    .unwrap();
    let living_accessors =
        crate::entities::vanilla_entity_metadata_living_entity_accessors_for_entity_type(
            VANILLA_ENTITY_TYPE_NAUTILUS_ID,
        )
        .unwrap();
    let (baby_data_id, baby_default) =
        crate::entities::vanilla_entity_metadata_baby_accessor_for_entity_type(
            VANILLA_ENTITY_TYPE_NAUTILUS_ID,
        )
        .unwrap();
    let (age_locked_data_id, age_locked_default) =
        vanilla_entity_metadata_age_locked_accessor_for_entity_type(
            VANILLA_ENTITY_TYPE_NAUTILUS_ID,
        )
        .unwrap();
    assert!(!baby_default);
    assert!(!age_locked_default);

    let client_success = LocalTargetUsePrediction::Result(LocalInteractionResult::Success(
        LocalInteractionSuccess::client_item_interaction(),
    ));
    let consume = LocalTargetUsePrediction::Result(LocalInteractionResult::Success(
        LocalInteractionSuccess::consume(false),
    ));
    assert_eq!(
        world.predict_local_entity_interaction(90, InteractionHand::MainHand, false),
        LocalTargetUsePrediction::Result(LocalInteractionResult::Pass)
    );
    set_main_hand(&mut world, VANILLA_ITEM_STONE_ID);
    assert_eq!(
        world.predict_local_entity_interaction(90, InteractionHand::MainHand, false),
        LocalTargetUsePrediction::UnsupportedEntityInteract(VANILLA_NAUTILUS_INTERACT_OWNER)
    );
    world.apply_update_tags(UpdateTags {
        registries: vec![
            RegistryTags {
                registry: "minecraft:item".to_string(),
                tags: vec![
                    TagNetworkPayload {
                        tag: VANILLA_NAUTILUS_TAMING_ITEMS_TAG.to_string(),
                        entries: vec![VANILLA_ITEM_STONE_ID],
                    },
                    TagNetworkPayload {
                        tag: VANILLA_NAUTILUS_FOOD_TAG.to_string(),
                        entries: vec![VANILLA_ITEM_APPLE_ID],
                    },
                    TagNetworkPayload {
                        tag: VANILLA_NAUTILUS_BUCKET_FOOD_TAG.to_string(),
                        entries: Vec::new(),
                    },
                ],
            },
            RegistryTags {
                registry: "minecraft:entity_type".to_string(),
                tags: vec![TagNetworkPayload {
                    tag: VANILLA_CANNOT_BE_AGE_LOCKED_TAG.to_string(),
                    entries: Vec::new(),
                }],
            },
        ],
    });
    let eating_success = LocalTargetUsePrediction::Result(LocalInteractionResult::Success(
        LocalInteractionSuccess::client_item_interaction(),
    ));
    assert_eq!(
        world.predict_local_entity_interaction(90, InteractionHand::MainHand, false),
        eating_success
    );
    set_main_hand(&mut world, VANILLA_ITEM_STONE_ID);
    assert_eq!(
        world.predict_local_entity_interaction(91, InteractionHand::MainHand, false),
        eating_success
    );
    assert!(world.apply_set_entity_data(SetEntityData {
        id: 90,
        values: vec![EntityDataValue {
            data_id: living_accessors.health_data_id,
            serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_FLOAT_ID,
            value: EntityDataValueKind::Float(15.0),
        }],
    }));
    set_main_hand(&mut world, VANILLA_ITEM_STONE_ID);
    assert_eq!(
        world.predict_local_entity_interaction(90, InteractionHand::MainHand, false),
        consume
    );

    set_main_hand(&mut world, VANILLA_ITEM_RED_DYE_ID);
    assert_eq!(
        world.predict_local_entity_interaction(90, InteractionHand::MainHand, false),
        LocalTargetUsePrediction::Result(LocalInteractionResult::Pass)
    );
    assert!(world.apply_set_entity_data(SetEntityData {
        id: 90,
        values: vec![EntityDataValue {
            data_id: tame_accessors.flags_data_id,
            serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_BYTE_ID,
            value: EntityDataValueKind::Byte(tame_accessors.tame_mask),
        }],
    }));
    assert_eq!(
        world.predict_local_entity_interaction(90, InteractionHand::MainHand, true),
        client_success
    );
    assert_eq!(
        world.predict_local_entity_interaction(90, InteractionHand::MainHand, false),
        client_success
    );
    set_main_hand(&mut world, VANILLA_ITEM_APPLE_ID);
    assert_eq!(
        world.predict_local_entity_interaction(90, InteractionHand::MainHand, false),
        consume
    );
    assert!(world.apply_set_entity_data(SetEntityData {
        id: 90,
        values: vec![EntityDataValue {
            data_id: living_accessors.health_data_id,
            serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_FLOAT_ID,
            value: EntityDataValueKind::Float(1.0),
        }],
    }));
    set_main_hand(&mut world, VANILLA_ITEM_APPLE_ID);
    assert_eq!(
        world.predict_local_entity_interaction(90, InteractionHand::MainHand, false),
        eating_success
    );
    set_main_hand(&mut world, VANILLA_ITEM_STONE_ID);
    assert_eq!(
        world.predict_local_entity_interaction(90, InteractionHand::MainHand, false),
        client_success
    );

    assert!(world.apply_set_entity_data(SetEntityData {
        id: 90,
        values: vec![
            EntityDataValue {
                data_id: baby_data_id,
                serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_BOOLEAN_ID,
                value: EntityDataValueKind::Boolean(true),
            },
            EntityDataValue {
                data_id: age_locked_data_id,
                serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_BOOLEAN_ID,
                value: EntityDataValueKind::Boolean(true),
            },
        ],
    }));
    set_main_hand(&mut world, VANILLA_ITEM_APPLE_ID);
    assert_eq!(
        world.predict_local_entity_interaction(90, InteractionHand::MainHand, false),
        consume
    );
    set_main_hand(&mut world, VANILLA_ITEM_GOLDEN_DANDELION_ID);
    assert_eq!(
        world.predict_local_entity_interaction(90, InteractionHand::MainHand, false),
        age_lock_success()
    );
    assert_eq!(
        world.predict_local_entity_interaction(90, InteractionHand::MainHand, false),
        LocalTargetUsePrediction::Result(LocalInteractionResult::Pass)
    );
    assert_eq!(
        world
            .entities
            .metadata_bool_for_entity(90, age_locked_data_id, age_locked_default),
        Some(false)
    );
    set_main_hand(&mut world, VANILLA_ITEM_APPLE_ID);
    assert_eq!(
        world.predict_local_entity_interaction(90, InteractionHand::MainHand, false),
        eating_success
    );
}

#[test]
fn creeper_interaction_uses_synced_igniter_tag_for_client_success() {
    let mut world = WorldStore::new();
    add_entity(&mut world, 93, VANILLA_ENTITY_TYPE_CREEPER_ID);
    assert_eq!(
        world.predict_local_entity_interaction(93, InteractionHand::MainHand, false),
        LocalTargetUsePrediction::Result(LocalInteractionResult::Pass)
    );

    set_main_hand(&mut world, VANILLA_ITEM_STONE_ID);
    assert_eq!(
        world.predict_local_entity_interaction(93, InteractionHand::MainHand, false),
        LocalTargetUsePrediction::UnsupportedEntityInteract(VANILLA_CREEPER_MOB_INTERACT_OWNER)
    );
    world.apply_update_tags(UpdateTags {
        registries: vec![RegistryTags {
            registry: "minecraft:item".to_string(),
            tags: vec![TagNetworkPayload {
                tag: VANILLA_CREEPER_IGNITERS_TAG.to_string(),
                entries: vec![VANILLA_ITEM_STONE_ID],
            }],
        }],
    });
    assert_eq!(
        world.predict_local_entity_interaction(93, InteractionHand::MainHand, false),
        LocalTargetUsePrediction::Result(LocalInteractionResult::Success(
            LocalInteractionSuccess {
                sound_action: Some(LocalInteractionSoundAction::CreeperIgnition {
                    entity_id: 93,
                    fire_charge: false,
                }),
                ..LocalInteractionSuccess::client_item_interaction()
            }
        ))
    );

    set_main_hand(&mut world, VANILLA_ITEM_RED_DYE_ID);
    assert_eq!(
        world.predict_local_entity_interaction(93, InteractionHand::MainHand, false),
        LocalTargetUsePrediction::Result(LocalInteractionResult::Pass)
    );
    set_main_hand(&mut world, VANILLA_ITEM_APPLE_ID);
    assert_eq!(
        world.predict_local_entity_interaction(93, InteractionHand::MainHand, false),
        LocalTargetUsePrediction::Result(LocalInteractionResult::Pass)
    );
}

#[test]
fn zombie_horse_mob_interaction_uses_synced_age_tame_vehicle_and_food_state() {
    let mut world = WorldStore::new();
    add_entity(&mut world, 81, VANILLA_ENTITY_TYPE_ZOMBIE_HORSE_ID);
    add_entity(&mut world, 82, VANILLA_ENTITY_TYPE_AREA_EFFECT_CLOUD_ID);
    let horse_accessors = vanilla_entity_metadata_abstract_horse_accessors_for_entity_type(
        VANILLA_ENTITY_TYPE_ZOMBIE_HORSE_ID,
    )
    .unwrap();
    let (baby_data_id, baby_default) =
        crate::entities::vanilla_entity_metadata_baby_accessor_for_entity_type(
            VANILLA_ENTITY_TYPE_ZOMBIE_HORSE_ID,
        )
        .unwrap();
    let (age_locked_data_id, age_locked_default) =
        vanilla_entity_metadata_age_locked_accessor_for_entity_type(
            VANILLA_ENTITY_TYPE_ZOMBIE_HORSE_ID,
        )
        .unwrap();
    assert!(!baby_default);
    assert!(!age_locked_default);

    let client_success = LocalTargetUsePrediction::Result(LocalInteractionResult::Success(
        LocalInteractionSuccess::client_item_interaction(),
    ));
    assert_eq!(
        world.predict_local_entity_interaction(81, InteractionHand::MainHand, false),
        client_success
    );

    assert!(world.apply_set_entity_data(SetEntityData {
        id: 81,
        values: vec![EntityDataValue {
            data_id: horse_accessors.flags_data_id,
            serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_BYTE_ID,
            value: EntityDataValueKind::Byte(horse_accessors.tame_mask),
        }],
    }));
    set_main_hand(&mut world, VANILLA_ITEM_STONE_ID);
    assert_eq!(
        world.predict_local_entity_interaction(81, InteractionHand::MainHand, true),
        client_success
    );

    assert_eq!(
        world.predict_local_entity_interaction(81, InteractionHand::MainHand, false),
        LocalTargetUsePrediction::UnsupportedEntityInteract(VANILLA_ZOMBIE_HORSE_INTERACT_OWNER)
    );
    world.apply_update_tags(UpdateTags {
        registries: vec![RegistryTags {
            registry: "minecraft:item".to_string(),
            tags: vec![TagNetworkPayload {
                tag: VANILLA_ZOMBIE_HORSE_FOOD_TAG.to_string(),
                entries: vec![VANILLA_ITEM_STONE_ID],
            }],
        }],
    });
    assert_eq!(
        world.predict_local_entity_interaction(81, InteractionHand::MainHand, false),
        LocalTargetUsePrediction::Result(LocalInteractionResult::Success(
            LocalInteractionSuccess::server_item_interaction()
        ))
    );

    set_main_hand(&mut world, VANILLA_ITEM_RED_DYE_ID);
    assert_eq!(
        world.predict_local_entity_interaction(81, InteractionHand::MainHand, false),
        client_success
    );
    assert!(world.apply_set_entity_data(SetEntityData {
        id: 81,
        values: vec![EntityDataValue {
            data_id: horse_accessors.flags_data_id,
            serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_BYTE_ID,
            value: EntityDataValueKind::Byte(horse_accessors.flags_default),
        }],
    }));
    assert_eq!(
        world.predict_local_entity_interaction(81, InteractionHand::MainHand, false),
        client_success
    );

    assert!(world.apply_set_entity_data(SetEntityData {
        id: 81,
        values: vec![EntityDataValue {
            data_id: baby_data_id,
            serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_BOOLEAN_ID,
            value: EntityDataValueKind::Boolean(true),
        }],
    }));
    set_main_hand(&mut world, VANILLA_ITEM_STONE_ID);
    assert_eq!(
        world.predict_local_entity_interaction(81, InteractionHand::MainHand, false),
        LocalTargetUsePrediction::Result(LocalInteractionResult::Success(
            LocalInteractionSuccess::server_item_interaction()
        ))
    );
    assert!(world.apply_set_entity_data(SetEntityData {
        id: 81,
        values: vec![EntityDataValue {
            data_id: age_locked_data_id,
            serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_BOOLEAN_ID,
            value: EntityDataValueKind::Boolean(true),
        }],
    }));
    assert_eq!(
        world.predict_local_entity_interaction(81, InteractionHand::MainHand, false),
        LocalTargetUsePrediction::Result(LocalInteractionResult::Success(
            LocalInteractionSuccess::server_item_interaction()
        ))
    );

    assert!(world.apply_set_entity_data(SetEntityData {
        id: 81,
        values: vec![EntityDataValue {
            data_id: baby_data_id,
            serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_BOOLEAN_ID,
            value: EntityDataValueKind::Boolean(false),
        }],
    }));
    assert!(world.apply_set_passengers(SetPassengers {
        vehicle_id: 81,
        passenger_ids: vec![82],
    }));
    assert_eq!(
        world.predict_local_entity_interaction(81, InteractionHand::MainHand, false),
        LocalTargetUsePrediction::Result(LocalInteractionResult::Success(
            LocalInteractionSuccess::consume(false)
        ))
    );
    set_main_hand(&mut world, VANILLA_ITEM_RED_DYE_ID);
    assert_eq!(
        world.predict_local_entity_interaction(81, InteractionHand::MainHand, true),
        LocalTargetUsePrediction::Result(LocalInteractionResult::Pass)
    );
    set_main_hand(&mut world, VANILLA_ITEM_APPLE_ID);
    assert_eq!(
        world.predict_local_entity_interaction(81, InteractionHand::MainHand, true),
        LocalTargetUsePrediction::Result(LocalInteractionResult::Pass)
    );
}

#[test]
fn zombie_horse_food_applies_client_heal_consumption_and_particle() {
    let mut world = WorldStore::new();
    add_entity_at(
        &mut world,
        81,
        VANILLA_ENTITY_TYPE_ZOMBIE_HORSE_ID,
        Vec3d {
            x: 1.0,
            y: 2.0,
            z: 3.0,
        },
    );
    world.apply_update_tags(UpdateTags {
        registries: vec![RegistryTags {
            registry: "minecraft:item".to_string(),
            tags: vec![TagNetworkPayload {
                tag: VANILLA_ZOMBIE_HORSE_FOOD_TAG.to_string(),
                entries: vec![VANILLA_ITEM_RED_MUSHROOM_ID, VANILLA_ITEM_WHEAT_ID],
            }],
        }],
    });
    let (baby_data_id, _) = crate::entities::vanilla_entity_metadata_baby_accessor_for_entity_type(
        VANILLA_ENTITY_TYPE_ZOMBIE_HORSE_ID,
    )
    .unwrap();
    assert!(world.apply_set_entity_data(SetEntityData {
        id: 81,
        values: vec![EntityDataValue {
            data_id: baby_data_id,
            serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_BOOLEAN_ID,
            value: EntityDataValueKind::Boolean(true),
        }],
    }));
    set_main_hand(&mut world, VANILLA_ITEM_WHEAT_ID);

    let particle_action = LocalInteractionParticleAction::EquineBabyFeeding {
        position_bits: [
            4_609_177_415_567_282_494,
            4_614_194_517_259_326_966,
            4_614_298_779_471_186_992,
        ],
    };
    assert_eq!(
        world.predict_local_entity_interaction(81, InteractionHand::MainHand, false),
        LocalTargetUsePrediction::Result(LocalInteractionResult::Success(
            LocalInteractionSuccess {
                particle_action: Some(particle_action),
                ..LocalInteractionSuccess::server_item_interaction()
            }
        ))
    );
    assert_eq!(world.entities.living_entity_health(81), Some(3.0));
    assert!(world
        .local_item_in_hand(InteractionHand::MainHand)
        .is_none());

    set_main_hand(&mut world, VANILLA_ITEM_RED_MUSHROOM_ID);
    assert_eq!(
        world.predict_local_entity_interaction(81, InteractionHand::MainHand, false),
        LocalTargetUsePrediction::Result(LocalInteractionResult::Success(
            LocalInteractionSuccess::server_item_interaction()
        ))
    );
    assert_eq!(world.entities.living_entity_health(81), Some(6.0));

    add_entity_at(
        &mut world,
        82,
        VANILLA_ENTITY_TYPE_ZOMBIE_HORSE_ID,
        Vec3d {
            x: 1.0,
            y: 2.0,
            z: 3.0,
        },
    );
    let living_accessors =
        crate::entities::vanilla_entity_metadata_living_entity_accessors_for_entity_type(
            VANILLA_ENTITY_TYPE_ZOMBIE_HORSE_ID,
        )
        .unwrap();
    let full_health = world
        .entities
        .attribute_value(82, VANILLA_ATTRIBUTE_MAX_HEALTH_ID)
        .unwrap() as f32;
    assert!(world.apply_set_entity_data(SetEntityData {
        id: 82,
        values: vec![
            EntityDataValue {
                data_id: baby_data_id,
                serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_BOOLEAN_ID,
                value: EntityDataValueKind::Boolean(true),
            },
            EntityDataValue {
                data_id: living_accessors.health_data_id,
                serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_FLOAT_ID,
                value: EntityDataValueKind::Float(full_health),
            },
        ],
    }));
    set_main_hand(&mut world, VANILLA_ITEM_WHEAT_ID);
    assert!(matches!(
        world.predict_local_entity_interaction(82, InteractionHand::MainHand, false),
        LocalTargetUsePrediction::Result(LocalInteractionResult::Success(
            LocalInteractionSuccess {
                sound_action: None,
                particle_action: Some(LocalInteractionParticleAction::EquineBabyFeeding { .. }),
                ..
            }
        ))
    ));
    assert_eq!(
        world
            .local_item_in_hand(InteractionHand::MainHand)
            .unwrap()
            .count,
        1,
        "AbstractHorse.handleEating emits the client particle but only the server ages and consumes"
    );
}

#[test]
fn zombie_horse_abstract_fallback_equips_body_armor_and_clears_ride_pose() {
    let mut world = WorldStore::new();
    add_entity(&mut world, 83, VANILLA_ENTITY_TYPE_ZOMBIE_HORSE_ID);
    world.apply_update_tags(UpdateTags {
        registries: vec![RegistryTags {
            registry: "minecraft:item".to_string(),
            tags: vec![TagNetworkPayload {
                tag: VANILLA_ZOMBIE_HORSE_FOOD_TAG.to_string(),
                entries: Vec::new(),
            }],
        }],
    });
    let horse_accessors = vanilla_entity_metadata_abstract_horse_accessors_for_entity_type(
        VANILLA_ENTITY_TYPE_ZOMBIE_HORSE_ID,
    )
    .unwrap();
    assert!(world.apply_set_entity_data(SetEntityData {
        id: 83,
        values: vec![EntityDataValue {
            data_id: horse_accessors.flags_data_id,
            serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_BYTE_ID,
            value: EntityDataValueKind::Byte(horse_accessors.tame_mask),
        }],
    }));

    let mut body_armor = stack(VANILLA_ITEM_STONE_ID);
    body_armor.count = 2;
    body_armor.component_patch = DataComponentPatchSummary {
        added_type_ids: vec![VANILLA_DATA_COMPONENT_EQUIPPABLE_TYPE_ID],
        equippable: Some(EquippableSummary {
            slot_id: VANILLA_EQUIPMENT_SLOT_BODY_ID,
            equip_sound: SoundEventSummary {
                registry_id: Some(0),
                sound_id: None,
                fixed_range_bits: None,
            },
            asset_id: Some("example:test_horse_armor".to_string()),
            camera_overlay: None,
            allowed_entities: None,
            dispensable: true,
            swappable: true,
            damage_on_hurt: false,
            equip_on_interact: false,
            can_be_sheared: true,
            shearing_sound: SoundEventSummary {
                registry_id: Some(0),
                sound_id: None,
                fixed_range_bits: None,
            },
        }),
        ..DataComponentPatchSummary::default()
    }
    .into();
    assert!(world.apply_set_player_inventory(SetPlayerInventory {
        slot: 0,
        item: body_armor,
    }));
    assert_eq!(
        world.predict_local_entity_interaction(83, InteractionHand::MainHand, false),
        LocalTargetUsePrediction::Result(LocalInteractionResult::Success(
            LocalInteractionSuccess::client_item_interaction()
        ))
    );
    assert_eq!(
        world
            .entities
            .equipment_item(83, EquipmentSlot::Body)
            .unwrap()
            .count,
        1
    );
    assert_eq!(
        world
            .local_item_in_hand(InteractionHand::MainHand)
            .unwrap()
            .count,
        1
    );

    let ride_flags =
        horse_accessors.tame_mask | horse_accessors.eating_mask | horse_accessors.standing_mask;
    assert!(world.apply_set_entity_data(SetEntityData {
        id: 83,
        values: vec![EntityDataValue {
            data_id: horse_accessors.flags_data_id,
            serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_BYTE_ID,
            value: EntityDataValueKind::Byte(ride_flags),
        }],
    }));
    set_main_hand(&mut world, VANILLA_ITEM_RED_DYE_ID);
    assert_eq!(
        world.predict_local_entity_interaction(83, InteractionHand::MainHand, false),
        LocalTargetUsePrediction::Result(LocalInteractionResult::Success(
            LocalInteractionSuccess::client_item_interaction()
        ))
    );
    assert_eq!(
        world.entities.metadata_byte_for_entity(
            83,
            horse_accessors.flags_data_id,
            horse_accessors.flags_default
        ),
        Some(horse_accessors.tame_mask)
    );
}

#[test]
fn camel_husk_mob_interaction_uses_secondary_and_synced_food_tag() {
    let mut world = WorldStore::new();
    add_entity(&mut world, 80, VANILLA_ENTITY_TYPE_CAMEL_HUSK_ID);

    assert_eq!(
        world.predict_local_entity_interaction(80, InteractionHand::MainHand, false),
        LocalTargetUsePrediction::Result(LocalInteractionResult::Success(
            LocalInteractionSuccess::consume(false)
        ))
    );
    assert_eq!(
        world.predict_local_entity_interaction(80, InteractionHand::MainHand, true),
        LocalTargetUsePrediction::Result(LocalInteractionResult::Success(
            LocalInteractionSuccess::client_item_interaction()
        ))
    );

    set_main_hand(&mut world, VANILLA_ITEM_STONE_ID);
    assert_eq!(
        world.predict_local_entity_interaction(80, InteractionHand::MainHand, false),
        LocalTargetUsePrediction::UnsupportedEntityInteract(VANILLA_CAMEL_HUSK_INTERACT_OWNER)
    );
    world.apply_update_tags(UpdateTags {
        registries: vec![RegistryTags {
            registry: "minecraft:item".to_string(),
            tags: vec![TagNetworkPayload {
                tag: VANILLA_CAMEL_HUSK_FOOD_TAG.to_string(),
                entries: vec![VANILLA_ITEM_STONE_ID],
            }],
        }],
    });
    assert_eq!(
        world.predict_local_entity_interaction(80, InteractionHand::MainHand, false),
        LocalTargetUsePrediction::Result(LocalInteractionResult::Success(
            LocalInteractionSuccess::server_item_interaction()
        ))
    );
    world.apply_update_tags(UpdateTags {
        registries: vec![RegistryTags {
            registry: "minecraft:item".to_string(),
            tags: vec![TagNetworkPayload {
                tag: VANILLA_CAMEL_HUSK_FOOD_TAG.to_string(),
                entries: Vec::new(),
            }],
        }],
    });
    assert_eq!(
        world.predict_local_entity_interaction(80, InteractionHand::MainHand, false),
        LocalTargetUsePrediction::Result(LocalInteractionResult::Success(
            LocalInteractionSuccess::consume(false)
        ))
    );

    set_main_hand(&mut world, VANILLA_ITEM_RED_DYE_ID);
    assert_eq!(
        world.predict_local_entity_interaction(80, InteractionHand::MainHand, false),
        LocalTargetUsePrediction::Result(LocalInteractionResult::Success(
            LocalInteractionSuccess::consume(false)
        ))
    );
}
