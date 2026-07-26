use std::collections::BTreeMap;

use bbb_protocol::{
    block_state_types::{
        vanilla_block_state_support_facts, VanillaBlockSupportDirection, VanillaBlockSupportType,
    },
    block_types::{
        vanilla_block_type_for_resource_id, VanillaCanSurviveOverride,
        VANILLA_BLOCK_BIG_DRIPLEAF_ID, VANILLA_BLOCK_BIG_DRIPLEAF_STEM_ID,
        VANILLA_BLOCK_POINTED_DRIPSTONE_ID, VANILLA_BLOCK_SCAFFOLDING_ID,
        VANILLA_BLOCK_SMALL_DRIPLEAF_ID,
    },
    packets::Direction,
};

use super::super::WorldStore;
use super::context::{opposite, relative};
use crate::{BlockPos, BlockProbe};

#[derive(Debug, Clone, Copy)]
enum BlockCanSurviveRule {
    FacingFull,
    BelowFull,
    BelowRigid,
    BigDripleaf,
    BigDripleafStem,
    BelowCenter,
    AboveCenter,
    AboveFull,
    Lantern,
    MangrovePropagule,
    Multiface,
    PointedDripstone,
    Scaffolding,
    SeaPickle,
    SmallDripleaf,
    BelowLegacySolid,
    FacingLegacySolid,
    FaceAttached,
    BelowTag(&'static str),
}

impl WorldStore {
    pub(in super::super) fn local_block_state_can_survive(
        &self,
        pos: BlockPos,
        block_state_id: i32,
        owner: VanillaCanSurviveOverride,
    ) -> Option<bool> {
        let state = self.registries().block_state(block_state_id)?;
        let rule = block_can_survive_rule(owner)?;
        self.evaluate_block_can_survive(pos, &state.properties, rule)
    }

