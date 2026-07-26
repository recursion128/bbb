use bbb_protocol::{
    block_state_types::VanillaBlockSupportType,
    block_types::{
        vanilla_block_type, BlockTypeCatalogEntry, VanillaCanSurviveOverride,
        VanillaGetStateForPlacementOverride,
    },
    item_types::VanillaBlockItemBehavior,
    packets::Direction,
};

use super::{
    super::{LocalBlockUseContext, WorldStore},
    context::{
        clockwise, counterclockwise, direction_axis, direction_name, opposite,
        placement_directions, relative, rotation_segment, world_click_y, DirectionAxis,
    },
    signal::placement_neighbor_signal_is_proven_absent,
    state::{
        block_state_for_placement, state_with_owned_properties, state_with_properties,
        target_is_full_water, BlockPlacementState,
    },
};
use crate::{BlockPos, BlockProbe, TerrainFluidKind};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct BellPlacement {
    pub(super) facing: Direction,
    pub(super) attachment: &'static str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct CeilingHangingSignPlacement {
    attached: bool,
    rotation: i32,
    waterlogged: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct PointedDripstonePlacement {
    pub(super) tip_direction: Direction,
    pub(super) thickness: &'static str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct TrapDoorPlacement {
    pub(super) facing: Direction,
    pub(super) half: &'static str,
    pub(super) waterlogged: bool,
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
        block_state_for_placement(world, primary, target_pos, replace_clicked, context, None);
    let wall_state =
        block_state_for_placement(world, wall, target_pos, replace_clicked, context, None);
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

pub(super) fn attached_block_state_for_placement(
    world: &WorldStore,
    block_type: &BlockTypeCatalogEntry,
    owner: VanillaGetStateForPlacementOverride,
    target_pos: BlockPos,
    replace_clicked: bool,
    context: LocalBlockUseContext,
    horizontal: Direction,
    directions: [Direction; 6],
    default_state_id: i32,
) -> BlockPlacementState {
    match owner {
        VanillaGetStateForPlacementOverride::CeilingHangingSignBlock => {
            let Some(placement) =
                ceiling_hanging_sign_placement(world, target_pos, horizontal, context)
            else {
                return BlockPlacementState::Unsupported;
            };
            state_with_owned_properties(
                world,
                block_type,
                default_state_id,
                [
                    ("attached", boolean_name(placement.attached).to_string()),
                    ("rotation", placement.rotation.to_string()),
                    (
                        "waterlogged",
                        boolean_name(placement.waterlogged).to_string(),
                    ),
                ],
            )
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
                    VanillaCanSurviveOverride::FaceAttachedHorizontalDirectionalBlock,
                ) {
                    Some(true) => return BlockPlacementState::State(state_id),
                    Some(false) => {}
                    None => return BlockPlacementState::Unsupported,
                }
            }
            BlockPlacementState::Fail
        }
        VanillaGetStateForPlacementOverride::LadderBlock => {
            if !replace_clicked {
                let Some(behind) = world.probe_block(relative(target_pos, opposite(context.face)))
                else {
                    return BlockPlacementState::Unsupported;
                };
                if behind.block_name.as_deref() == Some(block_type.resource_id)
                    && direction_property(&behind, "facing") == Some(context.face)
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
                        VanillaCanSurviveOverride::LadderBlock,
                    ) {
                        Some(true) => return BlockPlacementState::State(state_id),
                        Some(false) => {}
                        None => return BlockPlacementState::Unsupported,
                    }
                }
            }
            BlockPlacementState::Fail
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
                        VanillaCanSurviveOverride::LanternBlock,
                    ) {
                        Some(true) => return BlockPlacementState::State(state_id),
                        Some(false) => {}
                        None => return BlockPlacementState::Unsupported,
                    }
                }
            }
            BlockPlacementState::Fail
        }
        VanillaGetStateForPlacementOverride::BaseCoralWallFanBlock
        | VanillaGetStateForPlacementOverride::WallBannerBlock
        | VanillaGetStateForPlacementOverride::WallSignBlock
        | VanillaGetStateForPlacementOverride::WallTorchBlock
        | VanillaGetStateForPlacementOverride::RedstoneWallTorchBlock => {
            let waterlogged = match owner {
                VanillaGetStateForPlacementOverride::BaseCoralWallFanBlock => {
                    let Some(waterlogged) = target_is_full_water(world, target_pos) else {
                        return BlockPlacementState::Unsupported;
                    };
                    Some(waterlogged)
                }
                VanillaGetStateForPlacementOverride::WallSignBlock => {
                    let Some(waterlogged) = target_is_source_water(world, target_pos) else {
                        return BlockPlacementState::Unsupported;
                    };
                    Some(waterlogged)
                }
                _ => None,
            };
            let survival_owner = match owner {
                VanillaGetStateForPlacementOverride::BaseCoralWallFanBlock => {
                    VanillaCanSurviveOverride::BaseCoralWallFanBlock
                }
                VanillaGetStateForPlacementOverride::WallBannerBlock => {
                    VanillaCanSurviveOverride::WallBannerBlock
                }
                VanillaGetStateForPlacementOverride::WallSignBlock => {
                    VanillaCanSurviveOverride::WallSignBlock
                }
                VanillaGetStateForPlacementOverride::WallTorchBlock => {
                    VanillaCanSurviveOverride::WallTorchBlock
                }
                VanillaGetStateForPlacementOverride::RedstoneWallTorchBlock => {
                    VanillaCanSurviveOverride::RedstoneWallTorchBlock
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
            BlockPlacementState::Fail
        }
        VanillaGetStateForPlacementOverride::WallHangingSignBlock => {
            let Some(facing) =
                wall_hanging_sign_facing(world, target_pos, context.face, directions)
            else {
                return BlockPlacementState::Unsupported;
            };
            let Some(facing) = facing else {
                return BlockPlacementState::Fail;
            };
            let Some(waterlogged) = target_is_source_water(world, target_pos) else {
                return BlockPlacementState::Unsupported;
            };
            state_with_properties(
                world,
                block_type,
                default_state_id,
                [
                    ("facing", direction_name(facing)),
                    ("waterlogged", boolean_name(waterlogged)),
                ],
            )
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
            BlockPlacementState::Fail
        }
        _ => BlockPlacementState::Unsupported,
    }
}

