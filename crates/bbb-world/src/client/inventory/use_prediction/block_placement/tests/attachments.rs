use super::*;

#[test]
fn hanging_sign_item_selects_ceiling_chain_and_wall_forms() {
    let ceiling_support = BlockPos { x: 4, y: 10, z: 4 };
    let ceiling_target = relative(ceiling_support, ProtocolDirection::Down);
    let mut ceiling = placement_world();
    set_block(
        &mut ceiling,
        ceiling_support,
        state_id("minecraft:stone", []),
    );
    set_block_tag(&mut ceiling, "minecraft:all_hanging_signs", &[]);
    set_held(&mut ceiling, VANILLA_ITEM_OAK_HANGING_SIGN_ID, 1);
    assert!(matches!(
        ceiling.predict_local_block_use_with_hit(
            ceiling_support,
            ProtocolDirection::Down,
            [0.25, 0.0, 0.75],
            false,
            InteractionHand::MainHand,
            false,
            40,
        ),
        LocalTargetUsePrediction::Result(LocalInteractionResult::Success(_))
    ));
    assert_eq!(
        ceiling.probe_block(ceiling_target).unwrap().block_state_id,
        state_id(
            "minecraft:oak_hanging_sign",
            [
                ("attached", "false"),
                ("rotation", "8"),
                ("waterlogged", "false"),
            ],
        )
    );

    let mut attached = placement_world();
    attached.set_local_player_pose(LocalPlayerPoseState {
        y_rot: 22.5,
        position: Vec3d {
            x: 8.0,
            y: 8.0,
            z: 8.0,
        },
        ..LocalPlayerPoseState::default()
    });
    set_block(
        &mut attached,
        ceiling_support,
        state_id("minecraft:stone", []),
    );
    set_block_tag(&mut attached, "minecraft:all_hanging_signs", &[]);
    set_held(&mut attached, VANILLA_ITEM_OAK_HANGING_SIGN_ID, 1);
    assert!(matches!(
        attached.predict_local_block_use_with_hit(
            ceiling_support,
            ProtocolDirection::Down,
            [0.25, 0.0, 0.75],
            false,
            InteractionHand::MainHand,
            true,
            41,
        ),
        LocalTargetUsePrediction::Result(LocalInteractionResult::Success(_))
    ));
    assert_eq!(
        attached.probe_block(ceiling_target).unwrap().block_state_id,
        state_id(
            "minecraft:oak_hanging_sign",
            [
                ("attached", "true"),
                ("rotation", "9"),
                ("waterlogged", "false"),
            ],
        )
    );

    let wall_clicked = BlockPos { x: 4, y: 8, z: 4 };
    let wall_target = relative(wall_clicked, ProtocolDirection::East);
    let mut wall = placement_world();
    set_block(&mut wall, wall_clicked, state_id("minecraft:stone", []));
    set_block_tag(&mut wall, "minecraft:wall_hanging_signs", &[]);
    set_held(&mut wall, VANILLA_ITEM_OAK_HANGING_SIGN_ID, 1);
    assert!(matches!(
        wall.predict_local_block_use_with_hit(
            wall_clicked,
            ProtocolDirection::East,
            [1.0, 0.5, 0.5],
            false,
            InteractionHand::MainHand,
            false,
            42,
        ),
        LocalTargetUsePrediction::Result(LocalInteractionResult::Success(_))
    ));
    assert_eq!(
        wall.probe_block(wall_target).unwrap().block_state_id,
        state_id(
            "minecraft:oak_wall_hanging_sign",
            [("facing", "north"), ("waterlogged", "false")],
        )
    );
}

