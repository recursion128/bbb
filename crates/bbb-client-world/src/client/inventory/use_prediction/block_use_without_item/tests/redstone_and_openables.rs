use super::*;

fn comparator_click_success(subtract: bool) -> LocalTargetUsePrediction {
    success(LocalInteractionSuccess {
        sound_action: Some(LocalInteractionSoundAction::BlockPositioned {
            pos: POS,
            sound_resource_id: "minecraft:block.comparator.click",
            volume_bits: 0.3_f32.to_bits(),
            pitch_bits: if subtract { 0.55_f32 } else { 0.5_f32 }.to_bits(),
        }),
        ..LocalInteractionSuccess::client_item_interaction()
    })
}

#[test]
fn button_press_and_powered_consume_match_vanilla_results() {
    let mut world = block_use_world(
        "minecraft:stone_button",
        [("face", "wall"), ("facing", "north"), ("powered", "false")],
        VANILLA_GAME_TYPE_SURVIVAL_ID as i8,
    );
    assert_eq!(
        use_main_hand(&mut world, 11),
        success(LocalInteractionSuccess {
            sound_action: Some(LocalInteractionSoundAction::BlockPositioned {
                pos: POS,
                sound_resource_id: "minecraft:block.stone_button.click_on",
                volume_bits: 1.0_f32.to_bits(),
                pitch_bits: 1.0_f32.to_bits(),
            }),
            ..LocalInteractionSuccess::client_item_interaction()
        })
    );
    assert_eq!(block_property(&world, "powered"), Some("true".to_string()));
    assert_eq!(world.local_block_predictions()[0].sequence, 11);

    assert_eq!(
        use_main_hand(&mut world, 12),
        success(LocalInteractionSuccess::consume(false))
    );
    assert_eq!(world.local_block_predictions().len(), 1);
}

#[test]
fn lever_only_requests_client_particle_when_switching_toward_powered() {
    let mut world = block_use_world(
        "minecraft:lever",
        [
            ("face", "ceiling"),
            ("facing", "east"),
            ("powered", "false"),
        ],
        VANILLA_GAME_TYPE_SURVIVAL_ID as i8,
    );
    assert_eq!(
        use_main_hand(&mut world, 1),
        success(LocalInteractionSuccess {
            particle_action: Some(LocalInteractionParticleAction::LeverActivation {
                pos: POS,
                facing: Direction::East,
                face: LocalLeverAttachFace::Ceiling,
            }),
            ..LocalInteractionSuccess::client_item_interaction()
        })
    );
    assert_eq!(
        block_property(&world, "powered"),
        Some("false".to_string()),
        "the client-side branch does not pull the lever state"
    );
    set_test_block(
        &mut world,
        state_id(
            "minecraft:lever",
            [("face", "ceiling"), ("facing", "east"), ("powered", "true")],
        ),
    );
    assert_eq!(
        use_main_hand(&mut world, 2),
        success(LocalInteractionSuccess::client_item_interaction())
    );
}

#[test]
fn wooden_and_copper_openables_toggle_while_iron_passes_to_item_use() {
    for (name, properties, expected_sound) in [
        (
            "minecraft:oak_door",
            vec![
                ("facing", "north"),
                ("half", "lower"),
                ("hinge", "left"),
                ("open", "false"),
                ("powered", "false"),
            ],
            "minecraft:block.wooden_door.open",
        ),
        (
            "minecraft:copper_trapdoor",
            vec![
                ("facing", "north"),
                ("half", "bottom"),
                ("open", "false"),
                ("powered", "false"),
                ("waterlogged", "false"),
            ],
            "minecraft:block.copper_trapdoor.open",
        ),
    ] {
        let mut world = block_use_world_vec(name, properties, VANILLA_GAME_TYPE_SURVIVAL_ID as i8);
        assert_eq!(
            use_main_hand(&mut world, 4),
            success(LocalInteractionSuccess {
                sound_action: Some(LocalInteractionSoundAction::BlockToggleSound {
                    pos: POS,
                    sound_resource_id: expected_sound,
                }),
                ..LocalInteractionSuccess::client_item_interaction()
            })
        );
        assert_eq!(block_property(&world, "open"), Some("true".to_string()));
    }

    for (name, properties) in [
        (
            "minecraft:iron_door",
            vec![
                ("facing", "north"),
                ("half", "lower"),
                ("hinge", "left"),
                ("open", "false"),
                ("powered", "false"),
            ],
        ),
        (
            "minecraft:iron_trapdoor",
            vec![
                ("facing", "north"),
                ("half", "bottom"),
                ("open", "false"),
                ("powered", "false"),
                ("waterlogged", "false"),
            ],
        ),
    ] {
        let mut world = block_use_world_vec(name, properties, VANILLA_GAME_TYPE_SURVIVAL_ID as i8);
        assert_eq!(use_main_hand(&mut world, 5), pass());
        assert!(world.local_block_predictions().is_empty());
    }
}

