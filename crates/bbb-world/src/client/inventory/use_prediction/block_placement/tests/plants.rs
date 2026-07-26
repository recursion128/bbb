use super::*;

#[test]
fn farmland_placement_uses_above_solidity_and_maintaining_tag() {
    let clicked = BlockPos { x: 4, y: 8, z: 4 };
    let target = relative(clicked, ProtocolDirection::Up);
    let above = relative(target, ProtocolDirection::Up);

    let mut open = placement_world();
    set_block(&mut open, clicked, state_id("minecraft:stone", []));
    set_held(&mut open, VANILLA_ITEM_FARMLAND_ID, 1);
    assert!(matches!(
        open.predict_local_block_use_with_hit(
            clicked,
            ProtocolDirection::Up,
            [0.5, 1.0, 0.5],
            false,
            InteractionHand::MainHand,
            false,
            58,
        ),
        LocalTargetUsePrediction::Result(LocalInteractionResult::Success(_))
    ));
    assert_eq!(
        open.probe_block(target).unwrap().block_state_id,
        state_id("minecraft:farmland", [("moisture", "0")])
    );

    let mut blocked = placement_world();
    set_block_tag(&mut blocked, "minecraft:maintains_farmland", &[]);
    set_block(&mut blocked, clicked, state_id("minecraft:stone", []));
    set_block(&mut blocked, above, state_id("minecraft:stone", []));
    set_held(&mut blocked, VANILLA_ITEM_FARMLAND_ID, 1);
    assert!(matches!(
        blocked.predict_local_block_use_with_hit(
            clicked,
            ProtocolDirection::Up,
            [0.5, 1.0, 0.5],
            false,
            InteractionHand::MainHand,
            false,
            59,
        ),
        LocalTargetUsePrediction::Result(LocalInteractionResult::Success(_))
    ));
    assert_eq!(
        blocked.probe_block(target).unwrap().block_state_id,
        state_id("minecraft:dirt", [])
    );

    let mut maintained = placement_world();
    set_block_tag(
        &mut maintained,
        "minecraft:maintains_farmland",
        &[VANILLA_BLOCK_STONE_ID],
    );
    set_block(&mut maintained, clicked, state_id("minecraft:stone", []));
    set_block(&mut maintained, above, state_id("minecraft:stone", []));
    set_held(&mut maintained, VANILLA_ITEM_FARMLAND_ID, 1);
    assert!(matches!(
        maintained.predict_local_block_use_with_hit(
            clicked,
            ProtocolDirection::Up,
            [0.5, 1.0, 0.5],
            false,
            InteractionHand::MainHand,
            false,
            60,
        ),
        LocalTargetUsePrediction::Result(LocalInteractionResult::Success(_))
    ));
    assert_eq!(
        maintained.probe_block(target).unwrap().block_state_id,
        state_id("minecraft:farmland", [("moisture", "0")])
    );
}

#[test]
fn dirt_path_placement_falls_back_to_dirt_below_solid_non_gate_blocks() {
    let clicked = BlockPos { x: 4, y: 8, z: 4 };
    let target = relative(clicked, ProtocolDirection::Up);
    let above = relative(target, ProtocolDirection::Up);

    let mut open = placement_world();
    set_block(&mut open, clicked, state_id("minecraft:stone", []));
    set_held(&mut open, VANILLA_ITEM_DIRT_PATH_ID, 1);
    assert!(matches!(
        use_block_at(&mut open, clicked, false, 104),
        LocalTargetUsePrediction::Result(LocalInteractionResult::Success(_))
    ));
    assert_eq!(
        open.probe_block(target).unwrap().block_state_id,
        state_id("minecraft:dirt_path", [])
    );

    let mut blocked = placement_world();
    set_block(&mut blocked, clicked, state_id("minecraft:stone", []));
    set_block(&mut blocked, above, state_id("minecraft:stone", []));
    set_held(&mut blocked, VANILLA_ITEM_DIRT_PATH_ID, 1);
    assert!(matches!(
        use_block_at(&mut blocked, clicked, false, 105),
        LocalTargetUsePrediction::Result(LocalInteractionResult::Success(_))
    ));
    assert_eq!(
        blocked.probe_block(target).unwrap().block_state_id,
        state_id("minecraft:dirt", [])
    );
    assert_eq!(held(&blocked).count, 0);
    blocked.apply_block_changed_ack(bbb_protocol::packets::BlockChangedAck { sequence: 105 });
    assert_eq!(
        blocked.probe_block(target).unwrap().block_state_id,
        state_id("minecraft:air", [])
    );

    let mut fence_gate = placement_world();
    set_block(&mut fence_gate, clicked, state_id("minecraft:stone", []));
    set_block(
        &mut fence_gate,
        above,
        state_id(
            "minecraft:oak_fence_gate",
            [
                ("facing", "north"),
                ("in_wall", "false"),
                ("open", "false"),
                ("powered", "false"),
            ],
        ),
    );
    set_held(&mut fence_gate, VANILLA_ITEM_DIRT_PATH_ID, 1);
    assert!(matches!(
        use_block_at(&mut fence_gate, clicked, false, 106),
        LocalTargetUsePrediction::Result(LocalInteractionResult::Success(_))
    ));
    assert_eq!(
        fence_gate.probe_block(target).unwrap().block_state_id,
        state_id("minecraft:dirt_path", [])
    );
    assert_eq!(held(&fence_gate).count, 0);
}

