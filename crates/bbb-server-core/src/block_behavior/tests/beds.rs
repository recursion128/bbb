use bbb_protocol::block_types::VANILLA_BLOCK_TYPES;
use bbb_server_types::directions::opposite;

use super::*;

const BED: BlockPos = BlockPos { x: 4, y: 70, z: 5 };
const HORIZONTAL: [Direction; 4] = [
    Direction::North,
    Direction::South,
    Direction::West,
    Direction::East,
];

fn bed(resource_id: &str, facing: Direction, part: &str, occupied: bool) -> i32 {
    state_of(
        resource_id,
        &[
            ("facing", facing_name(facing)),
            ("occupied", if occupied { "true" } else { "false" }),
            ("part", part),
        ],
    )
}

fn facing_name(direction: Direction) -> &'static str {
    match direction {
        Direction::North => "north",
        Direction::South => "south",
        Direction::West => "west",
        Direction::East => "east",
        _ => panic!("bed facing must be horizontal"),
    }
}

fn updated(current: i32, direction: Direction, neighbour: i32) -> i32 {
    let mut world = NativeWorld::new();
    world.set(BED, current);
    world.set(relative(BED, direction), neighbour);
    world.shape_update(BED, direction);
    world.state(BED)
}

#[test]
fn both_halves_copy_occupied_from_their_partner_for_every_facing() {
    {
        let entry = VANILLA_BLOCK_TYPES
            .iter()
            .find(|entry| entry.resource_id == "minecraft:white_bed")
            .unwrap();
        for facing in HORIZONTAL {
            for (part, neighbour_part, partner_direction) in
                [("foot", "head", facing), ("head", "foot", opposite(facing))]
            {
                for occupied in [false, true] {
                    let current = bed(entry.resource_id, facing, part, !occupied);
                    let expected = bed(entry.resource_id, facing, part, occupied);
                    let neighbour = bed(entry.resource_id, facing, neighbour_part, occupied);
                    let returned = updated(current, partner_direction, neighbour);
                    assert_eq!(
                        returned, expected,
                        "{} {facing:?} {part} occupied={occupied}",
                        entry.resource_id
                    );
                }
            }
        }
    }
}

#[test]
fn missing_wrong_block_or_same_part_partner_removes_the_half() {
    let current = bed("minecraft:white_bed", Direction::East, "foot", false);
    let same_part = bed("minecraft:white_bed", Direction::East, "foot", true);
    let wrong_colour = bed("minecraft:red_bed", Direction::East, "head", true);

    for neighbour in [stone(), same_part, wrong_colour] {
        let returned = updated(current, Direction::East, neighbour);
        assert_eq!(returned, air());
    }
}

#[test]
fn unrelated_side_update_preserves_the_half() {
    let current = bed("minecraft:white_bed", Direction::East, "foot", false);
    let neighbour = bed("minecraft:white_bed", Direction::East, "head", true);
    assert_eq!(updated(current, Direction::North, neighbour), current);
}
