use super::native_level::{NativeLevel, DIMENSION};
use super::*;
use crate::entity::horse_phenotype::HorsePhenotypeState;
use bbb_protocol::entity_types::{
    VANILLA_ENTITY_TYPE_ALLAY_ID, VANILLA_ENTITY_TYPE_COPPER_GOLEM_ID,
    VANILLA_ENTITY_TYPE_DONKEY_ID,
};
use bbb_protocol::equipment_slots::EquipmentSlot;
use bbb_protocol::NbtSummaryEntry;
use std::collections::BTreeMap;
const POSITION: Vec3d = Vec3d {
    x: 0.5,
    y: 64.0,
    z: 0.5,
};
fn spawn(level: &mut NativeLevel, kind: i32) -> i32 {
    let mut data = crate::entity::GeneralLivingData::new(kind).unwrap();
    data.set_no_ai(true);
    level
        .world
        .spawn_stored_entity_for_test(DIMENSION, kind, POSITION, EntityData::GeneralLiving(data))
        .0
}
fn equip(level: &mut NativeLevel, id: i32, slot: EquipmentSlot, item: &str) {
    level
        .world
        .stored_entity_mut_for_test(DIMENSION, id)
        .unwrap()
        .living_mut()
        .unwrap()
        .set_item_slot(slot, crate::entity::item::plain_stack(item, 1));
}
fn put_internal(level: &mut NativeLevel, id: i32, slot: usize, item: &str) {
    let EntityData::GeneralLiving(data) = &mut level
        .world
        .stored_entity_mut_for_test(DIMENSION, id)
        .unwrap()
        .data
    else {
        unreachable!()
    };
    assert!(data
        .internal_inventory_mut()
        .unwrap()
        .set_item(slot, crate::entity::item::plain_stack(item, 1)));
}
fn drop_all_death_loot(level: &mut NativeLevel, id: i32) {
    let result = level
        .world
        .hurt_stored_entity_by_command_for_test(DIMENSION, id, "minecraft:generic", 1000.0, None)
        .unwrap();
    assert!(result.applied && result.succeeded);
    assert!(
        level
            .world
            .stored_entity_for_test(DIMENSION, id)
            .unwrap()
            .living()
            .unwrap()
            .dead
    );
}
fn dropped(level: &NativeLevel) -> BTreeMap<String, i32> {
    let mut items = BTreeMap::new();
    for item in level.world.stored_items_for_test(DIMENSION) {
        *items.entry(item.2).or_default() += item.3;
    }
    items
}
fn prevented_stack() -> bbb_server_types::item_stack::RuntimeItemStack {
    bbb_server_types::item_stack::RuntimeItemStack::try_read(
        &NbtSummaryValue::Compound(vec![
            NbtSummaryEntry {
                name: "id".into(),
                value: NbtSummaryValue::string("minecraft:cake"),
            },
            NbtSummaryEntry {
                name: "count".into(),
                value: NbtSummaryValue::Int(1),
            },
            NbtSummaryEntry {
                name: "components".into(),
                value: NbtSummaryValue::Compound(vec![NbtSummaryEntry {
                    name: "minecraft:enchantments".into(),
                    value: NbtSummaryValue::Compound(vec![NbtSummaryEntry {
                        name: "minecraft:vanishing_curse".into(),
                        value: NbtSummaryValue::Int(1),
                    }]),
                }]),
            },
        ]),
        &crate::entity::test_items::BuiltinItems,
    )
    .expect("an enchanted stack decodes")
}

