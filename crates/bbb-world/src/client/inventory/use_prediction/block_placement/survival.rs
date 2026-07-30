use std::collections::BTreeMap;

use bbb_protocol::{
    block_state_types::{
        vanilla_block_state_support_facts, VanillaBlockSupportDirection, VanillaBlockSupportType,
    },
    block_types::{
        vanilla_block_type_for_resource_id, VanillaCanSurviveOverride,
        VanillaGetStateForPlacementOverride, VANILLA_BLOCK_BIG_DRIPLEAF_ID,
        VANILLA_BLOCK_BIG_DRIPLEAF_STEM_ID, VANILLA_BLOCK_POINTED_DRIPSTONE_ID,
        VANILLA_BLOCK_SMALL_DRIPLEAF_ID,
    },
    packets::Direction,
};

use super::super::WorldStore;
use super::{
    context::{opposite, relative},
    scaffolding, surface,
};
use crate::{block_name_is_air, BlockPos, BlockProbe};

#[derive(Debug, Clone, Copy)]
enum BlockCanSurviveRule {
    Always,
    FacingFull,
    BelowFull,
    BelowRigid,
    BasePressurePlate,
    BigDripleaf,
    BigDripleafStem,
    Bell,
    BelowCenter,
    AboveCenter,
    AboveFull,
    Bamboo,
    Cactus,
    CactusFlower,
    Cake,
    Carpet,
    ChorusFlower,
    ChorusPlant,
    Crop,
    DirtPath,
    DoublePlant,
    Farmland,
    GrowingPlant {
        growth_direction: Direction,
        head_name: &'static str,
        body_name: &'static str,
        cannot_attach_tag: Option<&'static str>,
    },
    HangingMoss,
    Lantern,
    LilyPad,
    MangrovePropagule,
    MossyCarpet,
    Multiface,
    Mushroom,
    PitcherCrop,
    PointedDripstone,
    RedStoneWire,
    Scaffolding,
    SeaPickle,
    Seagrass,
    SmallDripleaf,
    SnowLayer,
    SporeBlossom,
    SugarCane,
    Vine,
    BelowLegacySolid,
    FacingLegacySolid,
    FacingTag(&'static str),
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
        let rule = match owner {
            VanillaCanSurviveOverride::GrowingPlantBlock => growing_plant_rule(&state.name)?,
            VanillaCanSurviveOverride::VegetationBlock => vegetation_rule(&state.name)?,
            _ => block_can_survive_rule(owner)?,
        };
        self.evaluate_block_can_survive(pos, &state.name, &state.properties, rule)
    }

