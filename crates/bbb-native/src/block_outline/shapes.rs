use std::collections::BTreeMap;

use super::{BlockOutlineBox, BlockOutlineShape, HorizontalDirection};

pub(super) fn outline_shape_for_block(
    block_name: Option<&str>,
    properties: &BTreeMap<String, String>,
) -> Option<BlockOutlineShape> {
    let block_name = block_name?;
    if block_name == "minecraft:snow" {
        return snow_layer_outline_box(properties).map(BlockOutlineShape::single);
    }
    if is_slab_block_name(block_name) {
        return match properties.get("type").map(String::as_str) {
            Some("bottom") => Some(BlockOutlineShape::single(BlockOutlineBox::BOTTOM_SLAB)),
            Some("top") => Some(BlockOutlineShape::single(BlockOutlineBox::TOP_SLAB)),
            Some("double") => Some(BlockOutlineShape::single(BlockOutlineBox::FULL)),
            _ => None,
        };
    }
    if is_pressure_plate_block_name(block_name) {
        return pressure_plate_outline_shape(properties);
    }
    if is_rail_block_name(block_name) {
        return rail_outline_shape(properties);
    }
    if is_cauldron_block_name(block_name) {
        return Some(cauldron_outline_shape());
    }
    if block_name == "minecraft:hopper" {
        return hopper_outline_shape(properties);
    }
    if is_campfire_block_name(block_name) {
        return Some(BlockOutlineShape::single(BlockOutlineBox::centered_column(
            16.0, 16.0, 0.0, 7.0,
        )));
    }
    if is_chain_block_name(block_name) {
        return chain_outline_shape(properties);
    }
    if block_name == "minecraft:ender_chest" {
        return Some(BlockOutlineShape::single(BlockOutlineBox::CHEST_SINGLE));
    }
    if is_standard_chest_block_name(block_name) {
        return chest_outline_shape(properties);
    }
    if is_bed_block_name(block_name) {
        return bed_outline_shape(properties);
    }
    if block_name == "minecraft:cake" {
        return cake_outline_shape(properties);
    }
    if is_candle_cake_block_name(block_name) {
        return Some(candle_cake_outline_shape());
    }
    if block_name == "minecraft:brewing_stand" {
        return Some(brewing_stand_outline_shape());
    }
    if block_name == "minecraft:enchanting_table" {
        return Some(BlockOutlineShape::single(BlockOutlineBox::centered_column(
            16.0, 16.0, 0.0, 12.0,
        )));
    }
    if block_name == "minecraft:stonecutter" {
        return Some(BlockOutlineShape::single(BlockOutlineBox::centered_column(
            16.0, 16.0, 0.0, 9.0,
        )));
    }
    if is_anvil_block_name(block_name) {
        return anvil_outline_shape(properties);
    }
    if block_name == "minecraft:lectern" {
        return lectern_outline_shape(properties);
    }
    if is_flower_pot_block_name(block_name) {
        return Some(BlockOutlineShape::single(BlockOutlineBox::FLOWER_POT));
    }
    if is_sapling_block_name(block_name) {
        return Some(BlockOutlineShape::single(BlockOutlineBox::SAPLING));
    }
    if let Some(outline) = simple_vegetation_outline_box(block_name) {
        return Some(BlockOutlineShape::single(outline));
    }
    if is_crop_block_name(block_name) {
        return crop_outline_shape(block_name, properties);
    }
    if is_wall_sign_block_name(block_name) {
        return wall_sign_outline_shape(properties);
    }
    if is_standing_sign_block_name(block_name) {
        return Some(BlockOutlineShape::single(BlockOutlineBox::SIGN));
    }
    if is_stair_block_name(block_name) {
        return stair_outline_shape(properties);
    }
    if is_trapdoor_block_name(block_name) {
        return trapdoor_outline_shape(properties);
    }
    if is_door_block_name(block_name) {
        return door_outline_shape(properties);
    }
    if block_name == "minecraft:ladder" {
        return ladder_outline_shape(properties);
    }
    if is_wall_torch_block_name(block_name) {
        return wall_torch_outline_shape(properties);
    }
    if is_floor_torch_block_name(block_name) {
        return Some(BlockOutlineShape::single(BlockOutlineBox::TORCH));
    }
    if is_button_block_name(block_name) {
        return button_outline_shape(properties);
    }
    if block_name == "minecraft:lever" {
        return lever_outline_shape(properties);
    }
    if is_fence_gate_block_name(block_name) {
        return fence_gate_outline_shape(properties);
    }
    if is_fence_block_name(block_name) {
        return fence_outline_shape(properties);
    }
    if is_pane_block_name(block_name) {
        return pane_outline_shape(properties);
    }
    if is_wall_block_name(block_name) {
        return wall_outline_shape(properties);
    }
    if block_name == "minecraft:pale_moss_carpet" {
        return pale_moss_carpet_outline_shape(properties);
    }
    if is_flat_carpet_block_name(block_name) {
        return Some(BlockOutlineShape::single(BlockOutlineBox::CARPET));
    }
    Some(BlockOutlineShape::single(BlockOutlineBox::FULL))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum StairShapeKind {
    Straight,
    Outer,
    Inner,
}

fn stair_outline_shape(properties: &BTreeMap<String, String>) -> Option<BlockOutlineShape> {
    let facing = HorizontalDirection::parse(properties.get("facing")?)?;
    let top = match properties.get("half").map(String::as_str)? {
        "bottom" => false,
        "top" => true,
        _ => return None,
    };
    let (kind, direction) = match properties.get("shape").map(String::as_str)? {
        "straight" => (StairShapeKind::Straight, facing),
        "outer_left" => (StairShapeKind::Outer, facing),
        "outer_right" => (StairShapeKind::Outer, facing.clockwise()),
        "inner_left" => (StairShapeKind::Inner, facing.counter_clockwise()),
        "inner_right" => (StairShapeKind::Inner, facing),
        _ => return None,
    };

    Some(BlockOutlineShape::from_boxes(
        stair_shape_boxes(kind, top)
            .into_iter()
            .map(|outline| outline.rotate_to_direction(direction))
            .collect(),
    ))
}

fn stair_shape_boxes(kind: StairShapeKind, top: bool) -> Vec<BlockOutlineBox> {
    let mut boxes = match kind {
        StairShapeKind::Straight => vec![
            BlockOutlineBox::BOTTOM_SLAB,
            BlockOutlineBox::STAIR_NORTH_HALF,
        ],
        StairShapeKind::Outer => vec![
            BlockOutlineBox::BOTTOM_SLAB,
            BlockOutlineBox::STAIR_NORTH_WEST_OCTET,
        ],
        StairShapeKind::Inner => vec![
            BlockOutlineBox::BOTTOM_SLAB,
            BlockOutlineBox::STAIR_NORTH_HALF,
            BlockOutlineBox::STAIR_SOUTH_EAST_OCTET,
        ],
    };

    if top {
        for outline in &mut boxes {
            *outline = outline.invert_y();
        }
    }
    boxes
}

fn pressure_plate_outline_shape(
    properties: &BTreeMap<String, String>,
) -> Option<BlockOutlineShape> {
    let pressed = if let Some(powered) = properties.get("powered").map(String::as_str) {
        match powered {
            "true" => true,
            "false" => false,
            _ => return None,
        }
    } else {
        let power = properties.get("power")?.parse::<u8>().ok()?;
        if power > 15 {
            return None;
        }
        power > 0
    };

    let outline = if pressed {
        BlockOutlineBox::PRESSURE_PLATE_PRESSED
    } else {
        BlockOutlineBox::PRESSURE_PLATE
    };
    Some(BlockOutlineShape::single(outline))
}

fn rail_outline_shape(properties: &BTreeMap<String, String>) -> Option<BlockOutlineShape> {
    let outline = match properties.get("shape").map(String::as_str)? {
        "ascending_north" | "ascending_east" | "ascending_south" | "ascending_west" => {
            BlockOutlineBox::RAIL_SLOPE
        }
        "north_south" | "east_west" | "south_east" | "south_west" | "north_west" | "north_east" => {
            BlockOutlineBox::RAIL_FLAT
        }
        _ => return None,
    };
    Some(BlockOutlineShape::single(outline))
}

fn cauldron_outline_shape() -> BlockOutlineShape {
    BlockOutlineShape::from_boxes(vec![
        BlockOutlineBox::from_pixels([0.0, 0.0, 0.0], [2.0, 3.0, 4.0]),
        BlockOutlineBox::from_pixels([0.0, 0.0, 12.0], [2.0, 3.0, 16.0]),
        BlockOutlineBox::from_pixels([14.0, 0.0, 0.0], [16.0, 3.0, 4.0]),
        BlockOutlineBox::from_pixels([14.0, 0.0, 12.0], [16.0, 3.0, 16.0]),
        BlockOutlineBox::from_pixels([2.0, 0.0, 0.0], [4.0, 3.0, 2.0]),
        BlockOutlineBox::from_pixels([2.0, 0.0, 14.0], [4.0, 3.0, 16.0]),
        BlockOutlineBox::from_pixels([12.0, 0.0, 0.0], [14.0, 3.0, 2.0]),
        BlockOutlineBox::from_pixels([12.0, 0.0, 14.0], [14.0, 3.0, 16.0]),
        BlockOutlineBox::from_pixels([0.0, 3.0, 0.0], [16.0, 4.0, 16.0]),
        BlockOutlineBox::from_pixels([0.0, 4.0, 0.0], [2.0, 16.0, 16.0]),
        BlockOutlineBox::from_pixels([14.0, 4.0, 0.0], [16.0, 16.0, 16.0]),
        BlockOutlineBox::from_pixels([2.0, 4.0, 0.0], [14.0, 16.0, 2.0]),
        BlockOutlineBox::from_pixels([2.0, 4.0, 14.0], [14.0, 16.0, 16.0]),
    ])
}

fn hopper_outline_shape(properties: &BTreeMap<String, String>) -> Option<BlockOutlineShape> {
    let spout = match properties.get("facing").map(String::as_str)? {
        "down" => BlockOutlineBox::from_pixels([6.0, 0.0, 6.0], [10.0, 6.0, 10.0]),
        "north" => BlockOutlineBox::from_pixels([6.0, 4.0, 0.0], [10.0, 8.0, 8.0]),
        "east" => BlockOutlineBox::from_pixels([8.0, 4.0, 6.0], [16.0, 8.0, 10.0]),
        "south" => BlockOutlineBox::from_pixels([6.0, 4.0, 8.0], [10.0, 8.0, 16.0]),
        "west" => BlockOutlineBox::from_pixels([0.0, 4.0, 6.0], [8.0, 8.0, 10.0]),
        _ => return None,
    };

    let mut boxes = spoutless_hopper_boxes();
    boxes.push(spout);
    Some(BlockOutlineShape::from_boxes(boxes))
}

fn spoutless_hopper_boxes() -> Vec<BlockOutlineBox> {
    vec![
        BlockOutlineBox::from_pixels([0.0, 10.0, 0.0], [16.0, 11.0, 16.0]),
        BlockOutlineBox::from_pixels([0.0, 11.0, 0.0], [16.0, 16.0, 2.0]),
        BlockOutlineBox::from_pixels([0.0, 11.0, 14.0], [16.0, 16.0, 16.0]),
        BlockOutlineBox::from_pixels([0.0, 11.0, 2.0], [2.0, 16.0, 14.0]),
        BlockOutlineBox::from_pixels([14.0, 11.0, 2.0], [16.0, 16.0, 14.0]),
        BlockOutlineBox::centered_column(8.0, 8.0, 4.0, 10.0),
    ]
}

fn chain_outline_shape(properties: &BTreeMap<String, String>) -> Option<BlockOutlineShape> {
    let outline = match properties.get("axis").map(String::as_str)? {
        "x" => BlockOutlineBox::from_pixels([0.0, 6.5, 6.5], [16.0, 9.5, 9.5]),
        "y" => BlockOutlineBox::from_pixels([6.5, 0.0, 6.5], [9.5, 16.0, 9.5]),
        "z" => BlockOutlineBox::from_pixels([6.5, 6.5, 0.0], [9.5, 9.5, 16.0]),
        _ => return None,
    };
    Some(BlockOutlineShape::single(outline))
}

fn chest_outline_shape(properties: &BTreeMap<String, String>) -> Option<BlockOutlineShape> {
    let outline = match properties.get("type").map(String::as_str)? {
        "single" => BlockOutlineBox::CHEST_SINGLE,
        "left" => {
            let connected = HorizontalDirection::parse(properties.get("facing")?)?.clockwise();
            chest_connected_outline(connected)
        }
        "right" => {
            let connected =
                HorizontalDirection::parse(properties.get("facing")?)?.counter_clockwise();
            chest_connected_outline(connected)
        }
        _ => return None,
    };
    Some(BlockOutlineShape::single(outline))
}

fn chest_connected_outline(connected: HorizontalDirection) -> BlockOutlineBox {
    match connected {
        HorizontalDirection::North => BlockOutlineBox::CHEST_CONNECTED_NORTH,
        HorizontalDirection::East => BlockOutlineBox::CHEST_CONNECTED_EAST,
        HorizontalDirection::South => BlockOutlineBox::CHEST_CONNECTED_SOUTH,
        HorizontalDirection::West => BlockOutlineBox::CHEST_CONNECTED_WEST,
    }
}

fn bed_outline_shape(properties: &BTreeMap<String, String>) -> Option<BlockOutlineShape> {
    let facing = HorizontalDirection::parse(properties.get("facing")?)?;
    let connected = match properties.get("part").map(String::as_str)? {
        "head" => facing.opposite(),
        "foot" => facing,
        _ => return None,
    };
    Some(bed_shape_for_direction(connected.opposite()))
}

fn bed_shape_for_direction(direction: HorizontalDirection) -> BlockOutlineShape {
    let (left_leg, right_leg) = match direction {
        HorizontalDirection::North => (
            BlockOutlineBox::from_pixels([0.0, 0.0, 0.0], [3.0, 3.0, 3.0]),
            BlockOutlineBox::from_pixels([13.0, 0.0, 0.0], [16.0, 3.0, 3.0]),
        ),
        HorizontalDirection::East => (
            BlockOutlineBox::from_pixels([13.0, 0.0, 0.0], [16.0, 3.0, 3.0]),
            BlockOutlineBox::from_pixels([13.0, 0.0, 13.0], [16.0, 3.0, 16.0]),
        ),
        HorizontalDirection::South => (
            BlockOutlineBox::from_pixels([13.0, 0.0, 13.0], [16.0, 3.0, 16.0]),
            BlockOutlineBox::from_pixels([0.0, 0.0, 13.0], [3.0, 3.0, 16.0]),
        ),
        HorizontalDirection::West => (
            BlockOutlineBox::from_pixels([0.0, 0.0, 13.0], [3.0, 3.0, 16.0]),
            BlockOutlineBox::from_pixels([0.0, 0.0, 0.0], [3.0, 3.0, 3.0]),
        ),
    };
    BlockOutlineShape::from_boxes(vec![BlockOutlineBox::BED_PLATFORM, left_leg, right_leg])
}

fn brewing_stand_outline_shape() -> BlockOutlineShape {
    BlockOutlineShape::from_boxes(vec![
        BlockOutlineBox::BREWING_STAND_ROD,
        BlockOutlineBox::BREWING_STAND_BASE,
    ])
}

fn cake_outline_shape(properties: &BTreeMap<String, String>) -> Option<BlockOutlineShape> {
    let bites = properties.get("bites")?.parse::<u8>().ok()?;
    if bites > 6 {
        return None;
    }

    Some(BlockOutlineShape::single(BlockOutlineBox::from_pixels(
        [1.0 + f64::from(bites) * 2.0, 0.0, 1.0],
        [15.0, 8.0, 15.0],
    )))
}

fn candle_cake_outline_shape() -> BlockOutlineShape {
    BlockOutlineShape::from_boxes(vec![
        BlockOutlineBox::centered_column(2.0, 2.0, 8.0, 14.0),
        BlockOutlineBox::centered_column(14.0, 14.0, 0.0, 8.0),
    ])
}

fn anvil_outline_shape(properties: &BTreeMap<String, String>) -> Option<BlockOutlineShape> {
    let facing = HorizontalDirection::parse(properties.get("facing")?)?;
    let x_axis = matches!(
        facing,
        HorizontalDirection::East | HorizontalDirection::West
    );
    let boxes = if x_axis {
        vec![
            BlockOutlineBox::centered_column(12.0, 12.0, 0.0, 4.0),
            BlockOutlineBox::centered_column(10.0, 8.0, 4.0, 5.0),
            BlockOutlineBox::centered_column(8.0, 4.0, 5.0, 10.0),
            BlockOutlineBox::centered_column(16.0, 10.0, 10.0, 16.0),
        ]
    } else {
        vec![
            BlockOutlineBox::centered_column(12.0, 12.0, 0.0, 4.0),
            BlockOutlineBox::centered_column(8.0, 10.0, 4.0, 5.0),
            BlockOutlineBox::centered_column(4.0, 8.0, 5.0, 10.0),
            BlockOutlineBox::centered_column(10.0, 16.0, 10.0, 16.0),
        ]
    };
    Some(BlockOutlineShape::from_boxes(boxes))
}

fn lectern_outline_shape(properties: &BTreeMap<String, String>) -> Option<BlockOutlineShape> {
    let facing = HorizontalDirection::parse(properties.get("facing")?)?;
    Some(BlockOutlineShape::from_boxes(
        lectern_north_boxes()
            .into_iter()
            .map(|outline| outline.rotate_to_direction(facing))
            .collect(),
    ))
}

fn lectern_north_boxes() -> Vec<BlockOutlineBox> {
    vec![
        BlockOutlineBox::centered_column(16.0, 16.0, 0.0, 2.0),
        BlockOutlineBox::centered_column(8.0, 8.0, 2.0, 14.0),
        BlockOutlineBox::from_pixels([0.0, 10.0, 1.0], [16.0, 14.0, 5.333333]),
        BlockOutlineBox::from_pixels([0.0, 12.0, 5.333333], [16.0, 16.0, 9.666667]),
        BlockOutlineBox::from_pixels([0.0, 14.0, 9.666667], [16.0, 18.0, 14.0]),
    ]
}

fn wall_sign_outline_shape(properties: &BTreeMap<String, String>) -> Option<BlockOutlineShape> {
    let outline = match HorizontalDirection::parse(properties.get("facing")?)? {
        HorizontalDirection::North => BlockOutlineBox::WALL_SIGN_NORTH,
        HorizontalDirection::East => BlockOutlineBox::WALL_SIGN_EAST,
        HorizontalDirection::South => BlockOutlineBox::WALL_SIGN_SOUTH,
        HorizontalDirection::West => BlockOutlineBox::WALL_SIGN_WEST,
    };
    Some(BlockOutlineShape::single(outline))
}

fn fence_outline_shape(properties: &BTreeMap<String, String>) -> Option<BlockOutlineShape> {
    let mut boxes = Vec::with_capacity(5);
    boxes.push(BlockOutlineBox::FENCE_POST);

    for (property, outline) in [
        ("north", BlockOutlineBox::FENCE_NORTH_ARM),
        ("east", BlockOutlineBox::FENCE_EAST_ARM),
        ("south", BlockOutlineBox::FENCE_SOUTH_ARM),
        ("west", BlockOutlineBox::FENCE_WEST_ARM),
    ] {
        match properties.get(property).map(String::as_str)? {
            "true" => boxes.push(outline),
            "false" => {}
            _ => return None,
        }
    }

    Some(BlockOutlineShape::from_boxes(boxes))
}

fn fence_gate_outline_shape(properties: &BTreeMap<String, String>) -> Option<BlockOutlineShape> {
    let facing = HorizontalDirection::parse(properties.get("facing")?)?;
    let in_wall = match properties.get("in_wall").map(String::as_str)? {
        "true" => true,
        "false" => false,
        _ => return None,
    };

    let outline = match (
        matches!(
            facing,
            HorizontalDirection::East | HorizontalDirection::West
        ),
        in_wall,
    ) {
        (true, true) => BlockOutlineBox::FENCE_GATE_X_IN_WALL,
        (true, false) => BlockOutlineBox::FENCE_GATE_X,
        (false, true) => BlockOutlineBox::FENCE_GATE_Z_IN_WALL,
        (false, false) => BlockOutlineBox::FENCE_GATE_Z,
    };
    Some(BlockOutlineShape::single(outline))
}

fn pane_outline_shape(properties: &BTreeMap<String, String>) -> Option<BlockOutlineShape> {
    let mut boxes = Vec::with_capacity(5);
    boxes.push(BlockOutlineBox::PANE_POST);

    for (property, outline) in [
        ("north", BlockOutlineBox::PANE_NORTH_ARM),
        ("east", BlockOutlineBox::PANE_EAST_ARM),
        ("south", BlockOutlineBox::PANE_SOUTH_ARM),
        ("west", BlockOutlineBox::PANE_WEST_ARM),
    ] {
        match properties.get(property).map(String::as_str)? {
            "true" => boxes.push(outline),
            "false" => {}
            _ => return None,
        }
    }

    Some(BlockOutlineShape::from_boxes(boxes))
}

fn wall_outline_shape(properties: &BTreeMap<String, String>) -> Option<BlockOutlineShape> {
    let mut boxes = Vec::with_capacity(5);
    match properties.get("up").map(String::as_str)? {
        "true" => boxes.push(BlockOutlineBox::WALL_POST),
        "false" => {}
        _ => return None,
    }

    for (property, low, tall) in [
        (
            "north",
            BlockOutlineBox::WALL_NORTH_LOW,
            BlockOutlineBox::WALL_NORTH_TALL,
        ),
        (
            "east",
            BlockOutlineBox::WALL_EAST_LOW,
            BlockOutlineBox::WALL_EAST_TALL,
        ),
        (
            "south",
            BlockOutlineBox::WALL_SOUTH_LOW,
            BlockOutlineBox::WALL_SOUTH_TALL,
        ),
        (
            "west",
            BlockOutlineBox::WALL_WEST_LOW,
            BlockOutlineBox::WALL_WEST_TALL,
        ),
    ] {
        match properties.get(property).map(String::as_str)? {
            "none" => {}
            "low" => boxes.push(low),
            "tall" => boxes.push(tall),
            _ => return None,
        }
    }

    Some(BlockOutlineShape::from_boxes(boxes))
}

fn trapdoor_outline_shape(properties: &BTreeMap<String, String>) -> Option<BlockOutlineShape> {
    let open = match properties.get("open").map(String::as_str)? {
        "true" => true,
        "false" => false,
        _ => return None,
    };

    let outline = if open {
        match HorizontalDirection::parse(properties.get("facing")?)? {
            HorizontalDirection::North => BlockOutlineBox::TRAPDOOR_NORTH_OPEN,
            HorizontalDirection::East => BlockOutlineBox::TRAPDOOR_EAST_OPEN,
            HorizontalDirection::South => BlockOutlineBox::TRAPDOOR_SOUTH_OPEN,
            HorizontalDirection::West => BlockOutlineBox::TRAPDOOR_WEST_OPEN,
        }
    } else {
        match properties.get("half").map(String::as_str)? {
            "bottom" => BlockOutlineBox::TRAPDOOR_BOTTOM,
            "top" => BlockOutlineBox::TRAPDOOR_TOP,
            _ => return None,
        }
    };

    Some(BlockOutlineShape::single(outline))
}

fn door_outline_shape(properties: &BTreeMap<String, String>) -> Option<BlockOutlineShape> {
    let facing = HorizontalDirection::parse(properties.get("facing")?)?;
    let open = match properties.get("open").map(String::as_str)? {
        "true" => true,
        "false" => false,
        _ => return None,
    };
    let direction = if open {
        match properties.get("hinge").map(String::as_str)? {
            "left" => facing.clockwise(),
            "right" => facing.counter_clockwise(),
            _ => return None,
        }
    } else {
        facing
    };

    let outline = match direction {
        HorizontalDirection::North => BlockOutlineBox::DOOR_NORTH,
        HorizontalDirection::East => BlockOutlineBox::DOOR_EAST,
        HorizontalDirection::South => BlockOutlineBox::DOOR_SOUTH,
        HorizontalDirection::West => BlockOutlineBox::DOOR_WEST,
    };
    Some(BlockOutlineShape::single(outline))
}

fn ladder_outline_shape(properties: &BTreeMap<String, String>) -> Option<BlockOutlineShape> {
    let outline = match HorizontalDirection::parse(properties.get("facing")?)? {
        HorizontalDirection::North => BlockOutlineBox::LADDER_NORTH,
        HorizontalDirection::East => BlockOutlineBox::LADDER_EAST,
        HorizontalDirection::South => BlockOutlineBox::LADDER_SOUTH,
        HorizontalDirection::West => BlockOutlineBox::LADDER_WEST,
    };
    Some(BlockOutlineShape::single(outline))
}

fn wall_torch_outline_shape(properties: &BTreeMap<String, String>) -> Option<BlockOutlineShape> {
    let outline = match HorizontalDirection::parse(properties.get("facing")?)? {
        HorizontalDirection::North => BlockOutlineBox::WALL_TORCH_NORTH,
        HorizontalDirection::East => BlockOutlineBox::WALL_TORCH_EAST,
        HorizontalDirection::South => BlockOutlineBox::WALL_TORCH_SOUTH,
        HorizontalDirection::West => BlockOutlineBox::WALL_TORCH_WEST,
    };
    Some(BlockOutlineShape::single(outline))
}

fn button_outline_shape(properties: &BTreeMap<String, String>) -> Option<BlockOutlineShape> {
    let facing = HorizontalDirection::parse(properties.get("facing")?)?;
    let powered = match properties.get("powered").map(String::as_str)? {
        "true" => true,
        "false" => false,
        _ => return None,
    };

    let outline = match (properties.get("face").map(String::as_str)?, powered) {
        ("wall", false) => BlockOutlineBox::BUTTON_WALL_NORTH,
        ("wall", true) => BlockOutlineBox::BUTTON_WALL_NORTH_PRESSED,
        ("floor", false) => BlockOutlineBox::BUTTON_FLOOR_NORTH,
        ("floor", true) => BlockOutlineBox::BUTTON_FLOOR_NORTH_PRESSED,
        ("ceiling", false) => BlockOutlineBox::BUTTON_CEILING_NORTH,
        ("ceiling", true) => BlockOutlineBox::BUTTON_CEILING_NORTH_PRESSED,
        _ => return None,
    };

    Some(BlockOutlineShape::single(
        outline.rotate_to_direction(facing),
    ))
}

fn lever_outline_shape(properties: &BTreeMap<String, String>) -> Option<BlockOutlineShape> {
    let facing = HorizontalDirection::parse(properties.get("facing")?)?;
    let outline = match properties.get("face").map(String::as_str)? {
        "wall" => BlockOutlineBox::LEVER_WALL_NORTH,
        "floor" => BlockOutlineBox::LEVER_FLOOR_NORTH,
        "ceiling" => BlockOutlineBox::LEVER_CEILING_NORTH,
        _ => return None,
    };

    Some(BlockOutlineShape::single(
        outline.rotate_to_direction(facing),
    ))
}

fn pale_moss_carpet_outline_shape(
    properties: &BTreeMap<String, String>,
) -> Option<BlockOutlineShape> {
    let mut boxes = Vec::with_capacity(5);
    match properties.get("bottom").map(String::as_str)? {
        "true" => boxes.push(BlockOutlineBox::CARPET),
        "false" => {}
        _ => return None,
    }

    for (property, low, tall) in [
        (
            "north",
            BlockOutlineBox::PALE_MOSS_NORTH_LOW,
            BlockOutlineBox::PALE_MOSS_NORTH_TALL,
        ),
        (
            "east",
            BlockOutlineBox::PALE_MOSS_EAST_LOW,
            BlockOutlineBox::PALE_MOSS_EAST_TALL,
        ),
        (
            "south",
            BlockOutlineBox::PALE_MOSS_SOUTH_LOW,
            BlockOutlineBox::PALE_MOSS_SOUTH_TALL,
        ),
        (
            "west",
            BlockOutlineBox::PALE_MOSS_WEST_LOW,
            BlockOutlineBox::PALE_MOSS_WEST_TALL,
        ),
    ] {
        match properties.get(property).map(String::as_str)? {
            "none" => {}
            "low" => boxes.push(low),
            "tall" => boxes.push(tall),
            _ => return None,
        }
    }

    if boxes.is_empty() {
        Some(BlockOutlineShape::single(BlockOutlineBox::FULL))
    } else {
        Some(BlockOutlineShape::from_boxes(boxes))
    }
}

fn crop_outline_shape(
    block_name: &str,
    properties: &BTreeMap<String, String>,
) -> Option<BlockOutlineShape> {
    let outline = match block_name {
        "minecraft:wheat" => {
            let age = age_property(properties, 7)?;
            BlockOutlineBox::centered_column(16.0, 16.0, 0.0, f64::from(2 + age * 2))
        }
        "minecraft:carrots" | "minecraft:potatoes" => {
            let age = age_property(properties, 7)?;
            BlockOutlineBox::centered_column(16.0, 16.0, 0.0, f64::from(2 + age))
        }
        "minecraft:beetroots" => {
            let age = age_property(properties, 3)?;
            BlockOutlineBox::centered_column(16.0, 16.0, 0.0, f64::from(2 + age * 2))
        }
        "minecraft:nether_wart" => {
            let age = age_property(properties, 3)?;
            BlockOutlineBox::centered_column(16.0, 16.0, 0.0, f64::from(5 + age * 3))
        }
        "minecraft:torchflower_crop" => {
            let age = age_property(properties, 1)?;
            BlockOutlineBox::centered_column(6.0, 6.0, 0.0, f64::from(6 + age * 4))
        }
        "minecraft:sweet_berry_bush" => match age_property(properties, 3)? {
            0 => BlockOutlineBox::SWEET_BERRY_SAPLING,
            3 => BlockOutlineBox::FULL,
            _ => BlockOutlineBox::SWEET_BERRY_GROWING,
        },
        _ => return None,
    };

    Some(BlockOutlineShape::single(outline))
}

fn age_property(properties: &BTreeMap<String, String>, max_age: u8) -> Option<u8> {
    let age = properties.get("age")?.parse::<u8>().ok()?;
    (age <= max_age).then_some(age)
}

fn is_crop_block_name(block_name: &str) -> bool {
    matches!(
        block_name,
        "minecraft:wheat"
            | "minecraft:carrots"
            | "minecraft:potatoes"
            | "minecraft:beetroots"
            | "minecraft:nether_wart"
            | "minecraft:torchflower_crop"
            | "minecraft:sweet_berry_bush"
    )
}

fn snow_layer_outline_box(properties: &BTreeMap<String, String>) -> Option<BlockOutlineBox> {
    let layers = properties.get("layers")?.parse::<u8>().ok()?;
    if !(1..=8).contains(&layers) {
        return None;
    }
    Some(BlockOutlineBox {
        min: [0.0, 0.0, 0.0],
        max: [1.0, f64::from(layers) / 8.0, 1.0],
    })
}

fn is_slab_block_name(block_name: &str) -> bool {
    block_name
        .strip_prefix("minecraft:")
        .is_some_and(|path| path.ends_with("_slab"))
}

fn is_stair_block_name(block_name: &str) -> bool {
    block_name
        .strip_prefix("minecraft:")
        .is_some_and(|path| path.ends_with("_stairs"))
}

fn is_pressure_plate_block_name(block_name: &str) -> bool {
    block_name
        .strip_prefix("minecraft:")
        .is_some_and(|path| path.ends_with("_pressure_plate"))
}

fn is_rail_block_name(block_name: &str) -> bool {
    matches!(
        block_name,
        "minecraft:rail"
            | "minecraft:powered_rail"
            | "minecraft:detector_rail"
            | "minecraft:activator_rail"
    )
}

fn is_cauldron_block_name(block_name: &str) -> bool {
    matches!(
        block_name,
        "minecraft:cauldron"
            | "minecraft:water_cauldron"
            | "minecraft:lava_cauldron"
            | "minecraft:powder_snow_cauldron"
    )
}

fn is_standard_chest_block_name(block_name: &str) -> bool {
    matches!(block_name, "minecraft:chest" | "minecraft:trapped_chest")
}

fn is_bed_block_name(block_name: &str) -> bool {
    block_name
        .strip_prefix("minecraft:")
        .is_some_and(|path| path.ends_with("_bed"))
}

fn is_anvil_block_name(block_name: &str) -> bool {
    matches!(
        block_name,
        "minecraft:anvil" | "minecraft:chipped_anvil" | "minecraft:damaged_anvil"
    )
}

fn is_campfire_block_name(block_name: &str) -> bool {
    matches!(block_name, "minecraft:campfire" | "minecraft:soul_campfire")
}

fn is_chain_block_name(block_name: &str) -> bool {
    matches!(
        block_name,
        "minecraft:iron_chain"
            | "minecraft:copper_chain"
            | "minecraft:exposed_copper_chain"
            | "minecraft:weathered_copper_chain"
            | "minecraft:oxidized_copper_chain"
            | "minecraft:waxed_copper_chain"
            | "minecraft:waxed_exposed_copper_chain"
            | "minecraft:waxed_weathered_copper_chain"
            | "minecraft:waxed_oxidized_copper_chain"
    )
}

fn is_candle_cake_block_name(block_name: &str) -> bool {
    block_name
        .strip_prefix("minecraft:")
        .is_some_and(|path| path == "candle_cake" || path.ends_with("_candle_cake"))
}

fn is_flower_pot_block_name(block_name: &str) -> bool {
    block_name
        .strip_prefix("minecraft:")
        .is_some_and(|path| path == "flower_pot" || path.starts_with("potted_"))
}

fn is_sapling_block_name(block_name: &str) -> bool {
    block_name
        .strip_prefix("minecraft:")
        .is_some_and(|path| path.ends_with("_sapling") && path != "bamboo_sapling")
}

fn simple_vegetation_outline_box(block_name: &str) -> Option<BlockOutlineBox> {
    match block_name {
        "minecraft:short_grass" | "minecraft:fern" | "minecraft:dead_bush" => {
            Some(BlockOutlineBox::GROUND_VEGETATION)
        }
        "minecraft:bush" => Some(BlockOutlineBox::BUSH),
        "minecraft:short_dry_grass" => Some(BlockOutlineBox::SHORT_DRY_GRASS),
        "minecraft:tall_dry_grass" => Some(BlockOutlineBox::TALL_DRY_GRASS),
        "minecraft:cactus" => Some(BlockOutlineBox::CACTUS),
        "minecraft:sugar_cane" => Some(BlockOutlineBox::SUGAR_CANE),
        "minecraft:lily_pad" => Some(BlockOutlineBox::LILY_PAD),
        _ => None,
    }
}

fn is_wall_sign_block_name(block_name: &str) -> bool {
    block_name
        .strip_prefix("minecraft:")
        .is_some_and(|path| path.ends_with("_wall_sign"))
}

fn is_standing_sign_block_name(block_name: &str) -> bool {
    block_name
        .strip_prefix("minecraft:")
        .is_some_and(|path| path.ends_with("_sign") && !path.ends_with("_hanging_sign"))
}

fn is_trapdoor_block_name(block_name: &str) -> bool {
    block_name
        .strip_prefix("minecraft:")
        .is_some_and(|path| path.ends_with("_trapdoor"))
}

fn is_door_block_name(block_name: &str) -> bool {
    block_name
        .strip_prefix("minecraft:")
        .is_some_and(|path| path.ends_with("_door"))
}

fn is_fence_gate_block_name(block_name: &str) -> bool {
    block_name
        .strip_prefix("minecraft:")
        .is_some_and(|path| path.ends_with("_fence_gate"))
}

fn is_floor_torch_block_name(block_name: &str) -> bool {
    matches!(
        block_name,
        "minecraft:torch"
            | "minecraft:redstone_torch"
            | "minecraft:soul_torch"
            | "minecraft:copper_torch"
    )
}

fn is_wall_torch_block_name(block_name: &str) -> bool {
    matches!(
        block_name,
        "minecraft:wall_torch"
            | "minecraft:redstone_wall_torch"
            | "minecraft:soul_wall_torch"
            | "minecraft:copper_wall_torch"
    )
}

fn is_button_block_name(block_name: &str) -> bool {
    block_name
        .strip_prefix("minecraft:")
        .is_some_and(|path| path.ends_with("_button"))
}

fn is_fence_block_name(block_name: &str) -> bool {
    block_name
        .strip_prefix("minecraft:")
        .is_some_and(|path| path.ends_with("_fence"))
}

fn is_pane_block_name(block_name: &str) -> bool {
    block_name == "minecraft:iron_bars"
        || block_name
            .strip_prefix("minecraft:")
            .is_some_and(|path| path.ends_with("glass_pane"))
}

fn is_wall_block_name(block_name: &str) -> bool {
    block_name
        .strip_prefix("minecraft:")
        .is_some_and(|path| path.ends_with("_wall"))
}

fn is_flat_carpet_block_name(block_name: &str) -> bool {
    let Some(path) = block_name.strip_prefix("minecraft:") else {
        return false;
    };
    matches!(
        path,
        "white_carpet"
            | "orange_carpet"
            | "magenta_carpet"
            | "light_blue_carpet"
            | "yellow_carpet"
            | "lime_carpet"
            | "pink_carpet"
            | "gray_carpet"
            | "light_gray_carpet"
            | "cyan_carpet"
            | "purple_carpet"
            | "blue_carpet"
            | "brown_carpet"
            | "green_carpet"
            | "red_carpet"
            | "black_carpet"
            | "moss_carpet"
    )
}