#[test]
fn hanging_sign_chain_snaps_axis_and_missing_support_fails_without_consuming() {
    let chain_support = BlockPos { x: 4, y: 10, z: 4 };
    let chain_target = relative(chain_support, ProtocolDirection::Down);
    let mut chain = placement_world();
    set_block(
        &mut chain,
        chain_support,
        state_id(
            "minecraft:oak_wall_hanging_sign",
            [("facing", "north"), ("waterlogged", "false")],
        ),
    );
    set_block_tag(
        &mut chain,
        "minecraft:all_hanging_signs",
        &[VANILLA_BLOCK_OAK_WALL_HANGING_SIGN_ID],
    );
    set_held(&mut chain, VANILLA_ITEM_OAK_HANGING_SIGN_ID, 1);
    let chain_result = chain.predict_local_block_use_with_hit(
        chain_support,
        ProtocolDirection::Down,
        [0.25, 0.0, 0.75],
        false,
        InteractionHand::MainHand,
        false,
        43,
    );
    assert!(
        matches!(
            chain_result,
            LocalTargetUsePrediction::Result(LocalInteractionResult::Success(_))
        ),
        "unexpected chain result: {chain_result:?}"
    );
    assert_eq!(
        chain.probe_block(chain_target).unwrap().block_state_id,
        state_id(
            "minecraft:oak_hanging_sign",
            [
                ("attached", "false"),
                ("rotation", "8"),
                ("waterlogged", "false"),
            ],
        )
    );

    let wall_chain_clicked = BlockPos { x: 4, y: 8, z: 4 };
    let wall_chain_target = relative(wall_chain_clicked, ProtocolDirection::East);
    let mut wall_chain = placement_world();
    set_block(
        &mut wall_chain,
        wall_chain_clicked,
        state_id(
            "minecraft:oak_wall_hanging_sign",
            [("facing", "north"), ("waterlogged", "false")],
        ),
    );
    wall_chain.apply_update_tags(UpdateTags {
        registries: vec![RegistryTags {
            registry: "minecraft:block".to_string(),
            tags: vec![
                TagNetworkPayload {
                    tag: "minecraft:all_hanging_signs".to_string(),
                    entries: vec![VANILLA_BLOCK_OAK_WALL_HANGING_SIGN_ID],
                },
                TagNetworkPayload {
                    tag: "minecraft:wall_hanging_signs".to_string(),
                    entries: vec![VANILLA_BLOCK_OAK_WALL_HANGING_SIGN_ID],
                },
            ],
        }],
    });
    set_held(&mut wall_chain, VANILLA_ITEM_OAK_HANGING_SIGN_ID, 1);
    assert!(matches!(
        wall_chain.predict_local_block_use_with_hit(
            wall_chain_clicked,
            ProtocolDirection::East,
            [1.0, 0.5, 0.5],
            false,
            InteractionHand::MainHand,
            false,
            44,
        ),
        LocalTargetUsePrediction::Result(LocalInteractionResult::Success(_))
    ));
    assert_eq!(
        wall_chain
            .probe_block(wall_chain_target)
            .unwrap()
            .block_state_id,
        state_id(
            "minecraft:oak_wall_hanging_sign",
            [("facing", "north"), ("waterlogged", "false")],
        )
    );

    let mut editable_side = placement_world();
    set_block(
        &mut editable_side,
        wall_chain_clicked,
        state_id(
            "minecraft:oak_wall_hanging_sign",
            [("facing", "east"), ("waterlogged", "false")],
        ),
    );
    set_held(&mut editable_side, VANILLA_ITEM_OAK_HANGING_SIGN_ID, 1);
    assert_eq!(
        editable_side.predict_local_block_use_with_hit(
            wall_chain_clicked,
            ProtocolDirection::East,
            [1.0, 0.5, 0.5],
            false,
            InteractionHand::MainHand,
            false,
            45,
        ),
        LocalTargetUsePrediction::UnsupportedBlockUseItemOn(
            VanillaBlockUseItemOnOverride::WallHangingSignBlock
        )
    );
    assert_eq!(held(&editable_side).count, 1);
    assert!(editable_side.local_block_predictions().is_empty());

    let unsupported_clicked = BlockPos { x: 4, y: 8, z: 4 };
    let mut unsupported = placement_world();
    set_block(
        &mut unsupported,
        unsupported_clicked,
        state_id("minecraft:stone", []),
    );
    unsupported.apply_update_tags(UpdateTags {
        registries: vec![RegistryTags {
            registry: "minecraft:block".to_string(),
            tags: vec![
                TagNetworkPayload {
                    tag: "minecraft:all_hanging_signs".to_string(),
                    entries: Vec::new(),
                },
                TagNetworkPayload {
                    tag: "minecraft:wall_hanging_signs".to_string(),
                    entries: Vec::new(),
                },
            ],
        }],
    });
    set_held(&mut unsupported, VANILLA_ITEM_OAK_HANGING_SIGN_ID, 1);
    assert_eq!(
        use_block_at(&mut unsupported, unsupported_clicked, false, 46),
        LocalTargetUsePrediction::Result(LocalInteractionResult::Fail)
    );
    assert_eq!(held(&unsupported).count, 1);
    assert!(unsupported.local_block_predictions().is_empty());
}