#[test]
fn fence_gate_uses_vanilla_player_direction_only_when_opening() {
    let mut world = block_use_world(
        "minecraft:oak_fence_gate",
        [
            ("facing", "north"),
            ("in_wall", "false"),
            ("open", "false"),
            ("powered", "false"),
        ],
        VANILLA_GAME_TYPE_SURVIVAL_ID as i8,
    );
    world.set_local_player_pose(LocalPlayerPoseState {
        y_rot: 0.0,
        ..LocalPlayerPoseState::default()
    });
    assert_eq!(
        use_main_hand(&mut world, 21),
        success(LocalInteractionSuccess {
            sound_action: Some(LocalInteractionSoundAction::BlockToggleSound {
                pos: POS,
                sound_resource_id: "minecraft:block.fence_gate.open",
            }),
            ..LocalInteractionSuccess::client_item_interaction()
        })
    );
    assert_eq!(block_property(&world, "open"), Some("true".to_string()));
    assert_eq!(
        block_property(&world, "facing"),
        Some("south".to_string()),
        "SOUTH player direction replaces opposite NORTH facing"
    );

    world.set_local_player_pose(LocalPlayerPoseState {
        y_rot: f32::NAN,
        ..LocalPlayerPoseState::default()
    });
    assert_eq!(
        use_main_hand(&mut world, 22),
        success(LocalInteractionSuccess {
            sound_action: Some(LocalInteractionSoundAction::BlockToggleSound {
                pos: POS,
                sound_resource_id: "minecraft:block.fence_gate.close",
            }),
            ..LocalInteractionSuccess::client_item_interaction()
        }),
        "closing does not query player direction"
    );
    assert_eq!(block_property(&world, "open"), Some("false".to_string()));
}

#[test]
fn repeater_cycles_delay_only_with_may_build_and_daylight_is_server_owned() {
    let mut survival = block_use_world(
        "minecraft:repeater",
        [
            ("delay", "4"),
            ("facing", "north"),
            ("locked", "false"),
            ("powered", "false"),
        ],
        VANILLA_GAME_TYPE_SURVIVAL_ID as i8,
    );
    assert_eq!(
        use_main_hand(&mut survival, 31),
        success(LocalInteractionSuccess::client_item_interaction())
    );
    assert_eq!(block_property(&survival, "delay"), Some("1".to_string()));

    let mut adventure = block_use_world(
        "minecraft:repeater",
        [
            ("delay", "2"),
            ("facing", "north"),
            ("locked", "false"),
            ("powered", "false"),
        ],
        VANILLA_GAME_TYPE_ADVENTURE_ID as i8,
    );
    assert_eq!(use_main_hand(&mut adventure, 32), pass());
    assert!(adventure.local_block_predictions().is_empty());

    let mut detector = block_use_world(
        "minecraft:daylight_detector",
        [("inverted", "false"), ("power", "7")],
        VANILLA_GAME_TYPE_SURVIVAL_ID as i8,
    );
    assert_eq!(
        use_main_hand(&mut detector, 33),
        success(LocalInteractionSuccess::client_item_interaction())
    );
    assert_eq!(
        block_property(&detector, "inverted"),
        Some("false".to_string())
    );
    assert!(detector.local_block_predictions().is_empty());
}

