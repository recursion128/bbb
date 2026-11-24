//! Shulker shells and relocation through live ticks, damage and tracked metadata.

use super::melee::{fixture, ALICE};
use super::*;
use crate::tests::entities::{console_command, console_command_answer};
use bbb_protocol::attribute_types::VANILLA_ATTRIBUTE_ARMOR_ID;
use bbb_protocol::entity_types::{
    VANILLA_ENTITY_TYPE_COW_ID, VANILLA_ENTITY_TYPE_HAPPY_GHAST_ID, VANILLA_ENTITY_TYPE_SHULKER_ID,
};
use bbb_protocol::packets::{EntityDataValueKind, SoundEventHolder};

fn saved_byte(world: &ServerWorld, id: i32, name: &str) -> i8 {
    let Some(NbtSummaryValue::Compound(fields)) =
        world.stored_entity_command_data_for_test(DIMENSION, id)
    else {
        panic!("shulker {id} has saved data");
    };
    match fields
        .iter()
        .find(|field| field.name == name)
        .unwrap()
        .value
    {
        NbtSummaryValue::Byte(value) => value,
        _ => panic!("{name} is a byte"),
    }
}

fn peek(world: &ServerWorld, id: i32) -> i8 {
    saved_byte(world, id, "Peek")
}

fn position(world: &ServerWorld, id: i32) -> Vec3d {
    world
        .stored_entity_for_test(DIMENSION, id)
        .unwrap()
        .position()
}

fn armor(world: &ServerWorld, id: i32) -> f64 {
    world
        .stored_entity_for_test(DIMENSION, id)
        .unwrap()
        .living()
        .unwrap()
        .attribute_value(VANILLA_ATTRIBUTE_ARMOR_ID)
}

fn metadata(tick: &TickResult, id: i32, peek: i8) -> bool {
    tick.events.iter().any(|event| {
        matches!(event,
        WorldEvent::Send { packet: ClientboundPlayPacket::SetEntityData(update), .. }
        if update.id == id && update.values.iter().any(|value|
            value.data_id == bbb_protocol::entity_synched_data::VANILLA_SHULKER_PEEK_DATA_ID
            && value.value == EntityDataValueKind::Byte(peek)))
    })
}

fn sound(tick: &TickResult, name: &str) -> bool {
    let registry_id =
        bbb_protocol::sound_event_types::vanilla_sound_event_registry_id(name).unwrap();
    tick.events.iter().any(|event| {
        matches!(event,
        WorldEvent::Send { packet: ClientboundPlayPacket::Sound(sound), .. }
        if sound.sound == SoundEventHolder::Reference { registry_id }
        && sound.volume == 1.0 && sound.pitch == 1.0)
    })
}

fn command(
    sender: &WorldCommandSender,
    receiver: &WorldCommandReceiver,
    world: &mut ServerWorld,
    text: &str,
) -> TickResult {
    let operation_id = world.game_time() as u64 + 10000;
    let tick = console_command(sender, receiver, world, operation_id, text);
    keep_alive(sender, &tick);
    assert_eq!(console_command_answer(&tick).0, Some(1), "{text}");
    tick
}

fn keep_alive(sender: &WorldCommandSender, tick: &TickResult) {
    for event in &tick.events {
        if let WorldEvent::Send {
            connection_id: ALICE,
            packet: ClientboundPlayPacket::KeepAlive { id },
        } = event
        {
            enqueue(
                sender,
                WorldCommand::Play {
                    connection_id: ALICE,
                    packet: PlayServerbound::KeepAlive { id: *id },
                },
            )
            .unwrap();
        }
    }
}

fn shells(world: &ServerWorld) -> Vec<i32> {
    world
        .stored_entity_ids_for_test(DIMENSION)
        .into_iter()
        .filter(|id| {
            world
                .stored_entity_for_test(DIMENSION, *id)
                .unwrap()
                .entity_type_id()
                == VANILLA_ENTITY_TYPE_SHULKER_ID
        })
        .collect()
}

