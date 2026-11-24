use super::*;
use crate::entity::tests::native_level::NativeLevel;
use bbb_protocol::entity_types::*;
use bbb_protocol::nbt_summary::json_value_to_nbt_summary;
use bbb_server_types::item_stack::RuntimeItemStack;

fn body(kind: i32) -> Box<Entity> {
    let mut store = crate::entity::EntityStore::new(1, crate::entity::EntityIdCounter::new());
    let id = store
        .spawn_general_living(kind, Default::default())
        .unwrap();
    store.detach(id).unwrap()
}

fn equip(entity: &mut Entity, slot: EquipmentSlot, stack: RuntimeItemStack) {
    entity
        .living_mut()
        .unwrap()
        .set_item_slot(slot, Some(stack));
}

fn patched(value: serde_json::Value) -> RuntimeItemStack {
    bbb_server_types::item_stack::RuntimeItemStack::try_read(
        &json_value_to_nbt_summary(&serde_json::json!({
            "id": "minecraft:stone", "count": 1, "components": { "minecraft:equippable": value }
        }))
        .unwrap(),
        &crate::entity::test_items::BuiltinItems,
    )
    .unwrap()
}

#[test]
fn valid_equipment_requires_a_saddle_equippable_for_the_slot() {
    let mut pig = body(VANILLA_ENTITY_TYPE_PIG_ID);
    let mut native = NativeLevel::new();
    let tags = native.with_level(|level| level.capture_registry_tags());
    assert_eq!(
        has_valid_item_with_registry_tags(&pig, &tags, EquipmentSlot::Saddle),
        Some(false)
    );
    equip(
        &mut pig,
        EquipmentSlot::Saddle,
        crate::entity::item::plain_stack("minecraft:stone", 1).unwrap(),
    );
    assert_eq!(
        has_valid_item_with_registry_tags(&pig, &tags, EquipmentSlot::Saddle),
        Some(false)
    );
    equip(
        &mut pig,
        EquipmentSlot::Saddle,
        crate::entity::item::plain_stack("minecraft:saddle", 1).unwrap(),
    );
    assert_eq!(
        has_valid_item_with_registry_tags(&pig, &tags, EquipmentSlot::Saddle),
        Some(true)
    );
    equip(
        &mut pig,
        EquipmentSlot::Saddle,
        patched(serde_json::json!({"slot": "body"})),
    );
    assert_eq!(
        has_valid_item_with_registry_tags(&pig, &tags, EquipmentSlot::Saddle),
        Some(false)
    );
}

#[test]
fn valid_equipment_allowed_entities_tag_and_patch_removal() {
    let mut pig = body(VANILLA_ENTITY_TYPE_PIG_ID);
    let mut native = NativeLevel::new();
    let tags = native.with_level(|level| level.capture_registry_tags());
    for (allowed, expected) in [
        (serde_json::json!(["minecraft:pig"]), Some(true)),
        (serde_json::json!("minecraft:horse"), Some(false)),
        (serde_json::json!([]), Some(false)),
        (serde_json::json!("#minecraft:can_equip_saddle"), Some(true)),
    ] {
        equip(
            &mut pig,
            EquipmentSlot::Saddle,
            patched(serde_json::json!({"slot": "saddle", "allowed_entities": allowed})),
        );
        assert_eq!(
            has_valid_item_with_registry_tags(&pig, &tags, EquipmentSlot::Saddle),
            expected
        );
    }
    let removed = bbb_server_types::item_stack::RuntimeItemStack::try_read(
        &json_value_to_nbt_summary(&serde_json::json!({
            "id": "minecraft:saddle", "count": 1, "components": {"!minecraft:equippable": {}}
        }))
        .unwrap(),
        &bbb_registry_codec::UnavailableHolderResolver,
    )
    .unwrap();
    equip(&mut pig, EquipmentSlot::Saddle, removed);
    assert_eq!(
        has_valid_item_with_registry_tags(&pig, &tags, EquipmentSlot::Saddle),
        Some(false)
    );
}

