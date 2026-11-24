//! `CreakingAi`'s IDLE activity and the FIGHT `MeleeAttack` on the real tick
//! path: a protector in reach of its target deals the attack's damage on the
//! attack's own cooldown, an IDLE protector draws a stroll target and walks to
//! it, a protector that lost its target re-acquires one from its own player
//! sensor, and a frozen protector neither attacks nor is pushed.

use super::*;

use bbb_protocol::debug_subscription_types::VANILLA_DEBUG_SUBSCRIPTION_BRAINS_ID;
use bbb_protocol::entity_types::{VANILLA_ENTITY_TYPE_CHICKEN_ID, VANILLA_ENTITY_TYPE_CREAKING_ID};
use uuid::Uuid;

use super::helpers::{
    admin_command as command, creaking_fixture_with_config, creaking_lay_floor,
    creaking_move_player as move_player, creaking_place_heart, creaking_protector as protector,
    creaking_spawn_protector as spawn_protector, level_sounds,
};

const DIMENSION: Java26BuiltinDimension = Java26BuiltinDimension::Overworld;
/// Where the heart stands in the arena, which is also the `HOME_POS` the
/// protector binds to.
const HEART: BlockPos = BlockPos { x: 0, y: 65, z: 4 };
/// One block east of the watcher and inside its melee reach.
const PROTECTOR_POSITION: Vec3d = Vec3d {
    x: 4.0,
    y: 65.0,
    z: 4.5,
};
/// One block east of the protector.
const WATCHER_POSITION: Vec3d = Vec3d {
    x: 5.0,
    y: 65.0,
    z: 4.5,
};
/// `Entity.getViewVector`'s yaw for the -X direction, which is where the
/// protector stands from the watcher.
const LOOKING_WEST: f32 = 90.0;
/// The same rotation turned half around, so the watcher looks away from it.
const LOOKING_EAST: f32 = 270.0;
/// The far corridor the idle cases use, so the arena's watcher is outside every
/// sensor this brain holds (`Attributes.FOLLOW_RANGE` is 32,
/// `Creaking.java:132`).
const FAR_HEART: BlockPos = BlockPos { x: 42, y: 65, z: 4 };
const FAR_PROTECTOR_POSITION: Vec3d = Vec3d {
    x: 44.5,
    y: 65.0,
    z: 4.5,
};
fn fixture() -> (WorldCommandSender, WorldCommandReceiver, ServerWorld) {
    // Survival, because a creative session's own abilities are what
    // `Player.hurtServer` refuses a mob attack on, and
    // Normal difficulty, so a mob attack lands its own damage rather than
    // the halved-and-plus-one Easy scaling.
    creaking_fixture_with_config(
        ServerWorldConfig {
            game_type: GameType::Survival,
            difficulty: Difficulty::Normal,
            ..operator_config(&[("alice", 1)])
        },
        Vec::new(),
    )
}

fn lay_floor(world: &mut ServerWorld, xs: std::ops::RangeInclusive<i32>) {
    creaking_lay_floor(world, xs, 67);
}

fn place_heart(world: &mut ServerWorld, at: BlockPos) {
    creaking_place_heart(world, at, "awake");
}

fn spawn_creaking(world: &mut ServerWorld, position: Vec3d, uuid: Uuid) -> i32 {
    world
        .restore_stored_entity_for_test(
            DIMENSION,
            uuid,
            VANILLA_ENTITY_TYPE_CREAKING_ID,
            position,
            crate::entity::EntityData::GeneralLiving(
                crate::entity::GeneralLivingData::new(VANILLA_ENTITY_TYPE_CREAKING_ID)
                    .expect("creaking is living"),
            ),
        )
        .expect("the creaking is restored with its own uuid")
}

/// The body that stands in the protector's own box, and the UUID its `NoAI`
/// flag is merged through.
///
/// A chicken, short enough (0.7 tall) not to break a watcher's line of sight to
/// the protector's eye 2.3 above its feet, and put under `NoAI` through
/// `/data merge`, which is how a save and a command both hand the flag in
/// (`Mob.isEffectiveAi`), so only
/// the push step moves it.
fn spawn_pusher(
    sender: &WorldCommandSender,
    receiver: &WorldCommandReceiver,
    world: &mut ServerWorld,
    position: Vec3d,
    uuid: Uuid,
) -> i32 {
    let entity_id = world
        .restore_stored_entity_for_test(
            DIMENSION,
            uuid,
            VANILLA_ENTITY_TYPE_CHICKEN_ID,
            position,
            crate::entity::EntityData::GeneralLiving(
                crate::entity::GeneralLivingData::new(VANILLA_ENTITY_TYPE_CHICKEN_ID)
                    .expect("the chicken is living"),
            ),
        )
        .expect("the pusher is restored with its own uuid");
    command(
        sender,
        receiver,
        world,
        2,
        &format!("data merge entity {uuid} {{NoAI:1b}}"),
    );
    entity_id
}

