use std::collections::BTreeMap;

use bbb_protocol::{
    block_types::vanilla_block_type_for_resource_id,
    item_types::VanillaItemUseOnOverride,
    packets::{Direction as ProtocolDirection, InteractionHand},
};

use super::{
    LocalBlockUseContext, LocalInteractionResult, LocalInteractionSoundAction,
    LocalInteractionSuccess, LocalTargetUsePrediction,
};
use crate::{
    block_name_is_air, block_name_is_campfire, block_name_is_candle, block_name_is_candle_cake,
    client::block_collision_shape_has_full_up_face,
    terrain::{block_name_is_fire_flammable, block_name_is_soul_fire_base},
    BlockPos, BlockProbe, WorldStore,
};

const FIRE_BLOCK: &str = "minecraft:fire";
const SOUL_FIRE_BLOCK: &str = "minecraft:soul_fire";
const NETHER_PORTAL_BLOCK: &str = "minecraft:nether_portal";
const OBSIDIAN_BLOCK: &str = "minecraft:obsidian";
const MAX_PORTAL_WIDTH: i32 = 21;
const MAX_PORTAL_HEIGHT: i32 = 21;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum IgnitionKind {
    FlintAndSteel,
    FireCharge,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PortalAxis {
    X,
    Z,
}

impl PortalAxis {
    fn right(self) -> BlockOffset {
        match self {
            // `PortalShape`: Axis.X uses WEST as `rightDir`.
            Self::X => BlockOffset::WEST,
            Self::Z => BlockOffset::SOUTH,
        }
    }

    fn property_value(self) -> &'static str {
        match self {
            Self::X => "x",
            Self::Z => "z",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct PortalShape {
    axis: PortalAxis,
    bottom_left: BlockPos,
    width: i32,
    height: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct BlockOffset {
    x: i32,
    y: i32,
    z: i32,
}

impl BlockOffset {
    const DOWN: Self = Self { x: 0, y: -1, z: 0 };
    const UP: Self = Self { x: 0, y: 1, z: 0 };
    const NORTH: Self = Self { x: 0, y: 0, z: -1 };
    const SOUTH: Self = Self { x: 0, y: 0, z: 1 };
    const WEST: Self = Self { x: -1, y: 0, z: 0 };
    const EAST: Self = Self { x: 1, y: 0, z: 0 };

    fn opposite(self) -> Self {
        Self {
            x: -self.x,
            y: -self.y,
            z: -self.z,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum IgnitionMutation {
    One {
        pos: BlockPos,
        block_state_id: i32,
    },
    Portal {
        shape: PortalShape,
        block_state_id: i32,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct IgnitionPlan {
    sound_pos: BlockPos,
    mutation: IgnitionMutation,
}

impl WorldStore {
    pub(super) fn predict_local_ignition_item_use_on(
        &mut self,
        owner: VanillaItemUseOnOverride,
        hand: InteractionHand,
        context: LocalBlockUseContext,
    ) -> LocalTargetUsePrediction {
        let kind = match owner {
            VanillaItemUseOnOverride::FlintAndSteelItem => IgnitionKind::FlintAndSteel,
            VanillaItemUseOnOverride::FireChargeItem => IgnitionKind::FireCharge,
            _ => return LocalTargetUsePrediction::UnsupportedItemUseOn(owner),
        };
        let plan = match self.plan_local_ignition(context) {
            Ok(Some(plan)) => plan,
            Ok(None) => {
                return LocalTargetUsePrediction::Result(LocalInteractionResult::Fail);
            }
            Err(()) => return LocalTargetUsePrediction::UnsupportedItemUseOn(owner),
        };

        // Fire charges shrink on both logical sides after the block mutation.
        // Validate the already-selected stack before applying any multi-block
        // portal prediction so an inconsistent inventory cannot leave a
        // partial local world mutation.
        let shrinks_fire_charge = kind == IgnitionKind::FireCharge
            && !self
                .local_player()
                .abilities
                .is_some_and(|abilities| abilities.instabuild);
        if shrinks_fire_charge
            && !self
                .local_item_in_hand(hand)
                .is_some_and(|stack| stack.count >= 1)
        {
            return LocalTargetUsePrediction::UnsupportedItemUseOn(owner);
        }
        if !self.apply_local_ignition_plan(&plan, context.sequence) {
            return LocalTargetUsePrediction::UnsupportedItemUseOn(owner);
        }
        if shrinks_fire_charge && !self.shrink_local_item_in_hand(hand, 1) {
            return LocalTargetUsePrediction::UnsupportedItemUseOn(owner);
        }

        let sound_action = match kind {
            IgnitionKind::FlintAndSteel => LocalInteractionSoundAction::FlintAndSteelUse {
                pos: plan.sound_pos,
            },
            IgnitionKind::FireCharge => LocalInteractionSoundAction::SuppressedFireChargeUse,
        };
        LocalTargetUsePrediction::Result(LocalInteractionResult::Success(
            LocalInteractionSuccess::ignition(sound_action),
        ))
    }

    fn plan_local_ignition(
        &self,
        context: LocalBlockUseContext,
    ) -> Result<Option<IgnitionPlan>, ()> {
        let clicked = self.required_block(context.pos)?;
        if let Some(block_state_id) = self.lightable_block_state(&clicked)? {
            return Ok(Some(IgnitionPlan {
                sound_pos: context.pos,
                mutation: IgnitionMutation::One {
                    pos: context.pos,
                    block_state_id,
                },
            }));
        }

        let fire_pos = offset(context.pos, direction_offset(context.face), 1);
        let target = self.required_block(fire_pos)?;
        if !target.block_name.as_deref().is_some_and(block_name_is_air) {
            return Ok(None);
        }

        let (fire_state_id, survives) = self.fire_state_for_placement(fire_pos)?;
        let portal_dimension = self
            .level_info()
            .map(|level| {
                matches!(
                    level.dimension.as_str(),
                    "minecraft:overworld" | "minecraft:the_nether"
                )
            })
            .ok_or(())?;

        if !survives {
            if !portal_dimension || !self.has_adjacent_obsidian(fire_pos)? {
                return Ok(None);
            }
            if self.find_empty_portal_shape(fire_pos)?.is_none() {
                return Ok(None);
            }
        }

        // `BaseFireBlock.onPlace` performs a second portal lookup, always
        // preferring Axis.X, even when ordinary fire survival made
        // `canBePlacedAt` short-circuit before its portal lookup.
        let mutation = if portal_dimension {
            if let Some(shape) = self.find_empty_portal_shape(fire_pos)? {
                let properties =
                    BTreeMap::from([("axis".to_string(), shape.axis.property_value().to_string())]);
                let block_state_id = self
                    .registries
                    .block_state_id_by_name_and_properties(NETHER_PORTAL_BLOCK, &properties)
                    .ok_or(())?;
                IgnitionMutation::Portal {
                    shape,
                    block_state_id,
                }
            } else {
                IgnitionMutation::One {
                    pos: fire_pos,
                    block_state_id: fire_state_id,
                }
            }
        } else {
            IgnitionMutation::One {
                pos: fire_pos,
                block_state_id: fire_state_id,
            }
        };
        Ok(Some(IgnitionPlan {
            sound_pos: fire_pos,
            mutation,
        }))
    }

    fn lightable_block_state(&self, block: &BlockProbe) -> Result<Option<i32>, ()> {
        let name = block.block_name.as_deref().ok_or(())?;
        let lightable = if block_name_is_campfire(name) || block_name_is_candle(name) {
            match (
                block.block_properties.get("lit").map(String::as_str),
                block
                    .block_properties
                    .get("waterlogged")
                    .map(String::as_str),
            ) {
                (Some("false"), Some("false")) => true,
                (Some("true" | "false"), Some("true" | "false")) => false,
                _ => return Err(()),
            }
        } else if block_name_is_candle_cake(name) {
            match block.block_properties.get("lit").map(String::as_str) {
                Some("false") => true,
                Some("true") => false,
                _ => return Err(()),
            }
        } else {
            false
        };
        if !lightable {
            return Ok(None);
        }
        let mut properties = block.block_properties.clone();
        properties.insert("lit".to_string(), "true".to_string());
        self.registries
            .block_state_id_by_name_and_properties(name, &properties)
            .map(Some)
            .ok_or(())
    }

    fn fire_state_for_placement(&self, pos: BlockPos) -> Result<(i32, bool), ()> {
        let below_pos = offset(pos, BlockOffset::DOWN, 1);
        let below = self.optional_block(below_pos)?;
        if below
            .as_ref()
            .and_then(|block| block.block_name.as_deref())
            .is_some_and(block_name_is_soul_fire_base)
        {
            let state_id = self
                .registries
                .block_state_id_by_name_and_properties(SOUL_FIRE_BLOCK, &BTreeMap::new())
                .ok_or(())?;
            return Ok((state_id, true));
        }

        let below_flammable = below
            .as_ref()
            .is_some_and(|block| block_is_fire_flammable(block));
        let below_sturdy = below
            .as_ref()
            .is_some_and(|block| block_collision_shape_has_full_up_face(block, below_pos));
        let default_properties = BTreeMap::from([
            ("age".to_string(), "0".to_string()),
            ("east".to_string(), "false".to_string()),
            ("north".to_string(), "false".to_string()),
            ("south".to_string(), "false".to_string()),
            ("up".to_string(), "false".to_string()),
            ("west".to_string(), "false".to_string()),
        ]);
        if below_flammable || below_sturdy {
            let state_id = self
                .registries
                .block_state_id_by_name_and_properties(FIRE_BLOCK, &default_properties)
                .ok_or(())?;
            return Ok((state_id, true));
        }

        let directional = [
            ("up", BlockOffset::UP),
            ("north", BlockOffset::NORTH),
            ("south", BlockOffset::SOUTH),
            ("west", BlockOffset::WEST),
            ("east", BlockOffset::EAST),
        ]
        .map(|(property, direction)| {
            self.optional_block(offset(pos, direction, 1)).map(|block| {
                (
                    property,
                    block
                        .as_ref()
                        .is_some_and(|block| block_is_fire_flammable(block)),
                )
            })
        });
        let mut properties = default_properties;
        let mut any_directional_flammable = false;
        for result in directional {
            let (property, flammable) = result?;
            any_directional_flammable |= flammable;
            properties.insert(property.to_string(), flammable.to_string());
        }
        let state_id = self
            .registries
            .block_state_id_by_name_and_properties(FIRE_BLOCK, &properties)
            .ok_or(())?;
        Ok((state_id, any_directional_flammable))
    }

    fn find_empty_portal_shape(&self, pos: BlockPos) -> Result<Option<PortalShape>, ()> {
        for axis in [PortalAxis::X, PortalAxis::Z] {
            if let Some(shape) = self.find_empty_portal_shape_on_axis(pos, axis)? {
                return Ok(Some(shape));
            }
        }
        Ok(None)
    }

    fn find_empty_portal_shape_on_axis(
        &self,
        pos: BlockPos,
        axis: PortalAxis,
    ) -> Result<Option<PortalShape>, ()> {
        let right = axis.right();
        let mut bottom = pos;
        let min_y = self
            .dimension()
            .min_y
            .max(pos.y.saturating_sub(MAX_PORTAL_HEIGHT));
        while bottom.y > min_y && self.portal_cell_is_empty(offset(bottom, BlockOffset::DOWN, 1))? {
            bottom = offset(bottom, BlockOffset::DOWN, 1);
        }
        let edge = self.distance_until_edge_above_frame(bottom, right.opposite())? - 1;
        if edge < 0 {
            return Ok(None);
        }
        let bottom_left = offset(bottom, right.opposite(), edge);
        let width = self.distance_until_edge_above_frame(bottom_left, right)?;
        if !(2..=MAX_PORTAL_WIDTH).contains(&width) {
            return Ok(None);
        }

        let (height, portal_count) = self.distance_until_portal_top(bottom_left, right, width)?;
        if !(3..=MAX_PORTAL_HEIGHT).contains(&height) || portal_count != 0 {
            return Ok(None);
        }
        for column in 0..width {
            let top = offset(offset(bottom_left, BlockOffset::UP, height), right, column);
            if !self.block_is_obsidian(top)? {
                return Ok(None);
            }
        }
        Ok(Some(PortalShape {
            axis,
            bottom_left,
            width,
            height,
        }))
    }

    fn distance_until_edge_above_frame(
        &self,
        pos: BlockPos,
        direction: BlockOffset,
    ) -> Result<i32, ()> {
        for distance in 0..=MAX_PORTAL_WIDTH {
            let candidate = offset(pos, direction, distance);
            if !self.portal_cell_is_empty(candidate)? {
                return Ok(if self.block_is_obsidian(candidate)? {
                    distance
                } else {
                    0
                });
            }
            if !self.block_is_obsidian(offset(candidate, BlockOffset::DOWN, 1))? {
                return Ok(0);
            }
        }
        Ok(0)
    }

    fn distance_until_portal_top(
        &self,
        bottom_left: BlockPos,
        right: BlockOffset,
        width: i32,
    ) -> Result<(i32, i32), ()> {
        let mut portal_count = 0;
        for height in 0..MAX_PORTAL_HEIGHT {
            let row = offset(bottom_left, BlockOffset::UP, height);
            if !self.block_is_obsidian(offset(row, right, -1))?
                || !self.block_is_obsidian(offset(row, right, width))?
            {
                return Ok((height, portal_count));
            }
            for column in 0..width {
                let cell = offset(row, right, column);
                if !self.portal_cell_is_empty(cell)? {
                    return Ok((height, portal_count));
                }
                if self.block_name(cell)? == NETHER_PORTAL_BLOCK {
                    portal_count += 1;
                }
            }
        }
        Ok((MAX_PORTAL_HEIGHT, portal_count))
    }

    fn has_adjacent_obsidian(&self, pos: BlockPos) -> Result<bool, ()> {
        for direction in [
            BlockOffset::DOWN,
            BlockOffset::UP,
            BlockOffset::NORTH,
            BlockOffset::SOUTH,
            BlockOffset::WEST,
            BlockOffset::EAST,
        ] {
            if self.block_is_obsidian(offset(pos, direction, 1))? {
                return Ok(true);
            }
        }
        Ok(false)
    }

    fn portal_cell_is_empty(&self, pos: BlockPos) -> Result<bool, ()> {
        let name = self.block_name(pos)?;
        Ok(block_name_is_air(&name)
            || matches!(
                name.as_str(),
                FIRE_BLOCK | SOUL_FIRE_BLOCK | NETHER_PORTAL_BLOCK
            ))
    }

    fn block_is_obsidian(&self, pos: BlockPos) -> Result<bool, ()> {
        Ok(self.block_name(pos)? == OBSIDIAN_BLOCK)
    }

    fn block_name(&self, pos: BlockPos) -> Result<String, ()> {
        if !self.dimension().contains_y(pos.y) {
            return Ok("minecraft:void_air".to_string());
        }
        let block = self.probe_block(pos).ok_or(())?;
        block.block_name.ok_or(())
    }

    fn required_block(&self, pos: BlockPos) -> Result<BlockProbe, ()> {
        if !self.dimension().contains_y(pos.y) {
            return Err(());
        }
        let block = self.probe_block(pos).ok_or(())?;
        let name = block.block_name.as_deref().ok_or(())?;
        vanilla_block_type_for_resource_id(name).ok_or(())?;
        Ok(block)
    }

    fn optional_block(&self, pos: BlockPos) -> Result<Option<BlockProbe>, ()> {
        if !self.dimension().contains_y(pos.y) {
            return Ok(None);
        }
        self.required_block(pos).map(Some)
    }

    fn apply_local_ignition_plan(&mut self, plan: &IgnitionPlan, sequence: i32) -> bool {
        match plan.mutation {
            IgnitionMutation::One {
                pos,
                block_state_id,
            } => self.predict_local_block_state(pos, block_state_id, sequence),
            IgnitionMutation::Portal {
                shape,
                block_state_id,
            } => {
                for height in 0..shape.height {
                    let row = offset(shape.bottom_left, BlockOffset::UP, height);
                    for column in 0..shape.width {
                        if !self.predict_local_block_state(
                            offset(row, shape.axis.right(), column),
                            block_state_id,
                            sequence,
                        ) {
                            return false;
                        }
                    }
                }
                true
            }
        }
    }
}

fn block_is_fire_flammable(block: &BlockProbe) -> bool {
    block
        .block_name
        .as_deref()
        .is_some_and(block_name_is_fire_flammable)
        && !block
            .block_properties
            .get("waterlogged")
            .is_some_and(|value| value == "true")
}

fn direction_offset(direction: ProtocolDirection) -> BlockOffset {
    match direction {
        ProtocolDirection::Down => BlockOffset::DOWN,
        ProtocolDirection::Up => BlockOffset::UP,
        ProtocolDirection::North => BlockOffset::NORTH,
        ProtocolDirection::South => BlockOffset::SOUTH,
        ProtocolDirection::West => BlockOffset::WEST,
        ProtocolDirection::East => BlockOffset::EAST,
    }
}

fn offset(pos: BlockPos, direction: BlockOffset, distance: i32) -> BlockPos {
    BlockPos {
        x: pos.x.saturating_add(direction.x.saturating_mul(distance)),
        y: pos.y.saturating_add(direction.y.saturating_mul(distance)),
        z: pos.z.saturating_add(direction.z.saturating_mul(distance)),
    }
}