#[test]
fn leaves_placement_computes_decay_distance_and_waterlogged_persistence() {
    let clicked = BlockPos { x: 4, y: 8, z: 4 };
    let target = relative(clicked, ProtocolDirection::Up);
    let mut neighbouring_leaf = placement_world();
    set_block_tag(
        &mut neighbouring_leaf,
        "minecraft:prevents_nearby_leaf_decay",
        &[VANILLA_BLOCK_OAK_LOG_ID],
    );
    set_block(
        &mut neighbouring_leaf,
        clicked,
        state_id("minecraft:stone", []),
    );
    set_block(
        &mut neighbouring_leaf,
        relative(target, ProtocolDirection::North),
        state_id(
            "minecraft:oak_leaves",
            [
                ("distance", "3"),
                ("persistent", "false"),
                ("waterlogged", "false"),
            ],
        ),
    );
    set_held(&mut neighbouring_leaf, VANILLA_ITEM_OAK_LEAVES_ID, 1);
    assert!(matches!(
        neighbouring_leaf.predict_local_block_use_with_hit(
            clicked,
            ProtocolDirection::Up,
            [0.5, 1.0, 0.5],
            false,
            InteractionHand::MainHand,
            false,
            56,
        ),
        LocalTargetUsePrediction::Result(LocalInteractionResult::Success(_))
    ));
    assert_eq!(
        neighbouring_leaf
            .probe_block(target)
            .unwrap()
            .block_state_id,
        state_id(
            "minecraft:oak_leaves",
            [
                ("distance", "4"),
                ("persistent", "true"),
                ("waterlogged", "false"),
            ],
        )
    );

    let water_target = BlockPos { x: 6, y: 8, z: 6 };
    let mut waterlogged = placement_world();
    set_block_tag(
        &mut waterlogged,
        "minecraft:prevents_nearby_leaf_decay",
        &[VANILLA_BLOCK_OAK_LOG_ID],
    );
    set_block(
        &mut waterlogged,
        water_target,
        state_id("minecraft:water", [("level", "1")]),
    );
    set_block(
        &mut waterlogged,
        relative(water_target, ProtocolDirection::Down),
        state_id("minecraft:oak_log", [("axis", "y")]),
    );
    set_held(&mut waterlogged, VANILLA_ITEM_OAK_LEAVES_ID, 1);
    assert!(matches!(
        waterlogged.predict_local_block_use_with_hit(
            water_target,
            ProtocolDirection::Up,
            [0.5, 0.5, 0.5],
            false,
            InteractionHand::MainHand,
            false,
            57,
        ),
        LocalTargetUsePrediction::Result(LocalInteractionResult::Success(_))
    ));
    assert_eq!(
        waterlogged
            .probe_block(water_target)
            .unwrap()
            .block_state_id,
        state_id(
            "minecraft:oak_leaves",
            [
                ("distance", "1"),
                ("persistent", "true"),
                ("waterlogged", "true"),
            ],
        )
    );
}

