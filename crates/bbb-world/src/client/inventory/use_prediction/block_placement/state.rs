use std::collections::BTreeMap;

use bbb_protocol::{
    block_state_types::vanilla_block_default_state_id,
    block_types::{BlockTypeCatalogEntry, VanillaGetStateForPlacementOverride},
    packets::Direction,
};

use super::super::{LocalBlockUseContext, WorldStore};
use crate::{BlockPos, TerrainFluidKind};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum BlockPlacementState {
    State(i32),
    Unsupported,
}

pub(super) fn block_item_can_replace(
    world: &WorldStore,
    pos: BlockPos,
    block_type: &BlockTypeCatalogEntry,
    context: LocalBlockUseContext,
    replacing_clicked: bool,
) -> Option<bool> {
    let block = world.probe_block(pos)?;
    if block.block_name.as_deref() != Some(block_type.resource_id) {
        return world.local_block_is_replaceable_by_different_item(pos);
    }
    let owner = block_type.get_state_for_placement_override;
    Some(match owner {
        Some(VanillaGetStateForPlacementOverride::SlabBlock) => {
            let block_half = block.block_properties.get("type")?.as_str();
            if block_half == "double" {
                false
            } else if !replacing_clicked {
                true
            } else {
                let above = world_click_y(context) - f64::from(pos.y) > 0.5;
                let horizontal_face = direction_axis(context.face) != DirectionAxis::Y;
                match block_half {
                    "bottom" => context.face == Direction::Up || above && horizontal_face,
                    "top" => context.face == Direction::Down || !above && horizontal_face,
                    _ => return None,
                }
            }
        }
        Some(VanillaGetStateForPlacementOverride::SnowLayerBlock) => {
            integer_property(&block.block_properties, "layers")? < 8
                && (!replacing_clicked || context.face == Direction::Up)
        }
        Some(VanillaGetStateForPlacementOverride::CandleBlock) => {
            !context.secondary_use_active
                && integer_property(&block.block_properties, "candles")? < 4
        }
        Some(VanillaGetStateForPlacementOverride::SeaPickleBlock) => {
            !context.secondary_use_active
                && integer_property(&block.block_properties, "pickles")? < 4
        }
        Some(VanillaGetStateForPlacementOverride::TurtleEggBlock) => {
            !context.secondary_use_active && integer_property(&block.block_properties, "eggs")? < 4
        }
        Some(VanillaGetStateForPlacementOverride::FlowerBedBlock)
        | Some(VanillaGetStateForPlacementOverride::LeafLitterBlock) => {
            !context.secondary_use_active
                && integer_property(&block.block_properties, "segment_amount")? < 4
        }
        Some(VanillaGetStateForPlacementOverride::ScaffoldingBlock) => true,
        Some(VanillaGetStateForPlacementOverride::MultifaceBlock) => {
            ["down", "up", "north", "south", "west", "east"]
                .into_iter()
                .any(|property| {
                    block
                        .block_properties
                        .get(property)
                        .is_some_and(|value| value == "false")
                })
        }
        Some(VanillaGetStateForPlacementOverride::VineBlock) => {
            ["up", "north", "south", "west", "east"]
                .into_iter()
                .filter(|property| {
                    block
                        .block_properties
                        .get(*property)
                        .is_some_and(|value| value == "true")
                })
                .count()
                < 5
        }
        _ => false,
    })
}

