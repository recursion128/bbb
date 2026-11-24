use super::*;

fn world_with_test_block(pos: BlockPos, mode: TestBlockMode, message: &str) -> WorldStore {
    let mut world = world_with_debug_player(false);
    grant_gamemaster_block_permission(&mut world);
    let block_state_id = bbb_client_world::vanilla_block_state_id(
        "minecraft:test_block",
        &BTreeMap::from([("mode".to_string(), mode.serialized_name().to_string())]),
    )
    .expect("vanilla test block state");
    let section_index = usize::try_from((pos.y + 64).div_euclid(16))
        .expect("test block lies in the overworld build height");
    let record = BlockEntityRecord::empty(
        pos.x.rem_euclid(16) as u8,
        i16::try_from(pos.y).unwrap(),
        pos.z.rem_euclid(16) as u8,
        VANILLA_BLOCK_ENTITY_TYPE_TEST_BLOCK_ID,
    )
    .with_raw_nbt(vec![0])
    .with_payload(bbb_client_world::BlockEntityPayload::TestBlock(Box::new(
        TestBlockEntityDataState {
            mode,
            message: message.into(),
            powered: false,
        },
    )));
    world.insert_decoded_chunk(ChunkColumn {
        pos: ChunkPos {
            x: pos.x.div_euclid(16),
            z: pos.z.div_euclid(16),
        },
        state: ChunkState::Decoded,
        heightmaps: Vec::new(),
        sections: (0..24)
            .map(|index| {
                std::sync::Arc::new(ChunkSection {
                    non_empty_block_count: if index == section_index { 4096 } else { 0 },
                    fluid_count: 0,
                    block_states: single_value_container(
                        PaletteDomain::BlockStates,
                        4096,
                        if index == section_index {
                            block_state_id
                        } else {
                            0
                        },
                    ),
                    biomes: single_value_container(PaletteDomain::Biomes, 64, 0),
                })
            })
            .collect(),
        block_entities: std::sync::Arc::new(vec![record]),
        light: LightData::default(),
    });
    world
}

fn world_with_test_instance_block(pos: BlockPos) -> WorldStore {
    let mut world = world_with_debug_player(false);
    grant_gamemaster_block_permission(&mut world);
    let block_state_id =
        bbb_client_world::vanilla_block_state_id("minecraft:test_instance_block", &BTreeMap::new())
            .expect("vanilla test-instance block state");
    let section_index = usize::try_from((pos.y + 64).div_euclid(16))
        .expect("test-instance block lies in the overworld build height");
    let record = BlockEntityRecord::empty(
        pos.x.rem_euclid(16) as u8,
        i16::try_from(pos.y).unwrap(),
        pos.z.rem_euclid(16) as u8,
        VANILLA_BLOCK_ENTITY_TYPE_TEST_INSTANCE_BLOCK_ID,
    )
    .with_raw_nbt(vec![0])
    .with_payload(bbb_client_world::BlockEntityPayload::TestInstanceBlock(
        Box::new(TestInstanceBlockEntityDataState {
            test: Some("example:initial".to_string()),
            size: TestInstanceBlockSizeState { x: 3, y: 4, z: 5 },
            rotation: TestInstanceBlockRotation::None,
            ignore_entities: false,
            status: TestInstanceBlockRunStatus::Cleared,
            error_message: None,
            error_markers: Vec::new(),
        }),
    ));
    world.insert_decoded_chunk(ChunkColumn {
        pos: ChunkPos {
            x: pos.x.div_euclid(16),
            z: pos.z.div_euclid(16),
        },
        state: ChunkState::Decoded,
        heightmaps: Vec::new(),
        sections: (0..24)
            .map(|index| {
                std::sync::Arc::new(ChunkSection {
                    non_empty_block_count: if index == section_index { 4096 } else { 0 },
                    fluid_count: 0,
                    block_states: single_value_container(
                        PaletteDomain::BlockStates,
                        4096,
                        if index == section_index {
                            block_state_id
                        } else {
                            0
                        },
                    ),
                    biomes: single_value_container(PaletteDomain::Biomes, 64, 0),
                })
            })
            .collect(),
        block_entities: std::sync::Arc::new(vec![record]),
        light: LightData::default(),
    });
    world
}

fn world_with_structure_block(pos: BlockPos) -> WorldStore {
    let mut world = world_with_debug_player(false);
    grant_gamemaster_block_permission(&mut world);
    let block_state_id = bbb_client_world::vanilla_block_state_id(
        "minecraft:structure_block",
        &BTreeMap::from([("mode".to_string(), "load".to_string())]),
    )
    .expect("vanilla structure block state");
    let section_index = usize::try_from((pos.y + 64).div_euclid(16))
        .expect("structure block lies in the overworld build height");
    let record = BlockEntityRecord::empty(
        pos.x.rem_euclid(16) as u8,
        i16::try_from(pos.y).unwrap(),
        pos.z.rem_euclid(16) as u8,
        VANILLA_BLOCK_ENTITY_TYPE_STRUCTURE_BLOCK_ID,
    )
    .with_raw_nbt(vec![0])
    .with_payload(bbb_client_world::BlockEntityPayload::StructureBlock(
        Box::new(StructureBlockEntityDataState {
            name: "example:initial".to_string(),
            author: "builder".into(),
            metadata: "initial data".into(),
            offset: BlockPos { x: 1, y: -2, z: 3 },
            size: StructureBlockSizeState { x: 4, y: 5, z: 6 },
            mirror: StructureBlockMirror::FrontBack,
            rotation: StructureBlockRotation::Clockwise90,
            mode: StructureBlockMode::Load,
            ignore_entities: false,
            strict: true,
            powered: false,
            show_air: true,
            show_bounding_box: false,
            integrity_bits: 0.75_f32.to_bits(),
            seed: 9,
        }),
    ));
    world.insert_decoded_chunk(ChunkColumn {
        pos: ChunkPos {
            x: pos.x.div_euclid(16),
            z: pos.z.div_euclid(16),
        },
        state: ChunkState::Decoded,
        heightmaps: Vec::new(),
        sections: (0..24)
            .map(|index| {
                std::sync::Arc::new(ChunkSection {
                    non_empty_block_count: if index == section_index { 4096 } else { 0 },
                    fluid_count: 0,
                    block_states: single_value_container(
                        PaletteDomain::BlockStates,
                        4096,
                        if index == section_index {
                            block_state_id
                        } else {
                            0
                        },
                    ),
                    biomes: single_value_container(PaletteDomain::Biomes, 64, 0),
                })
            })
            .collect(),
        block_entities: std::sync::Arc::new(vec![record]),
        light: LightData::default(),
    });
    world
}

