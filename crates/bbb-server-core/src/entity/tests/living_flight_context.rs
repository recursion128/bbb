use super::*;
use crate::entity::tests::native_level::{NativeLevel, DIMENSION};
use crate::level_event::VanillaGameEvent;
use bbb_protocol::entity_types::{VANILLA_ENTITY_TYPE_GHAST_ID, VANILLA_ENTITY_TYPE_PHANTOM_ID};
use bbb_protocol::equipment_slots::EquipmentSlot;
use bbb_server_types::geometry::Vec3Ops;

const START: Vec3d = Vec3d {
    x: 0.5,
    y: 80.0,
    z: 0.5,
};
const MOTION: Vec3d = Vec3d {
    x: 0.25,
    y: -0.125,
    z: 0.5,
};

fn body(kind: i32, no_ai: bool, equipped: bool) -> (EntityStore, i32) {
    let mut store = EntityStore::new(0x464c_4947, EntityIdCounter::new());
    let id = store.spawn_general_living(kind, START).unwrap();
    let entity = store.get_mut(id).unwrap();
    if let EntityData::GeneralLiving(data) = &mut entity.data {
        data.set_no_ai(no_ai);
    }
    entity.set_delta_movement(MOTION);
    entity.fall_distance = 7.0;
    entity.portal_cooldown = 3;
    if equipped {
        entity
            .living_mut()
            .unwrap()
            .set_item_slot(EquipmentSlot::MainHand, Some(stack_of("minecraft:dirt", 1)));
    }
    store.drain_lifecycle_events();
    (store, id)
}

fn tick(
    store: &mut EntityStore,
    level: &mut crate::world::simulation_level::LevelCtx<'_, '_>,
    id: i32,
) {
    assert!(matches!(
        crate::entity::tick_one_entity(store, level, id),
        crate::entity::EntityTickOutcome::Complete
    ));
}

#[test]
fn living_flight_context_actual_ticks_move_then_apply_air_water_and_lava_drag() {
    for kind in [VANILLA_ENTITY_TYPE_GHAST_ID, VANILLA_ENTITY_TYPE_PHANTOM_ID] {
        for (fluid, drag) in [
            (None, f64::from(0.91_f32)),
            (Some("minecraft:water"), f64::from(0.8_f32)),
            (Some("minecraft:lava"), 0.5),
        ] {
            let (mut store, id) = body(kind, false, false);
            let mut native = NativeLevel::new();
            if let Some(fluid) = fluid {
                for x in -3..=3 {
                    for y in 78..=86 {
                        for z in -3..=3 {
                            native.set(BlockPos { x, y, z }, state_id(fluid));
                        }
                    }
                }
            }
            native.with_level(|level| tick(&mut store, level, id));
            let entity = store.get(id).unwrap();
            let travelled = entity.position().plus(START.scale(-1.0));
            assert!(travelled.length_sqr() > 0.0);
            assert!(
                entity
                    .delta_movement()
                    .distance_to_sqr(travelled.scale(drag))
                    < 1.0e-20
            );
            if fluid.is_none() {
                assert_eq!(entity.fall_distance(), 7.0);
            }
        }
    }
}

#[test]
fn living_flight_context_actual_ground_contact_keeps_empty_fall_check_override() {
    for kind in [VANILLA_ENTITY_TYPE_GHAST_ID, VANILLA_ENTITY_TYPE_PHANTOM_ID] {
        let (mut store, id) = body(kind, false, false);
        let entity = store.get_mut(id).unwrap();
        entity.set_pos(Vec3d { y: 65.0, ..START });
        entity.set_delta_movement(Vec3d {
            x: 0.0,
            y: -2.0,
            z: 0.0,
        });
        let health = entity.living().unwrap().health();
        let mut native = NativeLevel::new();

        native.with_level(|level| tick(&mut store, level, id));
        let entity = store.get(id).unwrap();
        assert_eq!(entity.position().y, 64.0);
        assert!(entity.on_ground());
        assert!(entity.is_supported_by(BlockPos { x: 0, y: 63, z: 0 }));
        assert_eq!(entity.fall_distance(), 7.0);
        assert_eq!(entity.living().unwrap().health(), health);
        assert!(!native
            .world
            .positioned_game_events_for_test(DIMENSION)
            .iter()
            .any(|event| event.recorded.game_event == VanillaGameEvent::HIT_GROUND));
    }
}
