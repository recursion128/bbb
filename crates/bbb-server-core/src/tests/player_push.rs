//! `LivingEntity.pushEntities` with a session player as the pusher, on the
//! real level tick: the stored bodies its own box holds are pushed out of it,
//! and a crowd above `MAX_ENTITY_CRAMMING` charges the session itself.
//!
//! The crowd is parked minecarts: they are pushable (`AbstractMinecart.java:
//! 168-170`) but a parked cart only ever pushes other carts
//! (`OldMinecartBehavior.java:382-388`), and a cart turned across the line
//! between two carts does not couple with the other one
//! (`AbstractMinecart.java:541-543`), so the pile never answers back.

use super::*;

/// A cart this far east of the session overlaps its box on every axis and is
/// still far enough away for `dd >= 1.0E-4F` (`AbstractMinecart.java:503`) to
/// give it an impulse.
const PUSHED_OFFSET_X: f64 = 0.2;

/// How many carts sit on the session's own position: `AbstractMinecart.push`
/// measures horizontal offsets only, so a cart at the same `x` and `z` gets
/// `dd == 0` and is never pushed -- which is what keeps the box above the rule
/// while the pushed carts leave it.
const STACKED_CARTS: usize = 24;

/// How many carts start one [`PUSHED_OFFSET_X`] east of it, which is the part
/// of the crowd the push acts on.
const PUSHED_CARTS: usize = 4;

/// How many ticks the charge is given: its draw is `nextInt(4) == 0`
/// (`LivingEntity.java:3161`).
const CRAMMING_TICKS: usize = 40;

/// A parked minecart at `position`, turned along `z` so the carts of the crowd,
/// which differ along `x`, never couple (`AbstractMinecart.java:541-543`).
fn spawn_cart(world: &mut ServerWorld, position: Vec3d) -> i32 {
    let (id, _) = world.spawn_stored_entity_for_test(
        Java26BuiltinDimension::Overworld,
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_MINECART_ID,
        position,
        crate::entity::EntityData::Minecart(crate::entity::minecart::MinecartData::new()),
    );
    world
        .stored_entity_mut_for_test(Java26BuiltinDimension::Overworld, id)
        .expect("the cart is stored")
        .set_rot(90.0, 0.0);
    id
}

/// A survival operator, which is what a hurt needs on both sides: the level to
/// run `/gamerule` and a session whose `GameType` leaves
/// `Abilities.invulnerable` unset (`GameType.java:72-76`).
fn survival_operator_config(operators: &[(&str, u128)]) -> ServerWorldConfig {
    ServerWorldConfig {
        game_type: GameType::Survival,
        ..operator_config(operators)
    }
}

/// A session standing still with `STACKED_CARTS` parked minecarts on its own
/// position and `PUSHED_CARTS` more just east of it inside the same box.
///
/// Survival, because `Player.hurtServer`'s own arm refuses a source to a
/// session whose `Abilities.invulnerable` is set, which is every creative one
/// (`GameType.java:72-76`); the push itself is not gated on the game type.
fn crowded_world(
    rule: Option<i32>,
) -> (
    WorldCommandSender,
    WorldCommandReceiver,
    ServerWorld,
    Vec<i32>,
    Vec<i32>,
) {
    let (sender, receiver) = mailbox();
    let mut world = ServerWorld::new(survival_operator_config(&[("alice", 1)])).unwrap();
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    world.tick(receiver.drain());
    if let Some(value) = rule {
        chat_command(
            &sender,
            &receiver,
            &mut world,
            1,
            &format!("gamerule max_entity_cramming {value}"),
        );
        assert_eq!(
            world.game_rule_for_test("minecraft:max_entity_cramming"),
            Some(bbb_protocol::game_rules::GameRuleValue::Integer(value)),
            "`/gamerule` writes the rule table the session push reads"
        );
    }
    let spot = world.publication().players[0].position;
    let stacked = (0..STACKED_CARTS)
        .map(|_| spawn_cart(&mut world, spot))
        .collect();
    let pushed = carts_east_of(&mut world, spot);
    (sender, receiver, world, stacked, pushed)
}