#[test]
fn valid_equipment_can_use_slot_requires_alive_adult_and_tame() {
    let mut native = NativeLevel::new();
    let tags = native.with_level(|level| level.capture_registry_tags());
    for kind in [
        VANILLA_ENTITY_TYPE_PIG_ID,
        VANILLA_ENTITY_TYPE_DONKEY_ID,
        VANILLA_ENTITY_TYPE_NAUTILUS_ID,
    ] {
        let mut entity = body(kind);
        equip(
            &mut entity,
            EquipmentSlot::Saddle,
            crate::entity::item::plain_stack("minecraft:saddle", 1).unwrap(),
        );
        let needs_tame = kind != VANILLA_ENTITY_TYPE_PIG_ID;
        assert_eq!(
            has_valid_item_with_registry_tags(&entity, &tags, EquipmentSlot::Saddle),
            Some(!needs_tame)
        );
        if let EntityData::GeneralLiving(data) = &mut entity.data {
            if let Some(horse) = data.abstract_horse_retained_mut() {
                horse.set_tame(true);
            }
            if let Some(tame) = data.tamable_retained_mut() {
                tame.set_tame(true);
            }
        }
        assert_eq!(
            has_valid_item_with_registry_tags(&entity, &tags, EquipmentSlot::Saddle),
            Some(true)
        );
        entity.living_mut().unwrap().set_health(0.0);
        assert_eq!(
            has_valid_item_with_registry_tags(&entity, &tags, EquipmentSlot::Saddle),
            Some(false)
        );
        entity.living_mut().unwrap().set_health(20.0);
        if let EntityData::GeneralLiving(data) = &mut entity.data {
            data.ageable_retained_mut().unwrap().set_age(-100);
        }
        assert_eq!(
            has_valid_item_with_registry_tags(&entity, &tags, EquipmentSlot::Saddle),
            Some(false)
        );
    }
}

#[test]
fn valid_equipment_horse_leaf_overrides_accept_saddle_without_alive_or_tame_gates() {
    let mut native = NativeLevel::new();
    let tags = native.with_level(|level| level.capture_registry_tags());
    for kind in [
        VANILLA_ENTITY_TYPE_HORSE_ID,
        VANILLA_ENTITY_TYPE_LLAMA_ID,
        VANILLA_ENTITY_TYPE_SKELETON_HORSE_ID,
    ] {
        let mut entity = body(kind);
        equip(
            &mut entity,
            EquipmentSlot::Saddle,
            patched(serde_json::json!({"slot": "saddle"})),
        );
        entity.living_mut().unwrap().set_health(0.0);
        assert_eq!(
            has_valid_item_with_registry_tags(&entity, &tags, EquipmentSlot::Saddle),
            Some(true)
        );
    }
    for kind in [
        VANILLA_ENTITY_TYPE_CAMEL_ID,
        VANILLA_ENTITY_TYPE_CAMEL_HUSK_ID,
    ] {
        let mut entity = body(kind);
        equip(
            &mut entity,
            EquipmentSlot::Saddle,
            patched(serde_json::json!({"slot": "saddle"})),
        );
        assert_eq!(
            has_valid_item_with_registry_tags(&entity, &tags, EquipmentSlot::Saddle),
            Some(true)
        );
        entity.living_mut().unwrap().set_health(0.0);
        assert_eq!(
            has_valid_item_with_registry_tags(&entity, &tags, EquipmentSlot::Saddle),
            Some(false)
        );
    }
}

#[test]
fn valid_equipment_custom_allowed_tag_uses_loaded_snapshot() {
    let mut pig = body(VANILLA_ENTITY_TYPE_PIG_ID);
    equip(
        &mut pig,
        EquipmentSlot::Saddle,
        patched(serde_json::json!({
            "slot": "saddle", "allowed_entities": "#example:shrieker_mount"
        })),
    );
    let mut native = NativeLevel::new();
    for (member, expected) in [("minecraft:pig", true), ("minecraft:horse", false)] {
        native.install_tags(crate::registry_tags::RegistryTagSnapshot::new([(
            "minecraft:entity_type".to_owned(),
            "example:shrieker_mount".to_owned(),
            vec![member.to_owned()],
        )]));
        let tags = native.with_level(|level| level.capture_registry_tags());
        assert_eq!(
            has_valid_item_with_registry_tags(&pig, &tags, EquipmentSlot::Saddle),
            Some(expected)
        );
    }
}
