use crate::connection::{ClientboundPlayPacket, ConnectionId};
use crate::entity::tests::native_level::{NativeLevel, DIMENSION};
use crate::world::WorldEvent;

use bbb_protocol::attribute_types::VANILLA_ATTRIBUTE_FOLLOW_RANGE_ID;
use bbb_protocol::difficulties::Difficulty;
use bbb_protocol::entity_synched_data::{VANILLA_BLAZE_FLAGS_DATA_ID, VANILLA_BLAZE_FLAG_CHARGED};
use bbb_protocol::entity_types::{
    VANILLA_ENTITY_TYPE_BLAZE_ID, VANILLA_ENTITY_TYPE_IRON_GOLEM_ID,
    VANILLA_ENTITY_TYPE_SMALL_FIREBALL_ID,
};
use bbb_protocol::packets::{EntityDataValueKind, Vec3d};

use super::before_ai_step;
use crate::entity::save::write_entity_chunk;
use crate::entity::tracking::non_default_synched_values;
use crate::entity::{EntityData, EntityIdCounter, EntityStore, GeneralLivingData};

fn position() -> Vec3d {
    Vec3d {
        x: 0.5,
        y: 64.0,
        z: 0.5,
    }
}

fn blaze_store(seed: i64) -> (EntityStore, i32) {
    let mut store = EntityStore::new(seed, EntityIdCounter::new());
    store.set_difficulty(Difficulty::Normal);
    let id = store
        .spawn_general_living(VANILLA_ENTITY_TYPE_BLAZE_ID, position())
        .unwrap();
    (store, id)
}

#[test]
fn blaze_autonomous_runtime_default_save_tracking_and_falling_are_exact() {
    let (mut store, id) = blaze_store(0xB1A2E);
    let entity = store.get(id).unwrap();
    assert!(non_default_synched_values(entity)
        .iter()
        .all(|value| value.data_id != VANILLA_BLAZE_FLAGS_DATA_ID));
    let record = write_entity_chunk(&store, (0, 0)).unwrap();
    assert!(record
        .entities()
        .next()
        .unwrap()
        .raw()
        .iter()
        .all(|field| field.name != "Charged" && field.name != "charged"));

    let entity = store.get_mut(id).unwrap();
    entity.on_ground = false;
    entity.set_delta_movement(Vec3d {
        x: 0.25,
        y: -0.5,
        z: -0.25,
    });
    before_ai_step(entity);
    assert_eq!(entity.delta_movement().y, -0.3);
}

fn native_blaze(player_x: f64, no_ai: bool, follow_range: f64) -> (NativeLevel, i32) {
    native_blaze_with_mode(player_x, no_ai, follow_range, "survival")
}

fn native_blaze_with_mode(
    player_x: f64,
    no_ai: bool,
    follow_range: f64,
    mode: &str,
) -> (NativeLevel, i32) {
    let mut native = NativeLevel::new();
    let (sender, receiver) = crate::tests::mailbox();
    crate::tests::join_loaded(&sender, &receiver, &mut native.world, 1, "alice", 1);
    crate::tests::console_command(
        &sender,
        &receiver,
        &mut native.world,
        1,
        "difficulty normal",
    );
    crate::tests::console_command(
        &sender,
        &receiver,
        &mut native.world,
        2,
        &format!("gamemode {mode} alice"),
    );
    native.world.set_player_position_for_test(
        ConnectionId(1),
        Vec3d {
            x: player_x,
            y: 64.0,
            z: 8.5,
        },
    );
    let mut data = GeneralLivingData::new(VANILLA_ENTITY_TYPE_BLAZE_ID).unwrap();
    data.set_no_ai(no_ai);
    data.living_mut()
        .attributes_mut()
        .get_instance(VANILLA_ATTRIBUTE_FOLLOW_RANGE_ID)
        .unwrap()
        .set_base_value(follow_range);
    let (id, _) = native.world.spawn_stored_entity_for_test(
        DIMENSION,
        VANILLA_ENTITY_TYPE_BLAZE_ID,
        Vec3d {
            x: 8.5,
            y: 64.0,
            z: 8.5,
        },
        EntityData::GeneralLiving(data),
    );
    (native, id)
}