fn hit_with_shulker_bullet(
    sender: &WorldCommandSender,
    receiver: &WorldCommandReceiver,
    world: &mut ServerWorld,
    target: i32,
) {
    let body = world.stored_entity_for_test(DIMENSION, target).unwrap();
    let origin = body.position();
    let health = body.living().unwrap().health();
    let uuid = body.uuid();
    let movement = Vec3d {
        x: -1.5,
        y: 0.0,
        z: 0.0,
    };
    let (bullet, _) = world.spawn_stored_entity_for_test(
        DIMENSION,
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_SHULKER_BULLET_ID,
        Vec3d {
            x: origin.x + 1.5,
            y: origin.y + 0.5,
            z: origin.z,
        },
        EntityData::ShulkerBullet(crate::entity::ShulkerBulletData::from_saved(
            crate::entity::hurting_projectile::ProjectileState::from_saved(None, true, true),
            Some(uuid),
            Some(bbb_protocol::packets::Direction::West),
            20,
            Vec3d {
                x: -1.0,
                y: 0.0,
                z: 0.0,
            },
        )),
    );
    world
        .stored_entity_mut_for_test(DIMENSION, bullet)
        .unwrap()
        .set_delta_movement(movement);
    for _ in 0..8 {
        let tick = world.tick(receiver.drain());
        keep_alive(sender, &tick);
        if world.stored_entity_for_test(DIMENSION, bullet).is_none() {
            break;
        }
    }
    assert!(
        world.stored_entity_for_test(DIMENSION, bullet).is_none(),
        "the real bullet hits and is destroyed"
    );
    assert!(
        world
            .stored_entity_for_test(DIMENSION, target)
            .unwrap()
            .living()
            .unwrap()
            .health()
            < health,
        "the actual projectile hurt succeeds"
    );
    for _ in 0..20 {
        let tick = world.tick(receiver.drain());
        keep_alive(sender, &tick);
    }
}

#[test]
fn shulker_bullet_hits_split_open_mobile_shells_and_respect_nearby_density() {
    for neighbors in [0, 5] {
        let (sender, receiver, mut world) = fixture();
        command(&sender, &receiver, &mut world, "gamemode creative alice");
        let origin = Vec3d {
            x: 8.5,
            y: 65.0,
            z: 12.5,
        };
        let (parent, uuid) = spawn(&mut world, VANILLA_ENTITY_TYPE_SHULKER_ID, origin);
        command(
            &sender,
            &receiver,
            &mut world,
            &format!(
                r#"data merge entity {uuid} {{NoAI:1b,Peek:100b,Color:5b,CustomName:{{text:"Parent"}},Tags:["parent"],data:{{kept:17}},equipment:{{mainhand:{{id:"minecraft:diamond",count:1}}}}}}"#
            ),
        );
        for (x, z) in [(3.0, 0.0), (-3.0, 0.0), (0.0, 3.0), (0.0, -3.0), (3.0, 3.0)]
            .into_iter()
            .take(neighbors)
        {
            let (_, neighbor) = spawn(
                &mut world,
                VANILLA_ENTITY_TYPE_SHULKER_ID,
                Vec3d {
                    x: origin.x + x,
                    z: origin.z + z,
                    ..origin
                },
            );
            command(
                &sender,
                &receiver,
                &mut world,
                &format!("data merge entity {neighbor} {{NoAI:1b}}"),
            );
        }
        if neighbors == 0 {
            hit_with_shulker_bullet(&sender, &receiver, &mut world, parent);
            assert_eq!(
                position(&world, parent),
                origin,
                "NoAI prevents the required relocation"
            );
            assert_eq!(shells(&world), vec![parent]);
        }
        let mut relocated = false;
        for _ in 0..80 {
            command(
                &sender,
                &receiver,
                &mut world,
                &format!("data merge entity {uuid} {{NoAI:0b,Peek:100b,Health:30.0f}}"),
            );
            hit_with_shulker_bullet(&sender, &receiver, &mut world, parent);
            if position(&world, parent) != origin {
                relocated = true;
                break;
            }
            assert_eq!(
                shells(&world).len(),
                neighbors + 1,
                "a failed relocation cannot leave offspring"
            );
        }
        assert!(
            relocated,
            "the open, healthy parent eventually relocates after a real hit"
        );
        let ids = shells(&world);
        assert_eq!(
            ids.len(),
            if neighbors == 0 { 2 } else { 6 },
            "one nearby shell always reproduces, while six suppress reproduction"
        );
        if neighbors == 0 {
            let child = *ids.iter().find(|id| **id != parent).unwrap();
            assert_eq!(position(&world, child), origin);
            assert_eq!(saved_byte(&world, child, "Color"), 5);
            assert!(
                matches!(peek(&world, child), 0 | 30),
                "the fresh shell has normal idle peek, not the parent's forced full peek"
            );
            let body = world.stored_entity_for_test(DIMENSION, child).unwrap();
            let living = body.living().unwrap();
            assert_eq!(living.health(), living.max_health());
            assert!(body.custom_name().is_none() && body.entity_tags().is_empty());
            assert!(
                living.equipment().is_empty(),
                "parent equipment is not inherited"
            );
            let Some(NbtSummaryValue::Compound(fields)) =
                world.stored_entity_command_data_for_test(DIMENSION, child)
            else {
                panic!("the child has saved data");
            };
            assert!(
                !fields.iter().any(|field| field.name == "data"),
                "parent custom data is not inherited"
            );
        }
        assert_eq!(
            world.publication().players.len(),
            1,
            "Alice stays connected through the bounded hit window"
        );
    }
}

