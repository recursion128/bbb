use std::collections::BTreeMap;

use bbb_protocol::{
    block_state_types::vanilla_block_default_state_id,
    block_types::{
        vanilla_block_type, BlockTypeCatalogEntry, VanillaGetStateForPlacementOverride,
        VANILLA_BLOCK_BAMBOO_SAPLING_ID,
    },
    packets::Direction,
};

use super::super::{LocalBlockUseContext, WorldStore};
use super::attachment::{
    attached_block_state_for_placement, bell_placement, cocoa_facing, pointed_dripstone_placement,
    trapdoor_placement,
};
use super::composite::{door_placement, double_high_above_can_replace};
use super::connections::{
    chest_placement, chorus_plant_placement, copper_chest_placement, fence_connections,
    fence_gate_placement, iron_bars_connections, mossy_carpet_placement, stair_shape,
    wall_placement,
};
use super::context::{
    clockwise, direction_axis, direction_axis_name, direction_name, front_and_top_name,
    horizontal_direction, nearest_directions, opposite, placement_directions, relative,
    rotation_segment, world_click_y, DirectionAxis,
};
use crate::{BlockPos, EnvironmentBiomeSampling, TerrainFluidKind};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum BlockPlacementState {
    State(i32),
    Fail,
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
    replace_clicked: bool,
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
    let Some(nearest_directions) = nearest_directions(pose.x_rot, pose.y_rot) else {
        return BlockPlacementState::Unsupported;
    };
    let Some(directions) = placement_directions(pose.x_rot, pose.y_rot, context, replace_clicked)
    else {
        return BlockPlacementState::Unsupported;
    };
    let nearest = nearest_directions[0];

    let properties = match owner {
        VanillaGetStateForPlacementOverride::AbstractFurnaceBlock
        | VanillaGetStateForPlacementOverride::BeehiveBlock
        | VanillaGetStateForPlacementOverride::CarvedPumpkinBlock
        | VanillaGetStateForPlacementOverride::ChiseledBookShelfBlock
        | VanillaGetStateForPlacementOverride::EndPortalFrameBlock
        | VanillaGetStateForPlacementOverride::GlazedTerracottaBlock
        | VanillaGetStateForPlacementOverride::LecternBlock
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
        VanillaGetStateForPlacementOverride::BambooStalkBlock => {
            let Some(target) = world.probe_block(target_pos) else {
                return BlockPlacementState::Unsupported;
            };
            if target.fluid.is_some() {
                return BlockPlacementState::Fail;
            }
            let Some(below) = world.probe_block(relative(target_pos, Direction::Down)) else {
                return BlockPlacementState::Unsupported;
            };
            let Some(supported) =
                world.placement_block_is_in_tag(&below, "minecraft:supports_bamboo")
            else {
                return BlockPlacementState::Unsupported;
            };
            if !supported {
                return BlockPlacementState::Fail;
            }
            if below.block_name.as_deref() == Some("minecraft:bamboo_sapling") {
                return BlockPlacementState::State(default_state_id);
            }
            if below.block_name.as_deref() == Some("minecraft:bamboo") {
                let Some(age) = integer_property(&below.block_properties, "age") else {
                    return BlockPlacementState::Unsupported;
                };
                return state_with_properties(
                    world,
                    block_type,
                    default_state_id,
                    [("age", if age > 0 { "1" } else { "0" })],
                );
            }
            let Some(above) = world.probe_block(relative(target_pos, Direction::Up)) else {
                return BlockPlacementState::Unsupported;
            };
            if above.block_name.as_deref() == Some("minecraft:bamboo") {
                let Some(age) = integer_property(&above.block_properties, "age") else {
                    return BlockPlacementState::Unsupported;
                };
                return state_with_properties(
                    world,
                    block_type,
                    default_state_id,
                    [("age", if age > 0 { "1" } else { "0" })],
                );
            }
            return vanilla_block_default_state_id(VANILLA_BLOCK_BAMBOO_SAPLING_ID)
                .map_or(BlockPlacementState::Unsupported, BlockPlacementState::State);
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
        VanillaGetStateForPlacementOverride::BaseRailBlock => {
            let Some(waterlogged) = target_is_source_water(world, target_pos) else {
                return BlockPlacementState::Unsupported;
            };
            let shape = if direction_axis(horizontal) == DirectionAxis::X {
                "east_west"
            } else {
                "north_south"
            };
            return state_with_properties(
                world,
                block_type,
                default_state_id,
                [("shape", shape), ("waterlogged", boolean_name(waterlogged))],
            );
        }
        VanillaGetStateForPlacementOverride::BigDripleafBlock => {
            let Some(waterlogged) = target_is_source_water(world, target_pos) else {
                return BlockPlacementState::Unsupported;
            };
            let Some(below) = world.probe_block(relative(target_pos, Direction::Down)) else {
                return BlockPlacementState::Unsupported;
            };
            let facing = if matches!(
                below.block_name.as_deref(),
                Some("minecraft:big_dripleaf" | "minecraft:big_dripleaf_stem")
            ) {
                let Some(facing) = direction_property(&below.block_properties, "facing") else {
                    return BlockPlacementState::Unsupported;
                };
                facing
            } else {
                opposite(horizontal)
            };
            return state_with_properties(
                world,
                block_type,
                default_state_id,
                [
                    ("facing", direction_name(facing)),
                    ("waterlogged", boolean_name(waterlogged)),
                ],
            );
        }
        VanillaGetStateForPlacementOverride::BellBlock => {
            let Some(placement) = bell_placement(world, target_pos, horizontal, context.face)
            else {
                return BlockPlacementState::Unsupported;
            };
            let Some(placement) = placement else {
                return BlockPlacementState::Fail;
            };
            return state_with_properties(
                world,
                block_type,
                default_state_id,
                [
                    ("facing", direction_name(placement.facing)),
                    ("attachment", placement.attachment),
                    ("powered", "false"),
                ],
            );
        }
        VanillaGetStateForPlacementOverride::BedBlock => {
            let head_pos = relative(target_pos, horizontal);
            let Some(replaceable) =
                block_item_can_replace(world, head_pos, block_type, context, false)
            else {
                return BlockPlacementState::Unsupported;
            };
            if !replaceable || !world.world_border().contains_block_pos(head_pos) {
                return BlockPlacementState::Fail;
            }
            return state_with_properties(
                world,
                block_type,
                default_state_id,
                [("facing", direction_name(horizontal))],
            );
        }
        VanillaGetStateForPlacementOverride::DoorBlock => {
            let Some(above_replaceable) =
                double_high_above_can_replace(world, target_pos, block_type, context)
            else {
                return BlockPlacementState::Unsupported;
            };
            if !above_replaceable {
                return BlockPlacementState::Fail;
            }
            let Some(placement) = door_placement(world, target_pos, horizontal, context) else {
                return BlockPlacementState::Unsupported;
            };
            return state_with_properties(
                world,
                block_type,
                default_state_id,
                [
                    ("facing", direction_name(horizontal)),
                    ("half", "lower"),
                    ("hinge", placement.hinge),
                    ("open", boolean_name(placement.powered)),
                    ("powered", boolean_name(placement.powered)),
                ],
            );
        }
        VanillaGetStateForPlacementOverride::DoublePlantBlock => {
            let Some(above_replaceable) =
                double_high_above_can_replace(world, target_pos, block_type, context)
            else {
                return BlockPlacementState::Unsupported;
            };
            if !above_replaceable {
                return BlockPlacementState::Fail;
            }
            return state_with_properties(world, block_type, default_state_id, [("half", "lower")]);
        }
        VanillaGetStateForPlacementOverride::SmallDripleafBlock => {
            let Some(above_replaceable) =
                double_high_above_can_replace(world, target_pos, block_type, context)
            else {
                return BlockPlacementState::Unsupported;
            };
            if !above_replaceable {
                return BlockPlacementState::Fail;
            }
            let Some(waterlogged) = target_is_source_water(world, target_pos) else {
                return BlockPlacementState::Unsupported;
            };
            return state_with_properties(
                world,
                block_type,
                default_state_id,
                [
                    ("facing", direction_name(opposite(horizontal))),
                    ("half", "lower"),
                    ("waterlogged", boolean_name(waterlogged)),
                ],
            );
        }
        VanillaGetStateForPlacementOverride::CocoaBlock => {
            let Some(facing) = cocoa_facing(world, target_pos, directions) else {
                return BlockPlacementState::Unsupported;
            };
            let Some(facing) = facing else {
                return BlockPlacementState::Fail;
            };
            return state_with_properties(
                world,
                block_type,
                default_state_id,
                [("facing", direction_name(facing)), ("age", "0")],
            );
        }
        VanillaGetStateForPlacementOverride::EndRodBlock => {
            let Some(behind) = world.probe_block(relative(target_pos, opposite(context.face)))
            else {
                return BlockPlacementState::Unsupported;
            };
            let same_direction = behind.block_name.as_deref() == Some(block_type.resource_id)
                && direction_property(&behind.block_properties, "facing") == Some(context.face);
            let facing = if same_direction {
                opposite(context.face)
            } else {
                context.face
            };
            return state_with_properties(
                world,
                block_type,
                default_state_id,
                [("facing", direction_name(facing))],
            );
        }
        VanillaGetStateForPlacementOverride::JigsawBlock => {
            let top = if direction_axis(context.face) == DirectionAxis::Y {
                opposite(horizontal)
            } else {
                Direction::Up
            };
            let Some(orientation) = front_and_top_name(context.face, top) else {
                return BlockPlacementState::Unsupported;
            };
            return state_with_properties(
                world,
                block_type,
                default_state_id,
                [("orientation", orientation)],
            );
        }
        VanillaGetStateForPlacementOverride::BannerBlock => {
            let Some(rotation) = rotation_segment(pose.y_rot + 180.0) else {
                return BlockPlacementState::Unsupported;
            };
            return state_with_owned_properties(
                world,
                block_type,
                default_state_id,
                [("rotation", rotation.to_string())],
            );
        }
        VanillaGetStateForPlacementOverride::StandingSignBlock => {
            let Some(rotation) = rotation_segment(pose.y_rot + 180.0) else {
                return BlockPlacementState::Unsupported;
            };
            let Some(waterlogged) = target_is_source_water(world, target_pos) else {
                return BlockPlacementState::Unsupported;
            };
            return state_with_owned_properties(
                world,
                block_type,
                default_state_id,
                [
                    ("rotation", rotation.to_string()),
                    ("waterlogged", boolean_name(waterlogged).to_string()),
                ],
            );
        }
        VanillaGetStateForPlacementOverride::SkullBlock => {
            let Some(rotation) = rotation_segment(pose.y_rot) else {
                return BlockPlacementState::Unsupported;
            };
            return state_with_owned_properties(
                world,
                block_type,
                default_state_id,
                [("rotation", rotation.to_string())],
            );
        }
        VanillaGetStateForPlacementOverride::ChestBlock => {
            let Some(waterlogged) = target_is_source_water(world, target_pos) else {
                return BlockPlacementState::Unsupported;
            };
            let Some(placement) =
                chest_placement(world, target_pos, block_type, horizontal, context)
            else {
                return BlockPlacementState::Unsupported;
            };
            return state_with_properties(
                world,
                block_type,
                default_state_id,
                [
                    ("facing", direction_name(placement.facing)),
                    ("type", placement.chest_type),
                    ("waterlogged", boolean_name(waterlogged)),
                ],
            );
        }
        VanillaGetStateForPlacementOverride::CopperChestBlock => {
            let Some(waterlogged) = target_is_source_water(world, target_pos) else {
                return BlockPlacementState::Unsupported;
            };
            let Some(placement) =
                copper_chest_placement(world, target_pos, block_type, horizontal, context)
            else {
                return BlockPlacementState::Unsupported;
            };
            let Some(placed_block_type) = vanilla_block_type(placement.block_type_id) else {
                return BlockPlacementState::Unsupported;
            };
            let Some(placed_default_state_id) =
                vanilla_block_default_state_id(placed_block_type.id)
            else {
                return BlockPlacementState::Unsupported;
            };
            return state_with_properties(
                world,
                placed_block_type,
                placed_default_state_id,
                [
                    ("facing", direction_name(placement.facing)),
                    ("type", placement.chest_type),
                    ("waterlogged", boolean_name(waterlogged)),
                ],
            );
        }
        VanillaGetStateForPlacementOverride::CreakingHeartBlock => {
            let axis = direction_axis(context.face);
            let (axis_name, axial_directions) = match axis {
                DirectionAxis::X => ("x", [Direction::West, Direction::East]),
                DirectionAxis::Y => ("y", [Direction::Down, Direction::Up]),
                DirectionAxis::Z => ("z", [Direction::North, Direction::South]),
            };
            let mut has_required_logs = true;
            for direction in axial_directions {
                let Some(neighbour) = world.probe_block(relative(target_pos, direction)) else {
                    return BlockPlacementState::Unsupported;
                };
                let Some(is_pale_oak_log) =
                    world.placement_block_is_in_tag(&neighbour, "minecraft:pale_oak_logs")
                else {
                    return BlockPlacementState::Unsupported;
                };
                if !is_pale_oak_log {
                    has_required_logs = false;
                    break;
                }
                let Some(neighbour_axis) =
                    neighbour.block_properties.get("axis").map(String::as_str)
                else {
                    return BlockPlacementState::Unsupported;
                };
                if neighbour_axis != axis_name {
                    has_required_logs = false;
                    break;
                }
            }
            let creaking_active = if has_required_logs {
                let position = [
                    f64::from(target_pos.x) + 0.5,
                    f64::from(target_pos.y) + 0.5,
                    f64::from(target_pos.z) + 0.5,
                ];
                let Ok(attributes) = world.sampled_client_environment_attributes_at(
                    position,
                    EnvironmentBiomeSampling::Direct,
                ) else {
                    return BlockPlacementState::Unsupported;
                };
                attributes.creaking_active.unwrap_or(false)
            } else {
                false
            };
            let heart_state = creaking_heart_state_name(has_required_logs, creaking_active);
            return state_with_properties(
                world,
                block_type,
                default_state_id,
                [("axis", axis_name), ("creaking_heart_state", heart_state)],
            );
        }
        VanillaGetStateForPlacementOverride::ChorusPlantBlock => {
            let Some(placement) = chorus_plant_placement(world, target_pos, block_type) else {
                return BlockPlacementState::Unsupported;
            };
            return state_with_properties(
                world,
                block_type,
                default_state_id,
                [
                    ("down", boolean_name(placement.down)),
                    ("up", boolean_name(placement.up)),
                    ("north", boolean_name(placement.north)),
                    ("east", boolean_name(placement.east)),
                    ("south", boolean_name(placement.south)),
                    ("west", boolean_name(placement.west)),
                ],
            );
        }
        VanillaGetStateForPlacementOverride::FenceBlock
        | VanillaGetStateForPlacementOverride::IronBarsBlock => {
            let Some(waterlogged) = target_is_source_water(world, target_pos) else {
                return BlockPlacementState::Unsupported;
            };
            let connections = match owner {
                VanillaGetStateForPlacementOverride::FenceBlock => {
                    fence_connections(world, target_pos, block_type)
                }
                VanillaGetStateForPlacementOverride::IronBarsBlock => {
                    iron_bars_connections(world, target_pos)
                }
                _ => unreachable!("matched cardinal connection owner"),
            };
            let Some(connections) = connections else {
                return BlockPlacementState::Unsupported;
            };
            return state_with_properties(
                world,
                block_type,
                default_state_id,
                [
                    ("north", boolean_name(connections.north)),
                    ("east", boolean_name(connections.east)),
                    ("south", boolean_name(connections.south)),
                    ("west", boolean_name(connections.west)),
                    ("waterlogged", boolean_name(waterlogged)),
                ],
            );
        }
        VanillaGetStateForPlacementOverride::FenceGateBlock => {
            let Some(placement) = fence_gate_placement(world, target_pos, horizontal) else {
                return BlockPlacementState::Unsupported;
            };
            return state_with_properties(
                world,
                block_type,
                default_state_id,
                [
                    ("facing", direction_name(horizontal)),
                    ("open", "false"),
                    ("powered", "false"),
                    ("in_wall", boolean_name(placement.in_wall)),
                ],
            );
        }
        VanillaGetStateForPlacementOverride::WallBlock => {
            let Some(waterlogged) = target_is_source_water(world, target_pos) else {
                return BlockPlacementState::Unsupported;
            };
            let Some(placement) = wall_placement(world, target_pos) else {
                return BlockPlacementState::Unsupported;
            };
            return state_with_properties(
                world,
                block_type,
                default_state_id,
                [
                    ("up", boolean_name(placement.up)),
                    ("north", placement.north),
                    ("east", placement.east),
                    ("south", placement.south),
                    ("west", placement.west),
                    ("waterlogged", boolean_name(waterlogged)),
                ],
            );
        }
        VanillaGetStateForPlacementOverride::MossyCarpetBlock => {
            let Some(placement) = mossy_carpet_placement(world, target_pos) else {
                return BlockPlacementState::Unsupported;
            };
            return state_with_properties(
                world,
                block_type,
                default_state_id,
                [
                    ("bottom", "true"),
                    ("north", placement.north),
                    ("east", placement.east),
                    ("south", placement.south),
                    ("west", placement.west),
                ],
            );
        }
        VanillaGetStateForPlacementOverride::NoteBlock => {
            let Some(instrument) = world.note_block_instrument_for_placement(target_pos) else {
                return BlockPlacementState::Unsupported;
            };
            return state_with_properties(
                world,
                block_type,
                default_state_id,
                [("instrument", instrument)],
            );
        }
        VanillaGetStateForPlacementOverride::RedStoneWireBlock => {
            let initial = BTreeMap::from([
                ("north".to_string(), "none".to_string()),
                ("east".to_string(), "none".to_string()),
                ("south".to_string(), "none".to_string()),
                ("west".to_string(), "none".to_string()),
                ("power".to_string(), "0".to_string()),
            ]);
            // Vanilla passes its non-dot cross state into `getConnectionState`,
            // whose first step rebuilds the missing sides from the default
            // state. The shared evaluator takes that rebuilt state plus the
            // original `wasDot` value.
            let Some(properties) =
                world.local_redstone_wire_connection_state(target_pos, initial, false)
            else {
                return BlockPlacementState::Unsupported;
            };
            return resolve_state(world, block_type.resource_id, properties);
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
        VanillaGetStateForPlacementOverride::MangrovePropaguleBlock => {
            let Some(waterlogged) = target_is_source_water(world, target_pos) else {
                return BlockPlacementState::Unsupported;
            };
            return state_with_properties(
                world,
                block_type,
                default_state_id,
                [
                    ("age", "4"),
                    ("hanging", "false"),
                    ("waterlogged", boolean_name(waterlogged)),
                ],
            );
        }
        VanillaGetStateForPlacementOverride::PointedDripstoneBlock => {
            let Some(placement) = pointed_dripstone_placement(
                world,
                target_pos,
                pose.x_rot,
                context.secondary_use_active,
            ) else {
                return BlockPlacementState::Unsupported;
            };
            let Some(placement) = placement else {
                return BlockPlacementState::Fail;
            };
            let Some(waterlogged) = target_is_source_water(world, target_pos) else {
                return BlockPlacementState::Unsupported;
            };
            return state_with_properties(
                world,
                block_type,
                default_state_id,
                [
                    (
                        "vertical_direction",
                        direction_name(placement.tip_direction),
                    ),
                    ("thickness", placement.thickness),
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
        VanillaGetStateForPlacementOverride::StairBlock => {
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
            let Some(shape) = stair_shape(world, target_pos, horizontal, half) else {
                return BlockPlacementState::Unsupported;
            };
            return state_with_properties(
                world,
                block_type,
                default_state_id,
                [
                    ("facing", direction_name(horizontal)),
                    ("half", half),
                    ("shape", shape),
                    ("waterlogged", boolean_name(waterlogged)),
                ],
            );
        }
        VanillaGetStateForPlacementOverride::TrapDoorBlock => {
            let Some(placement) =
                trapdoor_placement(world, target_pos, replace_clicked, horizontal, context)
            else {
                return BlockPlacementState::Unsupported;
            };
            return state_with_properties(
                world,
                block_type,
                default_state_id,
                [
                    ("facing", direction_name(placement.facing)),
                    ("half", placement.half),
                    ("open", "false"),
                    ("powered", "false"),
                    ("waterlogged", boolean_name(placement.waterlogged)),
                ],
            );
        }
        owner @ (VanillaGetStateForPlacementOverride::CeilingHangingSignBlock
        | VanillaGetStateForPlacementOverride::FaceAttachedHorizontalDirectionalBlock
        | VanillaGetStateForPlacementOverride::LadderBlock
        | VanillaGetStateForPlacementOverride::LanternBlock
        | VanillaGetStateForPlacementOverride::WallBannerBlock
        | VanillaGetStateForPlacementOverride::WallSignBlock
        | VanillaGetStateForPlacementOverride::WallTorchBlock
        | VanillaGetStateForPlacementOverride::RedstoneWallTorchBlock
        | VanillaGetStateForPlacementOverride::WallHangingSignBlock
        | VanillaGetStateForPlacementOverride::WallSkullBlock) => {
            return attached_block_state_for_placement(
                world,
                block_type,
                owner,
                target_pos,
                replace_clicked,
                context,
                horizontal,
                directions,
                default_state_id,
            );
        }
        _ => return BlockPlacementState::Unsupported,
    };

    state_with_properties(world, block_type, default_state_id, properties)
}

pub(super) const fn creaking_heart_state_name(
    has_required_logs: bool,
    creaking_active: bool,
) -> &'static str {
    if !has_required_logs {
        "uprooted"
    } else if creaking_active {
        "awake"
    } else {
        "dormant"
    }
}

pub(super) fn state_with_properties<const N: usize>(
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

pub(super) fn state_with_owned_properties<const N: usize>(
    world: &WorldStore,
    block_type: &BlockTypeCatalogEntry,
    default_state_id: i32,
    updates: [(&str, String); N],
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
        properties.insert(name.to_string(), value);
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

fn direction_property(properties: &BTreeMap<String, String>, property: &str) -> Option<Direction> {
    match properties.get(property)?.as_str() {
        "down" => Some(Direction::Down),
        "up" => Some(Direction::Up),
        "north" => Some(Direction::North),
        "south" => Some(Direction::South),
        "west" => Some(Direction::West),
        "east" => Some(Direction::East),
        _ => None,
    }
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