    fn evaluate_block_can_survive(
        &self,
        pos: BlockPos,
        properties: &BTreeMap<String, String>,
        rule: BlockCanSurviveRule,
    ) -> Option<bool> {
        match rule {
            BlockCanSurviveRule::FacingFull => {
                let facing = direction_property(properties, "facing")?;
                let support = self.probe_block(relative(pos, opposite(facing)))?;
                self.placement_face_sturdy(&support, facing, VanillaBlockSupportType::Full)
            }
            BlockCanSurviveRule::BelowFull => {
                let below = self.probe_block(relative(pos, Direction::Down))?;
                self.placement_face_sturdy(&below, Direction::Up, VanillaBlockSupportType::Full)
            }
            BlockCanSurviveRule::BelowRigid => {
                let below = self.probe_block(relative(pos, Direction::Down))?;
                self.placement_face_sturdy(&below, Direction::Up, VanillaBlockSupportType::Rigid)
            }
            BlockCanSurviveRule::BigDripleaf => {
                let below = self.probe_block(relative(pos, Direction::Down))?;
                let below_type = block_type_id(&below)?;
                if matches!(
                    below_type,
                    VANILLA_BLOCK_BIG_DRIPLEAF_ID | VANILLA_BLOCK_BIG_DRIPLEAF_STEM_ID
                ) {
                    Some(true)
                } else {
                    self.placement_block_is_in_tag(&below, "minecraft:supports_big_dripleaf")
                }
            }
            BlockCanSurviveRule::BigDripleafStem => {
                let below = self.probe_block(relative(pos, Direction::Down))?;
                let below_type = block_type_id(&below)?;
                let supported_below = if below_type == VANILLA_BLOCK_BIG_DRIPLEAF_STEM_ID {
                    true
                } else {
                    self.placement_block_is_in_tag(&below, "minecraft:supports_big_dripleaf")?
                };
                if !supported_below {
                    return Some(false);
                }
                let above = self.probe_block(relative(pos, Direction::Up))?;
                Some(matches!(
                    block_type_id(&above)?,
                    VANILLA_BLOCK_BIG_DRIPLEAF_ID | VANILLA_BLOCK_BIG_DRIPLEAF_STEM_ID
                ))
            }
            BlockCanSurviveRule::BelowCenter => {
                let below = self.probe_block(relative(pos, Direction::Down))?;
                self.placement_face_sturdy(&below, Direction::Up, VanillaBlockSupportType::Center)
            }
            BlockCanSurviveRule::AboveCenter => {
                let above = self.probe_block(relative(pos, Direction::Up))?;
                self.placement_face_sturdy(&above, Direction::Down, VanillaBlockSupportType::Center)
            }
            BlockCanSurviveRule::AboveFull => {
                let above = self.probe_block(relative(pos, Direction::Up))?;
                self.placement_face_sturdy(&above, Direction::Down, VanillaBlockSupportType::Full)
            }
            BlockCanSurviveRule::Lantern => {
                let hanging = boolean_property(properties, "hanging")?;
                let (support_direction, sturdy_direction) = if hanging {
                    (Direction::Up, Direction::Down)
                } else {
                    (Direction::Down, Direction::Up)
                };
                let support = self.probe_block(relative(pos, support_direction))?;
                if hanging
                    && self
                        .placement_block_is_in_tag(&support, "minecraft:unstable_bottom_center")?
                {
                    return Some(false);
                }
                self.placement_face_sturdy(
                    &support,
                    sturdy_direction,
                    VanillaBlockSupportType::Center,
                )
            }
            BlockCanSurviveRule::MangrovePropagule => {
                let hanging = boolean_property(properties, "hanging")?;
                let (support_direction, tag) = if hanging {
                    (
                        Direction::Up,
                        "minecraft:supports_hanging_mangrove_propagule",
                    )
                } else {
                    (Direction::Down, "minecraft:supports_mangrove_propagule")
                };
                let support = self.probe_block(relative(pos, support_direction))?;
                self.placement_block_is_in_tag(&support, tag)
            }
            BlockCanSurviveRule::Multiface => {
                let mut has_face = false;
                for (direction, property) in [
                    (Direction::Down, "down"),
                    (Direction::Up, "up"),
                    (Direction::North, "north"),
                    (Direction::South, "south"),
                    (Direction::West, "west"),
                    (Direction::East, "east"),
                ] {
                    let Some(value) = properties.get(property) else {
                        continue;
                    };
                    let face = parse_boolean_property(value)?;
                    if !face {
                        continue;
                    }
                    has_face = true;
                    let support = self.probe_block(relative(pos, direction))?;
                    if !self
                        .placement_full_support_or_collision_face(&support, opposite(direction))?
                    {
                        return Some(false);
                    }
                }
                Some(has_face)
            }
            BlockCanSurviveRule::PointedDripstone => {
                let tip_direction = direction_property(properties, "vertical_direction")?;
                let behind = self.probe_block(relative(pos, opposite(tip_direction)))?;
                if block_type_id(&behind)? == VANILLA_BLOCK_POINTED_DRIPSTONE_ID
                    && direction_property(&behind.block_properties, "vertical_direction")?
                        == tip_direction
                {
                    return Some(true);
                }
                self.placement_face_sturdy(&behind, tip_direction, VanillaBlockSupportType::Full)
            }
            BlockCanSurviveRule::Scaffolding => {
                let below = self.probe_block(relative(pos, Direction::Down))?;
                let mut distance = 7;
                if block_type_id(&below)? == VANILLA_BLOCK_SCAFFOLDING_ID {
                    distance = integer_property(&below.block_properties, "distance")?;
                } else if self.placement_face_sturdy(
                    &below,
                    Direction::Up,
                    VanillaBlockSupportType::Full,
                )? {
                    return Some(true);
                }

                for direction in [
                    Direction::North,
                    Direction::East,
                    Direction::South,
                    Direction::West,
                ] {
                    let neighbour = self.probe_block(relative(pos, direction))?;
                    if block_type_id(&neighbour)? == VANILLA_BLOCK_SCAFFOLDING_ID {
                        distance = distance
                            .min(integer_property(&neighbour.block_properties, "distance")? + 1);
                        if distance == 1 {
                            break;
                        }
                    }
                }
                Some(distance < 7)
            }
            BlockCanSurviveRule::SeaPickle => {
                let below = self.probe_block(relative(pos, Direction::Down))?;
                let collision = self.placement_collision_face_nonempty(&below, Direction::Up)?;
                if collision {
                    Some(true)
                } else {
                    self.placement_face_sturdy(&below, Direction::Up, VanillaBlockSupportType::Full)
                }
            }
            BlockCanSurviveRule::SmallDripleaf => {
                match properties.get("half").map(String::as_str) {
                    Some("upper") => {
                        let below = self.probe_block(relative(pos, Direction::Down))?;
                        Some(
                            block_type_id(&below)? == VANILLA_BLOCK_SMALL_DRIPLEAF_ID
                                && below
                                    .block_properties
                                    .get("half")
                                    .is_some_and(|half| half == "lower"),
                        )
                    }
                    Some("lower") => {
                        let below = self.probe_block(relative(pos, Direction::Down))?;
                        self.placement_block_is_in_tag(&below, "minecraft:supports_small_dripleaf")
                    }
                    _ => None,
                }
            }
            BlockCanSurviveRule::BelowLegacySolid => {
                let below = self.probe_block(relative(pos, Direction::Down))?;
                vanilla_block_state_support_facts(below.block_state_id)
                    .map(|facts| facts.legacy_solid())
            }
            BlockCanSurviveRule::FacingLegacySolid => {
                let facing = direction_property(properties, "facing")?;
                let support = self.probe_block(relative(pos, opposite(facing)))?;
                vanilla_block_state_support_facts(support.block_state_id)
                    .map(|facts| facts.legacy_solid())
            }
            BlockCanSurviveRule::FaceAttached => {
                let face = properties.get("face")?.as_str();
                let support_direction = match face {
                    "ceiling" => Direction::Up,
                    "floor" => Direction::Down,
                    "wall" => opposite(direction_property(properties, "facing")?),
                    _ => return None,
                };
                let support = self.probe_block(relative(pos, support_direction))?;
                self.placement_face_sturdy(
                    &support,
                    opposite(support_direction),
                    VanillaBlockSupportType::Full,
                )
            }
            BlockCanSurviveRule::BelowTag(tag) => {
                let below = self.probe_block(relative(pos, Direction::Down))?;
                self.placement_block_is_in_tag(&below, tag)
            }
        }
    }

