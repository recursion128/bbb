use super::*;

#[test]
fn f3_i_consumes_copy_data_modifier_without_toggling_overlay() {
    let mut input = ClientInputState::new(true);

    assert!(input.handle_debug_overlay_key(
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Pressed,
        None,
        None
    ));
    assert!(input.handle_debug_overlay_key(
        PhysicalKey::Code(KeyCode::KeyI),
        ElementState::Pressed,
        None,
        None
    ));
    assert!(input.handle_debug_overlay_key(
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Released,
        None,
        None
    ));
    assert!(!input.debug_overlay_visible());
}

#[test]
fn shift_f3_i_copies_block_recreate_command_to_clipboard_and_reports_feedback() {
    let mut input = ClientInputState::new(true);
    let mut world = world_with_debug_player(false);
    let target_pos = BlockPos { x: 0, y: 1, z: 3 };
    insert_empty_chunk_for_block(&mut world, target_pos);
    assert!(world.apply_block_update(ProtocolBlockUpdate {
        pos: ProtocolBlockPos {
            x: target_pos.x,
            y: target_pos.y,
            z: target_pos.z,
        },
        block_state_id: vanilla_block_state_id("minecraft:oak_log", [("axis", "x")]),
    }));
    world.set_local_player_pose(LocalPlayerPoseState {
        position: ProtocolVec3d {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        },
        y_rot: 0.0,
        x_rot: 0.0,
        ..LocalPlayerPoseState::default()
    });
    let mut clipboard = MockDebugClipboard::accepting();
    input.set_shift_key(KeyCode::ShiftLeft, true);

    assert!(input.handle_debug_overlay_key_with_clipboard(
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Pressed,
        Some(&mut world),
        None,
        Some(&mut clipboard)
    ));
    assert!(input.handle_debug_overlay_key_with_clipboard(
        PhysicalKey::Code(KeyCode::KeyI),
        ElementState::Pressed,
        Some(&mut world),
        None,
        Some(&mut clipboard)
    ));

    assert_eq!(
        clipboard.text.as_deref(),
        Some("/setblock 0 1 3 minecraft:oak_log[axis=x]")
    );
    let messages = &world.client_chat().messages;
    assert_eq!(messages.len(), 1);
    assert_eq!(messages[0].kind, ChatMessageKind::ClientSystem);
    assert_eq!(
        messages[0].content,
        "[Debug]: Copied client-side block data to clipboard"
    );

    assert!(input.handle_debug_overlay_key_with_clipboard(
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Released,
        Some(&mut world),
        None,
        Some(&mut clipboard)
    ));
    assert!(!input.debug_overlay_visible());
}

#[test]
fn shift_f3_i_with_permission_copies_local_block_entity_nbt_to_clipboard() {
    let mut input = ClientInputState::new(true);
    let mut world = world_with_debug_player(false);
    grant_debug_recreate_nbt_permission(&mut world);
    let target_pos = BlockPos { x: 0, y: 1, z: 3 };
    insert_empty_chunk_for_block(&mut world, target_pos);
    assert!(world.apply_block_update(ProtocolBlockUpdate {
        pos: ProtocolBlockPos {
            x: target_pos.x,
            y: target_pos.y,
            z: target_pos.z,
        },
        block_state_id: vanilla_block_state_id("minecraft:spawner", []),
    }));
    assert!(world
        .apply_block_entity_data(BlockEntityData {
            pos: ProtocolBlockPos {
                x: target_pos.x,
                y: target_pos.y,
                z: target_pos.z,
            },
            block_entity_type_id:
                bbb_protocol::block_entity_types::VANILLA_BLOCK_ENTITY_TYPE_MOB_SPAWNER_ID,
            raw_nbt: nbt_compound(vec![nbt_string("Lock", "secret")]),
        })
        .expect("local block entity nbt should decode"));
    world.set_local_player_pose(LocalPlayerPoseState {
        position: ProtocolVec3d {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        },
        y_rot: 0.0,
        x_rot: 0.0,
        ..LocalPlayerPoseState::default()
    });
    let mut clipboard = MockDebugClipboard::accepting();
    input.set_shift_key(KeyCode::ShiftLeft, true);

    assert!(input.handle_debug_overlay_key_with_clipboard(
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Pressed,
        Some(&mut world),
        None,
        Some(&mut clipboard)
    ));
    assert!(input.handle_debug_overlay_key_with_clipboard(
        PhysicalKey::Code(KeyCode::KeyI),
        ElementState::Pressed,
        Some(&mut world),
        None,
        Some(&mut clipboard)
    ));

    assert_eq!(
        clipboard.text.as_deref(),
        Some("/setblock 0 1 3 minecraft:spawner{Lock:\"secret\"}")
    );
    assert!(input.take_debug_recreate_server_query_requests().is_empty());
    let messages = &world.client_chat().messages;
    assert_eq!(messages.len(), 1);
    assert_eq!(
        messages[0].content,
        "[Debug]: Copied client-side block data to clipboard"
    );
}