/// The `ClientboundAnimatePacket` actions one tick sent for `entity_id`
/// (`LivingEntity.swing`).
fn animations(tick: &TickResult, entity_id: i32) -> Vec<u8> {
    tick.events
        .iter()
        .filter_map(|event| match event {
            WorldEvent::Send {
                packet: ClientboundPlayPacket::EntityAnimation(animation),
                ..
            } if animation.id == entity_id => Some(animation.action),
            _ => None,
        })
        .collect()
}

/// The entity events one tick broadcast for `entity_id`
/// (`ClientboundEntityEventPacket`).
fn entity_events(tick: &TickResult, entity_id: i32) -> Vec<i8> {
    tick.events
        .iter()
        .filter_map(|event| match event {
            WorldEvent::Send {
                packet: ClientboundPlayPacket::EntityEvent(entity_event),
                ..
            } if entity_event.entity_id == entity_id => Some(entity_event.event_id),
            _ => None,
        })
        .collect()
}

fn stored_delta(world: &ServerWorld, entity_id: i32) -> Vec3d {
    protector(world, entity_id).delta_movement()
}

fn stored_position(world: &ServerWorld, entity_id: i32) -> Vec3d {
    protector(world, entity_id).position()
}

/// The session's subscription plus the corridor and the heart: the shared start
/// of the cases that need a watcher.
fn arena(sender: &WorldCommandSender, receiver: &WorldCommandReceiver, world: &mut ServerWorld) {
    play(
        sender,
        receiver,
        world,
        1,
        PlayServerbound::DebugSubscriptionRequest {
            subscription_ids: vec![VANILLA_DEBUG_SUBSCRIPTION_BRAINS_ID],
        },
    );
    lay_floor(world, -2..=30);
    place_heart(world, HEART);
}

/// The far corridor and its own heart, for the cases that need no player in any
/// of the protector's sensors.
fn far_arena(
    sender: &WorldCommandSender,
    receiver: &WorldCommandReceiver,
    world: &mut ServerWorld,
) {
    play(
        sender,
        receiver,
        world,
        1,
        PlayServerbound::DebugSubscriptionRequest {
            subscription_ids: vec![VANILLA_DEBUG_SUBSCRIPTION_BRAINS_ID],
        },
    );
    lay_floor(world, 40..=60);
    place_heart(world, FAR_HEART);
}

