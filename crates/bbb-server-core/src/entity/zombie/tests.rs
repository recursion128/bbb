use crate::entity::tests::native_level::{state, NativeLevel, DIMENSION};
use bbb_protocol::attribute_types::VANILLA_ATTRIBUTE_MOVEMENT_SPEED_ID;
use bbb_protocol::entity_synched_data::{
    VANILLA_ZOMBIE_BABY_DATA_ID, VANILLA_ZOMBIE_DROWNED_CONVERSION_DATA_ID,
};
use bbb_protocol::entity_types::{
    VANILLA_ENTITY_TYPES, VANILLA_ENTITY_TYPE_DROWNED_ID, VANILLA_ENTITY_TYPE_HUSK_ID,
    VANILLA_ENTITY_TYPE_ZOMBIE_ID, VANILLA_ENTITY_TYPE_ZOMBIE_VILLAGER_ID,
    VANILLA_ENTITY_TYPE_ZOMBIFIED_PIGLIN_ID,
};
use bbb_protocol::equipment_slots::EquipmentSlot;
use bbb_protocol::packets::{BlockPos, EntityDataValueKind, Vec3d};
use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue};
use bbb_server_persistence::Java26EntityChunkData;
use bbb_server_types::save::{NamedCompound, JAVA_26_1_DATA_VERSION};

use super::{
    apply_conversion_phenotype, before_super, synched_values, ZombieRetainedState,
    BABY_SPEED_MODIFIER_ID,
};
use crate::entity::item::plain_stack;
use crate::entity::save::{read_entity_chunk, write_entity_chunk};
use crate::entity::tracking::non_default_synched_values;
use crate::entity::{tick_one_entity, EntityData, EntityIdCounter, EntityStore};

fn position() -> Vec3d {
    Vec3d {
        x: 0.5,
        y: 64.0,
        z: 0.5,
    }
}

fn entry(name: &str, value: NbtSummaryValue) -> NbtSummaryEntry {
    NbtSummaryEntry {
        name: name.into(),
        value,
    }
}

fn field<'a>(entries: &'a [NbtSummaryEntry], name: &str) -> Option<&'a NbtSummaryValue> {
    entries
        .iter()
        .find(|entry| entry.name == name)
        .map(|entry| &entry.value)
}

fn set_field(entries: &mut Vec<NbtSummaryEntry>, name: &str, value: NbtSummaryValue) {
    if let Some(field) = entries.iter_mut().find(|entry| entry.name == name) {
        field.value = value;
    } else {
        entries.push(entry(name, value));
    }
}

fn chunk_document(entries: Vec<NbtSummaryEntry>) -> NamedCompound {
    NamedCompound::new(
        "",
        vec![
            entry("DataVersion", NbtSummaryValue::Int(JAVA_26_1_DATA_VERSION)),
            entry("Position", NbtSummaryValue::IntArray(vec![0, 0])),
            entry(
                "Entities",
                NbtSummaryValue::List(vec![NbtSummaryValue::Compound(entries)]),
            ),
        ],
    )
}

fn chunk(entries: Vec<NbtSummaryEntry>) -> Java26EntityChunkData {
    Java26EntityChunkData::parse(chunk_document(entries), 0, 0).unwrap()
}

fn default_record(entity_type_id: i32) -> Vec<NbtSummaryEntry> {
    if entity_type_id == VANILLA_ENTITY_TYPE_ZOMBIE_VILLAGER_ID {
        let mut record = default_record(VANILLA_ENTITY_TYPE_ZOMBIE_ID);
        set_field(
            &mut record,
            "id",
            NbtSummaryValue::string("minecraft:zombie_villager"),
        );
        record.extend([
            entry(
                "VillagerData",
                NbtSummaryValue::Compound(vec![
                    entry("type", NbtSummaryValue::string("minecraft:plains")),
                    entry("profession", NbtSummaryValue::string("minecraft:none")),
                    entry("level", NbtSummaryValue::Int(1)),
                ]),
            ),
            entry("VillagerDataFinalized", NbtSummaryValue::Byte(1)),
            entry("ConversionTime", NbtSummaryValue::Int(-1)),
            entry("Xp", NbtSummaryValue::Int(0)),
        ]);
        return record;
    }
    let mut store = EntityStore::new(0x20_61, EntityIdCounter::new());
    store
        .spawn_general_living(entity_type_id, position())
        .expect("Zombie hierarchy uses GeneralLiving");
    write_entity_chunk(&store, (0, 0))
        .unwrap()
        .entities()
        .next()
        .unwrap()
        .raw()
        .to_vec()
}