#[test]
fn shift_f3_i_without_permission_omits_local_block_entity_nbt() {
    let mut input = ClientInputState::new(true);
    let mut world = world_with_debug_player(false);
    let target_pos = BlockPos { x: 0, y: 1, z: 3 };
    insert_empty_chunk_for_block(&mut world, target_pos);
    assert!(world.apply_block_update(ProtocolBlockUpdate {
        pos: ProtocolBlockPos {
            x: target_pos.x,
            y: target_pos.y,
            z: target_pos.z,
        },
        block_state_id: vanilla_block_state_id("minecraft:spawner", []),
    }));
    assert!(world
        .apply_block_entity_data(BlockEntityData {
            pos: ProtocolBlockPos {
                x: target_pos.x,
                y: target_pos.y,
                z: target_pos.z,
            },
            block_entity_type_id:
                bbb_protocol::block_entity_types::VANILLA_BLOCK_ENTITY_TYPE_MOB_SPAWNER_ID,
            raw_nbt: nbt_compound(vec![nbt_string("Lock", "secret")]),
        })
        .expect("local block entity nbt should decode"));
    world.set_local_player_pose(LocalPlayerPoseState {
        position: ProtocolVec3d {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        },
        y_rot: 0.0,
        x_rot: 0.0,
        ..LocalPlayerPoseState::default()
    });
    let mut clipboard = MockDebugClipboard::accepting();
    input.set_shift_key(KeyCode::ShiftLeft, true);

    assert!(input.handle_debug_overlay_key_with_clipboard(
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Pressed,
        Some(&mut world),
        None,
        Some(&mut clipboard)
    ));
    assert!(input.handle_debug_overlay_key_with_clipboard(
        PhysicalKey::Code(KeyCode::KeyI),
        ElementState::Pressed,
        Some(&mut world),
        None,
        Some(&mut clipboard)
    ));

    assert_eq!(
        clipboard.text.as_deref(),
        Some("/setblock 0 1 3 minecraft:spawner")
    );
    assert!(input.take_debug_recreate_server_query_requests().is_empty());
    let messages = &world.client_chat().messages;
    assert_eq!(messages.len(), 1);
    assert_eq!(
        messages[0].content,
        "[Debug]: Copied client-side block data to clipboard"
    );
}

#[test]
fn f3_i_without_shift_records_server_recreate_query_request_shell() {
    let mut input = ClientInputState::new(true);
    let mut world = world_with_debug_player(false);
    grant_debug_recreate_nbt_permission(&mut world);
    let target_pos = BlockPos { x: 0, y: 1, z: 3 };
    insert_empty_chunk_for_block(&mut world, target_pos);
    assert!(world.apply_block_update(ProtocolBlockUpdate {
        pos: ProtocolBlockPos {
            x: target_pos.x,
            y: target_pos.y,
            z: target_pos.z,
        },
        block_state_id: vanilla_block_state_id("minecraft:oak_log", [("axis", "x")]),
    }));
    world.set_local_player_pose(LocalPlayerPoseState {
        position: ProtocolVec3d {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        },
        y_rot: 0.0,
        x_rot: 0.0,
        ..LocalPlayerPoseState::default()
    });
    let mut clipboard = MockDebugClipboard::accepting();

    assert!(input.handle_debug_overlay_key_with_clipboard(
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Pressed,
        Some(&mut world),
        None,
        Some(&mut clipboard)
    ));
    assert!(input.handle_debug_overlay_key_with_clipboard(
        PhysicalKey::Code(KeyCode::KeyI),
        ElementState::Pressed,
        Some(&mut world),
        None,
        Some(&mut clipboard)
    ));

    assert_eq!(clipboard.text, None);
    assert_eq!(
        input.pending_debug_recreate_server_query,
        Some(PendingDebugRecreateServerQuery {
            transaction_id: 0,
            client_level_generation: world.client_level_generation(),
            target: PendingDebugRecreateServerQueryTarget::Block {
                pos: target_pos,
                description: "minecraft:oak_log[axis=x]".to_string(),
            },
        })
    );
    assert_eq!(
        input.take_debug_recreate_server_query_requests(),
        vec![DebugRecreateServerQueryRequest::BlockEntityTag {
            transaction_id: 0,
            pos: ProtocolBlockPos {
                x: target_pos.x,
                y: target_pos.y,
                z: target_pos.z,
            },
        }]
    );
    assert!(input.take_debug_recreate_server_query_requests().is_empty());
    assert!(world.client_chat().messages.is_empty());

    assert!(input.handle_debug_overlay_key_with_clipboard(
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Released,
        Some(&mut world),
        None,
        Some(&mut clipboard)
    ));
    assert!(!input.debug_overlay_visible());
}