    pub(super) fn placement_face_sturdy(
        &self,
        block: &BlockProbe,
        direction: Direction,
        support_type: VanillaBlockSupportType,
    ) -> Option<bool> {
        let facts = vanilla_block_state_support_facts(block.block_state_id)?;
        if !facts.dynamic_shape() {
            return Some(facts.face_sturdy(support_direction(direction), support_type));
        }
        Some(match support_type {
            VanillaBlockSupportType::Full => self.block_has_full_sturdy_face(block, direction),
            VanillaBlockSupportType::Center => {
                self.block_has_vertical_center_sturdy_face(block, direction)
            }
            VanillaBlockSupportType::Rigid => {
                self.block_has_vertical_rigid_sturdy_face(block, direction)
            }
        })
    }

    fn placement_collision_face_nonempty(
        &self,
        block: &BlockProbe,
        direction: Direction,
    ) -> Option<bool> {
        let facts = vanilla_block_state_support_facts(block.block_state_id)?;
        Some(if facts.dynamic_shape() {
            self.block_has_nonempty_collision_face(block, direction)
        } else {
            facts.collision_face_nonempty(support_direction(direction))
        })
    }

    fn placement_full_support_or_collision_face(
        &self,
        block: &BlockProbe,
        direction: Direction,
    ) -> Option<bool> {
        let facts = vanilla_block_state_support_facts(block.block_state_id)?;
        Some(if facts.dynamic_shape() {
            self.block_has_full_support_or_collision_face(block, sturdy_face_index(direction))
        } else {
            let direction = support_direction(direction);
            facts.face_sturdy(direction, VanillaBlockSupportType::Full)
                || facts.collision_face_full(direction)
        })
    }