fn ceiling_hanging_sign_placement(
    world: &WorldStore,
    target_pos: BlockPos,
    horizontal: Direction,
    context: LocalBlockUseContext,
) -> Option<CeilingHangingSignPlacement> {
    let above = world.probe_block(relative(target_pos, Direction::Up))?;
    let is_below_hanging_sign =
        world.placement_block_is_in_tag(&above, "minecraft:all_hanging_signs")?;
    let mut attached = !world.placement_collision_face_is_full(&above, Direction::Down)?
        || context.secondary_use_active;
    if is_below_hanging_sign && !context.secondary_use_active {
        if above.block_properties.contains_key("facing") {
            let above_facing = direction_property(&above, "facing")?;
            if direction_axis(above_facing) == direction_axis(horizontal) {
                attached = false;
            }
        } else if let Some(rotation) = above.block_properties.get("rotation") {
            let rotation = rotation.parse::<i32>().ok()?;
            if cardinal_rotation_axis(rotation) == Some(direction_axis(horizontal)) {
                attached = false;
            }
        }
    }
    let rotation = if attached {
        rotation_segment(world.local_player().pose?.y_rot + 180.0)?
    } else {
        direction_rotation_segment(opposite(horizontal))?
    };
    Some(CeilingHangingSignPlacement {
        attached,
        rotation,
        waterlogged: target_is_source_water(world, target_pos)?,
    })
}

fn wall_hanging_sign_facing(
    world: &WorldStore,
    target_pos: BlockPos,
    clicked_face: Direction,
    directions: [Direction; 6],
) -> Option<Option<Direction>> {
    for direction in directions {
        if direction_axis(direction) == DirectionAxis::Y
            || direction_axis(direction) == direction_axis(clicked_face)
        {
            continue;
        }
        let facing = opposite(direction);
        if wall_hanging_sign_can_place(world, target_pos, facing)? {
            return Some(Some(facing));
        }
    }
    Some(None)
}