#[test]
fn f3_i_without_gamemaster_permission_copies_client_recreate_command_without_query() {
    let mut input = ClientInputState::new(true);
    let mut world = world_with_debug_player(false);
    let target_pos = BlockPos { x: 0, y: 1, z: 3 };
    insert_empty_chunk_for_block(&mut world, target_pos);
    assert!(world.apply_block_update(ProtocolBlockUpdate {
        pos: ProtocolBlockPos {
            x: target_pos.x,
            y: target_pos.y,
            z: target_pos.z,
        },
        block_state_id: vanilla_block_state_id("minecraft:oak_log", [("axis", "x")]),
    }));
    world.set_local_player_pose(LocalPlayerPoseState {
        position: ProtocolVec3d {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        },
        y_rot: 0.0,
        x_rot: 0.0,
        ..LocalPlayerPoseState::default()
    });
    let mut clipboard = MockDebugClipboard::accepting();

    assert!(input.handle_debug_overlay_key_with_clipboard(
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Pressed,
        Some(&mut world),
        None,
        Some(&mut clipboard)
    ));
    assert!(input.handle_debug_overlay_key_with_clipboard(
        PhysicalKey::Code(KeyCode::KeyI),
        ElementState::Pressed,
        Some(&mut world),
        None,
        Some(&mut clipboard)
    ));

    assert_eq!(
        clipboard.text.as_deref(),
        Some("/setblock 0 1 3 minecraft:oak_log[axis=x]")
    );
    assert_eq!(input.pending_debug_recreate_server_query, None);
    assert!(input.take_debug_recreate_server_query_requests().is_empty());
    let messages = &world.client_chat().messages;
    assert_eq!(messages.len(), 1);
    assert_eq!(
        messages[0].content,
        "[Debug]: Copied client-side block data to clipboard"
    );
}

#[test]
fn f3_i_server_block_tag_response_copies_recreate_command() {
    let mut input = ClientInputState::new(true);
    let mut world = world_with_debug_player(false);
    grant_debug_recreate_nbt_permission(&mut world);
    let target_pos = BlockPos { x: 0, y: 1, z: 3 };
    insert_empty_chunk_for_block(&mut world, target_pos);
    assert!(world.apply_block_update(ProtocolBlockUpdate {
        pos: ProtocolBlockPos {
            x: target_pos.x,
            y: target_pos.y,
            z: target_pos.z,
        },
        block_state_id: vanilla_block_state_id("minecraft:oak_log", [("axis", "x")]),
    }));
    world.set_local_player_pose(LocalPlayerPoseState {
        position: ProtocolVec3d {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        },
        y_rot: 0.0,
        x_rot: 0.0,
        ..LocalPlayerPoseState::default()
    });
    let mut clipboard = MockDebugClipboard::accepting();

    assert!(input.handle_debug_overlay_key_with_clipboard(
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Pressed,
        Some(&mut world),
        None,
        Some(&mut clipboard)
    ));
    assert!(input.handle_debug_overlay_key_with_clipboard(
        PhysicalKey::Code(KeyCode::KeyI),
        ElementState::Pressed,
        Some(&mut world),
        None,
        Some(&mut clipboard)
    ));

    assert_eq!(
        input.take_debug_recreate_server_query_requests(),
        vec![DebugRecreateServerQueryRequest::BlockEntityTag {
            transaction_id: 0,
            pos: ProtocolBlockPos {
                x: target_pos.x,
                y: target_pos.y,
                z: target_pos.z,
            },
        }]
    );
    assert!(world.apply_block_update(ProtocolBlockUpdate {
        pos: ProtocolBlockPos {
            x: target_pos.x,
            y: target_pos.y,
            z: target_pos.z,
        },
        block_state_id: vanilla_block_state_id("minecraft:stone", []),
    }));
    world.apply_tag_query(TagQuery {
        transaction_id: 0,
        tag_present: true,
        raw_nbt: nbt_compound(vec![
            nbt_string("id", "minecraft:chest"),
            nbt_string("Lock", "secret"),
        ]),
    });

    assert!(input.consume_debug_recreate_server_query_response(&mut world, &mut clipboard));
    assert_eq!(
        clipboard.text.as_deref(),
        Some("/setblock 0 1 3 minecraft:oak_log[axis=x]{Lock:\"secret\",id:\"minecraft:chest\"}")
    );
    assert_eq!(input.pending_debug_recreate_server_query, None);
    let messages = &world.client_chat().messages;
    assert_eq!(messages.len(), 1);
    assert_eq!(messages[0].kind, ChatMessageKind::ClientSystem);
    assert_eq!(
        messages[0].content,
        "[Debug]: Copied server-side block data to clipboard"
    );
}

