//! `ChestBlock.combine` (`ChestBlock.java:285-298`) through
//! `DoubleBlockCombiner.combineWithNeigbour` (`DoubleBlockCombiner.java:14-61`).

use std::collections::{BTreeMap, HashSet};

use bbb_protocol::block_state_types::vanilla_block_state_id_by_name_and_properties;
use bbb_protocol::packets::BlockPos;

use crate::block_behavior::chest::{combine, ChestCombine, ChestHalf};

fn chest(facing: &str, kind: &str) -> i32 {
    let properties = BTreeMap::from([
        ("facing".to_string(), facing.to_string()),
        ("type".to_string(), kind.to_string()),
        ("waterlogged".to_string(), "false".to_string()),
    ]);
    vanilla_block_state_id_by_name_and_properties("minecraft:chest", &properties)
        .expect("minecraft:chest is a vanilla 26.1 block")
}

fn named(name: &str) -> i32 {
    vanilla_block_state_id_by_name_and_properties(name, &BTreeMap::new())
        .unwrap_or_else(|| panic!("{name} is a vanilla 26.1 block"))
}

fn pos(x: i32, y: i32, z: i32) -> BlockPos {
    BlockPos { x, y, z }
}

/// A grid that answers air everywhere but the cells it was given.
struct Grid {
    states: BTreeMap<(i32, i32, i32), i32>,
    chests: HashSet<(i32, i32, i32)>,
}

impl Grid {
    fn new() -> Self {
        Self {
            states: BTreeMap::new(),
            chests: HashSet::new(),
        }
    }

    fn block(mut self, at: BlockPos, state: i32) -> Self {
        self.states.insert((at.x, at.y, at.z), state);
        self
    }

    /// A position that carries a live `ChestBlockEntity`, which is what
    /// `entityType.getBlockEntity(level, pos)` (`DoubleBlockCombiner.java:24`)
    /// answers non-null for.
    fn chest_entity(mut self, at: BlockPos, state: i32) -> Self {
        self.chests.insert((at.x, at.y, at.z));
        self.block(at, state)
    }

    fn combine(&self, at: BlockPos) -> ChestCombine {
        let state = self.state(at);
        combine(
            at,
            state,
            &mut |query| self.state(query),
            &mut |query| self.chests.contains(&(query.x, query.y, query.z)),
            &mut |_| false,
        )
    }

    fn state(&self, at: BlockPos) -> i32 {
        self.states
            .get(&(at.x, at.y, at.z))
            .copied()
            .unwrap_or_else(|| named("minecraft:air"))
    }
}

/// `ChestBlock.getBlockType` (`ChestBlock.java:148-155`): `RIGHT` is the
/// *first* half of the compound container and `LEFT` the second, which is the
/// opposite of what the property names suggest.
#[test]
fn the_right_half_is_the_first_container_and_the_left_half_the_second() {
    use crate::block_behavior::chest::block_type;
    assert_eq!(
        block_type(chest("north", "single")),
        Some(ChestHalf::Single)
    );
    assert_eq!(block_type(chest("north", "right")), Some(ChestHalf::First));
    assert_eq!(block_type(chest("north", "left")), Some(ChestHalf::Second));
    assert_eq!(block_type(named("minecraft:stone")), None);
}

/// `ChestBlock.getConnectedDirection` (`ChestBlock.java:199-202`): a `LEFT`
/// half reaches clockwise from its facing, every other half
/// counter-clockwise.
#[test]
fn a_half_reaches_for_its_partner_the_way_vanilla_rotates() {
    use crate::block_behavior::chest::connected_direction;
    use bbb_protocol::packets::Direction;
    assert_eq!(
        connected_direction(chest("north", "left")),
        Some(Direction::East)
    );
    assert_eq!(
        connected_direction(chest("north", "right")),
        Some(Direction::West)
    );
    assert_eq!(
        connected_direction(chest("east", "left")),
        Some(Direction::South)
    );
    assert_eq!(
        connected_direction(chest("east", "right")),
        Some(Direction::North)
    );
}

/// `:36-38`: a single chest answers `Single` without looking at a neighbour at
/// all.
#[test]
fn a_single_chest_is_its_own_container() {
    let at = pos(0, 64, 0);
    let grid = Grid::new().chest_entity(at, chest("north", "single"));
    assert_eq!(grid.combine(at), ChestCombine::Single(at));
}

/// `:40-56`: the two halves must be the same block, the other type, and face
/// the same way; the pair comes back in `first, second` order whichever half
/// was clicked.
#[test]
fn two_matching_halves_combine_into_one_double_chest() {
    let right = pos(0, 64, 0);
    let left = pos(-1, 64, 0);
    let grid = Grid::new()
        .chest_entity(right, chest("north", "right"))
        .chest_entity(left, chest("north", "left"));
    assert_eq!(grid.combine(right), ChestCombine::Double(right, left));
    assert_eq!(
        grid.combine(left),
        ChestCombine::Double(right, left),
        "clicking either half addresses the same compound container"
    );
}

/// `:44-46`: a neighbour facing the other way is not a partner, so the walk
/// falls through to `:60`'s `Single`.
#[test]
fn a_neighbour_that_faces_elsewhere_is_not_a_partner() {
    let right = pos(0, 64, 0);
    let left = pos(-1, 64, 0);
    let grid = Grid::new()
        .chest_entity(right, chest("north", "right"))
        .chest_entity(left, chest("south", "left"));
    assert_eq!(grid.combine(right), ChestCombine::Single(right));
}

/// `:29-31` with `ChestBlock.isBlockedChestByBlock`
/// (`ChestBlock.java:335-338`): a redstone conductor directly above blocks the
/// chest, and it blocks the pair when it sits above either half (`:47-49`).
#[test]
fn a_redstone_conductor_above_either_half_blocks_the_chest() {
    let right = pos(0, 64, 0);
    let left = pos(-1, 64, 0);
    let stone = named("minecraft:stone");
    let glass = named("minecraft:glass");

    let blocked = Grid::new()
        .chest_entity(right, chest("north", "single"))
        .block(pos(0, 65, 0), stone);
    assert_eq!(blocked.combine(right), ChestCombine::None);

    // `isRedstoneConductor` is not `isSolid`: glass is neither, so it does not
    // block.
    let open = Grid::new()
        .chest_entity(right, chest("north", "single"))
        .block(pos(0, 65, 0), glass);
    assert_eq!(open.combine(right), ChestCombine::Single(right));

    let partner_blocked = Grid::new()
        .chest_entity(right, chest("north", "right"))
        .chest_entity(left, chest("north", "left"))
        .block(pos(-1, 65, 0), stone);
    assert_eq!(partner_blocked.combine(right), ChestCombine::None);
}
