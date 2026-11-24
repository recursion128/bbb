use super::*;

#[test]
fn multiface_items_select_the_first_vacant_supported_face_and_source_water() {
    let clicked = BlockPos { x: 4, y: 8, z: 4 };
    let target = relative(clicked, ProtocolDirection::Up);
    for (sequence, item_id, block_name) in [
        (107, VANILLA_ITEM_GLOW_LICHEN_ID, "minecraft:glow_lichen"),
        (108, VANILLA_ITEM_RESIN_CLUMP_ID, "minecraft:resin_clump"),
        (109, VANILLA_ITEM_SCULK_VEIN_ID, "minecraft:sculk_vein"),
    ] {
        let mut world = placement_world();
        set_block(&mut world, clicked, state_id("minecraft:stone", []));
        set_held(&mut world, item_id, 1);
        assert!(matches!(
            use_block_at(&mut world, clicked, false, sequence),
            LocalTargetUsePrediction::Result(LocalInteractionResult::Success(_))
        ));
        assert_eq!(
            world.probe_block(target).unwrap().block_state_id,
            state_id(
                block_name,
                [
                    ("down", "true"),
                    ("east", "false"),
                    ("north", "false"),
                    ("south", "false"),
                    ("up", "false"),
                    ("waterlogged", "false"),
                    ("west", "false"),
                ],
            )
        );
        assert_eq!(held(&world).count, 0);
    }

    let mut waterlogged = placement_world();
    set_block(&mut waterlogged, clicked, state_id("minecraft:stone", []));
    set_block(
        &mut waterlogged,
        target,
        state_id("minecraft:water", [("level", "0")]),
    );
    set_held(&mut waterlogged, VANILLA_ITEM_GLOW_LICHEN_ID, 1);
    assert!(matches!(
        use_block_at(&mut waterlogged, clicked, false, 110),
        LocalTargetUsePrediction::Result(LocalInteractionResult::Success(_))
    ));
    assert_eq!(
        waterlogged.probe_block(target).unwrap().block_state_id,
        state_id(
            "minecraft:glow_lichen",
            [
                ("down", "true"),
                ("east", "false"),
                ("north", "false"),
                ("south", "false"),
                ("up", "false"),
                ("waterlogged", "true"),
                ("west", "false"),
            ],
        )
    );

    let south = relative(target, ProtocolDirection::South);
    set_block(&mut waterlogged, south, state_id("minecraft:stone", []));
    set_held(&mut waterlogged, VANILLA_ITEM_GLOW_LICHEN_ID, 1);
    assert!(matches!(
        use_block_at(&mut waterlogged, target, false, 111),
        LocalTargetUsePrediction::Result(LocalInteractionResult::Success(_))
    ));
    assert_eq!(
        waterlogged.probe_block(target).unwrap().block_state_id,
        state_id(
            "minecraft:glow_lichen",
            [
                ("down", "true"),
                ("east", "false"),
                ("north", "false"),
                ("south", "true"),
                ("up", "false"),
                ("waterlogged", "true"),
                ("west", "false"),
            ],
        )
    );
}

#[test]
fn vine_placement_adds_supported_faces_and_can_reuse_an_unchanged_vine() {
    let support = BlockPos { x: 4, y: 8, z: 4 };
    let target = relative(support, ProtocolDirection::East);
    let mut world = placement_world();
    set_block(&mut world, support, state_id("minecraft:stone", []));
    set_held(&mut world, VANILLA_ITEM_VINE_ID, 3);
    assert!(matches!(
        world.predict_local_block_use_with_hit(
            support,
            ProtocolDirection::East,
            [1.0, 0.5, 0.5],
            false,
            InteractionHand::MainHand,
            false,
            112,
        ),
        LocalTargetUsePrediction::Result(LocalInteractionResult::Success(_))
    ));
    assert_eq!(
        world.probe_block(target).unwrap().block_state_id,
        state_id(
            "minecraft:vine",
            [
                ("east", "false"),
                ("north", "false"),
                ("south", "false"),
                ("up", "false"),
                ("west", "true"),
            ],
        )
    );

    set_block(
        &mut world,
        relative(target, ProtocolDirection::South),
        state_id("minecraft:stone", []),
    );
    assert!(matches!(
        use_block_at(&mut world, target, false, 113),
        LocalTargetUsePrediction::Result(LocalInteractionResult::Success(_))
    ));
    assert_eq!(
        world.probe_block(target).unwrap().block_state_id,
        state_id(
            "minecraft:vine",
            [
                ("east", "false"),
                ("north", "false"),
                ("south", "true"),
                ("up", "false"),
                ("west", "true"),
            ],
        )
    );

    set_block(
        &mut world,
        relative(target, ProtocolDirection::South),
        state_id("minecraft:air", []),
    );
    assert!(matches!(
        use_block_at(&mut world, target, false, 114),
        LocalTargetUsePrediction::Result(LocalInteractionResult::Success(_))
    ));
    assert_eq!(held(&world).count, 0);
}

