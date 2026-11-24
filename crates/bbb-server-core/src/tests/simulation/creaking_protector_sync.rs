//! A bound protector and a session player on the real tick: the session's
//! effect rows reach the other sessions as deltas and pairing rows, and the
//! two bodies push each other from both sides under every team collision rule.

use super::*;

use crate::entity::living::mob_effects::ActiveMobEffect;
use bbb_facts::scoreboard_team_options::TeamCollisionRule;
use bbb_protocol::entity_synched_data::{
    VANILLA_AVATAR_MODEL_CUSTOMIZATION_DATA_ID, VANILLA_LIVING_ENTITY_EFFECT_AMBIENCE_DATA_ID,
    VANILLA_LIVING_ENTITY_EFFECT_PARTICLES_DATA_ID,
};
use bbb_protocol::packets::{EntityDataValueKind, ParticlePayload};
use bbb_protocol::particle_types::VANILLA_PARTICLE_TYPE_ENTITY_EFFECT_ID;
use uuid::Uuid;

use super::helpers::{
    creaking_fixture_with_config, creaking_lay_corridor as lay_corridor, creaking_place_heart,
    creaking_protector as protector, creaking_spawn_protector as spawn_protector,
    CREAKING_HEART as HEART, CREAKING_PROTECTOR_POS as PROTECTOR_POSITION,
};

fn fixture_with_team_rules(
    teams: Vec<(Vec<String>, TeamCollisionRule)>,
) -> (WorldCommandSender, WorldCommandReceiver, ServerWorld) {
    creaking_fixture_with_config(operator_config(&[("alice", 1)]), teams)
}

fn place_heart(world: &mut ServerWorld, at: BlockPos) {
    creaking_place_heart(world, at, "awake");
}

const DIMENSION: Java26BuiltinDimension = Java26BuiltinDimension::Overworld;
const SUBJECT: u64 = 1;
const OBSERVER: u64 = 2;

/// A tenth of a block east of the protector: inside its 0.9-wide box and past
/// `Entity.push(Entity)`'s `0.01F` epsilon (`Entity.java:1788`).
const BESIDE_PROTECTOR: Vec3d = Vec3d {
    x: 3.6,
    y: 65.0,
    z: 4.5,
};

fn protector_uuid() -> Uuid {
    Uuid::from_u128(0x00c4_ea4c)
}

struct Stage {
    sender: WorldCommandSender,
    receiver: WorldCommandReceiver,
    world: ServerWorld,
    protector: i32,
    subject: i32,
}

/// The bound protector, alice beside it and bob tracking her, over a save
/// whose scoreboard carries `teams`.
fn staged(teams: Vec<(Vec<String>, TeamCollisionRule)>) -> Stage {
    let (sender, receiver, mut world) = fixture_with_team_rules(teams);
    join_loaded(&sender, &receiver, &mut world, OBSERVER, "bob", 2);
    lay_corridor(&mut world);
    place_heart(&mut world, HEART);
    let protector = spawn_protector(&mut world, PROTECTOR_POSITION, protector_uuid(), HEART);
    world.set_player_position_for_test(ConnectionId(SUBJECT), BESIDE_PROTECTOR);
    for connection_id in [SUBJECT, OBSERVER] {
        world.reset_pending_chunks_for_test(ConnectionId(connection_id), &[]);
    }
    for connection_id in [SUBJECT, OBSERVER] {
        world.reconcile_player_tracking_for_test(ConnectionId(connection_id));
    }
    world
        .stored_entity_mut_for_test(DIMENSION, protector)
        .expect("the protector is live")
        .set_delta_movement(Vec3d::default());
    let subject = entity_id_of(&world, SUBJECT);
    Stage {
        sender,
        receiver,
        world,
        protector,
        subject,
    }
}