#[test]
fn a_sideways_no_ai_shell_blocks_mob_and_player_movement_until_closed() {
    let (sender, receiver, mut world) = fixture();
    let origin = Vec3d {
        x: 8.5,
        y: 65.0,
        z: 12.5,
    };
    assert!(world.set_block(
        DIMENSION,
        BlockPos { x: 7, y: 65, z: 12 },
        block_state_id("minecraft:stone"),
        SET_BLOCK_UPDATE_ALL,
    ));
    let (shell, uuid) = spawn(&mut world, VANILLA_ENTITY_TYPE_SHULKER_ID, origin);
    command(
        &sender,
        &receiver,
        &mut world,
        &format!("data merge entity {uuid} {{AttachFace:4b,NoAI:1b,Peek:100b}}"),
    );
    for _ in 0..40 {
        let tick = world.tick(receiver.drain());
        keep_alive(&sender, &tick);
        if world
            .stored_entity_for_test(DIMENSION, shell)
            .unwrap()
            .bounding_box()
            .max_x
            > 9.99
        {
            break;
        }
    }
    let bounds = world
        .stored_entity_for_test(DIMENSION, shell)
        .unwrap()
        .bounding_box();
    assert!(
        (bounds.max_x - 10.0).abs() < 0.01,
        "the lid opens east: {bounds:?}"
    );
    assert_eq!(position(&world, shell), origin);
    let (cow, cow_uuid) = spawn(
        &mut world,
        VANILLA_ENTITY_TYPE_COW_ID,
        Vec3d { x: 11.5, ..origin },
    );
    command(
        &sender,
        &receiver,
        &mut world,
        &format!("data merge entity {cow_uuid} {{NoGravity:1b,Motion:[-1.8d,0.0d,0.0d]}}"),
    );
    let cow_body = world.stored_entity_for_test(DIMENSION, cow).unwrap();
    assert!(
        cow_body.position().x < 11.5 && cow_body.bounding_box().min_x >= bounds.max_x - 1.0e-6,
        "ordinary travel collides with the open lid: {:?}",
        cow_body.position()
    );
    let outside = Vec3d { x: 11.0, ..origin };
    let through_lid = Vec3d { x: 9.7, ..origin };
    world.set_player_position_for_test(ALICE, outside);
    let rejected = play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::MovePlayer(MovePlayer {
            position: Some(through_lid),
            rotation: None,
            on_ground: true,
            horizontal_collision: false,
        }),
    );
    let correction = rejected
        .events
        .iter()
        .find_map(|event| match event {
            WorldEvent::Send {
                connection_id: ALICE,
                packet: ClientboundPlayPacket::PlayerPosition(update),
            } => Some(*update),
            _ => None,
        })
        .expect("a client entering the open lid receives its own collision correction");
    assert_eq!(world.publication().players[0].position, outside);
    play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::AcceptTeleportation { id: correction.id },
    );
    command(
        &sender,
        &receiver,
        &mut world,
        &format!("data merge entity {uuid} {{Peek:0b}}"),
    );
    // Shulker.java:155-160,210-224: `/data merge` preserves `currentPeekAmount`
    // so the lid closes smoothly over 20 ticks rather than collapsing instantly.
    assert!(
        world
            .stored_entity_for_test(DIMENSION, shell)
            .unwrap()
            .bounding_box()
            .max_x
            > 9.9,
        "command reload preserves current_peek before closing"
    );
    for _ in 0..40 {
        let tick = world.tick(receiver.drain());
        keep_alive(&sender, &tick);
        if world
            .stored_entity_for_test(DIMENSION, shell)
            .unwrap()
            .bounding_box()
            .max_x
            < 9.01
        {
            break;
        }
    }
    let closed = world
        .stored_entity_for_test(DIMENSION, shell)
        .unwrap()
        .bounding_box();
    assert!(
        (closed.max_x - 9.0).abs() < 0.01,
        "closing frees the lid's space: {closed:?}"
    );
    play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::MovePlayer(MovePlayer {
            position: Some(through_lid),
            rotation: None,
            on_ground: true,
            horizontal_collision: false,
        }),
    );
    assert_eq!(world.publication().players[0].position, through_lid);
}