#[test]
fn scaffolding_item_updates_the_target_and_allows_the_distance_seven_edge() {
    let clicked = BlockPos { x: 4, y: 8, z: 4 };
    let target = relative(clicked, ProtocolDirection::Up);
    let mut supported = placement_world();
    set_block(&mut supported, clicked, state_id("minecraft:stone", []));
    set_block(
        &mut supported,
        target,
        state_id("minecraft:water", [("level", "0")]),
    );
    set_held(&mut supported, VANILLA_ITEM_SCAFFOLDING_ID, 1);
    assert!(matches!(
        use_block_at(&mut supported, clicked, false, 115),
        LocalTargetUsePrediction::Result(LocalInteractionResult::Success(_))
    ));
    assert_eq!(
        supported.probe_block(target).unwrap().block_state_id,
        state_id(
            "minecraft:scaffolding",
            [
                ("bottom", "false"),
                ("distance", "0"),
                ("waterlogged", "true"),
            ],
        )
    );

    let mut flowing = placement_world();
    set_block(&mut flowing, clicked, state_id("minecraft:stone", []));
    set_block(
        &mut flowing,
        target,
        state_id("minecraft:water", [("level", "3")]),
    );
    set_held(&mut flowing, VANILLA_ITEM_SCAFFOLDING_ID, 1);
    assert!(matches!(
        use_block_at(&mut flowing, clicked, false, 118),
        LocalTargetUsePrediction::Result(LocalInteractionResult::Success(_))
    ));
    assert_eq!(
        flowing.probe_block(target).unwrap().block_state_id,
        state_id(
            "minecraft:scaffolding",
            [
                ("bottom", "false"),
                ("distance", "0"),
                ("waterlogged", "false"),
            ],
        )
    );

    let scaffold = BlockPos { x: 4, y: 8, z: 4 };
    let horizontal_target = relative(scaffold, ProtocolDirection::South);
    let mut horizontal = placement_world();
    set_block(
        &mut horizontal,
        scaffold,
        state_id(
            "minecraft:scaffolding",
            [
                ("bottom", "true"),
                ("distance", "6"),
                ("waterlogged", "false"),
            ],
        ),
    );
    set_held(&mut horizontal, VANILLA_ITEM_SCAFFOLDING_ID, 1);
    assert!(matches!(
        use_block_at(&mut horizontal, scaffold, false, 116),
        LocalTargetUsePrediction::Result(LocalInteractionResult::Success(_))
    ));
    assert_eq!(
        horizontal
            .probe_block(horizontal_target)
            .unwrap()
            .block_state_id,
        state_id(
            "minecraft:scaffolding",
            [
                ("bottom", "true"),
                ("distance", "7"),
                ("waterlogged", "false"),
            ],
        )
    );

    let mut vine_candidate = placement_world();
    set_block(
        &mut vine_candidate,
        scaffold,
        state_id(
            "minecraft:scaffolding",
            [
                ("bottom", "false"),
                ("distance", "0"),
                ("waterlogged", "false"),
            ],
        ),
    );
    set_block(
        &mut vine_candidate,
        horizontal_target,
        state_id(
            "minecraft:vine",
            [
                ("east", "true"),
                ("north", "true"),
                ("south", "true"),
                ("up", "true"),
                ("west", "true"),
            ],
        ),
    );
    set_held(&mut vine_candidate, VANILLA_ITEM_SCAFFOLDING_ID, 1);
    assert!(matches!(
        use_block_at(&mut vine_candidate, scaffold, false, 120),
        LocalTargetUsePrediction::Result(LocalInteractionResult::Success(_))
    ));
    assert_eq!(
        vine_candidate
            .probe_block(horizontal_target)
            .unwrap()
            .block_state_id,
        state_id(
            "minecraft:scaffolding",
            [
                ("bottom", "true"),
                ("distance", "1"),
                ("waterlogged", "false"),
            ],
        )
    );

    let vertical_target = relative(scaffold, ProtocolDirection::Up);
    let mut vertical = placement_world();
    set_block(
        &mut vertical,
        scaffold,
        state_id(
            "minecraft:scaffolding",
            [
                ("bottom", "true"),
                ("distance", "2"),
                ("waterlogged", "false"),
            ],
        ),
    );
    set_held(&mut vertical, VANILLA_ITEM_SCAFFOLDING_ID, 1);
    assert!(matches!(
        vertical.predict_local_block_use_with_hit(
            scaffold,
            ProtocolDirection::East,
            [1.0, 0.5, 0.5],
            false,
            InteractionHand::MainHand,
            false,
            119,
        ),
        LocalTargetUsePrediction::Result(LocalInteractionResult::Success(_))
    ));
    assert_eq!(
        vertical
            .probe_block(vertical_target)
            .unwrap()
            .block_state_id,
        state_id(
            "minecraft:scaffolding",
            [
                ("bottom", "false"),
                ("distance", "2"),
                ("waterlogged", "false"),
            ],
        )
    );

    let unsupported_target = BlockPos { x: 6, y: 9, z: 6 };
    let mut unsupported = placement_world();
    set_held(&mut unsupported, VANILLA_ITEM_SCAFFOLDING_ID, 1);
    assert_eq!(
        use_block_at(&mut unsupported, unsupported_target, false, 117),
        LocalTargetUsePrediction::Result(LocalInteractionResult::Fail)
    );
    assert_eq!(held(&unsupported).count, 1);
    assert!(unsupported.local_block_predictions().is_empty());
}