#[test]
fn f3_i_server_entity_tag_response_copies_recreate_command_without_uuid_or_pos() {
    let mut input = ClientInputState::new(true);
    let mut world = world_with_debug_player(false);
    grant_debug_recreate_nbt_permission(&mut world);
    world.apply_add_entity(AddEntity {
        id: 50,
        uuid: Uuid::from_u128(50),
        entity_type_id: VANILLA_ENTITY_TYPE_CREEPER_ID,
        position: ProtocolVec3d {
            x: 0.0,
            y: 0.0,
            z: 3.0,
        },
        delta_movement: ProtocolVec3d::default(),
        x_rot: 0.0,
        y_rot: 0.0,
        y_head_rot: 0.0,
        data: 0,
    });
    world.set_local_player_pose(LocalPlayerPoseState {
        position: ProtocolVec3d {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        },
        y_rot: 0.0,
        x_rot: 0.0,
        ..LocalPlayerPoseState::default()
    });
    let mut clipboard = MockDebugClipboard::accepting();

    assert!(input.handle_debug_overlay_key_with_clipboard(
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Pressed,
        Some(&mut world),
        None,
        Some(&mut clipboard)
    ));
    assert!(input.handle_debug_overlay_key_with_clipboard(
        PhysicalKey::Code(KeyCode::KeyI),
        ElementState::Pressed,
        Some(&mut world),
        None,
        Some(&mut clipboard)
    ));

    assert_eq!(
        input.take_debug_recreate_server_query_requests(),
        vec![DebugRecreateServerQueryRequest::EntityTag {
            transaction_id: 0,
            entity_id: 50,
        }]
    );
    world.apply_tag_query(TagQuery {
        transaction_id: 99,
        tag_present: true,
        raw_nbt: nbt_compound(vec![nbt_byte("Charged", 1)]),
    });
    assert!(!input.consume_debug_recreate_server_query_response(&mut world, &mut clipboard));
    assert_eq!(clipboard.text, None);
    assert!(input.pending_debug_recreate_server_query.is_some());

    world.apply_tag_query(TagQuery {
        transaction_id: 0,
        tag_present: true,
        raw_nbt: nbt_compound(vec![
            nbt_long_array("UUID", &[1, 2]),
            nbt_list(
                "Pos",
                6,
                vec![
                    0.0f64.to_be_bytes().to_vec(),
                    0.0f64.to_be_bytes().to_vec(),
                    3.0f64.to_be_bytes().to_vec(),
                ],
            ),
            nbt_byte("Charged", 1),
            nbt_string("CustomName", "Boom"),
        ]),
    });

    assert!(input.consume_debug_recreate_server_query_response(&mut world, &mut clipboard));
    assert_eq!(
        clipboard.text.as_deref(),
        Some("/summon minecraft:creeper 0.00 0.00 3.00 {Charged: 1b, CustomName: \"Boom\"}")
    );
    assert_eq!(input.pending_debug_recreate_server_query, None);
    let messages = &world.client_chat().messages;
    assert_eq!(messages.len(), 1);
    assert_eq!(
        messages[0].content,
        "[Debug]: Copied server-side entity data to clipboard"
    );
}

