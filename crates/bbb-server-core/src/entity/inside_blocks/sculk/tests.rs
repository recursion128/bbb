use crate::connection::ConnectionId;
use crate::entity::tests::native_level::{NativeLevel, DIMENSION};
use bbb_protocol::block_state_types::vanilla_block_default_state_id;
use bbb_protocol::block_types::vanilla_block_type_for_resource_id;
use bbb_protocol::entity_types::{VANILLA_ENTITY_TYPE_PIG_ID, VANILLA_ENTITY_TYPE_WARDEN_ID};
use bbb_protocol::packets::Vec3d;

use super::*;
use crate::entity::{EntityData, EntityIdCounter};

const SUPPORT: BlockPos = BlockPos { x: 0, y: 63, z: 0 };
const POSITION: Vec3d = Vec3d {
    x: 0.5,
    y: 63.5,
    z: 0.5,
};

fn state(resource: &str) -> i32 {
    vanilla_block_default_state_id(vanilla_block_type_for_resource_id(resource).unwrap().id)
        .unwrap()
}

fn store() -> EntityStore {
    EntityStore::new(0x5C_01, EntityIdCounter::new())
}

fn living(store: &mut EntityStore, kind: i32) -> i32 {
    store.spawn_general_living(kind, POSITION).unwrap()
}

fn orb(store: &mut EntityStore) -> i32 {
    crate::entity::experience_orb::spawn_experience_orb(store, POSITION, 1).unwrap()
}

fn arrow(store: &mut EntityStore, owner: Option<Uuid>) -> Box<Entity> {
    let id = crate::entity::arrow::spawn_arrow(
        store,
        POSITION,
        Vec3d::default(),
        crate::entity::item::plain_stack("minecraft:arrow", 1).unwrap(),
    )
    .unwrap();
    let mut entity = store.detach(id).unwrap();
    let EntityData::Arrow(data) = &mut entity.data else {
        panic!("arrow")
    };
    data.projectile.set_owner(owner);
    entity
}

fn level(block: &str) -> NativeLevel {
    let mut native = NativeLevel::new();
    native.set(SUPPORT, state(block));
    native
}

fn candidate(native: &NativeLevel) -> Option<crate::block_entity::vibration::VibrationInfo> {
    let sensor = native
        .world
        .get_block_entity(DIMENSION, SUPPORT)
        .unwrap()
        .sculk_sensor()
        .unwrap();
    sensor
        .vibration_data()
        .selection_strategy()
        .current()
        .map(|(info, _)| info)
        .or_else(|| sensor.vibration_data().current_vibration().cloned())
}

fn observed_owner(
    entity: &Entity,
    store: &mut EntityStore,
    native: &mut NativeLevel,
) -> Option<Uuid> {
    let state = native.world.block_state(DIMENSION, SUPPORT).unwrap();
    native
        .with_level(|level| step_on_sensor(entity, Some(store), level, state, SUPPORT))
        .unwrap();
    let event = candidate(native).expect("actual step schedules a saved vibration");
    assert_eq!(event.source, Some(entity.uuid()));
    assert_eq!(event.pos, entity.position());
    event.projectile_owner
}

#[test]
fn sculk_step_contact_living_and_nonliving_preserve_source_in_saved_vibration() {
    for block in [
        "minecraft:sculk_sensor",
        "minecraft:calibrated_sculk_sensor",
    ] {
        for living_body in [false, true] {
            let mut store = store();
            let id = if living_body {
                living(&mut store, VANILLA_ENTITY_TYPE_PIG_ID)
            } else {
                orb(&mut store)
            };
            let entity = store.detach(id).unwrap();
            let mut native = level(block);
            assert_eq!(observed_owner(&entity, &mut store, &mut native), None);
            let event = candidate(&native).unwrap();
            assert_eq!(event.game_event, "minecraft:step");
        }
    }
}

#[test]
fn sculk_step_contact_warden_does_not_activate_the_sensor() {
    let mut store = store();
    let id = living(&mut store, VANILLA_ENTITY_TYPE_WARDEN_ID);
    let entity = store.detach(id).unwrap();
    let mut native = level("minecraft:sculk_sensor");
    let state = native.world.block_state(DIMENSION, SUPPORT).unwrap();
    native
        .with_level(|level| step_on_sensor(&entity, Some(&mut store), level, state, SUPPORT))
        .unwrap();
    assert!(candidate(&native).is_none());
    assert_eq!(native.world.block_state(DIMENSION, SUPPORT), Some(state));
}

