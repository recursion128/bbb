use super::*;
use crate::entity::{living, Entity, EntityTickOutcome};
use crate::registry_tags::RegistryTagSnapshot;
use bbb_protocol::entity_types::{
    VANILLA_ENTITY_TYPE_COW_ID, VANILLA_ENTITY_TYPE_GHAST_ID, VANILLA_ENTITY_TYPE_PHANTOM_ID,
    VANILLA_ENTITY_TYPE_STRIDER_ID,
};
use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue};
use std::sync::Arc;

const HISTORY: BlockPos = BlockPos { x: 6, y: 68, z: 8 };
const FEET: BlockPos = BlockPos { x: 8, y: 65, z: 8 };

fn state(name: &str) -> i32 {
    bbb_protocol::block_state_types::vanilla_block_default_state_id(
        bbb_protocol::block_types::vanilla_block_type_for_resource_id(name)
            .unwrap()
            .id,
    )
    .unwrap()
}

fn arena(level: &mut LevelCtx<'_, '_>) {
    for x in 5..=11 {
        for z in 6..=10 {
            for y in 63..=73 {
                level
                    .write_block(
                        BlockPos { x, y, z },
                        state(if y == 64 {
                            "minecraft:stone"
                        } else {
                            "minecraft:air"
                        }),
                    )
                    .unwrap();
            }
        }
    }
    level
        .write_block(HISTORY, state("minecraft:ladder"))
        .unwrap();
}

fn command(store: &mut EntityStore, id: i32, no_gravity: bool, distance: f64) {
    let NbtSummaryValue::Compound(mut entries) = store.command_data(id).unwrap() else {
        unreachable!()
    };
    entries.retain(|entry| {
        entry.name != "NoAI" && entry.name != "NoGravity" && entry.name != "fall_distance"
    });
    entries.extend([
        NbtSummaryEntry {
            name: "NoAI".into(),
            value: NbtSummaryValue::Byte(0),
        },
        NbtSummaryEntry {
            name: "NoGravity".into(),
            value: NbtSummaryValue::Byte(i8::from(no_gravity)),
        },
        NbtSummaryEntry {
            name: "fall_distance".into(),
            value: NbtSummaryValue::Double(distance.to_bits()),
        },
    ]);
    store.load_command_data(id, &entries).unwrap();
}

fn prime(store: &mut EntityStore, level: &mut LevelCtx<'_, '_>, kind: Option<i32>) -> i32 {
    let start = Vec3d {
        x: HISTORY.x as f64 + 0.5,
        y: HISTORY.y as f64 + 0.25,
        z: 8.5,
    };
    let id = match kind {
        Some(kind) => store.spawn_general_living(kind, start).unwrap(),
        None => crate::entity::spawn_armor_stand(store, start).unwrap(),
    };
    command(store, id, true, 0.0);
    let mut body = store.detach(id).unwrap();
    living::travel_in_air(&mut body, store, level, Vec3d::default()).unwrap();
    assert_eq!(history(&body), Some(HISTORY));
    store.attach(body);
    store.drain_lifecycle_events();
    id
}

fn history(body: &Entity) -> Option<BlockPos> {
    body.living().unwrap().last_climbable_pos()
}

fn tick(store: &mut EntityStore, level: &mut LevelCtx<'_, '_>, id: i32) {
    assert!(matches!(
        crate::entity::tick_one_entity(store, level, id),
        EntityTickOutcome::Complete
    ));
}

fn climbable_tag(level: &mut LevelCtx<'_, '_>, members: &[&str]) {
    let config = crate::tests::datapack_config();
    let registries = &config.datapacks.as_ref().unwrap().tags.registries;
    let mut replaced = false;
    let tags: Vec<_> = registries
        .iter()
        .flat_map(|registry| {
            registry
                .tags
                .iter()
                .map(|(tag, existing)| {
                    let values = if registry.registry_id == "minecraft:block"
                        && tag == "minecraft:climbable"
                    {
                        replaced = true;
                        members.iter().map(|member| (*member).to_string()).collect()
                    } else {
                        existing.clone()
                    };
                    (registry.registry_id.clone(), tag.clone(), values)
                })
                .collect::<Vec<_>>()
        })
        .collect();
    assert!(replaced);
    level
        .chunks
        .install_registry_tags(Arc::new(RegistryTagSnapshot::new(tags)));
}

