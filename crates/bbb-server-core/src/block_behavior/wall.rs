use super::{block_tag_matches, BlockShapeCtx, BlockTags};

use crate::fluid::{above, FlowingFluid};
use bbb_protocol::block_state_types::{
    vanilla_block_state_static_collision_shape, VanillaBlockCollisionBox,
    VanillaBlockSupportType as SupportType,
};
use bbb_protocol::block_types::{vanilla_block_type, VanillaGetStateForPlacementOverride};
use bbb_protocol::packets::{BlockPos, Direction};
use bbb_server_types::chunk_ticks::{FluidTickType, TickPriority};
use bbb_server_types::directions::{opposite, relative};
use bbb_server_types::fluid::{block_state_fluid_state, FluidFamily};

use super::support::str_property;
use super::{
    block_type_of, bool_property, default_block_state_id, direction_property,
    direction_serialized_name, face_sturdy_at, with_property,
};

const UP: &str = "up";
const WATERLOGGED: &str = "waterlogged";
const WALLS_TAG: &str = "minecraft:walls";
const WALL_POST_OVERRIDE_TAG: &str = "minecraft:wall_post_override";

#[derive(Clone, Copy)]
struct Bounds {
    min_x: f64,
    min_y: f64,
    min_z: f64,
    max_x: f64,
    max_y: f64,
    max_z: f64,
}

const TEST_POST: Bounds = Bounds {
    min_x: 7.0 / 16.0,
    min_y: 0.0,
    min_z: 7.0 / 16.0,
    max_x: 9.0 / 16.0,
    max_y: 1.0,
    max_z: 9.0 / 16.0,
};

#[must_use]
pub(super) fn is_wall(block_state_id: i32) -> bool {
    block_type_of(block_state_id)
        .and_then(vanilla_block_type)
        .and_then(|block| block.get_state_for_placement_override)
        == Some(VanillaGetStateForPlacementOverride::WallBlock)
}

fn horizontal_axis(direction: Direction) -> Option<char> {
    match direction {
        Direction::North | Direction::South => Some('z'),
        Direction::West | Direction::East => Some('x'),
        Direction::Down | Direction::Up => None,
    }
}

fn gate_connects(neighbour_state: i32, direction: Direction) -> bool {
    if !super::footing::is_fence_gate(neighbour_state) {
        return false;
    }
    direction_property(neighbour_state, "facing")
        .and_then(horizontal_axis)
        .zip(horizontal_axis(direction))
        .is_some_and(|(gate_axis, direction_axis)| gate_axis != direction_axis)
}

fn connects_to_at(
    neighbour_state: i32,
    neighbour_pos: BlockPos,
    direction: Direction,
    membership: &dyn Fn(&str, &str, &str) -> bool,
) -> bool {
    super::iron_bars::is_iron_bars(neighbour_state)
        || gate_connects(neighbour_state, direction)
        || block_tag_matches(neighbour_state, WALLS_TAG, membership)
        || !super::fence::is_exception_for_connection_at(neighbour_state, membership)
            && face_sturdy_at(neighbour_state, neighbour_pos, direction, SupportType::Full)
}

fn test_wall(direction: Direction) -> Option<Bounds> {
    Some(match direction {
        Direction::North => Bounds {
            min_x: 7.0 / 16.0,
            min_y: 0.0,
            min_z: 0.0,
            max_x: 9.0 / 16.0,
            max_y: 1.0,
            max_z: 9.0 / 16.0,
        },
        Direction::East => Bounds {
            min_x: 7.0 / 16.0,
            min_y: 0.0,
            min_z: 7.0 / 16.0,
            max_x: 1.0,
            max_y: 1.0,
            max_z: 9.0 / 16.0,
        },
        Direction::South => Bounds {
            min_x: 7.0 / 16.0,
            min_y: 0.0,
            min_z: 7.0 / 16.0,
            max_x: 9.0 / 16.0,
            max_y: 1.0,
            max_z: 1.0,
        },
        Direction::West => Bounds {
            min_x: 0.0,
            min_y: 0.0,
            min_z: 7.0 / 16.0,
            max_x: 9.0 / 16.0,
            max_y: 1.0,
            max_z: 9.0 / 16.0,
        },
        Direction::Down | Direction::Up => return None,
    })
}