fn zombie_state(store: &EntityStore, id: i32) -> ZombieRetainedState {
    let EntityData::GeneralLiving(data) = store.get(id).unwrap().data() else {
        unreachable!("Zombie hierarchy uses GeneralLiving")
    };
    data.zombie_retained().unwrap()
}

fn entity_zombie_state(entity: &crate::entity::Entity) -> ZombieRetainedState {
    let EntityData::GeneralLiving(data) = entity.data() else {
        unreachable!("Zombie hierarchy uses GeneralLiving")
    };
    data.zombie_retained().unwrap()
}

#[test]
fn zombie_villager_entity_region_roundtrip_resolves_fixed_registry_holders_locally() {
    let mut source = EntityStore::new(0x46_49_58_45_44, EntityIdCounter::new());
    let id = source
        .spawn_general_living(VANILLA_ENTITY_TYPE_ZOMBIE_VILLAGER_ID, position())
        .unwrap();
    assert!(crate::entity::finalize_spawner_villager_type(
        source.get_mut(id).unwrap(),
        "minecraft:plains",
    ));
    let saved = write_entity_chunk(&source, (0, 0)).unwrap();

    let mut restored = EntityStore::new(0x48_4f_4c_44_45_52, EntityIdCounter::new());
    let id = read_entity_chunk(&mut restored, &saved).unwrap()[0];
    assert!(crate::entity::resolve_spawner_registry_holders(
        restored.get_mut(id).unwrap(),
        |_, _| None,
    ));

    let rewritten = write_entity_chunk(&restored, (0, 0)).unwrap();
    let data = field(rewritten.entities().next().unwrap().raw(), "VillagerData").unwrap();
    let NbtSummaryValue::Compound(data) = data else {
        panic!("VillagerData is a compound")
    };
    assert_eq!(
        field(data, "type"),
        Some(&NbtSummaryValue::string("minecraft:plains"))
    );
    assert_eq!(
        field(data, "profession"),
        Some(&NbtSummaryValue::string("minecraft:none"))
    );
}

