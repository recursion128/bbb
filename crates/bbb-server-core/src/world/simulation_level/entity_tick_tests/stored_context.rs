use super::*;
use crate::registry_tags::RegistryTagSnapshot;
use crate::world::simulation_level::LevelCtx;
use bbb_protocol::block_state_types::vanilla_block_default_state_id;
use bbb_protocol::equipment_slots::EquipmentSlot;
use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue};
use std::sync::Arc;

const PLATFORM: BlockPos = BlockPos { x: 8, y: 65, z: 8 };
const WALKABLE: &str = "minecraft:powder_snow_walkable_mobs";

fn state(name: &str) -> i32 {
    vanilla_block_default_state_id(
        bbb_protocol::block_types::vanilla_block_type_for_resource_id(name)
            .unwrap()
            .id,
    )
    .unwrap()
}

fn install(level: &mut LevelCtx<'_, '_>, platform: &str) {
    for x in 7..=9 {
        for z in 7..=9 {
            for y in 62..=70 {
                level
                    .write_block(BlockPos { x, y, z }, state("minecraft:air"))
                    .unwrap();
            }
            level
                .write_block(BlockPos { x, y: 63, z }, state("minecraft:stone"))
                .unwrap();
        }
    }
    level.write_block(PLATFORM, state(platform)).unwrap();
}

fn boots() -> bbb_server_types::item_stack::RuntimeItemStack {
    bbb_server_types::item_stack::RuntimeItemStack::try_read(
        &NbtSummaryValue::Compound(vec![
            NbtSummaryEntry {
                name: "id".into(),
                value: NbtSummaryValue::String("minecraft:leather_boots".into()),
            },
            NbtSummaryEntry {
                name: "count".into(),
                value: NbtSummaryValue::Int(1),
            },
        ]),
        &bbb_registry_codec::UnavailableHolderResolver,
    )
    .unwrap()
}

fn stand(
    store: &mut EntityStore,
    y: f64,
    delta_y: f64,
    descending: bool,
    leather_boots: bool,
) -> i32 {
    let id = crate::entity::spawn_armor_stand(store, Vec3d { x: 8.5, y, z: 8.5 }).unwrap();
    let body = store.get_mut(id).unwrap();
    body.set_delta_movement(Vec3d {
        x: 0.0,
        y: delta_y,
        z: 0.0,
    });
    body.set_shared_flag(
        bbb_protocol::entity_synched_data::VANILLA_ENTITY_SHARED_FLAG_SHIFT_KEY_DOWN,
        descending,
    );
    if leather_boots {
        body.living_mut()
            .unwrap()
            .set_item_slot(EquipmentSlot::Feet, Some(boots()));
    }
    id
}

fn tick(store: &mut EntityStore, level: &mut LevelCtx<'_, '_>, id: i32) {
    assert!(matches!(
        crate::entity::tick_one_entity(store, level, id),
        crate::entity::EntityTickOutcome::Complete
    ));
}

fn tags(omit_entity_registry: bool, stand_walkable: bool) -> Arc<RegistryTagSnapshot> {
    let config = crate::tests::datapack_config();
    let registries: Vec<_> = config
        .datapacks
        .as_ref()
        .unwrap()
        .tags
        .registries
        .iter()
        .filter(|registry| !omit_entity_registry || registry.registry_id != "minecraft:entity_type")
        .collect();
    let tags = registries.iter().flat_map(|registry| {
        registry.tags.iter().map(|(tag, members)| {
            let mut members = members.clone();
            if stand_walkable && registry.registry_id == "minecraft:entity_type" && tag == WALKABLE
            {
                members.push("minecraft:armor_stand".into());
            }
            (registry.registry_id.clone(), tag.clone(), members)
        })
    });
    Arc::new(RegistryTagSnapshot::new(tags))
}

fn distance(store: &mut EntityStore, id: i32, value: f64) {
    let NbtSummaryValue::Compound(mut record) = store.command_data(id).unwrap() else {
        unreachable!()
    };
    record
        .iter_mut()
        .find(|entry| entry.name == "fall_distance")
        .unwrap()
        .value = NbtSummaryValue::Double(value.to_bits());
    store.load_command_data(id, &record).unwrap();
}

