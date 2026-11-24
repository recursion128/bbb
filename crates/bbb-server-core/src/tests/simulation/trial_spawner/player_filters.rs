use super::*;
use crate::tests::entities::{console_command, console_command_answer};

fn command(
    world: &mut ServerWorld,
    sender: &WorldCommandSender,
    receiver: &WorldCommandReceiver,
    id: u64,
    text: &str,
) -> TickResult {
    let tick = console_command(sender, receiver, world, id, text);
    assert!(
        console_command_answer(&tick)
            .0
            .is_some_and(|result| result > 0),
        "{text}: {:?}",
        console_command_answer(&tick)
    );
    tick
}

fn missing_detected_player(
    cross_level: bool,
) -> (WorldCommandSender, WorldCommandReceiver, ServerWorld) {
    let (sender, receiver, mut world) = dispensing_world(true, 100);
    world.tick(receiver.drain());
    tick_many(&receiver, &mut world, 97);
    assert_eq!(world.game_time(), 98);
    assert_eq!(trial_spawner_state(&mut world), TrialSpawnerState::Active);
    assert_eq!(
        trial_spawner_data(&mut world)
            .detected_players()
            .iter()
            .copied()
            .collect::<Vec<_>>(),
        vec![Uuid::from_u128(1)]
    );
    assert!(ominous_item_spawners(&world).is_empty());
    if cross_level {
        world.ensure_entity_transfer_full_chunk_for_test(Java26BuiltinDimension::Nether, (0, 0));
        let tick = command(
            &mut world,
            &sender,
            &receiver,
            1,
            "execute in minecraft:the_nether run tp alice 4.5 64 4.5",
        );
        let id = tick
            .events
            .iter()
            .find_map(|event| match event {
                WorldEvent::Send {
                    connection_id: ConnectionId(1),
                    packet: ClientboundPlayPacket::PlayerPosition(update),
                } => Some(update.id),
                _ => None,
            })
            .expect("actual dimension transfer positions the session");
        play(
            &sender,
            &receiver,
            &mut world,
            1,
            PlayServerbound::AcceptTeleportation { id },
        );
        assert_eq!(
            world.player_dimension(ConnectionId(1)),
            Java26BuiltinDimension::Nether
        );
        assert!(world.player_health_for_test(ConnectionId(1)) > 0.0);
    } else {
        command(&mut world, &sender, &receiver, 1, "kill alice");
        assert_eq!(world.player_health_for_test(ConnectionId(1)), 0.0);
        world.tick(receiver.drain());
    }
    assert_eq!(world.game_time(), 100);
    assert_eq!(trial_spawner_state(&mut world), TrialSpawnerState::Active);
    let data = trial_spawner_data(&mut world);
    assert!(data.detected_players().contains(&Uuid::from_u128(1)));
    assert_eq!(data.current_mobs().len(), 1);
    let (mob, mob_uuid, _) = stored_mob(&mut world, VANILLA_ENTITY_TYPE_ZOMBIE_ID);
    assert!(data.current_mobs().contains(&mob_uuid));
    assert!(world
        .stored_entity_for_test(Java26BuiltinDimension::Overworld, mob)
        .unwrap()
        .is_alive());
    assert!(ominous_item_spawners(&world).is_empty());
    assert_eq!(data.cooldown_ends_at(), 100);
    (sender, receiver, world)
}

#[test]
fn player_filters_dead_or_foreign_detected_session_refuses_items_even_with_a_live_mob() {
    for cross_level in [false, true] {
        let (_, _, mut world) = missing_detected_player(cross_level);
        world.set_game_time_for_test(101);
        world.tick_trial_spawner_for_test(Java26BuiltinDimension::Overworld, at());
        assert!(ominous_item_spawners(&world).is_empty());
        assert_eq!(trial_spawner_data(&mut world).cooldown_ends_at(), 100);
    }
}

