use super::*;
use bbb_protocol::{
    attribute_types::{vanilla_attribute_type, VANILLA_ATTRIBUTE_BLOCK_INTERACTION_RANGE_ID},
    packets::BlockHitResult as ProtocolBlockHitResult,
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
    Block(ProtocolBlockHitResult),
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
        if let LocalItemUsePovResult::Block(hit) = pov {
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
            LocalItemUsePovResult::Block(_) => LocalItemUsePrediction::Result(
                LocalInteractionResult::Success(LocalInteractionSuccess::client_item_interaction()),
            ),
        }
    }
}