#[test]
fn sculk_projectile_owner_accepts_nonliving_and_dead_unremoved_stored_entities() {
    for is_living in [false, true] {
        let mut store = store();
        let owner = if is_living {
            living(&mut store, VANILLA_ENTITY_TYPE_PIG_ID)
        } else {
            orb(&mut store)
        };
        if is_living {
            crate::entity::living::set_health(store.get_mut(owner).unwrap(), 0.0);
        }
        let uuid = store.get(owner).unwrap().uuid();
        let entity = arrow(&mut store, Some(uuid));
        let mut level = level("minecraft:sculk_sensor");
        assert_eq!(observed_owner(&entity, &mut store, &mut level), Some(uuid));
    }
}

#[test]
fn sculk_projectile_removed_owner_is_not_kept_in_saved_vibration() {
    let mut store = store();
    let id = living(&mut store, VANILLA_ENTITY_TYPE_PIG_ID);
    let uuid = store.get(id).unwrap().uuid();
    let entity = arrow(&mut store, Some(uuid));
    store
        .remove(id, crate::entity::RemovalReason::Discarded)
        .unwrap();
    let mut native = level("minecraft:sculk_sensor");
    assert_eq!(observed_owner(&entity, &mut store, &mut native), None);
}

#[test]
fn sculk_projectile_owner_session_player_is_valid_when_dead() {
    let mut store = store();
    let uuid = Uuid::from_u128(42);
    let entity = arrow(&mut store, Some(uuid));
    let mut level = level("minecraft:sculk_sensor");
    let (sender, receiver) = crate::tests::mailbox();
    crate::tests::join_loaded(
        &sender,
        &receiver,
        &mut level.world,
        1,
        "alice",
        uuid.as_u128(),
    );
    level.world.set_player_dead_for_test(ConnectionId(1));
    assert_eq!(observed_owner(&entity, &mut store, &mut level), Some(uuid));
}

#[test]
fn sculk_projectile_owner_cross_dimension_presence_and_absence() {
    for exists in [true, false] {
        let mut native = level("minecraft:sculk_sensor");
        let (_, uuid) = native.world.spawn_stored_entity_for_test(
            bbb_server_types::save::Java26BuiltinDimension::Nether,
            VANILLA_ENTITY_TYPE_PIG_ID,
            Vec3d {
                y: 64.0,
                ..POSITION
            },
            EntityData::GeneralLiving(
                crate::entity::GeneralLivingData::new(VANILLA_ENTITY_TYPE_PIG_ID).unwrap(),
            ),
        );
        native.world.ensure_entity_transfer_full_chunk_for_test(
            bbb_server_types::save::Java26BuiltinDimension::Nether,
            (0, 0),
        );
        native.world.track_stored_chunk_for_test(
            bbb_server_types::save::Java26BuiltinDimension::Nether,
            (0, 0),
        );
        native.world.tick(Vec::new());
        let mut store = store();
        let entity = arrow(
            &mut store,
            Some(if exists { uuid } else { Uuid::from_u128(42) }),
        );
        assert_eq!(
            observed_owner(&entity, &mut store, &mut native),
            exists.then_some(uuid)
        );
    }
}

#[test]
fn sculk_projectile_without_owner_and_item_thrower_do_not_resolve_owner() {
    let mut store = store();
    let projectile = arrow(&mut store, None);
    let id = crate::entity::item::spawn_item_entity(
        &mut store,
        POSITION,
        crate::entity::item::plain_stack("minecraft:stone", 1).unwrap(),
    )
    .unwrap();
    let mut item = store.detach(id).unwrap();
    let EntityData::Item(data) = &mut item.data else {
        panic!("item")
    };
    data.set_thrower(Some(Uuid::from_u128(42)));
    for (entity, is_projectile) in [(&projectile, true), (&item, false)] {
        let mut level = level("minecraft:sculk_sensor");
        assert_eq!(observed_owner(entity, &mut store, &mut level), None);
        assert_eq!(entity.is_projectile_class(), is_projectile);
    }
}