#[test]
fn zombie_underwater_conversion_census_save_restore_and_tracking_are_exact() {
    let owned = VANILLA_ENTITY_TYPES
        .iter()
        .filter_map(|facts| ZombieRetainedState::new(facts.id).map(|_| facts.id))
        .collect::<Vec<_>>();
    assert_eq!(
        owned,
        [
            VANILLA_ENTITY_TYPE_DROWNED_ID,
            VANILLA_ENTITY_TYPE_HUSK_ID,
            VANILLA_ENTITY_TYPE_ZOMBIE_ID,
            VANILLA_ENTITY_TYPE_ZOMBIE_VILLAGER_ID,
            VANILLA_ENTITY_TYPE_ZOMBIFIED_PIGLIN_ID,
        ]
    );

    let record = default_record(VANILLA_ENTITY_TYPE_ZOMBIE_ID);
    assert_eq!(field(&record, "IsBaby"), Some(&NbtSummaryValue::Byte(0)));
    assert_eq!(
        field(&record, "CanBreakDoors"),
        Some(&NbtSummaryValue::Byte(0))
    );
    assert_eq!(
        field(&record, "InWaterTime"),
        Some(&NbtSummaryValue::Int(-1))
    );
    assert_eq!(
        field(&record, "DrownedConversionTime"),
        Some(&NbtSummaryValue::Int(-1))
    );
    let mut water_store = EntityStore::new(1, EntityIdCounter::new());
    let water_id = water_store
        .spawn_general_living(VANILLA_ENTITY_TYPE_ZOMBIE_ID, position())
        .unwrap();
    let mut native = NativeLevel::new();
    native.set(BlockPos { x: 0, y: 64, z: 0 }, state("minecraft:water"));
    native.set(BlockPos { x: 0, y: 65, z: 0 }, state("minecraft:water"));
    for _ in 0..3 {
        native.with_level(|level| tick_one_entity(&mut water_store, level, water_id));
    }
    let water_record = write_entity_chunk(&water_store, (0, 0))
        .unwrap()
        .entities()
        .next()
        .unwrap()
        .raw()
        .to_vec();
    assert_eq!(
        field(&water_record, "InWaterTime"),
        Some(&NbtSummaryValue::Int(1))
    );
    let mut converting = record.clone();
    set_field(&mut converting, "InWaterTime", NbtSummaryValue::Int(17));
    set_field(
        &mut converting,
        "DrownedConversionTime",
        NbtSummaryValue::Int(42),
    );
    set_field(&mut converting, "IsBaby", NbtSummaryValue::Byte(1));
    set_field(&mut converting, "CanBreakDoors", NbtSummaryValue::Byte(1));
    let mut store = EntityStore::new(2, EntityIdCounter::new());
    let id = read_entity_chunk(&mut store, &chunk(converting)).unwrap()[0];
    assert_eq!(zombie_state(&store, id).in_water_time(), 17);
    assert_eq!(zombie_state(&store, id).conversion_time(), 42);
    assert!(zombie_state(&store, id).is_baby());
    assert!(zombie_state(&store, id).can_break_doors());
    assert!(store
        .get(id)
        .unwrap()
        .living()
        .unwrap()
        .attributes()
        .instance(VANILLA_ATTRIBUTE_MOVEMENT_SPEED_ID)
        .unwrap()
        .modifier(BABY_SPEED_MODIFIER_ID)
        .is_some());
    assert!(non_default_synched_values(store.get(id).unwrap())
        .iter()
        .any(|value| {
            value.data_id == VANILLA_ZOMBIE_DROWNED_CONVERSION_DATA_ID
                && value.value == EntityDataValueKind::Boolean(true)
        }));
    store.get_mut(id).unwrap().set_was_touching_water(true);
    let rewritten = write_entity_chunk(&store, (0, 0))
        .unwrap()
        .entities()
        .next()
        .unwrap()
        .raw()
        .to_vec();
    assert_eq!(
        field(&rewritten, "InWaterTime"),
        Some(&NbtSummaryValue::Int(17))
    );
    assert_eq!(field(&rewritten, "IsBaby"), Some(&NbtSummaryValue::Byte(1)));
    assert_eq!(
        field(&rewritten, "CanBreakDoors"),
        Some(&NbtSummaryValue::Byte(1))
    );
    assert_eq!(
        field(&rewritten, "DrownedConversionTime"),
        Some(&NbtSummaryValue::Int(42))
    );
    let EntityData::GeneralLiving(data) = store.get(id).unwrap().data() else {
        unreachable!()
    };
    assert!(data.opaque_fields().iter().all(|entry| {
        !matches!(
            entry.name.to_jdk_string().as_str(),
            "IsBaby" | "CanBreakDoors" | "InWaterTime" | "DrownedConversionTime"
        )
    }));

    let mut missing = record.clone();
    missing.retain(|entry| {
        !matches!(
            entry.name.to_jdk_string().as_str(),
            "IsBaby" | "CanBreakDoors" | "InWaterTime" | "DrownedConversionTime"
        )
    });
    let mut defaults = EntityStore::new(3, EntityIdCounter::new());
    let id = read_entity_chunk(&mut defaults, &chunk(missing)).unwrap()[0];
    assert_eq!(zombie_state(&defaults, id).in_water_time(), 0);
    assert_eq!(zombie_state(&defaults, id).conversion_time(), -1);
    assert!(!zombie_state(&defaults, id).is_baby());
    assert!(!zombie_state(&defaults, id).can_break_doors());
    assert!(synched_values(zombie_state(&defaults, id))
        .iter()
        .all(|value| value.is_default));

    let mut negative_conversion = default_record(VANILLA_ENTITY_TYPE_ZOMBIE_ID);
    set_field(
        &mut negative_conversion,
        "DrownedConversionTime",
        NbtSummaryValue::Int(-2),
    );
    let mut negative = EntityStore::new(31, EntityIdCounter::new());
    let id = read_entity_chunk(&mut negative, &chunk(negative_conversion)).unwrap()[0];
    assert_eq!(zombie_state(&negative, id).conversion_time(), -2);
    assert!(
        !synched_values(zombie_state(&negative, id))
            .iter()
            .find(|value| { value.value.data_id == VANILLA_ZOMBIE_DROWNED_CONVERSION_DATA_ID })
            .unwrap()
            .is_default
    );

    for (name, value) in [
        ("IsBaby", NbtSummaryValue::Byte(2)),
        ("CanBreakDoors", NbtSummaryValue::Int(1)),
        ("InWaterTime", NbtSummaryValue::Short(1)),
        ("DrownedConversionTime", NbtSummaryValue::Byte(1)),
    ] {
        let mut invalid = record.clone();
        set_field(&mut invalid, name, value);
        let mut refused = EntityStore::new(4, EntityIdCounter::new());
        assert_eq!(
            read_entity_chunk(&mut refused, &chunk(invalid))
                .unwrap()
                .len(),
            1
        );
    }
}