#[test]
fn coral_fan_item_selects_floor_or_wall_form_with_full_water_state() {
    let mut floor = placement_world();
    set_held(&mut floor, VANILLA_ITEM_TUBE_CORAL_FAN_ID, 1);
    assert!(matches!(
        use_block(&mut floor, 47),
        LocalTargetUsePrediction::Result(LocalInteractionResult::Success(_))
    ));
    assert_eq!(
        floor.probe_block(TARGET).unwrap().block_state_id,
        state_id("minecraft:tube_coral_fan", [("waterlogged", "false")])
    );

    let wall_target = relative(CLICKED, ProtocolDirection::East);
    let mut wall = placement_world();
    set_block(
        &mut wall,
        wall_target,
        state_id("minecraft:water", [("level", "0")]),
    );
    set_held(&mut wall, VANILLA_ITEM_TUBE_CORAL_FAN_ID, 1);
    assert!(matches!(
        wall.predict_local_block_use_with_hit(
            CLICKED,
            ProtocolDirection::East,
            [1.0, 0.5, 0.5],
            false,
            InteractionHand::MainHand,
            false,
            48,
        ),
        LocalTargetUsePrediction::Result(LocalInteractionResult::Success(_))
    ));
    assert_eq!(
        wall.probe_block(wall_target).unwrap().block_state_id,
        state_id(
            "minecraft:tube_coral_wall_fan",
            [("facing", "east"), ("waterlogged", "true")],
        )
    );

    let mut flowing = placement_world();
    set_block(
        &mut flowing,
        wall_target,
        state_id("minecraft:water", [("level", "1")]),
    );
    set_held(&mut flowing, VANILLA_ITEM_TUBE_CORAL_FAN_ID, 1);
    assert!(matches!(
        flowing.predict_local_block_use_with_hit(
            CLICKED,
            ProtocolDirection::East,
            [1.0, 0.5, 0.5],
            false,
            InteractionHand::MainHand,
            false,
            49,
        ),
        LocalTargetUsePrediction::Result(LocalInteractionResult::Success(_))
    ));
    assert_eq!(
        flowing.probe_block(wall_target).unwrap().block_state_id,
        state_id(
            "minecraft:tube_coral_wall_fan",
            [("facing", "east"), ("waterlogged", "false")],
        )
    );
}

#[test]
fn coral_fan_item_fails_without_a_floor_or_wall_support() {
    let target = BlockPos { x: 4, y: 8, z: 4 };
    let mut world = placement_world();
    set_held(&mut world, VANILLA_ITEM_TUBE_CORAL_FAN_ID, 1);
    assert_eq!(
        world.predict_local_block_use_with_hit(
            target,
            ProtocolDirection::Up,
            [0.5, 0.5, 0.5],
            false,
            InteractionHand::MainHand,
            false,
            50,
        ),
        LocalTargetUsePrediction::Result(LocalInteractionResult::Fail)
    );
    assert_eq!(held(&world).count, 1);
    assert!(world.local_block_predictions().is_empty());
}

#[test]
fn hanging_roots_placement_reads_source_water_and_ceiling_support() {
    let support = BlockPos { x: 4, y: 10, z: 4 };
    let target = relative(support, ProtocolDirection::Down);
    let place = |world: &mut WorldStore, sequence| {
        world.predict_local_block_use_with_hit(
            support,
            ProtocolDirection::Down,
            [0.5, 0.0, 0.5],
            false,
            InteractionHand::MainHand,
            false,
            sequence,
        )
    };

    let mut source = placement_world();
    set_block(&mut source, support, state_id("minecraft:stone", []));
    set_block(
        &mut source,
        target,
        state_id("minecraft:water", [("level", "0")]),
    );
    set_held(&mut source, VANILLA_ITEM_HANGING_ROOTS_ID, 1);
    assert!(matches!(
        place(&mut source, 64),
        LocalTargetUsePrediction::Result(LocalInteractionResult::Success(_))
    ));
    assert_eq!(
        source.probe_block(target).unwrap().block_state_id,
        state_id("minecraft:hanging_roots", [("waterlogged", "true")])
    );

    let mut flowing = placement_world();
    set_block(&mut flowing, support, state_id("minecraft:stone", []));
    set_block(
        &mut flowing,
        target,
        state_id("minecraft:water", [("level", "3")]),
    );
    set_held(&mut flowing, VANILLA_ITEM_HANGING_ROOTS_ID, 1);
    assert!(matches!(
        place(&mut flowing, 65),
        LocalTargetUsePrediction::Result(LocalInteractionResult::Success(_))
    ));
    assert_eq!(
        flowing.probe_block(target).unwrap().block_state_id,
        state_id("minecraft:hanging_roots", [("waterlogged", "false")])
    );

    let mut unsupported = placement_world();
    set_block(
        &mut unsupported,
        support,
        state_id(
            "minecraft:oak_slab",
            [("type", "top"), ("waterlogged", "false")],
        ),
    );
    set_held(&mut unsupported, VANILLA_ITEM_HANGING_ROOTS_ID, 1);
    assert_eq!(
        place(&mut unsupported, 66),
        LocalTargetUsePrediction::Result(LocalInteractionResult::Fail)
    );
    assert_eq!(
        unsupported.probe_block(target).unwrap().block_state_id,
        state_id("minecraft:air", [])
    );
    assert_eq!(held(&unsupported).count, 1);
    assert!(unsupported.local_block_predictions().is_empty());
}

