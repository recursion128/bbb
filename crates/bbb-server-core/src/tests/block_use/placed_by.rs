//! The placed block's own `setPlacedBy` after a block item lands.

use super::*;

#[test]
fn block_item_placement_runs_the_placed_block_set_placed_by() {
    use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_WITHER_ID;

    let (sender, receiver) = mailbox();
    let mut world = ServerWorld::new(ServerWorldConfig {
        admin_lists: operator_config(&[("builder", 1)]).admin_lists,
        ..datapack_config()
    })
    .unwrap();
    join_loaded(&sender, &receiver, &mut world, 1, "builder", 1);
    chat_command(&sender, &receiver, &mut world, 1, "difficulty normal");
    let dimension = Java26BuiltinDimension::Overworld;
    world.set_player_position_for_test(
        ConnectionId(1),
        Vec3d {
            x: 20.5,
            y: 70.0,
            z: 5.5,
        },
    );
    let state = |name: &str, properties: &[(&str, &str)]| {
        bbb_protocol::block_state_types::vanilla_block_state_id_by_name_and_properties(
            name,
            &properties
                .iter()
                .map(|(key, value)| (key.to_string(), value.to_string()))
                .collect(),
        )
        .unwrap()
    };
    let set_state = |world: &mut ServerWorld, pos: BlockPos, state: i32| {
        assert!(world.set_block(dimension, pos, state, crate::fluid::SET_BLOCK_UPDATE_ALL,));
    };
    let set = |world: &mut ServerWorld, pos: BlockPos, name: &str| {
        set_state(world, pos, block_state_id(name))
    };
    let hold = |world: &mut ServerWorld, item: &str| {
        chat_command(
            &sender,
            &receiver,
            world,
            1,
            &format!("item replace entity builder weapon.mainhand with {item}"),
        );
    };

    // `WitherSkullBlock.checkSpawn` from the third skull.
    let bottom = BlockPos { x: 20, y: 70, z: 8 };
    set(&mut world, bottom, "minecraft:soul_sand");
    for x in 19..=21 {
        set(
            &mut world,
            BlockPos { x, y: 71, z: 8 },
            "minecraft:soul_sand",
        );
    }
    for x in [19, 21] {
        set_state(
            &mut world,
            BlockPos { x, y: 72, z: 8 },
            state(
                "minecraft:wither_skeleton_skull",
                &[("rotation", "0"), ("powered", "false")],
            ),
        );
    }
    hold(&mut world, "wither_skeleton_skull");
    place_block_item(
        &sender,
        &receiver,
        &mut world,
        BlockPos { y: 71, ..bottom },
        Direction::Up,
        1.0,
        1,
    );
    for pos in [bottom, BlockPos { y: 72, ..bottom }] {
        assert_eq!(live_block(&mut world, pos), Some(0), "{pos:?}");
    }
    let wither = world
        .stored_entity_ids_for_test(dimension)
        .into_iter()
        .find(|id| {
            world
                .stored_entity_for_test(dimension, *id)
                .is_some_and(|entity| entity.entity_type_id() == VANILLA_ENTITY_TYPE_WITHER_ID)
        })
        .expect("the completed pattern summons a wither");
    assert_eq!(
        world
            .stored_entity_for_test(dimension, wither)
            .unwrap()
            .position(),
        Vec3d {
            x: 20.5,
            y: 70.55,
            z: 8.5,
        }
    );
    let NbtSummaryValue::Compound(data) = world
        .stored_entity_command_data_for_test(dimension, wither)
        .unwrap()
    else {
        unreachable!()
    };
    // Placement runs before the level tick, which already ticks the new wither once.
    assert_eq!(field(&data, "Invul"), &NbtSummaryValue::Int(219));
    assert_eq!(
        field(&data, "Health"),
        &NbtSummaryValue::Float(100.0f32.to_bits())
    );

    // `JukeboxBlock.setPlacedBy` reads the item's `RecordItem`.
    let jukebox = BlockPos { x: 22, y: 70, z: 8 };
    set(&mut world, BlockPos { y: 69, ..jukebox }, "minecraft:stone");
    hold(
        &mut world,
        "jukebox[block_entity_data={id:jukebox,RecordItem:{id:music_disc_13,count:1}}]",
    );
    place_block_item(
        &sender,
        &receiver,
        &mut world,
        BlockPos { y: 69, ..jukebox },
        Direction::Up,
        1.0,
        2,
    );
    assert_eq!(
        live_block(&mut world, jukebox),
        Some(state("minecraft:jukebox", &[("has_record", "true")]))
    );

    // `DiodeBlock.setPlacedBy` schedules the first power check; nothing else
    // re-reads the input of a freshly placed diode.
    let repeater = BlockPos { x: 18, y: 70, z: 8 };
    set(
        &mut world,
        BlockPos { y: 69, ..repeater },
        "minecraft:stone",
    );
    set(
        &mut world,
        BlockPos { z: 7, ..repeater },
        "minecraft:redstone_block",
    );
    hold(&mut world, "repeater");
    place_block_item(
        &sender,
        &receiver,
        &mut world,
        BlockPos { y: 69, ..repeater },
        Direction::Up,
        1.0,
        3,
    );
    // The delay-1 tick lands in the same server tick as the placement packet.
    assert_eq!(
        live_block(&mut world, repeater),
        Some(state(
            "minecraft:repeater",
            &[
                ("facing", "north"),
                ("delay", "1"),
                ("locked", "false"),
                ("powered", "true"),
            ],
        ))
    );
}

