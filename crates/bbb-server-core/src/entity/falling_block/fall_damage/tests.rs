use super::*;
use crate::entity::falling_block::FallingBlockData;
use crate::entity::store::EntityIdCounter;
use crate::entity::tests::native_level::NativeLevel;
use bbb_protocol::block_state_types::{
    vanilla_block_default_state_id, vanilla_block_state_identity,
};
use bbb_protocol::block_types::{
    VANILLA_BLOCK_ANVIL_ID, VANILLA_BLOCK_CHIPPED_ANVIL_ID, VANILLA_BLOCK_DAMAGED_ANVIL_ID,
    VANILLA_BLOCK_POINTED_DRIPSTONE_ID, VANILLA_BLOCK_SAND_ID,
};
use bbb_protocol::entity_types::{
    VANILLA_ENTITY_TYPE_COW_ID, VANILLA_ENTITY_TYPE_FALLING_BLOCK_ID,
};
use bbb_protocol::packets::{BlockPos, Vec3d};
use uuid::Uuid;

fn state(block: i32) -> i32 {
    vanilla_block_default_state_id(block).unwrap()
}

fn position() -> Vec3d {
    Vec3d {
        x: 0.5,
        y: 64.0,
        z: 0.5,
    }
}

fn fixture(
    block: i32,
    per_distance: f32,
    max: i32,
    level: NativeLevel,
) -> (EntityStore, Box<Entity>, NativeLevel) {
    let mut store = EntityStore::new(71, EntityIdCounter::new());
    let id = store
        .restore(
            Uuid::from_u128(0x506),
            VANILLA_ENTITY_TYPE_FALLING_BLOCK_ID,
            position(),
            EntityData::FallingBlock(
                FallingBlockData::new(state(block), BlockPos { x: 0, y: 64, z: 0 })
                    .with_saved_fields(0, true, false, true, per_distance, max),
            ),
        )
        .unwrap();
    let source = store.detach(id).unwrap();
    (store, source, level)
}

fn cow(store: &mut EntityStore, pos: Vec3d) -> i32 {
    store
        .spawn_general_living(VANILLA_ENTITY_TYPE_COW_ID, pos)
        .unwrap()
}

fn call(
    source: &mut Entity,
    store: &mut EntityStore,
    native: &mut NativeLevel,
    distance: f64,
) -> Option<bool> {
    native.with_level(|level| {
        crate::entity::falling_block::cause_fall_damage_inner(
            source,
            store,
            level,
            distance,
            0.0,
            "minecraft:void",
        )
    })
}

fn falling(source: &Entity) -> FallingBlockData {
    let EntityData::FallingBlock(data) = source.data() else {
        panic!()
    };
    data.clone()
}

#[test]
fn stored_falling_damage_thresholds_and_float_power_ignore_incoming_parameters() {
    for (distance, damage) in [(1.0, 0.0), (1.000001, 2.0), (2.000001, 4.0), (100.0, 40.0)] {
        let (mut store, mut source, mut level) =
            fixture(VANILLA_BLOCK_SAND_ID, 2.0, 40, NativeLevel::new());
        let target = cow(&mut store, position());
        let before = store.get(target).unwrap().living().unwrap().health();
        assert_eq!(
            call(&mut source, &mut store, &mut level, distance),
            Some(false)
        );
        assert_eq!(
            store.get(target).unwrap().living().unwrap().health(),
            (before - damage).max(0.0)
        );
    }
    let (mut store, mut source, mut level) =
        fixture(VANILLA_BLOCK_SAND_ID, 0.7, 40, NativeLevel::new());
    let target = cow(&mut store, position());
    assert_eq!(call(&mut source, &mut store, &mut level, 11.0), Some(false));
    assert_eq!(store.get(target).unwrap().living().unwrap().health(), 3.0);
}

