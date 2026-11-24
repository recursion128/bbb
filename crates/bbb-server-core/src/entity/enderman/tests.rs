use super::*;
use crate::entity::damage::{
    StoredDamageCausingEntity, StoredEntityDamageRules, StoredEntityDamageSource,
};
use crate::entity::tests::native_level::{NativeLevel, DIMENSION};
use crate::entity::{EntityData, EntityIdCounter};
use crate::level_event::VanillaGameEvent;
use bbb_protocol::entity_types::{VANILLA_ENTITY_TYPE_COW_ID, VANILLA_ENTITY_TYPE_ENDERMAN_ID};

const ORIGIN: Vec3d = Vec3d {
    x: 0.5,
    y: 96.0,
    z: 0.5,
};

fn fixture() -> (NativeLevel, EntityStore, Box<Entity>) {
    let mut native = NativeLevel::new();
    for x in -2..=2 {
        for z in -2..=2 {
            native
                .world
                .load_entity_chunk_for_test(DIMENSION, (x, z))
                .unwrap();
        }
    }
    let mut store = EntityStore::new(17, EntityIdCounter::new());
    let id = store
        .spawn_general_living(VANILLA_ENTITY_TYPE_ENDERMAN_ID, ORIGIN)
        .unwrap();
    let mut entity = store.detach(id).unwrap();
    let EntityData::GeneralLiving(data) = &mut entity.data else {
        unreachable!()
    };
    data.set_no_ai(true);
    entity.set_no_gravity(true);
    store.drain_lifecycle_events();
    (native, store, entity)
}

#[test]
fn enderman_evades_projectile_damage_and_accepts_a_living_attack() {
    let (mut native, mut store, mut entity) = fixture();
    let before = entity.living().unwrap().health();
    let arrow = bbb_server_datapack::damage::builtin_damage_type_registry()
        .iter()
        .find(|kind| kind.resource_id == "minecraft:arrow")
        .unwrap();
    let effects = native.with_level(|level| {
        crate::entity::general_living::hurt_server_with_runtime_in_level(
            &mut entity,
            &mut store,
            level,
            StoredEntityDamageSource::without_attacker_views(arrow, None, None),
            10.0,
            StoredEntityDamageRules::default(),
        )
    });
    assert!(effects.outcome.succeeded);
    assert_eq!(entity.living().unwrap().health(), before);
    assert_eq!(entity.position().y, 64.0);
    let packets = store.drain_lifecycle_events();
    assert!(packets.iter().any(|event| matches!(event,
        crate::entity::EntityLifecycleEvent::Broadcast { packet: crate::connection::ClientboundPlayPacket::EntityEvent(packet), .. }
        if packet.entity_id == entity.id() && packet.event_id == 46)));
    assert!(!packets.iter().any(|event| matches!(
        event,
        crate::entity::EntityLifecycleEvent::Broadcast {
            packet: crate::connection::ClientboundPlayPacket::DamageEvent(_),
            ..
        }
    )));

    let cow = store
        .spawn_general_living(VANILLA_ENTITY_TYPE_COW_ID, Vec3d { x: 100.0, ..ORIGIN })
        .unwrap();
    let attack = bbb_server_datapack::damage::builtin_damage_type_registry()
        .iter()
        .find(|kind| kind.resource_id == "minecraft:mob_attack")
        .unwrap();
    let position = entity.position();
    let effects = native.with_level(|level| {
        crate::entity::general_living::hurt_server_with_runtime_in_level(
            &mut entity,
            &mut store,
            level,
            StoredEntityDamageSource::without_attacker_views(
                attack,
                Some(cow),
                Some(StoredDamageCausingEntity::Mob { entity_id: cow }),
            ),
            1.0,
            StoredEntityDamageRules::default(),
        )
    });
    assert!(effects.outcome.succeeded);
    assert_eq!(entity.living().unwrap().health(), before - 1.0);
    assert_eq!(entity.position(), position);
}

#[test]
fn enderman_environment_hurt_teleports_without_moving_the_damage_event() {
    let (mut native, mut store, mut entity) = fixture();
    let before = entity.living().unwrap().health();
    for _ in 0..32 {
        assert!(native.with_level(|level| {
            crate::entity::general_living::hurt_from_environment(
                &mut entity,
                &mut store,
                level,
                "minecraft:drown",
                1.0,
            )
        }));
        if entity.position() != ORIGIN {
            break;
        }
    }
    assert_eq!(entity.position().y, 64.0);
    assert_eq!(entity.living().unwrap().health(), before - 1.0);
    let events = native.world.positioned_game_events_for_test(DIMENSION);
    for kind in [VanillaGameEvent::ENTITY_DAMAGE, VanillaGameEvent::TELEPORT] {
        assert!(events
            .iter()
            .any(|event| event.recorded.game_event == kind && event.position == ORIGIN));
    }
}
