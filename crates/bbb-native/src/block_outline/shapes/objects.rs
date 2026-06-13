use std::collections::BTreeMap;

use crate::block_outline::{BlockOutlineBox, BlockOutlineShape, HorizontalDirection};

pub(super) fn object_outline_shape_for_block(
    block_name: &str,
    properties: &BTreeMap<String, String>,
) -> Option<Option<BlockOutlineShape>> {
    if is_cauldron_block_name(block_name) {
        return Some(Some(cauldron_outline_shape()));
    }
    if block_name == "minecraft:hopper" {
        return Some(hopper_outline_shape(properties));
    }
    if is_campfire_block_name(block_name) {
        return Some(Some(BlockOutlineShape::single(
            BlockOutlineBox::centered_column(16.0, 16.0, 0.0, 7.0),
        )));
    }
    if is_chain_block_name(block_name) {
        return Some(chain_outline_shape(properties));
    }
    if is_lantern_block_name(block_name) {
        return Some(lantern_outline_shape(properties));
    }
    if block_name == "minecraft:ender_chest" {
        return Some(Some(BlockOutlineShape::single(
            BlockOutlineBox::CHEST_SINGLE,
        )));
    }
    if is_standard_chest_block_name(block_name) {
        return Some(chest_outline_shape(properties));
    }
    if is_bed_block_name(block_name) {
        return Some(bed_outline_shape(properties));
    }
    if block_name == "minecraft:cake" {
        return Some(cake_outline_shape(properties));
    }
    if is_candle_cake_block_name(block_name) {
        return Some(Some(candle_cake_outline_shape()));
    }
    if is_candle_block_name(block_name) {
        return Some(candle_outline_shape(properties));
    }
    if block_name == "minecraft:brewing_stand" {
        return Some(Some(brewing_stand_outline_shape()));
    }
    if block_name == "minecraft:enchanting_table" {
        return Some(Some(BlockOutlineShape::single(
            BlockOutlineBox::centered_column(16.0, 16.0, 0.0, 12.0),
        )));
    }
    if block_name == "minecraft:stonecutter" {
        return Some(Some(BlockOutlineShape::single(
            BlockOutlineBox::centered_column(16.0, 16.0, 0.0, 9.0),
        )));
    }
    if is_anvil_block_name(block_name) {
        return Some(anvil_outline_shape(properties));
    }
    if block_name == "minecraft:lectern" {
        return Some(lectern_outline_shape(properties));
    }
    None
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

fn lantern_outline_shape(properties: &BTreeMap<String, String>) -> Option<BlockOutlineShape> {
    let hanging = match properties.get("hanging").map(String::as_str)? {
        "true" => true,
        "false" => false,
        _ => return None,
    };
    let offset = if hanging { 1.0 } else { 0.0 };
    Some(BlockOutlineShape::from_boxes(vec![
        BlockOutlineBox::centered_column(4.0, 4.0, 7.0 + offset, 9.0 + offset),
        BlockOutlineBox::centered_column(6.0, 6.0, offset, 7.0 + offset),
    ]))
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

fn candle_outline_shape(properties: &BTreeMap<String, String>) -> Option<BlockOutlineShape> {
    let outline = match properties.get("candles")?.parse::<u8>().ok()? {
        1 => BlockOutlineBox::centered_column(2.0, 2.0, 0.0, 6.0),
        2 => BlockOutlineBox::from_pixels([5.0, 0.0, 6.0], [11.0, 6.0, 9.0]),
        3 => BlockOutlineBox::from_pixels([5.0, 0.0, 6.0], [10.0, 6.0, 11.0]),
        4 => BlockOutlineBox::from_pixels([5.0, 0.0, 5.0], [11.0, 6.0, 10.0]),
        _ => return None,
    };
    Some(BlockOutlineShape::single(outline))
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

fn is_lantern_block_name(block_name: &str) -> bool {
    block_name.strip_prefix("minecraft:").is_some_and(|path| {
        path == "lantern"
            || path == "soul_lantern"
            || path == "copper_lantern"
            || path.ends_with("_copper_lantern")
    })
}

fn is_candle_cake_block_name(block_name: &str) -> bool {
    block_name
        .strip_prefix("minecraft:")
        .is_some_and(|path| path == "candle_cake" || path.ends_with("_candle_cake"))
}

fn is_candle_block_name(block_name: &str) -> bool {
    block_name
        .strip_prefix("minecraft:")
        .is_some_and(|path| path == "candle" || path.ends_with("_candle"))
}