#[test]
fn comparator_cycles_mode_with_may_build_and_keeps_server_output_state() {
    let mut survival = block_use_world(
        "minecraft:comparator",
        [
            ("facing", "north"),
            ("mode", "compare"),
            ("powered", "true"),
        ],
        VANILLA_GAME_TYPE_SURVIVAL_ID as i8,
    );
    insert_air_chunk(&mut survival, ChunkPos { x: 0, z: -1 });
    insert_air_chunk(&mut survival, ChunkPos { x: -1, z: 0 });
    assert_eq!(
        use_main_hand(&mut survival, 41),
        comparator_click_success(true)
    );
    assert_eq!(
        block_property(&survival, "mode"),
        Some("subtract".to_string())
    );
    assert_eq!(
        block_property(&survival, "powered"),
        Some("true".to_string())
    );

    let mut adventure = block_use_world(
        "minecraft:comparator",
        [
            ("facing", "north"),
            ("mode", "subtract"),
            ("powered", "false"),
        ],
        VANILLA_GAME_TYPE_ADVENTURE_ID as i8,
    );
    assert_eq!(use_main_hand(&mut adventure, 42), pass());
    assert_eq!(
        block_property(&adventure, "mode"),
        Some("subtract".to_string())
    );
}

#[test]
fn comparator_refreshes_analog_output_power_and_side_input() {
    let mut world = block_use_world_with_entity(
        "minecraft:comparator",
        [
            ("facing", "north"),
            ("mode", "compare"),
            ("powered", "false"),
        ],
        VANILLA_GAME_TYPE_SURVIVAL_ID as i8,
        VANILLA_BLOCK_ENTITY_TYPE_COMPARATOR_ID,
    );
    insert_air_chunk(&mut world, ChunkPos { x: 0, z: -1 });
    insert_air_chunk(&mut world, ChunkPos { x: -1, z: 0 });
    insert_air_chunk(&mut world, ChunkPos { x: -1, z: -1 });
    set_block_at(
        &mut world,
        BlockPos { z: -1, ..POS },
        state_id("minecraft:cake", [("bites", "2")]),
    );

    assert_eq!(
        use_main_hand(&mut world, 43),
        comparator_click_success(true)
    );
    assert_eq!(block_property(&world, "mode").as_deref(), Some("subtract"));
    assert_eq!(block_property(&world, "powered").as_deref(), Some("true"));
    assert_eq!(world.comparator_output_at(POS), Some(10));

    set_block_at(
        &mut world,
        BlockPos { x: 1, ..POS },
        state_id("minecraft:redstone_block", []),
    );
    assert_eq!(
        use_main_hand(&mut world, 44),
        comparator_click_success(false)
    );
    assert_eq!(block_property(&world, "mode").as_deref(), Some("compare"));
    assert_eq!(block_property(&world, "powered").as_deref(), Some("false"));
    assert_eq!(world.comparator_output_at(POS), Some(0));
}

#[test]
fn comparator_reads_synchronized_decorated_pot_inventory() {
    let mut world = block_use_world_with_entity(
        "minecraft:comparator",
        [
            ("facing", "north"),
            ("mode", "compare"),
            ("powered", "false"),
        ],
        VANILLA_GAME_TYPE_SURVIVAL_ID as i8,
        VANILLA_BLOCK_ENTITY_TYPE_COMPARATOR_ID,
    );
    insert_air_chunk(&mut world, ChunkPos { x: 0, z: -1 });
    insert_air_chunk(&mut world, ChunkPos { x: -1, z: 0 });
    let pot_pos = BlockPos { z: -1, ..POS };
    set_block_at(
        &mut world,
        pot_pos,
        state_id(
            "minecraft:decorated_pot",
            [
                ("cracked", "false"),
                ("facing", "north"),
                ("waterlogged", "false"),
            ],
        ),
    );
    insert_block_entity_record(
        &mut world,
        pot_pos,
        VANILLA_BLOCK_ENTITY_TYPE_DECORATED_POT_ID,
        decorated_pot_item_nbt("minecraft:stone", 64),
    );
    assert_eq!(
        world.block_entity_type_id_at(pot_pos),
        Some(VANILLA_BLOCK_ENTITY_TYPE_DECORATED_POT_ID)
    );
    assert!(world
        .block_entity_raw_nbt_at(pot_pos)
        .is_some_and(|raw_nbt| !raw_nbt.is_empty()));

    assert_eq!(
        use_main_hand(&mut world, 45),
        comparator_click_success(true)
    );
    assert_eq!(block_property(&world, "mode").as_deref(), Some("subtract"));
    assert_eq!(world.comparator_output_at(POS), Some(15));
    assert_eq!(block_property(&world, "powered").as_deref(), Some("true"));
}