pub(super) fn block_state_for_placement(
    world: &WorldStore,
    block_type: &BlockTypeCatalogEntry,
    target_pos: BlockPos,
    _replace_clicked: bool,
    context: LocalBlockUseContext,
) -> BlockPlacementState {
    let Some(default_state_id) = vanilla_block_default_state_id(block_type.id) else {
        return BlockPlacementState::Unsupported;
    };
    let Some(owner) = block_type.get_state_for_placement_override else {
        return BlockPlacementState::State(default_state_id);
    };
    let Some(pose) = world.local_player().pose else {
        return BlockPlacementState::Unsupported;
    };
    let Some(horizontal) = horizontal_direction(pose.y_rot) else {
        return BlockPlacementState::Unsupported;
    };
    let Some(nearest) = nearest_directions(pose.x_rot, pose.y_rot).map(|directions| directions[0])
    else {
        return BlockPlacementState::Unsupported;
    };

    let properties = match owner {
        VanillaGetStateForPlacementOverride::AbstractFurnaceBlock
        | VanillaGetStateForPlacementOverride::BeehiveBlock
        | VanillaGetStateForPlacementOverride::CarvedPumpkinBlock
        | VanillaGetStateForPlacementOverride::ChiseledBookShelfBlock
        | VanillaGetStateForPlacementOverride::EndPortalFrameBlock
        | VanillaGetStateForPlacementOverride::GlazedTerracottaBlock
        | VanillaGetStateForPlacementOverride::LoomBlock
        | VanillaGetStateForPlacementOverride::StonecutterBlock
        | VanillaGetStateForPlacementOverride::VaultBlock => {
            [("facing", direction_name(opposite(horizontal)))]
        }
        VanillaGetStateForPlacementOverride::AnvilBlock => {
            let Some(facing) = clockwise(horizontal) else {
                return BlockPlacementState::Unsupported;
            };
            [("facing", direction_name(facing))]
        }
        VanillaGetStateForPlacementOverride::BarrelBlock
        | VanillaGetStateForPlacementOverride::CommandBlock
        | VanillaGetStateForPlacementOverride::DispenserBlock
        | VanillaGetStateForPlacementOverride::PistonBaseBlock => {
            [("facing", direction_name(opposite(nearest)))]
        }
        VanillaGetStateForPlacementOverride::ObserverBlock => [("facing", direction_name(nearest))],
        VanillaGetStateForPlacementOverride::RotatedPillarBlock
        | VanillaGetStateForPlacementOverride::InfestedRotatedPillarBlock => {
            [("axis", direction_axis_name(context.face))]
        }
        VanillaGetStateForPlacementOverride::AmethystClusterBlock => {
            let Some(waterlogged) = target_is_source_water(world, target_pos) else {
                return BlockPlacementState::Unsupported;
            };
            return state_with_properties(
                world,
                block_type,
                default_state_id,
                [
                    ("facing", direction_name(context.face)),
                    ("waterlogged", boolean_name(waterlogged)),
                ],
            );
        }
        VanillaGetStateForPlacementOverride::ChainBlock => {
            let Some(waterlogged) = target_is_source_water(world, target_pos) else {
                return BlockPlacementState::Unsupported;
            };
            return state_with_properties(
                world,
                block_type,
                default_state_id,
                [
                    ("axis", direction_axis_name(context.face)),
                    ("waterlogged", boolean_name(waterlogged)),
                ],
            );
        }
        VanillaGetStateForPlacementOverride::ShulkerBoxBlock => {
            [("facing", direction_name(context.face))]
        }
        VanillaGetStateForPlacementOverride::HopperBlock => {
            let facing = opposite(context.face);
            let facing = if direction_axis(facing) == DirectionAxis::Y {
                Direction::Down
            } else {
                facing
            };
            [("facing", direction_name(facing))]
        }
        VanillaGetStateForPlacementOverride::DecoratedPotBlock => {
            let Some(waterlogged) = target_is_source_water(world, target_pos) else {
                return BlockPlacementState::Unsupported;
            };
            return state_with_properties(
                world,
                block_type,
                default_state_id,
                [
                    ("facing", direction_name(horizontal)),
                    ("waterlogged", boolean_name(waterlogged)),
                    ("cracked", "false"),
                ],
            );
        }
        VanillaGetStateForPlacementOverride::EnderChestBlock => {
            let Some(waterlogged) = target_is_source_water(world, target_pos) else {
                return BlockPlacementState::Unsupported;
            };
            return state_with_properties(
                world,
                block_type,
                default_state_id,
                [
                    ("facing", direction_name(opposite(horizontal))),
                    ("waterlogged", boolean_name(waterlogged)),
                ],
            );
        }
        VanillaGetStateForPlacementOverride::CalibratedSculkSensorBlock => {
            let Some(waterlogged) = target_is_source_water(world, target_pos) else {
                return BlockPlacementState::Unsupported;
            };
            return state_with_properties(
                world,
                block_type,
                default_state_id,
                [
                    ("facing", direction_name(horizontal)),
                    ("waterlogged", boolean_name(waterlogged)),
                ],
            );
        }
        VanillaGetStateForPlacementOverride::CampfireBlock => {
            let Some(waterlogged) = target_is_source_water(world, target_pos) else {
                return BlockPlacementState::Unsupported;
            };
            let Some(below) = world.probe_block(relative(target_pos, Direction::Down)) else {
                return BlockPlacementState::Unsupported;
            };
            let signal_fire = below.block_name.as_deref() == Some("minecraft:hay_block");
            return state_with_properties(
                world,
                block_type,
                default_state_id,
                [
                    ("facing", direction_name(horizontal)),
                    ("lit", boolean_name(!waterlogged)),
                    ("signal_fire", boolean_name(signal_fire)),
                    ("waterlogged", boolean_name(waterlogged)),
                ],
            );
        }
        VanillaGetStateForPlacementOverride::CopperGolemStatueBlock
        | VanillaGetStateForPlacementOverride::DriedGhastBlock => {
            let Some(waterlogged) = target_is_source_water(world, target_pos) else {
                return BlockPlacementState::Unsupported;
            };
            return state_with_properties(
                world,
                block_type,
                default_state_id,
                [
                    ("facing", direction_name(opposite(horizontal))),
                    ("waterlogged", boolean_name(waterlogged)),
                ],
            );
        }
        VanillaGetStateForPlacementOverride::LightningRodBlock => {
            let Some(waterlogged) = target_is_source_water(world, target_pos) else {
                return BlockPlacementState::Unsupported;
            };
            return state_with_properties(
                world,
                block_type,
                default_state_id,
                [
                    ("facing", direction_name(context.face)),
                    ("waterlogged", boolean_name(waterlogged)),
                ],
            );
        }
        VanillaGetStateForPlacementOverride::BarrierBlock
        | VanillaGetStateForPlacementOverride::HeavyCoreBlock
        | VanillaGetStateForPlacementOverride::MangroveRootsBlock
        | VanillaGetStateForPlacementOverride::SculkSensorBlock
        | VanillaGetStateForPlacementOverride::SculkShriekerBlock
        | VanillaGetStateForPlacementOverride::WaterloggedTransparentBlock => {
            let Some(waterlogged) = target_is_source_water(world, target_pos) else {
                return BlockPlacementState::Unsupported;
            };
            return state_with_properties(
                world,
                block_type,
                default_state_id,
                [("waterlogged", boolean_name(waterlogged))],
            );
        }
        VanillaGetStateForPlacementOverride::BaseCoralPlantTypeBlock
        | VanillaGetStateForPlacementOverride::ConduitBlock => {
            let Some(waterlogged) = target_is_full_water(world, target_pos) else {
                return BlockPlacementState::Unsupported;
            };
            return state_with_properties(
                world,
                block_type,
                default_state_id,
                [("waterlogged", boolean_name(waterlogged))],
            );
        }
        VanillaGetStateForPlacementOverride::SlabBlock => {
            if let Some(existing) = matching_target_block(world, target_pos, block_type) {
                let mut properties = existing.block_properties;
                properties.insert("type".to_string(), "double".to_string());
                properties.insert("waterlogged".to_string(), "false".to_string());
                return resolve_state(world, block_type.resource_id, properties);
            }
            let Some(waterlogged) = target_is_source_water(world, target_pos) else {
                return BlockPlacementState::Unsupported;
            };
            let local_y = world_click_y(context) - f64::from(target_pos.y);
            let half = if context.face != Direction::Down
                && (context.face == Direction::Up || local_y <= 0.5)
            {
                "bottom"
            } else {
                "top"
            };
            return state_with_properties(
                world,
                block_type,
                default_state_id,
                [("type", half), ("waterlogged", boolean_name(waterlogged))],
            );
        }
        VanillaGetStateForPlacementOverride::CandleBlock
        | VanillaGetStateForPlacementOverride::SnowLayerBlock
        | VanillaGetStateForPlacementOverride::SeaPickleBlock
        | VanillaGetStateForPlacementOverride::TurtleEggBlock => {
            let (property, maximum) = match owner {
                VanillaGetStateForPlacementOverride::CandleBlock => ("candles", 4),
                VanillaGetStateForPlacementOverride::SnowLayerBlock => ("layers", 8),
                VanillaGetStateForPlacementOverride::SeaPickleBlock => ("pickles", 4),
                VanillaGetStateForPlacementOverride::TurtleEggBlock => ("eggs", 4),
                _ => unreachable!("matched stacking owner"),
            };
            if let Some(existing) = matching_target_block(world, target_pos, block_type) {
                let Some(value) = integer_property(&existing.block_properties, property) else {
                    return BlockPlacementState::Unsupported;
                };
                let mut properties = existing.block_properties;
                properties.insert(property.to_string(), (value + 1).min(maximum).to_string());
                return resolve_state(world, block_type.resource_id, properties);
            }
            if matches!(
                owner,
                VanillaGetStateForPlacementOverride::CandleBlock
                    | VanillaGetStateForPlacementOverride::SeaPickleBlock
            ) {
                let Some(waterlogged) = target_is_source_water(world, target_pos) else {
                    return BlockPlacementState::Unsupported;
                };
                return state_with_properties(
                    world,
                    block_type,
                    default_state_id,
                    [("waterlogged", boolean_name(waterlogged))],
                );
            }
            return BlockPlacementState::State(default_state_id);
        }
        VanillaGetStateForPlacementOverride::FlowerBedBlock
        | VanillaGetStateForPlacementOverride::LeafLitterBlock => {
            if let Some(existing) = matching_target_block(world, target_pos, block_type) {
                let Some(amount) = integer_property(&existing.block_properties, "segment_amount")
                else {
                    return BlockPlacementState::Unsupported;
                };
                let mut properties = existing.block_properties;
                properties.insert(
                    "segment_amount".to_string(),
                    (amount + 1).min(4).to_string(),
                );
                return resolve_state(world, block_type.resource_id, properties);
            }
            return state_with_properties(
                world,
                block_type,
                default_state_id,
                [("facing", direction_name(opposite(horizontal)))],
            );
        }
        _ => return BlockPlacementState::Unsupported,
    };

    state_with_properties(world, block_type, default_state_id, properties)
}