/// `MeleeAttack` deals the creaking's own `ATTACK_DAMAGE` to the player it
/// holds as its target, on the tick the attack's own cooldown allows.
#[test]
fn a_bound_protector_in_reach_attacks_the_player_it_holds_as_its_target() {
    use crate::tests::entities::console_command_answer;

    let (sender, receiver) = mailbox();
    let mut config = operator_config(&[("alice", 1)]);
    config.game_type = GameType::Survival;
    config.difficulty = Difficulty::Normal;
    config.datapacks = datapack_config().datapacks;
    let mut world = spawn_pinned_restore(config, &restore_candidate(0, None, Vec::new()));
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    world.set_game_time_for_test(13_000);
    world.apply_clock_change(
        "minecraft:overworld",
        crate::commands::CommandClockChange::SetTotalTicks(13_000),
        &mut Vec::new(),
    );
    lay_floor(&mut world, -2..=30);
    place_heart(&mut world, HEART);
    let entity_id = spawn_protector(
        &mut world,
        PROTECTOR_POSITION,
        helpers::uuid_scanning_players_immediately(),
        HEART,
    );

    let first = move_player(
        &sender,
        &receiver,
        &mut world,
        Some(WATCHER_POSITION),
        LOOKING_WEST,
    );
    let mut observed = level_sounds(&first);
    for _ in 0..80 {
        if protector(&world, entity_id).creaking_is_active()
            && !protector(&world, entity_id).creaking_can_move()
        {
            break;
        }
        observed.extend(level_sounds(&world.tick(receiver.drain())));
    }
    assert!(observed
        .iter()
        .any(|sound| sound == "minecraft:entity.creaking.activate"));
    assert!(observed
        .iter()
        .any(|sound| sound == "minecraft:entity.creaking.freeze"));
    assert!(protector(&world, entity_id).creaking_is_active());
    assert!(!protector(&world, entity_id).creaking_can_move());

    let thaw_tick = move_player(&sender, &receiver, &mut world, None, LOOKING_EAST);
    assert!(level_sounds(&thaw_tick)
        .iter()
        .any(|sound| sound == "minecraft:entity.creaking.unfreeze"));
    assert!(protector(&world, entity_id).creaking_can_move());
    let mut attack_tick = thaw_tick;
    for _ in 0..80 {
        if world.player_health_for_test(ConnectionId(1)) < 20.0 {
            break;
        }
        attack_tick = world.tick(receiver.drain());
    }
    assert_eq!(
        set_healths(&attack_tick),
        [(ConnectionId(1), 17.0)],
        "`Mob.doHurtTarget` deals the creaking's declared attack damage"
    );
    assert!(
        level_sounds(&attack_tick)
            .iter()
            .any(|sound| sound == "minecraft:entity.creaking.attack"),
        "`Creaking.playAttackSound` (`Creaking.java:418-420`) follows a landed hit"
    );
    assert_eq!(
        animations(&attack_tick, entity_id),
        vec![0],
        "`body.swing(InteractionHand.MAIN_HAND)` sends SWING_MAIN_HAND (`:33`)"
    );
    assert_eq!(
        entity_events(&attack_tick, entity_id),
        vec![4],
        "`Creaking.doHurtTarget` broadcasts entity event 4 (`Creaking.java:147`)"
    );
    let mut health = 17.0_f32;
    let mut later_damage = None;
    for _ in 0..80 {
        let tick = world.tick(receiver.drain());
        if let Some((_, announced)) = set_healths(&tick).first().copied() {
            if announced < health {
                later_damage = Some(health - announced);
                break;
            }
            health = announced;
        }
    }
    assert_eq!(later_damage, Some(3.0));
    // The player never moved: it stays inside the protector's attack box, which
    // is why both hits landed without a walk.
    assert_eq!(protector(&world, entity_id).position(), PROTECTOR_POSITION);
    let frozen = command(&sender, &receiver, &mut world, 100, "tick freeze");
    assert_eq!(console_command_answer(&frozen).0, Some(1));
    let player = snapshot_player(&frozen, 1);
    let player_uuid = player.uuid;
    assert_eq!(player_uuid, Uuid::from_u128(1));
    assert_eq!(
        world.mob_target_for_test(DIMENSION, entity_id),
        Some(player_uuid)
    );
    let uuid = protector(&world, entity_id).uuid();
    let predicate = "{condition:'minecraft:entity_properties',entity:'this',predicate:{type:'minecraft:creaking',targeted_entity:{type:'minecraft:player'}}}";
    let positive = format!("execute as {uuid} if predicate {predicate}");
    let wrong = positive.replace("minecraft:player", "minecraft:cow");
    let inverted = format!(
        "execute as {uuid} if predicate {{condition:'minecraft:inverted',term:{predicate}}}"
    );
    let relation = format!("execute as {uuid} on target if entity @s[type=minecraft:player]");
    for (operation, text, expected) in [
        (101, &positive, Some(1)),
        (102, &wrong, None),
        (103, &inverted, None),
        (104, &relation, Some(1)),
    ] {
        let result = command(&sender, &receiver, &mut world, operation, text);
        assert_eq!(console_command_answer(&result).0, expected, "{text}");
        assert_eq!(snapshot_player(&result, 1).uuid, player_uuid);
    }
    let changed = command(
        &sender,
        &receiver,
        &mut world,
        105,
        "gamemode spectator alice",
    );
    assert_eq!(console_command_answer(&changed).0, Some(1));
    let resumed = command(&sender, &receiver, &mut world, 106, "tick unfreeze");
    assert_eq!(console_command_answer(&resumed).0, Some(0));
    let mut sounds = level_sounds(&resumed);
    for _ in 0..200 {
        if !protector(&world, entity_id).creaking_is_active() {
            break;
        }
        sounds.extend(level_sounds(&world.tick(receiver.drain())));
    }
    assert!(!protector(&world, entity_id).creaking_is_active());
    assert!(sounds
        .iter()
        .any(|sound| sound == "minecraft:entity.creaking.deactivate"));
    let frozen = command(&sender, &receiver, &mut world, 107, "tick freeze");
    assert_eq!(console_command_answer(&frozen).0, Some(1));
    assert_eq!(world.mob_target_for_test(DIMENSION, entity_id), None);
    for (operation, text, expected) in [
        (108, &positive, None),
        (109, &inverted, Some(1)),
        (110, &relation, None),
    ] {
        let result = command(&sender, &receiver, &mut world, operation, text);
        assert_eq!(console_command_answer(&result).0, expected, "{text}");
    }
}

