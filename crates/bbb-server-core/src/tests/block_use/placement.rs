//! Placing a block from an item, and the armor stand item.

use super::*;

#[test]
fn block_item_placement_context_selects_targets_and_places_pillars_slabs_and_stairs() {
    use crate::block_behavior::{
        can_replace_for_placement, state_for_placement, BlockPlacementContext,
    };
    use bbb_protocol::block_types::VanillaGetStateForPlacementOverride;
    use bbb_protocol::item_types::{VanillaBlockItemKind, VANILLA_ITEM_TYPES};

    let air = block_state_id("minecraft:air");
    let mut counts = [0; 3];
    for item in VANILLA_ITEM_TYPES {
        let Some(behavior) = item.block_item_behavior else {
            continue;
        };
        if behavior.kind != VanillaBlockItemKind::BlockItem {
            continue;
        }
        let Some(owner) =
            crate::block_behavior::get_state_for_placement_override(behavior.block_type_id)
        else {
            continue;
        };
        let index = match owner {
            VanillaGetStateForPlacementOverride::RotatedPillarBlock => 0,
            VanillaGetStateForPlacementOverride::SlabBlock => 1,
            VanillaGetStateForPlacementOverride::StairBlock => 2,
            _ => continue,
        };
        counts[index] += 1;
        let context = BlockPlacementContext {
            pos: BlockPos { x: 9, y: 70, z: 9 },
            clicked_face: Direction::Up,
            click_x: 0.5,
            click_y: 0.5,
            click_z: 0.5,
            replacing_clicked: false,
            horizontal_direction: Direction::South,
            secondary_use_active: false,
            inside: false,
        };
        assert!(
            state_for_placement(
                item.id,
                context,
                crate::registry_tags::RegistryTagSnapshot::builtin(),
                &mut |_| air
            )
            .is_some(),
            "{}",
            item.resource_id
        );
    }
    assert_eq!(counts, [57, 62, 58]);

    let oak_slab_item =
        bbb_protocol::item_types::vanilla_item_type_for_resource_id("minecraft:oak_slab")
            .expect("oak slab item")
            .id;
    let bottom_slab = placement_state(
        "minecraft:oak_slab",
        &[("type", "bottom"), ("waterlogged", "false")],
    );
    let mut slab_context = BlockPlacementContext {
        pos: BlockPos { x: 9, y: 70, z: 9 },
        clicked_face: Direction::East,
        click_x: 0.5,
        click_y: 0.75,
        click_z: 0.5,
        replacing_clicked: true,
        horizontal_direction: Direction::South,
        secondary_use_active: false,
        inside: false,
    };
    assert_eq!(
        can_replace_for_placement(oak_slab_item, bottom_slab, slab_context),
        Some(true)
    );
    slab_context.click_y = 0.25;
    assert_eq!(
        can_replace_for_placement(oak_slab_item, bottom_slab, slab_context),
        Some(false)
    );
    slab_context.replacing_clicked = false;
    assert_eq!(
        can_replace_for_placement(oak_slab_item, bottom_slab, slab_context),
        Some(true)
    );

    let oak_stair_item =
        bbb_protocol::item_types::vanilla_item_type_for_resource_id("minecraft:oak_stairs")
            .expect("oak stair item")
            .id;
    let stair_pos = BlockPos { x: 9, y: 70, z: 9 };
    let stair_context = BlockPlacementContext {
        pos: stair_pos,
        clicked_face: Direction::Up,
        click_x: 0.5,
        click_y: 0.5,
        click_z: 0.5,
        replacing_clicked: false,
        horizontal_direction: Direction::South,
        secondary_use_active: false,
        inside: false,
    };
    let stair_with_facing = |facing| {
        placement_state(
            "minecraft:oak_stairs",
            &[
                ("facing", facing),
                ("half", "bottom"),
                ("shape", "straight"),
                ("waterlogged", "false"),
            ],
        )
    };
    let outer = state_for_placement(
        oak_stair_item,
        stair_context,
        crate::registry_tags::RegistryTagSnapshot::builtin(),
        &mut |at| {
            if at == stair_pos {
                air
            } else if at == (BlockPos { z: 10, ..stair_pos }) {
                stair_with_facing("east")
            } else if at == (BlockPos { z: 8, ..stair_pos }) {
                stair_with_facing("west")
            } else {
                air
            }
        },
    )
    .expect("outer stair placement");
    assert_eq!(
        crate::block_behavior::string_property(outer, "shape").as_deref(),
        Some("outer_left")
    );
    let inner = state_for_placement(
        oak_stair_item,
        stair_context,
        crate::registry_tags::RegistryTagSnapshot::builtin(),
        &mut |at| {
            if at == (BlockPos { z: 8, ..stair_pos }) {
                stair_with_facing("east")
            } else {
                air
            }
        },
    )
    .expect("inner stair placement");
    assert_eq!(
        crate::block_behavior::string_property(inner, "shape").as_deref(),
        Some("inner_left")
    );

    let (sender, receiver) = mailbox();
    let mut world = ServerWorld::new(creative_fixture_config()).unwrap();
    join_loaded(&sender, &receiver, &mut world, 1, "builder", 1);
    place_player(
        &sender,
        &receiver,
        &mut world,
        1,
        Vec3d {
            x: 6.5,
            y: 65.0,
            z: 6.5,
        },
        Some(PlayerRotation {
            y_rot: 0.0,
            x_rot: 0.0,
        }),
    );

    let replaceable = BlockPos { x: 6, y: 64, z: 6 };
    assert!(world.set_block(
        Java26BuiltinDimension::Overworld,
        replaceable,
        block_state_id("minecraft:short_grass"),
        crate::fluid::SET_BLOCK_UPDATE_ALL,
    ));
    select_placement_item(&sender, &receiver, &mut world, "minecraft:stone");
    place_block_item(
        &sender,
        &receiver,
        &mut world,
        BlockPos { x: 6, y: 63, z: 6 },
        Direction::Up,
        1.0,
        1,
    );
    assert_eq!(
        live_block(&mut world, replaceable),
        Some(block_state_id("minecraft:stone"))
    );

    let pillar_support = BlockPos { x: 7, y: 64, z: 6 };
    assert!(world.set_block(
        Java26BuiltinDimension::Overworld,
        pillar_support,
        block_state_id("minecraft:stone"),
        crate::fluid::SET_BLOCK_UPDATE_ALL,
    ));
    select_placement_item(&sender, &receiver, &mut world, "minecraft:oak_log");
    place_block_item(
        &sender,
        &receiver,
        &mut world,
        pillar_support,
        Direction::East,
        0.5,
        2,
    );
    assert_eq!(
        live_block(&mut world, BlockPos { x: 8, y: 64, z: 6 }),
        Some(placement_state("minecraft:oak_log", &[("axis", "x")]))
    );

    let slab = BlockPos { x: 6, y: 64, z: 8 };
    assert!(world.set_block(
        Java26BuiltinDimension::Overworld,
        slab,
        placement_state("minecraft:water", &[("level", "0")]),
        crate::fluid::SET_BLOCK_UPDATE_ALL,
    ));
    select_placement_item(&sender, &receiver, &mut world, "minecraft:oak_slab");
    place_block_item(
        &sender,
        &receiver,
        &mut world,
        BlockPos { x: 6, y: 63, z: 8 },
        Direction::Up,
        1.0,
        3,
    );
    assert_eq!(
        live_block(&mut world, slab),
        Some(placement_state(
            "minecraft:oak_slab",
            &[("type", "bottom"), ("waterlogged", "true")]
        ))
    );
    place_block_item(&sender, &receiver, &mut world, slab, Direction::Up, 1.0, 4);
    assert_eq!(
        live_block(&mut world, slab),
        Some(placement_state(
            "minecraft:oak_slab",
            &[("type", "double"), ("waterlogged", "false")]
        ))
    );

    let stair = BlockPos { x: 6, y: 64, z: 10 };
    assert!(world.set_block(
        Java26BuiltinDimension::Overworld,
        BlockPos { x: 6, y: 64, z: 11 },
        placement_state(
            "minecraft:oak_stairs",
            &[
                ("facing", "east"),
                ("half", "bottom"),
                ("shape", "straight"),
                ("waterlogged", "false"),
            ],
        ),
        crate::fluid::SET_BLOCK_UPDATE_ALL,
    ));
    select_placement_item(&sender, &receiver, &mut world, "minecraft:oak_stairs");
    place_block_item(
        &sender,
        &receiver,
        &mut world,
        BlockPos { x: 6, y: 63, z: 10 },
        Direction::Up,
        1.0,
        5,
    );
    assert_eq!(
        live_block(&mut world, stair),
        Some(placement_state(
            "minecraft:oak_stairs",
            &[
                ("facing", "south"),
                ("half", "bottom"),
                ("shape", "outer_left"),
                ("waterlogged", "false"),
            ]
        ))
    );

    let candle = BlockPos { x: 4, y: 64, z: 6 };
    assert!(world.set_block(
        Java26BuiltinDimension::Overworld,
        candle,
        placement_state(
            "minecraft:candle",
            &[("candles", "1"), ("lit", "false"), ("waterlogged", "false"),]
        ),
        crate::fluid::SET_BLOCK_UPDATE_ALL,
    ));
    select_placement_item(&sender, &receiver, &mut world, "minecraft:stone");
    place_block_item(
        &sender,
        &receiver,
        &mut world,
        candle,
        Direction::Up,
        1.0,
        6,
    );
    assert_eq!(
        live_block(&mut world, BlockPos { y: 65, ..candle }),
        Some(block_state_id("minecraft:stone"))
    );
    assert_eq!(
        live_block(&mut world, candle),
        Some(placement_state(
            "minecraft:candle",
            &[("candles", "1"), ("lit", "false"), ("waterlogged", "false"),]
        ))
    );

    place_player(
        &sender,
        &receiver,
        &mut world,
        1,
        Vec3d {
            x: 4.5,
            y: 318.0,
            z: 2.5,
        },
        None,
    );
    let top = BlockPos { x: 2, y: 319, z: 2 };
    assert!(world.set_block(
        Java26BuiltinDimension::Overworld,
        BlockPos { y: 318, ..top },
        block_state_id("minecraft:dirt"),
        crate::fluid::SET_BLOCK_UPDATE_ALL,
    ));
    assert!(world.set_block(
        Java26BuiltinDimension::Overworld,
        top,
        block_state_id("minecraft:short_grass"),
        crate::fluid::SET_BLOCK_UPDATE_ALL,
    ));
    place_block_item(&sender, &receiver, &mut world, top, Direction::Up, 1.0, 7);
    assert_eq!(
        live_block(&mut world, top),
        Some(block_state_id("minecraft:stone"))
    );
}

