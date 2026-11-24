use super::*;
use crate::entity::FluidTag;
use crate::level_event::VanillaGameEvent;
use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_COW_ID;
use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue};

const WET: BlockPos = BlockPos { x: 10, y: 68, z: 8 };
#[derive(Clone, Copy)]
enum Body {
    Cow,
    ArmorStand,
}

fn state(name: &str) -> i32 {
    bbb_protocol::block_state_types::vanilla_block_default_state_id(
        bbb_protocol::block_types::vanilla_block_type_for_resource_id(name)
            .unwrap()
            .id,
    )
    .unwrap()
}

fn arena(level: &mut LevelCtx<'_, '_>) {
    for x in 6..=13 {
        for z in 6..=10 {
            for y in 63..=73 {
                level
                    .write_block(BlockPos { x, y, z }, state("minecraft:air"))
                    .unwrap();
            }
            level
                .write_block(BlockPos { x, y: 64, z }, state("minecraft:stone"))
                .unwrap();
        }
    }
}

fn spawn(store: &mut EntityStore, kind: Body, start: Vec3d, delta: Vec3d) -> i32 {
    let id = match kind {
        Body::Cow => store
            .spawn_general_living(VANILLA_ENTITY_TYPE_COW_ID, start)
            .unwrap(),
        Body::ArmorStand => crate::entity::spawn_armor_stand(store, start).unwrap(),
    };
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
            value: NbtSummaryValue::Double(8.0_f64.to_bits()),
        },
    ]);
    store.load_command_data(id, &data).unwrap();
    let body = store.get_mut(id).unwrap();
    body.set_delta_movement(delta);
    assert!(!body.is_in_water());
    assert_eq!(body.fall_distance(), 8.0);
    id
}

fn tick(store: &mut EntityStore, level: &mut LevelCtx<'_, '_>, id: i32) {
    assert!(matches!(
        crate::entity::tick_one_entity(store, level, id),
        crate::entity::EntityTickOutcome::Complete
    ));
}

fn hits(level: &LevelCtx<'_, '_>) -> usize {
    level
        .event_sink
        .recent_game_events()
        .filter(|event| event.game_event == VanillaGameEvent::HIT_GROUND)
        .count()
}

#[test]
fn living_fall_fluid_dry_landing_control_calls_stone_fall_on() {
    with_config(crate::tests::datapack_config(), |level| {
        arena(level);
        let mut store = batch_store();
        let id = spawn(
            &mut store,
            Body::Cow,
            Vec3d {
                x: 8.5,
                y: 66.0,
                z: 8.5,
            },
            Vec3d {
                x: 2.0,
                y: -2.0,
                z: 0.0,
            },
        );
        let health = store.get(id).unwrap().living().unwrap().health();
        tick(&mut store, level, id);
        let body = store.get(id).unwrap();
        assert_eq!(
            body.position(),
            Vec3d {
                x: 10.5,
                y: 65.0,
                z: 8.5
            }
        );
        assert!(body.on_ground());
        assert_eq!(body.tick_count(), 1);
        assert_eq!(body.fall_distance(), 0.0);
        assert!(body.living().unwrap().health() < health);
        assert_eq!(hits(level), 1);
    });
}

#[test]
fn living_fall_fluid_enters_water_before_stone_landing_damage_and_hit_ground() {
    // LivingEntity.java:344-375; Entity.java:1554-1578.
    for horizontal in [false, true] {
        with_config(crate::tests::datapack_config(), |level| {
            arena(level);
            let water = BlockPos {
                x: if horizontal { 10 } else { 8 },
                y: 65,
                z: 8,
            };
            level.write_block(water, state("minecraft:water")).unwrap();
            let mut store = batch_store();
            let start = Vec3d {
                x: 8.5,
                y: 66.0,
                z: 8.5,
            };
            let id = spawn(
                &mut store,
                Body::Cow,
                start,
                Vec3d {
                    x: if horizontal { 2.0 } else { 0.0 },
                    y: -2.0,
                    z: 0.0,
                },
            );
            let health = store.get(id).unwrap().living().unwrap().health();
            assert!(store.get(id).unwrap().bounding_box().min_y > 65.0 + f64::from(8.0_f32 / 9.0));
            tick(&mut store, level, id);
            let body = store.get(id).unwrap();
            assert_eq!(
                body.position(),
                Vec3d {
                    x: f64::from(water.x) + 0.5,
                    y: 65.0,
                    z: 8.5
                }
            );
            assert!(body.on_ground());
            assert!(body.is_supported_by(BlockPos { y: 64, ..water }));
            assert!(
                body.is_in_water(),
                "the landing prefix must refresh contact before fallOn"
            );
            assert_eq!(body.fluid_height(FluidTag::Water), f64::from(8.0_f32 / 9.0));
            assert_eq!(body.fall_distance(), 0.0);
            assert_eq!(body.living().unwrap().health(), health);
            assert_eq!(hits(level), 0);
            assert_eq!(
                level.block_state(BlockPos { y: 64, ..water }),
                state("minecraft:stone")
            );
        });
    }
}

fn airborne(store: &mut EntityStore, kind: Body) -> i32 {
    spawn(
        store,
        kind,
        Vec3d {
            x: 8.5,
            y: 69.25,
            z: 8.5,
        },
        Vec3d {
            x: 2.0,
            y: -1.0,
            z: 0.0,
        },
    )
}

fn entered(body: &Entity) {
    assert_eq!(body.tick_count(), 1);
    assert_eq!(
        body.position(),
        Vec3d {
            x: 10.5,
            y: 68.25,
            z: 8.5
        }
    );
    assert!(!body.on_ground());
    assert!(
        body.is_in_water(),
        "airborne contact must update during this move"
    );
    assert_eq!(
        body.fluid_height(FluidTag::Water),
        f64::from(8.0_f32 / 9.0) - 0.25
    );
    assert_eq!(body.fall_distance(), 0.0);
}

#[test]
fn living_fall_fluid_airborne_entry_refreshes_tracker_and_resets_this_tick() {
    for kind in [Body::Cow, Body::ArmorStand] {
        with_config(crate::tests::datapack_config(), |level| {
            arena(level);
            level.write_block(WET, state("minecraft:water")).unwrap();
            let mut store = batch_store();
            let id = airborne(&mut store, kind);
            assert!(store.get(id).unwrap().bounding_box().max_x < f64::from(WET.x));
            let health = store.get(id).unwrap().living().unwrap().health();
            tick(&mut store, level, id);
            entered(store.get(id).unwrap());
            assert_eq!(store.get(id).unwrap().living().unwrap().health(), health);
            assert_eq!(hits(level), 0);
        });
    }
}
