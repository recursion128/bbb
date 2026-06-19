use std::collections::BTreeMap;

use super::local_player::LocalPlayerPoseState;
use crate::{BlockPos, BlockProbe, TerrainMaterialClass, WorldStore};

pub(super) const COLLISION_EPSILON: f64 = 1.0e-7;

const LOCAL_PLAYER_HALF_WIDTH: f64 = 0.3;
const MAX_COLLISION_BOXES: usize = 16;
const PX: f64 = 1.0 / 16.0;

pub(super) fn local_player_collides(world: &WorldStore, bounds: LocalPlayerBounds) -> bool {
    let min_x = block_floor(bounds.min_x + COLLISION_EPSILON);
    let max_x = block_floor(bounds.max_x - COLLISION_EPSILON);
    let min_y = block_floor(bounds.min_y + COLLISION_EPSILON) - 1;
    let max_y = block_floor(bounds.max_y - COLLISION_EPSILON);
    let min_z = block_floor(bounds.min_z + COLLISION_EPSILON);
    let max_z = block_floor(bounds.max_z - COLLISION_EPSILON);

    for y in min_y..=max_y {
        for z in min_z..=max_z {
            for x in min_x..=max_x {
                let Some(block) = world.probe_block(BlockPos { x, y, z }) else {
                    continue;
                };
                if block_collides_with_local_player_bounds(&block, BlockPos { x, y, z }, bounds) {
                    return true;
                }
            }
        }
    }
    false
}

pub(super) fn local_player_block_collision_is_empty(block: &BlockProbe) -> bool {
    block_collision_shape(block).is_none()
}

impl WorldStore {
    pub(crate) fn local_player_pose_collides_with_block(
        &self,
        pos: BlockPos,
        pose: LocalPlayerPoseState,
    ) -> bool {
        let Some(block) = self.probe_block(pos) else {
            return false;
        };
        block_collides_with_local_player_bounds(&block, pos, LocalPlayerBounds::for_pose(pose))
    }
}

fn block_collides_with_local_player_bounds(
    block: &BlockProbe,
    pos: BlockPos,
    bounds: LocalPlayerBounds,
) -> bool {
    if let Some(shape) = block_collision_shape(block) {
        return bounds_intersects_block_shape(bounds, pos, shape);
    }
    false
}

fn block_collision_shape(block: &BlockProbe) -> Option<BlockCollisionShape> {
    if is_slab_block(block) {
        return match block.block_properties.get("type").map(String::as_str) {
            Some("bottom") => Some(BlockCollisionShape::single(BlockCollisionBox::BOTTOM_SLAB)),
            Some("top") => Some(BlockCollisionShape::single(BlockCollisionBox::TOP_SLAB)),
            Some("double") => Some(BlockCollisionShape::single(BlockCollisionBox::FULL)),
            _ => None,
        };
    }
    if is_stair_block(block) {
        return stair_collision_shape(&block.block_properties);
    }
    if let Some(block_name) = block.block_name.as_deref() {
        if is_leaves_block_name(block_name) {
            return Some(BlockCollisionShape::single(BlockCollisionBox::FULL));
        }
        if block_name == "minecraft:snow" {
            return snow_layer_collision_shape(&block.block_properties);
        }
        if is_flat_carpet_block_name(block_name) {
            return Some(BlockCollisionShape::single(BlockCollisionBox::CARPET));
        }
        if is_copper_grate_block_name(block_name) {
            return Some(BlockCollisionShape::single(BlockCollisionBox::FULL));
        }
        if is_chain_block_name(block_name) {
            return chain_collision_shape(&block.block_properties);
        }
        if is_ladder_block_name(block_name) {
            return ladder_collision_shape(&block.block_properties);
        }
        if is_lantern_block_name(block_name) {
            return lantern_collision_shape(&block.block_properties);
        }
        if is_rod_block_name(block_name) {
            return rod_collision_shape(&block.block_properties);
        }
        if is_campfire_block_name(block_name) {
            return Some(BlockCollisionShape::single(BlockCollisionBox::CAMPFIRE));
        }
        if block_name == "minecraft:cake" {
            return cake_collision_shape(&block.block_properties);
        }
        if block_name == "minecraft:lily_pad" {
            return Some(BlockCollisionShape::single(BlockCollisionBox::LILY_PAD));
        }
        if is_cactus_or_honey_block_name(block_name) {
            return Some(BlockCollisionShape::single(
                BlockCollisionBox::CENTERED_14PX_COLUMN_15PX_HIGH,
            ));
        }
        if is_farmland_or_dirt_path_block_name(block_name) {
            return Some(BlockCollisionShape::single(
                BlockCollisionBox::FULL_WIDTH_15PX_HIGH,
            ));
        }
        if is_soul_sand_or_mud_block_name(block_name) {
            return Some(BlockCollisionShape::single(
                BlockCollisionBox::FULL_WIDTH_14PX_HIGH,
            ));
        }
        if is_cauldron_block_name(block_name) {
            return Some(cauldron_collision_shape());
        }
        if block_name == "minecraft:hopper" {
            return hopper_collision_shape(&block.block_properties);
        }
        if block_name == "minecraft:composter" {
            return Some(composter_collision_shape());
        }
        if block_name == "minecraft:ender_chest" {
            return Some(BlockCollisionShape::single(BlockCollisionBox::CHEST_SINGLE));
        }
        if is_standard_chest_block_name(block_name) {
            return chest_collision_shape(&block.block_properties);
        }
        if is_bed_block_name(block_name) {
            return bed_collision_shape(&block.block_properties);
        }
        if block_name == "minecraft:enchanting_table" {
            return Some(BlockCollisionShape::single(
                BlockCollisionBox::centered_column(16.0, 16.0, 0.0, 12.0),
            ));
        }
        if block_name == "minecraft:stonecutter" {
            return Some(BlockCollisionShape::single(
                BlockCollisionBox::centered_column(16.0, 16.0, 0.0, 9.0),
            ));
        }
        if block_name == "minecraft:brewing_stand" {
            return Some(brewing_stand_collision_shape());
        }
        if is_anvil_block_name(block_name) {
            return anvil_collision_shape(&block.block_properties);
        }
        if is_door_block_name(block_name) {
            return door_collision_shape(&block.block_properties);
        }
        if is_trapdoor_block_name(block_name) {
            return trapdoor_collision_shape(&block.block_properties);
        }
        if is_fence_gate_block_name(block_name) {
            return fence_gate_collision_shape(&block.block_properties);
        }
        if is_fence_block_name(block_name) {
            return fence_collision_shape(&block.block_properties);
        }
        if is_bars_or_pane_block_name(block_name) {
            return bars_or_pane_collision_shape(&block.block_properties);
        }
        if is_wall_block_name(block_name) {
            return wall_collision_shape(&block.block_properties);
        }
    }
    match block.material {
        TerrainMaterialClass::Opaque | TerrainMaterialClass::Translucent => {
            Some(BlockCollisionShape::single(BlockCollisionBox::FULL))
        }
        TerrainMaterialClass::Invisible => {
            if matches!(block.block_name.as_deref(), Some("minecraft:barrier")) {
                Some(BlockCollisionShape::single(BlockCollisionBox::FULL))
            } else {
                None
            }
        }
        TerrainMaterialClass::Empty
        | TerrainMaterialClass::Cutout
        | TerrainMaterialClass::Fluid => None,
    }
}

