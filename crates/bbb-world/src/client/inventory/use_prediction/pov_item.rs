use std::collections::BTreeMap;

use super::*;
use crate::{LocalPlayerPoseState, TerrainFluidKind};
use bbb_protocol::{
    attribute_types::{vanilla_attribute_type, VANILLA_ATTRIBUTE_BLOCK_INTERACTION_RANGE_ID},
    block_types::{
        vanilla_block_type, vanilla_block_type_for_resource_id, VanillaCanSurviveOverride,
        VANILLA_BLOCK_FROGSPAWN_ID, VANILLA_BLOCK_LILY_PAD_ID,
    },
    data_component_types::VANILLA_DATA_COMPONENT_POTION_CONTENTS_TYPE_ID,
    entity_types::vanilla_entity_type_for_resource_id,
    item_types::{
        ItemTypeCatalogEntry, VANILLA_ITEM_FROGSPAWN_ID, VANILLA_ITEM_GLASS_BOTTLE_ID,
        VANILLA_ITEM_LILY_PAD_ID, VANILLA_ITEM_POTION_ID,
    },
    packets::{
        BlockHitResult as ProtocolBlockHitResult,
        DataComponentPatchSummary as ProtocolDataComponentPatchSummary,
        Direction as ProtocolDirection, ItemStackSummary as ProtocolItemStackSummary,
        Vec3d as ProtocolVec3d,
    },
    potion_types::vanilla_potion_for_resource_id,
};

const SUPPORTS_LILY_PAD_TAG: &str = "minecraft:supports_lily_pad";
const SUPPORTS_FROGSPAWN_TAG: &str = "minecraft:supports_frogspawn";
const LILY_PAD_PLACE_SOUND: &str = "minecraft:block.lily_pad.place";
const FROGSPAWN_PLACE_SOUND: &str = "minecraft:block.frogspawn.place";
const BLOCK_PLACE_VOLUME: f32 = 1.0;
const BLOCK_PLACE_PITCH: f32 = 0.8;

#[derive(Debug, Clone, Copy)]
struct PlaceOnWaterBlockFacts {
    block_type_id: i32,
    can_survive_override: VanillaCanSurviveOverride,
    support_tag: &'static str,
    place_sound: &'static str,
    collision_bounds: Option<([f64; 3], [f64; 3])>,
}

/// Fluid shape selected by Vanilla's `Item.getPlayerPOVHitResult`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LocalItemUsePovFluidMode {
    None,
    SourceOnly,
    Any,
}

/// Native raycast inputs for one source-derived no-target `Item.use` owner.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LocalItemUsePovQuery {
    pub fluid_mode: LocalItemUsePovFluidMode,
    pub max_distance: f64,
}

/// Result of the native `OUTLINE + Fluid` player-eye raycast.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LocalItemUsePovResult {
    Miss,
    Block {
        hit: ProtocolBlockHitResult,
        location: ProtocolVec3d,
    },
}

impl WorldStore {
    /// Returns the player-eye raycast contract required by the held item's
    /// source-derived `Item.use` override.
    pub fn local_item_use_pov_query(&self, hand: InteractionHand) -> Option<LocalItemUsePovQuery> {
        let item_id = self.local_item_in_hand(hand)?.item_id?;
        let item_type = vanilla_item_type(item_id)?;
        let fluid_mode = match item_type.use_override? {
            VanillaItemUseOverride::BoatItem => LocalItemUsePovFluidMode::Any,
            VanillaItemUseOverride::BottleItem
            | VanillaItemUseOverride::PlaceOnWaterBlockItem
            | VanillaItemUseOverride::SpawnEggItem => LocalItemUsePovFluidMode::SourceOnly,
            VanillaItemUseOverride::BucketItem if item_id == VANILLA_ITEM_BUCKET_ID => {
                LocalItemUsePovFluidMode::SourceOnly
            }
            VanillaItemUseOverride::BucketItem | VanillaItemUseOverride::EnderEyeItem => {
                LocalItemUsePovFluidMode::None
            }
            _ => return None,
        };
        let default_range = vanilla_attribute_type(VANILLA_ATTRIBUTE_BLOCK_INTERACTION_RANGE_ID)
            .map(|attribute| f64::from_bits(attribute.default_value_bits))
            .expect("generated block interaction range attribute");
        let max_distance = self
            .local_player_id()
            .and_then(|player_id| {
                self.entities
                    .attribute_value(player_id, VANILLA_ATTRIBUTE_BLOCK_INTERACTION_RANGE_ID)
            })
            .unwrap_or(default_range)
            .max(0.0);
        Some(LocalItemUsePovQuery {
            fluid_mode,
            max_distance,
        })
    }