fn world_with_jigsaw_block(pos: BlockPos, orientation: &str) -> WorldStore {
    let mut world = world_with_debug_player(false);
    grant_gamemaster_block_permission(&mut world);
    let block_state_id = bbb_client_world::vanilla_block_state_id(
        "minecraft:jigsaw",
        &BTreeMap::from([("orientation".to_string(), orientation.to_string())]),
    )
    .expect("vanilla jigsaw block state");
    let section_index = usize::try_from((pos.y + 64).div_euclid(16))
        .expect("jigsaw block lies in the overworld build height");
    let record = BlockEntityRecord::empty(
        pos.x.rem_euclid(16) as u8,
        i16::try_from(pos.y).unwrap(),
        pos.z.rem_euclid(16) as u8,
        VANILLA_BLOCK_ENTITY_TYPE_JIGSAW_ID,
    )
    .with_raw_nbt(vec![0])
    .with_payload(bbb_client_world::BlockEntityPayload::JigsawBlock(Box::new(
        JigsawBlockEntityDataState {
            name: "example:entry".to_string(),
            target: "example:target".to_string(),
            pool: "example:village/houses".to_string(),
            final_state: "minecraft:oak_planks[axis=y]".into(),
            joint: JigsawJoint::Rollable,
            selection_priority: -2,
            placement_priority: 3,
        },
    )));
    world.insert_decoded_chunk(ChunkColumn {
        pos: ChunkPos {
            x: pos.x.div_euclid(16),
            z: pos.z.div_euclid(16),
        },
        state: ChunkState::Decoded,
        heightmaps: Vec::new(),
        sections: (0..24)
            .map(|index| {
                std::sync::Arc::new(ChunkSection {
                    non_empty_block_count: if index == section_index { 4096 } else { 0 },
                    fluid_count: 0,
                    block_states: single_value_container(
                        PaletteDomain::BlockStates,
                        4096,
                        if index == section_index {
                            block_state_id
                        } else {
                            0
                        },
                    ),
                    biomes: single_value_container(PaletteDomain::Biomes, 64, 0),
                })
            })
            .collect(),
        block_entities: std::sync::Arc::new(vec![record]),
        light: LightData::default(),
    });
    world
}

fn click_structure_block_editor(
    input: &mut ClientInputState,
    counters: &mut NetCounters,
    commands: &Option<mpsc::Sender<NetCommand>>,
    rect: bbb_client_render::HudCommandBlockEditorRect,
) -> bool {
    input.handle_structure_block_editor_mouse_input(
        counters,
        commands,
        MouseButton::Left,
        ElementState::Pressed,
        Some(PhysicalPosition::new(
            f64::from(rect.x + 1),
            f64::from(rect.y + 1),
        )),
        PhysicalSize::new(320, 240),
        &FixedCommandSuggestionTextMeasurer,
    )
}

fn click_jigsaw_block_editor(
    input: &mut ClientInputState,
    counters: &mut NetCounters,
    commands: &Option<mpsc::Sender<NetCommand>>,
    rect: bbb_client_render::HudCommandBlockEditorRect,
) -> bool {
    input.handle_jigsaw_block_editor_mouse_input(
        counters,
        commands,
        MouseButton::Left,
        ElementState::Pressed,
        Some(PhysicalPosition::new(
            f64::from(rect.x + 1),
            f64::from(rect.y + 1),
        )),
        PhysicalSize::new(320, 240),
        &FixedCommandSuggestionTextMeasurer,
    )
}

fn click_test_instance_editor(
    input: &mut ClientInputState,
    counters: &mut NetCounters,
    commands: &Option<mpsc::Sender<NetCommand>>,
    rect: bbb_client_render::HudCommandBlockEditorRect,
) -> bool {
    input.handle_test_instance_block_editor_mouse_input(
        counters,
        commands,
        MouseButton::Left,
        ElementState::Pressed,
        Some(PhysicalPosition::new(
            f64::from(rect.x + 1),
            f64::from(rect.y + 1),
        )),
        PhysicalSize::new(320, 240),
        &FixedCommandSuggestionTextMeasurer,
    )
}

fn click_test_block_editor(
    input: &mut ClientInputState,
    counters: &mut NetCounters,
    commands: &Option<mpsc::Sender<NetCommand>>,
    rect: bbb_client_render::HudCommandBlockEditorRect,
) -> bool {
    input.handle_test_block_editor_mouse_input(
        counters,
        commands,
        MouseButton::Left,
        ElementState::Pressed,
        Some(PhysicalPosition::new(
            f64::from(rect.x + 1),
            f64::from(rect.y + 1),
        )),
        PhysicalSize::new(320, 240),
        &FixedCommandSuggestionTextMeasurer,
    )
}