fn session_position(world: &ServerWorld) -> Vec3d {
    world.publication().players[0].position
}

/// The horizontal distance from the session's own position, which is the gap
/// `Entity.push(Entity)` (`Entity.java:1785-1786`) measures.
fn gap(world: &ServerWorld, entity_id: i32) -> f64 {
    let spot = session_position(world);
    let body = world
        .stored_entity_for_test(Java26BuiltinDimension::Overworld, entity_id)
        .expect("the cart is stored")
        .position();
    let x = body.x - spot.x;
    let z = body.z - spot.z;
    (x * x + z * z).sqrt()
}

fn delta_x(world: &ServerWorld, entity_id: i32) -> f64 {
    world
        .stored_entity_for_test(Java26BuiltinDimension::Overworld, entity_id)
        .expect("the cart is stored")
        .delta_movement()
        .x
}

/// Ticks until the session's health leaves `before`, or `CRAMMING_TICKS` of
/// them. Answers `(ticks, health)`, counting from the caller's own baseline
/// rather than from a fresh read: the charge draw is `nextInt(4) == 0`
/// (`LivingEntity.java:3161`), so it can already have landed on a tick the
/// caller ran itself, and `0` is that answer. Re-reading here instead would
/// fold the charge into the baseline and return the first regeneration heal.
fn ticks_until_charged(
    receiver: &WorldCommandReceiver,
    world: &mut ServerWorld,
    before: f32,
) -> (usize, f32) {
    let held = world.player_health_for_test(ConnectionId(1));
    if held != before {
        return (0, held);
    }
    for tick in 1..=CRAMMING_TICKS {
        world.tick(receiver.drain());
        let after = world.player_health_for_test(ConnectionId(1));
        if after != before {
            return (tick, after);
        }
    }
    (CRAMMING_TICKS, before)
}

#[test]
fn a_crowd_in_a_sessions_box_is_pushed_out_of_it_and_charges_the_session() {
    let (_sender, receiver, mut world, stacked, pushed) = crowded_world(None);
    let before: Vec<f64> = pushed.iter().map(|id| gap(&world, *id)).collect();
    assert!(
        before.iter().all(|gap| *gap > 0.01),
        "the pushed carts start inside the box and off the session: {before:?}"
    );
    let health = world.player_health_for_test(ConnectionId(1));

    world.tick(receiver.drain());

    // `this.push(-xa, 0.0, -za)` (`AbstractMinecart.java:519`) with `xa` the
    // session's offset from the cart, so a cart east of it is written further
    // east.
    for id in &pushed {
        assert!(
            delta_x(&world, *id) > 0.0,
            "the cart east of the session is pushed further east: {}",
            delta_x(&world, *id)
        );
    }
    for (index, id) in pushed.iter().enumerate() {
        assert!(
            gap(&world, *id) > before[index],
            "the cart leaves the session's box: {} -> {}",
            before[index],
            gap(&world, *id)
        );
    }
    // The stacked half is inside the same box and gets no impulse at all,
    // which is what keeps the count above the rule.
    for id in &stacked {
        assert_eq!(delta_x(&world, *id), 0.0);
    }

    let (tick, after) = ticks_until_charged(&receiver, &mut world, health);
    assert!(
        after != health,
        "24 carts in the box must charge the session"
    );
    assert_eq!(
        health - after,
        6.0,
        "`hurtServer(serverLevel, damageSources().cramming(), 6.0F)` at tick {tick}"
    );
    assert_eq!(
        after, 14.0,
        "a session with no armor takes the whole charge"
    );
}