#[test]
fn dirt_path_placement_pushes_live_entities_even_when_obstruction_refuses() {
    use crate::block_behavior::BlockPlacementContext;

    let (sender, receiver) = mailbox();
    let mut world = ServerWorld::new(creative_fixture_config()).unwrap();
    join_loaded(&sender, &receiver, &mut world, 1, "builder", 1);
    let dimension = Java26BuiltinDimension::Overworld;
    let item = bbb_protocol::item_types::vanilla_item_type_for_resource_id("minecraft:dirt_path")
        .expect("built-in dirt-path item")
        .id;
    let dirt = block_state_id("minecraft:dirt");
    let stone = block_state_id("minecraft:stone");
    let context = |pos| BlockPlacementContext {
        pos,
        clicked_face: Direction::Up,
        click_x: 0.5,
        click_y: 0.5,
        click_z: 0.5,
        replacing_clicked: false,
        horizontal_direction: Direction::South,
        secondary_use_active: false,
        inside: false,
    };
    let stand = || {
        crate::entity::EntityData::ArmorStand(
            crate::entity::ArmorStandData::new().expect("armor stand attributes"),
        )
    };

    let accepted = BlockPos { x: 8, y: 70, z: 8 };
    assert!(world.set_block(
        dimension,
        BlockPos { y: 71, ..accepted },
        stone,
        crate::fluid::SET_BLOCK_UPDATE_ALL,
    ));
    let (pushed, _) = world.spawn_stored_entity_for_test(
        dimension,
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_ARMOR_STAND_ID,
        Vec3d {
            x: 8.5,
            y: 70.9375,
            z: 8.5,
        },
        stand(),
    );
    assert_eq!(
        world.placed_block_state(ConnectionId(1), dimension, context(accepted), Some(item)),
        Some(dirt),
    );
    assert_eq!(
        world
            .stored_entity_for_test(dimension, pushed)
            .unwrap()
            .position()
            .y,
        71.0,
    );

    let refused = BlockPos { x: 12, y: 70, z: 8 };
    assert!(world.set_block(
        dimension,
        BlockPos { y: 71, ..refused },
        stone,
        crate::fluid::SET_BLOCK_UPDATE_ALL,
    ));
    let (pushed_before_refusal, _) = world.spawn_stored_entity_for_test(
        dimension,
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_ARMOR_STAND_ID,
        Vec3d {
            x: 12.5,
            y: 70.9375,
            z: 8.5,
        },
        stand(),
    );
    world.spawn_stored_entity_for_test(
        dimension,
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_ARMOR_STAND_ID,
        Vec3d {
            x: 12.5,
            y: 69.0,
            z: 8.5,
        },
        stand(),
    );
    assert_eq!(
        world.placed_block_state(ConnectionId(1), dimension, context(refused), Some(item)),
        None,
    );
    assert_eq!(
        world
            .stored_entity_for_test(dimension, pushed_before_refusal)
            .unwrap()
            .position()
            .y,
        71.0,
    );
}

#[test]
fn pitcher_plant_placement_preflights_and_writes_both_halves() {
    use crate::block_behavior::BlockPlacementContext;

    let (sender, receiver) = mailbox();
    let mut world = ServerWorld::new(creative_fixture_config()).unwrap();
    join_loaded(&sender, &receiver, &mut world, 1, "builder", 1);
    let dimension = Java26BuiltinDimension::Overworld;
    place_player(
        &sender,
        &receiver,
        &mut world,
        1,
        Vec3d {
            x: 20.5,
            y: 70.0,
            z: 8.5,
        },
        Some(PlayerRotation {
            y_rot: 0.0,
            x_rot: 0.0,
        }),
    );
    select_placement_item(&sender, &receiver, &mut world, "minecraft:pitcher_plant");
    let item =
        bbb_protocol::item_types::vanilla_item_type_for_resource_id("minecraft:pitcher_plant")
            .expect("pitcher plant item")
            .id;
    let air = block_state_id("minecraft:air");
    let dirt = block_state_id("minecraft:dirt");
    let lower = placement_state("minecraft:pitcher_plant", &[("half", "lower")]);
    let upper = placement_state("minecraft:pitcher_plant", &[("half", "upper")]);

    let support = BlockPos { x: 20, y: 69, z: 8 };
    let lower_pos = BlockPos { y: 70, ..support };
    let upper_pos = BlockPos { y: 71, ..support };
    assert!(world.set_block(dimension, support, dirt, crate::fluid::SET_BLOCK_UPDATE_ALL,));
    world.set_block(
        dimension,
        lower_pos,
        air,
        crate::fluid::SET_BLOCK_UPDATE_ALL,
    );
    world.set_block(
        dimension,
        upper_pos,
        air,
        crate::fluid::SET_BLOCK_UPDATE_ALL,
    );
    place_block_item(
        &sender,
        &receiver,
        &mut world,
        support,
        Direction::Up,
        1.0,
        9,
    );
    assert_eq!(live_block(&mut world, lower_pos), Some(lower));
    assert_eq!(live_block(&mut world, upper_pos), Some(upper));

    let blocked = BlockPos { x: 22, y: 70, z: 8 };
    assert!(world.set_block(
        dimension,
        BlockPos { y: 69, ..blocked },
        dirt,
        crate::fluid::SET_BLOCK_UPDATE_ALL,
    ));
    assert!(world.set_block(
        dimension,
        BlockPos { y: 71, ..blocked },
        block_state_id("minecraft:stone"),
        crate::fluid::SET_BLOCK_UPDATE_ALL,
    ));
    let context = |pos| BlockPlacementContext {
        pos,
        clicked_face: Direction::Up,
        click_x: 0.5,
        click_y: 1.0,
        click_z: 0.5,
        replacing_clicked: false,
        horizontal_direction: Direction::South,
        secondary_use_active: false,
        inside: false,
    };
    assert_eq!(
        world.placed_block_state(ConnectionId(1), dimension, context(blocked), Some(item)),
        None,
    );

    let top = BlockPos {
        x: 24,
        y: 319,
        z: 8,
    };
    assert!(world.set_block(
        dimension,
        BlockPos { y: 318, ..top },
        dirt,
        crate::fluid::SET_BLOCK_UPDATE_ALL,
    ));
    assert_eq!(
        world.placed_block_state(ConnectionId(1), dimension, context(top), Some(item)),
        None,
        "the out-of-height upper cell is rejected before any write",
    );
}

#[test]
fn command_block_placement_uses_live_game_master_permission_and_view() {
    use crate::block_behavior::BlockPlacementContext;

    let (sender, receiver) = mailbox();
    let mut world = ServerWorld::new(operator_config(&[("builder", 1)])).unwrap();
    join_loaded(&sender, &receiver, &mut world, 1, "builder", 1);
    select_placement_item(&sender, &receiver, &mut world, "minecraft:command_block");
    place_player(
        &sender,
        &receiver,
        &mut world,
        1,
        Vec3d {
            x: 20.5,
            y: 70.0,
            z: 8.5,
        },
        Some(PlayerRotation {
            y_rot: 0.0,
            x_rot: 0.0,
        }),
    );
    let dimension = Java26BuiltinDimension::Overworld;
    let support = BlockPos {
        x: 20,
        y: 69,
        z: 10,
    };
    let target = BlockPos { y: 70, ..support };
    assert!(world.set_block(
        dimension,
        support,
        block_state_id("minecraft:stone"),
        crate::fluid::SET_BLOCK_UPDATE_ALL,
    ));
    place_block_item(
        &sender,
        &receiver,
        &mut world,
        support,
        Direction::Up,
        1.0,
        10,
    );
    assert_eq!(
        live_block(&mut world, target),
        Some(placement_state(
            "minecraft:command_block",
            &[("conditional", "false"), ("facing", "north")],
        )),
    );

    let item =
        bbb_protocol::item_types::vanilla_item_type_for_resource_id("minecraft:command_block")
            .unwrap()
            .id;
    for (x_rot, y_rot, facing) in [(-90.0, 0.0, "down"), (0.0, 90.0, "east")] {
        place_player(
            &sender,
            &receiver,
            &mut world,
            1,
            Vec3d {
                x: 20.5,
                y: 70.0,
                z: 8.5,
            },
            Some(PlayerRotation { x_rot, y_rot }),
        );
        assert_eq!(
            world.placed_block_state(
                ConnectionId(1),
                dimension,
                BlockPlacementContext::from(target),
                Some(item)
            ),
            Some(placement_state(
                "minecraft:command_block",
                &[("conditional", "false"), ("facing", facing)]
            ))
        );
    }

    let (sender, receiver) = mailbox();
    let mut world = ServerWorld::new(creative_fixture_config()).unwrap();
    join_loaded(&sender, &receiver, &mut world, 1, "builder", 1);
    let item =
        bbb_protocol::item_types::vanilla_item_type_for_resource_id("minecraft:command_block")
            .expect("command block item")
            .id;
    assert_eq!(
        world.placed_block_state(
            ConnectionId(1),
            dimension,
            BlockPlacementContext {
                pos: target,
                clicked_face: Direction::Up,
                click_x: 0.5,
                click_y: 1.0,
                click_z: 0.5,
                replacing_clicked: false,
                horizontal_direction: Direction::South,
                secondary_use_active: false,
                inside: false,
            },
            Some(item),
        ),
        None,
        "creative without gamemaster permission cannot place the item",
    );
}