    pub(super) fn placement_block_is_in_tag(&self, block: &BlockProbe, tag: &str) -> Option<bool> {
        let block_type_id = block_type_id(block)?;
        self.registry_holder_ids_for_tag("minecraft:block", tag)
            .map(|holder_ids| holder_ids.contains(&block_type_id))
    }
}

fn support_direction(direction: Direction) -> VanillaBlockSupportDirection {
    match direction {
        Direction::Down => VanillaBlockSupportDirection::Down,
        Direction::Up => VanillaBlockSupportDirection::Up,
        Direction::North => VanillaBlockSupportDirection::North,
        Direction::South => VanillaBlockSupportDirection::South,
        Direction::West => VanillaBlockSupportDirection::West,
        Direction::East => VanillaBlockSupportDirection::East,
    }
}

fn sturdy_face_index(direction: Direction) -> usize {
    match direction {
        Direction::West => 0,
        Direction::South => 1,
        Direction::East => 2,
        Direction::North => 3,
        Direction::Down => 4,
        Direction::Up => 5,
    }
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

fn boolean_property(properties: &BTreeMap<String, String>, property: &str) -> Option<bool> {
    parse_boolean_property(properties.get(property)?)
}

fn parse_boolean_property(value: &str) -> Option<bool> {
    match value {
        "true" => Some(true),
        "false" => Some(false),
        _ => None,
    }
}

fn integer_property(properties: &BTreeMap<String, String>, property: &str) -> Option<i32> {
    properties.get(property)?.parse().ok()
}

fn block_type_id(block: &BlockProbe) -> Option<i32> {
    block
        .block_name
        .as_deref()
        .and_then(vanilla_block_type_for_resource_id)
        .map(|block_type| block_type.id)
}

#[cfg(test)]
pub(in super::super) const fn block_can_survive_owner_supported(
    owner: VanillaCanSurviveOverride,
) -> bool {
    block_can_survive_rule(owner).is_some()
}

const fn block_can_survive_rule(owner: VanillaCanSurviveOverride) -> Option<BlockCanSurviveRule> {
    Some(match owner {
        VanillaCanSurviveOverride::AmethystClusterBlock
        | VanillaCanSurviveOverride::BaseCoralWallFanBlock
        | VanillaCanSurviveOverride::LadderBlock => BlockCanSurviveRule::FacingFull,
        VanillaCanSurviveOverride::BaseCoralPlantTypeBlock
        | VanillaCanSurviveOverride::DoorBlock
        | VanillaCanSurviveOverride::LeafLitterBlock => BlockCanSurviveRule::BelowFull,
        VanillaCanSurviveOverride::BaseRailBlock => BlockCanSurviveRule::BelowRigid,
        VanillaCanSurviveOverride::BigDripleafBlock => BlockCanSurviveRule::BigDripleaf,
        VanillaCanSurviveOverride::BigDripleafStemBlock => BlockCanSurviveRule::BigDripleafStem,
        VanillaCanSurviveOverride::CandleBlock => BlockCanSurviveRule::BelowCenter,
        VanillaCanSurviveOverride::CeilingHangingSignBlock => BlockCanSurviveRule::AboveCenter,
        VanillaCanSurviveOverride::HangingRootsBlock => BlockCanSurviveRule::AboveFull,
        VanillaCanSurviveOverride::LanternBlock => BlockCanSurviveRule::Lantern,
        VanillaCanSurviveOverride::MangrovePropaguleBlock => BlockCanSurviveRule::MangrovePropagule,
        VanillaCanSurviveOverride::MultifaceBlock => BlockCanSurviveRule::Multiface,
        VanillaCanSurviveOverride::PointedDripstoneBlock => BlockCanSurviveRule::PointedDripstone,
        VanillaCanSurviveOverride::ScaffoldingBlock => BlockCanSurviveRule::Scaffolding,
        VanillaCanSurviveOverride::SeaPickleBlock => BlockCanSurviveRule::SeaPickle,
        VanillaCanSurviveOverride::SmallDripleafBlock => BlockCanSurviveRule::SmallDripleaf,
        VanillaCanSurviveOverride::StandingSignBlock => BlockCanSurviveRule::BelowLegacySolid,
        VanillaCanSurviveOverride::BannerBlock => BlockCanSurviveRule::BelowLegacySolid,
        VanillaCanSurviveOverride::WallBannerBlock | VanillaCanSurviveOverride::WallSignBlock => {
            BlockCanSurviveRule::FacingLegacySolid
        }
        VanillaCanSurviveOverride::RedstoneWallTorchBlock
        | VanillaCanSurviveOverride::WallTorchBlock => BlockCanSurviveRule::FacingFull,
        VanillaCanSurviveOverride::FaceAttachedHorizontalDirectionalBlock => {
            BlockCanSurviveRule::FaceAttached
        }
        VanillaCanSurviveOverride::DoublePlantBlock => {
            BlockCanSurviveRule::BelowTag("minecraft:supports_vegetation")
        }
        _ => return None,
    })
}