#[test]
fn a_scaled_no_ai_shell_moves_live_players_and_entities_against_block_collisions() {
    let (sender, receiver, mut world) = fixture();
    let origin = Vec3d {
        x: 8.5,
        y: 65.0,
        z: 12.5,
    };
    let (shell, uuid) = spawn(&mut world, VANILLA_ENTITY_TYPE_SHULKER_ID, origin);
    command(
        &sender,
        &receiver,
        &mut world,
        &format!("data merge entity {uuid} {{NoAI:1b}}"),
    );
    command(
        &sender,
        &receiver,
        &mut world,
        &format!("attribute {uuid} minecraft:scale base set 2"),
    );
    assert!(world.set_block(
        DIMENSION,
        BlockPos { x: 7, y: 70, z: 12 },
        block_state_id("minecraft:stone"),
        SET_BLOCK_UPDATE_ALL
    ));
    let cow_origin = Vec3d {
        x: 7.9,
        y: 67.1,
        z: 12.5,
    };
    let (cow, cow_uuid) = spawn(&mut world, VANILLA_ENTITY_TYPE_COW_ID, cow_origin);
    command(
        &sender,
        &receiver,
        &mut world,
        &format!("data merge entity {cow_uuid} {{NoAI:1b,NoGravity:1b}}"),
    );
    let player_origin = Vec3d {
        x: 9.1,
        ..cow_origin
    };
    world.set_player_position_for_test(ALICE, player_origin);
    command(
        &sender,
        &receiver,
        &mut world,
        &format!("data merge entity {uuid} {{Peek:100b}}"),
    );
    for _ in 0..40 {
        let tick = world.tick(receiver.drain());
        keep_alive(&sender, &tick);
        if world
            .stored_entity_for_test(DIMENSION, shell)
            .unwrap()
            .bounding_box()
            .max_y
            > 68.99
        {
            break;
        }
    }
    let bounds = world
        .stored_entity_for_test(DIMENSION, shell)
        .unwrap()
        .bounding_box();
    assert!(
        (bounds.max_y - 69.0).abs() < 0.01,
        "scaled shell has full physical height: {bounds:?}"
    );
    assert_eq!(position(&world, shell), origin);
    let cow_body = world.stored_entity_for_test(DIMENSION, cow).unwrap();
    assert!(
        cow_body.position().y > cow_origin.y + 0.5
            && cow_body.bounding_box().max_y <= 70.0 + 1.0e-6,
        "the opening pushes the cow until its ceiling: {:?}",
        cow_body.bounding_box()
    );
    let pushed = world.publication().players[0].position;
    assert!(
        pushed.y > 68.99 && pushed.x == player_origin.x && pushed.z == player_origin.z,
        "the online session is actually lifted: {pushed:?}"
    );
    let rejected = play(
        &sender,
        &receiver,
        &mut world,
        1,
        PlayServerbound::MovePlayer(MovePlayer {
            position: Some(player_origin),
            rotation: None,
            on_ground: false,
            horizontal_collision: false,
        }),
    );
    assert!(
        rejected.events.iter().any(|event| matches!(event,
        WorldEvent::Send { connection_id: ALICE,
            packet: ClientboundPlayPacket::PlayerPosition(update) } if update.position == pushed)),
        "a stale client move into the shell receives the pushed authoritative position"
    );
    assert_eq!(world.publication().players[0].position, pushed);
}