#[test]
fn player_filters_a_local_live_session_keeps_the_original_sampling_and_cooldown() {
    for cross_level in [false, true] {
        let (sender, receiver, mut world) = missing_detected_player(cross_level);
        world
            .level_block_entity_for_test(Java26BuiltinDimension::Overworld, at())
            .and_then(BlockEntity::trial_spawner_mut)
            .unwrap()
            .state_data_mut()
            .set_cooldown_ends_at(1000);
        join_loaded(&sender, &receiver, &mut world, 2, "bob", 2);
        stand_at(
            &sender,
            &receiver,
            &mut world,
            2,
            Vec3d {
                x: 4.5,
                y: 64.0,
                z: 4.5,
            },
        );
        for _ in 0..20 {
            if trial_spawner_data(&mut world)
                .detected_players()
                .contains(&Uuid::from_u128(2))
            {
                break;
            }
            world.tick(receiver.drain());
        }
        let data = trial_spawner_data(&mut world);
        assert_eq!(data.detected_players().len(), 2);
        assert!(data.detected_players().contains(&Uuid::from_u128(2)));
        assert_eq!(data.current_mobs().len(), 1);
        assert!(ominous_item_spawners(&world).is_empty());
        let player = world
            .nearby_players_for_test(Java26BuiltinDimension::Overworld)
            .into_iter()
            .find(|player| player.uuid == Uuid::from_u128(2))
            .unwrap();
        assert!(!player.dead_or_dying && !player.spectator && !player.instabuild);
        world.set_game_time_for_test(1001);
        // A coin picks the mob or the player list; a blocked mob column spawns
        // nothing and leaves the cooldown, so the next tick tries again.
        for _ in 0..40 {
            world.tick_trial_spawner_for_test(Java26BuiltinDimension::Overworld, at());
            if !ominous_item_spawners(&world).is_empty() {
                break;
            }
        }
        let spawners = ominous_item_spawners(&world);
        assert_eq!(spawners.len(), 1);
        let spawned = spawners[0].1;
        if (spawned.x, spawned.z) == (player.position.x, player.position.z) {
            // `getBbHeight() + 2 + nextInt(4)` above the player.
            assert!((0..4).any(|offset| spawned.y
                == (player.position.y + f64::from(player.dimensions[1] + 2.0 + offset as f32))
                    .floor()
                    - 0.5));
        }
        assert_eq!(trial_spawner_data(&mut world).cooldown_ends_at(), 1161);
    }
}

/// `TrialSpawnerState.calculatePositionAbove` (`:198-205`): the cell the item
/// spawner would occupy must have an empty collision shape, so a target inside
/// a shaped block refuses the spawn without spending the cooldown.
#[test]
fn a_blocked_target_cell_refuses_the_ominous_item_spawn() {
    let (_, receiver, mut world) = dispensing_world(true, 100);
    world.tick(receiver.drain());
    tick_many(&receiver, &mut world, 3);
    tick_many(&receiver, &mut world, 40);
    assert_eq!(world.game_time(), 44);
    let (mob_id, _, mob_position) = stored_mob(&mut world, VANILLA_ENTITY_TYPE_ZOMBIE_ID);
    // The zombie would otherwise chase the player off the cell blocked below.
    set_no_ai(&mut world, mob_id);
    let mob_cell = BlockPos {
        x: mob_position.x.floor() as i32,
        y: mob_position.y.floor() as i32,
        z: mob_position.z.floor() as i32,
    };
    // Both candidate lists must refuse, because which one the boolean picks is
    // the level random's: the zombie's cell and the player's feet cell are
    // filled, and each sits on a solid block.
    assert!(world.set_block(
        Java26BuiltinDimension::Overworld,
        mob_cell,
        block_state_id("minecraft:stone"),
        SET_BLOCK_UPDATE_ALL,
    ));
    assert!(world.set_block(
        Java26BuiltinDimension::Overworld,
        BlockPos { x: 4, y: 64, z: 4 },
        block_state_id("minecraft:stone"),
        SET_BLOCK_UPDATE_ALL,
    ));
    tick_many(&receiver, &mut world, 56);
    assert_eq!(world.game_time(), 100);

    assert!(ominous_item_spawners(&world).is_empty());
    assert_eq!(
        trial_spawner_data(&mut world).cooldown_ends_at(),
        100,
        "a refused position does not spend the cooldown"
    );

    // Clearing the cells lets the same cooldown through.
    assert!(world.set_block(
        Java26BuiltinDimension::Overworld,
        mob_cell,
        block_state_id("minecraft:air"),
        SET_BLOCK_UPDATE_ALL,
    ));
    assert!(world.set_block(
        Java26BuiltinDimension::Overworld,
        BlockPos { x: 4, y: 64, z: 4 },
        block_state_id("minecraft:air"),
        SET_BLOCK_UPDATE_ALL,
    ));
    world.tick(receiver.drain());
    assert_eq!(world.game_time(), 101);
    assert_eq!(ominous_item_spawners(&world).len(), 1);
    assert_eq!(trial_spawner_data(&mut world).cooldown_ends_at(), 261);
}