/// `Entity.push(Entity)`'s arithmetic for a pair
/// offset along x alone: the impulse the body at `from` gives the body at
/// `from + gap`, whose `0.05F` widens before it scales the double.
fn pair_arm(gap: f64) -> f64 {
    let dd = gap.abs().sqrt();
    gap / dd * (1.0 / dd).min(1.0) * f64::from(0.05_f32)
}

/// Every `ClientboundSetEntityMotionPacket` `recipient` was sent for
/// `entity_id`.
fn motions_to(tick: &TickResult, recipient: u64, entity_id: i32) -> Vec<Vec3d> {
    tick.events
        .iter()
        .filter_map(|event| match event.packet_for(ConnectionId(recipient)) {
            Some(ClientboundPlayPacket::SetEntityMotion(motion)) if motion.id == entity_id => {
                Some(motion.delta_movement)
            }
            _ => None,
        })
        .collect()
}

/// Every `ClientboundSetEntityDataPacket` `recipient` was sent for
/// `entity_id`, as `(data id, value)` rows per packet.
fn data_to(
    tick: &TickResult,
    recipient: u64,
    entity_id: i32,
) -> Vec<Vec<(u8, EntityDataValueKind)>> {
    tick.events
        .iter()
        .filter_map(|event| match event.packet_for(ConnectionId(recipient)) {
            Some(ClientboundPlayPacket::SetEntityData(update)) if update.id == entity_id => Some(
                update
                    .values
                    .iter()
                    .map(|value| (value.data_id, value.value.clone()))
                    .collect(),
            ),
            _ => None,
        })
        .collect()
}

/// `travelInAir`'s friction for a body that has not landed: `blockFriction`
/// is `1.0F` off the ground.
fn airborne_friction() -> f64 {
    f64::from(0.91_f32)
}

/// The staged protector's x and delta around one real tick, and the motion
/// bob is told alice carries on the following tick's sweep.
struct PushTick {
    before_x: f64,
    after_x: f64,
    delta_x: f64,
    published: Vec<Vec3d>,
}

fn one_push_tick(stage: &mut Stage) -> PushTick {
    let body = protector(&stage.world, stage.protector);
    assert!(!body.on_ground(), "the staged protector has not moved yet");
    let before_x = body.position().x;
    stage.world.tick(stage.receiver.drain());
    let body = protector(&stage.world, stage.protector);
    let (after_x, delta_x) = (body.position().x, body.delta_movement().x);
    let published = stage.world.tick(stage.receiver.drain());
    PushTick {
        before_x,
        after_x,
        delta_x,
        published: motions_to(&published, OBSERVER, stage.subject),
    }
}

/// Both bodies take both arms of the pair in one tick. Alice's step runs in
/// the player half ahead of the entity loop, so the arm it gives the protector
/// is what the protector's travel moves by and
/// then scales; its own step comes after that
/// travel and reads the gap the travel widened.
fn assert_both_arms(tick: &PushTick, case: &str) {
    let session_arm = pair_arm(BESIDE_PROTECTOR.x - tick.before_x);
    assert_eq!(tick.after_x, tick.before_x + -session_arm, "{case}");
    let protector_arm = pair_arm(BESIDE_PROTECTOR.x - tick.after_x);
    assert_eq!(
        tick.delta_x,
        -session_arm * airborne_friction() + -protector_arm,
        "{case}"
    );
    assert_eq!(
        tick.published,
        vec![Vec3d {
            x: session_arm + protector_arm,
            y: 0.0,
            z: 0.0
        }],
        "{case}: `Entity.push` raised `needsSync` (`Entity.java:1822`)"
    );
}

#[test]
fn a_protector_and_a_session_push_each_other_from_both_sides_each_tick() {
    let mut stage = staged(Vec::new());
    let tick = one_push_tick(&mut stage);
    assert_both_arms(&tick, "no team");
    assert!(
        protector(&stage.world, stage.protector).is_heart_bound(),
        "the pushes leave the heart binding alone"
    );
}