#[test]
fn zombie_phenotype_tracking_and_baby_speed_modifier_are_exact() {
    let mut store = EntityStore::new(51, EntityIdCounter::new());
    let id = store
        .spawn_general_living(VANILLA_ENTITY_TYPE_ZOMBIE_ID, position())
        .unwrap();
    let base_speed = store
        .get(id)
        .unwrap()
        .living()
        .unwrap()
        .attribute_value(VANILLA_ATTRIBUTE_MOVEMENT_SPEED_ID);
    assert!(apply_conversion_phenotype(
        store.get_mut(id).unwrap(),
        true,
        true,
    ));
    let entity = store.get_mut(id).unwrap();
    assert_eq!(
        entity.take_synched_dirty(),
        vec![VANILLA_ZOMBIE_BABY_DATA_ID]
    );
    let EntityData::GeneralLiving(data) = entity.data() else {
        unreachable!()
    };
    let state = data.zombie_retained().unwrap();
    assert!(state.is_baby());
    assert!(state.can_break_doors());
    let speed = data
        .living()
        .attributes()
        .instance(VANILLA_ATTRIBUTE_MOVEMENT_SPEED_ID)
        .unwrap();
    let modifier = speed.modifier(BABY_SPEED_MODIFIER_ID).unwrap();
    assert_eq!(modifier.amount, 0.5);
    assert_eq!(
        modifier.operation,
        crate::entity::AttributeOperation::AddMultipliedBase
    );
    assert_eq!(speed.value(), base_speed * 1.5);
    assert!(non_default_synched_values(entity).iter().any(|value| {
        value.data_id == VANILLA_ZOMBIE_BABY_DATA_ID
            && value.value == EntityDataValueKind::Boolean(true)
    }));

    assert!(apply_conversion_phenotype(entity, false, false));
    assert_eq!(
        entity.take_synched_dirty(),
        vec![VANILLA_ZOMBIE_BABY_DATA_ID]
    );
    let EntityData::GeneralLiving(data) = entity.data() else {
        unreachable!()
    };
    assert!(data
        .living()
        .attributes()
        .instance(VANILLA_ATTRIBUTE_MOVEMENT_SPEED_ID)
        .unwrap()
        .modifier(BABY_SPEED_MODIFIER_ID)
        .is_none());
    assert_eq!(
        data.living()
            .attribute_value(VANILLA_ATTRIBUTE_MOVEMENT_SPEED_ID),
        base_speed
    );
    let mut retained = data.zombie_retained().unwrap();
    retained.set_can_break_doors(true);
    assert!(retained.can_break_doors());
}