fn wall_hanging_sign_can_place(
    world: &WorldStore,
    target_pos: BlockPos,
    facing: Direction,
) -> Option<bool> {
    let clockwise = clockwise(facing)?;
    let counterclockwise = counterclockwise(facing)?;
    if wall_hanging_sign_can_attach(world, target_pos, facing, clockwise)? {
        Some(true)
    } else {
        wall_hanging_sign_can_attach(world, target_pos, facing, counterclockwise)
    }
}

fn wall_hanging_sign_can_attach(
    world: &WorldStore,
    target_pos: BlockPos,
    facing: Direction,
    support_direction: Direction,
) -> Option<bool> {
    let support = world.probe_block(relative(target_pos, support_direction))?;
    if world.placement_block_is_in_tag(&support, "minecraft:wall_hanging_signs")? {
        let support_facing = direction_property(&support, "facing")?;
        return Some(direction_axis(support_facing) == direction_axis(facing));
    }
    world.placement_face_sturdy(
        &support,
        opposite(support_direction),
        VanillaBlockSupportType::Full,
    )
}

fn direction_rotation_segment(direction: Direction) -> Option<i32> {
    Some(match direction {
        Direction::South => 0,
        Direction::West => 4,
        Direction::North => 8,
        Direction::East => 12,
        Direction::Down | Direction::Up => return None,
    })
}

fn cardinal_rotation_axis(rotation: i32) -> Option<DirectionAxis> {
    match rotation {
        0 | 8 => Some(DirectionAxis::Z),
        4 | 12 => Some(DirectionAxis::X),
        _ => None,
    }
}

pub(super) fn bell_placement(
    world: &WorldStore,
    target_pos: BlockPos,
    horizontal: Direction,
    clicked_face: Direction,
) -> Option<Option<BellPlacement>> {
    if direction_axis(clicked_face) == DirectionAxis::Y {
        let attachment = if clicked_face == Direction::Down {
            "ceiling"
        } else {
            "floor"
        };
        return bell_attachment_can_survive(world, target_pos, horizontal, attachment).map(
            |survives| {
                survives.then_some(BellPlacement {
                    facing: horizontal,
                    attachment,
                })
            },
        );
    }

    let double_attached = match direction_axis(clicked_face) {
        DirectionAxis::X => {
            full_face_support(world, target_pos, Direction::West)?
                && full_face_support(world, target_pos, Direction::East)?
        }
        DirectionAxis::Z => {
            full_face_support(world, target_pos, Direction::North)?
                && full_face_support(world, target_pos, Direction::South)?
        }
        DirectionAxis::Y => unreachable!("vertical bell placement returned above"),
    };
    let facing = opposite(clicked_face);
    let wall_attachment = if double_attached {
        "double_wall"
    } else {
        "single_wall"
    };
    if bell_attachment_can_survive(world, target_pos, facing, wall_attachment)? {
        return Some(Some(BellPlacement {
            facing,
            attachment: wall_attachment,
        }));
    }

    let attachment = if full_face_support(world, target_pos, Direction::Down)? {
        "floor"
    } else {
        "ceiling"
    };
    bell_attachment_can_survive(world, target_pos, facing, attachment)
        .map(|survives| survives.then_some(BellPlacement { facing, attachment }))
}

pub(super) fn cocoa_facing(
    world: &WorldStore,
    target_pos: BlockPos,
    directions: [Direction; 6],
) -> Option<Option<Direction>> {
    for direction in directions {
        if direction_axis(direction) == DirectionAxis::Y {
            continue;
        }
        let support = world.probe_block(relative(target_pos, direction))?;
        if world.placement_block_is_in_tag(&support, "minecraft:supports_cocoa")? {
            return Some(Some(direction));
        }
    }
    Some(None)
}