#[test]
fn stored_falling_damage_uses_each_damage_type_and_publishes_actual_death() {
    let mut startup = (*crate::registry_sync::builtin_datapack_startup_shared()).clone();
    startup
        .loot
        .registries
        .iter_mut()
        .find(|registry| registry.registry_id == "minecraft:loot_table")
        .unwrap()
        .elements
        .iter_mut()
        .find(|element| element.id == "minecraft:entities/cow")
        .unwrap()
        .value = bbb_server_datapack::loot::LootElementValue::Table(
        bbb_server_datapack::loot::parse_loot_table(&serde_json::json!({
            "type":"minecraft:entity", "pools":[{
                "rolls":1, "entries":[{"type":"minecraft:item", "name":"minecraft:amethyst_shard"}],
                "conditions":[{"condition":"minecraft:damage_source_properties", "predicate":{
                    "source_entity":{"movement":{"y":{"min":-21,"max":-19},"fall_distance":{"min":2}}}
                }}]
            }]
        }), None)
        .unwrap(),
    );
    let startup = std::sync::Arc::new(startup);
    for (block, damage_type) in [
        (VANILLA_BLOCK_SAND_ID, "minecraft:falling_block"),
        (VANILLA_BLOCK_ANVIL_ID, "minecraft:falling_anvil"),
        (
            VANILLA_BLOCK_POINTED_DRIPSTONE_ID,
            "minecraft:falling_stalactite",
        ),
    ] {
        let (mut store, mut source, mut level) = fixture(
            block,
            40.0,
            40,
            NativeLevel::with_datapacks(std::sync::Arc::clone(&startup)),
        );
        source.set_pos(Vec3d {
            x: 0.25,
            ..position()
        });
        source.set_delta_movement(Vec3d {
            y: -1.0,
            ..Vec3d::default()
        });
        source.set_fall_distance(2.0);
        let target = cow(&mut store, position());
        assert_eq!(call(&mut source, &mut store, &mut level, 2.0), Some(false));
        assert_eq!(store.get(target).unwrap().living().unwrap().health(), 0.0);
        let drops: Vec<_> = level
            .world
            .stored_items_for_test(crate::entity::tests::native_level::DIMENSION)
            .into_iter()
            .map(|item| item.2)
            .collect();
        assert_eq!(drops, ["minecraft:amethyst_shard"]);
        assert_eq!(
            store
                .get(target)
                .unwrap()
                .living()
                .unwrap()
                .last_damage_type
                .as_deref(),
            Some(damage_type)
        );
        assert!(level
            .world
            .positioned_game_events_for_test(crate::entity::tests::native_level::DIMENSION)
            .iter()
            .any(|event| event.recorded.game_event
                == crate::level_event::VanillaGameEvent::ENTITY_DIE
                && event.recorded.pos == BlockPos { x: 0, y: 64, z: 0 }
                && event.recorded.source_entity_id == Some(target)));
        assert!(store.get(source.id()).is_none());
    }
}

#[test]
fn stored_falling_damage_includes_riders() {
    let (mut store, mut source, mut level) =
        fixture(VANILLA_BLOCK_SAND_ID, 2.0, 40, NativeLevel::new());
    let first = cow(&mut store, position());
    let second = cow(&mut store, position());
    assert!(store.link_existing_passenger_forced(first, second));
    let detached = store.detach(first).unwrap();
    store.attach(detached);
    let dead = cow(&mut store, position());
    crate::entity::living::set_health(store.get_mut(dead).unwrap(), 0.0);
    let outside = cow(
        &mut store,
        Vec3d {
            x: 2.0,
            ..position()
        },
    );
    assert_eq!(call(&mut source, &mut store, &mut level, 2.0), Some(false));
    assert_eq!(store.get(first).unwrap().living().unwrap().health(), 8.0);
    assert_eq!(store.get(second).unwrap().living().unwrap().health(), 8.0);
    assert_eq!(store.get(dead).unwrap().living().unwrap().health(), 0.0);
    assert_eq!(store.get(outside).unwrap().living().unwrap().health(), 10.0);
}

#[test]
fn stored_falling_damage_anvil_chain_facing_and_custom_tag_cancel_drop() {
    for (block, next) in [
        (VANILLA_BLOCK_ANVIL_ID, Some(VANILLA_BLOCK_CHIPPED_ANVIL_ID)),
        (
            VANILLA_BLOCK_CHIPPED_ANVIL_ID,
            Some(VANILLA_BLOCK_DAMAGED_ANVIL_ID),
        ),
        (VANILLA_BLOCK_DAMAGED_ANVIL_ID, None),
    ] {
        let (mut store, mut source, mut level) = fixture(block, 2.0, 40, NativeLevel::new());
        let EntityData::FallingBlock(data) = &mut source.data else {
            panic!()
        };
        data.block_state_id =
            crate::fluid::with_property(data.block_state_id(), "facing", "west").unwrap();
        let before = data.block_state_id();
        assert_eq!(
            call(&mut source, &mut store, &mut level, 100.0),
            Some(false)
        );
        if let Some(next) = next {
            assert_eq!(
                vanilla_block_state_block_type_id(falling(&source).block_state_id()),
                Some(next)
            );
            assert_eq!(
                vanilla_block_state_identity(falling(&source).block_state_id())
                    .unwrap()
                    .properties["facing"],
                "west"
            );
            assert!(!falling(&source).cancel_drop());
        } else {
            assert_eq!(falling(&source).block_state_id(), before);
            assert!(falling(&source).cancel_drop());
        }
    }
    let (mut store, mut source, mut level) =
        fixture(VANILLA_BLOCK_SAND_ID, 2.0, 40, NativeLevel::new());
    level.install_tags(crate::registry_tags::RegistryTagSnapshot::new([(
        "minecraft:block".into(),
        "minecraft:anvil".into(),
        vec!["minecraft:sand".into()],
    )]));
    assert_eq!(
        call(&mut source, &mut store, &mut level, 100.0),
        Some(false)
    );
    assert!(falling(&source).cancel_drop());
}