#[test]
fn a_live_rule_above_the_crowd_leaves_the_session_alone() {
    // The live rule, not the registered 24: the same 28 carts are below 40.
    let (_sender, receiver, mut world, _stacked, pushed) = crowded_world(Some(40));
    let before = world.player_health_for_test(ConnectionId(1));
    world.tick(receiver.drain());
    // The push is not gated on the cramming rule; only the charge is.
    assert!(
        pushed.iter().all(|id| delta_x(&world, *id) > 0.0),
        "a raised rule does not stop the carts being pushed"
    );
    let (_, after) = ticks_until_charged(&receiver, &mut world, before);
    assert_eq!(after, before, "28 carts are below a rule of 40");
    assert_eq!(
        world.game_rule_for_test("minecraft:max_entity_cramming"),
        Some(bbb_protocol::game_rules::GameRuleValue::Integer(40))
    );

    // The control for the same crowd: the registered rule charges it.
    let (_sender, receiver, mut world, _stacked, _pushed) = crowded_world(None);
    let before = world.player_health_for_test(ConnectionId(1));
    let (_, after) = ticks_until_charged(&receiver, &mut world, before);
    assert_eq!(after, before - 6.0, "the same crowd charges at 24");
}

#[test]
fn a_frozen_world_pushes_a_sessions_crowd_nowhere_and_charges_it_nothing() {
    // `ServerPlayer.pushEntities` (`ServerPlayer.java:1297-1301`) is the one
    // arm of the player half `tickRateManager().runsNormally()` gates; the
    // pickup arm beside it carries no gate at all.
    let (_sender, receiver, mut world, stacked, pushed) = crowded_world(None);
    world.set_tick_frozen(true, &mut Vec::new());
    // `TickRateManager.tick` (`TickRateManager.java:56-61`) recomputes the flag
    // at the head of the next tick, which is the tick the gate then reads.
    world.tick(receiver.drain());
    assert!(
        !world.tick_rate().runs_normally(),
        "`/tick freeze` is what makes the gate false"
    );
    let before = world.player_health_for_test(ConnectionId(1));
    for _ in 0..CRAMMING_TICKS {
        world.tick(receiver.drain());
        assert_eq!(world.player_health_for_test(ConnectionId(1)), before);
    }
    for id in stacked.iter().chain(pushed.iter()) {
        assert_eq!(delta_x(&world, *id), 0.0, "a frozen world runs no push");
    }

    // The same world unfrozen: the push and the box it counts are back.
    world.set_tick_frozen(false, &mut Vec::new());
    world.tick(receiver.drain());
    assert!(world.tick_rate().runs_normally());
    world.tick(receiver.drain());
    assert!(
        pushed.iter().all(|id| delta_x(&world, *id) > 0.0),
        "unfreezing resumes the arm"
    );
}

#[test]
fn a_lone_body_outside_the_box_is_neither_pushed_nor_charged() {
    let (sender, receiver) = mailbox();
    let mut world = ServerWorld::new(survival_operator_config(&[("alice", 1)])).unwrap();
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    world.tick(receiver.drain());
    let spot = session_position(&world);
    // Three blocks east: outside the session's own 0.6-wide box, and outside
    // every inflated one too, because `getPushableEntities` takes the bare box.
    let id = spawn_cart(
        &mut world,
        Vec3d {
            x: spot.x + 3.0,
            y: spot.y,
            z: spot.z,
        },
    );
    let before_gap = gap(&world, id);
    let before = world.player_health_for_test(ConnectionId(1));
    assert_eq!(before, 20.0);
    for _ in 0..CRAMMING_TICKS {
        world.tick(receiver.drain());
        assert_eq!(
            delta_x(&world, id),
            0.0,
            "no push reaches a body outside the box"
        );
        assert_eq!(world.player_health_for_test(ConnectionId(1)), before);
    }
    assert_eq!(
        gap(&world, id),
        before_gap,
        "the cart is never moved by the session"
    );
}

/// Where one session is, which the publication cannot say once a second one
/// has joined.
fn spot_of(world: &ServerWorld, connection_id: u64) -> Vec3d {
    world
        .player_movement_validation_for_test(ConnectionId(connection_id))
        .0
}

/// `PUSHED_CARTS` carts one [`PUSHED_OFFSET_X`] east of `spot`, which is the
/// crowd without the cramming arm: four bodies in the box and a rule of 24.
fn carts_east_of(world: &mut ServerWorld, spot: Vec3d) -> Vec<i32> {
    (0..PUSHED_CARTS)
        .map(|_| {
            spawn_cart(
                world,
                Vec3d {
                    x: spot.x + PUSHED_OFFSET_X,
                    y: spot.y,
                    z: spot.z,
                },
            )
        })
        .collect()
}