#[test]
fn lectern_placement_uses_live_permission_data_and_initializes_its_block_entity() {
    use crate::block_behavior::BlockPlacementContext;

    let (sender, receiver) = mailbox();
    let mut world = ServerWorld::new(operator_config(&[("builder", 1)])).unwrap();
    join_loaded(&sender, &receiver, &mut world, 1, "builder", 1);
    let dimension = Java26BuiltinDimension::Overworld;
    let context = BlockPlacementContext {
        pos: BlockPos { x: 20, y: 70, z: 8 },
        clicked_face: Direction::Up,
        click_x: 0.5,
        click_y: 1.0,
        click_z: 0.5,
        replacing_clicked: false,
        horizontal_direction: Direction::South,
        secondary_use_active: false,
        inside: false,
    };
    let mut book = lectern_book_tag();
    let NbtSummaryValue::Compound(entries) = &mut book else {
        unreachable!()
    };
    entries.push(nbt(
        "components",
        NbtSummaryValue::Compound(vec![nbt(
            "minecraft:written_book_content",
            NbtSummaryValue::Compound(vec![
                nbt(
                    "title",
                    NbtSummaryValue::Compound(vec![nbt(
                        "raw",
                        NbtSummaryValue::string("Placement book"),
                    )]),
                ),
                nbt("author", NbtSummaryValue::string("builder")),
                nbt("pages", NbtSummaryValue::List(vec![])),
            ]),
        )]),
    ));
    let item = lectern_item(
        bbb_protocol::block_entity_types::VANILLA_BLOCK_ENTITY_TYPE_LECTERN_ID,
        vec![nbt("Book", book.clone())],
    );
    let with_book = placement_state(
        "minecraft:lectern",
        &[
            ("facing", "north"),
            ("has_book", "true"),
            ("powered", "false"),
        ],
    );
    assert_eq!(
        world.placed_block_state_with_item(ConnectionId(1), dimension, context, &item,),
        Some(with_book),
    );

    assert_eq!(
        world.placed_block_state_with_item(
            ConnectionId(1),
            dimension,
            BlockPlacementContext {
                horizontal_direction: Direction::East,
                ..context
            },
            &item
        ),
        Some(placement_state(
            "minecraft:lectern",
            &[
                ("facing", "west"),
                ("has_book", "true"),
                ("powered", "false")
            ]
        ))
    );
    let mut unprivileged = ServerWorld::new(creative_fixture_config()).unwrap();
    let (unprivileged_sender, unprivileged_receiver) = mailbox();
    join_loaded(
        &unprivileged_sender,
        &unprivileged_receiver,
        &mut unprivileged,
        1,
        "builder",
        1,
    );
    assert_eq!(
        unprivileged.placed_block_state_with_item(ConnectionId(1), dimension, context, &item),
        Some(placement_state(
            "minecraft:lectern",
            &[
                ("facing", "north"),
                ("has_book", "false"),
                ("powered", "false")
            ]
        ))
    );

    let without_book = lectern_item(
        bbb_protocol::block_entity_types::VANILLA_BLOCK_ENTITY_TYPE_LECTERN_ID,
        Vec::new(),
    );
    assert_eq!(
        world.placed_block_state_with_item(ConnectionId(1), dimension, context, &without_book,),
        Some(placement_state(
            "minecraft:lectern",
            &[
                ("facing", "north"),
                ("has_book", "false"),
                ("powered", "false"),
            ],
        )),
    );

    play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::SetCreativeModeSlot(SetCreativeModeSlot { slot_num: 36, item }),
    );
    place_player(
        &sender,
        &receiver,
        &mut world,
        1,
        Vec3d {
            x: 20.5,
            y: 70.0,
            z: 6.5,
        },
        Some(PlayerRotation {
            y_rot: 0.0,
            x_rot: 0.0,
        }),
    );
    let support = BlockPos { x: 20, y: 69, z: 8 };
    assert!(world.set_block(
        dimension,
        support,
        block_state_id("minecraft:stone"),
        crate::fluid::SET_BLOCK_UPDATE_ALL,
    ));
    place_block_item(
        &sender,
        &receiver,
        &mut world,
        support,
        Direction::Up,
        1.0,
        11,
    );
    assert_eq!(live_block(&mut world, context.pos), Some(with_book));
    let lectern = world
        .level_block_entity_for_test(dimension, context.pos)
        .and_then(|block_entity| block_entity.lectern())
        .expect("placed lectern block entity");
    assert!(lectern.has_book().expect("the fixture operation succeeds"));
    assert_eq!(
        lectern.book().item_id(),
        Some(bbb_protocol::item_types::VANILLA_ITEM_WRITTEN_BOOK_ID),
    );
}

/// `LecternBlock.getStateForPlacement` sets `HAS_BOOK` from the component's own
/// key, so a `Book` the record read cannot carry still places a lectern.
#[test]
fn a_lectern_whose_book_cannot_be_read_places_without_a_book() {
    use crate::block_behavior::BlockPlacementContext;

    let (sender, receiver) = mailbox();
    let mut world = ServerWorld::new(operator_config(&[("builder", 1)])).unwrap();
    join_loaded(&sender, &receiver, &mut world, 1, "builder", 1);
    let dimension = Java26BuiltinDimension::Overworld;
    let context = BlockPlacementContext {
        pos: BlockPos { x: 20, y: 70, z: 8 },
        clicked_face: Direction::Up,
        click_x: 0.5,
        click_y: 1.0,
        click_z: 0.5,
        replacing_clicked: false,
        horizontal_direction: Direction::South,
        secondary_use_active: false,
        inside: false,
    };
    let malformed = lectern_item(
        bbb_protocol::block_entity_types::VANILLA_BLOCK_ENTITY_TYPE_LECTERN_ID,
        vec![nbt("Book", NbtSummaryValue::Int(1))],
    );
    assert_eq!(
        world.placed_block_state_with_item(ConnectionId(1), dimension, context, &malformed),
        Some(placement_state(
            "minecraft:lectern",
            &[
                ("facing", "north"),
                ("has_book", "true"),
                ("powered", "false"),
            ],
        )),
    );

    play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::SetCreativeModeSlot(SetCreativeModeSlot {
            slot_num: 36,
            item: malformed,
        }),
    );
    place_player(
        &sender,
        &receiver,
        &mut world,
        1,
        Vec3d {
            x: 20.5,
            y: 70.0,
            z: 6.5,
        },
        Some(PlayerRotation {
            y_rot: 0.0,
            x_rot: 0.0,
        }),
    );
    let support = BlockPos { x: 20, y: 69, z: 8 };
    assert!(world.set_block(
        dimension,
        support,
        block_state_id("minecraft:stone"),
        crate::fluid::SET_BLOCK_UPDATE_ALL,
    ));
    place_block_item(
        &sender,
        &receiver,
        &mut world,
        support,
        Direction::Up,
        1.0,
        11,
    );

    assert_eq!(
        live_block(&mut world, context.pos),
        Some(placement_state(
            "minecraft:lectern",
            &[
                ("facing", "north"),
                ("has_book", "true"),
                ("powered", "false"),
            ],
        )),
        "vanilla places the lectern with the property its component set",
    );
    let lectern = world
        .level_block_entity_for_test(dimension, context.pos)
        .and_then(|block_entity| block_entity.lectern())
        .expect("placed lectern block entity");
    assert!(lectern.book().item_id().is_none());
}

#[test]
fn standing_and_wall_block_item_uses_live_player_view_and_consumes() {
    let (sender, receiver) = mailbox();
    let mut config = operator_config(&[("builder", 1)]);
    config.force_game_mode = true;
    let mut world = ServerWorld::new(config).unwrap();
    join_loaded(&sender, &receiver, &mut world, 1, "builder", 1);
    select_placement_item(&sender, &receiver, &mut world, "minecraft:torch");
    chat_command(
        &sender,
        &receiver,
        &mut world,
        1,
        "defaultgamemode survival",
    );
    place_player(
        &sender,
        &receiver,
        &mut world,
        1,
        Vec3d {
            x: 6.5,
            y: 66.0,
            z: 6.5,
        },
        Some(PlayerRotation {
            y_rot: 0.0,
            x_rot: 90.0,
        }),
    );

    let target = BlockPos { x: 6, y: 65, z: 6 };
    assert!(world.set_block(
        Java26BuiltinDimension::Overworld,
        BlockPos { y: 64, ..target },
        block_state_id("minecraft:stone"),
        crate::fluid::SET_BLOCK_UPDATE_ALL,
    ));
    assert!(world.set_block(
        Java26BuiltinDimension::Overworld,
        target,
        block_state_id("minecraft:short_grass"),
        crate::fluid::SET_BLOCK_UPDATE_ALL,
    ));
    let placed = place_block_item(
        &sender,
        &receiver,
        &mut world,
        target,
        Direction::Up,
        0.5,
        8,
    );
    assert_eq!(
        live_block(&mut world, target),
        Some(block_state_id("minecraft:torch"))
    );
    assert_eq!(placed.snapshot.players[0].inventory.slots[0].count, 0);
    // `ItemStack.useOn` (`ItemStack.java:367-369`) counts the placement.
    assert_eq!(
        world.player_stat_for_test(
            ConnectionId(1),
            bbb_server_types::stats::StatKey::new(
                bbb_protocol::stat_types::VANILLA_STAT_TYPE_ITEM_USED_ID,
                item_id("minecraft:torch"),
            ),
        ),
        1
    );
}

#[test]
fn armor_stand_item_uses_the_replaceable_target_angle_sound_event_consume_and_stat() {
    // `ArmorStandItem.useOn` uses `BlockPlaceContext.getClickedPos`, then adds
    // and reports the stand in this order (`ArmorStandItem.java:33-55`).
    let (sender, receiver) = mailbox();
    let mut config = operator_config(&[("alice", 1)]);
    config.force_game_mode = true;
    let mut world = spawn_pinned_world(config);
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::SetCreativeModeSlot(SetCreativeModeSlot {
            slot_num: 36,
            item: armor_stand_item(2),
        }),
    );
    chat_command(
        &sender,
        &receiver,
        &mut world,
        1,
        "defaultgamemode survival",
    );
    play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::MovePlayer(MovePlayer {
            position: None,
            rotation: Some(PlayerRotation {
                y_rot: 135.0,
                x_rot: 0.0,
            }),
            on_ground: true,
            horizontal_collision: false,
        }),
    );
    let below = BlockPos { x: 3, y: 63, z: 0 };
    let clicked = BlockPos { x: 3, y: 64, z: 0 };
    assert!(world.set_block(
        Java26BuiltinDimension::Overworld,
        below,
        block_state_id("minecraft:dirt"),
        crate::fluid::SET_BLOCK_UPDATE_ALL,
    ));
    assert!(world.set_block(
        Java26BuiltinDimension::Overworld,
        clicked,
        block_state_id("minecraft:short_grass"),
        crate::fluid::SET_BLOCK_UPDATE_ALL,
    ));

    let placed = play(
        &sender,
        &receiver,
        &mut world,
        1,
        armor_stand_use(clicked, Direction::Up, 1),
    );
    let adds = armor_stand_adds(&placed);
    assert_eq!(adds.len(), 1, "{placed:?}");
    assert_eq!(
        adds[0].position,
        Vec3d {
            x: 3.5,
            y: 64.0,
            z: 0.5
        }
    );
    assert_eq!(adds[0].y_rot, -45.0);
    assert_eq!(placed.snapshot.players[0].inventory.slots[0].count, 1);
    assert!(placed.events.iter().any(|event| matches!(
        event,
        WorldEvent::Send {
            packet: ClientboundPlayPacket::Sound(sound),
            ..
        } if sound.source == SoundSource::Blocks
            && sound.position == adds[0].position
            && sound.volume == 0.75
            && sound.pitch == 0.8
            && matches!(sound.sound, SoundEventHolder::Reference { registry_id }
                if vanilla_sound_event_resource_id(registry_id)
                    == Some("minecraft:entity.armor_stand.place"))
    )));
    let (game_events, _) = world.game_events(Java26BuiltinDimension::Overworld);
    assert!(game_events.iter().any(|event| {
        event.game_event == crate::level_event::VanillaGameEvent::ENTITY_PLACE
            && event.source_entity_id == Some(entity_id_of(&world, 1))
    }));

    let stats = play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::ClientCommand(ClientCommandAction::RequestStats),
    );
    assert!(stats.events.iter().any(|event| matches!(
        event,
        WorldEvent::Send {
            packet: ClientboundPlayPacket::AwardStats(update),
            ..
        } if update.stats.iter().any(|entry|
            entry.stat_type_id == bbb_protocol::stat_types::VANILLA_STAT_TYPE_ITEM_USED_ID
                && entry.value_id == bbb_protocol::item_types::VANILLA_ITEM_ARMOR_STAND_ID
                && entry.amount == 1)
    )));
}