#[test]
fn seagrass_placement_requires_full_water_and_an_allowed_sturdy_floor() {
    let clicked = BlockPos { x: 4, y: 8, z: 4 };
    let target = relative(clicked, ProtocolDirection::Up);

    let mut full_falling = placement_world();
    set_block_tag(&mut full_falling, "minecraft:cannot_support_seagrass", &[]);
    set_block(&mut full_falling, clicked, state_id("minecraft:stone", []));
    set_block(
        &mut full_falling,
        target,
        state_id("minecraft:water", [("level", "8")]),
    );
    set_held(&mut full_falling, VANILLA_ITEM_SEAGRASS_ID, 1);
    assert!(matches!(
        use_block_at(&mut full_falling, clicked, false, 75),
        LocalTargetUsePrediction::Result(LocalInteractionResult::Success(_))
    ));
    assert_eq!(
        full_falling.probe_block(target).unwrap().block_state_id,
        state_id("minecraft:seagrass", [])
    );

    let mut partial = placement_world();
    set_block_tag(&mut partial, "minecraft:cannot_support_seagrass", &[]);
    set_block(&mut partial, clicked, state_id("minecraft:stone", []));
    set_block(
        &mut partial,
        target,
        state_id("minecraft:water", [("level", "3")]),
    );
    set_held(&mut partial, VANILLA_ITEM_SEAGRASS_ID, 1);
    assert_eq!(
        use_block_at(&mut partial, clicked, false, 76),
        LocalTargetUsePrediction::Result(LocalInteractionResult::Fail)
    );
    assert_eq!(
        partial.probe_block(target).unwrap().block_state_id,
        state_id("minecraft:water", [("level", "3")])
    );
    assert_eq!(held(&partial).count, 1);

    let mut forbidden_floor = placement_world();
    set_block_tag(
        &mut forbidden_floor,
        "minecraft:cannot_support_seagrass",
        &[VANILLA_BLOCK_STONE_ID],
    );
    set_block(
        &mut forbidden_floor,
        clicked,
        state_id("minecraft:stone", []),
    );
    set_block(
        &mut forbidden_floor,
        target,
        state_id("minecraft:water", [("level", "0")]),
    );
    set_held(&mut forbidden_floor, VANILLA_ITEM_SEAGRASS_ID, 1);
    assert_eq!(
        use_block_at(&mut forbidden_floor, clicked, false, 77),
        LocalTargetUsePrediction::Result(LocalInteractionResult::Fail)
    );
    assert_eq!(
        forbidden_floor.probe_block(target).unwrap().block_state_id,
        state_id("minecraft:water", [("level", "0")])
    );
    assert_eq!(held(&forbidden_floor).count, 1);
}

#[test]
fn growing_plant_heads_use_the_shared_level_random_for_age() {
    let upward_clicked = BlockPos { x: 4, y: 8, z: 4 };
    let downward_clicked = BlockPos { x: 4, y: 10, z: 4 };

    for (sequence, item_id, block_name, growth_direction) in [
        (
            89,
            VANILLA_ITEM_KELP_ID,
            "minecraft:kelp",
            ProtocolDirection::Up,
        ),
        (
            90,
            VANILLA_ITEM_TWISTING_VINES_ID,
            "minecraft:twisting_vines",
            ProtocolDirection::Up,
        ),
        (
            91,
            VANILLA_ITEM_WEEPING_VINES_ID,
            "minecraft:weeping_vines",
            ProtocolDirection::Down,
        ),
        (
            92,
            VANILLA_ITEM_GLOW_BERRIES_ID,
            "minecraft:cave_vines",
            ProtocolDirection::Down,
        ),
    ] {
        let (clicked, cursor) = if growth_direction == ProtocolDirection::Up {
            (upward_clicked, [0.5, 1.0, 0.5])
        } else {
            (downward_clicked, [0.5, 0.0, 0.5])
        };
        let target = relative(clicked, growth_direction);
        let mut world = placement_world();
        set_block(&mut world, clicked, state_id("minecraft:stone", []));
        if item_id == VANILLA_ITEM_KELP_ID {
            set_block_tag(&mut world, "minecraft:cannot_support_kelp", &[]);
            set_block(
                &mut world,
                target,
                state_id("minecraft:water", [("level", "0")]),
            );
        }
        set_held(&mut world, item_id, 1);

        let mut random = crate::LevelEventSoundRandomState::with_seed(i64::from(sequence));
        let mut control = random.clone();
        let age = control.next_int_bound(25).to_string();
        assert!(matches!(
            world.predict_local_block_use_with_hit_and_random(
                clicked,
                growth_direction,
                cursor,
                false,
                InteractionHand::MainHand,
                false,
                sequence,
                &mut random,
            ),
            LocalTargetUsePrediction::Result(LocalInteractionResult::Success(_))
        ));
        let expected_state = if block_name == "minecraft:cave_vines" {
            state_id(block_name, [("age", age.as_str()), ("berries", "false")])
        } else {
            state_id(block_name, [("age", age.as_str())])
        };
        assert_eq!(
            world.probe_block(target).unwrap().block_state_id,
            expected_state
        );
        assert_eq!(held(&world).count, 0);
        assert_eq!(
            random.next_int_bound(997),
            control.next_int_bound(997),
            "{block_name} must consume exactly one age draw"
        );
    }
}