#[test]
fn test_block_editor_preserves_hidden_message_cycles_modes_and_done_sends_once() {
    let pos = BlockPos { x: 0, y: 0, z: 0 };
    let (tx, mut rx) = mpsc::channel(4);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = world_with_test_block(pos, TestBlockMode::Log, "initial");
    let layout = bbb_client_render::hud_test_block_editor_layout(PhysicalSize::new(320, 240));
    assert!(open_block_editor(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        pos
    ));

    let hud = input.test_block_editor_hud_state().unwrap();
    assert_eq!(hud.mode, TestBlockMode::Log);
    assert_eq!(hud.message.text(), "initial");
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::ControlLeft),
        ElementState::Pressed,
    );
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::KeyA),
        ElementState::Pressed,
    );
    handle_text_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        &"😀".repeat(100),
    );
    assert_eq!(
        input
            .test_block_editor_hud_state()
            .unwrap()
            .message
            .text()
            .encode_utf16()
            .count(),
        128
    );

    for expected in [
        TestBlockMode::Fail,
        TestBlockMode::Accept,
        TestBlockMode::Start,
    ] {
        assert!(click_test_block_editor(
            &mut input,
            &mut counters,
            &commands,
            layout.mode,
        ));
        assert_eq!(input.test_block_editor_hud_state().unwrap().mode, expected);
    }
    let hidden_message = input.test_block_editor_hud_state().unwrap().message.text();
    handle_text_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        "ignored while hidden",
    );
    assert_eq!(
        input.test_block_editor_hud_state().unwrap().message.text(),
        hidden_message
    );
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::Enter),
        ElementState::Pressed,
    );
    assert!(input.test_block_editor_is_active());
    assert!(rx.try_recv().is_err(), "Enter must not submit the editor");

    assert!(click_test_block_editor(
        &mut input,
        &mut counters,
        &commands,
        layout.mode,
    ));
    assert_eq!(
        input.test_block_editor_hud_state().unwrap().mode,
        TestBlockMode::Log
    );
    assert!(click_test_block_editor(
        &mut input,
        &mut counters,
        &commands,
        layout.done,
    ));
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::SetTestBlock(SetTestBlock {
            pos: ProtocolBlockPos { x: 0, y: 0, z: 0 },
            mode: TestBlockMode::Log,
            message: hidden_message.into(),
        })
    );
    assert_eq!(counters.set_test_block_commands_queued, 1);
    assert!(!input.test_block_editor_is_active());
    assert!(input.screen.test_block().is_none());
    assert!(rx.try_recv().is_err());
}

#[test]
fn test_block_editor_right_click_cancel_and_escape_never_send_editor_packet() {
    let pos = BlockPos { x: 0, y: 0, z: 0 };
    let (tx, mut rx) = mpsc::channel(6);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = world_with_test_block(pos, TestBlockMode::Fail, "failure");

    assert!(mouse::start_use_item(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        Some(CrosshairTarget::Block(
            crate::crosshair::CrosshairBlockHit {
                pos,
                face: ProtocolDirection::Up,
                cursor: [0.25, 0.5, 0.75],
                inside: false,
            }
        )),
        None,
    ));
    assert!(input.test_block_editor_is_active());
    assert!(matches!(rx.try_recv(), Ok(NetCommand::UseItemOn(_))));
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::Swing(InteractionHand::MainHand)
    );
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::Escape),
        ElementState::Pressed,
    );
    assert!(!input.test_block_editor_is_active());
    assert!(rx.try_recv().is_err());

    assert!(open_block_editor(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        pos
    ));
    let cancel =
        bbb_client_render::hud_test_block_editor_layout(PhysicalSize::new(320, 240)).cancel;
    assert!(click_test_block_editor(
        &mut input,
        &mut counters,
        &commands,
        cancel,
    ));
    assert!(!input.test_block_editor_is_active());
    assert!(rx.try_recv().is_err());
    assert_eq!(counters.set_test_block_commands_queued, 0);
    assert_eq!(counters.use_item_on_commands_queued, 1);
}

#[test]
fn test_instance_block_editor_initializes_queries_and_applies_only_present_response_size() {
    let pos = BlockPos { x: 0, y: 0, z: 0 };
    let (tx, mut rx) = mpsc::channel(8);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = world_with_test_instance_block(pos);
    assert!(open_block_editor(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        pos
    ));

    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::TestInstanceBlockAction(TestInstanceBlockAction {
            pos: ProtocolBlockPos { x: 0, y: 0, z: 0 },
            action: TestInstanceBlockActionKind::Init,
            test: Some("example:initial".to_string()),
            size: Vec3i { x: 3, y: 4, z: 5 },
            rotation: TestInstanceBlockRotation::None,
            ignore_entities: false,
        })
    );
    assert!(
        rx.try_recv().is_err(),
        "same open must not send a second INIT"
    );

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::ControlLeft),
        ElementState::Pressed,
    );
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::KeyA),
        ElementState::Pressed,
    );
    handle_text_input(&mut input, &mut counters, &mut world, &commands, "Bad ID");
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::TestInstanceBlockAction(TestInstanceBlockAction {
            pos: ProtocolBlockPos { x: 0, y: 0, z: 0 },
            action: TestInstanceBlockActionKind::Query,
            test: None,
            size: Vec3i { x: 3, y: 4, z: 5 },
            rotation: TestInstanceBlockRotation::None,
            ignore_entities: false,
        })
    );
    assert!(
        input
            .test_instance_block_editor_hud_state()
            .unwrap()
            .local_invalid_id
    );

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::KeyA),
        ElementState::Pressed,
    );
    handle_text_input(&mut input, &mut counters, &mut world, &commands, "new:test");
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::TestInstanceBlockAction(TestInstanceBlockAction {
            pos: ProtocolBlockPos { x: 0, y: 0, z: 0 },
            action: TestInstanceBlockActionKind::Query,
            test: Some("new:test".to_string()),
            size: Vec3i { x: 3, y: 4, z: 5 },
            rotation: TestInstanceBlockRotation::None,
            ignore_entities: false,
        })
    );
    assert!(
        !input
            .test_instance_block_editor_hud_state()
            .unwrap()
            .local_invalid_id
    );

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::Tab),
        ElementState::Pressed,
    );
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::KeyA),
        ElementState::Pressed,
    );
    handle_text_input(&mut input, &mut counters, &mut world, &commands, "33");
    world.apply_test_instance_block_status(TestInstanceBlockStatus {
        status: RichTextComponent::plain("No size"),
        size: None,
    });
    input.apply_screen_requests(&mut world, &mut counters, &commands);
    assert_eq!(
        input.test_instance_block_editor_hud_state().unwrap().fields[1].text(),
        "33"
    );

    world.apply_test_instance_block_status(TestInstanceBlockStatus {
        status: RichTextComponent::plain("Sized"),
        size: Some(Vec3i { x: 8, y: 9, z: 10 }),
    });
    input.apply_screen_requests(&mut world, &mut counters, &commands);
    let state = input.test_instance_block_editor_hud_state().unwrap();
    assert_eq!(state.fields[1].text(), "8");
    assert_eq!(state.fields[2].text(), "9");
    assert_eq!(state.fields[3].text(), "10");
    assert_eq!(state.response_status.unwrap().text(), "Sized");
    assert_eq!(counters.test_instance_block_action_commands_queued, 3);
}