    pub(super) fn predict_local_ender_eye_item_use(
        &self,
        pov: Option<LocalItemUsePovResult>,
    ) -> LocalItemUsePrediction {
        let Some(pov) = pov else {
            return LocalItemUsePrediction::UnsupportedOverride(
                VanillaItemUseOverride::EnderEyeItem,
            );
        };
        if let LocalItemUsePovResult::Block { hit, .. } = pov {
            if self
                .probe_block(BlockPos {
                    x: hit.pos.x,
                    y: hit.pos.y,
                    z: hit.pos.z,
                })
                .and_then(|block| block.block_name)
                .as_deref()
                == Some("minecraft:end_portal_frame")
            {
                return LocalItemUsePrediction::Result(LocalInteractionResult::Pass);
            }
        }
        LocalItemUsePrediction::Result(LocalInteractionResult::Success(LocalInteractionSuccess {
            starts_using_item: true,
            ..LocalInteractionSuccess::server_item_interaction()
        }))
    }

    pub(super) fn predict_local_spawn_egg_item_use(
        &self,
        pov: Option<LocalItemUsePovResult>,
    ) -> LocalItemUsePrediction {
        let Some(pov) = pov else {
            return LocalItemUsePrediction::UnsupportedOverride(
                VanillaItemUseOverride::SpawnEggItem,
            );
        };
        match pov {
            LocalItemUsePovResult::Miss => {
                LocalItemUsePrediction::Result(LocalInteractionResult::Pass)
            }
            LocalItemUsePovResult::Block { .. } => LocalItemUsePrediction::Result(
                LocalInteractionResult::Success(LocalInteractionSuccess::client_item_interaction()),
            ),
        }
    }

    pub(super) fn predict_local_place_on_water_block_item_use(
        &mut self,
        hand: InteractionHand,
        item_type: &ItemTypeCatalogEntry,
        stack: &ProtocolItemStackSummary,
        pov: Option<LocalItemUsePovResult>,
        sequence: Option<i32>,
    ) -> LocalItemUsePrediction {
        let unresolved_override = || {
            LocalItemUsePrediction::UnsupportedOverride(
                VanillaItemUseOverride::PlaceOnWaterBlockItem,
            )
        };
        let Some(facts) = place_on_water_block_facts(item_type.id) else {
            return unresolved_override();
        };
        if stack.item_id != Some(item_type.id) || stack.count < 1 {
            return unresolved_override();
        }
        let Some(pov) = pov else {
            return unresolved_override();
        };
        let LocalItemUsePovResult::Block { hit, .. } = pov else {
            // The current POV boundary intentionally carries no miss position
            // or direction, both of which `BlockHitResult.withPosition`
            // preserves. Keep that rare placement edge explicit.
            return unresolved_override();
        };
        let Some(sequence) = sequence else {
            return unresolved_override();
        };

        // `PlaceOnWaterBlockItem.use` shifts only the raycast block position
        // upward, then `BlockPlaceContext` resolves replace-clicked versus the
        // hit direction's relative position.
        let shifted_pos = above(place_on_water_block_pos(hit.pos));
        let Some(replace_shifted) = self.local_block_is_replaceable_by_different_item(shifted_pos)
        else {
            return unresolved_override();
        };
        let target_pos = if replace_shifted {
            shifted_pos
        } else {
            place_on_water_relative(shifted_pos, hit.direction)
        };
        if !replace_shifted {
            let Some(replace_target) =
                self.local_block_is_replaceable_by_different_item(target_pos)
            else {
                return unresolved_override();
            };
            if !replace_target {
                return LocalItemUsePrediction::Result(LocalInteractionResult::Fail);
            }
        }
        if !self.is_feature_enabled("minecraft:vanilla") {
            return LocalItemUsePrediction::Result(LocalInteractionResult::Fail);
        }

        let Some(block_type) = vanilla_block_type(facts.block_type_id).filter(|block_type| {
            block_type.can_survive_override == Some(facts.can_survive_override)
        }) else {
            return unresolved_override();
        };
        let Some(placement_state_id) = self
            .registries()
            .block_state_id_by_name_and_properties(block_type.resource_id, &BTreeMap::new())
        else {
            return unresolved_override();
        };
        match self.local_place_on_water_block_can_survive(target_pos, facts.support_tag) {
            Ok(false) => return LocalItemUsePrediction::Result(LocalInteractionResult::Fail),
            Ok(true) => {}
            Err(prediction) => return prediction,
        }

        if let Some((local_min, local_max)) = facts.collision_bounds {
            let origin = [
                f64::from(target_pos.x),
                f64::from(target_pos.y),
                f64::from(target_pos.z),
            ];
            let query_min = std::array::from_fn(|axis| origin[axis] + local_min[axis]);
            let query_max = std::array::from_fn(|axis| origin[axis] + local_max[axis]);
            match self.local_block_placement_is_unobstructed(query_min, query_max) {
                Some(true) => {}
                Some(false) => {
                    return LocalItemUsePrediction::Result(LocalInteractionResult::Fail);
                }
                None => return unresolved_override(),
            }
        }

        if !self.predict_local_block_state(target_pos, placement_state_id, sequence)
            || !self.consume_local_item_in_hand(hand, 1)
        {
            return unresolved_override();
        }
        LocalItemUsePrediction::Result(LocalInteractionResult::Success(
            LocalInteractionSuccess::block_place(
                target_pos,
                facts.place_sound,
                BLOCK_PLACE_VOLUME,
                BLOCK_PLACE_PITCH,
            ),
        ))
    }

