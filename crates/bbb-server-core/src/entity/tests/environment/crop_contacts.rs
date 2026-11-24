use super::*;
use crate::entity::tests::native_level::{NativeLevel, DIMENSION};

const CROPS: [&str; 2] = ["minecraft:wheat", "minecraft:pitcher_crop"];

fn actor(kind: &str) -> (EntityStore, i32) {
    let mut store = EntityStore::new(17, EntityIdCounter::new());
    // Keep drops above the collision bulb so they use ordinary block contacts.
    let position = Vec3d {
        y: FEET.y + 0.5,
        ..FEET
    };
    let id = if kind == "minecraft:item" {
        item_at(&mut store, position)
    } else {
        living_at(&mut store, kind, position)
    };
    let entity = store.get_mut(id).unwrap();
    entity.set_delta_movement(Vec3d {
        x: 2.0,
        y: 0.0,
        z: 0.0,
    });
    entity.portal_cooldown = 3;
    store.drain_lifecycle_events();
    (store, id)
}

fn assert_traversed(store: &EntityStore, id: i32, native: &mut NativeLevel, crop: &str) {
    let entity = store.get(id).unwrap();
    assert!(entity.position().x > 1.5, "{crop}");
    // The later cobweb proves the contact walk continued beyond the crop.
    assert_eq!(
        entity.stuck_speed_multiplier(),
        Vec3d {
            x: 0.25,
            y: f64::from(0.05_f32),
            z: 0.25
        },
        "{crop}"
    );
    assert_eq!(
        native.world.block_state(DIMENSION, pos(0, 64, 0)),
        Some(state_id(crop))
    );
    assert!(native.world.stored_items_for_test(DIMENSION).is_empty());
    assert!(native
        .world
        .positioned_game_events_for_test(DIMENSION)
        .iter()
        .all(|event| event.recorded.game_event
            != crate::level_event::VanillaGameEvent::BLOCK_DESTROY));
}

#[test]
fn crop_contacts_allow_living_and_nonliving_tick_traversal() {
    for crop in CROPS {
        for kind in ["minecraft:cow", "minecraft:item"] {
            let (mut store, id) = actor(kind);
            let mut native = NativeLevel::new();
            for x in 0..=3 {
                native.set(pos(x, 63, 0), state_id("minecraft:farmland"));
            }
            native.set(pos(0, 64, 0), state_id(crop));
            native.set(pos(1, 64, 0), state_id("minecraft:cobweb"));
            native.with_level(|level| {
                crate::entity::tick_one_entity(&mut store, level, id);
            });
            assert_traversed(&store, id, &mut native, crop);
            match store.get(id).unwrap().data() {
                EntityData::Item(_) => {}
                EntityData::GeneralLiving(_) => {
                    assert_eq!(store.get(id).unwrap().living().unwrap().health(), 10.0);
                }
                _ => unreachable!(),
            }
        }
    }
}
