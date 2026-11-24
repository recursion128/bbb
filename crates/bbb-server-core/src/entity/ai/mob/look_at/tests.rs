use bbb_protocol::entity_types::{
    VANILLA_ENTITY_TYPE_COW_ID, VANILLA_ENTITY_TYPE_HORSE_ID, VANILLA_ENTITY_TYPE_MANNEQUIN_ID,
    VANILLA_ENTITY_TYPE_VEX_ID, VANILLA_ENTITY_TYPE_VINDICATOR_ID, VANILLA_ENTITY_TYPE_ZOMBIE_ID,
};
use bbb_protocol::packets::BlockPos;
use uuid::Uuid;

use super::*;
use crate::connection::ConnectionId;
use crate::entity::ai::mob::sensing::Sensing;
use crate::entity::ai::mob::spec::{mob_spec, GoalSpec, LookAtGoalSpec};
use crate::entity::general_living::GeneralLivingData;
use crate::entity::store::{EntityIdCounter, EntityStore};
use crate::entity::tests::native_level::{state, NativeLevel};
use crate::entity::{tick_entities, EntityData};
use crate::tests::mailbox;

fn position(x: f64) -> Vec3d {
    Vec3d { x, y: 64.0, z: 0.5 }
}

fn seeded_uuid(seed: i64) -> Uuid {
    Uuid::from_u64_pair(0, seed as u64)
}

fn horse_at(store: &mut EntityStore, x: f64) -> i32 {
    store
        .restore(
            seeded_uuid(7),
            VANILLA_ENTITY_TYPE_HORSE_ID,
            position(x),
            EntityData::GeneralLiving(
                GeneralLivingData::new(VANILLA_ENTITY_TYPE_HORSE_ID).unwrap(),
            ),
        )
        .unwrap()
}

fn look_spec(entity_type_id: i32, kind: LookAtTargetKind) -> LookAtSpec {
    mob_spec(entity_type_id)
        .unwrap()
        .goals
        .iter()
        .find_map(|(_, goal)| match goal {
            GoalSpec::LookAt(LookAtGoalSpec::Standard(spec)) if spec.target_kind == kind => {
                Some(*spec)
            }
            _ => None,
        })
        .unwrap()
}

#[test]
fn mob_look_at_filters_mob_class_height_and_real_block_visibility() {
    let mut store = EntityStore::new(1, EntityIdCounter::new());
    let source_id = store
        .restore(
            seeded_uuid(1),
            VANILLA_ENTITY_TYPE_VEX_ID,
            position(0.5),
            EntityData::GeneralLiving(GeneralLivingData::new(VANILLA_ENTITY_TYPE_VEX_ID).unwrap()),
        )
        .unwrap();
    let zombie_id = store
        .restore(
            seeded_uuid(2),
            VANILLA_ENTITY_TYPE_ZOMBIE_ID,
            position(4.5),
            EntityData::GeneralLiving(
                GeneralLivingData::new(VANILLA_ENTITY_TYPE_ZOMBIE_ID).unwrap(),
            ),
        )
        .unwrap();
    store
        .spawn_general_living(VANILLA_ENTITY_TYPE_MANNEQUIN_ID, position(3.5))
        .unwrap();
    // Inside the eight-block look distance but above the three-block search
    // box, so only `canUse` refuses it.
    store
        .restore(
            seeded_uuid(3),
            VANILLA_ENTITY_TYPE_ZOMBIE_ID,
            Vec3d {
                y: position(0.5).y + 5.0,
                ..position(1.5)
            },
            EntityData::GeneralLiving(
                GeneralLivingData::new(VANILLA_ENTITY_TYPE_ZOMBIE_ID).unwrap(),
            ),
        )
        .unwrap();
    let source = store.detach(source_id).unwrap();
    let spec = look_spec(VANILLA_ENTITY_TYPE_VEX_ID, LookAtTargetKind::AnyMob);
    let origin = Vec3d {
        y: source.position().y + f64::from(source.eye_height()),
        ..source.position()
    };
    let mut sensing = Sensing::default();
    let mut native = NativeLevel::new();
    assert_eq!(
        native.with_level(|level| {
            let targets = TargetView::new(&source, &store, level, &sensing);
            nearest_look_at_target(&targets, spec.target_kind, spec.distance, origin, |_| true)
                .map(|target| target.entity_id())
        }),
        Some(zombie_id)
    );

    native.set(BlockPos { x: 2, y: 65, z: 0 }, state("minecraft:stone"));
    sensing.tick();
    assert!(native.with_level(|level| {
        let targets = TargetView::new(&source, &store, level, &sensing);
        nearest_look_at_target(&targets, spec.target_kind, spec.distance, origin, |_| true)
            .is_none()
    }));
}

