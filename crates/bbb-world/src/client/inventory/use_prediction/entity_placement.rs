use super::*;
use crate::entities::{
    hanging_facing, prospective_item_frame_geometry, prospective_painting_geometry,
    vanilla_entity_metadata_hanging_direction_accessor_for_entity_type, ItemFrameFacing,
};
use crate::registries::FrozenRegistryMembership;
use bbb_protocol::{
    block_state_types::vanilla_block_state_support_facts,
    block_types::VanillaCanSurviveOverride,
    data_component_types::VANILLA_DATA_COMPONENT_PAINTING_VARIANT_TYPE_ID,
    entity_types::{
        VANILLA_ENTITY_TYPE_ARMOR_STAND_ID, VANILLA_ENTITY_TYPE_GLOW_ITEM_FRAME_ID,
        VANILLA_ENTITY_TYPE_ITEM_FRAME_ID, VANILLA_ENTITY_TYPE_PAINTING_ID,
    },
    item_types::{
        VANILLA_ITEM_ARMOR_STAND_ID, VANILLA_ITEM_GLOW_ITEM_FRAME_ID, VANILLA_ITEM_ITEM_FRAME_ID,
        VANILLA_ITEM_PAINTING_ID,
    },
};

const PAINTING_VARIANT_REGISTRY: &str = "minecraft:painting_variant";
const PLACEABLE_PAINTING_VARIANT_TAG: &str = "minecraft:placeable";
const HANGING_SUPPORT_DEFLATE: f64 = 1.0e-7;

#[derive(Debug, Clone, Copy)]
struct HangingPlacementGeometry {
    position: [f64; 3],
    min: [f64; 3],
    max: [f64; 3],
}

impl WorldStore {
    pub(super) fn predict_local_armor_stand_item_use_on(
        &mut self,
        owner: VanillaItemUseOnOverride,
        hand: InteractionHand,
        context: LocalBlockUseContext,
    ) -> LocalTargetUsePrediction {
        let unresolved_owner = || LocalTargetUsePrediction::UnsupportedItemUseOn(owner);
        let Some(stack) = self.local_item_in_hand(hand) else {
            return unresolved_owner();
        };
        if stack.item_id != Some(VANILLA_ITEM_ARMOR_STAND_ID) || stack.count < 1 {
            return unresolved_owner();
        }
        if context.face == ProtocolDirection::Down {
            return LocalTargetUsePrediction::Result(LocalInteractionResult::Fail);
        }

        let Some(replace_clicked) = self.local_block_is_replaceable_by_different_item(context.pos)
        else {
            return unresolved_owner();
        };
        let target_pos = if replace_clicked {
            context.pos
        } else {
            relative(context.pos, context.face)
        };
        let Some(bounds) = vanilla_bounds_for_type(VANILLA_ENTITY_TYPE_ARMOR_STAND_ID) else {
            return unresolved_owner();
        };
        let origin = [
            f64::from(target_pos.x) + 0.5,
            f64::from(target_pos.y),
            f64::from(target_pos.z) + 0.5,
        ];
        let query_min = std::array::from_fn(|axis| origin[axis] + f64::from(bounds.min[axis]));
        let query_max = std::array::from_fn(|axis| origin[axis] + f64::from(bounds.max[axis]));

        match self.local_empty_context_block_bounds_are_free(query_min, query_max) {
            Some(true) => {}
            Some(false) => {
                return LocalTargetUsePrediction::Result(LocalInteractionResult::Fail);
            }
            None => return unresolved_owner(),
        }
        match self.local_non_spectator_entity_bounds_are_empty(query_min, query_max) {
            Some(true) => {}
            Some(false) => {
                return LocalTargetUsePrediction::Result(LocalInteractionResult::Fail);
            }
            None => return unresolved_owner(),
        }
        // `MultiPlayerGameMode.performUseItemOn` restores the pre-call count
        // after creative `ItemStack.useOn`, despite `ArmorStandItem` itself
        // calling `shrink`.
        if !self.consume_local_item_in_hand(hand, 1) {
            return unresolved_owner();
        }
        LocalTargetUsePrediction::Result(LocalInteractionResult::Success(
            LocalInteractionSuccess::client_item_interaction(),
        ))
    }

