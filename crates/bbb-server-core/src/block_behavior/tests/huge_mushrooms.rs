use bbb_protocol::block_types::{VanillaGetStateForPlacementOverride, VANILLA_BLOCK_TYPES};

use super::*;

const MUSHROOM_POS: BlockPos = BlockPos { x: 4, y: 70, z: 5 };

fn mushroom_part(resource_id: &str) -> i32 {
    state_of(
        resource_id,
        &[
            ("north", "true"),
            ("east", "true"),
            ("south", "true"),
            ("west", "true"),
            ("up", "true"),
            ("down", "true"),
        ],
    )
}

fn mushroom_entries() -> Vec<&'static bbb_protocol::block_types::BlockTypeCatalogEntry> {
    VANILLA_BLOCK_TYPES
        .iter()
        .filter(|entry| {
            entry.get_state_for_placement_override
                == Some(VanillaGetStateForPlacementOverride::HugeMushroomBlock)
        })
        .collect()
}

fn update(world: &mut NativeWorld, state: i32, direction: Direction, neighbour: i32) -> i32 {
    world.set(MUSHROOM_POS, state);
    world.set(relative(MUSHROOM_POS, direction), neighbour);
    world.shape_update(MUSHROOM_POS, direction);
    world.state(MUSHROOM_POS)
}

#[test]
fn every_part_hides_the_face_touched_by_the_same_block() {
    for entry in mushroom_entries() {
        let state = mushroom_part(entry.resource_id);
        for direction in DIRECTION_VALUES {
            let mut world = NativeWorld::new();
            let updated = update(&mut world, state, direction, state);
            assert_eq!(
                updated,
                with_property(state, direction_serialized_name(direction), "false")
                    .expect("huge mushroom parts have all six face properties"),
                "{} {direction:?}",
                entry.resource_id
            );
        }
    }
}

#[test]
fn a_different_part_or_non_mushroom_neighbour_leaves_every_face_visible() {
    let entries = mushroom_entries();
    for (index, entry) in entries.iter().enumerate() {
        let state = mushroom_part(entry.resource_id);
        let different = mushroom_part(entries[(index + 1) % entries.len()].resource_id);
        for neighbour in [different, stone()] {
            for direction in DIRECTION_VALUES {
                let mut world = NativeWorld::new();
                assert_eq!(update(&mut world, state, direction, neighbour), state);
            }
        }
    }
}