#[test]
fn mob_look_at_turns_the_head_towards_a_stored_mob() {
    let mut store = EntityStore::new(1, EntityIdCounter::new());
    let source_id = store
        .restore(
            seeded_uuid(324),
            VANILLA_ENTITY_TYPE_VINDICATOR_ID,
            Vec3d {
                z: 8.5,
                ..position(8.5)
            },
            EntityData::GeneralLiving(
                GeneralLivingData::new(VANILLA_ENTITY_TYPE_VINDICATOR_ID).unwrap(),
            ),
        )
        .unwrap();
    let target_id = store
        .restore(
            seeded_uuid(2),
            VANILLA_ENTITY_TYPE_COW_ID,
            Vec3d {
                z: 8.5,
                ..position(12.5)
            },
            EntityData::GeneralLiving(GeneralLivingData::new(VANILLA_ENTITY_TYPE_COW_ID).unwrap()),
        )
        .unwrap();
    for id in [source_id, target_id] {
        store
            .get_mut(id)
            .unwrap()
            .living_mut()
            .unwrap()
            .attributes_mut()
            .get_instance(bbb_protocol::attribute_types::VANILLA_ATTRIBUTE_MOVEMENT_SPEED_ID)
            .unwrap()
            .set_base_value(0.0);
    }

    let mut native = NativeLevel::new();
    let turned = (0..2_000).any(|_| {
        native.with_level(|level| tick_entities(&mut store, level, true));
        let source = store.get(source_id).unwrap();
        let target = store.get(target_id).unwrap();
        let dx = target.position().x - source.position().x;
        let dz = target.position().z - source.position().z;
        let wanted = dz.atan2(dx).to_degrees() as f32 - 90.0;
        crate::entity::living::wrap_degrees(source.y_head_rot() - wanted).abs() < 5.0
            && crate::entity::living::wrap_degrees(source.y_head_rot()).abs() > 5.0
    });
    assert!(turned);
}

fn join_player(
    native: &mut NativeLevel,
    sender: &crate::mailbox::WorldCommandSender,
    receiver: &crate::mailbox::WorldCommandReceiver,
    connection: u64,
    x: f64,
) -> i32 {
    crate::tests::join_loaded(
        sender,
        receiver,
        &mut native.world,
        connection,
        &format!("player{connection}"),
        u128::from(connection),
    );
    let connection = ConnectionId(connection);
    native
        .world
        .set_player_position_for_test(connection, position(x));
    native.world.player_entity_id_for_test(connection)
}

#[test]
fn player_look_admits_only_the_session_players_its_conditions_accept() {
    let mut native = NativeLevel::new();
    let (sender, receiver) = mailbox();
    let near = join_player(&mut native, &sender, &receiver, 1, 3.5);
    join_player(&mut native, &sender, &receiver, 2, 2.5);
    join_player(&mut native, &sender, &receiver, 3, 1.5);
    join_player(&mut native, &sender, &receiver, 4, 20.5);
    let rider = join_player(&mut native, &sender, &receiver, 5, 0.5);
    let abilities = native.world.player_abilities_for_test(ConnectionId(2));
    native.world.set_player_mode_and_abilities_for_test(
        ConnectionId(2),
        bbb_protocol::game_types::GameType::Spectator,
        abilities,
    );
    native.world.set_player_dead_for_test(ConnectionId(3));
    let ids = EntityIdCounter::new();
    for _ in 0..rider {
        ids.next_id();
    }
    let mut store = EntityStore::new(1, ids);
    let horse = horse_at(&mut store, 0.5);
    assert!(store.attach_test_player(rider, horse, crate::entity::ExternalPlayerCanRide::Allowed));
    let source = store.detach(horse).unwrap();
    let spec = look_spec(VANILLA_ENTITY_TYPE_HORSE_ID, LookAtTargetKind::Player);
    let origin = Vec3d {
        y: source.position().y + f64::from(source.eye_height()),
        ..source.position()
    };
    let mut sensing = Sensing::default();
    assert_eq!(
        native.with_level(|level| {
            let targets = TargetView::new(&source, &store, level, &sensing);
            nearest_look_at_target(&targets, spec.target_kind, spec.distance, origin, |id| {
                !store.has_indirect_passenger(source.id(), id)
            })
            .map(|target| target.entity_id())
        }),
        Some(near)
    );
    native
        .world
        .set_player_position_for_test(ConnectionId(1), position(20.5));
    sensing.tick();
    assert!(native.with_level(|level| {
        let targets = TargetView::new(&source, &store, level, &sensing);
        nearest_look_at_target(&targets, spec.target_kind, spec.distance, origin, |id| {
            !store.has_indirect_passenger(source.id(), id)
        })
        .is_none()
    }));
}

#[test]
fn the_production_tick_turns_a_mob_head_onto_a_session_player() {
    let mut native = NativeLevel::new();
    let (sender, receiver) = mailbox();
    join_player(&mut native, &sender, &receiver, 1, 3.0);
    let rider = join_player(&mut native, &sender, &receiver, 2, -1.0);
    let ids = EntityIdCounter::new();
    for _ in 0..rider {
        ids.next_id();
    }
    let mut store = EntityStore::new(1, ids);
    let vex = store
        .restore(
            seeded_uuid(6),
            VANILLA_ENTITY_TYPE_VEX_ID,
            position(0.5),
            EntityData::GeneralLiving(GeneralLivingData::new(VANILLA_ENTITY_TYPE_VEX_ID).unwrap()),
        )
        .unwrap();
    assert!(store.attach_test_player(rider, vex, crate::entity::ExternalPlayerCanRide::Allowed));
    for _ in 0..3 {
        native.with_level(|level| tick_entities(&mut store, level, true));
    }
    assert!(store.get(vex).unwrap().y_head_rot() < 0.0);
}