#[test]
fn blaze_autonomous_runtime_melee_and_no_ai_gates_are_exact() {
    let (mut native, _) = native_blaze(9.0, false, 48.0);
    for _ in 0..500 {
        native.world.tick(Vec::new());
        if native.world.player_health_for_test(ConnectionId(1)) < 20.0 {
            break;
        }
    }
    assert_eq!(native.world.player_health_for_test(ConnectionId(1)), 14.0);
    let (mut native, _) = native_blaze(9.0, true, 48.0);
    for _ in 0..100 {
        let tick = native.world.tick(Vec::new());
        assert!(!tick.events.iter().any(|event| matches!(event,
            WorldEvent::Send { packet: ClientboundPlayPacket::AddEntity(add), .. }
            | WorldEvent::Broadcast { packet: ClientboundPlayPacket::AddEntity(add), .. }
                if add.entity_type_id == VANILLA_ENTITY_TYPE_SMALL_FIREBALL_ID)));
    }
    assert_eq!(native.world.player_health_for_test(ConnectionId(1)), 20.0);
}

#[test]
fn blaze_player_search_reads_the_follow_range_attribute() {
    for (follow_range, admitted) in [(48.0, true), (32.0, false)] {
        let (mut native, id) = native_blaze(48.5, false, follow_range);
        for _ in 0..40 {
            native.world.tick(Vec::new());
        }
        assert_eq!(
            native.world.mob_target_for_test(DIMENSION, id).is_some(),
            admitted
        );
    }
}

#[test]
fn blaze_autonomous_runtime_charges_and_fires_three_projectiles() {
    let (mut native, id) = native_blaze(18.5, false, 48.0);
    let mut saw_charged = false;
    let mut saw_firing_sound = false;
    let mut projectiles = std::collections::BTreeSet::new();
    for _ in 0..600 {
        let tick = native.world.tick(Vec::new());
        let entity = native.world.stored_entity_for_test(DIMENSION, id).unwrap();
        saw_charged |= non_default_synched_values(entity).iter().any(|value| {
            value.data_id == VANILLA_BLAZE_FLAGS_DATA_ID
                && value.value == EntityDataValueKind::Byte(VANILLA_BLAZE_FLAG_CHARGED)
        });
        for event in tick.events {
            saw_firing_sound |= matches!(&event,
                WorldEvent::Send { packet: ClientboundPlayPacket::LevelEvent(event), .. }
                | WorldEvent::Broadcast { packet: ClientboundPlayPacket::LevelEvent(event), .. }
                    if event.event_type == 1018);
            if let WorldEvent::Send {
                packet: ClientboundPlayPacket::AddEntity(add),
                ..
            }
            | WorldEvent::Broadcast {
                packet: ClientboundPlayPacket::AddEntity(add),
                ..
            } = event
            {
                if add.entity_type_id == VANILLA_ENTITY_TYPE_SMALL_FIREBALL_ID {
                    projectiles.insert(add.id);
                }
            }
        }
        if projectiles.len() >= 3 {
            break;
        }
    }
    assert!(saw_charged);
    assert!(saw_firing_sound);
    assert_eq!(projectiles.len(), 3);
}

#[test]
fn blaze_shoots_fireballs_at_a_mob_target() {
    let (mut native, id) = native_blaze_with_mode(8.5, false, 48.0, "creative");
    let mut golem = GeneralLivingData::new(VANILLA_ENTITY_TYPE_IRON_GOLEM_ID).unwrap();
    golem.set_no_ai(true);
    let (golem_id, golem_uuid) = native.world.spawn_stored_entity_for_test(
        DIMENSION,
        VANILLA_ENTITY_TYPE_IRON_GOLEM_ID,
        Vec3d {
            x: 20.5,
            y: 64.0,
            z: 8.5,
        },
        EntityData::GeneralLiving(golem),
    );
    native
        .world
        .set_mob_target_for_test(DIMENSION, id, Some((golem_id, golem_uuid)));
    let fired = (0..300).any(|_| {
        native.world.tick(Vec::new()).events.iter().any(|event| {
            matches!(event,
                WorldEvent::Send { packet: ClientboundPlayPacket::AddEntity(add), .. }
                | WorldEvent::Broadcast { packet: ClientboundPlayPacket::AddEntity(add), .. }
                    if add.entity_type_id == VANILLA_ENTITY_TYPE_SMALL_FIREBALL_ID)
        })
    });
    assert!(fired);
}