#[test]
fn an_idle_shulker_peeks_closes_and_keeps_an_attacking_shell_open() {
    let (sender, receiver, mut world) = fixture();
    command(&sender, &receiver, &mut world, "gamemode creative alice");
    let (id, _) = spawn(
        &mut world,
        VANILLA_ENTITY_TYPE_SHULKER_ID,
        Vec3d {
            x: 12.5,
            y: 65.0,
            z: 8.5,
        },
    );
    assert_eq!(armor(&world, id), 20.0);
    let mut opened = false;
    let mut open_sound = false;
    for _ in 0..600 {
        let tick = world.tick(Vec::new());
        opened |= metadata(&tick, id, 30);
        open_sound |= sound(&tick, "minecraft:entity.shulker.open");
        if opened && open_sound {
            break;
        }
    }
    assert!(
        opened && open_sound,
        "idle peeking sends metadata and shell sound"
    );
    assert_eq!(peek(&world, id), 30);
    assert_eq!(armor(&world, id), 0.0);
    let mut closed = false;
    let mut close_sound = false;
    for _ in 0..70 {
        let tick = world.tick(Vec::new());
        closed |= metadata(&tick, id, 0);
        close_sound |= sound(&tick, "minecraft:entity.shulker.close");
        if closed && close_sound {
            break;
        }
    }
    assert!(closed && close_sound, "the bounded idle peek closes");
    assert_eq!(armor(&world, id), 20.0);
    tick_until(&mut world, 600, |world| peek(world, id) == 30);
    // Shulker.java:96: beyond the 8-block priority-1 LookAtPlayerGoal, which
    // would otherwise take LOOK from the attack, but inside the attack box.
    world.set_player_position_for_test(
        ALICE,
        Vec3d {
            x: 12.5,
            y: 74.5,
            z: 8.5,
        },
    );
    command(&sender, &receiver, &mut world, "gamemode survival alice");
    tick_until(&mut world, 100, |world| peek(world, id) == 100);
    for _ in 0..65 {
        let tick = world.tick(Vec::new());
        assert!(
            !metadata(&tick, id, 0),
            "idle stop cannot close an attacking shell"
        );
        assert_eq!(peek(&world, id), 100);
    }
    assert_eq!(target_of(&world, id), Some(Uuid::from_u128(1)));
    assert_eq!(armor(&world, id), 0.0);
    assert!(world.player_health_for_test(ALICE) > 0.0);
}

#[test]
fn idle_peeking_requires_support_and_clear_space_in_the_attachment_direction() {
    let (sender, receiver, mut world) = fixture();
    command(&sender, &receiver, &mut world, "gamemode creative alice");
    let stone = block_state_id("minecraft:stone");
    let mut ids = Vec::new();
    for (x, support, opening) in [(3, false, false), (6, true, true), (12, true, false)] {
        for (at, enabled) in [(x + 1, support), (x - 1, opening)] {
            if enabled {
                assert!(world.set_block(
                    DIMENSION,
                    BlockPos { x: at, y: 65, z: 8 },
                    if x == 12 && at == x + 1 {
                        bbb_protocol::block_state_types::vanilla_block_default_state_id(
                            bbb_protocol::block_types::VANILLA_BLOCK_SHULKER_BOX_ID,
                        )
                        .unwrap()
                    } else {
                        stone
                    },
                    SET_BLOCK_UPDATE_ALL
                ));
            }
        }
        let (id, uuid) = spawn(
            &mut world,
            VANILLA_ENTITY_TYPE_SHULKER_ID,
            Vec3d {
                x: f64::from(x) + 0.5,
                y: 65.0,
                z: 8.5,
            },
        );
        command(
            &sender,
            &receiver,
            &mut world,
            &format!("data merge entity {uuid} {{AttachFace:5b,Invulnerable:1b}}"),
        );
        ids.push((id, position(&world, id)));
    }
    assert!(world.set_block(
        DIMENSION,
        BlockPos {
            x: 13,
            y: 65,
            z: 12
        },
        stone,
        SET_BLOCK_UPDATE_ALL,
    ));
    let (_, uuid) = spawn(
        &mut world,
        VANILLA_ENTITY_TYPE_HAPPY_GHAST_ID,
        Vec3d {
            x: 10.0,
            y: 65.0,
            z: 12.5,
        },
    );
    command(
        &sender,
        &receiver,
        &mut world,
        &format!("data merge entity {uuid} {{NoAI:1b,still_timeout:1000}}"),
    );
    let (blocked, uuid) = spawn(
        &mut world,
        VANILLA_ENTITY_TYPE_SHULKER_ID,
        Vec3d {
            x: 12.5,
            y: 65.0,
            z: 12.5,
        },
    );
    command(
        &sender,
        &receiver,
        &mut world,
        &format!("data merge entity {uuid} {{AttachFace:5b,Invulnerable:1b}}"),
    );
    let blocked_position = position(&world, blocked);
    let mut peeks = [false; 4];
    for _ in 0..600 {
        world.tick(Vec::new());
        for (index, (id, origin)) in ids.iter().enumerate() {
            assert_eq!(
                position(&world, *id),
                *origin,
                "a usable local face avoids teleportation"
            );
            peeks[index] |= peek(&world, *id) == 30;
        }
        assert_eq!(position(&world, blocked), blocked_position);
        peeks[3] |= peek(&world, blocked) == 30;
    }
    assert_eq!(
        saved_byte(&world, ids[0].0, "AttachFace"),
        0,
        "missing east support switches to the floor"
    );
    assert_eq!(
        saved_byte(&world, ids[1].0, "AttachFace"),
        0,
        "a blocked west opening switches to the floor"
    );
    assert_eq!(
        saved_byte(&world, ids[2].0, "AttachFace"),
        5,
        "a clear east attachment stays in place"
    );
    assert_eq!(
        saved_byte(&world, blocked, "AttachFace"),
        0,
        "the happy ghast blocks the west opening"
    );
    assert!(
        peeks.into_iter().all(|opened| opened),
        "each shell opens toward its clear space"
    );
}

