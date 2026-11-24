use super::*;
use crate::entity::{EntityData, GeneralLivingData};
use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_DROWNED_ID;
use bbb_protocol::packets::{MovePlayer, PlayerRotation};

const DIMENSION: Java26BuiltinDimension = Java26BuiltinDimension::Overworld;

fn body(world: &ServerWorld, id: i32) -> &crate::entity::Entity {
    world
        .stored_entity_for_test(DIMENSION, id)
        .expect("the drowned remains live")
}

fn answer_keep_alives(sender: &WorldCommandSender, tick: TickResult) -> TickResult {
    for event in &tick.events {
        if let WorldEvent::Send {
            connection_id,
            packet: ClientboundPlayPacket::KeepAlive { id },
        } = event
        {
            enqueue(
                sender,
                WorldCommand::Play {
                    connection_id: *connection_id,
                    packet: PlayServerbound::KeepAlive { id: *id },
                },
            )
            .unwrap();
        }
    }
    tick
}

fn move_player(
    sender: &WorldCommandSender,
    receiver: &WorldCommandReceiver,
    world: &mut ServerWorld,
    position: Vec3d,
) {
    answer_keep_alives(
        sender,
        play(
            sender,
            receiver,
            world,
            1,
            PlayServerbound::MovePlayer(MovePlayer {
                position: Some(position),
                rotation: Some(PlayerRotation {
                    y_rot: 90.0,
                    x_rot: 0.0,
                }),
                on_ground: false,
                horizontal_collision: false,
            }),
        ),
    );
    assert_eq!(
        world.player_transition_state_for_test(ConnectionId(1)).1,
        position,
        "the fixture's player movement is accepted"
    );
}

fn tick(
    sender: &WorldCommandSender,
    receiver: &WorldCommandReceiver,
    world: &mut ServerWorld,
) -> TickResult {
    answer_keep_alives(sender, world.tick(receiver.drain()))
}

fn pool_with_shore(world: &mut ServerWorld) {
    let stone = block_state_id("minecraft:stone");
    let water = helpers::default_block_state("minecraft:water");
    let air = block_state_id("minecraft:air");
    for x in -2..=18 {
        let floor = if x < 8 { 61 } else { (x - 8 + 62).min(68) };
        for z in 2..=8 {
            for y in 61..=73 {
                let state = if y <= floor || ((z == 2 || z == 8) && y <= 70) {
                    stone
                } else if x < 14 && y <= 68 {
                    water
                } else {
                    air
                };
                let _ = world.set_block(
                    DIMENSION,
                    BlockPos { x, y, z },
                    state,
                    SET_BLOCK_UPDATE_CLIENTS,
                );
            }
        }
    }
    for z in 3..=7 {
        for y in 62..=64 {
            let _ = world.set_block(
                DIMENSION,
                BlockPos { x: 5, y, z },
                stone,
                SET_BLOCK_UPDATE_CLIENTS,
            );
        }
    }
}