#[test]
fn growing_plant_body_placement_reuses_the_family_without_consuming_random() {
    let upward_clicked = BlockPos { x: 4, y: 8, z: 4 };
    let downward_clicked = BlockPos { x: 4, y: 10, z: 4 };

    for (sequence, item_id, head_name, body_name, growth_direction) in [
        (
            93,
            VANILLA_ITEM_KELP_ID,
            "minecraft:kelp",
            "minecraft:kelp_plant",
            ProtocolDirection::Up,
        ),
        (
            94,
            VANILLA_ITEM_TWISTING_VINES_ID,
            "minecraft:twisting_vines",
            "minecraft:twisting_vines_plant",
            ProtocolDirection::Up,
        ),
        (
            95,
            VANILLA_ITEM_WEEPING_VINES_ID,
            "minecraft:weeping_vines",
            "minecraft:weeping_vines_plant",
            ProtocolDirection::Down,
        ),
        (
            96,
            VANILLA_ITEM_GLOW_BERRIES_ID,
            "minecraft:cave_vines",
            "minecraft:cave_vines_plant",
            ProtocolDirection::Down,
        ),
    ] {
        let (clicked, cursor) = if growth_direction == ProtocolDirection::Up {
            (upward_clicked, [0.5, 1.0, 0.5])
        } else {
            (downward_clicked, [0.5, 0.0, 0.5])
        };
        let target = relative(clicked, growth_direction);
        let growth_neighbour = relative(target, growth_direction);
        let mut world = placement_world();
        set_block(&mut world, clicked, state_id("minecraft:stone", []));
        if item_id == VANILLA_ITEM_KELP_ID {
            set_block_tag(&mut world, "minecraft:cannot_support_kelp", &[]);
            set_block(
                &mut world,
                target,
                state_id("minecraft:water", [("level", "0")]),
            );
        }
        let neighbour_state = if head_name == "minecraft:cave_vines" {
            state_id(head_name, [("age", "6"), ("berries", "false")])
        } else {
            state_id(head_name, [("age", "6")])
        };
        set_block(&mut world, growth_neighbour, neighbour_state);
        set_held(&mut world, item_id, 1);

        let mut random = crate::LevelEventSoundRandomState::with_seed(i64::from(sequence));
        let mut control = random.clone();
        assert!(matches!(
            world.predict_local_block_use_with_hit_and_random(
                clicked,
                growth_direction,
                cursor,
                false,
                InteractionHand::MainHand,
                false,
                sequence,
                &mut random,
            ),
            LocalTargetUsePrediction::Result(LocalInteractionResult::Success(_))
        ));
        let expected_state = if body_name == "minecraft:cave_vines_plant" {
            state_id(body_name, [("berries", "false")])
        } else {
            state_id(body_name, [])
        };
        assert_eq!(
            world.probe_block(target).unwrap().block_state_id,
            expected_state
        );
        assert_eq!(held(&world).count, 0);
        assert_eq!(
            random.next_int_bound(997),
            control.next_int_bound(997),
            "{body_name} must not consume an age draw"
        );
    }
}