#[test]
fn gate_and_trapdoor_use_exact_wall_hit_and_signal_free_states() {
    let clicked = BlockPos { x: 4, y: 8, z: 4 };
    let gate_target = relative(clicked, ProtocolDirection::Up);
    let mut gate = placement_world();
    set_block(&mut gate, clicked, state_id("minecraft:stone", []));
    set_block(
        &mut gate,
        relative(gate_target, ProtocolDirection::East),
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
    set_block_tag(
        &mut gate,
        "minecraft:walls",
        &[VANILLA_BLOCK_COBBLESTONE_WALL_ID],
    );
    set_held(&mut gate, VANILLA_ITEM_OAK_FENCE_GATE_ID, 1);
    assert!(matches!(
        use_block_at(&mut gate, clicked, false, 30),
        LocalTargetUsePrediction::Result(LocalInteractionResult::Success(_))
    ));
    assert_eq!(
        gate.probe_block(gate_target).unwrap().block_state_id,
        state_id(
            "minecraft:oak_fence_gate",
            [
                ("facing", "south"),
                ("in_wall", "true"),
                ("open", "false"),
                ("powered", "false"),
            ],
        )
    );

    let trapdoor_target = relative(clicked, ProtocolDirection::East);
    let mut trapdoor = placement_world();
    set_block(&mut trapdoor, clicked, state_id("minecraft:stone", []));
    set_held(&mut trapdoor, VANILLA_ITEM_OAK_TRAPDOOR_ID, 1);
    assert!(matches!(
        trapdoor.predict_local_block_use_with_hit(
            clicked,
            ProtocolDirection::East,
            [1.0, 0.75, 0.5],
            false,
            InteractionHand::MainHand,
            false,
            31,
        ),
        LocalTargetUsePrediction::Result(LocalInteractionResult::Success(_))
    ));
    assert_eq!(
        trapdoor
            .probe_block(trapdoor_target)
            .unwrap()
            .block_state_id,
        state_id(
            "minecraft:oak_trapdoor",
            [
                ("facing", "east"),
                ("half", "top"),
                ("open", "false"),
                ("powered", "false"),
                ("waterlogged", "false"),
            ],
        )
    );

    let mut uncertain = placement_world();
    set_block(&mut uncertain, clicked, state_id("minecraft:stone", []));
    set_block(
        &mut uncertain,
        relative(gate_target, ProtocolDirection::West),
        state_id("minecraft:redstone_block", []),
    );
    set_held(&mut uncertain, VANILLA_ITEM_OAK_FENCE_GATE_ID, 1);
    assert_eq!(
        use_block_at(&mut uncertain, clicked, false, 32),
        LocalTargetUsePrediction::UnsupportedItemUseOn(VanillaItemUseOnOverride::BlockItem)
    );
    assert_eq!(held(&uncertain).count, 1);
    assert!(uncertain.local_block_predictions().is_empty());
}

#[test]
fn bell_selects_double_wall_and_falls_back_to_floor() {
    let clicked = BlockPos { x: 4, y: 8, z: 4 };
    let target = relative(clicked, ProtocolDirection::East);
    let mut double_wall = placement_world();
    set_block(&mut double_wall, clicked, state_id("minecraft:stone", []));
    set_block(
        &mut double_wall,
        relative(target, ProtocolDirection::East),
        state_id("minecraft:stone", []),
    );
    set_held(&mut double_wall, VANILLA_ITEM_BELL_ID, 1);
    assert!(matches!(
        double_wall.predict_local_block_use_with_hit(
            clicked,
            ProtocolDirection::East,
            [1.0, 0.5, 0.5],
            false,
            InteractionHand::MainHand,
            false,
            33,
        ),
        LocalTargetUsePrediction::Result(LocalInteractionResult::Success(_))
    ));
    assert_eq!(
        double_wall.probe_block(target).unwrap().block_state_id,
        state_id(
            "minecraft:bell",
            [
                ("attachment", "double_wall"),
                ("facing", "west"),
                ("powered", "false"),
            ],
        )
    );

    let mut floor_fallback = placement_world();
    set_block(
        &mut floor_fallback,
        clicked,
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
    set_block(
        &mut floor_fallback,
        relative(target, ProtocolDirection::Down),
        state_id("minecraft:stone", []),
    );
    set_held(&mut floor_fallback, VANILLA_ITEM_BELL_ID, 1);
    assert!(matches!(
        floor_fallback.predict_local_block_use_with_hit(
            clicked,
            ProtocolDirection::East,
            [1.0, 0.5, 0.5],
            false,
            InteractionHand::MainHand,
            false,
            34,
        ),
        LocalTargetUsePrediction::Result(LocalInteractionResult::Success(_))
    ));
    assert_eq!(
        floor_fallback.probe_block(target).unwrap().block_state_id,
        state_id(
            "minecraft:bell",
            [
                ("attachment", "floor"),
                ("facing", "west"),
                ("powered", "false"),
            ],
        )
    );
}

#[test]
fn cocoa_and_propagule_apply_tag_support_and_placed_age() {
    let clicked = BlockPos { x: 4, y: 8, z: 4 };
    let cocoa_target = relative(clicked, ProtocolDirection::East);
    let mut cocoa = placement_world();
    set_block(&mut cocoa, clicked, state_id("minecraft:stone", []));
    set_block_tag(
        &mut cocoa,
        "minecraft:supports_cocoa",
        &[VANILLA_BLOCK_STONE_ID],
    );
    set_held(&mut cocoa, VANILLA_ITEM_COCOA_BEANS_ID, 1);
    assert!(matches!(
        cocoa.predict_local_block_use_with_hit(
            clicked,
            ProtocolDirection::East,
            [1.0, 0.5, 0.5],
            false,
            InteractionHand::MainHand,
            false,
            35,
        ),
        LocalTargetUsePrediction::Result(LocalInteractionResult::Success(_))
    ));
    assert_eq!(
        cocoa.probe_block(cocoa_target).unwrap().block_state_id,
        state_id("minecraft:cocoa", [("age", "0"), ("facing", "west")])
    );

    let propagule_target = relative(clicked, ProtocolDirection::Up);
    let mut propagule = placement_world();
    set_block(&mut propagule, clicked, state_id("minecraft:stone", []));
    set_block_tag(
        &mut propagule,
        "minecraft:supports_mangrove_propagule",
        &[VANILLA_BLOCK_STONE_ID],
    );
    set_held(&mut propagule, VANILLA_ITEM_MANGROVE_PROPAGULE_ID, 1);
    assert!(matches!(
        use_block_at(&mut propagule, clicked, false, 36),
        LocalTargetUsePrediction::Result(LocalInteractionResult::Success(_))
    ));
    assert_eq!(
        propagule
            .probe_block(propagule_target)
            .unwrap()
            .block_state_id,
        state_id(
            "minecraft:mangrove_propagule",
            [
                ("age", "4"),
                ("hanging", "false"),
                ("stage", "0"),
                ("waterlogged", "false"),
            ],
        )
    );
}

#[test]
fn pointed_dripstone_computes_merge_and_middle_thickness() {
    let clicked = BlockPos { x: 4, y: 8, z: 4 };
    let target = relative(clicked, ProtocolDirection::Up);
    let front = relative(target, ProtocolDirection::Up);
    let mut merged = placement_world();
    set_block(&mut merged, clicked, state_id("minecraft:stone", []));
    set_block(
        &mut merged,
        front,
        state_id(
            "minecraft:pointed_dripstone",
            [
                ("thickness", "tip"),
                ("vertical_direction", "down"),
                ("waterlogged", "false"),
            ],
        ),
    );
    set_held(&mut merged, VANILLA_ITEM_POINTED_DRIPSTONE_ID, 1);
    assert!(matches!(
        use_block_at(&mut merged, clicked, false, 37),
        LocalTargetUsePrediction::Result(LocalInteractionResult::Success(_))
    ));
    assert_eq!(
        merged.probe_block(target).unwrap().block_state_id,
        state_id(
            "minecraft:pointed_dripstone",
            [
                ("thickness", "tip_merge"),
                ("vertical_direction", "up"),
                ("waterlogged", "false"),
            ],
        )
    );

    let mut separate_tips = placement_world();
    set_block(&mut separate_tips, clicked, state_id("minecraft:stone", []));
    set_block(
        &mut separate_tips,
        front,
        state_id(
            "minecraft:pointed_dripstone",
            [
                ("thickness", "tip"),
                ("vertical_direction", "down"),
                ("waterlogged", "false"),
            ],
        ),
    );
    set_held(&mut separate_tips, VANILLA_ITEM_POINTED_DRIPSTONE_ID, 1);
    assert!(matches!(
        use_block_at(&mut separate_tips, clicked, true, 38),
        LocalTargetUsePrediction::Result(LocalInteractionResult::Success(_))
    ));
    assert_eq!(
        separate_tips.probe_block(target).unwrap().block_state_id,
        state_id(
            "minecraft:pointed_dripstone",
            [
                ("thickness", "tip"),
                ("vertical_direction", "up"),
                ("waterlogged", "false"),
            ],
        )
    );

    let mut middle = placement_world();
    set_block(
        &mut middle,
        clicked,
        state_id(
            "minecraft:pointed_dripstone",
            [
                ("thickness", "base"),
                ("vertical_direction", "up"),
                ("waterlogged", "false"),
            ],
        ),
    );
    set_block(
        &mut middle,
        front,
        state_id(
            "minecraft:pointed_dripstone",
            [
                ("thickness", "frustum"),
                ("vertical_direction", "up"),
                ("waterlogged", "false"),
            ],
        ),
    );
    set_held(&mut middle, VANILLA_ITEM_POINTED_DRIPSTONE_ID, 1);
    assert!(matches!(
        use_block_at(&mut middle, clicked, false, 39),
        LocalTargetUsePrediction::Result(LocalInteractionResult::Success(_))
    ));
    assert_eq!(
        middle.probe_block(target).unwrap().block_state_id,
        state_id(
            "minecraft:pointed_dripstone",
            [
                ("thickness", "middle"),
                ("vertical_direction", "up"),
                ("waterlogged", "false"),
            ],
        )
    );
}