/// A frozen protector deals no damage even with its target inside its attack
/// box, and no body inside its box can move it.
#[test]
fn a_frozen_protector_neither_attacks_nor_is_pushed() {
    let (sender, receiver, mut world) = fixture();
    arena(&sender, &receiver, &mut world);
    let entity_id = spawn_protector(
        &mut world,
        PROTECTOR_POSITION,
        helpers::uuid_scanning_players_immediately(),
        HEART,
    );

    let _ = move_player(
        &sender,
        &receiver,
        &mut world,
        Some(WATCHER_POSITION),
        LOOKING_WEST,
    );
    let freeze_tick = world.tick(receiver.drain());
    assert!(
        level_sounds(&freeze_tick)
            .iter()
            .any(|sound| sound == "minecraft:entity.creaking.freeze"),
        "the stare freezes the protector"
    );
    assert!(!protector(&world, entity_id).creaking_can_move());
    let frozen_at = stored_position(&world, entity_id);

    // A body inside the protector's own box, put there after the stare was
    // established so it cannot stand in the line of sight itself.
    let pusher_id = spawn_pusher(
        &sender,
        &receiver,
        &mut world,
        Vec3d {
            x: frozen_at.x,
            y: frozen_at.y,
            z: frozen_at.z + 0.05,
        },
        Uuid::from_u128(0xfeed),
    );

    for tick_index in 1..=6 {
        let tick = world.tick(receiver.drain());
        assert!(
            set_healths(&tick).is_empty(),
            "a frozen protector runs no FIGHT behaviour at all (tick {tick_index})"
        );
        assert!(
            level_sounds(&tick)
                .iter()
                .all(|sound| sound != "minecraft:entity.creaking.attack"),
            "a frozen protector plays no attack sound (tick {tick_index})"
        );
        assert_eq!(
            animations(&tick, entity_id),
            Vec::<u8>::new(),
            "a frozen protector never swings (tick {tick_index})"
        );
        assert_eq!(
            stored_position(&world, entity_id),
            frozen_at,
            "`Creaking.isPushable` refuses the pair and `Creaking.push` refuses the write (tick {tick_index})"
        );
        assert_eq!(
            protector(&world, entity_id).delta_movement().x,
            0.0,
            "a frozen protector carries no push impulse (tick {tick_index})"
        );
        assert_eq!(
            protector(&world, entity_id).delta_movement().z,
            0.0,
            "a frozen protector carries no push impulse (tick {tick_index})"
        );
    }
    assert_eq!(
        stored_position(&world, pusher_id),
        Vec3d {
            z: frozen_at.z + 0.05,
            ..frozen_at
        },
        "the NoAI chicken keeps its position despite the collision impulse"
    );
    assert!(stored_delta(&world, pusher_id).z > 0.0);
    assert_eq!(
        protector(&world, entity_id).creaking_attack_animation_ticks(),
        0,
        "no attack animation was ever started"
    );
}

/// The unfrozen protector takes a push while the NoAI chicken stays in place.
#[test]
fn an_unfrozen_protector_takes_the_push_the_frozen_one_refuses() {
    let (sender, receiver, mut world) = fixture();
    far_arena(&sender, &receiver, &mut world);
    let entity_id = spawn_protector(
        &mut world,
        FAR_PROTECTOR_POSITION,
        helpers::uuid_scanning_players_immediately(),
        FAR_HEART,
    );
    let pusher_id = spawn_pusher(
        &sender,
        &receiver,
        &mut world,
        Vec3d {
            z: FAR_PROTECTOR_POSITION.z + 0.05,
            ..FAR_PROTECTOR_POSITION
        },
        Uuid::from_u128(0xfeed),
    );
    assert!(protector(&world, entity_id).creaking_can_move());

    for (id, gap) in [(entity_id, 0.0), (pusher_id, 0.05)] {
        let body = world
            .stored_entity_mut_for_test(DIMENSION, id)
            .expect("both bodies are live");
        body.set_pos(Vec3d {
            z: FAR_PROTECTOR_POSITION.z + gap,
            ..FAR_PROTECTOR_POSITION
        });
        body.set_delta_movement(Vec3d::default());
    }
    let tick = world.tick(receiver.drain());
    assert!(
        set_healths(&tick).is_empty(),
        "the far arena holds no player to attack"
    );
    let protector_delta = protector(&world, entity_id).delta_movement();
    let pusher_delta = stored_delta(&world, pusher_id);
    assert!(
        protector_delta.z < 0.0,
        "the protector takes the chicken's push: {protector_delta:?}"
    );
    assert!(
        pusher_delta.z > 0.0,
        "the NoAI chicken still receives a collision impulse: {pusher_delta:?}"
    );
    assert_eq!(
        stored_position(&world, pusher_id),
        Vec3d {
            z: FAR_PROTECTOR_POSITION.z + 0.05,
            ..FAR_PROTECTOR_POSITION
        }
    );
}