#[test]
fn test_instance_block_editor_buttons_send_clamped_exact_actions_and_close_once() {
    let pos = BlockPos { x: 0, y: 0, z: 0 };
    let (tx, mut rx) = mpsc::channel(16);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = world_with_test_instance_block(pos);
    let layout =
        bbb_client_render::hud_test_instance_block_editor_layout(PhysicalSize::new(320, 240));
    assert!(open_block_editor(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        pos
    ));
    assert!(matches!(
        rx.try_recv(),
        Ok(NetCommand::TestInstanceBlockAction(
            TestInstanceBlockAction {
                action: TestInstanceBlockActionKind::Init,
                ..
            }
        ))
    ));

    assert!(click_test_instance_editor(
        &mut input,
        &mut counters,
        &commands,
        layout.rotation,
    ));
    assert!(click_test_instance_editor(
        &mut input,
        &mut counters,
        &commands,
        layout.save,
    ));
    assert!(input.test_instance_block_editor_is_active());
    assert!(rx.try_recv().is_err(), "rotated Save is disabled");
    assert!(click_test_instance_editor(
        &mut input,
        &mut counters,
        &commands,
        layout.include_entities,
    ));
    for _ in 0..3 {
        assert!(click_test_instance_editor(
            &mut input,
            &mut counters,
            &commands,
            layout.rotation,
        ));
    }
    assert!(click_test_instance_editor(
        &mut input,
        &mut counters,
        &commands,
        layout.size[0],
    ));
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::ControlLeft),
        ElementState::Pressed,
    );
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::KeyA),
        ElementState::Pressed,
    );
    handle_text_input(&mut input, &mut counters, &mut world, &commands, "99");
    assert!(click_test_instance_editor(
        &mut input,
        &mut counters,
        &commands,
        layout.run,
    ));
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::TestInstanceBlockAction(TestInstanceBlockAction {
            pos: ProtocolBlockPos { x: 0, y: 0, z: 0 },
            action: TestInstanceBlockActionKind::Run,
            test: Some("example:initial".to_string()),
            size: Vec3i { x: 48, y: 4, z: 5 },
            rotation: TestInstanceBlockRotation::None,
            ignore_entities: true,
        })
    );
    assert!(!input.test_instance_block_editor_is_active());
    assert!(input.screen.test_instance_block().is_none());
    assert!(rx.try_recv().is_err());
}

#[test]
fn test_instance_block_editor_reset_save_and_done_buttons_map_to_exact_actions() {
    let pos = BlockPos { x: 0, y: 0, z: 0 };
    let layout =
        bbb_client_render::hud_test_instance_block_editor_layout(PhysicalSize::new(320, 240));
    for (expected_action, rect) in [
        (TestInstanceBlockActionKind::Reset, layout.reset),
        (TestInstanceBlockActionKind::Save, layout.save),
        (TestInstanceBlockActionKind::Set, layout.done),
    ] {
        let (tx, mut rx) = mpsc::channel(4);
        let commands = Some(tx);
        let mut input = ClientInputState::new(true);
        let mut counters = NetCounters::default();
        let mut world = world_with_test_instance_block(pos);
        assert!(open_block_editor(
            &mut input,
            &mut world,
            &mut counters,
            &commands,
            pos
        ));
        assert!(matches!(
            rx.try_recv(),
            Ok(NetCommand::TestInstanceBlockAction(
                TestInstanceBlockAction {
                    action: TestInstanceBlockActionKind::Init,
                    ..
                }
            ))
        ));
        assert!(click_test_instance_editor(
            &mut input,
            &mut counters,
            &commands,
            rect,
        ));
        assert_eq!(
            rx.try_recv().unwrap(),
            NetCommand::TestInstanceBlockAction(TestInstanceBlockAction {
                pos: ProtocolBlockPos { x: 0, y: 0, z: 0 },
                action: expected_action,
                test: Some("example:initial".to_string()),
                size: Vec3i { x: 3, y: 4, z: 5 },
                rotation: TestInstanceBlockRotation::None,
                ignore_entities: false,
            })
        );
        assert_eq!(counters.test_instance_block_action_commands_queued, 2);
        assert!(!input.test_instance_block_editor_is_active());
        assert!(input.screen.test_instance_block().is_none());
        assert!(rx.try_recv().is_err());
    }
}

#[test]
fn test_instance_block_editor_cancel_escape_and_right_click_use_are_exact() {
    let pos = BlockPos { x: 0, y: 0, z: 0 };
    let (tx, mut rx) = mpsc::channel(12);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = world_with_test_instance_block(pos);

    assert!(mouse::start_use_item(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        Some(CrosshairTarget::Block(
            crate::crosshair::CrosshairBlockHit {
                pos,
                face: ProtocolDirection::Up,
                cursor: [0.25, 0.5, 0.75],
                inside: false,
            }
        )),
        None,
    ));
    assert!(input.test_instance_block_editor_is_active());
    assert!(matches!(
        rx.try_recv(),
        Ok(NetCommand::TestInstanceBlockAction(
            TestInstanceBlockAction {
                action: TestInstanceBlockActionKind::Init,
                ..
            }
        ))
    ));
    assert!(matches!(rx.try_recv(), Ok(NetCommand::UseItemOn(_))));
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::Swing(InteractionHand::MainHand)
    );

    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::Escape),
        ElementState::Pressed,
    );
    assert!(!input.test_instance_block_editor_is_active());
    assert!(rx.try_recv().is_err());

    assert!(open_block_editor(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        pos
    ));
    assert!(matches!(
        rx.try_recv(),
        Ok(NetCommand::TestInstanceBlockAction(
            TestInstanceBlockAction {
                action: TestInstanceBlockActionKind::Init,
                ..
            }
        ))
    ));
    let cancel =
        bbb_client_render::hud_test_instance_block_editor_layout(PhysicalSize::new(320, 240))
            .cancel;
    assert!(click_test_instance_editor(
        &mut input,
        &mut counters,
        &commands,
        cancel,
    ));
    assert!(!input.test_instance_block_editor_is_active());
    assert!(rx.try_recv().is_err());

    assert!(open_block_editor(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        pos
    ));
    assert!(matches!(
        rx.try_recv(),
        Ok(NetCommand::TestInstanceBlockAction(
            TestInstanceBlockAction {
                action: TestInstanceBlockActionKind::Init,
                ..
            }
        ))
    ));
    world.apply_open_screen(ProtocolOpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_GENERIC_9X3_ID,
        title: bbb_protocol::RichTextComponent::plain("Chest"),
    });
    input.apply_screen_requests(&mut world, &mut counters, &None);
    assert!(!input.test_instance_block_editor_is_active());
    assert!(input.screen.test_instance_block().is_none());
    assert_eq!(counters.test_instance_block_action_commands_queued, 3);
    assert_eq!(counters.use_item_on_commands_queued, 1);
}

