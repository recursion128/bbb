use super::*;
use crate::world::simulation_level::LevelCtx;
use bbb_protocol::entity_synched_data::VANILLA_ENTITY_SHARED_FLAG_SHIFT_KEY_DOWN;
use bbb_protocol::entity_types::{
    VANILLA_ENTITY_TYPE_GHAST_ID, VANILLA_ENTITY_TYPE_PHANTOM_ID, VANILLA_ENTITY_TYPE_STRIDER_ID,
};
use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue};

const POS: BlockPos = BlockPos { x: 8, y: 65, z: 8 };

fn state(name: &str) -> i32 {
    bbb_protocol::block_state_types::vanilla_block_default_state_id(
        bbb_protocol::block_types::vanilla_block_type_for_resource_id(name)
            .unwrap()
            .id,
    )
    .unwrap()
}

fn platform(level: &mut LevelCtx<'_, '_>, value: i32) {
    for x in 5..=11 {
        for z in 5..=11 {
            for y in 62..=74 {
                level
                    .write_block(BlockPos { x, y, z }, state("minecraft:air"))
                    .unwrap();
            }
            level
                .write_block(BlockPos { x, y: 63, z }, state("minecraft:stone"))
                .unwrap();
            level.write_block(BlockPos { x, y: 65, z }, value).unwrap();
        }
    }
}

fn spawn(store: &mut EntityStore, kind: i32, y: f64, shift: bool) -> i32 {
    let id = store
        .spawn_general_living(kind, Vec3d { x: 8.5, y, z: 8.5 })
        .unwrap();
    let NbtSummaryValue::Compound(mut data) = store.command_data(id).unwrap() else {
        unreachable!()
    };
    data.retain(|entry| entry.name != "NoAI" && entry.name != "fall_distance");
    data.extend([
        NbtSummaryEntry {
            name: "NoAI".into(),
            value: NbtSummaryValue::Byte(0),
        },
        NbtSummaryEntry {
            name: "fall_distance".into(),
            value: NbtSummaryValue::Double(2.0_f64.to_bits()),
        },
    ]);
    store.load_command_data(id, &data).unwrap();
    let body = store.get_mut(id).unwrap();
    body.set_delta_movement(Vec3d {
        x: 0.0,
        y: -1.0,
        z: 0.0,
    });
    body.set_shared_flag(VANILLA_ENTITY_SHARED_FLAG_SHIFT_KEY_DOWN, shift);
    id
}

fn tick(store: &mut EntityStore, level: &mut LevelCtx<'_, '_>, id: i32) {
    assert!(matches!(
        crate::entity::tick_one_entity(store, level, id),
        crate::entity::EntityTickOutcome::Complete
    ));
}

#[test]
fn living_travel_actual_flying_scaffolding_support_preserves_empty_check_and_base_ray() {
    for kind in [VANILLA_ENTITY_TYPE_GHAST_ID, VANILLA_ENTITY_TYPE_PHANTOM_ID] {
        for shift in [false, true] {
            with_config(crate::tests::datapack_config(), |level| {
                let scaffold = crate::fluid::with_property(
                    crate::fluid::with_property(state("minecraft:scaffolding"), "distance", "0")
                        .unwrap(),
                    "bottom",
                    "false",
                )
                .unwrap();
                platform(level, scaffold);
                let mut store = batch_store();
                let id = spawn(&mut store, kind, 66.5, shift);
                if shift {
                    store.get_mut(id).unwrap().set_delta_movement(Vec3d {
                        y: -2.0,
                        ..Vec3d::default()
                    });
                }
                tick(&mut store, level, id);
                let body = store.get(id).unwrap();
                assert_eq!(body.tick_count(), 1, "kind={kind}");
                if shift {
                    assert!(body.position().y < 66.0, "kind={kind}");
                } else {
                    assert_eq!(body.position().y, 66.0, "kind={kind}");
                }
                assert_eq!(body.on_ground(), !shift);
                assert!(LevelCtx::registry_tag_membership(
                    level,
                    "minecraft:block",
                    "minecraft:fall_damage_resetting",
                    "minecraft:scaffolding",
                ));
                // Entity.java:736-744: the full-block reset ray also runs for empty fall checks.
                assert_eq!(body.fall_distance(), if shift { 0.0 } else { 2.0 });
                assert_eq!(level.block_state(POS), scaffold);
            });
        }
    }
}

#[test]
fn living_travel_actual_strider_source_lava_stops_on_half_height() {
    with_config(crate::tests::datapack_config(), |level| {
        platform(level, state("minecraft:lava"));
        let mut store = batch_store();
        let id = spawn(&mut store, VANILLA_ENTITY_TYPE_STRIDER_ID, 66.0, true);
        tick(&mut store, level, id);
        let body = store.get(id).unwrap();
        assert_eq!(body.tick_count(), 1);
        assert_eq!(body.position().y, 65.5);
        assert!(body.on_ground());
        assert!(body.is_supported_by(POS));
        assert_eq!(level.block_state(POS), state("minecraft:lava"));
    });
}