#[test]
fn comparator_requires_exactly_one_matching_item_frame() {
    let mut world = block_use_world_with_entity(
        "minecraft:comparator",
        [
            ("facing", "north"),
            ("mode", "compare"),
            ("powered", "false"),
        ],
        VANILLA_GAME_TYPE_SURVIVAL_ID as i8,
        VANILLA_BLOCK_ENTITY_TYPE_COMPARATOR_ID,
    );
    insert_air_chunk(&mut world, ChunkPos { x: 0, z: -1 });
    insert_air_chunk(&mut world, ChunkPos { x: -1, z: 0 });
    insert_air_chunk(&mut world, ChunkPos { x: -1, z: -1 });
    set_block_at(
        &mut world,
        BlockPos { z: -1, ..POS },
        state_id("minecraft:stone", []),
    );
    add_item_frame(&mut world, 701, BlockPos { z: -2, ..POS }, 2, 5);
    add_item_frame(&mut world, 702, BlockPos { z: -2, ..POS }, 3, 7);
    assert_eq!(
        world.entities.comparator_item_frame_analog_output(
            BlockPos { z: -2, ..POS },
            ItemFrameFacing::North,
        ),
        Some(Some(6))
    );

    assert_eq!(
        use_main_hand(&mut world, 46),
        comparator_click_success(true)
    );
    assert_eq!(world.comparator_output_at(POS), Some(6));

    add_item_frame(&mut world, 703, BlockPos { z: -2, ..POS }, 2, 7);
    assert_eq!(
        use_main_hand(&mut world, 47),
        comparator_click_success(false)
    );
    assert_eq!(world.comparator_output_at(POS), Some(0));
    assert_eq!(block_property(&world, "powered").as_deref(), Some("false"));
}

#[test]
fn comparator_reads_lectern_pages_and_active_sculk_frequency() {
    let mut world = block_use_world_with_entity(
        "minecraft:comparator",
        [
            ("facing", "north"),
            ("mode", "compare"),
            ("powered", "false"),
        ],
        VANILLA_GAME_TYPE_SURVIVAL_ID as i8,
        VANILLA_BLOCK_ENTITY_TYPE_COMPARATOR_ID,
    );
    insert_air_chunk(&mut world, ChunkPos { x: 0, z: -1 });
    insert_air_chunk(&mut world, ChunkPos { x: -1, z: 0 });
    let input_pos = BlockPos { z: -1, ..POS };
    set_block_at(
        &mut world,
        input_pos,
        state_id(
            "minecraft:lectern",
            [
                ("facing", "north"),
                ("has_book", "true"),
                ("powered", "false"),
            ],
        ),
    );
    insert_block_entity_record(
        &mut world,
        input_pos,
        bbb_protocol::block_entity_types::VANILLA_BLOCK_ENTITY_TYPE_LECTERN_ID,
        lectern_book_nbt(3, 1),
    );

    assert_eq!(
        use_main_hand(&mut world, 48),
        comparator_click_success(true)
    );
    assert_eq!(world.comparator_output_at(POS), Some(8));

    set_block_at(
        &mut world,
        input_pos,
        state_id(
            "minecraft:sculk_sensor",
            [
                ("power", "6"),
                ("sculk_sensor_phase", "active"),
                ("waterlogged", "false"),
            ],
        ),
    );
    insert_block_entity_record(
        &mut world,
        input_pos,
        bbb_protocol::block_entity_types::VANILLA_BLOCK_ENTITY_TYPE_SCULK_SENSOR_ID,
        int_nbt("last_vibration_frequency", 12),
    );
    assert_eq!(
        use_main_hand(&mut world, 49),
        comparator_click_success(false)
    );
    assert_eq!(world.comparator_output_at(POS), Some(12));

    // Zero-page lectern book must not panic on i32 clamp and yields redstone signal 15.
    set_block_at(
        &mut world,
        input_pos,
        state_id(
            "minecraft:lectern",
            [
                ("facing", "north"),
                ("has_book", "true"),
                ("powered", "false"),
            ],
        ),
    );
    insert_block_entity_record(
        &mut world,
        input_pos,
        bbb_protocol::block_entity_types::VANILLA_BLOCK_ENTITY_TYPE_LECTERN_ID,
        lectern_book_nbt(0, 0),
    );
    assert_eq!(
        use_main_hand(&mut world, 50),
        comparator_click_success(true)
    );
    assert_eq!(world.comparator_output_at(POS), Some(15));
}