#[test]
fn turtle_server_query_preserves_authoritative_home_pos() {
    let mut input = ClientInputState::new(true);
    let mut world = world_with_debug_player(false);
    world.apply_add_entity(AddEntity {
        id: 57,
        uuid: Uuid::from_u128(57),
        entity_type_id: VANILLA_ENTITY_TYPE_TURTLE_ID,
        position: ProtocolVec3d {
            x: 4.0,
            y: 5.0,
            z: 6.0,
        },
        delta_movement: ProtocolVec3d::default(),
        x_rot: 0.0,
        y_rot: 0.0,
        y_head_rot: 0.0,
        data: 0,
    });
    input.pending_debug_recreate_server_query = Some(PendingDebugRecreateServerQuery {
        transaction_id: 9,
        client_level_generation: world.client_level_generation(),
        target: PendingDebugRecreateServerQueryTarget::Entity {
            entity_id: 57,
            entity_uuid: Uuid::from_u128(57),
            entity_type: "minecraft:turtle".to_string(),
            position: [4.0, 5.0, 6.0],
        },
    });
    world.apply_tag_query(TagQuery {
        transaction_id: 9,
        tag_present: true,
        raw_nbt: nbt_compound(vec![
            nbt_long_array("UUID", &[1, 2]),
            nbt_list(
                "Pos",
                6,
                vec![
                    4.0f64.to_be_bytes().to_vec(),
                    5.0f64.to_be_bytes().to_vec(),
                    6.0f64.to_be_bytes().to_vec(),
                ],
            ),
            nbt_int_array("home_pos", &[12, 63, -8]),
            nbt_byte("has_egg", 1),
        ]),
    });
    let mut clipboard = MockDebugClipboard::accepting();

    assert!(input.consume_debug_recreate_server_query_response(&mut world, &mut clipboard));
    assert_eq!(
        clipboard.text.as_deref(),
        Some(
            "/summon minecraft:turtle 4.00 5.00 6.00 \
             {home_pos: [I; 12, 63, -8], has_egg: 1b}"
        )
    );
    assert_eq!(input.pending_debug_recreate_server_query, None);
    assert_eq!(
        world.client_chat().messages[0].content,
        "[Debug]: Copied server-side entity data to clipboard"
    );
}

#[test]
fn f3_i_null_server_tag_response_reports_failure_without_copying() {
    let mut input = ClientInputState::new(true);
    let mut world = world_with_debug_player(false);
    grant_debug_recreate_nbt_permission(&mut world);
    let target_pos = BlockPos { x: 0, y: 1, z: 3 };
    insert_empty_chunk_for_block(&mut world, target_pos);
    assert!(world.apply_block_update(ProtocolBlockUpdate {
        pos: ProtocolBlockPos {
            x: target_pos.x,
            y: target_pos.y,
            z: target_pos.z,
        },
        block_state_id: vanilla_block_state_id("minecraft:stone", []),
    }));
    world.set_local_player_pose(LocalPlayerPoseState {
        position: ProtocolVec3d {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        },
        y_rot: 0.0,
        x_rot: 0.0,
        ..LocalPlayerPoseState::default()
    });
    let mut clipboard = MockDebugClipboard::accepting();

    assert!(input.handle_debug_overlay_key_with_clipboard(
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Pressed,
        Some(&mut world),
        None,
        Some(&mut clipboard)
    ));
    assert!(input.handle_debug_overlay_key_with_clipboard(
        PhysicalKey::Code(KeyCode::KeyI),
        ElementState::Pressed,
        Some(&mut world),
        None,
        Some(&mut clipboard)
    ));
    input.take_debug_recreate_server_query_requests();
    world.apply_tag_query(TagQuery {
        transaction_id: 0,
        tag_present: false,
        raw_nbt: vec![0],
    });

    assert!(input.consume_debug_recreate_server_query_response(&mut world, &mut clipboard));
    assert_eq!(clipboard.text, None);
    assert_eq!(input.pending_debug_recreate_server_query, None);
    let messages = &world.client_chat().messages;
    assert_eq!(messages.len(), 1);
    assert_eq!(
        messages[0].content,
        "[Debug]: Server returned no recreate data"
    );
}

#[test]
fn f3_i_entity_disappearing_before_matching_response_does_not_copy() {
    let mut input = ClientInputState::new(true);
    let mut world = world_with_debug_player(false);
    world.apply_add_entity(AddEntity {
        id: 51,
        uuid: Uuid::from_u128(51),
        entity_type_id: VANILLA_ENTITY_TYPE_CREEPER_ID,
        position: ProtocolVec3d {
            x: 1.0,
            y: 2.0,
            z: 3.0,
        },
        delta_movement: ProtocolVec3d::default(),
        x_rot: 0.0,
        y_rot: 0.0,
        y_head_rot: 0.0,
        data: 0,
    });
    input.pending_debug_recreate_server_query = Some(PendingDebugRecreateServerQuery {
        transaction_id: 7,
        client_level_generation: world.client_level_generation(),
        target: PendingDebugRecreateServerQueryTarget::Entity {
            entity_id: 51,
            entity_uuid: Uuid::from_u128(51),
            entity_type: "minecraft:creeper".to_string(),
            position: [1.0, 2.0, 3.0],
        },
    });
    assert_eq!(
        world.apply_remove_entities(ProtocolRemoveEntities {
            entity_ids: vec![51],
        }),
        1
    );
    world.apply_add_entity(AddEntity {
        id: 51,
        uuid: Uuid::from_u128(52),
        entity_type_id: VANILLA_ENTITY_TYPE_CREEPER_ID,
        position: ProtocolVec3d {
            x: 1.0,
            y: 2.0,
            z: 3.0,
        },
        delta_movement: ProtocolVec3d::default(),
        x_rot: 0.0,
        y_rot: 0.0,
        y_head_rot: 0.0,
        data: 0,
    });
    world.apply_tag_query(TagQuery {
        transaction_id: 7,
        tag_present: true,
        raw_nbt: nbt_compound(vec![nbt_byte("Charged", 1)]),
    });
    let mut clipboard = MockDebugClipboard::accepting();

    assert!(input.consume_debug_recreate_server_query_response(&mut world, &mut clipboard));
    assert_eq!(clipboard.text, None);
    assert_eq!(input.pending_debug_recreate_server_query, None);
    assert_eq!(
        world.client_chat().messages[0].content,
        "[Debug]: Debug recreate target disappeared before server response"
    );
}