#[test]
fn zombie_underwater_conversion_threshold_reset_no_ai_and_overrides_are_exact() {
    let mut native = NativeLevel::new();
    native.set(BlockPos { x: 0, y: 64, z: 0 }, state("minecraft:water"));
    native.set(BlockPos { x: 0, y: 65, z: 0 }, state("minecraft:water"));
    let mut store = EntityStore::new(6, EntityIdCounter::new());
    let id = store
        .spawn_general_living(VANILLA_ENTITY_TYPE_ZOMBIE_ID, position())
        .unwrap();
    let mut entity = store.detach(id).unwrap();
    assert!(native.with_level(|level| crate::entity::base_tick(&mut entity, &mut store, level)));
    let EntityData::GeneralLiving(data) = &mut entity.data else {
        unreachable!()
    };
    data.zombie_retained_mut()
        .unwrap()
        .set_in_water_time_for_test(599);
    native.with_level(|level| before_super(&mut entity, &mut store, level));
    assert_eq!(entity_zombie_state(&entity).conversion_time(), 300);
    assert_eq!(
        entity.take_synched_dirty(),
        vec![VANILLA_ZOMBIE_DROWNED_CONVERSION_DATA_ID]
    );
    native.with_level(|level| before_super(&mut entity, &mut store, level));
    assert_eq!(entity_zombie_state(&entity).conversion_time(), 299);

    let EntityData::GeneralLiving(data) = &mut entity.data else {
        unreachable!()
    };
    data.zombie_retained_mut()
        .unwrap()
        .set_in_water_time_for_test(17);
    data.zombie_retained_mut().unwrap().converting = false;
    native.set(BlockPos { x: 0, y: 64, z: 0 }, state("minecraft:air"));
    native.set(BlockPos { x: 0, y: 65, z: 0 }, state("minecraft:air"));
    assert!(native.with_level(|level| crate::entity::base_tick(&mut entity, &mut store, level)));
    native.with_level(|level| before_super(&mut entity, &mut store, level));
    assert_eq!(entity_zombie_state(&entity).in_water_time(), -1);

    let EntityData::GeneralLiving(data) = &mut entity.data else {
        unreachable!()
    };
    data.mob.as_mut().unwrap().set_no_ai(true);
    data.zombie_retained_mut()
        .unwrap()
        .set_in_water_time_for_test(599);
    native.with_level(|level| before_super(&mut entity, &mut store, level));
    assert_eq!(entity_zombie_state(&entity).in_water_time(), 599);

    let dead_id = store
        .spawn_general_living(VANILLA_ENTITY_TYPE_ZOMBIE_ID, position())
        .unwrap();
    let mut dead = store.detach(dead_id).unwrap();
    let EntityData::GeneralLiving(data) = &mut dead.data else {
        unreachable!()
    };
    data.zombie_retained_mut()
        .unwrap()
        .set_in_water_time_for_test(599);
    data.living_mut().set_health(0.0);
    native.with_level(|level| before_super(&mut dead, &mut store, level));
    assert_eq!(entity_zombie_state(&dead).in_water_time(), 599);

    let id = store
        .spawn_general_living(VANILLA_ENTITY_TYPE_DROWNED_ID, position())
        .unwrap();
    let mut override_entity = store.detach(id).unwrap();
    native.with_level(|level| before_super(&mut override_entity, &mut store, level));
    assert_eq!(entity_zombie_state(&override_entity).in_water_time(), 0);
}