    fn local_place_on_water_block_can_survive(
        &self,
        pos: BlockPos,
        support_tag: &'static str,
    ) -> Result<bool, LocalItemUsePrediction> {
        let Some(fluid_supports) = self.registry_holder_ids_for_tag("minecraft:fluid", support_tag)
        else {
            return Err(LocalItemUsePrediction::MissingRegistryTag(support_tag));
        };
        let Some(block_supports) = self.registry_holder_ids_for_tag("minecraft:block", support_tag)
        else {
            return Err(LocalItemUsePrediction::MissingRegistryTag(support_tag));
        };
        let Some(placed_at) = self.probe_block(pos) else {
            return Err(LocalItemUsePrediction::UnsupportedOverride(
                VanillaItemUseOverride::PlaceOnWaterBlockItem,
            ));
        };
        let below_pos = BlockPos {
            y: pos.y.saturating_sub(1),
            ..pos
        };
        let Some(below) = self.probe_block(below_pos) else {
            return Err(LocalItemUsePrediction::UnsupportedOverride(
                VanillaItemUseOverride::PlaceOnWaterBlockItem,
            ));
        };
        let fluid_resource_id = match below.fluid {
            None => "minecraft:empty",
            Some(fluid) => match (fluid.kind, fluid.amount == 8 && !fluid.falling) {
                (TerrainFluidKind::Water, true) => "minecraft:water",
                (TerrainFluidKind::Water, false) => "minecraft:flowing_water",
                (TerrainFluidKind::Lava, true) => "minecraft:lava",
                (TerrainFluidKind::Lava, false) => "minecraft:flowing_lava",
            },
        };
        let Some(fluid_holder_id) =
            self.registry_holder_id_by_resource_id("minecraft:fluid", fluid_resource_id)
        else {
            return Err(LocalItemUsePrediction::UnsupportedOverride(
                VanillaItemUseOverride::PlaceOnWaterBlockItem,
            ));
        };
        let Some(below_block_name) = below.block_name.as_deref() else {
            return Err(LocalItemUsePrediction::UnsupportedOverride(
                VanillaItemUseOverride::PlaceOnWaterBlockItem,
            ));
        };
        let Some(below_block_type) = vanilla_block_type_for_resource_id(below_block_name) else {
            return Err(LocalItemUsePrediction::UnsupportedOverride(
                VanillaItemUseOverride::PlaceOnWaterBlockItem,
            ));
        };
        Ok((fluid_supports.contains(&fluid_holder_id)
            || block_supports.contains(&below_block_type.id))
            && placed_at.fluid.is_none())
    }