pub(super) fn pointed_dripstone_placement(
    world: &WorldStore,
    target_pos: BlockPos,
    player_x_rot: f32,
    secondary_use_active: bool,
) -> Option<Option<PointedDripstonePlacement>> {
    let default_tip_direction = if player_x_rot < 0.0 {
        Direction::Down
    } else {
        Direction::Up
    };
    let tip_direction =
        if valid_pointed_dripstone_placement(world, target_pos, default_tip_direction)? {
            default_tip_direction
        } else {
            let opposite_tip = opposite(default_tip_direction);
            if !valid_pointed_dripstone_placement(world, target_pos, opposite_tip)? {
                return Some(None);
            }
            opposite_tip
        };
    let base_direction = opposite(tip_direction);
    let in_front = world.probe_block(relative(target_pos, tip_direction))?;
    let thickness = if pointed_dripstone_has_direction(&in_front, base_direction)? {
        let in_front_thickness = dripstone_thickness(&in_front)?;
        if secondary_use_active && in_front_thickness != "tip_merge" {
            "tip"
        } else {
            "tip_merge"
        }
    } else if !pointed_dripstone_has_direction(&in_front, tip_direction)? {
        "tip"
    } else {
        let in_front_thickness = dripstone_thickness(&in_front)?;
        if matches!(in_front_thickness, "tip" | "tip_merge") {
            "frustum"
        } else {
            let behind = world.probe_block(relative(target_pos, base_direction))?;
            if pointed_dripstone_has_direction(&behind, tip_direction)? {
                "middle"
            } else {
                "base"
            }
        }
    };
    Some(Some(PointedDripstonePlacement {
        tip_direction,
        thickness,
    }))
}

pub(super) fn trapdoor_placement(
    world: &WorldStore,
    target_pos: BlockPos,
    replace_clicked: bool,
    horizontal: Direction,
    context: LocalBlockUseContext,
) -> Option<TrapDoorPlacement> {
    placement_neighbor_signal_is_proven_absent(world, &[target_pos])?;
    let waterlogged = target_is_source_water(world, target_pos)?;
    let (facing, half) = if !replace_clicked && direction_axis(context.face) != DirectionAxis::Y {
        (
            context.face,
            if world_click_y(context) - f64::from(target_pos.y) > 0.5 {
                "top"
            } else {
                "bottom"
            },
        )
    } else {
        (
            opposite(horizontal),
            if context.face == Direction::Up {
                "bottom"
            } else {
                "top"
            },
        )
    };
    Some(TrapDoorPlacement {
        facing,
        half,
        waterlogged,
    })
}

fn bell_attachment_can_survive(
    world: &WorldStore,
    target_pos: BlockPos,
    facing: Direction,
    attachment: &str,
) -> Option<bool> {
    match attachment {
        "floor" => full_face_support(world, target_pos, Direction::Down),
        "ceiling" => stable_center_support_above(world, target_pos),
        "single_wall" | "double_wall" => full_face_support(world, target_pos, facing),
        _ => None,
    }
}

fn full_face_support(
    world: &WorldStore,
    target_pos: BlockPos,
    support_direction: Direction,
) -> Option<bool> {
    let support = world.probe_block(relative(target_pos, support_direction))?;
    world.placement_face_sturdy(
        &support,
        opposite(support_direction),
        VanillaBlockSupportType::Full,
    )
}

fn stable_center_support_above(world: &WorldStore, target_pos: BlockPos) -> Option<bool> {
    let above = world.probe_block(relative(target_pos, Direction::Up))?;
    if world.placement_block_is_in_tag(&above, "minecraft:unstable_bottom_center")? {
        return Some(false);
    }
    world.placement_face_sturdy(&above, Direction::Down, VanillaBlockSupportType::Center)
}

fn valid_pointed_dripstone_placement(
    world: &WorldStore,
    target_pos: BlockPos,
    tip_direction: Direction,
) -> Option<bool> {
    let behind = world.probe_block(relative(target_pos, opposite(tip_direction)))?;
    if pointed_dripstone_has_direction(&behind, tip_direction)? {
        return Some(true);
    }
    world.placement_face_sturdy(&behind, tip_direction, VanillaBlockSupportType::Full)
}

fn pointed_dripstone_has_direction(block: &BlockProbe, direction: Direction) -> Option<bool> {
    if block.block_name.as_deref() != Some("minecraft:pointed_dripstone") {
        return Some(false);
    }
    direction_property(block, "vertical_direction").map(|value| value == direction)
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

fn dripstone_thickness(block: &BlockProbe) -> Option<&str> {
    match block.block_properties.get("thickness")?.as_str() {
        value @ ("tip" | "tip_merge" | "frustum" | "middle" | "base") => Some(value),
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

fn boolean_name(value: bool) -> &'static str {
    if value {
        "true"
    } else {
        "false"
    }
}