fn conversion_case(source_type: i32, target_type: i32, event_id: i32, silent: bool) {
    let mut store = EntityStore::new(i64::from(source_type), EntityIdCounter::new());
    let id = store.spawn_general_living(source_type, position()).unwrap();
    {
        let source = store.get_mut(id).unwrap();
        source.set_silent(silent);
        source.set_delta_movement(Vec3d {
            x: 0.25,
            y: 0.5,
            z: -0.75,
        });
        assert!(apply_conversion_phenotype(source, true, true));
        let EntityData::GeneralLiving(data) = &mut source.data else {
            unreachable!()
        };
        data.zombie_retained_mut()
            .unwrap()
            .set_conversion_time_for_test(0);
        data.living_mut().set_item_slot(
            EquipmentSlot::MainHand,
            plain_stack("minecraft:iron_sword", 1),
        );
    }
    store.drain_lifecycle_events();
    let mut native = NativeLevel::new();
    native.set(BlockPos { x: 0, y: 64, z: 0 }, state("minecraft:water"));
    native.set(BlockPos { x: 0, y: 65, z: 0 }, state("minecraft:water"));
    native.with_level(|level| tick_one_entity(&mut store, level, id));
    let events = native.world.drain_level_events_for_test(DIMENSION);
    assert_eq!(
        events.iter().any(|event| event.event_type == event_id
            && event.pos == BlockPos { x: 0, y: 64, z: 0 }
            && event.data == 0),
        !silent
    );
    assert!(store.get(id).is_none());
    let child = store
        .ids()
        .into_iter()
        .map(|id| store.get(id).unwrap())
        .find(|entity| entity.entity_type_id() == target_type)
        .unwrap();
    assert_eq!(child.entity_type_id(), target_type);
    assert_eq!(
        child.delta_movement(),
        Vec3d {
            x: 0.25,
            y: 0.5,
            z: -0.75,
        }
    );
    assert!(child
        .living()
        .unwrap()
        .item_by_slot(EquipmentSlot::MainHand)
        .is_some());
    assert!(entity_zombie_state(child).is_baby());
    assert!(entity_zombie_state(child).can_break_doors());
    assert!(child
        .living()
        .unwrap()
        .attributes()
        .instance(VANILLA_ATTRIBUTE_MOVEMENT_SPEED_ID)
        .unwrap()
        .modifier(BABY_SPEED_MODIFIER_ID)
        .is_some());
    assert!(child.living().unwrap().attributes().has_modifier(
        bbb_protocol::attribute_types::VANILLA_ATTRIBUTE_KNOCKBACK_RESISTANCE_ID,
        "minecraft:random_spawn_bonus",
    ));
}

fn restored_conversion_case(source_type: i32) {
    let mut record = default_record(source_type);
    set_field(
        &mut record,
        "DrownedConversionTime",
        NbtSummaryValue::Int(0),
    );
    let mut store = EntityStore::new(i64::from(source_type), EntityIdCounter::new());
    let id = read_entity_chunk(&mut store, &chunk(record)).unwrap()[0];
    store.drain_lifecycle_events();
    let mut native = NativeLevel::new();
    native.with_level(|level| tick_one_entity(&mut store, level, id));
    assert!(store.get(id).is_none());
    assert!(store
        .ids()
        .into_iter()
        .map(|id| store.get(id).unwrap())
        .any(|entity| entity.entity_type_id() == VANILLA_ENTITY_TYPE_DROWNED_ID));
    assert!(native
        .world
        .drain_level_events_for_test(DIMENSION)
        .iter()
        .any(|event| event.event_type == 1040
            && event.pos == BlockPos { x: 0, y: 64, z: 0 }
            && event.data == 0));
}

#[test]
fn zombie_underwater_conversion_targets_events_and_silent_gate_are_exact() {
    conversion_case(
        VANILLA_ENTITY_TYPE_ZOMBIE_ID,
        VANILLA_ENTITY_TYPE_DROWNED_ID,
        1040,
        false,
    );
    conversion_case(
        VANILLA_ENTITY_TYPE_HUSK_ID,
        VANILLA_ENTITY_TYPE_ZOMBIE_ID,
        1041,
        false,
    );
    conversion_case(
        VANILLA_ENTITY_TYPE_HUSK_ID,
        VANILLA_ENTITY_TYPE_ZOMBIE_ID,
        1041,
        true,
    );
    restored_conversion_case(VANILLA_ENTITY_TYPE_ZOMBIE_VILLAGER_ID);
}

#[path = "tests/baby_geometry.rs"]
mod baby_geometry;
