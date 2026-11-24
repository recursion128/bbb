use super::enderman_freeze::{rotate, stare_rotation, teleport};
use super::*;
use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_ENDERMAN_ID;
use bbb_protocol::item_types::VANILLA_ITEM_CARVED_PUMPKIN_ID;

fn fixture() -> (WorldCommandSender, WorldCommandReceiver, ServerWorld, i32) {
    let (sender, receiver) = mailbox();
    let mut config = datapack_config();
    config.game_type = GameType::Survival;
    config.admin_lists = operator_config(&[("alice", 1)]).admin_lists;
    let mut world = spawn_pinned_world(config);
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    install_floor(&mut world);
    teleport(
        &sender,
        &receiver,
        &mut world,
        1,
        Vec3d {
            x: 8.5,
            y: 65.0,
            z: 4.5,
        },
    );
    for x in -2..=2 {
        for z in -2..=2 {
            world.ensure_entity_transfer_full_chunk_for_test(DIMENSION, (x, z));
        }
    }
    for x in 0..=15 {
        for z in -32..0 {
            assert!(world.set_block(
                DIMENSION,
                BlockPos { x, y: 64, z },
                block_state_id("minecraft:stone"),
                SET_BLOCK_UPDATE_ALL
            ));
        }
    }
    let (enderman, _) = spawn(
        &mut world,
        VANILLA_ENTITY_TYPE_ENDERMAN_ID,
        Vec3d {
            x: 8.5,
            y: 65.0,
            z: 12.5,
        },
    );
    (sender, receiver, world, enderman)
}

fn look_tick(
    sender: &WorldCommandSender,
    receiver: &WorldCommandReceiver,
    world: &mut ServerWorld,
    enderman: i32,
) -> TickResult {
    let rotation = stare_rotation(world, enderman);
    play(
        sender,
        receiver,
        world,
        1,
        PlayServerbound::MovePlayer(MovePlayer {
            position: None,
            rotation: Some(rotation),
            on_ground: true,
            horizontal_collision: false,
        }),
    )
}

fn lock_without_damage(
    sender: &WorldCommandSender,
    receiver: &WorldCommandReceiver,
    world: &mut ServerWorld,
    enderman: i32,
) {
    assert_eq!(target_of(world, enderman), None);
    for tick in 0..8 {
        look_tick(sender, receiver, world, enderman);
        if tick < 2 {
            assert_eq!(
                target_of(world, enderman),
                None,
                "the pending stare delays acquisition"
            );
        }
        if target_of(world, enderman) == Some(Uuid::from_u128(1)) {
            return;
        }
    }
    panic!("an unharmed session player's sustained stare locks within eight ticks");
}

fn teleported(tick: &TickResult, enderman: i32) -> bool {
    tick.events.iter().any(|event| {
        matches!(event,
        WorldEvent::Send { packet: ClientboundPlayPacket::EntityEvent(event), .. }
            if event.entity_id == enderman && event.event_id == 46)
    })
}

#[test]
fn a_live_head_disguise_blocks_an_unharmed_players_delayed_stare_lock_until_removed() {
    let (sender, receiver, mut world, enderman) = fixture();
    world.set_inventory_slot_for_test(
        ConnectionId(1),
        39,
        ItemStackSummary {
            item_id: Some(VANILLA_ITEM_CARVED_PUMPKIN_ID),
            count: 1,
            component_patch: DataComponentPatchSummary::default().into(),
        },
    );
    for _ in 0..8 {
        let tick = look_tick(&sender, &receiver, &mut world, enderman);
        assert_eq!(target_of(&world, enderman), None);
        assert!(!teleported(&tick, enderman));
    }
    world.set_inventory_slot_for_test(ConnectionId(1), 39, ItemStackSummary::empty());
    lock_without_damage(&sender, &receiver, &mut world, enderman);
    assert!(
        world
            .stored_entity_for_test(DIMENSION, enderman)
            .unwrap()
            .tick_count()
            < 600
    );
}

#[test]
fn a_locked_enderman_teleports_away_from_a_close_stare_and_retains_its_player_target() {
    let (sender, receiver, mut world, enderman) = fixture();
    lock_without_damage(&sender, &receiver, &mut world, enderman);
    let position = world
        .stored_entity_for_test(DIMENSION, enderman)
        .unwrap()
        .position();
    teleport(
        &sender,
        &receiver,
        &mut world,
        2,
        Vec3d {
            z: position.z - 2.5,
            ..position
        },
    );
    for _ in 0..40 {
        let before = world
            .stored_entity_for_test(DIMENSION, enderman)
            .unwrap()
            .position();
        let tick = look_tick(&sender, &receiver, &mut world, enderman);
        if teleported(&tick, enderman) {
            let body = world.stored_entity_for_test(DIMENSION, enderman).unwrap();
            assert!(
                body.tick_count() < 600,
                "the daylight path cannot own this teleport"
            );
            assert_ne!(body.old_position(), before);
            assert_eq!(target_of(&world, enderman), Some(Uuid::from_u128(1)));
            return;
        }
    }
    panic!("the close stare teleports on a real loaded destination within forty ticks");
}

#[test]
fn a_distant_unstaring_locked_player_triggers_a_delayed_real_teleport_towards() {
    let (sender, receiver, mut world, enderman) = fixture();
    lock_without_damage(&sender, &receiver, &mut world, enderman);
    teleport(
        &sender,
        &receiver,
        &mut world,
        2,
        Vec3d {
            x: 8.5,
            y: 65.0,
            z: -24.0,
        },
    );
    rotate(
        &sender,
        &receiver,
        &mut world,
        PlayerRotation {
            y_rot: 180.0,
            x_rot: 0.0,
        },
    );
    for tick_number in 0..48 {
        let before = world
            .stored_entity_for_test(DIMENSION, enderman)
            .unwrap()
            .position();
        let tick = world.tick(Vec::new());
        assert_eq!(target_of(&world, enderman), Some(Uuid::from_u128(1)));
        let body = world.stored_entity_for_test(DIMENSION, enderman).unwrap();
        let snapped = body.old_position();
        let displacement = (snapped.x - before.x).powi(2)
            + (snapped.y - before.y).powi(2)
            + (snapped.z - before.z).powi(2);
        let has_teleport = teleported(&tick, enderman) || displacement > 64.0;
        if tick_number < 20 {
            assert!(
                !has_teleport,
                "the far teleport counter waits fifteen full goal ticks"
            );
        }
        if has_teleport {
            assert!(
                displacement > 64.0,
                "the teleport moves beyond ordinary navigation"
            );
            assert!(
                body.tick_count() < 600,
                "the daylight path cannot own this teleport"
            );
            let player = world.publication().players[0].position;
            let distance = |position: Vec3d| {
                (position.x - player.x).powi(2)
                    + (position.y - player.y).powi(2)
                    + (position.z - player.z).powi(2)
            };
            assert!(
                distance(body.old_position()) < distance(before),
                "teleportTowards closes the actual distance"
            );
            return;
        }
    }
    let body = world.stored_entity_for_test(DIMENSION, enderman).unwrap();
    panic!("no real teleportTowards within forty-eight ticks; body {:?}, old {:?}, ticks {}, player {:?}",
        body.position(), body.old_position(), body.tick_count(), world.publication().players[0].position);
}