#[test]
fn climbable_history_world_travel_updates_ladder_trapdoor_and_live_tag_but_keeps_negative_history()
{
    with_config(crate::tests::datapack_config(), |level| {
        arena(level);
        let mut store = batch_store();
        let id = prime(&mut store, level, Some(VANILLA_ENTITY_TYPE_COW_ID));
        let mut body = store.detach(id).unwrap();
        body.set_pos(Vec3d {
            x: 8.5,
            y: 65.25,
            z: 8.5,
        });
        body.set_delta_movement(Vec3d::default());
        living::travel_in_air(&mut body, &mut store, level, Vec3d::default()).unwrap();
        assert_eq!(history(&body), Some(HISTORY));

        let trapdoor =
            crate::fluid::with_property(state("minecraft:oak_trapdoor"), "open", "true").unwrap();
        let facing = crate::block_behavior::string_property(trapdoor, "facing").unwrap();
        let ladder =
            crate::fluid::with_property(state("minecraft:ladder"), "facing", &facing).unwrap();
        level
            .write_block(BlockPos { y: 64, ..FEET }, ladder)
            .unwrap();
        level.write_block(FEET, trapdoor).unwrap();
        // LivingEntity.java:1684-1714: actual travel records the open
        // trapdoor whose ladder faces the same way.
        assert_eq!(history(&body), Some(HISTORY));
        living::travel_in_air(&mut body, &mut store, level, Vec3d::default()).unwrap();
        assert_eq!(history(&body), Some(FEET));
        let negative = BlockPos { x: 9, y: 67, z: 8 };
        body.set_pos(Vec3d {
            x: 9.5,
            y: 67.25,
            z: 8.5,
        });
        level
            .write_block(BlockPos { y: 66, ..negative }, ladder)
            .unwrap();
        level
            .write_block(
                negative,
                crate::fluid::with_property(trapdoor, "open", "false").unwrap(),
            )
            .unwrap();
        living::travel_in_air(&mut body, &mut store, level, Vec3d::default()).unwrap();
        assert_eq!(history(&body), Some(FEET));
        level.write_block(negative, trapdoor).unwrap();
        level
            .write_block(
                BlockPos { y: 66, ..negative },
                crate::fluid::with_property(ladder, "facing", "south").unwrap(),
            )
            .unwrap();
        living::travel_in_air(&mut body, &mut store, level, Vec3d::default()).unwrap();
        assert_eq!(history(&body), Some(FEET));

        let custom = BlockPos { x: 10, y: 67, z: 8 };
        level
            .write_block(custom, state("minecraft:structure_void"))
            .unwrap();
        body.set_pos(Vec3d {
            x: 10.5,
            y: 67.25,
            z: 8.5,
        });
        body.set_delta_movement(Vec3d::default());
        climbable_tag(level, &["minecraft:structure_void"]);
        living::travel_in_air(&mut body, &mut store, level, Vec3d::default()).unwrap();
        assert_eq!(history(&body), Some(custom));
        climbable_tag(level, &[]);
        let removed = BlockPos { y: 69, ..custom };
        level
            .write_block(removed, state("minecraft:structure_void"))
            .unwrap();
        body.set_pos(Vec3d {
            x: 10.5,
            y: 69.25,
            z: 8.5,
        });
        living::travel_in_air(&mut body, &mut store, level, Vec3d::default()).unwrap();
        assert_eq!(history(&body), Some(custom));
    });
}

#[test]
fn climbable_history_world_post_move_horizontal_collision_records_new_feet() {
    with_config(crate::tests::datapack_config(), |level| {
        arena(level);
        level.write_block(FEET, state("minecraft:ladder")).unwrap();
        level
            .write_block(BlockPos { x: 9, ..FEET }, state("minecraft:stone"))
            .unwrap();
        let mut store = batch_store();
        // A NoGravity armor stand has noPhysics (ArmorStand.java:113-115, :163) and never collides
        // (Entity.java:707-712), so the mover is a cow.
        let id = prime(&mut store, level, Some(VANILLA_ENTITY_TYPE_COW_ID));
        let mut body = store.detach(id).unwrap();
        body.set_pos(Vec3d {
            x: 7.75,
            y: 65.25,
            z: 8.5,
        });
        body.set_delta_movement(Vec3d {
            x: 1.25,
            y: 0.0,
            z: 0.0,
        });
        assert_eq!(body.block_position().x, 7);
        assert_eq!(
            level.block_state(body.block_position()),
            state("minecraft:air")
        );
        living::travel_in_air(&mut body, &mut store, level, Vec3d::default()).unwrap();
        assert!(body.horizontal_collision());
        assert_eq!(body.position().x, 9.0 - f64::from(body.bb_width()) / 2.0);
        assert_eq!(body.block_position(), FEET);
        assert_eq!(history(&body), Some(FEET));
        assert_eq!(body.delta_movement().y, 0.2 * f64::from(0.98_f32));
    });
}