fn state_with_properties<const N: usize>(
    world: &WorldStore,
    block_type: &BlockTypeCatalogEntry,
    default_state_id: i32,
    updates: [(&str, &str); N],
) -> BlockPlacementState {
    let Some(default_state) = world.registries().block_state(default_state_id) else {
        return BlockPlacementState::Unsupported;
    };
    if default_state.name != block_type.resource_id {
        return BlockPlacementState::Unsupported;
    }
    let mut properties = default_state.properties.clone();
    for (name, value) in updates {
        if !properties.contains_key(name) {
            return BlockPlacementState::Unsupported;
        }
        properties.insert(name.to_string(), value.to_string());
    }
    resolve_state(world, block_type.resource_id, properties)
}

fn resolve_state(
    world: &WorldStore,
    block_name: &str,
    properties: BTreeMap<String, String>,
) -> BlockPlacementState {
    world
        .registries()
        .block_state_id_by_name_and_properties(block_name, &properties)
        .map_or(BlockPlacementState::Unsupported, BlockPlacementState::State)
}

fn matching_target_block(
    world: &WorldStore,
    target_pos: BlockPos,
    block_type: &BlockTypeCatalogEntry,
) -> Option<crate::BlockProbe> {
    world
        .probe_block(target_pos)
        .filter(|block| block.block_name.as_deref() == Some(block_type.resource_id))
}

