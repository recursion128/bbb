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