#[test]
fn jigsaw_block_editor_preloads_and_done_sends_exact_set_once() {
    let pos = BlockPos { x: 0, y: 0, z: 0 };
    let (tx, mut rx) = mpsc::channel(4);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = world_with_jigsaw_block(pos, "up_north");
    assert!(open_block_editor(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        pos
    ));

    let hud = input.jigsaw_block_editor_hud_state().unwrap();
    assert_eq!(hud.fields[0].text(), "example:village/houses");
    assert_eq!(hud.fields[1].text(), "example:entry");
    assert_eq!(hud.fields[2].text(), "example:target");
    assert_eq!(hud.fields[3].text(), "minecraft:oak_planks[axis=y]");
    assert_eq!(hud.fields[4].text(), "-2");
    assert_eq!(hud.fields[5].text(), "3");
    assert_eq!(hud.focus_index, 0);
    assert_eq!(hud.joint, JigsawJoint::Rollable);
    assert!(hud.joint_editable);
    assert_eq!(hud.levels, 0);
    assert!(hud.keep_jigsaws);
    assert!(hud.controls_enabled);

    let editor = input.screen.jigsaw_block_mut().unwrap();
    for (field, value) in editor.fields.iter_mut().zip([
        "example:new_pool",
        "new_entry",
        "example:new_target",
        "minecraft:stone",
        "-7",
        "bad",
    ]) {
        field.set_text(value, None);
    }
    editor.joint = JigsawJoint::Aligned;
    let done = bbb_client_render::hud_jigsaw_block_editor_layout(PhysicalSize::new(320, 240)).done;
    assert!(click_jigsaw_block_editor(
        &mut input,
        &mut counters,
        &commands,
        done,
    ));
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::SetJigsawBlock(SetJigsawBlock {
            pos: ProtocolBlockPos { x: 0, y: 0, z: 0 },
            name: "minecraft:new_entry".to_string(),
            target: "example:new_target".to_string(),
            pool: "example:new_pool".to_string(),
            final_state: "minecraft:stone".into(),
            joint: JigsawJoint::Aligned,
            selection_priority: -7,
            placement_priority: 0,
        })
    );
    assert_eq!(counters.set_jigsaw_block_commands_queued, 1);
    assert_eq!(counters.jigsaw_generate_commands_queued, 0);
    assert!(!input.jigsaw_block_editor_is_active());
    assert!(input.screen.jigsaw_block().is_none());
    assert!(rx.try_recv().is_err());
}

#[test]
fn jigsaw_generate_uses_exact_slider_toggle_joint_and_set_first_order() {
    let pos = BlockPos { x: 0, y: 0, z: 0 };
    let (tx, mut rx) = mpsc::channel(4);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = world_with_jigsaw_block(pos, "down_south");
    assert!(open_block_editor(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        pos
    ));
    let layout = bbb_client_render::hud_jigsaw_block_editor_layout(PhysicalSize::new(320, 240));

    assert!(input.handle_jigsaw_block_editor_mouse_input(
        &mut counters,
        &commands,
        MouseButton::Left,
        ElementState::Pressed,
        Some(PhysicalPosition::new(
            f64::from(layout.levels.x + 50),
            f64::from(layout.levels.y + 1),
        )),
        PhysicalSize::new(320, 240),
        &FixedCommandSuggestionTextMeasurer,
    ));
    assert_eq!(input.jigsaw_block_editor_hud_state().unwrap().levels, 10);
    assert!(input.handle_jigsaw_block_editor_cursor_moved(
        Some(PhysicalPosition::new(
            f64::from(layout.levels.x + 96),
            f64::from(layout.levels.y + 1),
        )),
        PhysicalSize::new(320, 240),
    ));
    assert_eq!(input.jigsaw_block_editor_hud_state().unwrap().levels, 20);
    assert!(input.handle_jigsaw_block_editor_mouse_input(
        &mut counters,
        &commands,
        MouseButton::Left,
        ElementState::Released,
        None,
        PhysicalSize::new(320, 240),
        &FixedCommandSuggestionTextMeasurer,
    ));
    assert!(click_jigsaw_block_editor(
        &mut input,
        &mut counters,
        &commands,
        layout.keep_jigsaws,
    ));
    assert!(click_jigsaw_block_editor(
        &mut input,
        &mut counters,
        &commands,
        layout.joint,
    ));
    let hud = input.jigsaw_block_editor_hud_state().unwrap();
    assert!(!hud.keep_jigsaws);
    assert_eq!(hud.joint, JigsawJoint::Aligned);

    assert!(click_jigsaw_block_editor(
        &mut input,
        &mut counters,
        &commands,
        layout.generate,
    ));
    assert!(matches!(rx.try_recv(), Ok(NetCommand::SetJigsawBlock(_))));
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::JigsawGenerate(JigsawGenerate {
            pos: ProtocolBlockPos { x: 0, y: 0, z: 0 },
            levels: 20,
            keep_jigsaws: false,
        })
    );
    assert_eq!(counters.set_jigsaw_block_commands_queued, 1);
    assert_eq!(counters.jigsaw_generate_commands_queued, 1);
    assert!(rx.try_recv().is_err());
}