fn is_slab_block(block: &BlockProbe) -> bool {
    block
        .block_name
        .as_deref()
        .is_some_and(|name| name.ends_with("_slab"))
}

fn is_stair_block(block: &BlockProbe) -> bool {
    block
        .block_name
        .as_deref()
        .is_some_and(|name| name.ends_with("_stairs"))
}

fn is_leaves_block_name(block_name: &str) -> bool {
    block_name
        .strip_prefix("minecraft:")
        .is_some_and(|path| path.ends_with("_leaves"))
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

fn is_copper_grate_block_name(block_name: &str) -> bool {
    block_name
        .strip_prefix("minecraft:")
        .is_some_and(|path| path == "copper_grate" || path.ends_with("_copper_grate"))
}

fn is_chain_block_name(block_name: &str) -> bool {
    block_name
        .strip_prefix("minecraft:")
        .is_some_and(|path| path.ends_with("_chain"))
}

fn is_ladder_block_name(block_name: &str) -> bool {
    block_name == "minecraft:ladder"
}

fn is_lantern_block_name(block_name: &str) -> bool {
    let Some(path) = block_name.strip_prefix("minecraft:") else {
        return false;
    };
    matches!(path, "lantern" | "soul_lantern") || path.ends_with("_copper_lantern")
}

fn is_rod_block_name(block_name: &str) -> bool {
    let Some(path) = block_name.strip_prefix("minecraft:") else {
        return false;
    };
    matches!(path, "end_rod" | "lightning_rod") || path.ends_with("_lightning_rod")
}

fn is_campfire_block_name(block_name: &str) -> bool {
    matches!(block_name, "minecraft:campfire" | "minecraft:soul_campfire")
}

fn is_cactus_or_honey_block_name(block_name: &str) -> bool {
    matches!(block_name, "minecraft:cactus" | "minecraft:honey_block")
}

fn is_farmland_or_dirt_path_block_name(block_name: &str) -> bool {
    matches!(block_name, "minecraft:farmland" | "minecraft:dirt_path")
}

fn is_soul_sand_or_mud_block_name(block_name: &str) -> bool {
    matches!(block_name, "minecraft:soul_sand" | "minecraft:mud")
}

fn is_door_block_name(block_name: &str) -> bool {
    block_name
        .strip_prefix("minecraft:")
        .is_some_and(|path| path.ends_with("_door"))
}

fn is_trapdoor_block_name(block_name: &str) -> bool {
    block_name
        .strip_prefix("minecraft:")
        .is_some_and(|path| path.ends_with("_trapdoor"))
}

fn is_fence_gate_block_name(block_name: &str) -> bool {
    block_name
        .strip_prefix("minecraft:")
        .is_some_and(|path| path.ends_with("_fence_gate"))
}

fn is_fence_block_name(block_name: &str) -> bool {
    block_name
        .strip_prefix("minecraft:")
        .is_some_and(|path| path.ends_with("_fence"))
}

fn is_bars_or_pane_block_name(block_name: &str) -> bool {
    block_name == "minecraft:iron_bars"
        || block_name
            .strip_prefix("minecraft:")
            .is_some_and(|path| path.ends_with("_bars") || path.ends_with("glass_pane"))
}

fn is_wall_block_name(block_name: &str) -> bool {
    block_name
        .strip_prefix("minecraft:")
        .is_some_and(|path| path.ends_with("_wall"))
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

fn stair_collision_shape(properties: &BTreeMap<String, String>) -> Option<BlockCollisionShape> {
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

    let mut shape = match kind {
        StairShapeKind::Straight => BlockCollisionShape::from_boxes([
            Some(BlockCollisionBox::BOTTOM_SLAB),
            Some(BlockCollisionBox::STAIR_NORTH_HALF),
            None,
            None,
            None,
        ]),
        StairShapeKind::Outer => BlockCollisionShape::from_boxes([
            Some(BlockCollisionBox::BOTTOM_SLAB),
            Some(BlockCollisionBox::STAIR_NORTH_WEST_OCTET),
            None,
            None,
            None,
        ]),
        StairShapeKind::Inner => BlockCollisionShape::from_boxes([
            Some(BlockCollisionBox::BOTTOM_SLAB),
            Some(BlockCollisionBox::STAIR_NORTH_HALF),
            Some(BlockCollisionBox::STAIR_SOUTH_EAST_OCTET),
            None,
            None,
        ]),
    };
    if top {
        shape = shape.invert_y();
    }
    Some(shape.rotate_to_direction(direction))
}

fn snow_layer_collision_shape(
    properties: &BTreeMap<String, String>,
) -> Option<BlockCollisionShape> {
    let layers = properties.get("layers")?.parse::<u8>().ok()?;
    if !(1..=8).contains(&layers) {
        return None;
    }
    let height = f64::from(layers - 1) / 8.0;
    Some(BlockCollisionShape::single(BlockCollisionBox::column(
        0.0, 0.0, 1.0, height, 1.0,
    )))
}

fn chain_collision_shape(properties: &BTreeMap<String, String>) -> Option<BlockCollisionShape> {
    let axis = ShapeAxis::parse(properties.get("axis")?)?;
    Some(BlockCollisionShape::single(
        BlockCollisionBox::centered_axis(axis, 3.0),
    ))
}

fn ladder_collision_shape(properties: &BTreeMap<String, String>) -> Option<BlockCollisionShape> {
    let facing = HorizontalDirection::parse(properties.get("facing")?)?;
    Some(
        BlockCollisionShape::single(BlockCollisionBox::NORTH_VERTICAL_THIN)
            .rotate_to_direction(facing),
    )
}

fn lantern_collision_shape(properties: &BTreeMap<String, String>) -> Option<BlockCollisionShape> {
    let y_offset = if bool_property(properties, "hanging")? {
        PX
    } else {
        0.0
    };
    Some(BlockCollisionShape::from_boxes([
        Some(BlockCollisionBox::cuboid(
            5.0 * PX,
            y_offset,
            5.0 * PX,
            11.0 * PX,
            y_offset + 7.0 * PX,
            11.0 * PX,
        )),
        Some(BlockCollisionBox::cuboid(
            6.0 * PX,
            y_offset + 7.0 * PX,
            6.0 * PX,
            10.0 * PX,
            y_offset + 9.0 * PX,
            10.0 * PX,
        )),
        None,
        None,
        None,
    ]))
}

fn rod_collision_shape(properties: &BTreeMap<String, String>) -> Option<BlockCollisionShape> {
    let axis = ShapeAxis::parse_direction(properties.get("facing")?)?;
    Some(BlockCollisionShape::single(
        BlockCollisionBox::centered_axis(axis, 4.0),
    ))
}

fn cake_collision_shape(properties: &BTreeMap<String, String>) -> Option<BlockCollisionShape> {
    let bites = properties.get("bites")?.parse::<u8>().ok()?;
    if bites > 6 {
        return None;
    }
    Some(BlockCollisionShape::single(BlockCollisionBox::from_pixels(
        [1.0 + f64::from(bites) * 2.0, 0.0, 1.0],
        [15.0, 8.0, 15.0],
    )))
}

fn door_collision_shape(properties: &BTreeMap<String, String>) -> Option<BlockCollisionShape> {
    let facing = HorizontalDirection::parse(properties.get("facing")?)?;
    let open = bool_property(properties, "open")?;
    let direction = if open {
        match properties.get("hinge").map(String::as_str)? {
            "left" => facing.clockwise(),
            "right" => facing.counter_clockwise(),
            _ => return None,
        }
    } else {
        facing
    };
    Some(
        BlockCollisionShape::single(BlockCollisionBox::NORTH_VERTICAL_THIN)
            .rotate_to_direction(direction),
    )
}

fn trapdoor_collision_shape(properties: &BTreeMap<String, String>) -> Option<BlockCollisionShape> {
    if bool_property(properties, "open")? {
        let facing = HorizontalDirection::parse(properties.get("facing")?)?;
        return Some(
            BlockCollisionShape::single(BlockCollisionBox::NORTH_VERTICAL_THIN)
                .rotate_to_direction(facing),
        );
    }
    match properties.get("half").map(String::as_str)? {
        "bottom" => Some(BlockCollisionShape::single(
            BlockCollisionBox::BOTTOM_TRAPDOOR,
        )),
        "top" => Some(BlockCollisionShape::single(BlockCollisionBox::TOP_TRAPDOOR)),
        _ => None,
    }
}

fn fence_collision_shape(properties: &BTreeMap<String, String>) -> Option<BlockCollisionShape> {
    cross_collision_shape(
        properties,
        BlockCollisionBox::FENCE_POST,
        BlockCollisionBox::FENCE_NORTH_ARM,
        BlockCollisionBox::FENCE_EAST_ARM,
        BlockCollisionBox::FENCE_SOUTH_ARM,
        BlockCollisionBox::FENCE_WEST_ARM,
    )
}

fn fence_gate_collision_shape(
    properties: &BTreeMap<String, String>,
) -> Option<BlockCollisionShape> {
    if bool_property(properties, "open")? {
        return None;
    }
    let facing = HorizontalDirection::parse(properties.get("facing")?)?;
    let shape_box = match facing.axis() {
        HorizontalAxis::X => BlockCollisionBox::FENCE_GATE_X_AXIS,
        HorizontalAxis::Z => BlockCollisionBox::FENCE_GATE_Z_AXIS,
    };
    Some(BlockCollisionShape::single(shape_box))
}

fn bars_or_pane_collision_shape(
    properties: &BTreeMap<String, String>,
) -> Option<BlockCollisionShape> {
    cross_collision_shape(
        properties,
        BlockCollisionBox::PANE_POST,
        BlockCollisionBox::PANE_NORTH_ARM,
        BlockCollisionBox::PANE_EAST_ARM,
        BlockCollisionBox::PANE_SOUTH_ARM,
        BlockCollisionBox::PANE_WEST_ARM,
    )
}

fn wall_collision_shape(properties: &BTreeMap<String, String>) -> Option<BlockCollisionShape> {
    let mut builder = BlockCollisionShapeBuilder::new();
    if bool_property(properties, "up")? {
        builder.push(BlockCollisionBox::WALL_POST);
    }
    if wall_side_has_collision(properties, "north")? {
        builder.push(BlockCollisionBox::WALL_NORTH_SIDE);
    }
    if wall_side_has_collision(properties, "east")? {
        builder.push(BlockCollisionBox::WALL_EAST_SIDE);
    }
    if wall_side_has_collision(properties, "south")? {
        builder.push(BlockCollisionBox::WALL_SOUTH_SIDE);
    }
    if wall_side_has_collision(properties, "west")? {
        builder.push(BlockCollisionBox::WALL_WEST_SIDE);
    }
    Some(builder.build())
}

fn cauldron_collision_shape() -> BlockCollisionShape {
    let mut builder = BlockCollisionShapeBuilder::new();
    builder.push(BlockCollisionBox::from_pixels(
        [0.0, 0.0, 0.0],
        [2.0, 3.0, 4.0],
    ));
    builder.push(BlockCollisionBox::from_pixels(
        [0.0, 0.0, 12.0],
        [2.0, 3.0, 16.0],
    ));
    builder.push(BlockCollisionBox::from_pixels(
        [14.0, 0.0, 0.0],
        [16.0, 3.0, 4.0],
    ));
    builder.push(BlockCollisionBox::from_pixels(
        [14.0, 0.0, 12.0],
        [16.0, 3.0, 16.0],
    ));
    builder.push(BlockCollisionBox::from_pixels(
        [2.0, 0.0, 0.0],
        [4.0, 3.0, 2.0],
    ));
    builder.push(BlockCollisionBox::from_pixels(
        [2.0, 0.0, 14.0],
        [4.0, 3.0, 16.0],
    ));
    builder.push(BlockCollisionBox::from_pixels(
        [12.0, 0.0, 0.0],
        [14.0, 3.0, 2.0],
    ));
    builder.push(BlockCollisionBox::from_pixels(
        [12.0, 0.0, 14.0],
        [14.0, 3.0, 16.0],
    ));
    builder.push(BlockCollisionBox::from_pixels(
        [0.0, 3.0, 0.0],
        [16.0, 4.0, 16.0],
    ));
    builder.push(BlockCollisionBox::from_pixels(
        [0.0, 4.0, 0.0],
        [2.0, 16.0, 16.0],
    ));
    builder.push(BlockCollisionBox::from_pixels(
        [14.0, 4.0, 0.0],
        [16.0, 16.0, 16.0],
    ));
    builder.push(BlockCollisionBox::from_pixels(
        [2.0, 4.0, 0.0],
        [14.0, 16.0, 2.0],
    ));
    builder.push(BlockCollisionBox::from_pixels(
        [2.0, 4.0, 14.0],
        [14.0, 16.0, 16.0],
    ));
    builder.build()
}

fn hopper_collision_shape(properties: &BTreeMap<String, String>) -> Option<BlockCollisionShape> {
    let spout = match properties.get("facing").map(String::as_str)? {
        "down" => BlockCollisionBox::from_pixels([6.0, 0.0, 6.0], [10.0, 6.0, 10.0]),
        "north" => BlockCollisionBox::from_pixels([6.0, 4.0, 0.0], [10.0, 8.0, 8.0]),
        "east" => BlockCollisionBox::from_pixels([8.0, 4.0, 6.0], [16.0, 8.0, 10.0]),
        "south" => BlockCollisionBox::from_pixels([6.0, 4.0, 8.0], [10.0, 8.0, 16.0]),
        "west" => BlockCollisionBox::from_pixels([0.0, 4.0, 6.0], [8.0, 8.0, 10.0]),
        _ => return None,
    };

    let mut builder = BlockCollisionShapeBuilder::new();
    push_spoutless_hopper_boxes(&mut builder);
    builder.push(spout);
    Some(builder.build())
}

fn push_spoutless_hopper_boxes(builder: &mut BlockCollisionShapeBuilder) {
    builder.push(BlockCollisionBox::from_pixels(
        [0.0, 10.0, 0.0],
        [16.0, 11.0, 16.0],
    ));
    builder.push(BlockCollisionBox::from_pixels(
        [0.0, 11.0, 0.0],
        [16.0, 16.0, 2.0],
    ));
    builder.push(BlockCollisionBox::from_pixels(
        [0.0, 11.0, 14.0],
        [16.0, 16.0, 16.0],
    ));
    builder.push(BlockCollisionBox::from_pixels(
        [0.0, 11.0, 2.0],
        [2.0, 16.0, 14.0],
    ));
    builder.push(BlockCollisionBox::from_pixels(
        [14.0, 11.0, 2.0],
        [16.0, 16.0, 14.0],
    ));
    builder.push(BlockCollisionBox::centered_column(8.0, 8.0, 4.0, 10.0));
}

fn composter_collision_shape() -> BlockCollisionShape {
    let hole_y = 2.0;
    BlockCollisionShape::from_boxes([
        Some(BlockCollisionBox::from_pixels(
            [0.0, 0.0, 0.0],
            [16.0, hole_y, 16.0],
        )),
        Some(BlockCollisionBox::from_pixels(
            [0.0, hole_y, 0.0],
            [16.0, 16.0, 2.0],
        )),
        Some(BlockCollisionBox::from_pixels(
            [0.0, hole_y, 14.0],
            [16.0, 16.0, 16.0],
        )),
        Some(BlockCollisionBox::from_pixels(
            [0.0, hole_y, 2.0],
            [2.0, 16.0, 14.0],
        )),
        Some(BlockCollisionBox::from_pixels(
            [14.0, hole_y, 2.0],
            [16.0, 16.0, 14.0],
        )),
    ])
}

fn chest_collision_shape(properties: &BTreeMap<String, String>) -> Option<BlockCollisionShape> {
    let shape_box = match properties.get("type").map(String::as_str)? {
        "single" => BlockCollisionBox::CHEST_SINGLE,
        "left" => {
            let connected = HorizontalDirection::parse(properties.get("facing")?)?.clockwise();
            chest_connected_collision_box(connected)
        }
        "right" => {
            let connected =
                HorizontalDirection::parse(properties.get("facing")?)?.counter_clockwise();
            chest_connected_collision_box(connected)
        }
        _ => return None,
    };
    Some(BlockCollisionShape::single(shape_box))
}

fn chest_connected_collision_box(connected: HorizontalDirection) -> BlockCollisionBox {
    match connected {
        HorizontalDirection::North => BlockCollisionBox::CHEST_CONNECTED_NORTH,
        HorizontalDirection::East => BlockCollisionBox::CHEST_CONNECTED_EAST,
        HorizontalDirection::South => BlockCollisionBox::CHEST_CONNECTED_SOUTH,
        HorizontalDirection::West => BlockCollisionBox::CHEST_CONNECTED_WEST,
    }
}

fn bed_collision_shape(properties: &BTreeMap<String, String>) -> Option<BlockCollisionShape> {
    let facing = HorizontalDirection::parse(properties.get("facing")?)?;
    let connected = match properties.get("part").map(String::as_str)? {
        "head" => facing.opposite(),
        "foot" => facing,
        _ => return None,
    };
    Some(bed_collision_shape_for_direction(connected.opposite()))
}

fn bed_collision_shape_for_direction(direction: HorizontalDirection) -> BlockCollisionShape {
    let (left_leg, right_leg) = match direction {
        HorizontalDirection::North => (
            BlockCollisionBox::from_pixels([0.0, 0.0, 0.0], [3.0, 3.0, 3.0]),
            BlockCollisionBox::from_pixels([13.0, 0.0, 0.0], [16.0, 3.0, 3.0]),
        ),
        HorizontalDirection::East => (
            BlockCollisionBox::from_pixels([13.0, 0.0, 0.0], [16.0, 3.0, 3.0]),
            BlockCollisionBox::from_pixels([13.0, 0.0, 13.0], [16.0, 3.0, 16.0]),
        ),
        HorizontalDirection::South => (
            BlockCollisionBox::from_pixels([13.0, 0.0, 13.0], [16.0, 3.0, 16.0]),
            BlockCollisionBox::from_pixels([0.0, 0.0, 13.0], [3.0, 3.0, 16.0]),
        ),
        HorizontalDirection::West => (
            BlockCollisionBox::from_pixels([0.0, 0.0, 13.0], [3.0, 3.0, 16.0]),
            BlockCollisionBox::from_pixels([0.0, 0.0, 0.0], [3.0, 3.0, 3.0]),
        ),
    };
    BlockCollisionShape::from_boxes([
        Some(BlockCollisionBox::BED_PLATFORM),
        Some(left_leg),
        Some(right_leg),
    ])
}

fn anvil_collision_shape(properties: &BTreeMap<String, String>) -> Option<BlockCollisionShape> {
    let facing = HorizontalDirection::parse(properties.get("facing")?)?;
    let boxes = match facing.axis() {
        HorizontalAxis::X => [
            Some(BlockCollisionBox::centered_column(12.0, 12.0, 0.0, 4.0)),
            Some(BlockCollisionBox::centered_column(10.0, 8.0, 4.0, 5.0)),
            Some(BlockCollisionBox::centered_column(8.0, 4.0, 5.0, 10.0)),
            Some(BlockCollisionBox::centered_column(16.0, 10.0, 10.0, 16.0)),
        ],
        HorizontalAxis::Z => [
            Some(BlockCollisionBox::centered_column(12.0, 12.0, 0.0, 4.0)),
            Some(BlockCollisionBox::centered_column(8.0, 10.0, 4.0, 5.0)),
            Some(BlockCollisionBox::centered_column(4.0, 8.0, 5.0, 10.0)),
            Some(BlockCollisionBox::centered_column(10.0, 16.0, 10.0, 16.0)),
        ],
    };
    Some(BlockCollisionShape::from_boxes(boxes))
}

fn brewing_stand_collision_shape() -> BlockCollisionShape {
    BlockCollisionShape::from_boxes([
        Some(BlockCollisionBox::BREWING_STAND_ROD),
        Some(BlockCollisionBox::BREWING_STAND_BASE),
    ])
}

fn cross_collision_shape(
    properties: &BTreeMap<String, String>,
    post: BlockCollisionBox,
    north_arm: BlockCollisionBox,
    east_arm: BlockCollisionBox,
    south_arm: BlockCollisionBox,
    west_arm: BlockCollisionBox,
) -> Option<BlockCollisionShape> {
    let mut builder = BlockCollisionShapeBuilder::new();
    builder.push(post);
    if bool_property(properties, "north")? {
        builder.push(north_arm);
    }
    if bool_property(properties, "east")? {
        builder.push(east_arm);
    }
    if bool_property(properties, "south")? {
        builder.push(south_arm);
    }
    if bool_property(properties, "west")? {
        builder.push(west_arm);
    }
    Some(builder.build())
}

fn bool_property(properties: &BTreeMap<String, String>, key: &str) -> Option<bool> {
    match properties.get(key).map(String::as_str)? {
        "true" => Some(true),
        "false" => Some(false),
        _ => None,
    }
}

fn wall_side_has_collision(properties: &BTreeMap<String, String>, key: &str) -> Option<bool> {
    match properties.get(key).map(String::as_str)? {
        "low" | "tall" => Some(true),
        "none" => Some(false),
        _ => None,
    }
}

fn bounds_intersects_block_shape(
    bounds: LocalPlayerBounds,
    pos: BlockPos,
    shape: BlockCollisionShape,
) -> bool {
    shape
        .boxes()
        .any(|shape_box| bounds_intersects_block_box(bounds, pos, shape_box))
}

fn bounds_intersects_block_box(
    bounds: LocalPlayerBounds,
    pos: BlockPos,
    shape: BlockCollisionBox,
) -> bool {
    let block_x = f64::from(pos.x);
    let min_x = block_x + shape.min_x;
    let max_x = block_x + shape.max_x;
    let min_y = f64::from(pos.y) + shape.min_y;
    let max_y = f64::from(pos.y) + shape.max_y;
    let block_z = f64::from(pos.z);
    let min_z = block_z + shape.min_z;
    let max_z = block_z + shape.max_z;

    bounds.max_x > min_x + COLLISION_EPSILON
        && bounds.min_x < max_x - COLLISION_EPSILON
        && bounds.max_y > min_y + COLLISION_EPSILON
        && bounds.min_y < max_y - COLLISION_EPSILON
        && bounds.max_z > min_z + COLLISION_EPSILON
        && bounds.min_z < max_z - COLLISION_EPSILON
}

fn block_floor(value: f64) -> i32 {
    value.floor() as i32
}

#[derive(Debug, Clone, Copy)]
pub(super) enum CollisionAxis {
    X,
    Y,
    Z,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum HorizontalDirection {
    North,
    East,
    South,
    West,
}

impl HorizontalDirection {
    fn parse(value: &str) -> Option<Self> {
        match value {
            "north" => Some(Self::North),
            "east" => Some(Self::East),
            "south" => Some(Self::South),
            "west" => Some(Self::West),
            _ => None,
        }
    }

    fn clockwise(self) -> Self {
        match self {
            Self::North => Self::East,
            Self::East => Self::South,
            Self::South => Self::West,
            Self::West => Self::North,
        }
    }

    fn counter_clockwise(self) -> Self {
        match self {
            Self::North => Self::West,
            Self::East => Self::North,
            Self::South => Self::East,
            Self::West => Self::South,
        }
    }

    fn opposite(self) -> Self {
        match self {
            Self::North => Self::South,
            Self::East => Self::West,
            Self::South => Self::North,
            Self::West => Self::East,
        }
    }

    fn quarter_turns_from_north(self) -> usize {
        match self {
            Self::North => 0,
            Self::East => 1,
            Self::South => 2,
            Self::West => 3,
        }
    }

    fn axis(self) -> HorizontalAxis {
        match self {
            Self::North | Self::South => HorizontalAxis::Z,
            Self::East | Self::West => HorizontalAxis::X,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum HorizontalAxis {
    X,
    Z,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ShapeAxis {
    X,
    Y,
    Z,
}

impl ShapeAxis {
    fn parse(value: &str) -> Option<Self> {
        match value {
            "x" => Some(Self::X),
            "y" => Some(Self::Y),
            "z" => Some(Self::Z),
            _ => None,
        }
    }

    fn parse_direction(value: &str) -> Option<Self> {
        match value {
            "east" | "west" => Some(Self::X),
            "up" | "down" => Some(Self::Y),
            "north" | "south" => Some(Self::Z),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum StairShapeKind {
    Straight,
    Outer,
    Inner,
}

#[derive(Debug, Clone, Copy)]
pub(super) struct LocalPlayerBounds {
    min_x: f64,
    min_y: f64,
    min_z: f64,
    max_x: f64,
    max_y: f64,
    max_z: f64,
}

impl LocalPlayerBounds {
    pub(super) fn for_pose(pose: LocalPlayerPoseState) -> Self {
        Self::at_height(pose.position, pose.body_height())
    }

    fn at_height(position: bbb_protocol::packets::Vec3d, height: f64) -> Self {
        Self {
            min_x: position.x - LOCAL_PLAYER_HALF_WIDTH,
            min_y: position.y,
            min_z: position.z - LOCAL_PLAYER_HALF_WIDTH,
            max_x: position.x + LOCAL_PLAYER_HALF_WIDTH,
            max_y: position.y + height,
            max_z: position.z + LOCAL_PLAYER_HALF_WIDTH,
        }
    }

    pub(super) fn moved(self, x: f64, y: f64, z: f64) -> Self {
        Self {
            min_x: self.min_x + x,
            min_y: self.min_y + y,
            min_z: self.min_z + z,
            max_x: self.max_x + x,
            max_y: self.max_y + y,
            max_z: self.max_z + z,
        }
    }

    pub(super) fn deflated(self, amount: f64) -> Self {
        Self {
            min_x: self.min_x + amount,
            min_y: self.min_y + amount,
            min_z: self.min_z + amount,
            max_x: self.max_x - amount,
            max_y: self.max_y - amount,
            max_z: self.max_z - amount,
        }
    }

    pub(super) fn min_x(self) -> f64 {
        self.min_x
    }

    pub(super) fn min_y(self) -> f64 {
        self.min_y
    }

    pub(super) fn min_z(self) -> f64 {
        self.min_z
    }

    pub(super) fn max_x(self) -> f64 {
        self.max_x
    }

    pub(super) fn max_y(self) -> f64 {
        self.max_y
    }

    pub(super) fn max_z(self) -> f64 {
        self.max_z
    }

    pub(super) fn edge_support_probe(self, min_height: f64) -> Self {
        Self {
            min_x: self.min_x + COLLISION_EPSILON,
            min_y: self.min_y - min_height - COLLISION_EPSILON,
            min_z: self.min_z + COLLISION_EPSILON,
            max_x: self.max_x - COLLISION_EPSILON,
            max_y: self.min_y,
            max_z: self.max_z - COLLISION_EPSILON,
        }
    }

    fn moved_axis(self, axis: CollisionAxis, amount: f64) -> Self {
        match axis {
            CollisionAxis::X => self.moved(amount, 0.0, 0.0),
            CollisionAxis::Y => self.moved(0.0, amount, 0.0),
            CollisionAxis::Z => self.moved(0.0, 0.0, amount),
        }
    }

    pub(super) fn swept_axis(self, axis: CollisionAxis, amount: f64) -> Self {
        let moved = self.moved_axis(axis, amount);
        match axis {
            CollisionAxis::X => Self {
                min_x: self.min_x.min(moved.min_x),
                max_x: self.max_x.max(moved.max_x),
                ..self
            },
            CollisionAxis::Y => Self {
                min_y: self.min_y.min(moved.min_y),
                max_y: self.max_y.max(moved.max_y),
                ..self
            },
            CollisionAxis::Z => Self {
                min_z: self.min_z.min(moved.min_z),
                max_z: self.max_z.max(moved.max_z),
                ..self
            },
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct BlockCollisionShape {
    boxes: [Option<BlockCollisionBox>; MAX_COLLISION_BOXES],
}

impl BlockCollisionShape {
    fn single(shape_box: BlockCollisionBox) -> Self {
        let mut boxes = [None; MAX_COLLISION_BOXES];
        boxes[0] = Some(shape_box);
        Self { boxes }
    }

    fn from_boxes<const N: usize>(boxes: [Option<BlockCollisionBox>; N]) -> Self {
        let mut out = [None; MAX_COLLISION_BOXES];
        let mut next_index = 0;
        for shape_box in boxes.into_iter().flatten() {
            if next_index >= MAX_COLLISION_BOXES {
                break;
            }
            out[next_index] = Some(shape_box);
            next_index += 1;
        }
        Self { boxes: out }
    }

    fn boxes(self) -> impl Iterator<Item = BlockCollisionBox> {
        self.boxes.into_iter().flatten()
    }

    fn invert_y(self) -> Self {
        Self {
            boxes: self
                .boxes
                .map(|shape_box| shape_box.map(BlockCollisionBox::invert_y)),
        }
    }

    fn rotate_to_direction(self, direction: HorizontalDirection) -> Self {
        let mut rotated = self;
        for _ in 0..direction.quarter_turns_from_north() {
            rotated = Self {
                boxes: rotated
                    .boxes
                    .map(|shape_box| shape_box.map(BlockCollisionBox::rotate_y_90)),
            };
        }
        rotated
    }
}

struct BlockCollisionShapeBuilder {
    boxes: [Option<BlockCollisionBox>; MAX_COLLISION_BOXES],
    next_index: usize,
}

impl BlockCollisionShapeBuilder {
    fn new() -> Self {
        Self {
            boxes: [None; MAX_COLLISION_BOXES],
            next_index: 0,
        }
    }

    fn push(&mut self, shape_box: BlockCollisionBox) {
        if self.next_index < MAX_COLLISION_BOXES {
            self.boxes[self.next_index] = Some(shape_box);
            self.next_index += 1;
        }
    }

    fn build(self) -> BlockCollisionShape {
        BlockCollisionShape { boxes: self.boxes }
    }
}

#[derive(Debug, Clone, Copy)]
struct BlockCollisionBox {
    min_x: f64,
    min_y: f64,
    min_z: f64,
    max_x: f64,
    max_y: f64,
    max_z: f64,
}

impl BlockCollisionBox {
    const FULL: Self = Self {
        min_x: 0.0,
        min_y: 0.0,
        min_z: 0.0,
        max_x: 1.0,
        max_y: 1.0,
        max_z: 1.0,
    };
    const BOTTOM_SLAB: Self = Self {
        min_x: 0.0,
        min_y: 0.0,
        min_z: 0.0,
        max_x: 1.0,
        max_y: 0.5,
        max_z: 1.0,
    };
    const TOP_SLAB: Self = Self {
        min_x: 0.0,
        min_y: 0.5,
        min_z: 0.0,
        max_x: 1.0,
        max_y: 1.0,
        max_z: 1.0,
    };
    const CARPET: Self = Self {
        min_x: 0.0,
        min_y: 0.0,
        min_z: 0.0,
        max_x: 1.0,
        max_y: PX,
        max_z: 1.0,
    };
    const CAMPFIRE: Self = Self {
        min_x: 0.0,
        min_y: 0.0,
        min_z: 0.0,
        max_x: 1.0,
        max_y: 7.0 * PX,
        max_z: 1.0,
    };
    const FULL_WIDTH_15PX_HIGH: Self = Self {
        min_x: 0.0,
        min_y: 0.0,
        min_z: 0.0,
        max_x: 1.0,
        max_y: 15.0 * PX,
        max_z: 1.0,
    };
    const FULL_WIDTH_14PX_HIGH: Self = Self {
        min_x: 0.0,
        min_y: 0.0,
        min_z: 0.0,
        max_x: 1.0,
        max_y: 14.0 * PX,
        max_z: 1.0,
    };
    const CENTERED_14PX_COLUMN_15PX_HIGH: Self = Self {
        min_x: PX,
        min_y: 0.0,
        min_z: PX,
        max_x: 15.0 * PX,
        max_y: 15.0 * PX,
        max_z: 15.0 * PX,
    };
    const LILY_PAD: Self = Self {
        min_x: PX,
        min_y: 0.0,
        min_z: PX,
        max_x: 15.0 * PX,
        max_y: 1.5 * PX,
        max_z: 15.0 * PX,
    };
    const CHEST_SINGLE: Self = Self {
        min_x: PX,
        min_y: 0.0,
        min_z: PX,
        max_x: 15.0 * PX,
        max_y: 14.0 * PX,
        max_z: 15.0 * PX,
    };
    const CHEST_CONNECTED_NORTH: Self = Self {
        min_x: PX,
        min_y: 0.0,
        min_z: 0.0,
        max_x: 15.0 * PX,
        max_y: 14.0 * PX,
        max_z: 15.0 * PX,
    };
    const CHEST_CONNECTED_EAST: Self = Self {
        min_x: PX,
        min_y: 0.0,
        min_z: PX,
        max_x: 1.0,
        max_y: 14.0 * PX,
        max_z: 15.0 * PX,
    };
    const CHEST_CONNECTED_SOUTH: Self = Self {
        min_x: PX,
        min_y: 0.0,
        min_z: PX,
        max_x: 15.0 * PX,
        max_y: 14.0 * PX,
        max_z: 1.0,
    };
    const CHEST_CONNECTED_WEST: Self = Self {
        min_x: 0.0,
        min_y: 0.0,
        min_z: PX,
        max_x: 15.0 * PX,
        max_y: 14.0 * PX,
        max_z: 15.0 * PX,
    };
    const BED_PLATFORM: Self = Self {
        min_x: 0.0,
        min_y: 3.0 * PX,
        min_z: 0.0,
        max_x: 1.0,
        max_y: 9.0 * PX,
        max_z: 1.0,
    };
    const BREWING_STAND_ROD: Self = Self {
        min_x: 7.0 * PX,
        min_y: 2.0 * PX,
        min_z: 7.0 * PX,
        max_x: 9.0 * PX,
        max_y: 14.0 * PX,
        max_z: 9.0 * PX,
    };
    const BREWING_STAND_BASE: Self = Self {
        min_x: PX,
        min_y: 0.0,
        min_z: PX,
        max_x: 15.0 * PX,
        max_y: 2.0 * PX,
        max_z: 15.0 * PX,
    };
    const BOTTOM_TRAPDOOR: Self = Self {
        min_x: 0.0,
        min_y: 0.0,
        min_z: 0.0,
        max_x: 1.0,
        max_y: 3.0 * PX,
        max_z: 1.0,
    };
    const TOP_TRAPDOOR: Self = Self {
        min_x: 0.0,
        min_y: 13.0 * PX,
        min_z: 0.0,
        max_x: 1.0,
        max_y: 1.0,
        max_z: 1.0,
    };
    const NORTH_VERTICAL_THIN: Self = Self {
        min_x: 0.0,
        min_y: 0.0,
        min_z: 13.0 * PX,
        max_x: 1.0,
        max_y: 1.0,
        max_z: 1.0,
    };
    const STAIR_NORTH_HALF: Self = Self {
        min_x: 0.0,
        min_y: 0.5,
        min_z: 0.0,
        max_x: 1.0,
        max_y: 1.0,
        max_z: 0.5,
    };
    const STAIR_NORTH_WEST_OCTET: Self = Self {
        min_x: 0.0,
        min_y: 0.5,
        min_z: 0.0,
        max_x: 0.5,
        max_y: 1.0,
        max_z: 0.5,
    };
    const STAIR_SOUTH_EAST_OCTET: Self = Self {
        min_x: 0.5,
        min_y: 0.5,
        min_z: 0.5,
        max_x: 1.0,
        max_y: 1.0,
        max_z: 1.0,
    };
    const FENCE_POST: Self = Self {
        min_x: 6.0 * PX,
        min_y: 0.0,
        min_z: 6.0 * PX,
        max_x: 10.0 * PX,
        max_y: 1.5,
        max_z: 10.0 * PX,
    };
    const FENCE_NORTH_ARM: Self = Self {
        min_x: 6.0 * PX,
        min_y: 0.0,
        min_z: 0.0,
        max_x: 10.0 * PX,
        max_y: 1.5,
        max_z: 8.0 * PX,
    };
    const FENCE_EAST_ARM: Self = Self {
        min_x: 8.0 * PX,
        min_y: 0.0,
        min_z: 6.0 * PX,
        max_x: 1.0,
        max_y: 1.5,
        max_z: 10.0 * PX,
    };
    const FENCE_SOUTH_ARM: Self = Self {
        min_x: 6.0 * PX,
        min_y: 0.0,
        min_z: 8.0 * PX,
        max_x: 10.0 * PX,
        max_y: 1.5,
        max_z: 1.0,
    };
    const FENCE_WEST_ARM: Self = Self {
        min_x: 0.0,
        min_y: 0.0,
        min_z: 6.0 * PX,
        max_x: 8.0 * PX,
        max_y: 1.5,
        max_z: 10.0 * PX,
    };
    const FENCE_GATE_Z_AXIS: Self = Self {
        min_x: 0.0,
        min_y: 0.0,
        min_z: 6.0 * PX,
        max_x: 1.0,
        max_y: 1.5,
        max_z: 10.0 * PX,
    };
    const FENCE_GATE_X_AXIS: Self = Self {
        min_x: 6.0 * PX,
        min_y: 0.0,
        min_z: 0.0,
        max_x: 10.0 * PX,
        max_y: 1.5,
        max_z: 1.0,
    };
    const PANE_POST: Self = Self {
        min_x: 7.0 * PX,
        min_y: 0.0,
        min_z: 7.0 * PX,
        max_x: 9.0 * PX,
        max_y: 1.0,
        max_z: 9.0 * PX,
    };
    const PANE_NORTH_ARM: Self = Self {
        min_x: 7.0 * PX,
        min_y: 0.0,
        min_z: 0.0,
        max_x: 9.0 * PX,
        max_y: 1.0,
        max_z: 8.0 * PX,
    };
    const PANE_EAST_ARM: Self = Self {
        min_x: 8.0 * PX,
        min_y: 0.0,
        min_z: 7.0 * PX,
        max_x: 1.0,
        max_y: 1.0,
        max_z: 9.0 * PX,
    };
    const PANE_SOUTH_ARM: Self = Self {
        min_x: 7.0 * PX,
        min_y: 0.0,
        min_z: 8.0 * PX,
        max_x: 9.0 * PX,
        max_y: 1.0,
        max_z: 1.0,
    };
    const PANE_WEST_ARM: Self = Self {
        min_x: 0.0,
        min_y: 0.0,
        min_z: 7.0 * PX,
        max_x: 8.0 * PX,
        max_y: 1.0,
        max_z: 9.0 * PX,
    };
    const WALL_POST: Self = Self {
        min_x: 4.0 * PX,
        min_y: 0.0,
        min_z: 4.0 * PX,
        max_x: 12.0 * PX,
        max_y: 1.5,
        max_z: 12.0 * PX,
    };
    const WALL_NORTH_SIDE: Self = Self {
        min_x: 5.0 * PX,
        min_y: 0.0,
        min_z: 0.0,
        max_x: 11.0 * PX,
        max_y: 1.5,
        max_z: 11.0 * PX,
    };
    const WALL_EAST_SIDE: Self = Self {
        min_x: 5.0 * PX,
        min_y: 0.0,
        min_z: 5.0 * PX,
        max_x: 1.0,
        max_y: 1.5,
        max_z: 11.0 * PX,
    };
    const WALL_SOUTH_SIDE: Self = Self {
        min_x: 5.0 * PX,
        min_y: 0.0,
        min_z: 5.0 * PX,
        max_x: 11.0 * PX,
        max_y: 1.5,
        max_z: 1.0,
    };
    const WALL_WEST_SIDE: Self = Self {
        min_x: 0.0,
        min_y: 0.0,
        min_z: 5.0 * PX,
        max_x: 11.0 * PX,
        max_y: 1.5,
        max_z: 11.0 * PX,
    };

    fn cuboid(min_x: f64, min_y: f64, min_z: f64, max_x: f64, max_y: f64, max_z: f64) -> Self {
        Self {
            min_x,
            min_y,
            min_z,
            max_x,
            max_y,
            max_z,
        }
    }

    fn from_pixels(min: [f64; 3], max: [f64; 3]) -> Self {
        Self::cuboid(
            min[0] * PX,
            min[1] * PX,
            min[2] * PX,
            max[0] * PX,
            max[1] * PX,
            max[2] * PX,
        )
    }

    fn column(min_x: f64, min_z: f64, max_x: f64, max_y: f64, max_z: f64) -> Self {
        Self {
            min_x,
            min_y: 0.0,
            min_z,
            max_x,
            max_y,
            max_z,
        }
    }

    fn centered_column(width_x_px: f64, width_z_px: f64, min_y_px: f64, max_y_px: f64) -> Self {
        Self::from_pixels(
            [
                (16.0 - width_x_px) * 0.5,
                min_y_px,
                (16.0 - width_z_px) * 0.5,
            ],
            [
                (16.0 + width_x_px) * 0.5,
                max_y_px,
                (16.0 + width_z_px) * 0.5,
            ],
        )
    }

    fn centered_axis(axis: ShapeAxis, width_px: f64) -> Self {
        let min = ((16.0 - width_px) / 2.0) * PX;
        let max = ((16.0 + width_px) / 2.0) * PX;
        match axis {
            ShapeAxis::X => Self::cuboid(0.0, min, min, 1.0, max, max),
            ShapeAxis::Y => Self::cuboid(min, 0.0, min, max, 1.0, max),
            ShapeAxis::Z => Self::cuboid(min, min, 0.0, max, max, 1.0),
        }
    }

    fn invert_y(self) -> Self {
        Self {
            min_x: self.min_x,
            min_y: 1.0 - self.max_y,
            min_z: self.min_z,
            max_x: self.max_x,
            max_y: 1.0 - self.min_y,
            max_z: self.max_z,
        }
    }

    fn rotate_y_90(self) -> Self {
        Self {
            min_x: 1.0 - self.max_z,
            min_y: self.min_y,
            min_z: self.min_x,
            max_x: 1.0 - self.min_z,
            max_y: self.max_y,
            max_z: self.max_x,
        }
    }
}