    pub(super) fn predict_local_hanging_entity_item_use_on(
        &mut self,
        owner: VanillaItemUseOnOverride,
        hand: InteractionHand,
        context: LocalBlockUseContext,
    ) -> LocalTargetUsePrediction {
        let unresolved_owner = || LocalTargetUsePrediction::UnsupportedItemUseOn(owner);
        let Some(stack) = self.local_item_in_hand(hand).cloned() else {
            return unresolved_owner();
        };
        if stack.count < 1 {
            return unresolved_owner();
        }
        let Some(entity_type_id) = hanging_entity_type_for_item(stack.item_id) else {
            return unresolved_owner();
        };
        let facing = item_frame_facing_from_protocol(context.face);
        let target_pos = relative(context.pos, context.face);

        if entity_type_id == VANILLA_ENTITY_TYPE_PAINTING_ID
            && matches!(facing, ItemFrameFacing::Down | ItemFrameFacing::Up)
        {
            return LocalTargetUsePrediction::Result(LocalInteractionResult::Fail);
        }
        if matches!(
            entity_type_id,
            VANILLA_ENTITY_TYPE_ITEM_FRAME_ID | VANILLA_ENTITY_TYPE_GLOW_ITEM_FRAME_ID
        ) && !self.hanging_target_is_inside_build_height(target_pos)
        {
            return LocalTargetUsePrediction::Result(LocalInteractionResult::Fail);
        }

        let survives = if entity_type_id == VANILLA_ENTITY_TYPE_PAINTING_ID {
            let placeable = match self.registry_holder_ids_for_tag(
                PAINTING_VARIANT_REGISTRY,
                PLACEABLE_PAINTING_VARIANT_TAG,
            ) {
                Some(holder_ids) => holder_ids.to_vec(),
                None if self
                    .frozen_registry_access()
                    .tag_membership(PAINTING_VARIANT_REGISTRY, PLACEABLE_PAINTING_VARIANT_TAG)
                    == FrozenRegistryMembership::Absent =>
                {
                    Vec::new()
                }
                None => return unresolved_owner(),
            };
            let mut any_placeable_survives = false;
            for holder_id in placeable {
                let Ok(Some(variant)) = self.painting_variant_client_projection(holder_id) else {
                    return unresolved_owner();
                };
                let Some(geometry) =
                    hanging_painting_geometry(target_pos, facing, variant.width, variant.height)
                else {
                    return unresolved_owner();
                };
                match self.hanging_geometry_survives(entity_type_id, facing, geometry) {
                    Some(true) => {
                        any_placeable_survives = true;
                        break;
                    }
                    Some(false) => {}
                    None => return unresolved_owner(),
                }
            }
            if !any_placeable_survives {
                false
            } else {
                let Some(forced_variant) = self.local_painting_item_variant_dimensions(&stack)
                else {
                    return unresolved_owner();
                };
                match forced_variant {
                    None => true,
                    Some((width, height)) => {
                        let Some(geometry) =
                            hanging_painting_geometry(target_pos, facing, width, height)
                        else {
                            return unresolved_owner();
                        };
                        let Some(survives) =
                            self.hanging_geometry_survives(entity_type_id, facing, geometry)
                        else {
                            return unresolved_owner();
                        };
                        survives
                    }
                }
            }
        } else {
            let geometry = hanging_geometry(prospective_item_frame_geometry(target_pos, facing));
            let Some(survives) = self.hanging_geometry_survives(entity_type_id, facing, geometry)
            else {
                return unresolved_owner();
            };
            survives
        };

        if !survives {
            return LocalTargetUsePrediction::Result(LocalInteractionResult::Success(
                LocalInteractionSuccess::consume(false),
            ));
        }
        // The item method calls `shrink`, but creative
        // `MultiPlayerGameMode.performUseItemOn` restores the original count.
        if !self.consume_local_item_in_hand(hand, 1) {
            return unresolved_owner();
        }
        LocalTargetUsePrediction::Result(LocalInteractionResult::Success(
            LocalInteractionSuccess::client_item_interaction(),
        ))
    }

    fn hanging_target_is_inside_build_height(&self, pos: BlockPos) -> bool {
        let dimension = self.dimension();
        pos.y >= dimension.min_y && pos.y < dimension.min_y.saturating_add(dimension.height)
    }