#[test]
fn shulker_lost_support_relocates_but_no_ai_only_changes_local_attachment() {
    let (sender, receiver, mut world) = fixture();
    command(&sender, &receiver, &mut world, "gamemode creative alice");
    let origin = Vec3d {
        x: 8.5,
        y: 65.0,
        z: 12.5,
    };
    let (id, uuid) = spawn(&mut world, VANILLA_ENTITY_TYPE_SHULKER_ID, origin);
    command(
        &sender,
        &receiver,
        &mut world,
        &format!("data merge entity {uuid} {{NoAI:1b,AttachFace:5b,Peek:30b}}"),
    );
    assert_eq!(position(&world, id), origin);
    assert_eq!(
        saved_byte(&world, id, "AttachFace"),
        0,
        "NoAI permits a new local face"
    );
    assert!(world.set_block(
        DIMENSION,
        BlockPos { x: 8, y: 64, z: 12 },
        block_state_id("minecraft:air"),
        SET_BLOCK_UPDATE_ALL
    ));
    for _ in 0..40 {
        let tick = world.tick(Vec::new());
        assert_eq!(
            position(&world, id),
            origin,
            "NoAI cannot teleport without support"
        );
        assert!(!sound(&tick, "minecraft:entity.shulker.teleport"));
    }
    let mut tick = command(
        &sender,
        &receiver,
        &mut world,
        &format!("data merge entity {uuid} {{NoAI:0b}}"),
    );
    let mut heard = false;
    let mut tracked_position = origin;
    for _ in 0..600 {
        heard |= sound(&tick, "minecraft:entity.shulker.teleport");
        for event in &tick.events {
            match event {
                WorldEvent::Send {
                    connection_id: ALICE,
                    packet: ClientboundPlayPacket::EntityPositionSync(update),
                } if update.id == id => tracked_position = update.position,
                WorldEvent::Send {
                    connection_id: ALICE,
                    packet: ClientboundPlayPacket::MoveEntity(update),
                } if update.id == id && update.has_position => {
                    tracked_position.x += f64::from(update.delta_x) / 4096.0;
                    tracked_position.y += f64::from(update.delta_y) / 4096.0;
                    tracked_position.z += f64::from(update.delta_z) / 4096.0;
                }
                _ => {}
            }
        }
        if position(&world, id) != origin && tracked_position == position(&world, id) && heard {
            break;
        }
        tick = world.tick(receiver.drain());
        keep_alive(&sender, &tick);
    }
    assert_ne!(position(&world, id), origin);
    assert!(heard, "teleport publishes its sound");
    assert_eq!(
        tracked_position,
        position(&world, id),
        "Alice receives the teleport position through the live movement packet"
    );
    assert_eq!(peek(&world, id), 0);
    let moved = position(&world, id);
    assert_eq!(
        moved.y, 65.0,
        "a floor attachment is selected instead of landing on arbitrary air"
    );
    assert_eq!(saved_byte(&world, id, "AttachFace"), 0);
}