/// `Entity.push(Entity)`'s own arithmetic (`Entity.java:1785-1800`) for a pair
/// of sessions offset along `x` alone, answered as the arm the pusher takes
/// (`:1805-1807`).
fn session_pair_arm(spot: Vec3d) -> f64 {
    let raw = spot.x - (spot.x + PUSHED_OFFSET_X);
    let dd = raw.abs().sqrt();
    raw / dd * (1.0 / dd).min(1.0) * f64::from(0.05_f32)
}

/// `AbstractMinecart.push(Entity)`'s arithmetic (`AbstractMinecart.java:500-520`)
/// with the session as the entity, which takes a quarter of the impulse, summed
/// over the pushed carts the way one walk sums it.
fn crowd_pusher_arm(spot: Vec3d) -> f64 {
    let raw = spot.x - (spot.x + PUSHED_OFFSET_X);
    let dd = (raw * raw).sqrt();
    let one = raw / dd * (1.0 / dd).min(1.0) * f64::from(0.1_f32) * 0.5 / 4.0;
    let mut summed = 0.0;
    for _ in 0..PUSHED_CARTS {
        summed += one;
    }
    summed
}

/// Every `ClientboundSetEntityMotionPacket` one connection was sent for one
/// entity this tick (`ServerEntity.java:172-188`).
fn motions_for(tick: &TickResult, recipient: ConnectionId, entity_id: i32) -> Vec<Vec3d> {
    tick.events
        .iter()
        .filter_map(|event| match event.packet_for(recipient) {
            Some(ClientboundPlayPacket::SetEntityMotion(motion)) if motion.id == entity_id => {
                Some(motion.delta_movement)
            }
            _ => None,
        })
        .collect()
}

/// A joined observer standing clear of the crowd and well inside the player
/// tracking range, which is 32 (`EntityType.java:1163-1173`).
fn watching_world() -> (WorldCommandSender, WorldCommandReceiver, ServerWorld, i32) {
    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_world(survival_operator_config(&[("alice", 1)]));
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    join_loaded(&sender, &receiver, &mut world, 2, "bob", 2);
    let spot = spot_of(&world, 1);
    world.set_player_position_for_test(
        ConnectionId(2),
        Vec3d {
            x: spot.x + 5.0,
            y: spot.y,
            z: spot.z,
        },
    );
    for connection_id in [1, 2] {
        world.reset_pending_chunks_for_test(ConnectionId(connection_id), &[]);
    }
    for connection_id in [1, 2] {
        world.reconcile_player_tracking_for_test(ConnectionId(connection_id));
    }
    let alice = entity_id_of(&world, 1);
    (sender, receiver, world, alice)
}

/// `Entity.push(Entity)` writes both sides of every pair (`:1801-1807`), and
/// the session's own side is the one this world had nowhere to keep: it lands
/// in `Entity.deltaMovement` and `ServerEntity.sendChanges` publishes it
/// because `Entity.push` raised `needsSync` (`Entity.java:1822`,
/// `ServerEntity.java:120`, `:172-188`).
///
/// The broadcast is one tick behind the push because `ChunkMap.tick` runs
/// ahead of the entity list that holds `aiStep` (`ServerLevel.java:387`
/// reaching `ServerChunkCache.java:329`, before `ServerLevel.java:412-434`),
/// which is the order this world keeps too.
#[test]
fn a_pushed_session_publishes_the_motion_its_own_arm_wrote() {
    let (_sender, receiver, mut world, alice) = watching_world();
    let spot = spot_of(&world, 1);
    let carts = carts_east_of(&mut world, spot);

    world.tick(receiver.drain());
    assert!(
        carts.iter().all(|id| delta_x(&world, *id) > 0.0),
        "the carts took the other arm of the same pairs"
    );

    let published = world.tick(receiver.drain());
    assert_eq!(
        motions_for(&published, ConnectionId(2), alice),
        vec![Vec3d {
            x: crowd_pusher_arm(spot),
            y: 0.0,
            z: 0.0
        }],
        "the observer is told the velocity the crowd left on the session"
    );
}

