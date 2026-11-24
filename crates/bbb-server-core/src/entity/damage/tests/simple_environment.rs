use super::*;
use crate::entity::*;

fn mark_but_false_bodies() -> Vec<EntityData> {
    vec![
        EntityData::Arrow(ArrowData::new(plain_stack("minecraft:arrow", 1).unwrap())),
        EntityData::Snowball(SnowballData::new(
            plain_stack("minecraft:snowball", 1).unwrap(),
        )),
        EntityData::FallingBlock(FallingBlockData::new(
            vanilla_block_default_state_id(
                vanilla_block_type_for_resource_id("minecraft:sand")
                    .unwrap()
                    .id,
            )
            .unwrap(),
            BlockPos { x: 1, y: 64, z: -3 },
        )),
    ]
}

fn unconditional_false_bodies() -> Vec<EntityData> {
    vec![
        EntityData::Tnt(TntData::new()),
        EntityData::Marker(MarkerData::new()),
        EntityData::SmallFireball(SmallFireballData::new(
            HurtingProjectileState::new(None, 0.1),
            plain_stack("minecraft:fire_charge", 1).unwrap(),
        )),
    ]
}

fn detached(data: EntityData) -> (EntityStore, Box<Entity>) {
    let type_id = bbb_protocol::entity_types::vanilla_entity_type_id_for_resource_id(
        data.entity_resource_id(),
    )
    .unwrap();
    let mut store = EntityStore::new(1, EntityIdCounter::new());
    let id = store.spawn(type_id, position(), data);
    store.drain_lifecycle_events();
    let body = store.detach(id).unwrap();
    (store, body)
}

#[test]
fn simple_environment_mark_but_false_uses_base_immunity() {
    for data in mark_but_false_bodies() {
        assert!(supports_detached_environment_hurt(&data));
        for invulnerable in [false, true] {
            let (mut store, mut body) = detached(data.clone());
            body.set_invulnerable(invulnerable);
            let mut native = NativeLevel::new();
            assert_eq!(
                native.with_level(|level| hurt_detached_from_environment(
                    &mut body,
                    &mut store,
                    level,
                    "minecraft:cactus",
                    1.0
                )),
                Some(false),
                "{}",
                data.entity_resource_id()
            );
            assert_eq!(
                body.hurt_marked(),
                !invulnerable,
                "{}",
                data.entity_resource_id()
            );
            assert!(body.removal_reason().is_none());
            assert!(native
                .world
                .positioned_game_events_for_test(DIMENSION)
                .is_empty());
        }
    }
}

#[test]
fn simple_environment_unconditional_false_bodies_are_not_hurt() {
    for data in unconditional_false_bodies() {
        assert!(supports_detached_environment_hurt(&data));
        let (mut store, mut body) = detached(data);
        let mut native = NativeLevel::new();
        assert_eq!(
            native.with_level(|level| hurt_detached_from_environment(
                &mut body,
                &mut store,
                level,
                "minecraft:lava",
                4.0
            )),
            Some(false),
            "{}",
            body.data().entity_resource_id()
        );
        assert!(!body.hurt_marked());
        assert!(body.is_alive());
        assert!(native
            .world
            .positioned_game_events_for_test(DIMENSION)
            .is_empty());
    }
}

#[test]
fn simple_environment_base_tick_burns_a_snowball() {
    let (mut store, mut body) = detached(EntityData::Snowball(SnowballData::new(
        plain_stack("minecraft:snowball", 1).unwrap(),
    )));
    body.set_remaining_fire_ticks(40);
    let mut native = NativeLevel::new();
    assert!(native.with_level(|level| base_tick_from_environment(&mut body, &mut store, level)));
    assert!(body.hurt_marked());
    assert_eq!(body.remaining_fire_ticks(), 39);
    assert!(native
        .world
        .positioned_game_events_for_test(DIMENSION)
        .is_empty());
}