fn integer_property(properties: &BTreeMap<String, String>, property: &str) -> Option<i32> {
    properties.get(property)?.parse().ok()
}

fn world_click_y(context: LocalBlockUseContext) -> f64 {
    f64::from(context.pos.y) + context.cursor[1]
}

fn target_is_source_water(world: &WorldStore, target_pos: BlockPos) -> Option<bool> {
    world.probe_block(target_pos).map(|block| {
        block.fluid.is_some_and(|fluid| {
            fluid.kind == TerrainFluidKind::Water && fluid.amount == 8 && !fluid.falling
        })
    })
}

fn target_is_full_water(world: &WorldStore, target_pos: BlockPos) -> Option<bool> {
    world.probe_block(target_pos).map(|block| {
        block
            .fluid
            .is_some_and(|fluid| fluid.kind == TerrainFluidKind::Water && fluid.amount == 8)
    })
}

fn boolean_name(value: bool) -> &'static str {
    if value {
        "true"
    } else {
        "false"
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DirectionAxis {
    X,
    Y,
    Z,
}

fn direction_axis(direction: Direction) -> DirectionAxis {
    match direction {
        Direction::Down | Direction::Up => DirectionAxis::Y,
        Direction::North | Direction::South => DirectionAxis::Z,
        Direction::West | Direction::East => DirectionAxis::X,
    }
}

fn direction_axis_name(direction: Direction) -> &'static str {
    match direction_axis(direction) {
        DirectionAxis::X => "x",
        DirectionAxis::Y => "y",
        DirectionAxis::Z => "z",
    }
}

fn direction_name(direction: Direction) -> &'static str {
    match direction {
        Direction::Down => "down",
        Direction::Up => "up",
        Direction::North => "north",
        Direction::South => "south",
        Direction::West => "west",
        Direction::East => "east",
    }
}

fn opposite(direction: Direction) -> Direction {
    match direction {
        Direction::Down => Direction::Up,
        Direction::Up => Direction::Down,
        Direction::North => Direction::South,
        Direction::South => Direction::North,
        Direction::West => Direction::East,
        Direction::East => Direction::West,
    }
}

