use super::*;

#[test]
fn concrete_powder_solidifies_in_replaced_or_exposed_water() {
    let mut replaced_water = placement_world();
    set_block(
        &mut replaced_water,
        CLICKED,
        state_id("minecraft:water", [("level", "0")]),
    );
    set_held(
        &mut replaced_water,
        VANILLA_ITEM_WHITE_CONCRETE_POWDER_ID,
        1,
    );
    assert!(matches!(
        use_block(&mut replaced_water, 51),
        LocalTargetUsePrediction::Result(LocalInteractionResult::Success(_))
    ));
    assert_eq!(
        replaced_water.probe_block(CLICKED).unwrap().block_state_id,
        state_id("minecraft:white_concrete", [])
    );

    let mut exposed_water = placement_world();
    set_block(
        &mut exposed_water,
        relative(TARGET, ProtocolDirection::Up),
        state_id("minecraft:water", [("level", "1")]),
    );
    set_held(&mut exposed_water, VANILLA_ITEM_WHITE_CONCRETE_POWDER_ID, 1);
    assert!(matches!(
        use_block(&mut exposed_water, 52),
        LocalTargetUsePrediction::Result(LocalInteractionResult::Success(_))
    ));
    assert_eq!(
        exposed_water.probe_block(TARGET).unwrap().block_state_id,
        state_id("minecraft:white_concrete", [])
    );
}

#[test]
fn concrete_powder_stays_powder_when_dry_below_water_or_water_is_face_sealed() {
    let clicked = BlockPos { x: 4, y: 8, z: 4 };
    let upper_target = relative(clicked, ProtocolDirection::Up);
    let mut dry = placement_world();
    set_block(&mut dry, clicked, state_id("minecraft:stone", []));
    set_held(&mut dry, VANILLA_ITEM_WHITE_CONCRETE_POWDER_ID, 1);
    assert!(matches!(
        dry.predict_local_block_use_with_hit(
            clicked,
            ProtocolDirection::Up,
            [0.5, 1.0, 0.5],
            false,
            InteractionHand::MainHand,
            false,
            53,
        ),
        LocalTargetUsePrediction::Result(LocalInteractionResult::Success(_))
    ));
    assert_eq!(
        dry.probe_block(upper_target).unwrap().block_state_id,
        state_id("minecraft:white_concrete_powder", [])
    );

    let side_target = relative(clicked, ProtocolDirection::East);
    let mut below_water = placement_world();
    set_block(&mut below_water, clicked, state_id("minecraft:stone", []));
    set_block(
        &mut below_water,
        relative(side_target, ProtocolDirection::Down),
        state_id("minecraft:water", [("level", "0")]),
    );
    set_held(&mut below_water, VANILLA_ITEM_WHITE_CONCRETE_POWDER_ID, 1);
    assert!(matches!(
        below_water.predict_local_block_use_with_hit(
            clicked,
            ProtocolDirection::East,
            [1.0, 0.5, 0.5],
            false,
            InteractionHand::MainHand,
            false,
            54,
        ),
        LocalTargetUsePrediction::Result(LocalInteractionResult::Success(_))
    ));
    assert_eq!(
        below_water.probe_block(side_target).unwrap().block_state_id,
        state_id("minecraft:white_concrete_powder", [])
    );

    let mut sealed_water = placement_world();
    set_block(&mut sealed_water, clicked, state_id("minecraft:stone", []));
    set_block(
        &mut sealed_water,
        relative(upper_target, ProtocolDirection::East),
        state_id("minecraft:barrier", [("waterlogged", "true")]),
    );
    set_held(&mut sealed_water, VANILLA_ITEM_WHITE_CONCRETE_POWDER_ID, 1);
    assert!(matches!(
        sealed_water.predict_local_block_use_with_hit(
            clicked,
            ProtocolDirection::Up,
            [0.5, 1.0, 0.5],
            false,
            InteractionHand::MainHand,
            false,
            55,
        ),
        LocalTargetUsePrediction::Result(LocalInteractionResult::Success(_))
    ));
    assert_eq!(
        sealed_water
            .probe_block(upper_target)
            .unwrap()
            .block_state_id,
        state_id("minecraft:white_concrete_powder", [])
    );
}