#[test]
fn comparator_reads_and_blocks_chest_inventory_signal() {
    let mut world = block_use_world_with_entity(
        "minecraft:comparator",
        [
            ("facing", "north"),
            ("mode", "compare"),
            ("powered", "false"),
        ],
        VANILLA_GAME_TYPE_SURVIVAL_ID as i8,
        VANILLA_BLOCK_ENTITY_TYPE_COMPARATOR_ID,
    );
    insert_air_chunk(&mut world, ChunkPos { x: 0, z: -1 });
    insert_air_chunk(&mut world, ChunkPos { x: -1, z: 0 });
    let chest_pos = BlockPos { z: -1, ..POS };
    set_block_at(
        &mut world,
        chest_pos,
        state_id(
            "minecraft:chest",
            [
                ("facing", "north"),
                ("type", "single"),
                ("waterlogged", "false"),
            ],
        ),
    );
    insert_block_entity_record(
        &mut world,
        chest_pos,
        bbb_protocol::block_entity_types::VANILLA_BLOCK_ENTITY_TYPE_CHEST_ID,
        container_item_nbt(0, "minecraft:stone", 64),
    );

    assert_eq!(
        use_main_hand(&mut world, 50),
        comparator_click_success(true)
    );
    assert_eq!(world.comparator_output_at(POS), Some(1));

    set_block_at(
        &mut world,
        BlockPos {
            y: chest_pos.y + 1,
            ..chest_pos
        },
        state_id("minecraft:stone", []),
    );
    assert_eq!(
        use_main_hand(&mut world, 51),
        comparator_click_success(false)
    );
    assert_eq!(world.comparator_output_at(POS), Some(0));
}

#[test]
fn redstone_wire_toggles_cross_and_dot_with_preserved_power() {
    let mut world = block_use_world_vec(
        "minecraft:redstone_wire",
        wire_properties("side", "side", "side", "side", "9"),
        VANILLA_GAME_TYPE_SURVIVAL_ID as i8,
    );
    assert_eq!(
        use_main_hand(&mut world, 51),
        success(LocalInteractionSuccess::client_item_interaction())
    );
    assert_wire_sides(&world, ["none", "none", "none", "none"], "9");

    set_test_block(
        &mut world,
        state_id_vec(
            "minecraft:redstone_wire",
            wire_properties("none", "none", "none", "none", "9"),
        ),
    );
    assert_eq!(
        use_main_hand(&mut world, 52),
        success(LocalInteractionSuccess::client_item_interaction())
    );
    assert_wire_sides(&world, ["side", "side", "side", "side"], "9");
}