    pub(super) fn predict_local_boat_item_use(
        &self,
        item_type: &ItemTypeCatalogEntry,
        pov: Option<LocalItemUsePovResult>,
    ) -> LocalItemUsePrediction {
        let Some(pov) = pov else {
            return LocalItemUsePrediction::UnsupportedOverride(VanillaItemUseOverride::BoatItem);
        };
        let LocalItemUsePovResult::Block { location, .. } = pov else {
            return LocalItemUsePrediction::Result(LocalInteractionResult::Pass);
        };
        let Some(player_pose) = self.local_player_pose() else {
            return LocalItemUsePrediction::UnsupportedOverride(VanillaItemUseOverride::BoatItem);
        };
        if self.local_boat_use_eye_is_obstructed(player_pose) {
            return LocalItemUsePrediction::Result(LocalInteractionResult::Pass);
        }

        let Some(entity_type) = vanilla_entity_type_for_resource_id(item_type.resource_id)
            .filter(|entity_type| crate::entities::is_vanilla_boat_type(entity_type.id))
        else {
            return LocalItemUsePrediction::UnsupportedOverride(VanillaItemUseOverride::BoatItem);
        };
        let Some(bounds) = vanilla_bounds_for_type(entity_type.id) else {
            return LocalItemUsePrediction::UnsupportedOverride(VanillaItemUseOverride::BoatItem);
        };
        let position = [location.x, location.y, location.z];
        let collision_targets = self.entities.vehicle_entity_collision_targets();
        if !self.prospective_entity_collision_bounds_are_free(
            entity_type.id,
            position,
            bounds.min.map(f64::from),
            bounds.max.map(f64::from),
            false,
            &collision_targets,
        ) {
            return LocalItemUsePrediction::Result(LocalInteractionResult::Fail);
        }

        LocalItemUsePrediction::Result(LocalInteractionResult::Success(
            LocalInteractionSuccess::client_item_interaction(),
        ))
    }

    pub(super) fn predict_local_bottle_item_use(
        &mut self,
        hand: InteractionHand,
        stack: &ProtocolItemStackSummary,
        pov: Option<LocalItemUsePovResult>,
    ) -> LocalItemUsePrediction {
        // `BottleItem` checks dragon-owned AreaEffectClouds first, but the
        // cloud's EntityReference owner is absent from both AddEntity and
        // synchronized metadata. The remote Vanilla client therefore cannot
        // enter that server-owned branch and proceeds to the POV water check.
        let Some(pov) = pov else {
            return LocalItemUsePrediction::UnsupportedOverride(VanillaItemUseOverride::BottleItem);
        };
        let LocalItemUsePovResult::Block { hit, .. } = pov else {
            return LocalItemUsePrediction::Result(LocalInteractionResult::Pass);
        };
        let pos = BlockPos {
            x: hit.pos.x,
            y: hit.pos.y,
            z: hit.pos.z,
        };
        let Some(block) = self.probe_block(pos) else {
            return LocalItemUsePrediction::UnsupportedOverride(VanillaItemUseOverride::BottleItem);
        };
        if !block
            .fluid
            .is_some_and(|fluid| fluid.kind == TerrainFluidKind::Water)
        {
            return LocalItemUsePrediction::Result(LocalInteractionResult::Pass);
        }
        if stack.item_id != Some(VANILLA_ITEM_GLASS_BOTTLE_ID) || stack.count < 1 {
            return LocalItemUsePrediction::UnsupportedOverride(VanillaItemUseOverride::BottleItem);
        }
        let Some(water) = vanilla_potion_for_resource_id("minecraft:water") else {
            return LocalItemUsePrediction::UnsupportedOverride(VanillaItemUseOverride::BottleItem);
        };
        let water_potion = ProtocolItemStackSummary {
            item_id: Some(VANILLA_ITEM_POTION_ID),
            count: 1,
            component_patch: ProtocolDataComponentPatchSummary {
                added: 1,
                added_type_ids: vec![VANILLA_DATA_COMPONENT_POTION_CONTENTS_TYPE_ID],
                potion_id: Some(water.id),
                ..ProtocolDataComponentPatchSummary::default()
            },
        };
        let Some(dropped) = self.replace_local_item_with_filled_result(
            hand,
            VANILLA_ITEM_GLASS_BOTTLE_ID,
            water_potion,
        ) else {
            return LocalItemUsePrediction::UnsupportedOverride(VanillaItemUseOverride::BottleItem);
        };
        LocalItemUsePrediction::Result(LocalInteractionResult::Success(
            LocalInteractionSuccess::bottle_fill(dropped),
        ))
    }