/// `EntitySelector.pushableBy` on both steps of
/// the same pair. The rule reads the same from either side, so a refused pair
/// is refused by both scans and neither body moves.
#[test]
fn every_team_collision_rule_filters_both_sides_of_the_pair() {
    use TeamCollisionRule::{Always, Never, PushOtherTeams, PushOwnTeam};
    let protector_name = protector_uuid().to_string();
    let alice = "alice".to_string();
    let together = |rule| vec![(vec![protector_name.clone(), alice.clone()], rule)];
    let apart = |protector_rule, alice_rule| {
        vec![
            (vec![protector_name.clone()], protector_rule),
            (vec![alice.clone()], alice_rule),
        ]
    };
    let cases = [
        ("one team, always", together(Always), true),
        ("one team, never", together(Never), false),
        ("one team, pushOwnTeam", together(PushOwnTeam), false),
        ("one team, pushOtherTeams", together(PushOtherTeams), true),
        ("two teams, pushOwnTeam", apart(Always, PushOwnTeam), true),
        (
            "two teams, pushOtherTeams",
            apart(PushOtherTeams, Always),
            false,
        ),
        (
            "alice alone, never",
            vec![(vec![alice.clone()], Never)],
            false,
        ),
    ];
    for (case, teams, pushed) in cases {
        let mut stage = staged(teams);
        let tick = one_push_tick(&mut stage);
        if pushed {
            assert_both_arms(&tick, case);
        } else {
            assert_eq!(tick.after_x, tick.before_x, "{case}");
            assert_eq!(tick.delta_x, 0.0, "{case}");
            assert!(tick.published.is_empty(), "{case}: {:?}", tick.published);
        }
    }
}

/// A new session joining the staged world and pairing with alice: the rows
/// her `ServerEntity` pairs from.
fn pairing_rows(
    stage: &mut Stage,
    connection_id: u64,
    name: &str,
) -> Vec<(u8, EntityDataValueKind)> {
    let (joined, handshake) = join_loaded_ticks(
        &stage.sender,
        &stage.receiver,
        &mut stage.world,
        connection_id,
        name,
        u128::from(connection_id),
    );
    let rows = [&joined, &handshake]
        .into_iter()
        .flat_map(|tick| data_to(tick, connection_id, stage.subject))
        .collect::<Vec<_>>();
    assert_eq!(
        rows.len(),
        1,
        "one metadata packet in the pairing: {rows:?}"
    );
    rows.into_iter().next().expect("the pairing metadata")
}

fn particles(row: &EntityDataValueKind) -> &[ParticlePayload] {
    let EntityDataValueKind::Particles(particles) = row else {
        panic!("the effect particle row is a particle list: {row:?}")
    };
    particles
}