#[test]
fn stored_context_actual_tick_scaffolding_supports_from_above_and_shift_passes_through() {
    for descending in [false, true] {
        with_config(crate::tests::datapack_config(), |level| {
            install(level, "minecraft:scaffolding");
            let mut store = batch_store();
            let id = stand(&mut store, 67.0, -3.0, descending, false);
            tick(&mut store, level, id);
            let body = store.get(id).unwrap();
            assert_eq!(body.tick_count(), 1);
            assert_eq!(
                body.position(),
                Vec3d {
                    x: 8.5,
                    y: if descending { 64.0 } else { 66.0 },
                    z: 8.5
                }
            );
            assert_eq!(body.on_ground(), !descending);
            assert_eq!(level.block_state(PLATFORM), state("minecraft:scaffolding"));
        });
    }
}

#[test]
fn stored_context_actual_tick_powder_snow_boots_support_but_bare_feet_and_shift_sink() {
    for (leather_boots, descending, y, grounded) in [
        (false, false, 65.5, false),
        (true, false, 66.0, true),
        (true, true, 65.5, false),
    ] {
        with_config(crate::tests::datapack_config(), |level| {
            install(level, "minecraft:powder_snow");
            assert!(!LevelCtx::registry_tag_membership(
                level,
                "minecraft:entity_type",
                WALKABLE,
                "minecraft:armor_stand"
            ));
            let mut store = batch_store();
            let id = stand(&mut store, 66.5, -1.0, descending, leather_boots);
            tick(&mut store, level, id);
            let body = store.get(id).unwrap();
            assert_eq!(body.tick_count(), 1);
            assert_eq!(body.position(), Vec3d { x: 8.5, y, z: 8.5 });
            assert_eq!(body.on_ground(), grounded);
            assert_eq!(
                body.living().unwrap().item_by_slot(EquipmentSlot::Feet),
                leather_boots.then(boots).as_ref()
            );
            assert_eq!(level.block_state(PLATFORM), state("minecraft:powder_snow"));
        });
    }
}

#[test]
fn stored_context_actual_tick_powder_snow_reads_loaded_walkable_membership() {
    with_config(crate::tests::datapack_config(), |level| {
        install(level, "minecraft:powder_snow");
        level.chunks.install_registry_tags(tags(false, true));
        assert!(LevelCtx::registry_tag_membership(
            level,
            "minecraft:entity_type",
            WALKABLE,
            "minecraft:armor_stand"
        ));
        let mut store = batch_store();
        let id = stand(&mut store, 66.5, -1.0, false, false);
        tick(&mut store, level, id);
        let body = store.get(id).unwrap();
        assert_eq!(body.tick_count(), 1);
        assert_eq!(body.position().y, 66.0);
        assert!(body.on_ground());
        assert!(body
            .living()
            .unwrap()
            .item_by_slot(EquipmentSlot::Feet)
            .is_none());
    });
}

#[test]
fn stored_context_actual_tick_powder_snow_prior_distance_precedes_boots_shift_and_missing_tag() {
    with_config(crate::tests::datapack_config(), |level| {
        install(level, "minecraft:powder_snow");
        level.chunks.install_registry_tags(tags(true, false));
        let mut store = batch_store();
        let id = stand(&mut store, 67.0, -3.0, true, true);
        distance(&mut store, id, 2.500_000_001);
        assert!(!LevelCtx::registry_tag_membership(
            level,
            "minecraft:entity_type",
            WALKABLE,
            "minecraft:armor_stand"
        ));
        tick(&mut store, level, id);
        let body = store.get(id).unwrap();
        assert_eq!(body.tick_count(), 1);
        assert_eq!(body.position().y, 65.0 + f64::from(0.9_f32));
        assert!(body.on_ground());
        assert_eq!(
            body.living().unwrap().item_by_slot(EquipmentSlot::Feet),
            Some(&boots())
        );
    });
}