#[test]
fn successful_low_health_damage_teleports_and_unrides_a_silent_shulker() {
    let (sender, receiver, mut world) = fixture();
    command(&sender, &receiver, &mut world, "gamemode creative alice");
    let origin = Vec3d {
        x: 8.5,
        y: 65.0,
        z: 12.5,
    };
    let (id, uuid) = spawn(&mut world, VANILLA_ENTITY_TYPE_SHULKER_ID, origin);
    let (vehicle, vehicle_uuid) = spawn(
        &mut world,
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_COW_ID,
        origin,
    );
    command(
        &sender,
        &receiver,
        &mut world,
        &format!("data merge entity {vehicle_uuid} {{NoAI:1b,NoGravity:1b}}"),
    );
    command(
        &sender,
        &receiver,
        &mut world,
        &format!("data merge entity {uuid} {{NoAI:1b,Silent:1b,Health:14.0f,Peek:30b}}"),
    );
    command(
        &sender,
        &receiver,
        &mut world,
        &format!("ride {uuid} mount {vehicle_uuid}"),
    );
    command(
        &sender,
        &receiver,
        &mut world,
        &format!("ride alice mount {uuid}"),
    );
    let riding_position = position(&world, id);
    let tick = command(
        &sender,
        &receiver,
        &mut world,
        &format!("damage {uuid} 0.1 minecraft:generic"),
    );
    assert!(!sound(&tick, "minecraft:entity.shulker.teleport"));
    assert_eq!(
        position(&world, id),
        riding_position,
        "NoAI forbids hurt teleportation"
    );
    for _ in 0..20 {
        world.tick(receiver.drain());
    }
    command(
        &sender,
        &receiver,
        &mut world,
        &format!("data merge entity {uuid} {{NoAI:0b}}"),
    );
    let mut unmounted = false;
    let mut passenger_ejected = false;
    let mut teleported = false;
    let mut dismount_origin = riding_position;
    for _ in 0..400 {
        let old_position = position(&world, id);
        let tick = command(
            &sender,
            &receiver,
            &mut world,
            &format!("damage {uuid} 0.01 minecraft:generic"),
        );
        assert!(
            !sound(&tick, "minecraft:entity.shulker.teleport"),
            "Silent suppresses the teleport sound"
        );
        unmounted |= tick.events.iter().any(|event| {
            matches!(event,
            WorldEvent::Send { packet: ClientboundPlayPacket::SetPassengers(update), .. }
            if update.vehicle_id == vehicle && !update.passenger_ids.contains(&id))
        });
        passenger_ejected |= tick.events.iter().any(|event| {
            matches!(event,
            WorldEvent::Send { connection_id: ALICE, packet: ClientboundPlayPacket::SetPassengers(update) }
            if update.vehicle_id == id && update.passenger_ids.is_empty())
        });
        if unmounted && passenger_ejected {
            teleported = true;
            dismount_origin = old_position;
            break;
        }
        for _ in 0..20 {
            let tick = world.tick(receiver.drain());
            keep_alive(&sender, &tick);
        }
    }
    assert!(
        teleported,
        "successful low-health hurt eventually teleports and breaks both riding relations"
    );
    assert_ne!(position(&world, id), riding_position);
    assert_eq!(peek(&world, id), 0);
    assert_eq!(target_of(&world, id), None);
    assert_eq!(world.external_player_vehicle_state_for_test(ALICE).2, None);
    let publication = world.publication();
    assert_eq!(
        publication.players.len(),
        1,
        "Alice stays connected through the damage window"
    );
    let landed = publication.players[0].position;
    assert_eq!((landed.x, landed.z), (dismount_origin.x, dismount_origin.z));
    assert!(
        (dismount_origin.y + 1.0..=dismount_origin.y + 2.0).contains(&landed.y),
        "the rider lands on the old shell within its physical opening range: {landed:?}"
    );
}
