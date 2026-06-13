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
    if is_stair_block_name(block_name) {
        return stair_outline_shape(properties);
    }
    if is_trapdoor_block_name(block_name) {
        return trapdoor_outline_shape(properties);
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

fn is_trapdoor_block_name(block_name: &str) -> bool {
    block_name
        .strip_prefix("minecraft:")
        .is_some_and(|path| path.ends_with("_trapdoor"))
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