#[test]
fn an_allay_death_drops_the_hand_item_unless_an_enchantment_holds_it() {
    let mut level = NativeLevel::new();
    let id = spawn(&mut level, VANILLA_ENTITY_TYPE_ALLAY_ID);
    equip(&mut level, id, EquipmentSlot::MainHand, "minecraft:cake");
    drop_all_death_loot(&mut level, id);
    assert_eq!(
        dropped(&level),
        BTreeMap::from([("minecraft:cake".into(), 1)])
    );
    assert!(level
        .world
        .stored_entity_for_test(DIMENSION, id)
        .unwrap()
        .living()
        .unwrap()
        .item_by_slot(EquipmentSlot::MainHand)
        .is_none());
}
#[test]
fn a_prevent_equipment_drop_enchantment_keeps_the_allays_hand_item() {
    let mut level = NativeLevel::new();
    let id = spawn(&mut level, VANILLA_ENTITY_TYPE_ALLAY_ID);
    level
        .world
        .stored_entity_mut_for_test(DIMENSION, id)
        .unwrap()
        .living_mut()
        .unwrap()
        .set_item_slot(EquipmentSlot::MainHand, Some(prevented_stack()));
    drop_all_death_loot(&mut level, id);
    assert!(dropped(&level).is_empty());
    assert!(level
        .world
        .stored_entity_for_test(DIMENSION, id)
        .unwrap()
        .living()
        .unwrap()
        .item_by_slot(EquipmentSlot::MainHand)
        .is_some());
}
#[test]
fn a_copper_golem_death_drops_only_the_preserved_slot() {
    let mut level = NativeLevel::new();
    let id = spawn(&mut level, VANILLA_ENTITY_TYPE_COPPER_GOLEM_ID);
    equip(
        &mut level,
        id,
        EquipmentSlot::Head,
        "minecraft:copper_ingot",
    );
    equip(&mut level, id, EquipmentSlot::MainHand, "minecraft:cake");
    let EntityData::GeneralLiving(data) = &mut level
        .world
        .stored_entity_mut_for_test(DIMENSION, id)
        .unwrap()
        .data
    else {
        unreachable!()
    };
    assert!(data
        .mob_retained_mut()
        .unwrap()
        .set_drop_chance(EquipmentSlot::Head, 2.0));
    let (sender, receiver) = crate::tests::mailbox();
    crate::tests::console_command(
        &sender,
        &receiver,
        &mut level.world,
        1,
        "gamerule mob_drops false",
    );
    drop_all_death_loot(&mut level, id);
    assert_eq!(
        dropped(&level),
        BTreeMap::from([("minecraft:copper_ingot".into(), 1)])
    );
    let living = level
        .world
        .stored_entity_for_test(DIMENSION, id)
        .unwrap()
        .living()
        .unwrap();
    assert!(living.item_by_slot(EquipmentSlot::Head).is_none());
    assert!(living.item_by_slot(EquipmentSlot::MainHand).is_some());
}
#[test]
fn a_chested_mount_death_drops_the_chest_and_clears_the_flag() {
    let mut level = NativeLevel::new();
    let id = spawn(&mut level, VANILLA_ENTITY_TYPE_DONKEY_ID);
    assert!(crate::entity::horse_phenotype::set_state(
        level
            .world
            .stored_entity_mut_for_test(DIMENSION, id)
            .unwrap(),
        HorsePhenotypeState::chested(true)
    ));
    let (sender, receiver) = crate::tests::mailbox();
    crate::tests::console_command(
        &sender,
        &receiver,
        &mut level.world,
        1,
        "gamerule mob_drops false",
    );
    drop_all_death_loot(&mut level, id);
    assert_eq!(
        dropped(&level),
        BTreeMap::from([("minecraft:chest".into(), 1)])
    );
    let EntityData::GeneralLiving(data) = level
        .world
        .stored_entity_for_test(DIMENSION, id)
        .unwrap()
        .data()
    else {
        unreachable!()
    };
    assert_eq!(
        data.horse_phenotype(),
        Some(HorsePhenotypeState::chested(false))
    );
}
#[test]
fn a_chestless_mount_death_drops_no_chest() {
    let mut level = NativeLevel::new();
    let id = spawn(&mut level, VANILLA_ENTITY_TYPE_DONKEY_ID);
    drop_all_death_loot(&mut level, id);
    assert!(!dropped(&level).contains_key("minecraft:chest"));
}
#[test]
fn an_allay_drops_internal_item_and_hand_even_without_mob_loot() {
    let mut level = NativeLevel::new();
    let id = spawn(&mut level, VANILLA_ENTITY_TYPE_ALLAY_ID);
    put_internal(&mut level, id, 0, "minecraft:apple");
    equip(&mut level, id, EquipmentSlot::MainHand, "minecraft:cake");
    let (sender, receiver) = crate::tests::mailbox();
    crate::tests::console_command(
        &sender,
        &receiver,
        &mut level.world,
        1,
        "gamerule mob_drops false",
    );
    drop_all_death_loot(&mut level, id);
    assert_eq!(
        dropped(&level),
        BTreeMap::from([("minecraft:apple".into(), 1), ("minecraft:cake".into(), 1)])
    );
    let EntityData::GeneralLiving(data) = level
        .world
        .stored_entity_for_test(DIMENSION, id)
        .unwrap()
        .data()
    else {
        unreachable!()
    };
    assert!(data.internal_inventory().unwrap().item(0).is_none());
}
#[test]
fn chested_mount_drops_sparse_cargo_and_chest_without_reusing_equipment_slots() {
    let mut level = NativeLevel::new();
    let id = spawn(&mut level, VANILLA_ENTITY_TYPE_DONKEY_ID);
    assert!(crate::entity::horse_phenotype::set_state(
        level
            .world
            .stored_entity_mut_for_test(DIMENSION, id)
            .unwrap(),
        HorsePhenotypeState::chested(true)
    ));
    let EntityData::GeneralLiving(data) = &mut level
        .world
        .stored_entity_mut_for_test(DIMENSION, id)
        .unwrap()
        .data
    else {
        unreachable!()
    };
    data.internal_inventory_mut()
        .unwrap()
        .resize_for_horse(HorsePhenotypeState::chested(true));
    put_internal(&mut level, id, 14, "minecraft:apple");
    put_internal(&mut level, id, 0, "minecraft:stick");
    equip(&mut level, id, EquipmentSlot::Saddle, "minecraft:saddle");
    let (sender, receiver) = crate::tests::mailbox();
    crate::tests::console_command(
        &sender,
        &receiver,
        &mut level.world,
        1,
        "gamerule mob_drops false",
    );
    drop_all_death_loot(&mut level, id);
    assert_eq!(
        dropped(&level),
        BTreeMap::from([
            ("minecraft:stick".into(), 1),
            ("minecraft:apple".into(), 1),
            ("minecraft:chest".into(), 1)
        ])
    );
    assert!(level
        .world
        .stored_entity_for_test(DIMENSION, id)
        .unwrap()
        .living()
        .unwrap()
        .item_by_slot(EquipmentSlot::Saddle)
        .is_some());
}
#[test]
fn a_piglin_death_drops_its_inventory() {
    let mut level = NativeLevel::new();
    let id = spawn(
        &mut level,
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_PIGLIN_ID,
    );
    put_internal(&mut level, id, 0, "minecraft:gold_ingot");
    put_internal(&mut level, id, 3, "minecraft:ender_pearl");
    drop_all_death_loot(&mut level, id);
    let items = dropped(&level);
    assert_eq!(items.get("minecraft:gold_ingot"), Some(&1));
    assert_eq!(items.get("minecraft:ender_pearl"), Some(&1));
}
