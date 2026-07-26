use super::*;

#[test]
fn cardinal_connection_and_chest_placement_read_live_neighbours() {
    let clicked = BlockPos { x: 4, y: 8, z: 4 };
    let target = relative(clicked, ProtocolDirection::Up);
    let north = relative(target, ProtocolDirection::North);
    let east = relative(target, ProtocolDirection::East);
    let south = relative(target, ProtocolDirection::South);
    let west = relative(target, ProtocolDirection::West);

    let mut fence = placement_world();
    set_connection_tags(&mut fence);
    set_block(&mut fence, clicked, state_id("minecraft:stone", []));
    set_block(&mut fence, north, state_id("minecraft:stone", []));
    set_block(
        &mut fence,
        east,
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
    set_block(
        &mut fence,
        south,
        state_id("minecraft:carved_pumpkin", [("facing", "north")]),
    );
    set_block(
        &mut fence,
        west,
        state_id(
            "minecraft:nether_brick_fence",
            [
                ("east", "false"),
                ("north", "false"),
                ("south", "false"),
                ("waterlogged", "false"),
                ("west", "false"),
            ],
        ),
    );
    set_held(&mut fence, VANILLA_ITEM_OAK_FENCE_ID, 1);
    assert!(matches!(
        use_block_at(&mut fence, clicked, false, 20),
        LocalTargetUsePrediction::Result(LocalInteractionResult::Success(_))
    ));
    assert_eq!(
        fence.probe_block(target).unwrap().block_state_id,
        state_id(
            "minecraft:oak_fence",
            [
                ("east", "true"),
                ("north", "true"),
                ("south", "false"),
                ("waterlogged", "false"),
                ("west", "false"),
            ],
        )
    );

    let mut bars = placement_world();
    set_connection_tags(&mut bars);
    set_block(&mut bars, clicked, state_id("minecraft:stone", []));
    set_block(&mut bars, north, state_id("minecraft:stone", []));
    set_block(
        &mut bars,
        east,
        state_id(
            "minecraft:cobblestone_wall",
            [
                ("east", "none"),
                ("north", "none"),
                ("south", "none"),
                ("up", "true"),
                ("waterlogged", "false"),
                ("west", "none"),
            ],
        ),
    );
    set_block(
        &mut bars,
        south,
        state_id("minecraft:carved_pumpkin", [("facing", "north")]),
    );
    set_block(
        &mut bars,
        west,
        state_id(
            "minecraft:glass_pane",
            [
                ("east", "false"),
                ("north", "false"),
                ("south", "false"),
                ("waterlogged", "false"),
                ("west", "false"),
            ],
        ),
    );
    set_held(&mut bars, VANILLA_ITEM_IRON_BARS_ID, 1);
    assert!(matches!(
        use_block_at(&mut bars, clicked, false, 21),
        LocalTargetUsePrediction::Result(LocalInteractionResult::Success(_))
    ));
    assert_eq!(
        bars.probe_block(target).unwrap().block_state_id,
        state_id(
            "minecraft:iron_bars",
            [
                ("east", "true"),
                ("north", "true"),
                ("south", "false"),
                ("waterlogged", "false"),
                ("west", "true"),
            ],
        )
    );

    let mut chest = placement_world();
    set_connection_tags(&mut chest);
    set_block(&mut chest, clicked, state_id("minecraft:stone", []));
    set_block(
        &mut chest,
        east,
        state_id(
            "minecraft:chest",
            [
                ("facing", "north"),
                ("type", "single"),
                ("waterlogged", "false"),
            ],
        ),
    );
    set_held(&mut chest, VANILLA_ITEM_CHEST_ID, 1);
    assert!(matches!(
        use_block_at(&mut chest, clicked, false, 22),
        LocalTargetUsePrediction::Result(LocalInteractionResult::Success(_))
    ));
    assert_eq!(
        chest.probe_block(target).unwrap().block_state_id,
        state_id(
            "minecraft:chest",
            [
                ("facing", "north"),
                ("type", "left"),
                ("waterlogged", "false"),
            ],
        )
    );

    let mut sneaking_chest = placement_world();
    set_connection_tags(&mut sneaking_chest);
    set_block(
        &mut sneaking_chest,
        clicked,
        state_id("minecraft:stone", []),
    );
    set_block(
        &mut sneaking_chest,
        east,
        state_id(
            "minecraft:chest",
            [
                ("facing", "north"),
                ("type", "single"),
                ("waterlogged", "false"),
            ],
        ),
    );
    set_held(&mut sneaking_chest, VANILLA_ITEM_CHEST_ID, 1);
    assert!(matches!(
        use_block_at(&mut sneaking_chest, clicked, true, 23),
        LocalTargetUsePrediction::Result(LocalInteractionResult::Success(_))
    ));
    assert_eq!(
        sneaking_chest.probe_block(target).unwrap().block_state_id,
        state_id(
            "minecraft:chest",
            [
                ("facing", "north"),
                ("type", "single"),
                ("waterlogged", "false"),
            ],
        )
    );
}

#[test]
fn copper_chest_placement_uses_least_oxidized_and_matching_wax_state() {
    assert_copper_chest_placement(
        VANILLA_ITEM_OXIDIZED_COPPER_CHEST_ID,
        "minecraft:exposed_copper_chest",
        "minecraft:exposed_copper_chest",
        26,
    );
    assert_copper_chest_placement(
        VANILLA_ITEM_WAXED_OXIDIZED_COPPER_CHEST_ID,
        "minecraft:exposed_copper_chest",
        "minecraft:exposed_copper_chest",
        27,
    );
    assert_copper_chest_placement(
        VANILLA_ITEM_WAXED_WEATHERED_COPPER_CHEST_ID,
        "minecraft:waxed_copper_chest",
        "minecraft:waxed_copper_chest",
        28,
    );
    assert_copper_chest_placement(
        VANILLA_ITEM_WAXED_COPPER_CHEST_ID,
        "minecraft:oxidized_copper_chest",
        "minecraft:copper_chest",
        29,
    );
}

#[test]
fn chorus_plant_placement_connects_six_ways_and_applies_branch_survival() {
    let clicked = BlockPos { x: 4, y: 8, z: 4 };
    let target = relative(clicked, ProtocolDirection::Up);

    let mut supported_below = placement_world();
    set_block_tag(
        &mut supported_below,
        "minecraft:supports_chorus_plant",
        &[VANILLA_BLOCK_END_STONE_ID],
    );
    set_block(
        &mut supported_below,
        clicked,
        state_id("minecraft:end_stone", []),
    );
    set_held(&mut supported_below, VANILLA_ITEM_CHORUS_PLANT_ID, 1);
    assert!(matches!(
        use_block_at(&mut supported_below, clicked, false, 38),
        LocalTargetUsePrediction::Result(LocalInteractionResult::Success(_))
    ));
    assert_eq!(
        supported_below.probe_block(target).unwrap().block_state_id,
        state_id(
            "minecraft:chorus_plant",
            [
                ("down", "true"),
                ("east", "false"),
                ("north", "false"),
                ("south", "false"),
                ("up", "false"),
                ("west", "false"),
            ],
        )
    );

    let side_target = relative(clicked, ProtocolDirection::East);
    let mut supported_branch = placement_world();
    set_block_tag(
        &mut supported_branch,
        "minecraft:supports_chorus_plant",
        &[VANILLA_BLOCK_END_STONE_ID],
    );
    set_block(
        &mut supported_branch,
        clicked,
        state_id(
            "minecraft:chorus_plant",
            [
                ("down", "true"),
                ("east", "false"),
                ("north", "false"),
                ("south", "false"),
                ("up", "false"),
                ("west", "false"),
            ],
        ),
    );
    set_block(
        &mut supported_branch,
        relative(clicked, ProtocolDirection::Down),
        state_id("minecraft:end_stone", []),
    );
    set_block(
        &mut supported_branch,
        relative(side_target, ProtocolDirection::Up),
        state_id("minecraft:chorus_flower", [("age", "0")]),
    );
    set_block(
        &mut supported_branch,
        relative(side_target, ProtocolDirection::East),
        state_id("minecraft:chorus_flower", [("age", "1")]),
    );
    set_held(&mut supported_branch, VANILLA_ITEM_CHORUS_PLANT_ID, 1);
    assert!(matches!(
        supported_branch.predict_local_block_use_with_hit(
            clicked,
            ProtocolDirection::East,
            [1.0, 0.5, 0.5],
            false,
            InteractionHand::MainHand,
            false,
            39,
        ),
        LocalTargetUsePrediction::Result(LocalInteractionResult::Success(_))
    ));
    assert_eq!(
        supported_branch
            .probe_block(side_target)
            .unwrap()
            .block_state_id,
        state_id(
            "minecraft:chorus_plant",
            [
                ("down", "false"),
                ("east", "true"),
                ("north", "false"),
                ("south", "false"),
                ("up", "true"),
                ("west", "true"),
            ],
        )
    );

    let mut blocked_branch = placement_world();
    set_block_tag(
        &mut blocked_branch,
        "minecraft:supports_chorus_plant",
        &[VANILLA_BLOCK_END_STONE_ID],
    );
    set_block(
        &mut blocked_branch,
        clicked,
        state_id("minecraft:end_stone", []),
    );
    set_block(
        &mut blocked_branch,
        relative(target, ProtocolDirection::Up),
        state_id("minecraft:stone", []),
    );
    set_block(
        &mut blocked_branch,
        relative(target, ProtocolDirection::North),
        state_id(
            "minecraft:chorus_plant",
            [
                ("down", "false"),
                ("east", "false"),
                ("north", "false"),
                ("south", "false"),
                ("up", "false"),
                ("west", "false"),
            ],
        ),
    );
    set_held(&mut blocked_branch, VANILLA_ITEM_CHORUS_PLANT_ID, 1);
    assert_eq!(
        use_block_at(&mut blocked_branch, clicked, false, 40),
        LocalTargetUsePrediction::Result(LocalInteractionResult::Fail)
    );
    assert_eq!(
        blocked_branch.probe_block(target).unwrap().block_state_id,
        state_id("minecraft:air", [])
    );
    assert_eq!(held(&blocked_branch).count, 1);
    assert!(blocked_branch.local_block_predictions().is_empty());
}

#[test]
fn mossy_carpet_placement_projects_supported_sides_and_requires_a_non_air_base() {
    let clicked = BlockPos { x: 4, y: 8, z: 4 };
    let target = relative(clicked, ProtocolDirection::Up);
    let north = relative(target, ProtocolDirection::North);
    let east = relative(target, ProtocolDirection::East);
    let above = relative(target, ProtocolDirection::Up);

    let mut supported = placement_world();
    set_block(&mut supported, clicked, state_id("minecraft:stone", []));
    set_block(&mut supported, north, state_id("minecraft:stone", []));
    set_block(&mut supported, east, state_id("minecraft:stone", []));
    set_block(
        &mut supported,
        above,
        state_id(
            "minecraft:pale_moss_carpet",
            [
                ("bottom", "false"),
                ("east", "none"),
                ("north", "low"),
                ("south", "none"),
                ("west", "none"),
            ],
        ),
    );
    set_held(&mut supported, VANILLA_ITEM_PALE_MOSS_CARPET_ID, 1);
    assert!(matches!(
        use_block_at(&mut supported, clicked, false, 30),
        LocalTargetUsePrediction::Result(LocalInteractionResult::Success(_))
    ));
    assert_eq!(
        supported.probe_block(target).unwrap().block_state_id,
        state_id(
            "minecraft:pale_moss_carpet",
            [
                ("bottom", "true"),
                ("east", "low"),
                ("north", "tall"),
                ("south", "none"),
                ("west", "none"),
            ],
        )
    );
    assert_eq!(held(&supported).count, 0);

    let mut unsupported_base = placement_world();
    set_block(
        &mut unsupported_base,
        clicked,
        state_id("minecraft:short_grass", []),
    );
    set_held(&mut unsupported_base, VANILLA_ITEM_PALE_MOSS_CARPET_ID, 1);
    assert_eq!(
        use_block_at(&mut unsupported_base, clicked, false, 31),
        LocalTargetUsePrediction::Result(LocalInteractionResult::Fail)
    );
    assert_eq!(
        unsupported_base
            .probe_block(clicked)
            .unwrap()
            .block_state_id,
        state_id("minecraft:short_grass", [])
    );
    assert_eq!(held(&unsupported_base).count, 1);
    assert!(unsupported_base.local_block_predictions().is_empty());
}

#[test]
fn redstone_wire_placement_reuses_connections_and_exact_support_rules() {
    let clicked = BlockPos { x: 4, y: 8, z: 4 };
    let target = relative(clicked, ProtocolDirection::Up);
    let east = relative(target, ProtocolDirection::East);

    let mut climbing = placement_world();
    set_block(&mut climbing, clicked, state_id("minecraft:stone", []));
    set_block(&mut climbing, east, state_id("minecraft:stone", []));
    set_block(
        &mut climbing,
        relative(east, ProtocolDirection::Up),
        state_id(
            "minecraft:redstone_wire",
            [
                ("east", "none"),
                ("north", "none"),
                ("power", "0"),
                ("south", "none"),
                ("west", "none"),
            ],
        ),
    );
    set_held(&mut climbing, VANILLA_ITEM_REDSTONE_ID, 1);
    assert!(matches!(
        use_block_at(&mut climbing, clicked, false, 32),
        LocalTargetUsePrediction::Result(LocalInteractionResult::Success(_))
    ));
    assert_eq!(
        climbing.probe_block(target).unwrap().block_state_id,
        state_id(
            "minecraft:redstone_wire",
            [
                ("east", "up"),
                ("north", "none"),
                ("power", "0"),
                ("south", "none"),
                ("west", "side"),
            ],
        )
    );

    let mut hopper = placement_world();
    set_block(
        &mut hopper,
        clicked,
        state_id(
            "minecraft:hopper",
            [("enabled", "true"), ("facing", "down")],
        ),
    );
    set_held(&mut hopper, VANILLA_ITEM_REDSTONE_ID, 1);
    assert!(matches!(
        use_block_at(&mut hopper, clicked, true, 33),
        LocalTargetUsePrediction::Result(LocalInteractionResult::Success(_))
    ));
    assert_eq!(
        hopper.probe_block(target).unwrap().block_state_id,
        state_id(
            "minecraft:redstone_wire",
            [
                ("east", "side"),
                ("north", "side"),
                ("power", "0"),
                ("south", "side"),
                ("west", "side"),
            ],
        )
    );

    let mut unsupported = placement_world();
    set_block(
        &mut unsupported,
        clicked,
        state_id(
            "minecraft:oak_slab",
            [("type", "bottom"), ("waterlogged", "false")],
        ),
    );
    set_held(&mut unsupported, VANILLA_ITEM_REDSTONE_ID, 1);
    assert_eq!(
        use_block_at(&mut unsupported, clicked, false, 34),
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
fn huge_mushroom_placement_hides_only_faces_against_the_same_block() {
    let clicked = BlockPos { x: 4, y: 8, z: 4 };
    let target = relative(clicked, ProtocolDirection::Up);
    let mushroom_state = || {
        state_id(
            "minecraft:brown_mushroom_block",
            [
                ("down", "true"),
                ("east", "true"),
                ("north", "true"),
                ("south", "true"),
                ("up", "true"),
                ("west", "true"),
            ],
        )
    };

    let mut world = placement_world();
    set_block(&mut world, clicked, mushroom_state());
    set_block(
        &mut world,
        relative(target, ProtocolDirection::Up),
        state_id(
            "minecraft:red_mushroom_block",
            [
                ("down", "true"),
                ("east", "true"),
                ("north", "true"),
                ("south", "true"),
                ("up", "true"),
                ("west", "true"),
            ],
        ),
    );
    set_block(
        &mut world,
        relative(target, ProtocolDirection::North),
        mushroom_state(),
    );
    set_block(
        &mut world,
        relative(target, ProtocolDirection::East),
        state_id(
            "minecraft:mushroom_stem",
            [
                ("down", "true"),
                ("east", "true"),
                ("north", "true"),
                ("south", "true"),
                ("up", "true"),
                ("west", "true"),
            ],
        ),
    );
    set_block(
        &mut world,
        relative(target, ProtocolDirection::South),
        mushroom_state(),
    );
    set_block(
        &mut world,
        relative(target, ProtocolDirection::West),
        state_id("minecraft:stone", []),
    );
    set_held(&mut world, VANILLA_ITEM_BROWN_MUSHROOM_BLOCK_ID, 1);

    assert!(matches!(
        use_block_at(&mut world, clicked, false, 61),
        LocalTargetUsePrediction::Result(LocalInteractionResult::Success(_))
    ));
    assert_eq!(
        world.probe_block(target).unwrap().block_state_id,
        state_id(
            "minecraft:brown_mushroom_block",
            [
                ("down", "false"),
                ("east", "true"),
                ("north", "false"),
                ("south", "false"),
                ("up", "true"),
                ("west", "true"),
            ],
        )
    );
    assert_eq!(held(&world).count, 0);
}

#[test]
fn trip_wire_placement_connects_to_wire_and_inward_facing_hooks() {
    let clicked = BlockPos { x: 4, y: 8, z: 4 };
    let target = relative(clicked, ProtocolDirection::Up);
    let trip_wire_state = || {
        state_id(
            "minecraft:tripwire",
            [
                ("attached", "false"),
                ("disarmed", "false"),
                ("east", "false"),
                ("north", "false"),
                ("powered", "false"),
                ("south", "false"),
                ("west", "false"),
            ],
        )
    };
    let hook_state = |facing| {
        state_id(
            "minecraft:tripwire_hook",
            [
                ("attached", "false"),
                ("facing", facing),
                ("powered", "false"),
            ],
        )
    };

    let mut world = placement_world();
    set_block(&mut world, clicked, state_id("minecraft:stone", []));
    set_block(
        &mut world,
        relative(target, ProtocolDirection::North),
        trip_wire_state(),
    );
    set_block(
        &mut world,
        relative(target, ProtocolDirection::East),
        hook_state("west"),
    );
    set_block(
        &mut world,
        relative(target, ProtocolDirection::South),
        hook_state("south"),
    );
    set_block(
        &mut world,
        relative(target, ProtocolDirection::West),
        state_id("minecraft:stone", []),
    );
    set_held(&mut world, VANILLA_ITEM_STRING_ID, 1);

    assert!(matches!(
        use_block_at(&mut world, clicked, false, 71),
        LocalTargetUsePrediction::Result(LocalInteractionResult::Success(_))
    ));
    assert_eq!(
        world.probe_block(target).unwrap().block_state_id,
        state_id(
            "minecraft:tripwire",
            [
                ("attached", "false"),
                ("disarmed", "false"),
                ("east", "true"),
                ("north", "true"),
                ("powered", "false"),
                ("south", "false"),
                ("west", "false"),
            ],
        )
    );
    assert_eq!(held(&world).count, 0);
}

#[test]
fn trip_wire_hook_placement_attaches_and_powers_the_whole_client_line() {
    let left_support = BlockPos { x: 3, y: 8, z: 4 };
    let placed_hook = BlockPos { x: 4, y: 8, z: 4 };
    let receiver = BlockPos { x: 8, y: 8, z: 4 };
    let right_support = BlockPos { x: 9, y: 8, z: 4 };
    let hook_state = |facing, attached, powered| {
        state_id(
            "minecraft:tripwire_hook",
            [
                ("attached", attached),
                ("facing", facing),
                ("powered", powered),
            ],
        )
    };
    let wire_state = |attached, powered| {
        state_id(
            "minecraft:tripwire",
            [
                ("attached", attached),
                ("disarmed", "false"),
                ("east", "true"),
                ("north", "false"),
                ("powered", powered),
                ("south", "false"),
                ("west", "true"),
            ],
        )
    };

    let mut world = placement_world();
    world.set_default_block_sound_profiles(BTreeMap::from([(
        "minecraft:tripwire_hook".to_string(),
        WorldBlockSoundProfile {
            break_sound: "minecraft:block.tripwire_hook.break".to_string(),
            place_sound: "minecraft:block.tripwire_hook.attach".to_string(),
            hit_sound: "minecraft:block.tripwire_hook.hit".to_string(),
            volume: 1.0,
            pitch: 1.0,
        },
    )]));
    set_block(&mut world, left_support, state_id("minecraft:stone", []));
    set_block(&mut world, right_support, state_id("minecraft:stone", []));
    set_block(&mut world, receiver, hook_state("west", "false", "false"));
    for (x, powered) in [(5, "true"), (6, "false"), (7, "false")] {
        set_block(
            &mut world,
            BlockPos { x, y: 8, z: 4 },
            wire_state("false", powered),
        );
    }
    set_held(&mut world, VANILLA_ITEM_TRIPWIRE_HOOK_ID, 1);

    let result = world.predict_local_block_use_with_hit(
        left_support,
        ProtocolDirection::East,
        [1.0, 0.5, 0.5],
        false,
        InteractionHand::MainHand,
        false,
        81,
    );
    let LocalTargetUsePrediction::Result(LocalInteractionResult::Success(success)) = result else {
        panic!("supported tripwire hook should place");
    };
    assert_eq!(
        success.sound_action,
        Some(LocalInteractionSoundAction::TripWireHookBlockPlace {
            pos: placed_hook,
            block_type_id: bbb_protocol::block_types::VANILLA_BLOCK_TRIPWIRE_HOOK_ID,
            suppressed_state_sound_count: 2,
            suppressed_detach_pitch: false,
        })
    );
    assert_eq!(
        world.probe_block(placed_hook).unwrap().block_state_id,
        hook_state("east", "true", "true")
    );
    assert_eq!(
        world.probe_block(receiver).unwrap().block_state_id,
        hook_state("west", "true", "true")
    );
    for x in 5..=7 {
        assert_eq!(
            world
                .probe_block(BlockPos { x, y: 8, z: 4 })
                .unwrap()
                .block_state_id,
            wire_state("true", if x == 5 { "true" } else { "false" })
        );
    }
    assert_eq!(world.local_block_predictions().len(), 5);
    assert_eq!(held(&world).count, 0);

    let mut seed_control = placement_world();
    let _first_suppressed_seed = seed_control.next_sound_seed();
    let _second_suppressed_seed = seed_control.next_sound_seed();
    let expected_place_seed = seed_control.next_sound_seed();
    let mut random = crate::LevelEventSoundRandomState::default();
    let effect = world
        .apply_local_interaction_sound_action(success.sound_action.unwrap(), &mut random)
        .expect("hook placement sound");
    let crate::LocalInteractionSoundEffect::Positioned(sound) = effect else {
        panic!("hook placement sound should be positioned");
    };
    assert_eq!(
        sound.sound.location.as_deref(),
        Some("minecraft:block.tripwire_hook.attach")
    );
    assert_eq!(sound.seed, expected_place_seed);

    world.apply_block_changed_ack(bbb_protocol::packets::BlockChangedAck { sequence: 81 });
    assert_eq!(
        world.probe_block(placed_hook).unwrap().block_state_id,
        state_id("minecraft:air", [])
    );
    assert_eq!(
        world.probe_block(receiver).unwrap().block_state_id,
        hook_state("west", "false", "false")
    );
    for (x, powered) in [(5, "true"), (6, "false"), (7, "false")] {
        assert_eq!(
            world
                .probe_block(BlockPos { x, y: 8, z: 4 })
                .unwrap()
                .block_state_id,
            wire_state("false", powered)
        );
    }

    let mut unsupported = placement_world();
    let unsupported_clicked = BlockPos { x: 4, y: 8, z: 4 };
    set_block(
        &mut unsupported,
        unsupported_clicked,
        state_id("minecraft:stone", []),
    );
    set_held(&mut unsupported, VANILLA_ITEM_TRIPWIRE_HOOK_ID, 1);
    assert_eq!(
        use_block_at(&mut unsupported, unsupported_clicked, false, 82),
        LocalTargetUsePrediction::Result(LocalInteractionResult::Fail)
    );
    assert_eq!(held(&unsupported).count, 1);
    assert!(unsupported.local_block_predictions().is_empty());
}

#[test]
fn trip_wire_hook_component_detach_advances_suppressed_random_streams() {
    let support = BlockPos { x: 3, y: 8, z: 4 };
    let target = BlockPos { x: 4, y: 8, z: 4 };
    let blocker = BlockPos { x: 5, y: 8, z: 4 };
    let hook_state = |facing, attached| {
        state_id(
            "minecraft:tripwire_hook",
            [
                ("attached", attached),
                ("facing", facing),
                ("powered", "false"),
            ],
        )
    };

    let mut world = placement_world();
    world.set_default_block_sound_profiles(BTreeMap::from([(
        "minecraft:tripwire_hook".to_string(),
        WorldBlockSoundProfile {
            break_sound: "minecraft:block.tripwire_hook.break".to_string(),
            place_sound: "minecraft:block.tripwire_hook.attach".to_string(),
            hit_sound: "minecraft:block.tripwire_hook.hit".to_string(),
            volume: 1.0,
            pitch: 1.0,
        },
    )]));
    set_block(&mut world, support, state_id("minecraft:stone", []));
    set_block(&mut world, blocker, hook_state("east", "false"));
    let mut stack = ItemStackSummary {
        item_id: Some(VANILLA_ITEM_TRIPWIRE_HOOK_ID),
        count: 1,
        component_patch: DataComponentPatchSummary::default(),
    };
    stack.component_patch.added = 1;
    stack
        .component_patch
        .added_type_ids
        .push(VANILLA_DATA_COMPONENT_BLOCK_STATE_TYPE_ID);
    stack.component_patch.block_state_properties =
        BTreeMap::from([("attached".to_string(), "true".to_string())]);
    world.apply_set_player_inventory(SetPlayerInventory {
        slot: 0,
        item: stack,
    });

    let result = world.predict_local_block_use_with_hit(
        support,
        ProtocolDirection::East,
        [1.0, 0.5, 0.5],
        false,
        InteractionHand::MainHand,
        false,
        83,
    );
    let LocalTargetUsePrediction::Result(LocalInteractionResult::Success(success)) = result else {
        panic!("component-modified hook should place");
    };
    assert_eq!(
        success.sound_action,
        Some(LocalInteractionSoundAction::TripWireHookBlockPlace {
            pos: target,
            block_type_id: bbb_protocol::block_types::VANILLA_BLOCK_TRIPWIRE_HOOK_ID,
            suppressed_state_sound_count: 1,
            suppressed_detach_pitch: true,
        })
    );
    assert_eq!(
        world.probe_block(target).unwrap().block_state_id,
        hook_state("east", "false")
    );

    let mut seed_control = placement_world();
    let _suppressed_seed = seed_control.next_sound_seed();
    let expected_place_seed = seed_control.next_sound_seed();
    let mut random = crate::LevelEventSoundRandomState::with_seed(83);
    let mut random_control = random.clone();
    let _detach_pitch_draw = random_control.next_float();
    let expected_next_float = random_control.next_float();
    let effect = world
        .apply_local_interaction_sound_action(success.sound_action.unwrap(), &mut random)
        .expect("hook placement sound");
    let crate::LocalInteractionSoundEffect::Positioned(sound) = effect else {
        panic!("hook placement sound should be positioned");
    };
    assert_eq!(sound.seed, expected_place_seed);
    assert_eq!(random.next_float(), expected_next_float);
}

#[test]
fn wall_placement_projects_side_height_and_center_post_from_the_block_above() {
    let clicked = BlockPos { x: 4, y: 8, z: 4 };
    let target = relative(clicked, ProtocolDirection::Up);
    let north = relative(target, ProtocolDirection::North);
    let south = relative(target, ProtocolDirection::South);

    let mut uncapped = placement_world();
    set_connection_tags(&mut uncapped);
    set_block(&mut uncapped, clicked, state_id("minecraft:stone", []));
    set_block(&mut uncapped, north, state_id("minecraft:stone", []));
    set_block(&mut uncapped, south, state_id("minecraft:stone", []));
    set_held(&mut uncapped, VANILLA_ITEM_COBBLESTONE_WALL_ID, 1);
    assert!(matches!(
        use_block_at(&mut uncapped, clicked, false, 24),
        LocalTargetUsePrediction::Result(LocalInteractionResult::Success(_))
    ));
    assert_eq!(
        uncapped.probe_block(target).unwrap().block_state_id,
        state_id(
            "minecraft:cobblestone_wall",
            [
                ("east", "none"),
                ("north", "low"),
                ("south", "low"),
                ("up", "false"),
                ("waterlogged", "false"),
                ("west", "none"),
            ],
        )
    );

    let mut capped = placement_world();
    set_connection_tags(&mut capped);
    set_block(&mut capped, clicked, state_id("minecraft:stone", []));
    set_block(&mut capped, north, state_id("minecraft:stone", []));
    set_block(&mut capped, south, state_id("minecraft:stone", []));
    set_block(
        &mut capped,
        relative(target, ProtocolDirection::Up),
        state_id("minecraft:stone", []),
    );
    set_held(&mut capped, VANILLA_ITEM_COBBLESTONE_WALL_ID, 1);
    assert!(matches!(
        use_block_at(&mut capped, clicked, false, 25),
        LocalTargetUsePrediction::Result(LocalInteractionResult::Success(_))
    ));
    assert_eq!(
        capped.probe_block(target).unwrap().block_state_id,
        state_id(
            "minecraft:cobblestone_wall",
            [
                ("east", "none"),
                ("north", "tall"),
                ("south", "tall"),
                ("up", "false"),
                ("waterlogged", "false"),
                ("west", "none"),
            ],
        )
    );
}

fn assert_copper_chest_placement(
    item_id: i32,
    neighbour_name: &str,
    expected_name: &str,
    sequence: i32,
) {
    let clicked = BlockPos { x: 4, y: 8, z: 4 };
    let target = relative(clicked, ProtocolDirection::Up);
    let east = relative(target, ProtocolDirection::East);
    let mut world = placement_world();
    set_connection_tags(&mut world);
    set_block(&mut world, clicked, state_id("minecraft:stone", []));
    set_block(
        &mut world,
        east,
        state_id(
            neighbour_name,
            [
                ("facing", "north"),
                ("type", "single"),
                ("waterlogged", "false"),
            ],
        ),
    );
    set_held(&mut world, item_id, 1);
    assert!(matches!(
        use_block_at(&mut world, clicked, false, sequence),
        LocalTargetUsePrediction::Result(LocalInteractionResult::Success(_))
    ));
    assert_eq!(
        world.probe_block(target).unwrap().block_state_id,
        state_id(
            expected_name,
            [
                ("facing", "north"),
                ("type", "left"),
                ("waterlogged", "false"),
            ],
        )
    );
    assert_eq!(held(&world).count, 0);
}

fn set_connection_tags(world: &mut WorldStore) {
    world.apply_update_tags(UpdateTags {
        registries: vec![RegistryTags {
            registry: "minecraft:block".to_string(),
            tags: vec![
                TagNetworkPayload {
                    tag: "minecraft:fences".to_string(),
                    entries: vec![
                        VANILLA_BLOCK_OAK_FENCE_ID,
                        VANILLA_BLOCK_NETHER_BRICK_FENCE_ID,
                    ],
                },
                TagNetworkPayload {
                    tag: "minecraft:wooden_fences".to_string(),
                    entries: vec![VANILLA_BLOCK_OAK_FENCE_ID],
                },
                TagNetworkPayload {
                    tag: "minecraft:walls".to_string(),
                    entries: vec![VANILLA_BLOCK_COBBLESTONE_WALL_ID],
                },
                TagNetworkPayload {
                    tag: "minecraft:shulker_boxes".to_string(),
                    entries: Vec::new(),
                },
                TagNetworkPayload {
                    tag: "minecraft:wall_post_override".to_string(),
                    entries: Vec::new(),
                },
                TagNetworkPayload {
                    tag: "minecraft:copper_chests".to_string(),
                    entries: vec![
                        VANILLA_BLOCK_COPPER_CHEST_ID,
                        VANILLA_BLOCK_EXPOSED_COPPER_CHEST_ID,
                        VANILLA_BLOCK_WEATHERED_COPPER_CHEST_ID,
                        VANILLA_BLOCK_OXIDIZED_COPPER_CHEST_ID,
                        VANILLA_BLOCK_WAXED_COPPER_CHEST_ID,
                        VANILLA_BLOCK_WAXED_EXPOSED_COPPER_CHEST_ID,
                        VANILLA_BLOCK_WAXED_WEATHERED_COPPER_CHEST_ID,
                        VANILLA_BLOCK_WAXED_OXIDIZED_COPPER_CHEST_ID,
                    ],
                },
            ],
        }],
    });
}