fn push_clipped(coords: &mut Vec<f64>, min: f64, max: f64, test_min: f64, test_max: f64) {
    if max > test_min && min < test_max {
        coords.push(min.max(test_min));
        coords.push(max.min(test_max));
    }
}

fn sort_dedup(coords: &mut Vec<f64>) {
    coords.sort_by(|left, right| {
        left.partial_cmp(right)
            .expect("collision bounds are finite")
    });
    coords.dedup();
}

fn contains(unit: VanillaBlockCollisionBox, x: f64, y: f64, z: f64) -> bool {
    unit.min_x <= x
        && x <= unit.max_x
        && unit.min_y <= y
        && y <= unit.max_y
        && unit.min_z <= z
        && z <= unit.max_z
}

fn is_covered(above_state: i32, test: Bounds) -> Option<bool> {
    let shape = vanilla_block_state_static_collision_shape(above_state)?;
    // `calculateFace(DOWN)` samples the discrete Y slice containing 1e-7 and
    // `SliceShape` stretches that slice over 0..1.
    let boxes = shape
        .boxes()
        .filter(|unit| unit.min_y <= 1.0e-7 && unit.max_y > 1.0e-7)
        .map(|unit| VanillaBlockCollisionBox {
            min_y: 0.0,
            max_y: 1.0,
            ..unit
        })
        .collect::<Vec<_>>();
    let mut xs = vec![test.min_x, test.max_x];
    let mut ys = vec![test.min_y, test.max_y];
    let mut zs = vec![test.min_z, test.max_z];
    for unit in &boxes {
        push_clipped(&mut xs, unit.min_x, unit.max_x, test.min_x, test.max_x);
        push_clipped(&mut ys, unit.min_y, unit.max_y, test.min_y, test.max_y);
        push_clipped(&mut zs, unit.min_z, unit.max_z, test.min_z, test.max_z);
    }
    sort_dedup(&mut xs);
    sort_dedup(&mut ys);
    sort_dedup(&mut zs);
    for x in xs.windows(2) {
        for y in ys.windows(2) {
            for z in zs.windows(2) {
                let midpoint = (
                    f64::midpoint(x[0], x[1]),
                    f64::midpoint(y[0], y[1]),
                    f64::midpoint(z[0], z[1]),
                );
                if !boxes
                    .iter()
                    .any(|unit| contains(*unit, midpoint.0, midpoint.1, midpoint.2))
                {
                    return Some(false);
                }
            }
        }
    }
    Some(true)
}

fn side_connected(block_state_id: i32, direction: Direction) -> Option<bool> {
    Some(str_property(block_state_id, direction_serialized_name(direction))? != "none")
}

fn with_side(block_state_id: i32, direction: Direction, side: &str) -> Option<i32> {
    with_property(block_state_id, direction_serialized_name(direction), side)
}