    fn evaluate_block_can_survive(
        &self,
        pos: BlockPos,
        block_name: &str,
        properties: &BTreeMap<String, String>,
        rule: BlockCanSurviveRule,
    ) -> Option<bool> {
        match rule {
            BlockCanSurviveRule::Always => Some(true),
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
            BlockCanSurviveRule::BasePressurePlate => {
                let below = self.probe_block(relative(pos, Direction::Down))?;
                let rigid = self.placement_face_sturdy(
                    &below,
                    Direction::Up,
                    VanillaBlockSupportType::Rigid,
                )?;
                if rigid {
                    Some(true)
                } else {
                    self.placement_face_sturdy(
                        &below,
                        Direction::Up,
                        VanillaBlockSupportType::Center,
                    )
                }
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
            BlockCanSurviveRule::Bell => {
                let attachment = properties.get("attachment")?.as_str();
                let facing = direction_property(properties, "facing")?;
                let support_direction = match attachment {
                    "floor" => Direction::Down,
                    "ceiling" => Direction::Up,
                    "single_wall" | "double_wall" => facing,
                    _ => return None,
                };
                let support = self.probe_block(relative(pos, support_direction))?;
                if attachment == "ceiling" {
                    if self
                        .placement_block_is_in_tag(&support, "minecraft:unstable_bottom_center")?
                    {
                        return Some(false);
                    }
                    self.placement_face_sturdy(
                        &support,
                        Direction::Down,
                        VanillaBlockSupportType::Center,
                    )
                } else {
                    self.placement_face_sturdy(
                        &support,
                        opposite(support_direction),
                        VanillaBlockSupportType::Full,
                    )
                }
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
            BlockCanSurviveRule::Bamboo => {
                let below = self.probe_block(relative(pos, Direction::Down))?;
                self.placement_block_is_in_tag(&below, "minecraft:supports_bamboo")
            }
            BlockCanSurviveRule::Cactus => {
                for direction in [
                    Direction::North,
                    Direction::East,
                    Direction::South,
                    Direction::West,
                ] {
                    let neighbor = self.probe_block(relative(pos, direction))?;
                    let solid =
                        vanilla_block_state_support_facts(neighbor.block_state_id)?.legacy_solid();
                    let lava = self.placement_fluid_is_in_tag(&neighbor, "minecraft:lava")?;
                    if solid || lava {
                        return Some(false);
                    }
                }
                let below = self.probe_block(relative(pos, Direction::Down))?;
                let supported = below.block_name.as_deref() == Some("minecraft:cactus")
                    || self.placement_block_is_in_tag(&below, "minecraft:supports_cactus")?;
                if !supported {
                    return Some(false);
                }
                let above = self.probe_block(relative(pos, Direction::Up))?;
                vanilla_block_state_support_facts(above.block_state_id).map(|facts| !facts.liquid())
            }
            BlockCanSurviveRule::CactusFlower => {
                let below = self.probe_block(relative(pos, Direction::Down))?;
                if self
                    .placement_block_is_in_tag(&below, "minecraft:support_override_cactus_flower")?
                {
                    return Some(true);
                }
                self.placement_face_sturdy(&below, Direction::Up, VanillaBlockSupportType::Center)
            }
            BlockCanSurviveRule::Cake => {
                let below = self.probe_block(relative(pos, Direction::Down))?;
                vanilla_block_state_support_facts(below.block_state_id)
                    .map(|facts| facts.legacy_solid())
            }
            BlockCanSurviveRule::Carpet => {
                let below = self.probe_block(relative(pos, Direction::Down))?;
                probe_is_air(&below).map(|air| !air)
            }
            BlockCanSurviveRule::ChorusFlower => {
                let below = self.probe_block(relative(pos, Direction::Down))?;
                if below.block_name.as_deref() == Some("minecraft:chorus_plant")
                    || self.placement_block_is_in_tag(&below, "minecraft:supports_chorus_flower")?
                {
                    return Some(true);
                }
                if !probe_is_air(&below)? {
                    return Some(false);
                }

                let mut chorus_neighbor = false;
                for direction in [
                    Direction::North,
                    Direction::East,
                    Direction::South,
                    Direction::West,
                ] {
                    let neighbor = self.probe_block(relative(pos, direction))?;
                    if neighbor.block_name.as_deref() == Some("minecraft:chorus_plant") {
                        if chorus_neighbor {
                            return Some(false);
                        }
                        chorus_neighbor = true;
                    } else if !probe_is_air(&neighbor)? {
                        return Some(false);
                    }
                }
                Some(chorus_neighbor)
            }
            BlockCanSurviveRule::ChorusPlant => {
                let below = self.probe_block(relative(pos, Direction::Down))?;
                let above = self.probe_block(relative(pos, Direction::Up))?;
                let block_above_or_below = !probe_is_air(&above)? && !probe_is_air(&below)?;
                for direction in [
                    Direction::North,
                    Direction::East,
                    Direction::South,
                    Direction::West,
                ] {
                    let neighbour_pos = relative(pos, direction);
                    let neighbour = self.probe_block(neighbour_pos)?;
                    if neighbour.block_name.as_deref() != Some("minecraft:chorus_plant") {
                        continue;
                    }
                    if block_above_or_below {
                        return Some(false);
                    }
                    let neighbour_below =
                        self.probe_block(relative(neighbour_pos, Direction::Down))?;
                    if self.placement_block_is_chorus_support(&neighbour_below)? {
                        return Some(true);
                    }
                }
                self.placement_block_is_chorus_support(&below)
            }
            BlockCanSurviveRule::Crop => {
                if self.placement_raw_brightness(pos)? < 8 {
                    return Some(false);
                }
                let below = self.probe_block(relative(pos, Direction::Down))?;
                self.placement_block_is_in_tag(&below, "minecraft:supports_crops")
            }
            BlockCanSurviveRule::Farmland => {
                let above = self.probe_block(relative(pos, Direction::Up))?;
                let solid = vanilla_block_state_support_facts(above.block_state_id)?.legacy_solid();
                if !solid {
                    Some(true)
                } else {
                    self.placement_block_is_in_tag(&above, "minecraft:maintains_farmland")
                }
            }
            BlockCanSurviveRule::DirtPath => {
                let above = self.probe_block(relative(pos, Direction::Up))?;
                let solid = vanilla_block_state_support_facts(above.block_state_id)?.legacy_solid();
                let fence_gate = above
                    .block_name
                    .as_deref()
                    .and_then(vanilla_block_type_for_resource_id)
                    .is_some_and(|block_type| {
                        block_type.get_state_for_placement_override
                            == Some(VanillaGetStateForPlacementOverride::FenceGateBlock)
                    });
                Some(!solid || fence_gate)
            }
            BlockCanSurviveRule::DoublePlant => match properties.get("half")?.as_str() {
                "lower" => {
                    let below = self.probe_block(relative(pos, Direction::Down))?;
                    self.placement_block_is_in_tag(&below, "minecraft:supports_vegetation")
                }
                "upper" => {
                    let below = self.probe_block(relative(pos, Direction::Down))?;
                    Some(
                        below.block_name.as_deref() == Some(block_name)
                            && below
                                .block_properties
                                .get("half")
                                .is_some_and(|half| half == "lower"),
                    )
                }
                _ => None,
            },
            BlockCanSurviveRule::GrowingPlant {
                growth_direction,
                head_name,
                body_name,
                cannot_attach_tag,
            } => {
                let attached_pos = relative(pos, opposite(growth_direction));
                let attached = self.probe_block(attached_pos)?;
                if let Some(tag) = cannot_attach_tag {
                    if self.placement_block_is_in_tag(&attached, tag)? {
                        return Some(false);
                    }
                }
                if matches!(
                    attached.block_name.as_deref(),
                    Some(name) if name == head_name || name == body_name
                ) {
                    return Some(true);
                }
                self.placement_face_sturdy(
                    &attached,
                    growth_direction,
                    VanillaBlockSupportType::Full,
                )
            }
            BlockCanSurviveRule::HangingMoss => {
                let above = self.probe_block(relative(pos, Direction::Up))?;
                if above.block_name.as_deref() == Some("minecraft:pale_hanging_moss") {
                    Some(true)
                } else {
                    self.placement_full_support_or_collision_face(&above, Direction::Down)
                }
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
            BlockCanSurviveRule::LilyPad => {
                let below = self.probe_block(relative(pos, Direction::Down))?;
                let supported = self
                    .placement_fluid_is_in_tag(&below, "minecraft:supports_lily_pad")?
                    || self.placement_block_is_in_tag(&below, "minecraft:supports_lily_pad")?;
                if !supported {
                    return Some(false);
                }
                self.probe_block(pos).map(|target| target.fluid.is_none())
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
            BlockCanSurviveRule::MossyCarpet => {
                let below = self.probe_block(relative(pos, Direction::Down))?;
                let below_name = below.block_name.as_deref()?;
                if boolean_property(properties, "bottom")? {
                    Some(!block_name_is_air(below_name))
                } else {
                    Some(
                        below_name == "minecraft:pale_moss_carpet"
                            && boolean_property(&below.block_properties, "bottom")?,
                    )
                }
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
            BlockCanSurviveRule::Mushroom => {
                let below = self.probe_block(relative(pos, Direction::Down))?;
                if self.placement_block_is_in_tag(
                    &below,
                    "minecraft:overrides_mushroom_light_requirement",
                )? {
                    return Some(true);
                }
                Some(
                    self.placement_raw_brightness(pos)? < 13
                        && vanilla_block_state_support_facts(below.block_state_id)?.solid_render(),
                )
            }
            BlockCanSurviveRule::PitcherCrop => match properties.get("half")?.as_str() {
                "lower" => {
                    if self.placement_raw_brightness(pos)? < 8 {
                        return Some(false);
                    }
                    let below = self.probe_block(relative(pos, Direction::Down))?;
                    self.placement_block_is_in_tag(&below, "minecraft:supports_crops")
                }
                "upper" => {
                    let below = self.probe_block(relative(pos, Direction::Down))?;
                    Some(
                        below.block_name.as_deref() == Some("minecraft:pitcher_crop")
                            && below
                                .block_properties
                                .get("half")
                                .is_some_and(|half| half == "lower"),
                    )
                }
                _ => None,
            },
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
            BlockCanSurviveRule::RedStoneWire => {
                let below = self.probe_block(relative(pos, Direction::Down))?;
                if below.block_name.as_deref() == Some("minecraft:hopper") {
                    Some(true)
                } else {
                    self.placement_face_sturdy(&below, Direction::Up, VanillaBlockSupportType::Full)
                }
            }
            BlockCanSurviveRule::Scaffolding => {
                scaffolding::distance(self, pos).map(|distance| distance < 7)
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
            BlockCanSurviveRule::Seagrass => {
                let below = self.probe_block(relative(pos, Direction::Down))?;
                if !self.placement_face_sturdy(
                    &below,
                    Direction::Up,
                    VanillaBlockSupportType::Full,
                )? {
                    return Some(false);
                }
                self.placement_block_is_in_tag(&below, "minecraft:cannot_support_seagrass")
                    .map(|cannot_support| !cannot_support)
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
            BlockCanSurviveRule::SnowLayer => {
                let below = self.probe_block(relative(pos, Direction::Down))?;
                if self.placement_block_is_in_tag(&below, "minecraft:cannot_support_snow_layer")? {
                    return Some(false);
                }
                if self
                    .placement_block_is_in_tag(&below, "minecraft:support_override_snow_layer")?
                {
                    return Some(true);
                }
                if below.block_name.as_deref() == Some("minecraft:snow")
                    && below
                        .block_properties
                        .get("layers")
                        .is_some_and(|layers| layers == "8")
                {
                    return Some(true);
                }
                self.placement_collision_face_is_full(&below, Direction::Up)
            }
            BlockCanSurviveRule::SporeBlossom => {
                let target = self.probe_block(pos)?;
                if self.placement_fluid_is_in_tag(&target, "minecraft:water")? {
                    return Some(false);
                }
                let above = self.probe_block(relative(pos, Direction::Up))?;
                self.placement_face_sturdy(&above, Direction::Down, VanillaBlockSupportType::Center)
            }
            BlockCanSurviveRule::SugarCane => {
                let below_pos = relative(pos, Direction::Down);
                let below = self.probe_block(below_pos)?;
                if below.block_name.as_deref() == Some("minecraft:sugar_cane") {
                    return Some(true);
                }
                if !self.placement_block_is_in_tag(&below, "minecraft:supports_sugar_cane")? {
                    return Some(false);
                }
                for direction in [
                    Direction::North,
                    Direction::East,
                    Direction::South,
                    Direction::West,
                ] {
                    let adjacent = self.probe_block(relative(below_pos, direction))?;
                    if self.placement_fluid_is_in_tag(
                        &adjacent,
                        "minecraft:supports_sugar_cane_adjacently",
                    )? || self.placement_block_is_in_tag(
                        &adjacent,
                        "minecraft:supports_sugar_cane_adjacently",
                    )? {
                        return Some(true);
                    }
                }
                Some(false)
            }
            BlockCanSurviveRule::Vine => surface::vine_can_survive(self, pos, properties),
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
            BlockCanSurviveRule::FacingTag(tag) => {
                let facing = direction_property(properties, "facing")?;
                let support = self.probe_block(relative(pos, facing))?;
                self.placement_block_is_in_tag(&support, tag)
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
        if support_type == VanillaBlockSupportType::Full {
            return self.block_full_face_sturdy(block, direction);
        }
        let facts = vanilla_block_state_support_facts(block.block_state_id)?;
        if !facts.dynamic_shape() {
            return Some(facts.face_sturdy(support_direction(direction), support_type));
        }
        Some(match support_type {
            VanillaBlockSupportType::Full => unreachable!("FULL returned above"),
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

    pub(super) fn placement_collision_face_is_full(
        &self,
        block: &BlockProbe,
        direction: Direction,
    ) -> Option<bool> {
        let facts = vanilla_block_state_support_facts(block.block_state_id)?;
        Some(if facts.dynamic_shape() {
            self.block_empty_context_collision_face_covers_rectangle(
                block,
                direction,
                [0.0, 1.0, 0.0, 1.0],
            )
        } else {
            facts.collision_face_full(support_direction(direction))
        })
    }

    pub(super) fn placement_full_support_or_collision_face(
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

    fn placement_fluid_is_in_tag(&self, block: &BlockProbe, tag: &str) -> Option<bool> {
        let fluid_resource_id = crate::terrain::vanilla_fluid_identity(block.fluid).resource_id();
        let holder_id =
            self.registry_holder_id_by_resource_id("minecraft:fluid", fluid_resource_id)?;
        self.registry_holder_ids_for_tag("minecraft:fluid", tag)
            .map(|holder_ids| holder_ids.contains(&holder_id))
    }

    fn placement_raw_brightness(&self, pos: BlockPos) -> Option<u8> {
        let light = self.sample_block_light(pos)?;
        let sky = if self.level_info()?.has_sky_light() {
            light.sky
        } else {
            0
        };
        Some(sky.max(light.block))
    }

    fn placement_block_is_chorus_support(&self, block: &BlockProbe) -> Option<bool> {
        if block.block_name.as_deref() == Some("minecraft:chorus_plant") {
            Some(true)
        } else {
            self.placement_block_is_in_tag(block, "minecraft:supports_chorus_plant")
        }
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

fn block_type_id(block: &BlockProbe) -> Option<i32> {
    block
        .block_name
        .as_deref()
        .and_then(vanilla_block_type_for_resource_id)
        .map(|block_type| block_type.id)
}

fn probe_is_air(block: &BlockProbe) -> Option<bool> {
    block.block_name.as_deref().map(block_name_is_air)
}

#[cfg(test)]
pub(in super::super) fn block_can_survive_owner_supported(
    owner: VanillaCanSurviveOverride,
    block_name: &str,
) -> bool {
    match owner {
        VanillaCanSurviveOverride::GrowingPlantBlock => growing_plant_rule(block_name).is_some(),
        VanillaCanSurviveOverride::VegetationBlock => vegetation_rule(block_name).is_some(),
        _ => block_can_survive_rule(owner).is_some(),
    }
}

fn vegetation_rule(block_name: &str) -> Option<BlockCanSurviveRule> {
    Some(match block_name {
        "minecraft:seagrass" => BlockCanSurviveRule::Seagrass,
        "minecraft:cactus_flower" => BlockCanSurviveRule::CactusFlower,
        "minecraft:lily_pad" => BlockCanSurviveRule::LilyPad,
        "minecraft:dead_bush" | "minecraft:short_dry_grass" | "minecraft:tall_dry_grass" => {
            BlockCanSurviveRule::BelowTag("minecraft:supports_dry_vegetation")
        }
        "minecraft:wither_rose" => BlockCanSurviveRule::BelowTag("minecraft:supports_wither_rose"),
        "minecraft:attached_pumpkin_stem" | "minecraft:pumpkin_stem" => {
            BlockCanSurviveRule::BelowTag("minecraft:supports_pumpkin_stem")
        }
        "minecraft:attached_melon_stem" | "minecraft:melon_stem" => {
            BlockCanSurviveRule::BelowTag("minecraft:supports_melon_stem")
        }
        "minecraft:nether_wart" => BlockCanSurviveRule::BelowTag("minecraft:supports_nether_wart"),
        "minecraft:warped_fungus" => {
            BlockCanSurviveRule::BelowTag("minecraft:supports_warped_fungus")
        }
        "minecraft:crimson_fungus" => {
            BlockCanSurviveRule::BelowTag("minecraft:supports_crimson_fungus")
        }
        "minecraft:warped_roots" => {
            BlockCanSurviveRule::BelowTag("minecraft:supports_warped_roots")
        }
        "minecraft:crimson_roots" => {
            BlockCanSurviveRule::BelowTag("minecraft:supports_crimson_roots")
        }
        "minecraft:nether_sprouts" => {
            BlockCanSurviveRule::BelowTag("minecraft:supports_nether_sprouts")
        }
        "minecraft:azalea" | "minecraft:flowering_azalea" => {
            BlockCanSurviveRule::BelowTag("minecraft:supports_azalea")
        }
        "minecraft:oak_sapling"
        | "minecraft:spruce_sapling"
        | "minecraft:birch_sapling"
        | "minecraft:jungle_sapling"
        | "minecraft:acacia_sapling"
        | "minecraft:cherry_sapling"
        | "minecraft:dark_oak_sapling"
        | "minecraft:pale_oak_sapling"
        | "minecraft:short_grass"
        | "minecraft:fern"
        | "minecraft:bush"
        | "minecraft:dandelion"
        | "minecraft:golden_dandelion"
        | "minecraft:torchflower"
        | "minecraft:poppy"
        | "minecraft:blue_orchid"
        | "minecraft:allium"
        | "minecraft:azure_bluet"
        | "minecraft:red_tulip"
        | "minecraft:orange_tulip"
        | "minecraft:white_tulip"
        | "minecraft:pink_tulip"
        | "minecraft:oxeye_daisy"
        | "minecraft:cornflower"
        | "minecraft:lily_of_the_valley"
        | "minecraft:sweet_berry_bush"
        | "minecraft:pink_petals"
        | "minecraft:wildflowers"
        | "minecraft:open_eyeblossom"
        | "minecraft:closed_eyeblossom"
        | "minecraft:firefly_bush" => {
            BlockCanSurviveRule::BelowTag("minecraft:supports_vegetation")
        }
        _ => return None,
    })
}

fn growing_plant_rule(block_name: &str) -> Option<BlockCanSurviveRule> {
    Some(match block_name {
        "minecraft:kelp" | "minecraft:kelp_plant" => BlockCanSurviveRule::GrowingPlant {
            growth_direction: Direction::Up,
            head_name: "minecraft:kelp",
            body_name: "minecraft:kelp_plant",
            cannot_attach_tag: Some("minecraft:cannot_support_kelp"),
        },
        "minecraft:weeping_vines" | "minecraft:weeping_vines_plant" => {
            BlockCanSurviveRule::GrowingPlant {
                growth_direction: Direction::Down,
                head_name: "minecraft:weeping_vines",
                body_name: "minecraft:weeping_vines_plant",
                cannot_attach_tag: None,
            }
        }
        "minecraft:twisting_vines" | "minecraft:twisting_vines_plant" => {
            BlockCanSurviveRule::GrowingPlant {
                growth_direction: Direction::Up,
                head_name: "minecraft:twisting_vines",
                body_name: "minecraft:twisting_vines_plant",
                cannot_attach_tag: None,
            }
        }
        "minecraft:cave_vines" | "minecraft:cave_vines_plant" => {
            BlockCanSurviveRule::GrowingPlant {
                growth_direction: Direction::Down,
                head_name: "minecraft:cave_vines",
                body_name: "minecraft:cave_vines_plant",
                cannot_attach_tag: None,
            }
        }
        _ => return None,
    })
}

const fn block_can_survive_rule(owner: VanillaCanSurviveOverride) -> Option<BlockCanSurviveRule> {
    Some(match owner {
        VanillaCanSurviveOverride::AmethystClusterBlock
        | VanillaCanSurviveOverride::BaseCoralWallFanBlock
        | VanillaCanSurviveOverride::LadderBlock
        | VanillaCanSurviveOverride::TripWireHookBlock => BlockCanSurviveRule::FacingFull,
        VanillaCanSurviveOverride::BaseCoralPlantTypeBlock
        | VanillaCanSurviveOverride::DoorBlock
        | VanillaCanSurviveOverride::LeafLitterBlock => BlockCanSurviveRule::BelowFull,
        VanillaCanSurviveOverride::BaseRailBlock | VanillaCanSurviveOverride::DiodeBlock => {
            BlockCanSurviveRule::BelowRigid
        }
        VanillaCanSurviveOverride::BasePressurePlateBlock => BlockCanSurviveRule::BasePressurePlate,
        VanillaCanSurviveOverride::BaseTorchBlock => BlockCanSurviveRule::BelowCenter,
        VanillaCanSurviveOverride::BigDripleafBlock => BlockCanSurviveRule::BigDripleaf,
        VanillaCanSurviveOverride::BigDripleafStemBlock => BlockCanSurviveRule::BigDripleafStem,
        VanillaCanSurviveOverride::BellBlock => BlockCanSurviveRule::Bell,
        VanillaCanSurviveOverride::BambooSaplingBlock
        | VanillaCanSurviveOverride::BambooStalkBlock => BlockCanSurviveRule::Bamboo,
        VanillaCanSurviveOverride::CactusBlock => BlockCanSurviveRule::Cactus,
        VanillaCanSurviveOverride::CakeBlock => BlockCanSurviveRule::Cake,
        VanillaCanSurviveOverride::CarpetBlock => BlockCanSurviveRule::Carpet,
        VanillaCanSurviveOverride::CandleBlock => BlockCanSurviveRule::BelowCenter,
        VanillaCanSurviveOverride::CeilingHangingSignBlock => BlockCanSurviveRule::AboveCenter,
        VanillaCanSurviveOverride::ChorusFlowerBlock => BlockCanSurviveRule::ChorusFlower,
        VanillaCanSurviveOverride::ChorusPlantBlock => BlockCanSurviveRule::ChorusPlant,
        VanillaCanSurviveOverride::CropBlock => BlockCanSurviveRule::Crop,
        VanillaCanSurviveOverride::DirtPathBlock => BlockCanSurviveRule::DirtPath,
        VanillaCanSurviveOverride::FarmlandBlock => BlockCanSurviveRule::Farmland,
        VanillaCanSurviveOverride::GrindstoneBlock => BlockCanSurviveRule::Always,
        VanillaCanSurviveOverride::HangingMossBlock => BlockCanSurviveRule::HangingMoss,
        VanillaCanSurviveOverride::HangingRootsBlock => BlockCanSurviveRule::AboveFull,
        VanillaCanSurviveOverride::LanternBlock => BlockCanSurviveRule::Lantern,
        VanillaCanSurviveOverride::MangrovePropaguleBlock => BlockCanSurviveRule::MangrovePropagule,
        VanillaCanSurviveOverride::MossyCarpetBlock => BlockCanSurviveRule::MossyCarpet,
        VanillaCanSurviveOverride::MultifaceBlock => BlockCanSurviveRule::Multiface,
        VanillaCanSurviveOverride::MushroomBlock => BlockCanSurviveRule::Mushroom,
        VanillaCanSurviveOverride::PitcherCropBlock => BlockCanSurviveRule::PitcherCrop,
        VanillaCanSurviveOverride::PointedDripstoneBlock => BlockCanSurviveRule::PointedDripstone,
        VanillaCanSurviveOverride::RedStoneWireBlock => BlockCanSurviveRule::RedStoneWire,
        VanillaCanSurviveOverride::ScaffoldingBlock => BlockCanSurviveRule::Scaffolding,
        VanillaCanSurviveOverride::SeaPickleBlock => BlockCanSurviveRule::SeaPickle,
        VanillaCanSurviveOverride::SmallDripleafBlock => BlockCanSurviveRule::SmallDripleaf,
        VanillaCanSurviveOverride::SnowLayerBlock => BlockCanSurviveRule::SnowLayer,
        VanillaCanSurviveOverride::SporeBlossomBlock => BlockCanSurviveRule::SporeBlossom,
        VanillaCanSurviveOverride::SugarCaneBlock => BlockCanSurviveRule::SugarCane,
        VanillaCanSurviveOverride::VineBlock => BlockCanSurviveRule::Vine,
        VanillaCanSurviveOverride::StandingSignBlock => BlockCanSurviveRule::BelowLegacySolid,
        VanillaCanSurviveOverride::BannerBlock => BlockCanSurviveRule::BelowLegacySolid,
        VanillaCanSurviveOverride::WallBannerBlock | VanillaCanSurviveOverride::WallSignBlock => {
            BlockCanSurviveRule::FacingLegacySolid
        }
        VanillaCanSurviveOverride::CocoaBlock => {
            BlockCanSurviveRule::FacingTag("minecraft:supports_cocoa")
        }
        VanillaCanSurviveOverride::RedstoneWallTorchBlock
        | VanillaCanSurviveOverride::WallTorchBlock => BlockCanSurviveRule::FacingFull,
        VanillaCanSurviveOverride::FaceAttachedHorizontalDirectionalBlock => {
            BlockCanSurviveRule::FaceAttached
        }
        VanillaCanSurviveOverride::DoublePlantBlock => BlockCanSurviveRule::DoublePlant,
        _ => return None,
    })
}