#[test]
fn drowned_swims_over_a_submerged_wall_then_follows_its_player_target_onto_land() {
    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_restore(
        ServerWorldConfig {
            game_type: GameType::Survival,
            difficulty: Difficulty::Normal,
            ..operator_config(&[("alice", 1)])
        },
        &restore_candidate(13_000, None, Vec::new()),
    );
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    answer_keep_alives(
        &sender,
        chat_command(&sender, &receiver, &mut world, 1, "time set midnight"),
    );
    pool_with_shore(&mut world);
    let joined_at = world.player_transition_state_for_test(ConnectionId(1)).1;
    move_player(
        &sender,
        &receiver,
        &mut world,
        Vec3d {
            y: 72.0,
            ..joined_at
        },
    );
    move_player(
        &sender,
        &receiver,
        &mut world,
        Vec3d {
            x: 4.5,
            y: 72.0,
            z: 5.5,
        },
    );
    move_player(
        &sender,
        &receiver,
        &mut world,
        Vec3d {
            x: 9.5,
            y: 66.0,
            z: 5.5,
        },
    );
    let (id, uuid) = world.spawn_stored_entity_for_test(
        DIMENSION,
        VANILLA_ENTITY_TYPE_DROWNED_ID,
        Vec3d {
            x: 1.5,
            y: 62.0,
            z: 5.5,
        },
        EntityData::GeneralLiving(GeneralLivingData::new(VANILLA_ENTITY_TYPE_DROWNED_ID).unwrap()),
    );
    answer_keep_alives(
        &sender,
        chat_command(
            &sender,
            &receiver,
            &mut world,
            1,
            &format!("data merge entity {uuid} {{PersistenceRequired:1b}}"),
        ),
    );
    let mut swam = false;
    let mut crossed = false;
    for _ in 0..300 {
        tick(&sender, &receiver, &mut world);
        let drowned = body(&world, id);
        swam |= drowned.is_swimming();
        if drowned.position().x > 6.2 && drowned.position().y > 64.9 {
            crossed = true;
            break;
        }
    }
    assert!(swam, "the waterborne player makes the drowned swim");
    assert!(
        crossed,
        "the drowned swims up and over the wall: {:?}",
        body(&world, id).position()
    );

    answer_keep_alives(
        &sender,
        chat_command(
            &sender,
            &receiver,
            &mut world,
            1,
            &format!("data merge entity {uuid} {{NoAI:1b}}"),
        ),
    );
    assert!(!body(&world, id).is_swimming());
    let stopped = body(&world, id).position();
    for _ in 0..5 {
        tick(&sender, &receiver, &mut world);
    }
    assert_eq!(body(&world, id).position(), stopped);
    answer_keep_alives(
        &sender,
        chat_command(
            &sender,
            &receiver,
            &mut world,
            1,
            &format!("data merge entity {uuid} {{NoAI:0b}}"),
        ),
    );

    move_player(
        &sender,
        &receiver,
        &mut world,
        Vec3d {
            x: 9.5,
            y: 69.0,
            z: 5.5,
        },
    );
    move_player(
        &sender,
        &receiver,
        &mut world,
        Vec3d {
            x: 15.5,
            y: 69.0,
            z: 5.5,
        },
    );
    let mut health = world.player_health_for_test(ConnectionId(1));
    let mut attacked = false;
    for _ in 0..800 {
        let tick = tick(&sender, &receiver, &mut world);
        for (_, next) in set_healths(&tick) {
            attacked |= next < health && body(&world, id).position().x > 14.0;
            health = next;
        }
        if body(&world, id).position().x > 14.0 && attacked {
            break;
        }
    }
    let drowned = body(&world, id);
    assert!(
        drowned.position().x > 14.0 && drowned.position().y >= 68.9,
        "the drowned climbs the shallow bank to its player: {:?}",
        drowned.position()
    );
    assert!(!drowned.is_in_water());
    assert!(!drowned.is_swimming());
    assert!(
        attacked,
        "the drowned reaches and attacks its player on shore"
    );
}

fn autonomous_pool(world: &mut ServerWorld) {
    let stone = block_state_id("minecraft:stone");
    let water = helpers::default_block_state("minecraft:water");
    let air = block_state_id("minecraft:air");
    for x in -6..=16 {
        let floor = (x + 53).clamp(55, 63);
        for z in -6..=16 {
            for y in 55..=68 {
                let state = if y <= floor {
                    stone
                } else if y <= 62 {
                    water
                } else {
                    air
                };
                let _ = world.set_block(
                    DIMENSION,
                    BlockPos { x, y, z },
                    state,
                    SET_BLOCK_UPDATE_CLIENTS,
                );
            }
        }
    }
}

fn autonomous_column(x: i32, z: i32) -> Java26ChunkData {
    let sections = (-5_i8..=20)
        .map(|y| {
            let mut section = if (-4..=19).contains(&y) {
                restore_section_with_cells(
                    y,
                    if y == 3 {
                        "minecraft:stone"
                    } else {
                        "minecraft:air"
                    },
                    &[],
                )
            } else {
                NbtSummaryValue::Compound(vec![nbt("Y", NbtSummaryValue::Byte(y))])
            };
            let NbtSummaryValue::Compound(entries) = &mut section else {
                unreachable!()
            };
            entries.push(nbt(
                "SkyLight",
                NbtSummaryValue::ByteArray(vec![if y > 3 { -1 } else { 0 }; 2048]),
            ));
            entries.push(nbt("BlockLight", NbtSummaryValue::ByteArray(vec![0; 2048])));
            section
        })
        .collect();
    let mut raw = restore_fixture_chunk_raw(sections);
    raw.entries_mut()
        .push(nbt("isLightOn", NbtSummaryValue::Byte(1)));
    for entry in raw.entries_mut() {
        match entry.name.to_jdk_string().as_str() {
            "xPos" => entry.value = NbtSummaryValue::Int(x),
            "zPos" => entry.value = NbtSummaryValue::Int(z),
            "Heightmaps" => {
                let NbtSummaryValue::Compound(maps) = &mut entry.value else {
                    unreachable!()
                };
                // The saved slab ends at y=63: all four first-available heights are 64.
                let packed: Vec<_> = (0..37)
                    .map(|word| {
                        (0..(256_usize - word * 7).min(7))
                            .fold(0_i64, |value, cell| value | (128_i64 << (cell * 9)))
                    })
                    .collect();
                for map in maps {
                    map.value = NbtSummaryValue::LongArray(packed.clone());
                }
            }
            _ => {}
        }
    }
    Java26ChunkData::parse(
        raw,
        Java26ChunkContext {
            expected_x: x,
            expected_z: z,
            min_section_y: -4,
            section_count: 24,
        },
    )
    .unwrap()
}

