use super::*;

const FENCE_POS: BlockPos = BlockPos { x: 4, y: 70, z: 5 };

fn fence(
    resource_id: &str,
    north: bool,
    east: bool,
    south: bool,
    west: bool,
    waterlogged: bool,
) -> i32 {
    state_of(
        resource_id,
        &[
            ("east", if east { "true" } else { "false" }),
            ("north", if north { "true" } else { "false" }),
            ("south", if south { "true" } else { "false" }),
            ("waterlogged", if waterlogged { "true" } else { "false" }),
            ("west", if west { "true" } else { "false" }),
        ],
    )
}

fn plain_fence(resource_id: &str) -> i32 {
    fence(resource_id, false, false, false, false, false)
}

fn gate(resource_id: &str, facing: &str) -> i32 {
    state_of(
        resource_id,
        &[
            ("facing", facing),
            ("in_wall", "false"),
            ("open", "false"),
            ("powered", "false"),
        ],
    )
}

fn update(world: &mut NativeWorld, state: i32, direction: Direction, neighbour_state: i32) -> i32 {
    world.set(FENCE_POS, state);
    world.set(relative(FENCE_POS, direction), neighbour_state);
    world.shape_update(FENCE_POS, direction);
    world.state(FENCE_POS)
}

#[test]
fn wooden_and_nether_brick_fences_only_join_their_own_kind() {
    let oak = plain_fence("minecraft:oak_fence");
    let nether = plain_fence("minecraft:nether_brick_fence");
    assert_eq!(
        update(
            &mut NativeWorld::new(),
            oak,
            Direction::East,
            plain_fence("minecraft:spruce_fence"),
        ),
        fence("minecraft:oak_fence", false, true, false, false, false)
    );
    assert_eq!(
        update(&mut NativeWorld::new(), oak, Direction::East, nether),
        oak
    );
    assert_eq!(
        update(&mut NativeWorld::new(), nether, Direction::East, nether),
        fence(
            "minecraft:nether_brick_fence",
            false,
            true,
            false,
            false,
            false,
        )
    );
    assert_eq!(
        update(&mut NativeWorld::new(), nether, Direction::East, oak),
        nether
    );
}

#[test]
fn sturdy_faces_connect_except_for_vanillas_exception_classes() {
    let connected = fence("minecraft:oak_fence", false, true, false, false, false);
    assert_eq!(
        update(
            &mut NativeWorld::new(),
            plain_fence("minecraft:oak_fence"),
            Direction::East,
            stone(),
        ),
        connected
    );
    for resource_id in [
        "minecraft:oak_leaves",
        "minecraft:barrier",
        "minecraft:melon",
    ] {
        assert_eq!(
            update(
                &mut NativeWorld::new(),
                connected,
                Direction::East,
                default_state(resource_id),
            ),
            plain_fence("minecraft:oak_fence"),
            "{resource_id}"
        );
    }

    assert_eq!(
        update(
            &mut NativeWorld::new(),
            connected,
            Direction::East,
            default_state("minecraft:shulker_box"),
        ),
        plain_fence("minecraft:oak_fence")
    );
}

#[test]
fn gates_connect_only_across_their_facing_axis() {
    let oak = plain_fence("minecraft:oak_fence");
    assert_eq!(
        update(
            &mut NativeWorld::new(),
            oak,
            Direction::East,
            gate("minecraft:oak_fence_gate", "north"),
        ),
        fence("minecraft:oak_fence", false, true, false, false, false)
    );
    assert_eq!(
        update(
            &mut NativeWorld::new(),
            oak,
            Direction::North,
            gate("minecraft:oak_fence_gate", "north"),
        ),
        oak
    );
}

#[test]
fn every_horizontal_direction_updates_only_its_property() {
    for (direction, expected) in [
        (
            Direction::North,
            fence("minecraft:oak_fence", true, false, false, false, false),
        ),
        (
            Direction::East,
            fence("minecraft:oak_fence", false, true, false, false, false),
        ),
        (
            Direction::South,
            fence("minecraft:oak_fence", false, false, true, false, false),
        ),
        (
            Direction::West,
            fence("minecraft:oak_fence", false, false, false, true, false),
        ),
    ] {
        assert_eq!(
            update(
                &mut NativeWorld::new(),
                plain_fence("minecraft:oak_fence"),
                direction,
                stone(),
            ),
            expected
        );
    }
}

#[test]
fn waterlogged_fence_rearms_water_and_does_not_join_dripstone() {
    let waterlogged = fence("minecraft:oak_fence", false, false, false, false, true);
    let mut vertical = NativeWorld::new();
    assert_eq!(
        update(&mut vertical, waterlogged, Direction::Up, stone()),
        waterlogged
    );
    assert!(vertical
        .saved_fluid_ticks()
        .iter()
        .any(|tick| tick.id == "minecraft:water"
            && (tick.x, tick.y, tick.z) == (FENCE_POS.x, FENCE_POS.y, FENCE_POS.z)));

    let mut horizontal = NativeWorld::new();
    update(&mut horizontal, waterlogged, Direction::East, stone());
    assert!(horizontal
        .saved_fluid_ticks()
        .iter()
        .any(|tick| tick.id == "minecraft:water"
            && (tick.x, tick.y, tick.z) == (FENCE_POS.x, FENCE_POS.y, FENCE_POS.z)));

    let mut dry = NativeWorld::new();
    update(
        &mut dry,
        plain_fence("minecraft:oak_fence"),
        Direction::Up,
        stone(),
    );
    assert!(dry.saved_fluid_ticks().is_empty());

    assert_eq!(
        update(
            &mut NativeWorld::new(),
            plain_fence("minecraft:oak_fence"),
            Direction::East,
            default_state("minecraft:pointed_dripstone"),
        ),
        plain_fence("minecraft:oak_fence")
    );
}