#[test]
fn f3_i_rejects_matching_response_and_unsent_request_after_client_level_change() {
    let mut input = ClientInputState::new(true);
    let mut world = world_with_debug_player(false);
    let entity = AddEntity {
        id: 51,
        uuid: Uuid::from_u128(51),
        entity_type_id: VANILLA_ENTITY_TYPE_CREEPER_ID,
        position: ProtocolVec3d {
            x: 1.0,
            y: 2.0,
            z: 3.0,
        },
        delta_movement: ProtocolVec3d::default(),
        x_rot: 0.0,
        y_rot: 0.0,
        y_head_rot: 0.0,
        data: 0,
    };
    world.apply_add_entity(entity.clone());
    let old_generation = world.client_level_generation();
    let pending = PendingDebugRecreateServerQuery {
        transaction_id: 7,
        client_level_generation: old_generation,
        target: PendingDebugRecreateServerQueryTarget::Entity {
            entity_id: 51,
            entity_uuid: Uuid::from_u128(51),
            entity_type: "minecraft:creeper".to_string(),
            position: [1.0, 2.0, 3.0],
        },
    };
    input.pending_debug_recreate_server_query = Some(pending.clone());
    input
        .debug_recreate_server_query_requests
        .push(DebugRecreateServerQueryRequest::EntityTag {
            transaction_id: 7,
            entity_id: 51,
        });

    world.clear_client_level();
    assert!(world.client_level_generation() > old_generation);
    assert!(input
        .take_debug_recreate_server_query_requests_for_level(world.client_level_generation())
        .is_empty());
    assert_eq!(input.pending_debug_recreate_server_query, None);

    world.apply_add_entity(entity);
    input.pending_debug_recreate_server_query = Some(pending);
    world.apply_tag_query(TagQuery {
        transaction_id: 7,
        tag_present: true,
        raw_nbt: nbt_compound(vec![nbt_byte("Charged", 1)]),
    });
    let mut clipboard = MockDebugClipboard::accepting();

    assert!(input.consume_debug_recreate_server_query_response(&mut world, &mut clipboard));
    assert_eq!(clipboard.text, None);
    assert_eq!(input.pending_debug_recreate_server_query, None);
    assert_eq!(
        world.client_chat().messages[0].content,
        "[Debug]: Debug recreate level changed before server response"
    );
}

#[test]
fn queues_debug_recreate_server_query_requests_as_tag_query_commands() {
    let (tx, mut rx) = mpsc::channel(2);
    let commands = Some(tx);
    let mut counters = NetCounters::default();

    queue_debug_recreate_server_query_request(
        &mut counters,
        &commands,
        DebugRecreateServerQueryRequest::BlockEntityTag {
            transaction_id: 3,
            pos: ProtocolBlockPos { x: -1, y: 2, z: 7 },
        },
    );
    queue_debug_recreate_server_query_request(
        &mut counters,
        &commands,
        DebugRecreateServerQueryRequest::EntityTag {
            transaction_id: 4,
            entity_id: 88,
        },
    );

    assert_eq!(counters.block_entity_tag_query_commands_queued, 1);
    assert_eq!(counters.entity_tag_query_commands_queued, 1);
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::BlockEntityTagQuery(BlockEntityTagQuery {
            transaction_id: 3,
            pos: ProtocolBlockPos { x: -1, y: 2, z: 7 },
        })
    );
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::EntityTagQuery(EntityTagQuery {
            transaction_id: 4,
            entity_id: 88,
        })
    );
}

pub(super) fn nbt_compound(entries: Vec<Vec<u8>>) -> Vec<u8> {
    let mut out = vec![10];
    for entry in entries {
        out.extend_from_slice(&entry);
    }
    out.push(0);
    out
}

