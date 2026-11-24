use super::*;
use crate::entity::tests::native_level::NativeLevel;
use crate::entity::Entity;
use bbb_protocol::entity_types::{VANILLA_ENTITY_TYPE_COW_ID, VANILLA_ENTITY_TYPE_GHAST_ID};
use bbb_server_types::geometry::Vec3Ops;
use std::collections::BTreeSet;

const START: Vec3d = Vec3d {
    x: 8.5,
    y: 80.0,
    z: 8.5,
};

fn actor(kind: i32) -> (EntityStore, i32) {
    let mut store = EntityStore::new(17, EntityIdCounter::new());
    let id = store.spawn_general_living(kind, START).unwrap();
    let body = store.get_mut(id).unwrap();
    body.fall_distance = 7.0;
    let EntityData::GeneralLiving(data) = &mut body.data else {
        unreachable!()
    };
    data.set_no_ai(true);
    store.drain_lifecycle_events();
    (store, id)
}

fn checked(
    body: &mut Entity,
    store: &mut EntityStore,
    native: &mut NativeLevel,
    delta: Vec3d,
) -> Result<(), ()> {
    native.with_level(|level| {
        crate::entity::movement::move_living_entity(
            body,
            store,
            level,
            crate::entity::MoverType::Self_,
            delta,
        )
    })
}

#[test]
fn stored_descent_moves_the_body_through_actual_open_space() {
    for (length, distance) in [(0.0, 7.0), (1.0, 7.0), (2.0, 0.0)] {
        let mut native = NativeLevel::new();
        let (mut store, id) = actor(VANILLA_ENTITY_TYPE_COW_ID);
        let mut body = store.detach(id).unwrap();
        body.fall_distance = distance;
        let delta = Vec3d {
            x: 0.0,
            y: -length,
            z: 0.0,
        };
        assert_eq!(checked(&mut body, &mut store, &mut native, delta), Ok(()));
        assert_eq!(body.position(), START.plus(delta));
    }
}

#[test]
fn stored_descent_clips_against_the_actual_landing_surface() {
    for (floor, expected_y) in [(79, 80.0), (78, 79.0)] {
        let mut native = NativeLevel::new();
        native.set(
            BlockPos {
                x: 8,
                y: floor,
                z: 8,
            },
            state_id("minecraft:stone"),
        );
        let (mut store, id) = actor(VANILLA_ENTITY_TYPE_COW_ID);
        let mut body = store.detach(id).unwrap();
        body.set_pos(Vec3d { y: 80.5, ..START });
        assert_eq!(
            checked(
                &mut body,
                &mut store,
                &mut native,
                Vec3d {
                    x: 0.0,
                    y: -3.0,
                    z: 0.0
                }
            ),
            Ok(())
        );
        assert_eq!(body.position().y, expected_y);
        assert!(body.on_ground());
    }
}

#[test]
fn stored_descent_resets_distance_only_for_water_reached_by_the_ray() {
    for (water_y, expected_distance) in [(74, 12.0), (70, 19.0)] {
        let mut native = NativeLevel::new();
        native.set(
            BlockPos {
                x: 8,
                y: water_y,
                z: 8,
            },
            state_id("minecraft:water"),
        );
        let (mut store, id) = actor(VANILLA_ENTITY_TYPE_COW_ID);
        let mut body = store.detach(id).unwrap();
        assert_eq!(
            checked(
                &mut body,
                &mut store,
                &mut native,
                Vec3d {
                    x: 0.0,
                    y: -12.0,
                    z: 0.0
                }
            ),
            Ok(())
        );
        assert_eq!(body.position().y, 68.0);
        assert_eq!(body.fall_distance(), expected_distance);
        assert!(!body.on_ground());
    }
}

#[test]
fn flying_movement_resets_distance_at_actual_water_contact() {
    let mut native = NativeLevel::new();
    native.set(BlockPos { x: 8, y: 79, z: 8 }, state_id("minecraft:water"));
    let (mut store, id) = actor(VANILLA_ENTITY_TYPE_GHAST_ID);
    let mut body = store.detach(id).unwrap();
    assert_eq!(
        checked(
            &mut body,
            &mut store,
            &mut native,
            Vec3d {
                x: 0.0,
                y: -2.0,
                z: 0.0
            }
        ),
        Ok(())
    );
    assert_eq!(body.position().y, 78.0);
    assert_eq!(body.fall_distance(), 0.0);
}

fn portal(native: &mut NativeLevel) {
    let mut frame = BTreeSet::new();
    for x in 4..=15 {
        for y in [79, 87] {
            frame.insert((x, y));
        }
    }
    for y in 79..=87 {
        for x in [4, 15] {
            frame.insert((x, y));
        }
    }
    for (x, y) in frame {
        native.set(BlockPos { x, y, z: 8 }, state_id("minecraft:obsidian"));
    }
    for x in 5..=14 {
        for y in 80..=86 {
            native.set(BlockPos { x, y, z: 8 }, state_id("minecraft:nether_portal"));
        }
    }
}

#[test]
fn stored_fall_resetting_ray_default_provider_hits_tag_full_shape_and_flowing_water_only() {
    for (resource, tagged, expected_distance) in [
        ("minecraft:cobweb", true, 0.0),
        ("minecraft:water", false, 0.0),
        ("minecraft:air", false, 7.0),
        ("minecraft:nether_portal", false, 7.0),
    ] {
        let mut native = NativeLevel::new();
        let (mut store, id) = actor(VANILLA_ENTITY_TYPE_GHAST_ID);
        let mut body = store.detach(id).unwrap();
        body.set_pos(Vec3d { y: 80.2, ..START });
        if resource == "minecraft:nether_portal" {
            portal(&mut native);
        } else if resource != "minecraft:air" {
            let state = if resource == "minecraft:water" {
                vanilla_block_state_id_by_name_and_properties(
                    resource,
                    &BTreeMap::from([("level".into(), "5".into())]),
                )
                .unwrap()
            } else {
                state_id(resource)
            };
            native.set(BlockPos { x: 9, y: 80, z: 8 }, state);
        }
        let members: Vec<&str> = tagged.then_some(resource).into_iter().collect();
        native.install_tags(crate::registry_tags::RegistryTagSnapshot::new([
            (
                "minecraft:block".into(),
                "minecraft:fall_damage_resetting".into(),
                members.iter().map(|value| (*value).to_owned()).collect(),
            ),
            (
                "minecraft:fluid".into(),
                "minecraft:water".into(),
                vec!["minecraft:water".into(), "minecraft:flowing_water".into()],
            ),
        ]));
        assert_eq!(
            checked(
                &mut body,
                &mut store,
                &mut native,
                Vec3d {
                    x: 2.0,
                    y: 0.0,
                    z: 0.0
                }
            ),
            Ok(()),
            "{resource}"
        );
        assert_eq!(
            body.position(),
            Vec3d {
                x: 10.5,
                y: 80.2,
                z: 8.5
            }
        );
        assert_eq!(body.fall_distance(), expected_distance, "{resource}");
    }
}