    fn local_boat_use_eye_is_obstructed(&self, player_pose: LocalPlayerPoseState) -> bool {
        let view = crate::entities::calculate_view_vector(player_pose.x_rot, player_pose.y_rot);
        let delta = [view.x * 5.0, view.y * 5.0, view.z * 5.0];
        let position = [
            player_pose.position.x,
            player_pose.position.y,
            player_pose.position.z,
        ];
        let mut search_min = [position[0] - 0.3, position[1], position[2] - 0.3];
        let mut search_max = [
            position[0] + 0.3,
            position[1] + player_pose.body_height(),
            position[2] + 0.3,
        ];
        for axis in 0..3 {
            if delta[axis] < 0.0 {
                search_min[axis] += delta[axis];
            } else {
                search_max[axis] += delta[axis];
            }
            search_min[axis] -= 1.0;
            search_max[axis] += 1.0;
        }
        let eye = [
            position[0],
            position[1] + player_pose.eye_height(),
            position[2],
        ];
        let local_player_id = self.local_player_id();

        self.entity_pick_targets().into_iter().any(|target| {
            if local_player_id == Some(target.entity_id) {
                return false;
            }
            let pick_radius = f64::from(target.bounds.pick_radius);
            let target_position = [target.position.x, target.position.y, target.position.z];
            let target_min = std::array::from_fn(|axis| {
                target_position[axis] + f64::from(target.bounds.min[axis]) - pick_radius
            });
            let target_max = std::array::from_fn(|axis| {
                target_position[axis] + f64::from(target.bounds.max[axis]) + pick_radius
            });
            aabbs_intersect(search_min, search_max, target_min, target_max)
                && point_is_inside_aabb(eye, target_min, target_max)
        })
    }
}

fn place_on_water_block_facts(item_id: i32) -> Option<PlaceOnWaterBlockFacts> {
    match item_id {
        VANILLA_ITEM_LILY_PAD_ID => Some(PlaceOnWaterBlockFacts {
            block_type_id: VANILLA_BLOCK_LILY_PAD_ID,
            can_survive_override: VanillaCanSurviveOverride::VegetationBlock,
            support_tag: SUPPORTS_LILY_PAD_TAG,
            place_sound: LILY_PAD_PLACE_SOUND,
            // `LilyPadBlock.SHAPE = Block.column(14, 0, 1.5)`.
            collision_bounds: Some((
                [1.0 / 16.0, 0.0, 1.0 / 16.0],
                [15.0 / 16.0, 1.5 / 16.0, 15.0 / 16.0],
            )),
        }),
        VANILLA_ITEM_FROGSPAWN_ID => Some(PlaceOnWaterBlockFacts {
            block_type_id: VANILLA_BLOCK_FROGSPAWN_ID,
            can_survive_override: VanillaCanSurviveOverride::FrogspawnBlock,
            support_tag: SUPPORTS_FROGSPAWN_TAG,
            place_sound: FROGSPAWN_PLACE_SOUND,
            // `FROGSPAWN` uses `Properties.noCollision()`.
            collision_bounds: None,
        }),
        _ => None,
    }
}

fn place_on_water_block_pos(pos: bbb_protocol::packets::BlockPos) -> BlockPos {
    BlockPos {
        x: pos.x,
        y: pos.y,
        z: pos.z,
    }
}

fn place_on_water_relative(pos: BlockPos, direction: ProtocolDirection) -> BlockPos {
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

fn aabbs_intersect(
    left_min: [f64; 3],
    left_max: [f64; 3],
    right_min: [f64; 3],
    right_max: [f64; 3],
) -> bool {
    (0..3).all(|axis| left_min[axis] < right_max[axis] && left_max[axis] > right_min[axis])
}

fn point_is_inside_aabb(point: [f64; 3], min: [f64; 3], max: [f64; 3]) -> bool {
    (0..3).all(|axis| point[axis] >= min[axis] && point[axis] < max[axis])
}