fn nbt_byte(name: &str, value: i8) -> Vec<u8> {
    let mut out = vec![1];
    write_mutf8(&mut out, name);
    out.push(value as u8);
    out
}

pub(super) fn nbt_string(name: &str, value: &str) -> Vec<u8> {
    let mut out = vec![8];
    write_mutf8(&mut out, name);
    write_mutf8(&mut out, value);
    out
}

fn nbt_list(name: &str, element_type: u8, values: Vec<Vec<u8>>) -> Vec<u8> {
    let mut out = vec![9];
    write_mutf8(&mut out, name);
    out.push(element_type);
    out.extend_from_slice(&(values.len() as i32).to_be_bytes());
    for value in values {
        out.extend_from_slice(&value);
    }
    out
}

fn nbt_long_array(name: &str, values: &[i64]) -> Vec<u8> {
    let mut out = vec![12];
    write_mutf8(&mut out, name);
    out.extend_from_slice(&(values.len() as i32).to_be_bytes());
    for value in values {
        out.extend_from_slice(&value.to_be_bytes());
    }
    out
}

fn nbt_int_array(name: &str, values: &[i32]) -> Vec<u8> {
    let mut out = vec![11];
    write_mutf8(&mut out, name);
    out.extend_from_slice(&(values.len() as i32).to_be_bytes());
    for value in values {
        out.extend_from_slice(&value.to_be_bytes());
    }
    out
}

fn write_mutf8(out: &mut Vec<u8>, value: &str) {
    let mut bytes = Vec::new();
    for unit in value.encode_utf16() {
        if unit == 0 {
            bytes.extend_from_slice(&[0xc0, 0x80]);
        } else if unit <= 0x7f {
            bytes.push(unit as u8);
        } else if unit <= 0x7ff {
            bytes.push((0xc0 | ((unit >> 6) & 0x1f)) as u8);
            bytes.push((0x80 | (unit & 0x3f)) as u8);
        } else {
            bytes.push((0xe0 | ((unit >> 12) & 0x0f)) as u8);
            bytes.push((0x80 | ((unit >> 6) & 0x3f)) as u8);
            bytes.push((0x80 | (unit & 0x3f)) as u8);
        }
    }
    out.extend_from_slice(&(bytes.len() as u16).to_be_bytes());
    out.extend_from_slice(&bytes);
}

#[test]
fn shift_f3_i_copies_entity_recreate_command_to_clipboard_and_reports_feedback() {
    let mut input = ClientInputState::new(true);
    let mut world = world_with_debug_player(false);
    world.apply_add_entity(AddEntity {
        id: 50,
        uuid: Uuid::from_u128(50),
        entity_type_id: VANILLA_ENTITY_TYPE_CREEPER_ID,
        position: ProtocolVec3d {
            x: 0.0,
            y: 0.0,
            z: 3.0,
        },
        delta_movement: ProtocolVec3d::default(),
        x_rot: 0.0,
        y_rot: 0.0,
        y_head_rot: 0.0,
        data: 0,
    });
    world.set_local_player_pose(LocalPlayerPoseState {
        position: ProtocolVec3d {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        },
        y_rot: 0.0,
        x_rot: 0.0,
        ..LocalPlayerPoseState::default()
    });
    let mut clipboard = MockDebugClipboard::accepting();
    input.set_shift_key(KeyCode::ShiftLeft, true);

    assert!(input.handle_debug_overlay_key_with_clipboard(
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Pressed,
        Some(&mut world),
        None,
        Some(&mut clipboard)
    ));
    assert!(input.handle_debug_overlay_key_with_clipboard(
        PhysicalKey::Code(KeyCode::KeyI),
        ElementState::Pressed,
        Some(&mut world),
        None,
        Some(&mut clipboard)
    ));

    assert_eq!(
        clipboard.text.as_deref(),
        Some("/summon minecraft:creeper 0.00 0.00 3.00")
    );
    let messages = &world.client_chat().messages;
    assert_eq!(messages.len(), 1);
    assert_eq!(messages[0].kind, ChatMessageKind::ClientSystem);
    assert_eq!(
        messages[0].content,
        "[Debug]: Copied client-side entity data to clipboard"
    );

    assert!(input.handle_debug_overlay_key_with_clipboard(
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Released,
        Some(&mut world),
        None,
        Some(&mut clipboard)
    ));
    assert!(!input.debug_overlay_visible());
}