fn relative(pos: BlockPos, direction: Direction) -> BlockPos {
    match direction {
        Direction::Down => BlockPos {
            y: pos.y - 1,
            ..pos
        },
        Direction::Up => BlockPos {
            y: pos.y + 1,
            ..pos
        },
        Direction::North => BlockPos {
            z: pos.z - 1,
            ..pos
        },
        Direction::South => BlockPos {
            z: pos.z + 1,
            ..pos
        },
        Direction::West => BlockPos {
            x: pos.x - 1,
            ..pos
        },
        Direction::East => BlockPos {
            x: pos.x + 1,
            ..pos
        },
    }
}

fn clockwise(direction: Direction) -> Option<Direction> {
    match direction {
        Direction::North => Some(Direction::East),
        Direction::South => Some(Direction::West),
        Direction::West => Some(Direction::North),
        Direction::East => Some(Direction::South),
        Direction::Down | Direction::Up => None,
    }
}

fn horizontal_direction(y_rot: f32) -> Option<Direction> {
    if !y_rot.is_finite() {
        return None;
    }
    let data_value = ((f64::from(y_rot) / 90.0 + 0.5).floor() as i32) & 3;
    Some(match data_value {
        0 => Direction::South,
        1 => Direction::West,
        2 => Direction::North,
        3 => Direction::East,
        _ => unreachable!("masked horizontal direction"),
    })
}

fn nearest_directions(x_rot: f32, y_rot: f32) -> Option<[Direction; 6]> {
    if !x_rot.is_finite() || !y_rot.is_finite() {
        return None;
    }
    let pitch = x_rot.to_radians();
    let yaw = -y_rot.to_radians();
    let pitch_sin = pitch.sin();
    let pitch_cos = pitch.cos();
    let yaw_sin = yaw.sin();
    let yaw_cos = yaw.cos();
    let x_pos = yaw_sin > 0.0;
    let y_pos = pitch_sin < 0.0;
    let z_pos = yaw_cos > 0.0;
    let x_yaw = yaw_sin.abs();
    let y_mag = pitch_sin.abs();
    let z_yaw = yaw_cos.abs();
    let x_mag = x_yaw * pitch_cos;
    let z_mag = z_yaw * pitch_cos;
    let axis_x = if x_pos {
        Direction::East
    } else {
        Direction::West
    };
    let axis_y = if y_pos {
        Direction::Up
    } else {
        Direction::Down
    };
    let axis_z = if z_pos {
        Direction::South
    } else {
        Direction::North
    };
    Some(if x_yaw > z_yaw {
        if y_mag > x_mag {
            direction_array(axis_y, axis_x, axis_z)
        } else if z_mag > y_mag {
            direction_array(axis_x, axis_z, axis_y)
        } else {
            direction_array(axis_x, axis_y, axis_z)
        }
    } else if y_mag > z_mag {
        direction_array(axis_y, axis_z, axis_x)
    } else if x_mag > y_mag {
        direction_array(axis_z, axis_x, axis_y)
    } else {
        direction_array(axis_z, axis_y, axis_x)
    })
}

fn direction_array(axis_1: Direction, axis_2: Direction, axis_3: Direction) -> [Direction; 6] {
    [
        axis_1,
        axis_2,
        axis_3,
        opposite(axis_3),
        opposite(axis_2),
        opposite(axis_1),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn horizontal_direction_matches_vanilla_quadrants_and_boundaries() {
        assert_eq!(horizontal_direction(0.0), Some(Direction::South));
        assert_eq!(horizontal_direction(44.999), Some(Direction::South));
        assert_eq!(horizontal_direction(45.0), Some(Direction::West));
        assert_eq!(horizontal_direction(90.0), Some(Direction::West));
        assert_eq!(horizontal_direction(180.0), Some(Direction::North));
        assert_eq!(horizontal_direction(270.0), Some(Direction::East));
        assert_eq!(horizontal_direction(-90.0), Some(Direction::East));
        assert_eq!(horizontal_direction(f32::NAN), None);
    }

    #[test]
    fn nearest_direction_matches_vanilla_view_axes() {
        assert_eq!(
            nearest_directions(0.0, 0.0).map(|directions| directions[0]),
            Some(Direction::South)
        );
        assert_eq!(
            nearest_directions(0.0, 90.0).map(|directions| directions[0]),
            Some(Direction::West)
        );
        assert_eq!(
            nearest_directions(-90.0, 0.0).map(|directions| directions[0]),
            Some(Direction::Up)
        );
        assert_eq!(
            nearest_directions(90.0, 0.0).map(|directions| directions[0]),
            Some(Direction::Down)
        );
    }
}