/// `ServerEntity.sendChanges`' motion arm is `needsSync || trackDelta ||
/// isFallFlying` (`ServerEntity.java:172`) and `EntityType.PLAYER` carries no
/// delta tracking (`EntityType.java:1556-1567`), so a session nothing pushed
/// publishes nothing and the bit is down again once the sweep has read it
/// (`:216`).
#[test]
fn an_unpushed_session_publishes_no_motion_and_the_bit_falls_again() {
    let (_sender, receiver, mut world, alice) = watching_world();
    let quiet = world.tick(receiver.drain());
    assert!(motions_for(&quiet, ConnectionId(2), alice).is_empty());

    let spot = spot_of(&world, 1);
    let carts = carts_east_of(&mut world, spot);
    world.tick(receiver.drain());
    for id in &carts {
        assert!(world.remove_stored_entity_for_test(
            Java26BuiltinDimension::Overworld,
            *id,
            crate::entity::RemovalReason::Discarded,
        ));
    }

    let published = world.tick(receiver.drain());
    assert_eq!(
        published
            .events
            .iter()
            .filter(|event| matches!(
                event,
                WorldEvent::Send {
                    connection_id,
                    packet: ClientboundPlayPacket::SetEntityMotion(motion),
                } if *connection_id == ConnectionId(2) && motion.id == alice
            ))
            .count(),
        1
    );

    let after = world.tick(receiver.drain());
    assert!(
        motions_for(&after, ConnectionId(2), alice).is_empty(),
        "an empty box leaves the bit down and the session's own delta at zero"
    );
}

/// `ServerGamePacketListenerImpl.handleMovePlayer`'s speed judgement
/// (`:1083-1103`) subtracts the distance the server's own simulation put on
/// the session, and a push is the one thing this world puts any there with.
#[test]
fn the_impulse_a_push_left_is_the_distance_the_move_judgement_expects() {
    // `movedDist - expectedDist > metersPerTick * deltaPackets` (`:1103`) with
    // `metersPerTick` 100 and one packet: a squared distance this far past the
    // rule is sent back on its own and allowed once the impulse comes off it.
    const JUST_PAST_THE_RULE: f64 = 100.002;
    const WELL_PAST_THE_RULE: f64 = 100.4;

    let judged = |squared: f64| {
        let (sender, receiver) = mailbox();
        let mut world = spawn_pinned_world(survival_operator_config(&[("alice", 1)]));
        join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
        let spot = spot_of(&world, 1);
        carts_east_of(&mut world, spot);
        world.tick(receiver.drain());
        let target = Vec3d {
            x: spot.x + squared.sqrt(),
            y: spot.y,
            z: spot.z,
        };
        play(
            &sender,
            &receiver,
            &mut world,
            1,
            PlayServerbound::MovePlayer(MovePlayer {
                position: Some(target),
                rotation: None,
                on_ground: true,
                horizontal_collision: false,
            }),
        );
        let arm = crowd_pusher_arm(spot);
        (arm * arm, spot_of(&world, 1) == target)
    };

    let (expected, accepted) = judged(JUST_PAST_THE_RULE);
    assert!(
        expected > JUST_PAST_THE_RULE - 100.0 && expected < WELL_PAST_THE_RULE - 100.0,
        "the crowd covers the first overshoot and not the second: {expected}"
    );
    assert!(
        accepted,
        "`expectedDist` takes the impulse off a move just past the rule"
    );
    assert!(
        !judged(WELL_PAST_THE_RULE).1,
        "a move past what the impulse covers is still sent back"
    );
}