#[test]
fn armor_stand_item_uses_the_relative_target_and_adjusts_to_its_support() {
    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_world(creative_fixture_config());
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::SetCreativeModeSlot(SetCreativeModeSlot {
            slot_num: 36,
            item: armor_stand_item(1),
        }),
    );
    let clicked = BlockPos { x: 2, y: 63, z: 0 };
    let slab = bbb_protocol::block_state_types::vanilla_block_state_id_by_name_and_properties(
        "minecraft:oak_slab",
        &BTreeMap::from([
            ("type".to_string(), "bottom".to_string()),
            ("waterlogged".to_string(), "false".to_string()),
        ]),
    )
    .expect("the bottom oak slab state exists");
    assert!(world.set_block(
        Java26BuiltinDimension::Overworld,
        clicked,
        slab,
        crate::fluid::SET_BLOCK_UPDATE_ALL,
    ));
    let placed = play(
        &sender,
        &receiver,
        &mut world,
        1,
        armor_stand_use(clicked, Direction::Up, 1),
    );
    let adds = armor_stand_adds(&placed);
    assert_eq!(adds.len(), 1, "{placed:?}");
    assert_eq!(
        adds[0].position,
        Vec3d {
            x: 2.5,
            y: 63.5,
            z: 0.5,
        }
    );
    assert_eq!(placed.snapshot.players[0].inventory.slots[0].count, 1);
}

/// `SnowLayerBlock.canBeReplaced` (`SnowLayerBlock.java:113-120`) yields only
/// a single layer to a foreign item, so a stand clicked onto two layers goes
/// above them and settles on their collision top.
#[test]
fn armor_stand_item_keeps_a_deep_snow_layer() {
    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_world(creative_fixture_config());
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::SetCreativeModeSlot(SetCreativeModeSlot {
            slot_num: 36,
            item: armor_stand_item(1),
        }),
    );
    let snow = BlockPos { x: 2, y: 64, z: 0 };
    assert!(world.set_block(
        Java26BuiltinDimension::Overworld,
        snow,
        placement_state("minecraft:snow", &[("layers", "2")]),
        crate::fluid::SET_BLOCK_UPDATE_ALL,
    ));
    let placed = play(
        &sender,
        &receiver,
        &mut world,
        1,
        armor_stand_use(snow, Direction::Up, 1),
    );
    let adds = armor_stand_adds(&placed);
    assert_eq!(adds.len(), 1, "{placed:?}");
    assert_eq!(
        adds[0].position,
        Vec3d {
            x: 2.5,
            y: 64.125,
            z: 0.5,
        }
    );
}

#[test]
fn a_spectator_inside_the_target_box_does_not_block_armor_stand_placement() {
    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_world(operator_config(&[("alice", 1)]));
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    join_loaded(&sender, &receiver, &mut world, 2, "bob", 2);
    place_player(
        &sender,
        &receiver,
        &mut world,
        2,
        Vec3d {
            x: 3.5,
            y: 64.0,
            z: 0.5,
        },
        None,
    );
    chat_command(&sender, &receiver, &mut world, 1, "gamemode spectator bob");
    play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::SetCreativeModeSlot(SetCreativeModeSlot {
            slot_num: 36,
            item: armor_stand_item(1),
        }),
    );

    let placed = play(
        &sender,
        &receiver,
        &mut world,
        1,
        armor_stand_use(BlockPos { x: 3, y: 63, z: 0 }, Direction::Up, 1),
    );
    let adds = armor_stand_adds(&placed);
    assert_eq!(
        adds.len(),
        2,
        "the two viewers each receive the add: {placed:?}"
    );
    assert!(adds.iter().all(|add| add.position
        == Vec3d {
            x: 3.5,
            y: 64.0,
            z: 0.5,
        }));
}

#[test]
fn armor_stand_item_refuses_down_and_collisions() {
    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_world(creative_fixture_config());
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::SetCreativeModeSlot(SetCreativeModeSlot {
            slot_num: 36,
            item: armor_stand_item(1),
        }),
    );
    let floor = BlockPos { x: 3, y: 63, z: 0 };
    let down = play(
        &sender,
        &receiver,
        &mut world,
        1,
        armor_stand_use(floor, Direction::Down, 1),
    );
    assert!(armor_stand_adds(&down).is_empty());
    assert_eq!(down.snapshot.players[0].inventory.slots[0].count, 1);

    let player_collision = play(
        &sender,
        &receiver,
        &mut world,
        1,
        armor_stand_use(BlockPos { x: 0, y: 63, z: 0 }, Direction::Up, 2),
    );
    assert!(armor_stand_adds(&player_collision).is_empty());
    assert_eq!(
        player_collision.snapshot.players[0].inventory.slots[0].count,
        1
    );

    let occupied = BlockPos { x: 3, y: 64, z: 0 };
    assert!(world.set_block(
        Java26BuiltinDimension::Overworld,
        occupied,
        block_state_id("minecraft:stone"),
        crate::fluid::SET_BLOCK_UPDATE_ALL,
    ));
    let block_collision = play(
        &sender,
        &receiver,
        &mut world,
        1,
        armor_stand_use(floor, Direction::Up, 3),
    );
    assert!(armor_stand_adds(&block_collision).is_empty());
    assert_eq!(
        block_collision.snapshot.players[0].inventory.slots[0].count,
        1
    );
    assert!(world.set_block(
        Java26BuiltinDimension::Overworld,
        occupied,
        world.fixture().air_state_id(),
        crate::fluid::SET_BLOCK_UPDATE_ALL,
    ));

    let (blocker_id, _) = world.spawn_stored_entity_for_test(
        Java26BuiltinDimension::Overworld,
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_ARMOR_STAND_ID,
        Vec3d {
            x: 3.5,
            y: 64.0,
            z: 0.5,
        },
        crate::entity::EntityData::ArmorStand(
            crate::entity::ArmorStandData::new().expect("armor stand attributes"),
        ),
    );
    world.tick(receiver.drain());
    let blocked = play(
        &sender,
        &receiver,
        &mut world,
        1,
        armor_stand_use(floor, Direction::Up, 4),
    );
    assert!(armor_stand_adds(&blocked).is_empty());
    assert_eq!(blocked.snapshot.players[0].inventory.slots[0].count, 1);
    assert!(world
        .stored_entity_for_test(Java26BuiltinDimension::Overworld, blocker_id)
        .is_some());
}

#[test]
fn placement_brightness_respects_the_level_skylight_fact() {
    let pos = BlockPos { x: 0, y: 64, z: 0 };
    for (has_skylight, expected) in [(false, 0), (true, 15)] {
        let mut chunks = crate::chunk::ChunkStore::new_with_biome_registry_and_skylight(
            default_overworld_level_source().unwrap(),
            0,
            bbb_server_types::chunk_data::VANILLA_26_1_BIOME_REGISTRY_LEN,
            has_skylight,
        );
        assert_eq!(
            ServerWorld::placement_raw_brightness_for_test(&chunks, pos, 0),
            0
        );
        assert!(!chunks.is_loaded(0, 0));
        chunks.try_ensure_served_full_column(0, 0).unwrap();
        assert_eq!(
            ServerWorld::placement_raw_brightness_for_test(&chunks, pos, 0),
            expected
        );
        assert_eq!(
            ServerWorld::placement_raw_brightness_for_test(&chunks, pos, 7),
            if has_skylight { 8 } else { 0 }
        );
        assert_eq!(
            ServerWorld::placement_raw_brightness_for_test(&chunks, BlockPos { y: 63, ..pos }, 0),
            0
        );
        assert!(chunks.is_loaded(0, 0));
        assert!(!chunks.is_loaded(1, 0));
        assert_eq!(
            ServerWorld::placement_raw_brightness_for_test(&chunks, BlockPos { x: 16, ..pos }, 0),
            0
        );
    }
}

/// `UseOnContext.getHorizontalDirection` (`UseOnContext.java:70-72`) end to end
/// off a live player: `Entity.getDirection` is `Direction.fromYRot`
/// (`Entity.java:3350-3352`, `Direction.java:291-293`) over the horizontal
/// order `BY_2D_DATA` gives (`Direction.java:61-64`), and
/// `DiodeBlock.getStateForPlacement` (`DiodeBlock.java:154-156`) faces the
/// repeater back at whoever placed it.
///
/// Four quadrants, because one rotation cannot tell a real read of the player's
/// yaw from a constant.
#[test]
fn a_repeater_placed_from_the_hand_faces_back_at_the_player() {
    for (y_rot, facing) in [
        (0.0_f32, "north"),
        (90.0, "east"),
        (180.0, "south"),
        (270.0, "west"),
    ] {
        let (sender, receiver) = mailbox();
        let mut config = operator_config(&[("builder", 1)]);
        config.force_game_mode = true;
        let mut world = ServerWorld::new(config).unwrap();
        join_loaded(&sender, &receiver, &mut world, 1, "builder", 1);
        select_placement_item(&sender, &receiver, &mut world, "minecraft:repeater");
        chat_command(
            &sender,
            &receiver,
            &mut world,
            1,
            "defaultgamemode survival",
        );
        place_player(
            &sender,
            &receiver,
            &mut world,
            1,
            Vec3d {
                x: 6.5,
                y: 66.0,
                z: 6.5,
            },
            Some(PlayerRotation { y_rot, x_rot: 0.0 }),
        );
        let support = BlockPos { x: 6, y: 64, z: 6 };
        assert!(world.set_block(
            Java26BuiltinDimension::Overworld,
            support,
            block_state_id("minecraft:stone"),
            crate::fluid::SET_BLOCK_UPDATE_ALL,
        ));

        let placed = place_block_item(
            &sender,
            &receiver,
            &mut world,
            support,
            Direction::Up,
            0.5,
            8,
        );

        let target = BlockPos { x: 6, y: 65, z: 6 };
        let state = live_block(&mut world, target).expect("the repeater is placed");
        assert_eq!(
            bbb_protocol::block_state_types::vanilla_block_state_identity(state)
                .map(|identity| identity.name.as_str()),
            Some("minecraft:repeater"),
            "y_rot {y_rot}"
        );
        assert_eq!(
            crate::block_behavior::string_property(state, "facing"),
            Some(facing.to_string()),
            "y_rot {y_rot}"
        );
        assert_eq!(placed.snapshot.players[0].inventory.slots[0].count, 0);
    }
}

