use std::collections::BTreeMap;

use bbb_protocol::{
    block_state_types::vanilla_block_default_state_id,
    block_types::{vanilla_block_type, BlockTypeCatalogEntry, VanillaGetStateForPlacementOverride},
    item_types::VanillaBlockItemBehavior,
    packets::Direction,
};

use super::super::{LocalBlockUseContext, WorldStore};
use super::context::{
    clockwise, counterclockwise, direction_axis, direction_axis_name, direction_name,
    front_and_top_name, horizontal_direction, nearest_directions, opposite, placement_directions,
    relative, rotation_segment, world_click_y, DirectionAxis,
};
use crate::{BlockPos, TerrainFluidKind};

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
        VanillaGetStateForPlacementOverride::FaceAttachedHorizontalDirectionalBlock => {
            for direction in directions {
                let (face, facing) = if direction_axis(direction) == DirectionAxis::Y {
                    (
                        if direction == Direction::Up {
                            "ceiling"
                        } else {
                            "floor"
                        },
                        horizontal,
                    )
                } else {
                    ("wall", opposite(direction))
                };
                let candidate = state_with_properties(
                    world,
                    block_type,
                    default_state_id,
                    [("face", face), ("facing", direction_name(facing))],
                );
                let BlockPlacementState::State(state_id) = candidate else {
                    return candidate;
                };
                match world.local_block_state_can_survive(
                    target_pos,
                    state_id,
                    bbb_protocol::block_types::VanillaCanSurviveOverride::FaceAttachedHorizontalDirectionalBlock,
                ) {
                    Some(true) => return BlockPlacementState::State(state_id),
                    Some(false) => {}
                    None => return BlockPlacementState::Unsupported,
                }
            }
            return BlockPlacementState::Fail;
        }
        VanillaGetStateForPlacementOverride::LadderBlock => {
            if !replace_clicked {
                let Some(behind) = world.probe_block(relative(target_pos, opposite(context.face)))
                else {
                    return BlockPlacementState::Unsupported;
                };
                if behind.block_name.as_deref() == Some(block_type.resource_id)
                    && direction_property(&behind.block_properties, "facing") == Some(context.face)
                {
                    return BlockPlacementState::Fail;
                }
            }
            let Some(waterlogged) = target_is_source_water(world, target_pos) else {
                return BlockPlacementState::Unsupported;
            };
            for direction in directions {
                if direction_axis(direction) != DirectionAxis::Y {
                    let candidate = state_with_properties(
                        world,
                        block_type,
                        default_state_id,
                        [
                            ("facing", direction_name(opposite(direction))),
                            ("waterlogged", boolean_name(waterlogged)),
                        ],
                    );
                    let BlockPlacementState::State(state_id) = candidate else {
                        return candidate;
                    };
                    match world.local_block_state_can_survive(
                        target_pos,
                        state_id,
                        bbb_protocol::block_types::VanillaCanSurviveOverride::LadderBlock,
                    ) {
                        Some(true) => return BlockPlacementState::State(state_id),
                        Some(false) => {}
                        None => return BlockPlacementState::Unsupported,
                    }
                }
            }
            return BlockPlacementState::Fail;
        }
        VanillaGetStateForPlacementOverride::LanternBlock => {
            let Some(waterlogged) = target_is_source_water(world, target_pos) else {
                return BlockPlacementState::Unsupported;
            };
            for direction in directions {
                if direction_axis(direction) == DirectionAxis::Y {
                    let candidate = state_with_properties(
                        world,
                        block_type,
                        default_state_id,
                        [
                            ("hanging", boolean_name(direction == Direction::Up)),
                            ("waterlogged", boolean_name(waterlogged)),
                        ],
                    );
                    let BlockPlacementState::State(state_id) = candidate else {
                        return candidate;
                    };
                    match world.local_block_state_can_survive(
                        target_pos,
                        state_id,
                        bbb_protocol::block_types::VanillaCanSurviveOverride::LanternBlock,
                    ) {
                        Some(true) => return BlockPlacementState::State(state_id),
                        Some(false) => {}
                        None => return BlockPlacementState::Unsupported,
                    }
                }
            }
            return BlockPlacementState::Fail;
        }
        VanillaGetStateForPlacementOverride::WallBannerBlock
        | VanillaGetStateForPlacementOverride::WallSignBlock
        | VanillaGetStateForPlacementOverride::WallTorchBlock
        | VanillaGetStateForPlacementOverride::RedstoneWallTorchBlock => {
            let waterlogged = if owner == VanillaGetStateForPlacementOverride::WallSignBlock {
                let Some(waterlogged) = target_is_source_water(world, target_pos) else {
                    return BlockPlacementState::Unsupported;
                };
                Some(waterlogged)
            } else {
                None
            };
            let survival_owner = match owner {
                VanillaGetStateForPlacementOverride::WallBannerBlock => {
                    bbb_protocol::block_types::VanillaCanSurviveOverride::WallBannerBlock
                }
                VanillaGetStateForPlacementOverride::WallSignBlock => {
                    bbb_protocol::block_types::VanillaCanSurviveOverride::WallSignBlock
                }
                VanillaGetStateForPlacementOverride::WallTorchBlock => {
                    bbb_protocol::block_types::VanillaCanSurviveOverride::WallTorchBlock
                }
                VanillaGetStateForPlacementOverride::RedstoneWallTorchBlock => {
                    bbb_protocol::block_types::VanillaCanSurviveOverride::RedstoneWallTorchBlock
                }
                _ => unreachable!("matched wall support owner"),
            };
            for direction in directions {
                if direction_axis(direction) != DirectionAxis::Y {
                    let facing = direction_name(opposite(direction)).to_string();
                    let candidate = if let Some(waterlogged) = waterlogged {
                        state_with_owned_properties(
                            world,
                            block_type,
                            default_state_id,
                            [
                                ("facing", facing),
                                ("waterlogged", boolean_name(waterlogged).to_string()),
                            ],
                        )
                    } else {
                        state_with_owned_properties(
                            world,
                            block_type,
                            default_state_id,
                            [("facing", facing)],
                        )
                    };
                    let BlockPlacementState::State(state_id) = candidate else {
                        return candidate;
                    };
                    match world.local_block_state_can_survive(target_pos, state_id, survival_owner)
                    {
                        Some(true) => return BlockPlacementState::State(state_id),
                        Some(false) => {}
                        None => return BlockPlacementState::Unsupported,
                    }
                }
            }
            return BlockPlacementState::Fail;
        }
        VanillaGetStateForPlacementOverride::WallSkullBlock => {
            for direction in directions {
                if direction_axis(direction) != DirectionAxis::Y {
                    let support_pos = relative(target_pos, direction);
                    let Some(replaceable) =
                        world.local_block_is_replaceable_by_different_item(support_pos)
                    else {
                        return BlockPlacementState::Unsupported;
                    };
                    if !replaceable {
                        return state_with_properties(
                            world,
                            block_type,
                            default_state_id,
                            [("facing", direction_name(opposite(direction)))],
                        );
                    }
                }
            }
            return BlockPlacementState::Fail;
        }
        _ => return BlockPlacementState::Unsupported,
    };

    state_with_properties(world, block_type, default_state_id, properties)
}