#[test]
fn coral_block_placement_only_advances_level_random_when_dry() {
    let clicked = BlockPos { x: 4, y: 8, z: 4 };
    let target = relative(clicked, ProtocolDirection::Up);

    let mut wet = placement_world();
    set_block(&mut wet, clicked, state_id("minecraft:stone", []));
    set_block(
        &mut wet,
        relative(target, ProtocolDirection::North),
        state_id("minecraft:water", [("level", "1")]),
    );
    set_held(&mut wet, VANILLA_ITEM_TUBE_CORAL_BLOCK_ID, 1);
    assert!(matches!(
        use_block_at(&mut wet, clicked, false, 97),
        LocalTargetUsePrediction::Result(LocalInteractionResult::Success(_))
    ));
    assert_eq!(
        wet.probe_block(target).unwrap().block_state_id,
        state_id("minecraft:tube_coral_block", [])
    );

    for (sequence, item_id, block_name) in [
        (
            98,
            VANILLA_ITEM_TUBE_CORAL_BLOCK_ID,
            "minecraft:tube_coral_block",
        ),
        (
            99,
            VANILLA_ITEM_BRAIN_CORAL_BLOCK_ID,
            "minecraft:brain_coral_block",
        ),
        (
            100,
            VANILLA_ITEM_BUBBLE_CORAL_BLOCK_ID,
            "minecraft:bubble_coral_block",
        ),
        (
            101,
            VANILLA_ITEM_FIRE_CORAL_BLOCK_ID,
            "minecraft:fire_coral_block",
        ),
        (
            102,
            VANILLA_ITEM_HORN_CORAL_BLOCK_ID,
            "minecraft:horn_coral_block",
        ),
    ] {
        let mut world = placement_world();
        set_block(&mut world, clicked, state_id("minecraft:stone", []));
        set_held(&mut world, item_id, 1);
        let mut random = crate::LevelEventSoundRandomState::with_seed(i64::from(sequence));
        let mut control = random.clone();
        let _scheduled_tick_delay = 60 + control.next_int_bound(40);

        assert!(matches!(
            world.predict_local_block_use_with_hit_and_random(
                clicked,
                ProtocolDirection::Up,
                [0.5, 1.0, 0.5],
                false,
                InteractionHand::MainHand,
                false,
                sequence,
                &mut random,
            ),
            LocalTargetUsePrediction::Result(LocalInteractionResult::Success(_))
        ));
        assert_eq!(
            world.probe_block(target).unwrap().block_state_id,
            state_id(block_name, [])
        );
        assert_eq!(held(&world).count, 0);
        assert_eq!(
            random.next_int_bound(997),
            control.next_int_bound(997),
            "{block_name} must consume exactly one scheduled-tick draw"
        );
    }

    let mut missing_random = placement_world();
    set_block(
        &mut missing_random,
        clicked,
        state_id("minecraft:stone", []),
    );
    set_held(&mut missing_random, VANILLA_ITEM_TUBE_CORAL_BLOCK_ID, 1);
    assert_eq!(
        use_block_at(&mut missing_random, clicked, false, 103),
        LocalTargetUsePrediction::UnsupportedItemUseOn(VanillaItemUseOnOverride::BlockItem)
    );
    assert_eq!(
        missing_random.probe_block(target).unwrap().block_state_id,
        state_id("minecraft:air", [])
    );
    assert_eq!(held(&missing_random).count, 1);
}

#[test]
fn pitcher_crop_placement_requires_crop_support_and_raw_brightness_eight() {
    let clicked = BlockPos { x: 4, y: 8, z: 4 };
    let target = relative(clicked, ProtocolDirection::Up);
    let set_raw_light = |world: &mut WorldStore, sky: u8, block: u8| {
        let light_section_index = target.y.div_euclid(16) - (world.dimension().min_section_y() - 1);
        let light_section_mask =
            1_i64 << u32::try_from(light_section_index).expect("light section");
        let packed_nibbles = |value: u8| value | (value << 4);
        assert!(world
            .apply_light_update(LightUpdate {
                chunk_x: 0,
                chunk_z: 0,
                light_data: LightUpdateData {
                    sky_y_mask: vec![light_section_mask],
                    block_y_mask: vec![light_section_mask],
                    empty_sky_y_mask: Vec::new(),
                    empty_block_y_mask: Vec::new(),
                    sky_updates: vec![vec![packed_nibbles(sky); 2048]],
                    block_updates: vec![vec![packed_nibbles(block); 2048]],
                },
            })
            .unwrap());
    };

    let mut supported = placement_world();
    set_block_tag(
        &mut supported,
        "minecraft:supports_crops",
        &[VANILLA_BLOCK_STONE_ID],
    );
    set_block(&mut supported, clicked, state_id("minecraft:stone", []));
    set_raw_light(&mut supported, 0, 8);
    set_held(&mut supported, VANILLA_ITEM_PITCHER_POD_ID, 1);
    assert!(matches!(
        use_block_at(&mut supported, clicked, false, 78),
        LocalTargetUsePrediction::Result(LocalInteractionResult::Success(_))
    ));
    assert_eq!(
        supported.probe_block(target).unwrap().block_state_id,
        state_id("minecraft:pitcher_crop", [("age", "0"), ("half", "lower")])
    );
    assert_eq!(held(&supported).count, 0);

    let mut dark = placement_world();
    set_block_tag(
        &mut dark,
        "minecraft:supports_crops",
        &[VANILLA_BLOCK_STONE_ID],
    );
    set_block(&mut dark, clicked, state_id("minecraft:stone", []));
    set_raw_light(&mut dark, 7, 6);
    set_held(&mut dark, VANILLA_ITEM_PITCHER_POD_ID, 1);
    assert_eq!(
        use_block_at(&mut dark, clicked, false, 79),
        LocalTargetUsePrediction::Result(LocalInteractionResult::Fail)
    );
    assert_eq!(
        dark.probe_block(target).unwrap().block_state_id,
        state_id("minecraft:air", [])
    );
    assert_eq!(held(&dark).count, 1);

    let mut unsupported = placement_world();
    set_block_tag(&mut unsupported, "minecraft:supports_crops", &[]);
    set_block(&mut unsupported, clicked, state_id("minecraft:stone", []));
    set_held(&mut unsupported, VANILLA_ITEM_PITCHER_POD_ID, 1);
    assert_eq!(
        use_block_at(&mut unsupported, clicked, false, 80),
        LocalTargetUsePrediction::Result(LocalInteractionResult::Fail)
    );
    assert_eq!(
        unsupported.probe_block(target).unwrap().block_state_id,
        state_id("minecraft:air", [])
    );
    assert_eq!(held(&unsupported).count, 1);
}