#[test]
fn jigsaw_editor_validation_limits_cancel_escape_and_screen_replacement_are_local() {
    let pos = BlockPos { x: 0, y: 0, z: 0 };
    let (tx, mut rx) = mpsc::channel(8);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = world_with_jigsaw_block(pos, "up_north");
    assert!(open_block_editor(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        pos
    ));
    let layout = bbb_client_render::hud_jigsaw_block_editor_layout(PhysicalSize::new(320, 240));
    assert!(input.handle_jigsaw_block_editor_key(
        &mut counters,
        &commands,
        KeyCode::Tab,
        true,
        &mut None
    ));
    assert_eq!(
        input.jigsaw_block_editor_hud_state().unwrap().focus_index,
        1
    );
    input.set_shift_key(KeyCode::ShiftLeft, true);
    assert!(input.handle_jigsaw_block_editor_key(
        &mut counters,
        &commands,
        KeyCode::Tab,
        true,
        &mut None
    ));
    input.set_shift_key(KeyCode::ShiftLeft, false);
    assert_eq!(
        input.jigsaw_block_editor_hud_state().unwrap().focus_index,
        0
    );
    input.screen.jigsaw_block_mut().unwrap().fields[0].set_text("Bad Pool", None);
    assert!(
        !input
            .jigsaw_block_editor_hud_state()
            .unwrap()
            .controls_enabled
    );
    assert!(click_jigsaw_block_editor(
        &mut input,
        &mut counters,
        &commands,
        layout.done,
    ));
    assert!(click_jigsaw_block_editor(
        &mut input,
        &mut counters,
        &commands,
        layout.generate,
    ));
    assert!(input.handle_jigsaw_block_editor_key(
        &mut counters,
        &commands,
        KeyCode::Enter,
        true,
        &mut None
    ));
    assert!(input.jigsaw_block_editor_is_active());
    assert!(rx.try_recv().is_err());

    {
        let editor = input.screen.jigsaw_block_mut().unwrap();
        editor.focus = jigsaw_block::JigsawBlockEditorFocus::Pool;
        editor.fields[0].set_cursor(editor.fields[0].value().len(), None);
        editor.fields[0].set_selection(0, None);
    }
    assert!(input.handle_jigsaw_block_editor_text(&"a".repeat(200)));
    assert_eq!(
        input.jigsaw_block_editor_hud_state().unwrap().fields[0]
            .text()
            .encode_utf16()
            .count(),
        128
    );
    {
        let editor = input.screen.jigsaw_block_mut().unwrap();
        editor.focus = jigsaw_block::JigsawBlockEditorFocus::FinalState;
        editor.fields[3].set_cursor(editor.fields[3].value().len(), None);
        editor.fields[3].set_selection(0, None);
    }
    assert!(input.handle_jigsaw_block_editor_text(&"😀".repeat(200)));
    assert_eq!(
        input.jigsaw_block_editor_hud_state().unwrap().fields[3]
            .text()
            .encode_utf16()
            .count(),
        256
    );
    assert!(click_jigsaw_block_editor(
        &mut input,
        &mut counters,
        &commands,
        layout.cancel,
    ));
    assert!(rx.try_recv().is_err());

    assert!(mouse::start_use_item(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        Some(CrosshairTarget::Block(
            crate::crosshair::CrosshairBlockHit {
                pos,
                face: ProtocolDirection::Up,
                cursor: [0.25, 0.5, 0.75],
                inside: false,
            }
        )),
        None,
    ));
    assert!(input.jigsaw_block_editor_is_active());
    assert!(matches!(rx.try_recv(), Ok(NetCommand::UseItemOn(_))));
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::Swing(InteractionHand::MainHand)
    );
    assert!(input.handle_jigsaw_block_editor_key(
        &mut counters,
        &commands,
        KeyCode::Escape,
        true,
        &mut None
    ));
    assert!(rx.try_recv().is_err());

    assert!(open_block_editor(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        pos
    ));
    world.apply_open_screen(ProtocolOpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_GENERIC_9X3_ID,
        title: bbb_protocol::RichTextComponent::plain("Chest"),
    });
    input.apply_screen_requests(&mut world, &mut counters, &None);
    assert!(!input.jigsaw_block_editor_is_active());
    assert!(input.screen.jigsaw_block().is_none());
    assert_eq!(counters.set_jigsaw_block_commands_queued, 0);
    assert_eq!(counters.jigsaw_generate_commands_queued, 0);

    let mut horizontal_world = world_with_jigsaw_block(pos, "north_up");
    let mut horizontal_input = ClientInputState::new(true);
    assert!(open_block_editor(
        &mut horizontal_input,
        &mut horizontal_world,
        &mut counters,
        &None,
        pos
    ));
    assert!(
        !horizontal_input
            .jigsaw_block_editor_hud_state()
            .unwrap()
            .joint_editable
    );
    assert!(click_jigsaw_block_editor(
        &mut horizontal_input,
        &mut counters,
        &None,
        layout.joint,
    ));
    assert_eq!(
        horizontal_input
            .jigsaw_block_editor_hud_state()
            .unwrap()
            .joint,
        JigsawJoint::Rollable
    );
}

#[test]
fn jigsaw_authoritative_refresh_preserves_unsent_native_fields_in_same_open() {
    let pos = BlockPos { x: 0, y: 0, z: 0 };
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = world_with_jigsaw_block(pos, "up_north");
    assert!(open_block_editor(
        &mut input,
        &mut world,
        &mut counters,
        &None,
        pos
    ));
    input.screen.jigsaw_block_mut().unwrap().fields[1].set_text("example:unsent", None);

    assert!(world
        .apply_block_entity_data(BlockEntityData {
            pos: ProtocolBlockPos { x: 0, y: 0, z: 0 },
            block_entity_type_id: VANILLA_BLOCK_ENTITY_TYPE_JIGSAW_ID,
            raw_nbt: vec![10, 0],
        })
        .unwrap());
    assert_eq!(
        input.jigsaw_block_editor_hud_state().unwrap().fields[1].text(),
        "example:unsent"
    );
    assert_eq!(
        world.jigsaw_block_data_at(pos).unwrap().name,
        "minecraft:empty"
    );
}