pub(super) fn standing_and_wall_state_for_placement(
    world: &WorldStore,
    behavior: VanillaBlockItemBehavior,
    target_pos: BlockPos,
    replace_clicked: bool,
    context: LocalBlockUseContext,
    attachment_direction: Direction,
) -> BlockPlacementState {
    let Some(primary) = vanilla_block_type(behavior.block_type_id) else {
        return BlockPlacementState::Unsupported;
    };
    let Some(wall) = behavior.wall_block_type_id.and_then(vanilla_block_type) else {
        return BlockPlacementState::Unsupported;
    };
    let primary_state =
        block_state_for_placement(world, primary, target_pos, replace_clicked, context);
    let wall_state = block_state_for_placement(world, wall, target_pos, replace_clicked, context);
    let Some(pose) = world.local_player().pose else {
        return BlockPlacementState::Unsupported;
    };
    let Some(directions) = placement_directions(pose.x_rot, pose.y_rot, context, replace_clicked)
    else {
        return BlockPlacementState::Unsupported;
    };

    for direction in directions {
        if direction == opposite(attachment_direction) {
            continue;
        }
        let (candidate, block_type) = if direction == attachment_direction {
            (primary_state, primary)
        } else {
            (wall_state, wall)
        };
        let state_id = match candidate {
            BlockPlacementState::State(state_id) => state_id,
            BlockPlacementState::Fail => continue,
            BlockPlacementState::Unsupported => return BlockPlacementState::Unsupported,
        };
        match block_type.can_survive_override {
            None => return BlockPlacementState::State(state_id),
            Some(owner) => match world.local_block_state_can_survive(target_pos, state_id, owner) {
                Some(true) => return BlockPlacementState::State(state_id),
                Some(false) => {}
                None => return BlockPlacementState::Unsupported,
            },
        }
    }
    BlockPlacementState::Fail
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

fn state_with_owned_properties<const N: usize>(
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

fn stair_shape(
    world: &WorldStore,
    pos: BlockPos,
    facing: Direction,
    half: &str,
) -> Option<&'static str> {
    let behind = world.probe_block(relative(pos, facing))?;
    if block_is_stair(&behind)
        && behind.block_properties.get("half").map(String::as_str) == Some(half)
    {
        let behind_facing = direction_property(&behind.block_properties, "facing")?;
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
        let front_facing = direction_property(&front.block_properties, "facing")?;
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
        direction_property(&neighbour.block_properties, "facing")? != facing
            || neighbour.block_properties.get("half").map(String::as_str) != Some(half),
    )
}

fn block_is_stair(block: &crate::BlockProbe) -> bool {
    block
        .block_name
        .as_deref()
        .and_then(bbb_protocol::block_types::vanilla_block_type_for_resource_id)
        .is_some_and(|block_type| {
            block_type.get_state_for_placement_override
                == Some(VanillaGetStateForPlacementOverride::StairBlock)
        })
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