#[test]
fn snowy_block_placement_reads_the_live_snow_tag_for_all_three_items() {
    let clicked = BlockPos { x: 4, y: 8, z: 4 };
    let target = relative(clicked, ProtocolDirection::Up);
    let above = relative(target, ProtocolDirection::Up);

    for (sequence, item_id, block_name) in [
        (84, VANILLA_ITEM_GRASS_BLOCK_ID, "minecraft:grass_block"),
        (85, VANILLA_ITEM_PODZOL_ID, "minecraft:podzol"),
        (86, VANILLA_ITEM_MYCELIUM_ID, "minecraft:mycelium"),
    ] {
        let mut world = placement_world();
        set_block_tag(&mut world, "minecraft:snow", &[VANILLA_BLOCK_SNOW_ID]);
        set_block(&mut world, clicked, state_id("minecraft:stone", []));
        set_block(
            &mut world,
            above,
            state_id("minecraft:snow", [("layers", "1")]),
        );
        set_held(&mut world, item_id, 1);
        assert!(matches!(
            use_block_at(&mut world, clicked, false, sequence),
            LocalTargetUsePrediction::Result(LocalInteractionResult::Success(_))
        ));
        assert_eq!(
            world.probe_block(target).unwrap().block_state_id,
            state_id(block_name, [("snowy", "true")])
        );
        assert_eq!(held(&world).count, 0);
    }

    let mut remapped = placement_world();
    set_block_tag(&mut remapped, "minecraft:snow", &[VANILLA_BLOCK_STONE_ID]);
    set_block(&mut remapped, clicked, state_id("minecraft:stone", []));
    set_block(&mut remapped, above, state_id("minecraft:stone", []));
    set_held(&mut remapped, VANILLA_ITEM_GRASS_BLOCK_ID, 1);
    assert!(matches!(
        use_block_at(&mut remapped, clicked, false, 87),
        LocalTargetUsePrediction::Result(LocalInteractionResult::Success(_))
    ));
    assert_eq!(
        remapped.probe_block(target).unwrap().block_state_id,
        state_id("minecraft:grass_block", [("snowy", "true")])
    );

    let mut excluded = placement_world();
    set_block_tag(&mut excluded, "minecraft:snow", &[]);
    set_block(&mut excluded, clicked, state_id("minecraft:stone", []));
    set_block(
        &mut excluded,
        above,
        state_id("minecraft:snow", [("layers", "1")]),
    );
    set_held(&mut excluded, VANILLA_ITEM_PODZOL_ID, 1);
    assert!(matches!(
        use_block_at(&mut excluded, clicked, false, 88),
        LocalTargetUsePrediction::Result(LocalInteractionResult::Success(_))
    ));
    assert_eq!(
        excluded.probe_block(target).unwrap().block_state_id,
        state_id("minecraft:podzol", [("snowy", "false")])
    );
}