/// `BlockPlaceContext.getNearestLookingDirections` (`BlockPlaceContext.java:71-89`)
/// end to end: `WallTorchBlock.getStateForPlacement` (`WallTorchBlock.java:63-79`)
/// takes the first horizontal face in that order whose torch can survive, so
/// the order decides which of two walls the torch attaches to.
///
/// The player looks west while clicking a wall to the north, and the target has
/// a wall on both sides. The `:85-86` rotate puts the clicked face's opposite
/// ahead of where the player is looking, so the torch faces south off the north
/// wall; reading the view order alone would attach it to the west one.
#[test]
fn a_wall_torch_attaches_to_the_face_that_was_clicked_not_the_one_looked_at() {
    let (sender, receiver) = mailbox();
    let mut config = operator_config(&[("builder", 1)]);
    config.force_game_mode = true;
    let mut world = ServerWorld::new(config).unwrap();
    join_loaded(&sender, &receiver, &mut world, 1, "builder", 1);
    select_placement_item(&sender, &receiver, &mut world, "minecraft:torch");
    chat_command(
        &sender,
        &receiver,
        &mut world,
        1,
        "defaultgamemode survival",
    );
    place_player(
        &sender,
        &receiver,
        &mut world,
        1,
        Vec3d {
            x: 6.5,
            y: 66.0,
            z: 8.5,
        },
        Some(PlayerRotation {
            y_rot: 90.0,
            x_rot: 0.0,
        }),
    );

    let clicked = BlockPos { x: 6, y: 65, z: 6 };
    let target = BlockPos { x: 6, y: 65, z: 7 };
    for pos in [clicked, BlockPos { x: 5, ..target }] {
        assert!(world.set_block(
            Java26BuiltinDimension::Overworld,
            pos,
            block_state_id("minecraft:stone"),
            crate::fluid::SET_BLOCK_UPDATE_ALL,
        ));
    }

    let _ = place_block_item(
        &sender,
        &receiver,
        &mut world,
        clicked,
        Direction::South,
        0.5,
        8,
    );

    let state = live_block(&mut world, target).expect("the torch is placed");
    assert_eq!(
        bbb_protocol::block_state_types::vanilla_block_state_identity(state)
            .map(|identity| identity.name.as_str()),
        Some("minecraft:wall_torch")
    );
    assert_eq!(
        crate::block_behavior::string_property(state, "facing"),
        Some("south".to_string())
    );
}

/// A block item placed with the cursor at a chosen point on the clicked face,
/// which `DoorBlock.getHinge` (`DoorBlock.java:185-190`) reads.
fn place_item_at_cursor(
    sender: &WorldCommandSender,
    receiver: &WorldCommandReceiver,
    world: &mut ServerWorld,
    hit: BlockPos,
    cursor_x: f32,
    cursor_z: f32,
) -> TickResult {
    play(
        sender,
        receiver,
        world,
        1,
        PlayServerbound::UseItemOn(UseItemOn {
            hand: InteractionHand::MainHand,
            hit: BlockHitResult {
                pos: hit,
                direction: Direction::Up,
                cursor_x,
                cursor_y: 1.0,
                cursor_z,
                inside: false,
                world_border_hit: false,
            },
            sequence: 8,
        }),
    )
}

/// A two-cell placement scene: a survival builder holding one `item`,
/// standing clear of the two cells it will occupy, with a `support` block
/// under them.
fn double_high_scene(
    item: &str,
    support_block: &str,
    y_rot: f32,
) -> (
    WorldCommandSender,
    WorldCommandReceiver,
    ServerWorld,
    BlockPos,
) {
    let (sender, receiver) = mailbox();
    let mut config = operator_config(&[("builder", 1)]);
    config.force_game_mode = true;
    let mut world = ServerWorld::new(config).unwrap();
    join_loaded(&sender, &receiver, &mut world, 1, "builder", 1);
    select_placement_item(&sender, &receiver, &mut world, item);
    chat_command(
        &sender,
        &receiver,
        &mut world,
        1,
        "defaultgamemode survival",
    );
    place_player(
        &sender,
        &receiver,
        &mut world,
        1,
        Vec3d {
            x: 6.5,
            y: 66.0,
            z: 8.5,
        },
        Some(PlayerRotation { y_rot, x_rot: 0.0 }),
    );
    let support = BlockPos { x: 6, y: 64, z: 6 };
    assert!(world.set_block(
        Java26BuiltinDimension::Overworld,
        support,
        block_state_id(support_block),
        crate::fluid::SET_BLOCK_UPDATE_ALL,
    ));
    (sender, receiver, world, support)
}

fn door_property(world: &mut ServerWorld, pos: BlockPos, property: &str) -> Option<String> {
    let state = live_block(world, pos)?;
    assert_eq!(
        bbb_protocol::block_state_types::vanilla_block_state_identity(state)
            .map(|identity| identity.name.as_str()),
        Some("minecraft:oak_door"),
        "{pos:?}"
    );
    crate::block_behavior::string_property(state, property)
}

/// `DoorBlock.getStateForPlacement` (`DoorBlock.java:139-153`) and
/// `setPlacedBy` (`:156-158`) end to end.
///
/// `FACING` is `getHorizontalDirection` unchanged (`:145`), so a door faces
/// the way the player faces rather than back at them the way a repeater does.
#[test]
fn a_door_placed_from_the_hand_writes_both_halves_facing_the_player() {
    for (y_rot, facing) in [
        (0.0_f32, "south"),
        (90.0, "west"),
        (180.0, "north"),
        (270.0, "east"),
    ] {
        let (sender, receiver, mut world, support) =
            double_high_scene("minecraft:oak_door", "minecraft:stone", y_rot);
        let placed = place_item_at_cursor(&sender, &receiver, &mut world, support, 0.5, 0.5);

        let lower = BlockPos { x: 6, y: 65, z: 6 };
        let upper = BlockPos { x: 6, y: 66, z: 6 };
        for (pos, half) in [(lower, "lower"), (upper, "upper")] {
            assert_eq!(
                door_property(&mut world, pos, "half"),
                Some(half.to_string()),
                "y_rot {y_rot}"
            );
            assert_eq!(
                door_property(&mut world, pos, "facing"),
                Some(facing.to_string()),
                "y_rot {y_rot}"
            );
            assert_eq!(
                door_property(&mut world, pos, "powered"),
                Some("false".to_string()),
                "y_rot {y_rot}"
            );
            assert_eq!(
                door_property(&mut world, pos, "open"),
                Some("false".to_string()),
                "y_rot {y_rot}"
            );
        }
        assert_eq!(placed.snapshot.players[0].inventory.slots[0].count, 0);
    }
}

/// `DoorBlock.getHinge`'s `solidBlockBalance` (`DoorBlock.java:175-181`): a
/// full block on the clockwise side of the placement direction leans the sum
/// positive, which is the one way out of the first `if` and answers `RIGHT`
/// without the click position being read at all.
///
/// The player faces south, so the clockwise side is west. The same click that
/// answers `LEFT` with both columns empty answers `RIGHT` with one stone in
/// the west column.
#[test]
fn a_door_hinges_away_from_a_full_block_on_the_clockwise_side() {
    for (west_column, hinge) in [(false, "left"), (true, "right")] {
        let (sender, receiver, mut world, support) =
            double_high_scene("minecraft:oak_door", "minecraft:stone", 0.0);
        if west_column {
            assert!(world.set_block(
                Java26BuiltinDimension::Overworld,
                BlockPos { x: 5, y: 65, z: 6 },
                block_state_id("minecraft:stone"),
                crate::fluid::SET_BLOCK_UPDATE_ALL,
            ));
        }
        place_item_at_cursor(&sender, &receiver, &mut world, support, 0.5, 0.5);
        assert_eq!(
            door_property(&mut world, BlockPos { x: 6, y: 65, z: 6 }, "hinge"),
            Some(hinge.to_string()),
            "west column {west_column}"
        );
    }
}

/// `DoorBlock.getHinge`'s last resort (`DoorBlock.java:183-190`), reached only
/// when neither column outweighs the other: the four negated comparisons on
/// `getStepX`/`getStepZ` reduce, for a door placed facing south, to "the click
/// landed at or past the middle of the face in X".
#[test]
fn a_doors_hinge_falls_back_to_where_the_click_landed_on_the_face() {
    for (cursor_x, hinge) in [(0.2_f32, "right"), (0.8, "left")] {
        let (sender, receiver, mut world, support) =
            double_high_scene("minecraft:oak_door", "minecraft:stone", 0.0);
        place_item_at_cursor(&sender, &receiver, &mut world, support, cursor_x, 0.5);
        assert_eq!(
            door_property(&mut world, BlockPos { x: 6, y: 65, z: 6 }, "hinge"),
            Some(hinge.to_string()),
            "cursor_x {cursor_x}"
        );
    }
}

/// `DoorBlock.getStateForPlacement`'s `:142` gate: no upper cell to take the
/// upper half means no door at all, not a lower half on its own.
#[test]
fn a_door_is_refused_when_the_cell_above_cannot_be_replaced() {
    let (sender, receiver, mut world, support) =
        double_high_scene("minecraft:oak_door", "minecraft:stone", 0.0);
    let upper = BlockPos { x: 6, y: 66, z: 6 };
    assert!(world.set_block(
        Java26BuiltinDimension::Overworld,
        upper,
        block_state_id("minecraft:stone"),
        crate::fluid::SET_BLOCK_UPDATE_ALL,
    ));
    let placed = place_item_at_cursor(&sender, &receiver, &mut world, support, 0.5, 0.5);
    assert_eq!(
        live_block(&mut world, BlockPos { x: 6, y: 65, z: 6 }),
        Some(block_state_id("minecraft:air"))
    );
    assert_eq!(
        live_block(&mut world, upper),
        Some(block_state_id("minecraft:stone"))
    );
    assert_eq!(placed.snapshot.players[0].inventory.slots[0].count, 1);
}

/// `DoorBlock.getStateForPlacement` (`DoorBlock.java:143`) reads the signal at
/// *both* cells, so a source that only touches the upper half still places the
/// door powered and open.
#[test]
fn a_door_is_placed_open_by_a_signal_that_only_reaches_its_upper_half() {
    let (sender, receiver, mut world, support) =
        double_high_scene("minecraft:oak_door", "minecraft:stone", 0.0);
    assert!(world.set_block(
        Java26BuiltinDimension::Overworld,
        BlockPos { x: 6, y: 66, z: 5 },
        block_state_id("minecraft:redstone_block"),
        crate::fluid::SET_BLOCK_UPDATE_ALL,
    ));
    place_item_at_cursor(&sender, &receiver, &mut world, support, 0.5, 0.5);
    for pos in [
        BlockPos { x: 6, y: 65, z: 6 },
        BlockPos { x: 6, y: 66, z: 6 },
    ] {
        assert_eq!(
            door_property(&mut world, pos, "powered"),
            Some("true".to_string()),
            "{pos:?}"
        );
        assert_eq!(
            door_property(&mut world, pos, "open"),
            Some("true".to_string()),
            "{pos:?}"
        );
    }
}

/// The other half of the `DoubleHighBlockItem` family: the six tall flowers
/// and grasses whose `DoublePlantBlock.getStateForPlacement`
/// (`DoublePlantBlock.java:64-67`) body was already here but which the item
/// kind refused to reach.
#[test]
fn a_tall_flower_placed_from_the_hand_writes_both_halves() {
    let (sender, receiver, mut world, support) =
        double_high_scene("minecraft:sunflower", "minecraft:dirt", 0.0);
    place_item_at_cursor(&sender, &receiver, &mut world, support, 0.5, 0.5);

    for (pos, half) in [
        (BlockPos { x: 6, y: 65, z: 6 }, "lower"),
        (BlockPos { x: 6, y: 66, z: 6 }, "upper"),
    ] {
        let state = live_block(&mut world, pos).expect("the sunflower is placed");
        assert_eq!(
            bbb_protocol::block_state_types::vanilla_block_state_identity(state)
                .map(|identity| identity.name.as_str()),
            Some("minecraft:sunflower"),
            "{pos:?}"
        );
        assert_eq!(
            crate::block_behavior::string_property(state, "half"),
            Some(half.to_string()),
            "{pos:?}"
        );
    }
}

