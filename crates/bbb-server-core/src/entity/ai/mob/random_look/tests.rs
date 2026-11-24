use crate::connection::ClientboundPlayPacket;
use crate::random::GameRandom;
use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_ZOMBIE_ID;
use bbb_protocol::packets::Vec3d;
use uuid::Uuid;

use super::*;
use crate::entity::general_living::GeneralLivingData;
use crate::entity::living::LivingEntityState;
use crate::entity::store::{EntityIdCounter, EntityStore};
use crate::entity::tests::native_level::NativeLevel;
use crate::entity::tracking::ServerEntity;
use crate::entity::{tick_entities, EntityData};

fn origin() -> Vec3d {
    Vec3d {
        x: 0.5,
        y: 64.0,
        z: 0.5,
    }
}

fn seeded_uuid(seed: i64) -> Uuid {
    Uuid::from_u64_pair(0, seed as u64)
}

#[test]
fn mob_random_look_base_controller_clamps_horizontal_and_vertical_rotation() {
    let living = LivingEntityState::new("minecraft:zombie").unwrap();
    let mut context = MobAiContext::from_living(
        1,
        1,
        origin(),
        65.74,
        30.0,
        &living,
        GameRandom::with_seed(0),
    );
    let mut control = LookControl::new();
    control.set_look_at(LookTarget {
        x: origin().x + 1.0,
        y: 67.74,
        z: origin().z + 1.0,
    });

    control.tick(&mut context);

    assert_eq!(context.body.y_head_rot, -10.0);
    assert_eq!(context.body.x_rot, -40.0);
}

#[test]
fn mob_random_look_no_ai_gate_prevents_goal_and_controller_work() {
    let mut data = GeneralLivingData::new(VANILLA_ENTITY_TYPE_ZOMBIE_ID).unwrap();
    data.set_no_ai(true);
    let mut store = EntityStore::new(1, EntityIdCounter::new());
    let id = store
        .restore(
            seeded_uuid(5120),
            VANILLA_ENTITY_TYPE_ZOMBIE_ID,
            origin(),
            EntityData::GeneralLiving(data),
        )
        .unwrap();

    let mut native = NativeLevel::new();
    native.with_level(|level| tick_entities(&mut store, level, true));

    let entity = store.get(id).unwrap();
    assert_eq!(entity.y_head_rot(), 0.0);
}

#[test]
fn mob_random_look_head_turn_reaches_the_existing_tracker_packet() {
    let mut store = EntityStore::new(1, EntityIdCounter::new());
    let id = store
        .restore(
            seeded_uuid(646),
            VANILLA_ENTITY_TYPE_ZOMBIE_ID,
            origin(),
            EntityData::GeneralLiving(
                GeneralLivingData::new(VANILLA_ENTITY_TYPE_ZOMBIE_ID).unwrap(),
            ),
        )
        .unwrap();
    let mut tracker = ServerEntity::new(store.get(id).unwrap(), 1, true);

    let mut native = NativeLevel::new();
    for _ in 0..100 {
        native.with_level(|level| tick_entities(&mut store, level, true));
        if store.get(id).unwrap().y_head_rot() != 0.0 {
            break;
        }
    }
    assert_ne!(store.get(id).unwrap().y_head_rot(), 0.0);
    let packets = tracker.send_changes(store.get_mut(id).unwrap());

    assert!(packets.iter().any(|packet| matches!(
        packet,
        ClientboundPlayPacket::RotateHead(head)
            if head.id == id && head.y_head_rot == store.get(id).unwrap().y_head_rot()
    )));
}

#[test]
fn dolphin_look_tilts_towards_the_target_and_preserves_swimming_pitch_until_idle() {
    let living = LivingEntityState::new("minecraft:dolphin").unwrap();
    let mut context = MobAiContext::from_living(
        1,
        1,
        origin(),
        64.3,
        30.0,
        &living,
        GameRandom::with_seed(0),
    );
    context.body.y_head_rot = 350.0;
    context.body.y_body_rot = 350.0;
    context.nav.navigation_done = false;
    let mut control = LookControl::for_entity(VANILLA_ENTITY_TYPE_DOLPHIN_ID).unwrap();
    control.set_look_at_with_limits(
        LookTarget {
            x: origin().x + 1.0,
            y: context.body.eye_y,
            z: origin().z,
        },
        10.0,
        7.0,
    );

    control.tick(&mut context);
    assert_eq!(context.body.y_head_rot, 340.0);
    assert_eq!(context.body.y_body_rot, 350.0);
    assert_eq!(context.body.x_rot, 23.0);
    control.tick(&mut context);
    assert_eq!(context.body.y_head_rot, 330.0);
    assert_eq!(context.body.y_body_rot, 346.0);
    assert_eq!(context.body.x_rot, 16.0);
    control.tick(&mut context);
    assert_eq!(context.body.x_rot, 16.0);
    context.nav.navigation_done = true;
    control.tick(&mut context);
    assert_eq!(context.body.x_rot, 11.0);
}

#[test]
fn floor_shulker_turns_its_head_toward_a_nearby_player() {
    use crate::connection::ConnectionId;
    let mut native = NativeLevel::new();
    native
        .world
        .load_entity_chunk_for_test(crate::entity::tests::native_level::DIMENSION, (0, 0))
        .unwrap();
    let (sender, receiver) = crate::tests::mailbox();
    crate::tests::join_loaded(&sender, &receiver, &mut native.world, 1, "player1", 1);
    native.world.set_player_position_for_test(
        ConnectionId(1),
        Vec3d {
            x: 10.5,
            y: 64.0,
            z: 8.5,
        },
    );
    let mut store = EntityStore::new(9, EntityIdCounter::new());
    let shulker = store
        .spawn_general_living(
            bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_SHULKER_ID,
            Vec3d {
                x: 5.5,
                y: 64.0,
                z: 8.5,
            },
        )
        .unwrap();
    let mut looked = false;
    for _ in 0..1200 {
        native.with_level(|level| {
            crate::entity::tick_one_entity(&mut store, level, shulker);
        });
        let head = store.get(shulker).unwrap().living().unwrap().y_head_rot;
        // Shulker: facing +X from a floor attachment is yaw -90.
        if crate::entity::living::wrap_degrees(head + 90.0).abs() < 1.0 {
            looked = true;
            break;
        }
    }
    assert!(looked);
}