fn update_shape_from_at(
    block_state_id: i32,
    above_state: i32,
    connections: [bool; 4],
    membership: &dyn Fn(&str, &str, &str) -> bool,
) -> Option<i32> {
    let directions = [
        Direction::North,
        Direction::East,
        Direction::South,
        Direction::West,
    ];
    let mut state = block_state_id;
    for (direction, connected) in directions.into_iter().zip(connections) {
        let side = if connected {
            if is_covered(above_state, test_wall(direction)?)? {
                "tall"
            } else {
                "low"
            }
        } else {
            "none"
        };
        state = with_side(state, direction, side)?;
    }

    let north = str_property(state, "north")?;
    let east = str_property(state, "east")?;
    let south = str_property(state, "south")?;
    let west = str_property(state, "west")?;
    let top_wall_has_post = is_wall(above_state) && bool_property(above_state, UP)?;
    let north_none = north == "none";
    let east_none = east == "none";
    let south_none = south == "none";
    let west_none = west == "none";
    let has_corner = north_none && east_none && south_none && west_none
        || north_none != south_none
        || west_none != east_none;
    let has_high_wall = north == "tall" && south == "tall" || east == "tall" && west == "tall";
    let raise = if top_wall_has_post || has_corner {
        true
    } else if has_high_wall {
        false
    } else {
        block_tag_matches(above_state, WALL_POST_OVERRIDE_TAG, membership)
            || is_covered(above_state, TEST_POST)?
    };
    with_property(state, UP, if raise { "true" } else { "false" })
}

pub(super) fn state_for_placement(
    block_type_id: i32,
    pos: BlockPos,
    tags: &dyn BlockTags,
    block_state_at: &mut dyn FnMut(BlockPos) -> i32,
) -> Option<i32> {
    let membership = |registry: &str, tag: &str, id: &str| tags.contains(registry, tag, id);
    let replaced = block_state_at(pos);
    block_type_of(replaced)?;
    let replaced_fluid = block_state_fluid_state(replaced);
    let waterlogged =
        replaced_fluid.is_source() && replaced_fluid.is_same_family(FluidFamily::Water);
    let directions = [
        Direction::North,
        Direction::East,
        Direction::South,
        Direction::West,
    ];
    let mut connections = [false; 4];
    for (index, direction) in directions.into_iter().enumerate() {
        let neighbour_pos = relative(pos, direction);
        let neighbour_state = block_state_at(neighbour_pos);
        connections[index] = connects_to_at(
            neighbour_state,
            neighbour_pos,
            opposite(direction),
            &membership,
        );
    }
    let above_state = block_state_at(above(pos));
    let state = with_property(
        default_block_state_id(block_type_id),
        WATERLOGGED,
        if waterlogged { "true" } else { "false" },
    )?;
    update_shape_from_at(state, above_state, connections, &membership)
}

pub(super) fn update_shape(
    level: &mut BlockShapeCtx<'_, '_, '_>,
    block_state_id: i32,
    pos: BlockPos,
    direction_to_neighbour: Direction,
    neighbour_state: i32,
) -> i32 {
    if bool_property(block_state_id, WATERLOGGED) == Some(true) {
        level.schedule_fluid_tick(
            pos,
            FluidTickType::of_fluid(FluidFamily::Water.source_identity()),
            FlowingFluid::WATER_TICK_DELAY,
            TickPriority::Normal,
        );
    }
    if direction_to_neighbour == Direction::Down {
        return block_state_id;
    }
    let directions = [
        Direction::North,
        Direction::East,
        Direction::South,
        Direction::West,
    ];
    let mut connections = [false; 4];
    for (index, direction) in directions.into_iter().enumerate() {
        connections[index] = match side_connected(block_state_id, direction) {
            Some(connected) => connected,
            None => return block_state_id,
        };
    }
    let above_state = if direction_to_neighbour == Direction::Up {
        neighbour_state
    } else {
        let Some(index) = directions
            .iter()
            .position(|direction| *direction == direction_to_neighbour)
        else {
            return block_state_id;
        };
        let neighbour_pos = relative(pos, direction_to_neighbour);
        connections[index] = connects_to_at(
            neighbour_state,
            neighbour_pos,
            opposite(direction_to_neighbour),
            &|registry, tag, id| level.registry_tag_membership(registry, tag, id),
        );
        level.block_state(above(pos))
    };
    update_shape_from_at(
        block_state_id,
        above_state,
        connections,
        &|registry, tag, id| level.registry_tag_membership(registry, tag, id),
    )
    .unwrap_or(block_state_id)
}