/// Two survival sessions half overlapping and a third watching from outside
/// both boxes, with every tracker reconciled.
fn overlapping_sessions_world() -> (
    WorldCommandSender,
    WorldCommandReceiver,
    ServerWorld,
    i32,
    i32,
) {
    let (sender, receiver) = mailbox();
    let mut world = spawn_pinned_world(survival_operator_config(&[("alice", 1)]));
    join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
    join_loaded(&sender, &receiver, &mut world, 2, "bob", 2);
    join_loaded(&sender, &receiver, &mut world, 3, "carol", 3);
    let spot = spot_of(&world, 1);
    for (connection_id, dx) in [(2, PUSHED_OFFSET_X), (3, 5.0)] {
        world.set_player_position_for_test(
            ConnectionId(connection_id),
            Vec3d {
                x: spot.x + dx,
                ..spot
            },
        );
    }
    for connection_id in [1, 2, 3] {
        world.reset_pending_chunks_for_test(ConnectionId(connection_id), &[]);
    }
    for connection_id in [1, 2, 3] {
        world.reconcile_player_tracking_for_test(ConnectionId(connection_id));
    }
    let (alice, bob) = (entity_id_of(&world, 1), entity_id_of(&world, 2));
    (sender, receiver, world, alice, bob)
}

/// What the observer is told about each of the two after one push tick: each
/// session's `pushEntities` pushes the other (`LivingEntity.java:3156-3180`),
/// and `Entity.push(Entity)` writes both halves of every pair
/// (`Entity.java:1782-1811`), so each ends up with two arms.
fn observed_push(
    world: &mut ServerWorld,
    receiver: &WorldCommandReceiver,
) -> (Vec<Vec3d>, Vec<Vec3d>) {
    world.tick(receiver.drain());
    let published = world.tick(receiver.drain());
    let (alice, bob) = (entity_id_of(world, 1), entity_id_of(world, 2));
    (
        motions_for(&published, ConnectionId(3), alice),
        motions_for(&published, ConnectionId(3), bob),
    )
}

#[test]
fn two_overlapping_sessions_push_each_other_apart() {
    let (_sender, receiver, mut world, _, _) = overlapping_sessions_world();
    let one_pair = session_pair_arm(spot_of(&world, 1));

    let (alice, bob) = observed_push(&mut world, &receiver);
    assert_eq!(alice.len(), 1);
    assert_eq!(bob.len(), 1);
    assert!(
        (alice[0].x - 2.0 * one_pair).abs() < 1e-12,
        "alice is pushed west: {alice:?}"
    );
    assert!(
        (bob[0].x + 2.0 * one_pair).abs() < 1e-12,
        "bob is pushed east: {bob:?}"
    );
    assert_eq!((alice[0].z, bob[0].z), (0.0, 0.0));
}

/// `EntitySelector.pushableBy` (`EntitySelector.java:29-56`) with both
/// sessions on one team.
#[test]
fn teammates_push_each_other_as_their_teams_collision_rule_says() {
    for (rule, pushes) in [
        ("always", true),
        ("never", false),
        ("pushOwnTeam", false),
        ("pushOtherTeams", true),
    ] {
        let (sender, receiver, mut world, _, _) = overlapping_sessions_world();
        // Bob steps out of alice's box while the team is set up, so the push
        // below is the first one the observer is told about.
        let spot = spot_of(&world, 1);
        let at = |dx: f64| Vec3d {
            x: spot.x + dx,
            ..spot
        };
        world.set_player_position_for_test(ConnectionId(2), at(3.0));
        for command in [
            "team add red".to_owned(),
            "team join red alice".to_owned(),
            "team join red bob".to_owned(),
            format!("team modify red collisionRule {rule}"),
        ] {
            chat_command(&sender, &receiver, &mut world, 1, &command);
        }
        world.tick(receiver.drain());
        world.set_player_position_for_test(ConnectionId(2), at(PUSHED_OFFSET_X));
        let (alice, bob) = observed_push(&mut world, &receiver);
        let pushed = |motions: &[Vec3d]| motions.iter().any(|motion| motion.x != 0.0);
        assert_eq!(
            (pushed(&alice), pushed(&bob)),
            (pushes, pushes),
            "rule {rule}"
        );
    }
}