#[test]
fn block_pattern_summons_award_players_intersecting_the_inflated_entity_box() {
    use crate::tests::parse_advancement;
    use bbb_server_datapack::advancement::{AdvancementElement, AdvancementSet};
    use serde_json::json;

    for (entity, body, range) in [
        ("snow_golem", "snow_block", 5.0),
        ("iron_golem", "iron_block", 5.0),
        ("copper_golem", "copper_block", 5.0),
        ("wither", "soul_sand", 50.0),
    ] {
        let (sender, receiver) = mailbox();
        let mut config = datapack_config();
        config.admin_lists = operator_config(&[("builder", 1)]).admin_lists;
        Arc::make_mut(config.datapacks.as_mut().unwrap()).advancements = AdvancementSet {
            advancements: vec![AdvancementElement {
                id: "test:summoned".into(),
                source_pack: "test".into(),
                advancement: parse_advancement(&json!({
                    "criteria":{"done":{"trigger":"minecraft:summoned_entity",
                        "conditions":{"entity":{"type":format!("minecraft:{entity}"),"nbt":"{Air:300s}"}}}},
                    "rewards":{"experience":7}
                }))
                .unwrap(),
            }],
            warnings: Vec::new(),
        };
        let mut world = ServerWorld::new(config).unwrap();
        for (id, name) in [(1, "builder"), (2, "observer"), (3, "distant")] {
            join_loaded(&sender, &receiver, &mut world, id, name, u128::from(id));
        }
        chat_command(&sender, &receiver, &mut world, 1, "difficulty normal");
        chat_command(
            &sender,
            &receiver,
            &mut world,
            1,
            "gamemode spectator observer",
        );
        for (id, x, z) in [
            (1, 20.5, 5.5),
            (2, 20.5 + range + 0.2, 8.5 + range + 0.2),
            (3, 20.5 + range + 2.0, 8.5),
        ] {
            world.set_player_position_for_test(ConnectionId(id), Vec3d { x, y: 70.0, z });
        }
        let bottom = BlockPos { x: 20, y: 70, z: 8 };
        let copper = entity == "copper_golem";
        let wide = matches!(entity, "iron_golem" | "wither");
        let head_y = if copper { 71 } else { 72 };
        assert!(world.set_block(
            Java26BuiltinDimension::Overworld,
            bottom,
            block_state_id(&format!("minecraft:{body}")),
            crate::fluid::SET_BLOCK_UPDATE_ALL,
        ));
        if !copper {
            for x in if wide { 19..=21 } else { 20..=20 } {
                assert!(world.set_block(
                    Java26BuiltinDimension::Overworld,
                    BlockPos { x, y: 71, z: 8 },
                    block_state_id(&format!("minecraft:{body}")),
                    crate::fluid::SET_BLOCK_UPDATE_ALL,
                ));
            }
        }
        if entity == "wither" {
            for x in [19, 21] {
                assert!(world.set_block(
                    Java26BuiltinDimension::Overworld,
                    BlockPos { x, y: head_y, z: 8 },
                    placement_state(
                        "minecraft:wither_skeleton_skull",
                        &[("rotation", "0"), ("powered", "false")]
                    ),
                    crate::fluid::SET_BLOCK_UPDATE_ALL,
                ));
            }
        }
        let head = if entity == "wither" {
            "wither_skeleton_skull"
        } else {
            "carved_pumpkin"
        };
        chat_command(
            &sender,
            &receiver,
            &mut world,
            1,
            &format!("item replace entity builder weapon.mainhand with {head}"),
        );
        place_block_item(
            &sender,
            &receiver,
            &mut world,
            BlockPos {
                y: head_y - 1,
                ..bottom
            },
            Direction::Up,
            1.0,
            1,
        );
        assert!(
            world.player_advancement_is_done(ConnectionId(1), "test:summoned"),
            "{entity}: builder"
        );
        assert!(
            world.player_advancement_is_done(ConnectionId(2), "test:summoned"),
            "{entity}: diagonal spectator"
        );
        assert!(
            !world.player_advancement_is_done(ConnectionId(3), "test:summoned"),
            "{entity}: distant"
        );
        assert_eq!(
            world.experience_total_for_test(ConnectionId(2)),
            7,
            "{entity}"
        );
    }
}