    /// `Some(None)` is an item without an effective painting-variant
    /// component; `Some(Some(..))` is its exact referenced/direct dimensions.
    /// `None` keeps malformed or unresolved component state explicit.
    fn local_painting_item_variant_dimensions(
        &self,
        stack: &bbb_protocol::packets::ItemStackSummary,
    ) -> Option<Option<(i32, i32)>> {
        if !self.item_stack_has_effective_component(
            stack,
            VANILLA_DATA_COMPONENT_PAINTING_VARIANT_TYPE_ID,
        ) {
            return Some(None);
        }
        let effective = self.effective_item_stack(stack);
        let patch = &effective.component_patch;
        match (
            patch.painting_variant_id,
            patch.painting_variant_direct.as_ref(),
        ) {
            (Some(holder_id), None) => self
                .painting_variant_client_projection(holder_id)
                .ok()
                .flatten()
                .map(|variant| Some((variant.width, variant.height))),
            (None, Some(variant)) if valid_painting_dimensions(variant.width, variant.height) => {
                Some(Some((variant.width, variant.height)))
            }
            _ => None,
        }
    }

    fn hanging_geometry_survives(
        &self,
        entity_type_id: i32,
        facing: ItemFrameFacing,
        geometry: HangingPlacementGeometry,
    ) -> Option<bool> {
        if !self.local_non_living_entity_block_and_border_bounds_are_free(
            entity_type_id,
            geometry.position,
            geometry.min,
            geometry.max,
        )? {
            return Some(false);
        }
        if !self.hanging_geometry_has_support(entity_type_id, facing, geometry)? {
            return Some(false);
        }

        let candidate_is_painting = entity_type_id == VANILLA_ENTITY_TYPE_PAINTING_ID;
        for target in self.entity_pick_targets() {
            let existing = self.probe_entity(target.entity_id)?;
            if !is_hanging_entity_type(existing.entity_type_id) {
                continue;
            }
            if existing.entity_type_id == VANILLA_ENTITY_TYPE_PAINTING_ID
                && existing.painting_variant_projection.is_none()
            {
                return None;
            }
            let accessor = vanilla_entity_metadata_hanging_direction_accessor_for_entity_type(
                existing.entity_type_id,
            )?;
            let existing_facing = hanging_facing(accessor, existing.data, &existing.data_values);
            let blocks_candidate = existing_facing == facing
                || candidate_is_painting
                    && existing.entity_type_id == VANILLA_ENTITY_TYPE_PAINTING_ID;
            if blocks_candidate
                && absolute_bounds_intersect(
                    geometry.min,
                    geometry.max,
                    target.position,
                    target.bounds,
                )
            {
                return Some(false);
            }
        }
        Some(true)
    }

    fn hanging_geometry_has_support(
        &self,
        entity_type_id: i32,
        facing: ItemFrameFacing,
        geometry: HangingPlacementGeometry,
    ) -> Option<bool> {
        let step = item_frame_facing_step(facing);
        let mut support_min: [f64; 3] = std::array::from_fn(|axis| {
            geometry.min[axis] - f64::from(step[axis]) * 0.5 + HANGING_SUPPORT_DEFLATE
        });
        let mut support_max: [f64; 3] = std::array::from_fn(|axis| {
            geometry.max[axis] - f64::from(step[axis]) * 0.5 - HANGING_SUPPORT_DEFLATE
        });
        for axis in 0..3 {
            if support_min[axis] > support_max[axis] {
                std::mem::swap(&mut support_min[axis], &mut support_max[axis]);
            }
        }
        let min = support_min.map(|coordinate| coordinate.floor() as i32);
        let max = support_max.map(|coordinate| coordinate.floor() as i32);
        let diode_supports = entity_type_id == VANILLA_ENTITY_TYPE_PAINTING_ID
            || !matches!(facing, ItemFrameFacing::Down | ItemFrameFacing::Up);
        for y in min[1]..=max[1] {
            for z in min[2]..=max[2] {
                for x in min[0]..=max[0] {
                    let Some(block) = self.probe_block(BlockPos { x, y, z }) else {
                        return Some(false);
                    };
                    let facts = vanilla_block_state_support_facts(block.block_state_id)?;
                    let block_type =
                        vanilla_block_type_for_resource_id(block.block_name.as_deref()?)?;
                    if !facts.legacy_solid()
                        && !(diode_supports
                            && block_type.can_survive_override
                                == Some(VanillaCanSurviveOverride::DiodeBlock))
                    {
                        return Some(false);
                    }
                }
            }
        }
        Some(true)
    }
}