fn water_source() -> i32 {
    bbb_protocol::block_state_types::vanilla_block_state_id_by_name_and_properties(
        "minecraft:water",
        &std::collections::BTreeMap::from([("level".to_string(), "0".to_string())]),
    )
    .expect("minecraft:water is a vanilla 26.1 block")
}

/// `SmallDripleafBlock.getStateForPlacement` (`SmallDripleafBlock.java:57-62`)
/// and `setPlacedBy` (`:65-73`), the last member of the `DoubleHighBlockItem`
/// family.
///
/// The facing is the opposite of the player's, and each half takes the water
/// standing in its *own* cell: a dripleaf placed into one block of water is
/// waterlogged below and dry above.
#[test]
fn a_small_dripleaf_takes_the_water_in_each_of_its_two_cells() {
    for (flooded, lower_water, upper_water) in [
        (0, "false", "false"),
        (1, "true", "false"),
        (2, "true", "true"),
    ] {
        let (sender, receiver, mut world, support) =
            double_high_scene("minecraft:small_dripleaf", "minecraft:clay", 0.0);
        for cell in 0..flooded {
            assert!(world.set_block(
                Java26BuiltinDimension::Overworld,
                BlockPos {
                    x: 6,
                    y: 65 + cell,
                    z: 6
                },
                water_source(),
                crate::fluid::SET_BLOCK_UPDATE_ALL,
            ));
        }

        place_item_at_cursor(&sender, &receiver, &mut world, support, 0.5, 0.5);

        for (cell, half, waterlogged) in [(0, "lower", lower_water), (1, "upper", upper_water)] {
            let pos = BlockPos {
                x: 6,
                y: 65 + cell,
                z: 6,
            };
            let state = live_block(&mut world, pos).expect("the dripleaf is placed");
            assert_eq!(
                bbb_protocol::block_state_types::vanilla_block_state_identity(state)
                    .map(|identity| identity.name.as_str()),
                Some("minecraft:small_dripleaf"),
                "flooded {flooded} {pos:?}"
            );
            for (property, expected) in [
                ("half", half),
                ("facing", "north"),
                ("waterlogged", waterlogged),
            ] {
                assert_eq!(
                    crate::block_behavior::string_property(state, property),
                    Some(expected.to_string()),
                    "flooded {flooded} {pos:?} {property}"
                );
            }
        }
    }
}

/// `BedBlock.getStateForPlacement` (`BedBlock.java:196-204`) and `setPlacedBy`
/// (`:295-304`): the foot faces the way the player faces and the head goes one
/// cell further that way, which is a *horizontal* second cell rather than the
/// one above.
#[test]
fn a_bed_puts_its_head_one_cell_along_the_way_the_player_faces() {
    for (y_rot, facing, head) in [
        (0.0_f32, "south", BlockPos { x: 6, y: 65, z: 7 }),
        (90.0, "west", BlockPos { x: 5, y: 65, z: 6 }),
        (180.0, "north", BlockPos { x: 6, y: 65, z: 5 }),
        (270.0, "east", BlockPos { x: 7, y: 65, z: 6 }),
    ] {
        let (sender, receiver, mut world, support) =
            double_high_scene("minecraft:red_bed", "minecraft:stone", y_rot);
        let placed = place_item_at_cursor(&sender, &receiver, &mut world, support, 0.5, 0.5);

        let foot = BlockPos { x: 6, y: 65, z: 6 };
        for (pos, part) in [(foot, "foot"), (head, "head")] {
            let state = live_block(&mut world, pos).expect("the bed is placed");
            assert_eq!(
                bbb_protocol::block_state_types::vanilla_block_state_identity(state)
                    .map(|identity| identity.name.as_str()),
                Some("minecraft:red_bed"),
                "y_rot {y_rot} {pos:?}"
            );
            for (property, expected) in [("part", part), ("facing", facing), ("occupied", "false")]
            {
                assert_eq!(
                    crate::block_behavior::string_property(state, property),
                    Some(expected.to_string()),
                    "y_rot {y_rot} {pos:?} {property}"
                );
            }
        }
        assert_eq!(placed.snapshot.players[0].inventory.slots[0].count, 0);
    }
}

/// `BedBlock.getStateForPlacement`'s second gate (`BedBlock.java:201`): the
/// border is asked about the cell the *head* would take, so a bed whose foot
/// is inside and whose head is not does not go down at all.
#[test]
fn a_bed_is_refused_when_its_head_would_land_outside_the_world_border() {
    let (sender, receiver, mut world, support) =
        double_high_scene("minecraft:red_bed", "minecraft:stone", 0.0);
    chat_command(&sender, &receiver, &mut world, 1, "worldborder set 14");

    let placed = place_item_at_cursor(&sender, &receiver, &mut world, support, 0.5, 0.5);

    let foot = BlockPos { x: 6, y: 65, z: 6 };
    assert_eq!(
        live_block(&mut world, foot),
        Some(block_state_id("minecraft:air"))
    );
    assert_eq!(placed.snapshot.players[0].inventory.slots[0].count, 1);
}

/// `SignItem` through `StandingAndWallBlockItem.getPlacementState`
/// (`StandingAndWallBlockItem.java:28-44`): the sign stands when the click is
/// on a floor and takes the wall block when it is on a wall, and either way
/// `SignItem.updateCustomBlockEntityTag` (`SignItem.java:23-36`) opens the
/// editor for whoever placed it.
#[test]
fn a_sign_stands_or_hangs_by_the_face_that_was_clicked_and_opens_its_editor() {
    let (sender, receiver, mut world, support) =
        double_high_scene("minecraft:oak_sign", "minecraft:stone", 90.0);

    let placed = place_item_at_cursor(&sender, &receiver, &mut world, support, 0.5, 0.5);

    let pos = BlockPos { x: 6, y: 65, z: 6 };
    let state = live_block(&mut world, pos).expect("the sign is placed");
    assert_eq!(
        bbb_protocol::block_state_types::vanilla_block_state_identity(state)
            .map(|identity| identity.name.as_str()),
        Some("minecraft:oak_sign")
    );
    // `RotationSegment.convertToSegment(getRotation() + 180)`
    // (`StandingSignBlock.java:46`) for a player looking west.
    assert_eq!(
        crate::block_behavior::string_property(state, "rotation"),
        Some("12".to_string())
    );
    assert_eq!(
        crate::block_behavior::string_property(state, "waterlogged"),
        Some("false".to_string())
    );
    assert_eq!(placed.snapshot.players[0].inventory.slots[0].count, 0);
    let editors = placed
        .events
        .iter()
        .filter(|event| {
            matches!(
                event,
                WorldEvent::Send {
                    packet: ClientboundPlayPacket::OpenSignEditor(editor),
                    ..
                } if editor.pos == pos && editor.is_front_text
            )
        })
        .count();
    assert_eq!(editors, 1);

    // Signs are op-only custom data (`BlockEntityType.java:274`): a creative
    // operator's load changes the sign and returns true, so no editor opens.
    chat_command(&sender, &receiver, &mut world, 1, "gamemode creative");
    chat_command(
        &sender,
        &receiver,
        &mut world,
        1,
        "item replace entity builder weapon.mainhand with \
         oak_sign[block_entity_data={id:sign,front_text:{messages:['A','','','']}}]",
    );
    let support = BlockPos { x: 5, y: 64, z: 8 };
    assert!(world.set_block(
        Java26BuiltinDimension::Overworld,
        support,
        block_state_id("minecraft:stone"),
        crate::fluid::SET_BLOCK_UPDATE_ALL,
    ));
    let placed = place_item_at_cursor(&sender, &receiver, &mut world, support, 0.5, 0.5);
    let pos = BlockPos { y: 65, ..support };
    let sign = world
        .get_block_entity(Java26BuiltinDimension::Overworld, pos)
        .and_then(crate::block_entity::BlockEntity::sign)
        .expect("the loaded sign is placed");
    assert_eq!(
        sign.text(true).messages(false)[0],
        NbtSummaryValue::string("A")
    );
    assert!(!placed.events.iter().any(|event| matches!(
        event,
        WorldEvent::Send {
            packet: ClientboundPlayPacket::OpenSignEditor(_),
            ..
        }
    )));
}

/// The wall half of the same item: a click on a vertical face takes
/// `WallSignBlock.getStateForPlacement` (`WallSignBlock.java:54-72`), whose
/// facing is the *opposite* of the first horizontal direction in the
/// nearest-looking order that survives.
///
/// Both cells beside the target hold a wall, so the two readings of `:63`
/// disagree: taking the direction itself would face the sign north, at the
/// wall behind it.
#[test]
fn a_sign_clicked_onto_a_vertical_face_becomes_the_wall_block() {
    let (sender, receiver, mut world, _support) =
        double_high_scene("minecraft:oak_sign", "minecraft:stone", 0.0);
    let wall = BlockPos { x: 6, y: 65, z: 6 };
    for pos in [wall, BlockPos { x: 6, y: 65, z: 8 }] {
        assert!(world.set_block(
            Java26BuiltinDimension::Overworld,
            pos,
            block_state_id("minecraft:stone"),
            crate::fluid::SET_BLOCK_UPDATE_ALL,
        ));
    }

    let _ = place_block_item(
        &sender,
        &receiver,
        &mut world,
        wall,
        Direction::South,
        0.5,
        8,
    );

    let pos = BlockPos { x: 6, y: 65, z: 7 };
    let state = live_block(&mut world, pos).expect("the wall sign is placed");
    assert_eq!(
        bbb_protocol::block_state_types::vanilla_block_state_identity(state)
            .map(|identity| identity.name.as_str()),
        Some("minecraft:oak_wall_sign")
    );
    assert_eq!(
        crate::block_behavior::string_property(state, "facing"),
        Some("south".to_string())
    );
}