fn autonomous_drowned(
    time: &str,
    position: Vec3d,
) -> (WorldCommandSender, WorldCommandReceiver, ServerWorld, i32) {
    let (sender, receiver) = mailbox();
    let mut candidate = restore_candidate_with_chunks(
        0,
        (-3..=3)
            .flat_map(|x| (-3..=3).map(move |z| ((x, z), autonomous_column(x, z))))
            .collect(),
    );
    candidate.world_gen_settings.dimensions.insert(
        level_key(DIMENSION),
        crate::chunk::noise_generator::overworld_noise_stem(),
    );
    let mut world = spawn_pinned_restore(
        ServerWorldConfig {
            game_type: GameType::Creative,
            datapacks: Some(crate::chunk::noise_generator::overworld_startup(true)),
            difficulty: Difficulty::Normal,
            ..operator_config(&[("alice", 1)])
        },
        &candidate,
    );
    assert_eq!(world.level_sea_level(DIMENSION), 63);
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    answer_keep_alives(
        &sender,
        chat_command(
            &sender,
            &receiver,
            &mut world,
            1,
            &format!("time set {time}"),
        ),
    );
    autonomous_pool(&mut world);
    let (id, uuid) = world.spawn_stored_entity_for_test(
        DIMENSION,
        VANILLA_ENTITY_TYPE_DROWNED_ID,
        position,
        EntityData::GeneralLiving(GeneralLivingData::new(VANILLA_ENTITY_TYPE_DROWNED_ID).unwrap()),
    );
    answer_keep_alives(
        &sender,
        chat_command(
            &sender,
            &receiver,
            &mut world,
            1,
            &format!("data merge entity {uuid} {{PersistenceRequired:1b}}"),
        ),
    );
    (sender, receiver, world, id)
}

#[test]
fn drowned_seeks_water_in_daylight_without_a_combat_target() {
    let (sender, receiver, mut world, id) = autonomous_drowned(
        "noon",
        Vec3d {
            x: 11.5,
            y: 64.0,
            z: 5.5,
        },
    );
    assert!(!body(&world, id).is_in_water());
    let mut entered = false;
    for _ in 0..800 {
        tick(&sender, &receiver, &mut world);
        if body(&world, id).is_in_water() {
            entered = true;
            break;
        }
    }
    assert!(
        entered,
        "the drowned seeks nearby water: {:?}",
        body(&world, id).position()
    );
}

#[test]
fn drowned_swims_up_from_deep_water_and_walks_onto_a_shallow_shore_at_night() {
    for deep in [true, false] {
        let (sender, receiver, mut world, id) = autonomous_drowned(
            "midnight",
            if deep {
                Vec3d {
                    x: 1.5,
                    y: 56.0,
                    z: 5.5,
                }
            } else {
                Vec3d {
                    x: 6.5,
                    y: 60.0,
                    z: 5.5,
                }
            },
        );
        let mut swam = false;
        let mut reached = false;
        for _ in 0..800 {
            tick(&sender, &receiver, &mut world);
            let drowned = body(&world, id);
            swam |= drowned.is_swimming();
            reached = if deep {
                drowned.position().y > 60.5
            } else {
                drowned.position().x > 8.5 && !drowned.is_in_water()
            };
            if reached {
                break;
            }
        }
        assert!(reached, "deep={deep}: {:?}", body(&world, id).position());
        if deep {
            assert!(
                swam,
                "the targetless drowned uses its swimming pose while surfacing"
            );
        } else {
            assert!(!body(&world, id).is_swimming());
        }
    }
}
