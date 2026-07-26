use bbb_protocol::{
    block_state_types::VanillaBlockSupportType,
    block_types::{
        vanilla_block_type_for_resource_id, BlockTypeCatalogEntry,
        VanillaGetStateForPlacementOverride,
    },
    item_types::{VANILLA_AXE_SCRAPEABLES, VANILLA_AXE_WAX_OFF},
    packets::Direction,
};

use super::{
    super::{find_block_transform, LocalBlockUseContext, WorldStore},
    context::{clockwise, counterclockwise, direction_axis, opposite, relative, DirectionAxis},
    signal::placement_neighbor_signal_is_proven_absent,
};
use crate::{terrain::block_name_is_leaves, BlockPos, BlockProbe};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct CardinalConnections {
    pub(super) north: bool,
    pub(super) east: bool,
    pub(super) south: bool,
    pub(super) west: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct ChestPlacement {
    pub(super) facing: Direction,
    pub(super) chest_type: &'static str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct CopperChestPlacement {
    pub(super) block_type_id: i32,
    pub(super) facing: Direction,
    pub(super) chest_type: &'static str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ChestConnectionKind {
    SameBlock,
    CopperTag,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct FenceGatePlacement {
    pub(super) in_wall: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct WallPlacement {
    pub(super) up: bool,
    pub(super) north: &'static str,
    pub(super) east: &'static str,
    pub(super) south: &'static str,
    pub(super) west: &'static str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct MossyCarpetPlacement {
    pub(super) north: &'static str,
    pub(super) east: &'static str,
    pub(super) south: &'static str,
    pub(super) west: &'static str,
}

pub(super) fn fence_connections(
    world: &WorldStore,
    pos: BlockPos,
    placing_block: &BlockTypeCatalogEntry,
) -> Option<CardinalConnections> {
    cardinal_connections(world, pos, |world, neighbour, direction| {
        fence_connects_to(world, neighbour, placing_block, opposite(direction))
    })
}

pub(super) fn fence_gate_placement(
    world: &WorldStore,
    pos: BlockPos,
    facing: Direction,
) -> Option<FenceGatePlacement> {
    placement_neighbor_signal_is_proven_absent(world, &[pos])?;
    let wall_directions = match direction_axis(facing) {
        DirectionAxis::Z => [Direction::West, Direction::East],
        DirectionAxis::X => [Direction::North, Direction::South],
        DirectionAxis::Y => return None,
    };
    let mut in_wall = false;
    for direction in wall_directions {
        let neighbour = world.probe_block(relative(pos, direction))?;
        in_wall |= world.placement_block_is_in_tag(&neighbour, "minecraft:walls")?;
    }
    Some(FenceGatePlacement { in_wall })
}

pub(super) fn iron_bars_connections(
    world: &WorldStore,
    pos: BlockPos,
) -> Option<CardinalConnections> {
    cardinal_connections(world, pos, |world, neighbour, direction| {
        iron_bars_connects_to(world, neighbour, opposite(direction))
    })
}

pub(super) fn wall_placement(world: &WorldStore, pos: BlockPos) -> Option<WallPlacement> {
    let connections = cardinal_connections(world, pos, |world, neighbour, direction| {
        wall_connects_to(world, neighbour, opposite(direction))
    })?;
    let above = world.probe_block(relative(pos, Direction::Up))?;
    let north = wall_side(world, &above, connections.north, [7.0, 9.0, 0.0, 9.0]);
    let east = wall_side(world, &above, connections.east, [7.0, 16.0, 7.0, 9.0]);
    let south = wall_side(world, &above, connections.south, [7.0, 9.0, 7.0, 16.0]);
    let west = wall_side(world, &above, connections.west, [0.0, 9.0, 7.0, 9.0]);

    let top_neighbour_has_post = catalog_block_type(&above).is_some_and(|block_type| {
        block_type.get_state_for_placement_override
            == Some(VanillaGetStateForPlacementOverride::WallBlock)
            && above
                .block_properties
                .get("up")
                .is_some_and(|up| up == "true")
    });
    let north_none = north == "none";
    let east_none = east == "none";
    let south_none = south == "none";
    let west_none = west == "none";
    let has_corner = north_none && south_none && west_none && east_none
        || north_none != south_none
        || west_none != east_none;
    let has_high_wall = north == "tall" && south == "tall" || east == "tall" && west == "tall";
    let up = if top_neighbour_has_post || has_corner {
        true
    } else if has_high_wall {
        false
    } else {
        world.placement_block_is_in_tag(&above, "minecraft:wall_post_override")?
            || above_covers_wall_test(world, &above, [7.0, 9.0, 7.0, 9.0])
    };

    Some(WallPlacement {
        up,
        north,
        east,
        south,
        west,
    })
}

pub(super) fn mossy_carpet_placement(
    world: &WorldStore,
    pos: BlockPos,
) -> Option<MossyCarpetPlacement> {
    let above = world.probe_block(relative(pos, Direction::Up))?;
    let above_is_topper = if above.block_name.as_deref() == Some("minecraft:pale_moss_carpet") {
        match above.block_properties.get("bottom")?.as_str() {
            "false" => true,
            "true" => false,
            _ => return None,
        }
    } else {
        false
    };
    let side = |direction, property| {
        let neighbour = world.probe_block(relative(pos, direction))?;
        if !world.placement_full_support_or_collision_face(&neighbour, opposite(direction))? {
            return Some("none");
        }
        if above_is_topper {
            match above.block_properties.get(property)?.as_str() {
                "low" | "tall" => return Some("tall"),
                "none" => {}
                _ => return None,
            }
        }
        Some("low")
    };

    Some(MossyCarpetPlacement {
        north: side(Direction::North, "north")?,
        east: side(Direction::East, "east")?,
        south: side(Direction::South, "south")?,
        west: side(Direction::West, "west")?,
    })
}

pub(super) fn chest_placement(
    world: &WorldStore,
    pos: BlockPos,
    block_type: &BlockTypeCatalogEntry,
    horizontal: Direction,
    context: LocalBlockUseContext,
) -> Option<ChestPlacement> {
    chest_placement_with_connection(
        world,
        pos,
        block_type,
        horizontal,
        context,
        ChestConnectionKind::SameBlock,
    )
}

pub(super) fn copper_chest_placement(
    world: &WorldStore,
    pos: BlockPos,
    block_type: &BlockTypeCatalogEntry,
    horizontal: Direction,
    context: LocalBlockUseContext,
) -> Option<CopperChestPlacement> {
    let placement = chest_placement_with_connection(
        world,
        pos,
        block_type,
        horizontal,
        context,
        ChestConnectionKind::CopperTag,
    )?;
    let result_block_type_id = if placement.chest_type == "single" {
        block_type.id
    } else {
        let connected_direction = match placement.chest_type {
            "left" => clockwise(placement.facing)?,
            "right" => counterclockwise(placement.facing)?,
            _ => return None,
        };
        let connected = world.probe_block(relative(pos, connected_direction))?;
        let connected_block_type = catalog_block_type(&connected)?;
        if connected_block_type.get_state_for_placement_override
            == Some(VanillaGetStateForPlacementOverride::CopperChestBlock)
        {
            least_oxidized_copper_chest(block_type, connected_block_type)?
        } else {
            block_type.id
        }
    };
    Some(CopperChestPlacement {
        block_type_id: result_block_type_id,
        facing: placement.facing,
        chest_type: placement.chest_type,
    })
}

fn chest_placement_with_connection(
    world: &WorldStore,
    pos: BlockPos,
    block_type: &BlockTypeCatalogEntry,
    horizontal: Direction,
    context: LocalBlockUseContext,
    connection_kind: ChestConnectionKind,
) -> Option<ChestPlacement> {
    let mut facing = opposite(horizontal);
    let mut chest_type = "single";
    if direction_axis(context.face) != DirectionAxis::Y && context.secondary_use_active {
        let neighbour_direction = opposite(context.face);
        if let Some(neighbour_facing) = candidate_chest_partner_facing(
            world,
            pos,
            neighbour_direction,
            block_type,
            connection_kind,
        )? {
            if direction_axis(neighbour_facing) != direction_axis(context.face) {
                facing = neighbour_facing;
                chest_type = if counterclockwise(facing)? == neighbour_direction {
                    "right"
                } else {
                    "left"
                };
            }
        }
    }

    if chest_type == "single" && !context.secondary_use_active {
        if candidate_chest_partner_facing(
            world,
            pos,
            clockwise(facing)?,
            block_type,
            connection_kind,
        )? == Some(facing)
        {
            chest_type = "left";
        } else if candidate_chest_partner_facing(
            world,
            pos,
            counterclockwise(facing)?,
            block_type,
            connection_kind,
        )? == Some(facing)
        {
            chest_type = "right";
        }
    }

    Some(ChestPlacement { facing, chest_type })
}

pub(super) fn stair_shape(
    world: &WorldStore,
    pos: BlockPos,
    facing: Direction,
    half: &str,
) -> Option<&'static str> {
    let behind = world.probe_block(relative(pos, facing))?;
    if block_is_stair(&behind)
        && behind.block_properties.get("half").map(String::as_str) == Some(half)
    {
        let behind_facing = direction_property(&behind, "facing")?;
        if direction_axis(behind_facing) != direction_axis(facing)
            && stair_can_take_shape(world, pos, facing, half, opposite(behind_facing))?
        {
            return Some(if behind_facing == counterclockwise(facing)? {
                "outer_left"
            } else {
                "outer_right"
            });
        }
    }

    let front = world.probe_block(relative(pos, opposite(facing)))?;
    if block_is_stair(&front)
        && front.block_properties.get("half").map(String::as_str) == Some(half)
    {
        let front_facing = direction_property(&front, "facing")?;
        if direction_axis(front_facing) != direction_axis(facing)
            && stair_can_take_shape(world, pos, facing, half, front_facing)?
        {
            return Some(if front_facing == counterclockwise(facing)? {
                "inner_left"
            } else {
                "inner_right"
            });
        }
    }
    Some("straight")
}

fn cardinal_connections(
    world: &WorldStore,
    pos: BlockPos,
    mut connects: impl FnMut(&WorldStore, &BlockProbe, Direction) -> Option<bool>,
) -> Option<CardinalConnections> {
    let north = connects(
        world,
        &world.probe_block(relative(pos, Direction::North))?,
        Direction::North,
    )?;
    let east = connects(
        world,
        &world.probe_block(relative(pos, Direction::East))?,
        Direction::East,
    )?;
    let south = connects(
        world,
        &world.probe_block(relative(pos, Direction::South))?,
        Direction::South,
    )?;
    let west = connects(
        world,
        &world.probe_block(relative(pos, Direction::West))?,
        Direction::West,
    )?;
    Some(CardinalConnections {
        north,
        east,
        south,
        west,
    })
}

fn fence_connects_to(
    world: &WorldStore,
    neighbour: &BlockProbe,
    placing_block: &BlockTypeCatalogEntry,
    direction_to_placing_block: Direction,
) -> Option<bool> {
    let face_solid = world.placement_face_sturdy(
        neighbour,
        direction_to_placing_block,
        VanillaBlockSupportType::Full,
    )?;
    if !connection_exception(world, neighbour)? && face_solid {
        return Some(true);
    }
    let same_fence = world.placement_block_is_in_tag(neighbour, "minecraft:fences")?
        && world.placement_block_is_in_tag(neighbour, "minecraft:wooden_fences")?
            == block_type_is_in_tag(world, placing_block, "minecraft:wooden_fences")?;
    if same_fence {
        return Some(true);
    }
    fence_gate_connects_to(neighbour, direction_to_placing_block)
}

fn iron_bars_connects_to(
    world: &WorldStore,
    neighbour: &BlockProbe,
    direction_to_placing_block: Direction,
) -> Option<bool> {
    let face_solid = world.placement_face_sturdy(
        neighbour,
        direction_to_placing_block,
        VanillaBlockSupportType::Full,
    )?;
    if !connection_exception(world, neighbour)? && face_solid {
        return Some(true);
    }
    let Some(block_type) = catalog_block_type(neighbour) else {
        return None;
    };
    if block_type.get_state_for_placement_override
        == Some(VanillaGetStateForPlacementOverride::IronBarsBlock)
    {
        return Some(true);
    }
    world
        .placement_block_is_in_tag(neighbour, "minecraft:walls")
        .map(|is_wall| is_wall)
}

fn wall_connects_to(
    world: &WorldStore,
    neighbour: &BlockProbe,
    direction_to_placing_block: Direction,
) -> Option<bool> {
    if world.placement_block_is_in_tag(neighbour, "minecraft:walls")? {
        return Some(true);
    }
    let face_solid = world.placement_face_sturdy(
        neighbour,
        direction_to_placing_block,
        VanillaBlockSupportType::Full,
    )?;
    if !connection_exception(world, neighbour)? && face_solid {
        return Some(true);
    }
    let Some(block_type) = catalog_block_type(neighbour) else {
        return None;
    };
    if block_type.get_state_for_placement_override
        == Some(VanillaGetStateForPlacementOverride::IronBarsBlock)
    {
        return Some(true);
    }
    fence_gate_connects_to(neighbour, direction_to_placing_block)
}

fn wall_side(
    world: &WorldStore,
    above: &BlockProbe,
    connected: bool,
    test_pixels: [f64; 4],
) -> &'static str {
    if !connected {
        "none"
    } else if above_covers_wall_test(world, above, test_pixels) {
        "tall"
    } else {
        "low"
    }
}

fn above_covers_wall_test(world: &WorldStore, above: &BlockProbe, test_pixels: [f64; 4]) -> bool {
    world.block_empty_context_collision_face_covers_rectangle(
        above,
        Direction::Down,
        test_pixels.map(|coordinate| coordinate / 16.0),
    )
}

fn fence_gate_connects_to(
    neighbour: &BlockProbe,
    direction_to_placing_block: Direction,
) -> Option<bool> {
    let Some(block_type) = catalog_block_type(neighbour) else {
        return None;
    };
    if block_type.get_state_for_placement_override
        != Some(VanillaGetStateForPlacementOverride::FenceGateBlock)
    {
        return Some(false);
    }
    let facing = direction_property(neighbour, "facing")?;
    Some(direction_axis(facing) != direction_axis(direction_to_placing_block))
}

fn candidate_chest_partner_facing(
    world: &WorldStore,
    pos: BlockPos,
    neighbour_direction: Direction,
    block_type: &BlockTypeCatalogEntry,
    connection_kind: ChestConnectionKind,
) -> Option<Option<Direction>> {
    let neighbour = world.probe_block(relative(pos, neighbour_direction))?;
    let can_connect = match connection_kind {
        ChestConnectionKind::SameBlock => {
            neighbour.block_name.as_deref() == Some(block_type.resource_id)
        }
        ChestConnectionKind::CopperTag => {
            world.placement_block_is_in_tag(&neighbour, "minecraft:copper_chests")?
        }
    };
    if !can_connect {
        return Some(None);
    }
    if neighbour.block_properties.get("type").map(String::as_str) != Some("single") {
        return Some(None);
    }
    direction_property(&neighbour, "facing").map(Some)
}

fn least_oxidized_copper_chest(
    placing: &BlockTypeCatalogEntry,
    connected: &BlockTypeCatalogEntry,
) -> Option<i32> {
    let (placing_unwaxed, placing_waxed) = unwaxed_block_type(placing)?;
    let (connected_unwaxed, connected_waxed) = unwaxed_block_type(connected)?;
    let wax_mismatch = placing_waxed != connected_waxed;
    let placing_rank = copper_weathering_rank(placing_unwaxed)?;
    let connected_rank = copper_weathering_rank(connected_unwaxed)?;
    if placing_rank <= connected_rank {
        Some(if wax_mismatch {
            placing_unwaxed
        } else {
            placing.id
        })
    } else {
        Some(if wax_mismatch {
            connected_unwaxed
        } else {
            connected.id
        })
    }
}

fn unwaxed_block_type(block_type: &BlockTypeCatalogEntry) -> Option<(i32, bool)> {
    match find_block_transform(VANILLA_AXE_WAX_OFF, block_type.id) {
        Some(transform) => Some((transform.target_block_type_id, true)),
        None => Some((block_type.id, false)),
    }
}

fn copper_weathering_rank(mut block_type_id: i32) -> Option<usize> {
    for rank in 0..=VANILLA_AXE_SCRAPEABLES.len() {
        let Some(transform) = find_block_transform(VANILLA_AXE_SCRAPEABLES, block_type_id) else {
            return Some(rank);
        };
        block_type_id = transform.target_block_type_id;
    }
    None
}

fn connection_exception(world: &WorldStore, block: &BlockProbe) -> Option<bool> {
    let name = block.block_name.as_deref()?;
    if block_name_is_leaves(name)
        || matches!(
            name,
            "minecraft:barrier"
                | "minecraft:carved_pumpkin"
                | "minecraft:jack_o_lantern"
                | "minecraft:melon"
                | "minecraft:pumpkin"
        )
    {
        return Some(true);
    }
    world.placement_block_is_in_tag(block, "minecraft:shulker_boxes")
}

fn block_type_is_in_tag(
    world: &WorldStore,
    block_type: &BlockTypeCatalogEntry,
    tag: &str,
) -> Option<bool> {
    world
        .registry_holder_ids_for_tag("minecraft:block", tag)
        .map(|ids| ids.contains(&block_type.id))
}

fn catalog_block_type(block: &BlockProbe) -> Option<&'static BlockTypeCatalogEntry> {
    block
        .block_name
        .as_deref()
        .and_then(vanilla_block_type_for_resource_id)
}

fn direction_property(block: &BlockProbe, property: &str) -> Option<Direction> {
    match block.block_properties.get(property)?.as_str() {
        "down" => Some(Direction::Down),
        "up" => Some(Direction::Up),
        "north" => Some(Direction::North),
        "south" => Some(Direction::South),
        "west" => Some(Direction::West),
        "east" => Some(Direction::East),
        _ => None,
    }
}

fn stair_can_take_shape(
    world: &WorldStore,
    pos: BlockPos,
    facing: Direction,
    half: &str,
    neighbour_direction: Direction,
) -> Option<bool> {
    let neighbour = world.probe_block(relative(pos, neighbour_direction))?;
    if !block_is_stair(&neighbour) {
        return Some(true);
    }
    Some(
        direction_property(&neighbour, "facing")? != facing
            || neighbour.block_properties.get("half").map(String::as_str) != Some(half),
    )
}

fn block_is_stair(block: &BlockProbe) -> bool {
    catalog_block_type(block).is_some_and(|block_type| {
        block_type.get_state_for_placement_override
            == Some(VanillaGetStateForPlacementOverride::StairBlock)
    })
}