/// `HangingSignItem` through the same
/// `StandingAndWallBlockItem.getPlacementState` body, with
/// `attachmentDirection = UP` (`HangingSignItem.java:11-13`): a click on the
/// underside of a block takes `CeilingHangingSignBlock.getStateForPlacement`
/// (`CeilingHangingSignBlock.java:96-125`).
///
/// The two cases are the two halves of `:103`. Under a full ceiling the sign
/// hangs off its edge and takes the rotation of the player's facing turned
/// around (`:119`); sneaking forces the middle attachment, and then the
/// rotation is the player's own angle plus 180 (`:120`), which for this yaw is
/// a different segment.
#[test]
fn a_hanging_sign_hangs_from_its_middle_only_when_the_ceiling_says_so() {
    for (sneaking, attached, rotation) in [(false, "false", "12"), (true, "true", "10")] {
        let (sender, receiver, mut world, _support) =
            double_high_scene("minecraft:oak_hanging_sign", "minecraft:stone", 45.0);
        let ceiling = BlockPos { x: 6, y: 66, z: 6 };
        assert!(world.set_block(
            Java26BuiltinDimension::Overworld,
            ceiling,
            block_state_id("minecraft:stone"),
            crate::fluid::SET_BLOCK_UPDATE_ALL,
        ));
        if sneaking {
            play(
                &sender,
                &receiver,
                &mut world,
                1,
                PlayServerbound::PlayerInput(PlayerInput {
                    forward: false,
                    backward: false,
                    left: false,
                    right: false,
                    jump: false,
                    shift: true,
                    sprint: false,
                }),
            );
        }

        let _ = place_block_item(
            &sender,
            &receiver,
            &mut world,
            ceiling,
            Direction::Down,
            0.5,
            8,
        );

        let pos = BlockPos { x: 6, y: 65, z: 6 };
        let state = live_block(&mut world, pos).expect("the hanging sign is placed");
        assert_eq!(
            bbb_protocol::block_state_types::vanilla_block_state_identity(state)
                .map(|identity| identity.name.as_str()),
            Some("minecraft:oak_hanging_sign"),
            "sneaking {sneaking}"
        );
        for (property, expected) in [("attached", attached), ("rotation", rotation)] {
            assert_eq!(
                crate::block_behavior::string_property(state, property),
                Some(expected.to_string()),
                "sneaking {sneaking} {property}"
            );
        }
    }
}

/// `CeilingHangingSignBlock.useItemOn` (`CeilingHangingSignBlock.java:67-87`)
/// passes a hanging sign clicked on a sign's bottom through to placement.
#[test]
fn a_hanging_sign_clicked_on_the_bottom_of_another_chains_below_it() {
    let (sender, receiver, mut world, _support) =
        double_high_scene("minecraft:oak_hanging_sign", "minecraft:stone", 45.0);
    let ceiling = BlockPos { x: 6, y: 67, z: 6 };
    assert!(world.set_block(
        Java26BuiltinDimension::Overworld,
        ceiling,
        block_state_id("minecraft:stone"),
        crate::fluid::SET_BLOCK_UPDATE_ALL,
    ));
    let first = BlockPos { x: 6, y: 66, z: 6 };
    let second = BlockPos { x: 6, y: 65, z: 6 };
    let _ = place_block_item(
        &sender,
        &receiver,
        &mut world,
        ceiling,
        Direction::Down,
        0.5,
        8,
    );
    world.stock_menu_slot_for_test(
        crate::connection::ConnectionId(1),
        36,
        ItemStackSummary {
            item_id: bbb_protocol::item_types::vanilla_item_type_for_resource_id(
                "minecraft:oak_hanging_sign",
            )
            .map(|item| item.id),
            count: 1,
            ..ItemStackSummary::empty()
        },
    );
    let _ = place_block_item(
        &sender,
        &receiver,
        &mut world,
        first,
        Direction::Down,
        0.5,
        9,
    );
    for pos in [first, second] {
        let state = live_block(&mut world, pos).expect("a hanging sign is placed");
        assert_eq!(
            bbb_protocol::block_state_types::vanilla_block_state_identity(state)
                .map(|identity| identity.name.as_str()),
            Some("minecraft:oak_hanging_sign"),
            "{pos:?}"
        );
    }
}

/// `WallHangingSignBlock.getStateForPlacement`
/// (`WallHangingSignBlock.java:117-134`) and the `canPlace`
/// (`WallHangingSignBlock.java:102-107`) that decides it.
///
/// The scene puts a wall on the clicked side *and* one east of the target, so
/// the `:124` axis filter is load-bearing: without it the first direction in
/// the order faces the sign south, which the east wall would happily hold.
#[test]
fn a_wall_hanging_sign_skips_the_clicked_axis_and_hangs_across_it() {
    let (sender, receiver, mut world, _support) =
        double_high_scene("minecraft:oak_hanging_sign", "minecraft:stone", 90.0);
    let wall = BlockPos { x: 6, y: 65, z: 6 };
    for pos in [wall, BlockPos { x: 7, y: 65, z: 7 }] {
        assert!(world.set_block(
            Java26BuiltinDimension::Overworld,
            pos,
            block_state_id("minecraft:stone"),
            crate::fluid::SET_BLOCK_UPDATE_ALL,
        ));
    }

    let _ = place_block_item(
        &sender,
        &receiver,
        &mut world,
        wall,
        Direction::South,
        0.5,
        8,
    );

    let pos = BlockPos { x: 6, y: 65, z: 7 };
    let state = live_block(&mut world, pos).expect("the wall hanging sign is placed");
    assert_eq!(
        bbb_protocol::block_state_types::vanilla_block_state_identity(state)
            .map(|identity| identity.name.as_str()),
        Some("minecraft:oak_wall_hanging_sign")
    );
    assert_eq!(
        crate::block_behavior::string_property(state, "facing"),
        Some("east".to_string())
    );
}

/// The three `GameMasterBlockItem` bodies that are not the command block.
///
/// `JigsawBlock.getStateForPlacement` (`JigsawBlock.java:51-61`) takes the
/// clicked face as its front and only reads the player when that front is
/// vertical; the structure block declares no body at all and takes
/// `Block.getStateForPlacement` (`Block.java:466-468`).
#[test]
fn the_remaining_game_master_blocks_place_from_the_face_that_was_clicked() {
    for (item, clicked_face, property, expected) in [
        ("minecraft:jigsaw", Direction::Up, "orientation", "up_north"),
        (
            "minecraft:jigsaw",
            Direction::North,
            "orientation",
            "north_up",
        ),
        ("minecraft:structure_block", Direction::Up, "mode", "load"),
        ("minecraft:test_block", Direction::Up, "mode", "start"),
    ] {
        let (sender, receiver, mut world, support) =
            double_high_scene(item, "minecraft:stone", 0.0);
        let clicked = if clicked_face == Direction::Up {
            support
        } else {
            let wall = BlockPos { x: 6, y: 65, z: 7 };
            assert!(world.set_block(
                Java26BuiltinDimension::Overworld,
                wall,
                block_state_id("minecraft:stone"),
                crate::fluid::SET_BLOCK_UPDATE_ALL,
            ));
            wall
        };
        chat_command(
            &sender,
            &receiver,
            &mut world,
            1,
            "defaultgamemode creative",
        );

        let _ = place_block_item(
            &sender,
            &receiver,
            &mut world,
            clicked,
            clicked_face,
            0.5,
            8,
        );

        let pos = BlockPos { x: 6, y: 65, z: 6 };
        let state = live_block(&mut world, pos).expect("the block is placed");
        assert_eq!(
            bbb_protocol::block_state_types::vanilla_block_state_identity(state)
                .map(|identity| identity.name.as_str()),
            Some(item),
            "{item} {clicked_face:?}"
        );
        assert_eq!(
            crate::block_behavior::string_property(state, property),
            Some(expected.to_string()),
            "{item} {clicked_face:?}"
        );
        if item == "minecraft:structure_block" {
            assert_eq!(
                world
                    .get_block_entity(Java26BuiltinDimension::Overworld, pos)
                    .unwrap()
                    .structure_block()
                    .unwrap()
                    .author(),
                "builder"
            );
        }
    }
}

fn scaffolding_state(distance: i32, bottom: bool) -> i32 {
    placement_state(
        "minecraft:scaffolding",
        &[
            ("bottom", if bottom { "true" } else { "false" }),
            ("distance", &distance.to_string()),
            ("waterlogged", "false"),
        ],
    )
}

fn assert_scaffolding(world: &mut ServerWorld, pos: BlockPos, distance: i32, bottom: bool) {
    let state = live_block(world, pos).expect("scaffolding is there");
    assert_eq!(
        bbb_protocol::block_state_types::vanilla_block_state_identity(state)
            .map(|identity| identity.name.as_str()),
        Some("minecraft:scaffolding"),
        "{pos:?}"
    );
    assert_eq!(
        bbb_server_types::block_queries::int_property(state, "distance"),
        Some(distance),
        "{pos:?}"
    );
    assert_eq!(
        bbb_server_types::block_queries::bool_property(state, "bottom"),
        Some(bottom),
        "{pos:?}"
    );
}

#[test]
fn scaffolding_places_on_a_sturdy_floor() {
    let (sender, receiver, mut world, support) =
        double_high_scene("minecraft:scaffolding", "minecraft:stone", 0.0);
    let placed = place_item_at_cursor(&sender, &receiver, &mut world, support, 0.5, 0.5);
    assert_scaffolding(&mut world, BlockPos { x: 6, y: 65, z: 6 }, 0, false);
    assert_eq!(placed.snapshot.players[0].inventory.slots[0].count, 0);
}

#[test]
fn scaffolding_extends_a_column_when_the_side_is_clicked() {
    let (sender, receiver, mut world, _support) =
        double_high_scene("minecraft:scaffolding", "minecraft:stone", 0.0);
    let column = BlockPos { x: 6, y: 65, z: 6 };
    assert!(world.set_block(
        Java26BuiltinDimension::Overworld,
        column,
        scaffolding_state(0, false),
        crate::fluid::SET_BLOCK_UPDATE_ALL,
    ));
    let placed = place_block_item(
        &sender,
        &receiver,
        &mut world,
        column,
        Direction::South,
        0.5,
        8,
    );
    assert_scaffolding(&mut world, column, 0, false);
    assert_scaffolding(&mut world, BlockPos { x: 6, y: 66, z: 6 }, 0, false);
    assert_eq!(placed.snapshot.players[0].inventory.slots[0].count, 0);
}

#[test]
fn scaffolding_extends_sideways_when_the_top_is_clicked() {
    let (sender, receiver, mut world, _support) =
        double_high_scene("minecraft:scaffolding", "minecraft:stone", 0.0);
    let column = BlockPos { x: 6, y: 65, z: 6 };
    let beside = BlockPos { x: 6, y: 65, z: 7 };
    assert!(world.set_block(
        Java26BuiltinDimension::Overworld,
        column,
        scaffolding_state(0, false),
        crate::fluid::SET_BLOCK_UPDATE_ALL,
    ));
    let placed = place_block_item(
        &sender,
        &receiver,
        &mut world,
        column,
        Direction::Up,
        1.0,
        8,
    );
    assert_scaffolding(&mut world, column, 0, false);
    assert_scaffolding(&mut world, beside, 1, true);
    assert_eq!(placed.snapshot.players[0].inventory.slots[0].count, 0);
}

#[test]
fn scaffolding_refuses_an_unsupported_neighbour() {
    let (sender, receiver, mut world, _support) =
        double_high_scene("minecraft:scaffolding", "minecraft:stone", 0.0);
    let floating = BlockPos { x: 6, y: 65, z: 6 };
    let beside = BlockPos { x: 7, y: 65, z: 6 };
    assert!(world.set_block(
        Java26BuiltinDimension::Overworld,
        floating,
        block_state_id("minecraft:stone"),
        crate::fluid::SET_BLOCK_UPDATE_ALL,
    ));
    let placed = place_block_item(
        &sender,
        &receiver,
        &mut world,
        floating,
        Direction::East,
        0.5,
        8,
    );
    assert_eq!(
        live_block(&mut world, beside),
        Some(block_state_id("minecraft:air"))
    );
    assert_eq!(placed.snapshot.players[0].inventory.slots[0].count, 1);
}

