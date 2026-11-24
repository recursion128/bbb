use super::*;
use crate::entity::tests::native_level::{NativeLevel, DIMENSION};

#[test]
fn native_hurt_uses_live_protection_immunity_and_death_events() {
    let mut native = NativeLevel::new();
    let mut store = EntityStore::new(37, EntityIdCounter::new());
    let id = store
        .spawn_general_living(
            VANILLA_ENTITY_TYPE_ZOMBIE_ID,
            Vec3d {
                x: 8.5,
                y: 80.0,
                z: 8.5,
            },
        )
        .unwrap();
    let mut body = store.detach(id).unwrap();
    assert_eq!(
        native.with_level(
            |level| crate::entity::general_living::hurt_from_environment_outcome(
                &mut body,
                &mut store,
                level,
                "minecraft:magic",
                4.0
            )
        ),
        Some(true)
    );
    assert_eq!(body.living().unwrap().health(), 16.0);
    let enchanted = |item: &str, enchantment: &str, level: i32| {
        bbb_server_types::item_stack::RuntimeItemStack::try_read(&bbb_protocol::nbt_summary::json_value_to_nbt_summary(&serde_json::json!({"id":item,"count":1,"components":{"minecraft:enchantments":{enchantment:level}}})).unwrap(), &crate::entity::test_items::BuiltinItems).unwrap()
    };
    body.living_mut().unwrap().set_item_slot(
        EquipmentSlot::Head,
        Some(enchanted("minecraft:stone", "minecraft:protection", 4)),
    );
    body.living_mut().unwrap().invulnerable_time = 0;
    body.set_invulnerable(true);
    assert_eq!(
        native.with_level(
            |level| crate::entity::general_living::hurt_from_environment_outcome(
                &mut body,
                &mut store,
                level,
                "minecraft:magic",
                4.0
            )
        ),
        Some(false)
    );
    assert_eq!(body.living().unwrap().health(), 16.0);
    body.set_invulnerable(false);
    assert_eq!(
        native.with_level(
            |level| crate::entity::general_living::hurt_from_environment_outcome(
                &mut body,
                &mut store,
                level,
                "minecraft:magic",
                4.0
            )
        ),
        Some(true)
    );
    assert!((body.living().unwrap().health() - 12.64).abs() < 0.001);
    body.living_mut().unwrap().set_item_slot(
        EquipmentSlot::Feet,
        Some(enchanted(
            "minecraft:leather_boots",
            "minecraft:frost_walker",
            1,
        )),
    );
    body.living_mut().unwrap().invulnerable_time = 0;
    let health = body.living().unwrap().health();
    assert_eq!(
        native.with_level(
            |level| crate::entity::general_living::hurt_from_environment_outcome(
                &mut body,
                &mut store,
                level,
                "minecraft:hot_floor",
                4.0
            )
        ),
        Some(false)
    );
    assert_eq!(body.living().unwrap().health(), health);
    assert_eq!(
        native.with_level(
            |level| crate::entity::general_living::hurt_from_environment_outcome(
                &mut body,
                &mut store,
                level,
                "minecraft:magic",
                1000.0
            )
        ),
        Some(true)
    );
    assert!(body.living().unwrap().is_dead_or_dying());
    assert!(native
        .world
        .positioned_game_events_for_test(DIMENSION)
        .iter()
        .any(
            |event| event.recorded.game_event == VanillaGameEvent::ENTITY_DIE
                && event.recorded.source_entity_id == Some(id)
                && event.recorded.pos == body.block_position()
        ));
}