fn home_navigation_arena(
    sender: &WorldCommandSender,
    receiver: &WorldCommandReceiver,
    world: &mut ServerWorld,
) {
    lay_floor(world, -2..=42);
    for x in [8.5, 16.5, 24.5, 30.5] {
        move_player(
            sender,
            receiver,
            world,
            Some(Vec3d {
                x,
                ..WATCHER_POSITION
            }),
            LOOKING_WEST,
        );
    }
    place_heart(world, HEART);
}

#[test]
fn a_protector_stops_at_home_range_while_an_unbound_creaking_keeps_chasing() {
    for bound in [true, false] {
        let (sender, receiver, mut world) = fixture();
        home_navigation_arena(&sender, &receiver, &mut world);
        let start = Vec3d {
            x: 28.5,
            ..PROTECTOR_POSITION
        };
        let uuid = helpers::uuid_scanning_players_immediately();
        let entity_id = if bound {
            spawn_protector(&mut world, start, uuid, HEART)
        } else {
            spawn_creaking(&mut world, start, uuid)
        };
        move_player(
            &sender,
            &receiver,
            &mut world,
            Some(Vec3d {
                x: 30.5,
                ..WATCHER_POSITION
            }),
            LOOKING_WEST,
        );
        world.tick(receiver.drain());
        assert!(protector(&world, entity_id).creaking_is_active());
        assert!(!protector(&world, entity_id).creaking_can_move());
        move_player(
            &sender,
            &receiver,
            &mut world,
            Some(Vec3d {
                x: 38.5,
                ..WATCHER_POSITION
            }),
            LOOKING_EAST,
        );
        for _ in 0..100 {
            world.tick(receiver.drain());
        }
        let end = stored_position(&world, entity_id);
        assert!(protector(&world, entity_id).creaking_can_move());
        if bound {
            assert!(
                end.x > start.x + 1.0,
                "the protector pursued the player: {end:?}"
            );
            assert!(
                end.x < 32.0,
                "home limits the whole body's pursuit: {end:?}"
            );
        } else {
            assert!(
                end.x > 34.0,
                "without a home it crosses the same boundary: {end:?}"
            );
        }
    }
}

#[test]
fn a_protector_outside_home_range_can_walk_back_toward_its_heart() {
    let (sender, receiver, mut world) = fixture();
    home_navigation_arena(&sender, &receiver, &mut world);
    let start = Vec3d {
        x: 33.5,
        ..PROTECTOR_POSITION
    };
    let entity_id = spawn_protector(
        &mut world,
        start,
        helpers::uuid_scanning_players_immediately(),
        HEART,
    );
    move_player(
        &sender,
        &receiver,
        &mut world,
        Some(Vec3d {
            x: 30.5,
            ..WATCHER_POSITION
        }),
        LOOKING_EAST,
    );
    world.tick(receiver.drain());
    assert!(protector(&world, entity_id).creaking_is_active());
    assert!(!protector(&world, entity_id).creaking_can_move());
    move_player(
        &sender,
        &receiver,
        &mut world,
        Some(Vec3d {
            x: 25.5,
            ..WATCHER_POSITION
        }),
        LOOKING_WEST,
    );
    for _ in 0..80 {
        world.tick(receiver.drain());
    }
    let end = stored_position(&world, entity_id);
    assert!(
        end.x < 31.0,
        "the protector can return through nodes outside the home range: {end:?}"
    );
    assert!(protector(&world, entity_id).creaking_can_move());
}