#[test]
fn f3_i_is_consumed_but_does_not_copy_when_reduced_debug_info_blocks_inspect() {
    let mut input = ClientInputState::new(true);
    let mut world = world_with_debug_player(true);
    let mut clipboard = MockDebugClipboard::accepting();

    assert!(input.handle_debug_overlay_key_with_clipboard(
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Pressed,
        Some(&mut world),
        None,
        Some(&mut clipboard)
    ));
    assert!(input.handle_debug_overlay_key_with_clipboard(
        PhysicalKey::Code(KeyCode::KeyI),
        ElementState::Pressed,
        Some(&mut world),
        None,
        Some(&mut clipboard)
    ));

    assert_eq!(clipboard.text, None);
    assert!(input.take_debug_recreate_server_query_requests().is_empty());
    assert!(world.client_chat().messages.is_empty());

    assert!(input.handle_debug_overlay_key_with_clipboard(
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Released,
        Some(&mut world),
        None,
        Some(&mut clipboard)
    ));
    assert!(!input.debug_overlay_visible());
}

#[test]
fn f3_l_toggles_profiling_and_lifecycle_feedback_uses_real_results() {
    // Like Vanilla's key callback, explicitly delivered events are not
    // discarded because a previously published focus snapshot is false.
    let mut input = ClientInputState::new(false);
    let mut world = WorldStore::new();

    assert!(input.handle_debug_overlay_key(
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Pressed,
        Some(&mut world),
        None
    ));
    assert!(input.handle_debug_overlay_key(
        PhysicalKey::Code(KeyCode::KeyL),
        ElementState::Pressed,
        Some(&mut world),
        None
    ));
    assert_eq!(
        input.take_debug_profiling_toggle_requests(),
        vec![DebugProfilingToggleRequest::Start]
    );
    assert!(input.handle_debug_overlay_key(
        PhysicalKey::Code(KeyCode::KeyL),
        ElementState::Pressed,
        Some(&mut world),
        None
    ));
    assert_eq!(
        input.take_debug_profiling_toggle_requests(),
        vec![DebugProfilingToggleRequest::Stop]
    );
    assert!(input.take_debug_profiling_toggle_requests().is_empty());
    assert!(world.client_chat().messages.is_empty());

    input.apply_debug_profiling_started(&mut world);
    input.apply_debug_profiling_stopped(&mut world, Duration::from_millis(2_500), 50, 20.0);
    input.apply_debug_profiling_report_saved(
        &mut world,
        std::path::Path::new("debug/profiling/result.zip"),
    );
    let messages = &world.client_chat().messages;
    assert_eq!(messages.len(), 3);
    assert_eq!(messages[0].kind, ChatMessageKind::ClientSystem);
    assert_eq!(
        messages[0].content,
        "[Debug]: Profiling started for 10 seconds. Use F3 + L to stop early"
    );
    assert_eq!(messages[1].kind, ChatMessageKind::ClientSystem);
    assert_eq!(
        messages[1].content,
        "[Debug]: Stopped tick profiling after 2.50 seconds and 50 ticks (20.00 ticks per second)"
    );
    assert_eq!(messages[2].kind, ChatMessageKind::ClientSystem);
    assert_eq!(
        messages[2].content,
        "[Debug]: Profiling ended. Saved results to debug/profiling/result.zip"
    );
    assert_eq!(messages[2].styled_content.len(), 4);
    assert_eq!(
        messages[2].styled_content[2].text,
        "Profiling ended. Saved results to "
    );
    assert_eq!(
        messages[2].styled_content[2].style,
        ComponentStyle::default()
    );
    assert_eq!(
        messages[2].styled_content[3].text,
        "debug/profiling/result.zip"
    );
    assert_eq!(messages[2].styled_content[3].style.underlined, Some(true));
    assert_eq!(
        messages[2].styled_content[3].style.click_event,
        Some(ComponentClickEvent::OpenFile {
            path: "debug/profiling/result.zip".into(),
        })
    );
    assert_eq!(world.counters().chat_messages_tracked, 3);
    assert!(input.handle_debug_overlay_key(
        PhysicalKey::Code(KeyCode::F3),
        ElementState::Released,
        Some(&mut world),
        None
    ));
    assert!(!input.debug_overlay_visible());
}

#[test]
fn profiling_report_failure_does_not_cancel_a_newer_recording() {
    let mut input = ClientInputState::new(true);
    let mut world = WorldStore::new();
    input.apply_debug_profiling_started(&mut world);
    input.apply_debug_profiling_report_failed(&mut world, "writer unavailable");
    assert!(input.debug_profiling_recording());
    assert_eq!(
        world.client_chat().messages[1].content,
        "[Debug]: Profiling failed: writer unavailable"
    );
}