#[test]
fn structure_block_editor_preloads_and_done_sends_clamped_exact_packet() {
    let pos = BlockPos { x: 0, y: 0, z: 0 };
    let (tx, mut rx) = mpsc::channel(4);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = world_with_structure_block(pos);
    assert!(open_block_editor(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        pos
    ));

    let hud = input.structure_block_editor_hud_state().unwrap();
    assert_eq!(hud.fields[0].text(), "example:initial");
    assert_eq!(hud.fields[1].text(), "1");
    assert_eq!(hud.fields[2].text(), "-2");
    assert_eq!(hud.fields[3].text(), "3");
    assert_eq!(hud.fields[4].text(), "4");
    assert_eq!(hud.fields[5].text(), "5");
    assert_eq!(hud.fields[6].text(), "6");
    assert_eq!(hud.fields[7].text(), "0.75");
    assert_eq!(hud.fields[8].text(), "9");
    assert_eq!(hud.fields[9].text(), "initial data");
    assert_eq!(hud.mode, StructureBlockMode::Load);
    assert_eq!(hud.mirror, StructureBlockMirror::FrontBack);
    assert_eq!(hud.rotation, StructureBlockRotation::Clockwise90);
    assert!(hud.include_entities);
    assert!(hud.strict);
    assert!(!hud.show_bounding_box);

    let editor = input.screen.structure_block_mut().unwrap();
    for (field, value) in editor.fields.iter_mut().zip([
        "example:saved",
        "99",
        "-99",
        "invalid",
        "99",
        "-1",
        "invalid",
        "2.5",
        "invalid",
        "payload",
    ]) {
        field.set_text(value, None);
    }
    editor.mode = StructureBlockMode::Load;
    editor.mirror = StructureBlockMirror::LeftRight;
    editor.rotation = StructureBlockRotation::Counterclockwise90;
    editor.include_entities = false;
    editor.strict = false;
    editor.show_air = true;
    editor.show_bounding_box = true;

    let done =
        bbb_client_render::hud_structure_block_editor_layout(PhysicalSize::new(320, 240)).done;
    assert!(click_structure_block_editor(
        &mut input,
        &mut counters,
        &commands,
        done,
    ));
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::SetStructureBlock(SetStructureBlock {
            pos: ProtocolBlockPos { x: 0, y: 0, z: 0 },
            update: StructureBlockUpdateKind::UpdateData,
            mode: StructureBlockMode::Load,
            name: "example:saved".to_string(),
            offset: ProtocolBlockPos {
                x: 48,
                y: -48,
                z: 0
            },
            size: Vec3i { x: 48, y: 0, z: 0 },
            mirror: StructureBlockMirror::LeftRight,
            rotation: StructureBlockRotation::Counterclockwise90,
            data: "payload".into(),
            ignore_entities: true,
            strict: false,
            show_air: true,
            show_bounding_box: true,
            integrity: 1.0,
            seed: 0,
        })
    );
    assert_eq!(counters.set_structure_block_commands_queued, 1);
    assert!(!input.structure_block_editor_is_active());
    assert!(input.screen.structure_block().is_none());
    assert!(rx.try_recv().is_err());
}

#[test]
fn structure_block_editor_save_load_and_detect_buttons_map_to_exact_updates() {
    let pos = BlockPos { x: 0, y: 0, z: 0 };
    let layout = bbb_client_render::hud_structure_block_editor_layout(PhysicalSize::new(320, 240));
    for (mode, expected_update, rect) in [
        (
            StructureBlockMode::Save,
            StructureBlockUpdateKind::SaveArea,
            layout.action,
        ),
        (
            StructureBlockMode::Load,
            StructureBlockUpdateKind::LoadArea,
            layout.action,
        ),
        (
            StructureBlockMode::Save,
            StructureBlockUpdateKind::ScanArea,
            layout.detect,
        ),
    ] {
        let (tx, mut rx) = mpsc::channel(2);
        let commands = Some(tx);
        let mut input = ClientInputState::new(true);
        let mut counters = NetCounters::default();
        let mut world = world_with_structure_block(pos);
        assert!(open_block_editor(
            &mut input,
            &mut world,
            &mut counters,
            &commands,
            pos
        ));
        input.screen.structure_block_mut().unwrap().mode = mode;
        assert!(click_structure_block_editor(
            &mut input,
            &mut counters,
            &commands,
            rect,
        ));
        let NetCommand::SetStructureBlock(packet) = rx.try_recv().unwrap() else {
            panic!("structure block action must queue its typed packet");
        };
        assert_eq!(packet.update, expected_update);
        assert_eq!(packet.mode, mode);
        assert_eq!(counters.set_structure_block_commands_queued, 1);
        assert!(!input.structure_block_editor_is_active());
        assert!(rx.try_recv().is_err());
    }
}