/// `updateDirtyEffects` settles both rows behind
/// the one `effectsDirty` gate; `ServerEntity.sendChanges` packs the dirty
/// items for the trackers and a new viewer
/// pairs from the non-default ones (`SynchedEntityData.java:92-106`).
#[test]
fn a_session_beside_a_protector_publishes_its_effect_rows_as_deltas_and_pairing() {
    let mut stage = staged(Vec::new());
    stage.world.tick(stage.receiver.drain());
    let subject = ConnectionId(SUBJECT);
    let mut events = Vec::new();

    // `onEffectAdded`: a visible non-ambient effect writes the
    // particle row and leaves the ambience item at its `false`.
    let speed = ActiveMobEffect::new("minecraft:speed", 603, 0).expect("speed");
    assert_eq!(
        stage
            .world
            .add_player_mob_effect(subject, speed, &mut events),
        Some(true)
    );
    let mut published = Vec::new();
    for _ in 0..6 {
        let tick = stage.world.tick(stage.receiver.drain());
        published.extend(data_to(&tick, OBSERVER, stage.subject));
    }
    // The add, then the `duration % 600 == 0` refresh.
    assert_eq!(published.len(), 2, "{published:?}");
    for rows in &published {
        assert_eq!(rows.len(), 1, "{rows:?}");
        assert_eq!(rows[0].0, VANILLA_LIVING_ENTITY_EFFECT_PARTICLES_DATA_ID);
        let list = particles(&rows[0].1);
        assert_eq!(list.len(), 1);
        assert_eq!(
            list[0].particle_type_id,
            VANILLA_PARTICLE_TYPE_ENTITY_EFFECT_ID
        );
        assert_eq!(list[0].raw_options[0], 255, "a non-ambient effect's alpha");
    }
    assert_eq!(
        published[0], published[1],
        "the refresh republishes the same list: `ColorParticleOption` has no `equals`"
    );

    // An ambient effect in its place makes every visible effect ambient, so
    // both items move in one packet.
    let haste = ActiveMobEffect::with_visibility("minecraft:haste", 1200, 0, true, true, true)
        .expect("haste");
    assert_eq!(
        stage
            .world
            .add_player_mob_effect(subject, haste, &mut events),
        Some(true)
    );
    assert!(stage
        .world
        .remove_player_mob_effect(subject, "minecraft:speed", &mut events)
        .is_some());
    let ambient = stage.world.tick(stage.receiver.drain());
    let rows = data_to(&ambient, OBSERVER, stage.subject);
    assert_eq!(rows.len(), 1, "{rows:?}");
    let ids: Vec<u8> = rows[0].iter().map(|(id, _)| *id).collect();
    assert_eq!(
        ids,
        [
            VANILLA_LIVING_ENTITY_EFFECT_PARTICLES_DATA_ID,
            VANILLA_LIVING_ENTITY_EFFECT_AMBIENCE_DATA_ID
        ]
    );
    assert_eq!(
        particles(&rows[0][0].1)[0].raw_options[0],
        38,
        "the ambient alpha"
    );
    assert_eq!(rows[0][1].1, EntityDataValueKind::Boolean(true));

    let carol = pairing_rows(&mut stage, 3, "carol");
    let mut expected_pairing = vec![(
        bbb_protocol::entity_synched_data::VANILLA_LIVING_ENTITY_HEALTH_DATA_ID,
        EntityDataValueKind::Float(20.0),
    )];
    expected_pairing.extend(rows[0].clone());
    expected_pairing.push((
        VANILLA_AVATAR_MODEL_CUSTOMIZATION_DATA_ID,
        EntityDataValueKind::Byte(127),
    ));
    assert_eq!(
        carol, expected_pairing,
        "a new viewer pairs effects and its configured skin parts"
    );

    // `removeEffectParticles` is the whole of the empty arm: the
    // ambience item keeps its `true`, so it stays out of the delta and in the
    // pairing.
    let removed = stage
        .world
        .remove_all_player_mob_effects(subject, &mut events);
    assert_eq!(removed.len(), 1);
    let cleared = stage.world.tick(stage.receiver.drain());
    assert_eq!(
        data_to(&cleared, OBSERVER, stage.subject),
        vec![vec![(
            VANILLA_LIVING_ENTITY_EFFECT_PARTICLES_DATA_ID,
            EntityDataValueKind::Particles(Vec::new())
        )]]
    );
    let dave = pairing_rows(&mut stage, 4, "dave");
    assert_eq!(
        dave,
        vec![
            (
                bbb_protocol::entity_synched_data::VANILLA_LIVING_ENTITY_HEALTH_DATA_ID,
                EntityDataValueKind::Float(20.0),
            ),
            (
                VANILLA_LIVING_ENTITY_EFFECT_AMBIENCE_DATA_ID,
                EntityDataValueKind::Boolean(true)
            ),
            (
                VANILLA_AVATAR_MODEL_CUSTOMIZATION_DATA_ID,
                EntityDataValueKind::Byte(127)
            ),
        ]
    );
}
