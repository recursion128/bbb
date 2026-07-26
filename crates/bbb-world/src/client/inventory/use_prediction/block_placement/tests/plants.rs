use super::*;

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
