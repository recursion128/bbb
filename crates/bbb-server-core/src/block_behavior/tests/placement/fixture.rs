pub(super) use bbb_protocol::block_state_types::{
    vanilla_block_default_state_id, vanilla_block_state_identity,
};
pub(super) use bbb_protocol::block_types::vanilla_block_type_for_resource_id;
pub(super) use bbb_protocol::item_types::vanilla_item_type_for_resource_id;
pub(super) use bbb_server_types::directions::opposite;

pub(super) use super::super::super::placement::{nearest_looking_directions, ordered_by_nearest};

pub(super) use super::super::*;

/// The cell `BlockPlaceContext.getClickedPos` answers, i.e. the one the block
/// goes into. Deliberately not the origin, so a read at the wrong position
/// cannot pass by coincidence.
pub(super) const CLICKED: BlockPos = BlockPos { x: 4, y: 70, z: -9 };

pub(super) fn item_id(name: &str) -> i32 {
    vanilla_item_type_for_resource_id(name)
        .unwrap_or_else(|| panic!("{name} is not a built-in 26.1 item"))
        .id
}

pub(super) fn default_state_of(name: &str) -> i32 {
    let entry = vanilla_block_type_for_resource_id(name)
        .unwrap_or_else(|| panic!("{name} is not a built-in 26.1 block"));
    vanilla_block_default_state_id(entry.id)
        .unwrap_or_else(|| panic!("{name} has no default state"))
}

pub(super) fn context(pos: BlockPos) -> BlockPlacementContext {
    BlockPlacementContext {
        pos,
        clicked_face: Direction::Up,
        click_x: 0.5,
        click_y: 0.5,
        click_z: 0.5,
        replacing_clicked: false,
        horizontal_direction: Direction::South,
        secondary_use_active: false,
        inside: false,
    }
}

pub(super) fn view(x_rot: f32, y_rot: f32) -> BlockPlacementView {
    BlockPlacementView { x_rot, y_rot }
}

/// Places `name` into a world that is air everywhere except the cell below
/// [`CLICKED`], and reports the positions the placement read off the level.
pub(super) fn place(name: &str, below_state: i32) -> (Option<i32>, Vec<BlockPos>) {
    let mut reads = Vec::new();
    let state = state_for_placement(item_id(name), context(CLICKED), vanilla_tags(), &mut |at| {
        reads.push(at);
        if at == below(CLICKED) {
            below_state
        } else {
            air()
        }
    });
    (state, reads)
}

pub(super) fn placement_view(x_rot: f32, y_rot: f32) -> BlockPlacementView {
    BlockPlacementView { x_rot, y_rot }
}

pub(super) fn multiface_state(name: &str, faces: &[Direction], waterlogged: bool) -> i32 {
    let has = |direction| {
        if faces.contains(&direction) {
            "true"
        } else {
            "false"
        }
    };
    state_of(
        name,
        &[
            ("down", has(Direction::Down)),
            ("up", has(Direction::Up)),
            ("north", has(Direction::North)),
            ("south", has(Direction::South)),
            ("west", has(Direction::West)),
            ("east", has(Direction::East)),
            ("waterlogged", if waterlogged { "true" } else { "false" }),
        ],
    )
}

pub(super) fn vine_state(faces: &[Direction]) -> i32 {
    let has = |direction| {
        if faces.contains(&direction) {
            "true"
        } else {
            "false"
        }
    };
    state_of(
        "minecraft:vine",
        &[
            ("east", has(Direction::East)),
            ("north", has(Direction::North)),
            ("south", has(Direction::South)),
            ("up", has(Direction::Up)),
            ("west", has(Direction::West)),
        ],
    )
}

pub(super) fn chorus_plant_state(faces: &[Direction]) -> i32 {
    let has = |direction| {
        if faces.contains(&direction) {
            "true"
        } else {
            "false"
        }
    };
    state_of(
        "minecraft:chorus_plant",
        &[
            ("down", has(Direction::Down)),
            ("up", has(Direction::Up)),
            ("north", has(Direction::North)),
            ("east", has(Direction::East)),
            ("south", has(Direction::South)),
            ("west", has(Direction::West)),
        ],
    )
}

pub(super) fn pointed_dripstone_state(direction: &str, thickness: &str, waterlogged: bool) -> i32 {
    state_of(
        "minecraft:pointed_dripstone",
        &[
            ("thickness", thickness),
            ("vertical_direction", direction),
            ("waterlogged", if waterlogged { "true" } else { "false" }),
        ],
    )
}

pub(super) fn sea_pickle_state(pickles: &str, waterlogged: bool) -> i32 {
    state_of(
        "minecraft:sea_pickle",
        &[
            ("pickles", pickles),
            ("waterlogged", if waterlogged { "true" } else { "false" }),
        ],
    )
}

pub(super) fn turtle_egg_state(eggs: &str, hatch: &str) -> i32 {
    state_of("minecraft:turtle_egg", &[("eggs", eggs), ("hatch", hatch)])
}

pub(super) fn chest_state(name: &str, facing: &str, chest_type: &str, waterlogged: bool) -> i32 {
    state_of(
        name,
        &[
            ("facing", facing),
            ("type", chest_type),
            ("waterlogged", if waterlogged { "true" } else { "false" }),
        ],
    )
}