#[test]
fn redstone_wire_recomputes_real_side_and_up_connections() {
    let mut side = block_use_world_vec(
        "minecraft:redstone_wire",
        wire_properties("side", "side", "side", "side", "0"),
        VANILLA_GAME_TYPE_SURVIVAL_ID as i8,
    );
    set_block_at(
        &mut side,
        relative(POS, Direction::East),
        state_id_vec(
            "minecraft:repeater",
            vec![
                ("delay", "1"),
                ("facing", "east"),
                ("locked", "false"),
                ("powered", "false"),
            ],
        ),
    );
    assert_eq!(
        use_main_hand(&mut side, 61),
        success(LocalInteractionSuccess::client_item_interaction())
    );
    assert_wire_sides(&side, ["none", "side", "none", "side"], "0");

    let mut climb = block_use_world_vec(
        "minecraft:redstone_wire",
        wire_properties("side", "side", "side", "side", "4"),
        VANILLA_GAME_TYPE_SURVIVAL_ID as i8,
    );
    let east = relative(POS, Direction::East);
    set_block_at(&mut climb, east, state_id("minecraft:stone", []));
    set_block_at(
        &mut climb,
        above(east),
        state_id_vec(
            "minecraft:redstone_wire",
            wire_properties("none", "none", "none", "none", "4"),
        ),
    );
    assert_eq!(
        use_main_hand(&mut climb, 62),
        success(LocalInteractionSuccess::client_item_interaction())
    );
    assert_wire_sides(&climb, ["none", "up", "none", "side"], "4");

    let mut support_override = block_use_world_vec(
        "minecraft:redstone_wire",
        wire_properties("side", "side", "side", "side", "4"),
        VANILLA_GAME_TYPE_SURVIVAL_ID as i8,
    );
    set_block_at(
        &mut support_override,
        east,
        state_id("minecraft:soul_sand", []),
    );
    set_block_at(
        &mut support_override,
        above(east),
        state_id_vec(
            "minecraft:redstone_wire",
            wire_properties("none", "none", "none", "none", "4"),
        ),
    );
    assert_eq!(
        use_main_hand(&mut support_override, 63),
        success(LocalInteractionSuccess::client_item_interaction())
    );
    assert_wire_sides(&support_override, ["none", "up", "none", "side"], "4");

    let mut blocked_climb = block_use_world_vec(
        "minecraft:redstone_wire",
        wire_properties("side", "side", "side", "side", "4"),
        VANILLA_GAME_TYPE_SURVIVAL_ID as i8,
    );
    set_block_at(
        &mut blocked_climb,
        above(POS),
        state_id("minecraft:stone", []),
    );
    set_block_at(&mut blocked_climb, east, state_id("minecraft:stone", []));
    set_block_at(
        &mut blocked_climb,
        above(east),
        state_id_vec(
            "minecraft:redstone_wire",
            wire_properties("none", "none", "none", "none", "4"),
        ),
    );
    assert_eq!(
        use_main_hand(&mut blocked_climb, 64),
        success(LocalInteractionSuccess::client_item_interaction())
    );
    assert_wire_sides(&blocked_climb, ["none", "none", "none", "none"], "4");
}

#[test]
fn redstone_wire_honors_always_and_never_conductor_overrides() {
    let east = relative(POS, Direction::East);
    let below_east = below(east);
    let lower_wire = state_id_vec(
        "minecraft:redstone_wire",
        wire_properties("none", "none", "none", "none", "0"),
    );

    let mut soul_sand = block_use_world_vec(
        "minecraft:redstone_wire",
        wire_properties("side", "side", "side", "side", "0"),
        VANILLA_GAME_TYPE_SURVIVAL_ID as i8,
    );
    set_block_at(&mut soul_sand, east, state_id("minecraft:soul_sand", []));
    set_block_at(&mut soul_sand, below_east, lower_wire);
    assert_eq!(
        use_main_hand(&mut soul_sand, 71),
        success(LocalInteractionSuccess::client_item_interaction())
    );
    assert_wire_sides(&soul_sand, ["none", "none", "none", "none"], "0");

    let mut glass = block_use_world_vec(
        "minecraft:redstone_wire",
        wire_properties("side", "side", "side", "side", "0"),
        VANILLA_GAME_TYPE_SURVIVAL_ID as i8,
    );
    set_block_at(&mut glass, east, state_id("minecraft:glass", []));
    set_block_at(&mut glass, below_east, lower_wire);
    assert_eq!(
        use_main_hand(&mut glass, 72),
        success(LocalInteractionSuccess::client_item_interaction())
    );
    assert_wire_sides(&glass, ["none", "side", "none", "side"], "0");
}