#[test]
fn bamboo_placement_selects_sapling_or_stalk_from_vertical_neighbours() {
    let support_entries = [
        VANILLA_BLOCK_BAMBOO_ID,
        VANILLA_BLOCK_BAMBOO_SAPLING_ID,
        VANILLA_BLOCK_DIRT_ID,
    ];

    let mut sapling = placement_world();
    set_block_tag(&mut sapling, "minecraft:supports_bamboo", &support_entries);
    set_block(&mut sapling, CLICKED, state_id("minecraft:dirt", []));
    set_held(&mut sapling, VANILLA_ITEM_BAMBOO_ID, 1);
    assert!(matches!(
        use_block(&mut sapling, 41),
        LocalTargetUsePrediction::Result(LocalInteractionResult::Success(_))
    ));
    assert_eq!(
        sapling.probe_block(TARGET).unwrap().block_state_id,
        state_id("minecraft:bamboo_sapling", [])
    );

    let mut above_sapling = placement_world();
    set_block_tag(
        &mut above_sapling,
        "minecraft:supports_bamboo",
        &support_entries,
    );
    set_block(
        &mut above_sapling,
        CLICKED,
        state_id("minecraft:bamboo_sapling", []),
    );
    set_held(&mut above_sapling, VANILLA_ITEM_BAMBOO_ID, 1);
    assert!(matches!(
        use_block(&mut above_sapling, 42),
        LocalTargetUsePrediction::Result(LocalInteractionResult::Success(_))
    ));
    assert_eq!(
        above_sapling.probe_block(TARGET).unwrap().block_state_id,
        state_id(
            "minecraft:bamboo",
            [("age", "0"), ("leaves", "none"), ("stage", "0")],
        )
    );

    let mut above_stalk = placement_world();
    set_block_tag(
        &mut above_stalk,
        "minecraft:supports_bamboo",
        &support_entries,
    );
    set_block(&mut above_stalk, CLICKED, state_id("minecraft:dirt", []));
    set_block(
        &mut above_stalk,
        relative(TARGET, ProtocolDirection::Up),
        state_id(
            "minecraft:bamboo",
            [("age", "1"), ("leaves", "large"), ("stage", "1")],
        ),
    );
    set_held(&mut above_stalk, VANILLA_ITEM_BAMBOO_ID, 1);
    assert!(matches!(
        use_block(&mut above_stalk, 43),
        LocalTargetUsePrediction::Result(LocalInteractionResult::Success(_))
    ));
    assert_eq!(
        above_stalk.probe_block(TARGET).unwrap().block_state_id,
        state_id(
            "minecraft:bamboo",
            [("age", "1"), ("leaves", "none"), ("stage", "0")],
        )
    );

    let mut below_stalk = placement_world();
    set_block_tag(
        &mut below_stalk,
        "minecraft:supports_bamboo",
        &support_entries,
    );
    set_block(
        &mut below_stalk,
        CLICKED,
        state_id(
            "minecraft:bamboo",
            [("age", "0"), ("leaves", "small"), ("stage", "1")],
        ),
    );
    set_held(&mut below_stalk, VANILLA_ITEM_BAMBOO_ID, 1);
    assert!(matches!(
        use_block(&mut below_stalk, 44),
        LocalTargetUsePrediction::Result(LocalInteractionResult::Success(_))
    ));
    assert_eq!(
        below_stalk.probe_block(TARGET).unwrap().block_state_id,
        state_id(
            "minecraft:bamboo",
            [("age", "0"), ("leaves", "none"), ("stage", "0")],
        )
    );
}

#[test]
fn bamboo_placement_rejects_nonempty_fluid_and_unsupported_ground() {
    let support_entries = [
        VANILLA_BLOCK_BAMBOO_ID,
        VANILLA_BLOCK_BAMBOO_SAPLING_ID,
        VANILLA_BLOCK_DIRT_ID,
    ];

    let mut fluid = placement_world();
    set_block_tag(&mut fluid, "minecraft:supports_bamboo", &support_entries);
    set_block(
        &mut fluid,
        CLICKED,
        state_id("minecraft:water", [("level", "0")]),
    );
    set_held(&mut fluid, VANILLA_ITEM_BAMBOO_ID, 1);
    assert_eq!(
        use_block(&mut fluid, 45),
        LocalTargetUsePrediction::Result(LocalInteractionResult::Fail)
    );
    assert_eq!(held(&fluid).count, 1);
    assert!(fluid.local_block_predictions().is_empty());

    let mut unsupported_ground = placement_world();
    set_block_tag(
        &mut unsupported_ground,
        "minecraft:supports_bamboo",
        &support_entries,
    );
    set_block(
        &mut unsupported_ground,
        CLICKED,
        state_id("minecraft:stone", []),
    );
    set_held(&mut unsupported_ground, VANILLA_ITEM_BAMBOO_ID, 1);
    assert_eq!(
        use_block(&mut unsupported_ground, 46),
        LocalTargetUsePrediction::Result(LocalInteractionResult::Fail)
    );
    assert_eq!(held(&unsupported_ground).count, 1);
    assert!(unsupported_ground.local_block_predictions().is_empty());
}
