use super::*;
use crate::{LocalPlayerPoseState, TerrainFluidKind};
use bbb_protocol::{
    attribute_types::{vanilla_attribute_type, VANILLA_ATTRIBUTE_BLOCK_INTERACTION_RANGE_ID},
    data_component_types::VANILLA_DATA_COMPONENT_POTION_CONTENTS_TYPE_ID,
    entity_types::vanilla_entity_type_for_resource_id,
    item_types::{ItemTypeCatalogEntry, VANILLA_ITEM_GLASS_BOTTLE_ID, VANILLA_ITEM_POTION_ID},
    packets::{
        BlockHitResult as ProtocolBlockHitResult,
        DataComponentPatchSummary as ProtocolDataComponentPatchSummary,
        ItemStackSummary as ProtocolItemStackSummary, Vec3d as ProtocolVec3d,
    },
    potion_types::vanilla_potion_for_resource_id,
};

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