#[test]
fn climbable_history_world_command_retains_but_durable_reload_starts_without_history() {
    for kind in [Some(VANILLA_ENTITY_TYPE_COW_ID), None] {
        with_config(crate::tests::datapack_config(), |level| {
            arena(level);
            let mut store = batch_store();
            let id = prime(&mut store, level, kind);
            command(&mut store, id, false, 2.0);
            assert_eq!(history(store.get(id).unwrap()), Some(HISTORY));
            let saved = crate::entity::save::write_entity_chunk(&store, (0, 0)).unwrap();
            let mut loaded = batch_store();
            let ids = crate::entity::save::read_entity_chunk(&mut loaded, &saved).unwrap();
            assert_eq!(ids.len(), 1);
            assert_eq!(
                loaded.get(ids[0]).unwrap().uuid(),
                store.get(id).unwrap().uuid()
            );
            assert_eq!(history(loaded.get(ids[0]).unwrap()), None);
        });
    }
}

#[test]
fn climbable_history_world_ordinary_landing_clears_even_without_fall_distance() {
    for kind in [Some(VANILLA_ENTITY_TYPE_COW_ID), None] {
        for distance in [0.0, 2.0] {
            with_config(crate::tests::datapack_config(), |level| {
                arena(level);
                let mut store = batch_store();
                let id = prime(&mut store, level, kind);
                command(&mut store, id, false, distance);
                let body = store.get_mut(id).unwrap();
                body.set_pos(Vec3d {
                    x: 8.5,
                    y: 65.0,
                    z: 8.5,
                });
                body.set_delta_movement(Vec3d {
                    x: 0.0,
                    y: -0.25,
                    z: 0.0,
                });
                tick(&mut store, level, id);
                let body = store.get(id).unwrap();
                assert_eq!(body.tick_count(), 1);
                assert!(body.on_ground());
                assert_eq!(body.position().y, 65.0);
                assert_eq!(body.fall_distance(), 0.0);
                // LivingEntity.java:371-374: successful ground completion clears
                // the history even when Entity has no fallOn distance to consume.
                assert_eq!(history(body), None);
            });
        }
    }
}

#[test]
fn climbable_history_world_empty_flying_and_old_lava_strider_preserve_history_on_ground() {
    for kind in [VANILLA_ENTITY_TYPE_GHAST_ID, VANILLA_ENTITY_TYPE_PHANTOM_ID] {
        with_config(crate::tests::datapack_config(), |level| {
            arena(level);
            let mut store = batch_store();
            let id = prime(&mut store, level, Some(kind));
            command(&mut store, id, false, 2.0);
            let body = store.get_mut(id).unwrap();
            body.set_pos(Vec3d {
                x: 8.5,
                y: 65.25,
                z: 8.5,
            });
            body.set_delta_movement(Vec3d {
                x: 0.0,
                y: -0.5,
                z: 0.0,
            });
            tick(&mut store, level, id);
            let body = store.get(id).unwrap();
            assert_eq!(body.tick_count(), 1);
            assert!(body.on_ground());
            assert_eq!(history(body), Some(HISTORY));
            assert_eq!(body.fall_distance(), 2.0);
        });
    }
    with_config(crate::tests::datapack_config(), |level| {
        arena(level);
        level.write_block(FEET, state("minecraft:lava")).unwrap();
        let mut store = batch_store();
        let id = prime(&mut store, level, Some(VANILLA_ENTITY_TYPE_STRIDER_ID));
        let body = store.get_mut(id).unwrap();
        body.set_pos(Vec3d {
            x: 8.5,
            y: 65.75,
            z: 8.5,
        });
        body.set_delta_movement(Vec3d::default());
        tick(&mut store, level, id);
        assert!(store.get(id).unwrap().is_in_lava());
        assert_eq!(history(store.get(id).unwrap()), Some(HISTORY));
        command(&mut store, id, false, 2.0);
        store.get_mut(id).unwrap().set_delta_movement(Vec3d {
            x: 0.0,
            y: -0.5,
            z: 0.0,
        });
        tick(&mut store, level, id);
        let body = store.get(id).unwrap();
        assert_eq!(body.tick_count(), 2);
        assert!(body.on_ground());
        assert_eq!(body.position().y, 65.5);
        assert_eq!(body.fall_distance(), 0.0);
        assert_eq!(history(body), Some(HISTORY));
    });
}