fn hanging_entity_type_for_item(item_id: Option<i32>) -> Option<i32> {
    match item_id? {
        VANILLA_ITEM_PAINTING_ID => Some(VANILLA_ENTITY_TYPE_PAINTING_ID),
        VANILLA_ITEM_ITEM_FRAME_ID => Some(VANILLA_ENTITY_TYPE_ITEM_FRAME_ID),
        VANILLA_ITEM_GLOW_ITEM_FRAME_ID => Some(VANILLA_ENTITY_TYPE_GLOW_ITEM_FRAME_ID),
        _ => None,
    }
}

fn is_hanging_entity_type(entity_type_id: i32) -> bool {
    matches!(
        entity_type_id,
        VANILLA_ENTITY_TYPE_PAINTING_ID
            | VANILLA_ENTITY_TYPE_ITEM_FRAME_ID
            | VANILLA_ENTITY_TYPE_GLOW_ITEM_FRAME_ID
    )
}

fn hanging_painting_geometry(
    pos: BlockPos,
    facing: ItemFrameFacing,
    width: i32,
    height: i32,
) -> Option<HangingPlacementGeometry> {
    prospective_painting_geometry(pos, facing, width, height).map(hanging_geometry)
}

fn hanging_geometry(
    (position, bounds): (crate::EntityVec3, crate::EntityPickBoundsState),
) -> HangingPlacementGeometry {
    let position = [position.x, position.y, position.z];
    HangingPlacementGeometry {
        position,
        min: std::array::from_fn(|axis| position[axis] + f64::from(bounds.min[axis])),
        max: std::array::from_fn(|axis| position[axis] + f64::from(bounds.max[axis])),
    }
}

fn absolute_bounds_intersect(
    query_min: [f64; 3],
    query_max: [f64; 3],
    position: crate::EntityVec3,
    bounds: crate::EntityPickBoundsState,
) -> bool {
    let position = [position.x, position.y, position.z];
    (0..3).all(|axis| {
        position[axis] + f64::from(bounds.min[axis]) < query_max[axis]
            && position[axis] + f64::from(bounds.max[axis]) > query_min[axis]
    })
}

fn valid_painting_dimensions(width: i32, height: i32) -> bool {
    (1..=16).contains(&width) && (1..=16).contains(&height)
}

fn item_frame_facing_from_protocol(direction: ProtocolDirection) -> ItemFrameFacing {
    match direction {
        ProtocolDirection::Down => ItemFrameFacing::Down,
        ProtocolDirection::Up => ItemFrameFacing::Up,
        ProtocolDirection::North => ItemFrameFacing::North,
        ProtocolDirection::South => ItemFrameFacing::South,
        ProtocolDirection::West => ItemFrameFacing::West,
        ProtocolDirection::East => ItemFrameFacing::East,
    }
}

fn item_frame_facing_step(facing: ItemFrameFacing) -> [i32; 3] {
    match facing {
        ItemFrameFacing::Down => [0, -1, 0],
        ItemFrameFacing::Up => [0, 1, 0],
        ItemFrameFacing::North => [0, 0, -1],
        ItemFrameFacing::South => [0, 0, 1],
        ItemFrameFacing::West => [-1, 0, 0],
        ItemFrameFacing::East => [1, 0, 0],
    }
}

fn relative(pos: BlockPos, direction: ProtocolDirection) -> BlockPos {
    match direction {
        ProtocolDirection::Down => BlockPos {
            y: pos.y.saturating_sub(1),
            ..pos
        },
        ProtocolDirection::Up => BlockPos {
            y: pos.y.saturating_add(1),
            ..pos
        },
        ProtocolDirection::North => BlockPos {
            z: pos.z.saturating_sub(1),
            ..pos
        },
        ProtocolDirection::South => BlockPos {
            z: pos.z.saturating_add(1),
            ..pos
        },
        ProtocolDirection::West => BlockPos {
            x: pos.x.saturating_sub(1),
            ..pos
        },
        ProtocolDirection::East => BlockPos {
            x: pos.x.saturating_add(1),
            ..pos
        },
    }
}