#[test]
fn structure_block_editor_mode_visibility_and_controls_are_exact() {
    use structure_block::StructureBlockEditorFocus as Focus;

    let pos = BlockPos { x: 0, y: 0, z: 0 };
    let commands = None;
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = world_with_structure_block(pos);
    assert!(open_block_editor(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        pos
    ));

    for (mode, focuses) in [
        (
            StructureBlockMode::Save,
            vec![
                Focus::Name,
                Focus::OffsetX,
                Focus::OffsetY,
                Focus::OffsetZ,
                Focus::SizeX,
                Focus::SizeY,
                Focus::SizeZ,
            ],
        ),
        (
            StructureBlockMode::Load,
            vec![
                Focus::Name,
                Focus::OffsetX,
                Focus::OffsetY,
                Focus::OffsetZ,
                Focus::Integrity,
                Focus::Seed,
            ],
        ),
        (StructureBlockMode::Corner, vec![Focus::Name]),
        (StructureBlockMode::Data, vec![Focus::Data]),
    ] {
        input.screen.structure_block_mut().unwrap().mode = mode;
        input.screen.structure_block_mut().unwrap().focus = focuses[0];
        for expected in &focuses {
            assert_eq!(input.screen.structure_block().unwrap().focus, *expected);
            input.handle_structure_block_editor_key(
                &mut counters,
                &commands,
                KeyCode::Tab,
                true,
                &mut None,
            );
        }
        assert_eq!(input.screen.structure_block().unwrap().focus, focuses[0]);
    }

    let layout = bbb_client_render::hud_structure_block_editor_layout(PhysicalSize::new(320, 240));
    let editor = input.screen.structure_block_mut().unwrap();
    editor.mode = StructureBlockMode::Load;
    editor.focus = Focus::Name;
    for rect in [
        layout.mirror,
        layout.rotations[2],
        layout.include_entities,
        layout.strict,
        layout.option,
    ] {
        assert!(click_structure_block_editor(
            &mut input,
            &mut counters,
            &commands,
            rect,
        ));
    }
    let hud = input.structure_block_editor_hud_state().unwrap();
    assert_eq!(hud.mirror, StructureBlockMirror::None);
    assert_eq!(hud.rotation, StructureBlockRotation::Clockwise180);
    assert!(!hud.include_entities);
    assert!(!hud.strict);
    assert!(hud.show_bounding_box);

    input.screen.structure_block_mut().unwrap().mode = StructureBlockMode::Save;
    assert!(click_structure_block_editor(
        &mut input,
        &mut counters,
        &commands,
        layout.option,
    ));
    assert!(!input.structure_block_editor_hud_state().unwrap().show_air);
}

#[test]
fn structure_block_editor_filters_names_cycles_modes_and_cancels_without_packet() {
    let pos = BlockPos { x: 0, y: 0, z: 0 };
    let (tx, mut rx) = mpsc::channel(4);
    let commands = Some(tx);
    let mut input = ClientInputState::new(true);
    let mut counters = NetCounters::default();
    let mut world = world_with_structure_block(pos);
    assert!(open_block_editor(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        pos
    ));
    {
        let editor = input.screen.structure_block_mut().unwrap();
        editor.focus = structure_block::StructureBlockEditorFocus::Name;
        editor.fields[0].set_cursor(editor.fields[0].value().len(), None);
        editor.fields[0].set_selection(0, None);
    }
    assert!(input.handle_structure_block_editor_text("Bad Name"));
    assert!(input.handle_structure_block_editor_text(":path/sub-1"));
    assert_eq!(
        input.structure_block_editor_hud_state().unwrap().fields[0].text(),
        "adame:path/sub-1"
    );

    let layout = bbb_client_render::hud_structure_block_editor_layout(PhysicalSize::new(320, 240));
    assert!(click_structure_block_editor(
        &mut input,
        &mut counters,
        &commands,
        layout.mode,
    ));
    assert_eq!(
        input.structure_block_editor_hud_state().unwrap().mode,
        StructureBlockMode::Corner
    );
    input.screen.structure_block_mut().unwrap().mode = StructureBlockMode::Corner;
    input.pressed_keys.insert(KeyCode::AltLeft);
    assert!(click_structure_block_editor(
        &mut input,
        &mut counters,
        &commands,
        layout.mode,
    ));
    let hud = input.structure_block_editor_hud_state().unwrap();
    assert_eq!(hud.mode, StructureBlockMode::Data);
    assert_eq!(hud.focus_index, 9);

    assert!(click_structure_block_editor(
        &mut input,
        &mut counters,
        &commands,
        layout.cancel,
    ));
    assert!(!input.structure_block_editor_is_active());
    assert_eq!(counters.set_structure_block_commands_queued, 0);
    assert!(rx.try_recv().is_err());

    assert!(mouse::start_use_item(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        Some(CrosshairTarget::Block(
            crate::crosshair::CrosshairBlockHit {
                pos,
                face: ProtocolDirection::Up,
                cursor: [0.25, 0.5, 0.75],
                inside: false,
            }
        )),
        None,
    ));
    assert!(input.structure_block_editor_is_active());
    assert!(matches!(rx.try_recv(), Ok(NetCommand::UseItemOn(_))));
    assert_eq!(
        rx.try_recv().unwrap(),
        NetCommand::Swing(InteractionHand::MainHand)
    );
    handle_key_input(
        &mut input,
        &mut counters,
        &mut world,
        &commands,
        PhysicalKey::Code(KeyCode::Escape),
        ElementState::Pressed,
    );
    assert!(!input.structure_block_editor_is_active());
    assert!(rx.try_recv().is_err());

    assert!(open_block_editor(
        &mut input,
        &mut world,
        &mut counters,
        &commands,
        pos
    ));
    world.apply_open_screen(ProtocolOpenScreen {
        container_id: 7,
        menu_type_id: VANILLA_MENU_TYPE_GENERIC_9X3_ID,
        title: bbb_protocol::RichTextComponent::plain("Chest"),
    });
    input.apply_screen_requests(&mut world, &mut counters, &None);
    assert!(!input.structure_block_editor_is_active());
    assert!(input.screen.structure_block().is_none());
    assert_eq!(counters.set_structure_block_commands_queued, 0);
    assert!(rx.try_recv().is_err());
}

fn open_block_editor(
    input: &mut ClientInputState,
    world: &mut WorldStore,
    counters: &mut NetCounters,
    commands: &Option<mpsc::Sender<NetCommand>>,
    pos: BlockPos,
) -> bool {
    // The fixture's login asked to close level-bound screens before this one.
    world.take_screen_requests();
    if let Some(editor) = world.structure_block_editor_at(pos) {
        input.open_structure_block_editor(world, counters, commands, editor);
    } else if let Some(editor) = world.jigsaw_block_editor_at(pos) {
        input.open_jigsaw_block_editor(world, counters, commands, editor);
    } else if let Some(editor) = world.test_block_editor_at(pos) {
        input.open_test_block_editor(world, counters, commands, editor);
    } else if let Some(editor) = world.test_instance_block_editor_at(pos) {
        input.open_test_instance_block_editor(world, counters, commands, editor);
    } else {
        return false;
    }
    true
}