#[test]
fn scaffolding_secondary_use_inside_walks_the_opposite_face() {
    let (sender, receiver, mut world, _support) =
        double_high_scene("minecraft:scaffolding", "minecraft:stone", 0.0);
    let column = BlockPos { x: 6, y: 65, z: 6 };
    assert!(world.set_block(
        Java26BuiltinDimension::Overworld,
        column,
        scaffolding_state(0, false),
        crate::fluid::SET_BLOCK_UPDATE_ALL,
    ));
    play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::PlayerInput(PlayerInput {
            shift: true,
            ..PlayerInput::default()
        }),
    );
    let placed = play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::UseItemOn(UseItemOn {
            hand: InteractionHand::MainHand,
            hit: BlockHitResult {
                pos: column,
                direction: Direction::South,
                cursor_x: 0.5,
                cursor_y: 0.5,
                cursor_z: 0.5,
                inside: true,
                world_border_hit: false,
            },
            sequence: 8,
        }),
    );
    assert_scaffolding(&mut world, BlockPos { x: 6, y: 65, z: 5 }, 1, true);
    assert_eq!(placed.snapshot.players[0].inventory.slots[0].count, 0);
}

/// `BlockItem.place` (`BlockItem.java:76-79`) loads `block_entity_data`, then
/// `applyComponentsFromItemStack` (`BlockEntity.java:276-299`) reads the
/// implicit components over it and keeps only the remaining added ones; the
/// box collects them back, saves and reloads them unchanged, and its explosion
/// drop copies the `blocks/shulker_box` include list.
#[test]
fn a_placed_shulker_box_applies_collects_saves_and_drops_its_item_components() {
    use crate::block_entity::{BlockEntity, BlockEntityItemProjection};
    use crate::explosion::{
        ExplosionDamageProfile, ExplosionInteraction, ExplosionPresentation, PendingExplosion,
    };
    use bbb_protocol::data_component_types::{
        vanilla_data_component_type, VANILLA_DATA_COMPONENT_CONTAINER_TYPE_ID as CONTAINER,
        VANILLA_DATA_COMPONENT_CUSTOM_DATA_TYPE_ID as CUSTOM_DATA,
        VANILLA_DATA_COMPONENT_CUSTOM_NAME_TYPE_ID as CUSTOM_NAME,
        VANILLA_DATA_COMPONENT_LOCK_TYPE_ID as LOCK,
    };
    use bbb_server_types::item_stack::RuntimeItemStack;

    let (sender, receiver) = mailbox();
    let mut world = ServerWorld::new(ServerWorldConfig {
        admin_lists: operator_config(&[("builder", 1)]).admin_lists,
        ..datapack_config()
    })
    .unwrap();
    join_loaded(&sender, &receiver, &mut world, 1, "builder", 1);
    let written = chat_command(
        &sender,
        &receiver,
        &mut world,
        1,
        "item replace entity builder weapon.mainhand with shulker_box[custom_name='P',lock={},\
         container=[{slot:0,item:{id:diamond,count:7,components:{lock:{}}}}],custom_data={r:1},\
         block_entity_data={id:shulker_box,Items:[{Slot:0b,id:stone,count:2}]},!rarity]",
    );
    assert!(
        system_chats(&written)
            .iter()
            .any(|(_, text)| text.contains("commands.item.entity.set.success.single")),
        "{written:?}"
    );
    let resolver = world.item_stack_resolver_for_test();
    let original = world
        .player_runtime_inventory_slot_for_test(ConnectionId(1), 0)
        .clone();
    let child = original
        .container_contents()
        .unwrap()
        .into_iter()
        .next()
        .unwrap();

    let dimension = Java26BuiltinDimension::Overworld;
    let pos = BlockPos { x: 20, y: 70, z: 8 };
    world.set_player_position_for_test(
        ConnectionId(1),
        Vec3d {
            x: 20.5,
            y: 70.0,
            z: 5.5,
        },
    );
    assert!(world.set_block(
        dimension,
        BlockPos { y: 69, ..pos },
        block_state_id("minecraft:stone"),
        crate::fluid::SET_BLOCK_UPDATE_ALL,
    ));
    place_block_item(
        &sender,
        &receiver,
        &mut world,
        BlockPos { y: 69, ..pos },
        Direction::Up,
        1.0,
        1,
    );

    // The component container replaces the `block_entity_data` items.
    let entity = world.get_block_entity(dimension, pos).unwrap();
    let items = entity.container_items().unwrap();
    assert!(items[0].same_item_same_components(&child));
    assert!(items[1..].iter().all(RuntimeItemStack::is_empty));
    let collected = entity.collect_components(BlockEntityItemProjection::with_resolver(&resolver));
    let collected_patch = |type_id| {
        let id = vanilla_data_component_type(type_id).unwrap().resource_id;
        collected
            .entries()
            .iter()
            .find(|entry| entry.id == id)
            .map(|entry| entry.value.clone().unwrap())
    };
    for type_id in [CUSTOM_NAME, CONTAINER, CUSTOM_DATA] {
        assert_eq!(
            collected_patch(type_id),
            original.component_nbt(type_id, &resolver).unwrap(),
        );
    }
    // An empty lock equals `LockCode.NO_LOCK`, so it is not collected
    // (`BaseContainerBlockEntity.java:81`, `:160-162`).
    assert_eq!(collected_patch(LOCK), None);

    let saved = entity
        .save_with_full_metadata_with_resolver(&resolver)
        .unwrap();
    let retained = saved
        .iter()
        .find(|entry| entry.name == "components")
        .unwrap();
    let NbtSummaryValue::Compound(retained) = &retained.value else {
        unreachable!()
    };
    assert_eq!(
        retained
            .iter()
            .map(|entry| entry.name.to_jdk_string())
            .collect::<Vec<_>>(),
        ["minecraft:custom_data"]
    );
    let state = world.block_state(dimension, pos).unwrap();
    let restored = BlockEntity::load_static_with_resolver(pos, state, &saved, &resolver)
        .unwrap()
        .unwrap()
        .save_with_full_metadata_with_resolver(&resolver)
        .unwrap();
    assert_eq!(restored, saved);

    world.explode(
        dimension,
        PendingExplosion {
            source: None,
            center: Vec3d {
                x: 20.5,
                y: 70.5,
                z: 8.5,
            },
            radius: 3.0,
            fire: false,
            interaction: ExplosionInteraction::Tnt,
            damage_source: None,
            damage_profile: ExplosionDamageProfile::Default,
            presentation: ExplosionPresentation::Default,
        },
        &mut Vec::new(),
    );
    let (drop, ..) = world
        .stored_items_for_test(dimension)
        .into_iter()
        .find(|(_, _, item, ..)| item == "minecraft:shulker_box")
        .unwrap();
    let dropped = world.stored_item_stack_for_test(dimension, drop).unwrap();
    for type_id in [CUSTOM_NAME, LOCK, CONTAINER] {
        assert_eq!(
            dropped.component_nbt(type_id, &resolver).unwrap(),
            collected_patch(type_id),
        );
    }
    assert_eq!(dropped.component_nbt(CUSTOM_DATA, &resolver).unwrap(), None);
}

#[test]
fn pitcher_crop_placement_reads_live_farmland_and_brightness() {
    use crate::block_behavior::BlockPlacementContext;
    let (sender, receiver) = mailbox();
    let mut world = ServerWorld::new(creative_fixture_config()).unwrap();
    join_loaded(&sender, &receiver, &mut world, 1, "builder", 1);
    let dimension = Java26BuiltinDimension::Overworld;
    let item = bbb_protocol::item_types::vanilla_item_type_for_resource_id("minecraft:pitcher_pod")
        .unwrap()
        .id;
    let default_state = |name| {
        bbb_protocol::block_state_types::vanilla_block_default_state_id(
            bbb_protocol::block_types::vanilla_block_type_for_resource_id(name)
                .unwrap()
                .id,
        )
        .unwrap()
    };
    let crop = default_state("minecraft:pitcher_crop");
    for (pos, soil, brightness, expected) in [
        (
            BlockPos { x: 8, y: 70, z: 8 },
            "minecraft:farmland",
            15,
            Some(crop),
        ),
        (
            BlockPos { x: 8, y: 60, z: 8 },
            "minecraft:farmland",
            0,
            None,
        ),
        (BlockPos { x: 10, y: 70, z: 8 }, "minecraft:dirt", 15, None),
    ] {
        world.set_block(
            dimension,
            BlockPos {
                y: pos.y - 1,
                ..pos
            },
            default_state(soil),
            crate::fluid::SET_BLOCK_UPDATE_ALL,
        );
        world.set_block(
            dimension,
            pos,
            block_state_id("minecraft:air"),
            crate::fluid::SET_BLOCK_UPDATE_ALL,
        );
        let actual = world.with_level_for_test(dimension, |level| level.raw_brightness(pos, 0));
        assert_eq!(actual, brightness, "{pos:?}");
        assert_eq!(
            world.placed_block_state(
                ConnectionId(1),
                dimension,
                BlockPlacementContext::from(pos),
                Some(item)
            ),
            expected,
            "{pos:?}"
        );
    }
}

#[test]
fn a_door_hinges_by_complete_neighbor_doors() {
    for (x, hinge) in [(7, "right"), (5, "left")] {
        let (sender, receiver, mut world, support) =
            double_high_scene("minecraft:oak_door", "minecraft:stone", 0.0);
        world.set_block(
            Java26BuiltinDimension::Overworld,
            BlockPos { x, ..support },
            block_state_id("minecraft:stone"),
            crate::fluid::SET_BLOCK_UPDATE_ALL,
        );
        for (y, half) in [(65, "lower"), (66, "upper")] {
            let neighbor = placement_state(
                "minecraft:oak_door",
                &[
                    ("facing", "south"),
                    ("half", half),
                    ("hinge", "left"),
                    ("open", "false"),
                    ("powered", "false"),
                ],
            );
            world.set_block(
                Java26BuiltinDimension::Overworld,
                BlockPos { x, y, z: 6 },
                neighbor,
                crate::fluid::SET_BLOCK_UPDATE_ALL,
            );
        }
        assert_eq!(
            door_property(&mut world, BlockPos { x, y: 65, z: 6 }, "half"),
            Some("lower".into())
        );
        assert_eq!(
            door_property(&mut world, BlockPos { x, y: 66, z: 6 }, "half"),
            Some("upper".into())
        );
        place_item_at_cursor(&sender, &receiver, &mut world, support, 0.5, 0.5);
        for y in [65, 66] {
            assert_eq!(
                door_property(&mut world, BlockPos { x: 6, y, z: 6 }, "hinge"),
                Some(hinge.into())
            );
        }
    }
}
