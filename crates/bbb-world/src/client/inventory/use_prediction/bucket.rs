use super::*;
use crate::{TerrainFluidKind, WorldStore};
use bbb_protocol::{
    block_state_types::vanilla_block_state_fluid_replaceable,
    block_types::{
        vanilla_block_type_for_resource_id, VanillaBucketPickupOverride,
        VanillaLiquidBlockContainerOverride, VANILLA_BLOCK_AIR_ID,
    },
    item_types::{
        ItemTypeCatalogEntry, VanillaBucketFluid, VanillaItemUseOverride, VANILLA_ITEM_BUCKET_ID,
        VANILLA_ITEM_LAVA_BUCKET_ID, VANILLA_ITEM_POWDER_SNOW_BUCKET_ID,
        VANILLA_ITEM_WATER_BUCKET_ID,
    },
    packets::{
        BlockHitResult as ProtocolBlockHitResult, ItemStackSummary as ProtocolItemStackSummary,
    },
};
use std::collections::BTreeMap;

const BUCKET_FILL: &str = "minecraft:item.bucket.fill";
const BUCKET_FILL_LAVA: &str = "minecraft:item.bucket.fill_lava";
const BUCKET_FILL_POWDER_SNOW: &str = "minecraft:item.bucket.fill_powder_snow";
const BUCKET_EMPTY: &str = "minecraft:item.bucket.empty";
const BUCKET_EMPTY_LAVA: &str = "minecraft:item.bucket.empty_lava";

#[derive(Debug, Clone, Copy)]
enum BucketEmptyOutcome {
    Sound {
        pos: BlockPos,
        sound_resource_id: &'static str,
        neutral_source: bool,
    },
    Evaporated {
        pos: BlockPos,
    },
}

#[derive(Debug, Clone, Copy)]
enum BucketEmptyAttempt {
    Success(BucketEmptyOutcome),
    Fail,
    Unsupported,
}

#[derive(Debug, Clone, Copy)]
enum BucketContainerPlacement {
    CanPlace,
    CannotPlace,
    Unsupported,
}

impl WorldStore {
    pub(super) fn predict_local_bucket_item_use(
        &mut self,
        hand: InteractionHand,
        item_type: &ItemTypeCatalogEntry,
        stack: &ProtocolItemStackSummary,
        pov: Option<LocalItemUsePovResult>,
        sequence: Option<i32>,
        secondary_use_active: bool,
    ) -> LocalItemUsePrediction {
        let unsupported =
            || LocalItemUsePrediction::UnsupportedOverride(VanillaItemUseOverride::BucketItem);
        let Some(behavior) = item_type.bucket_behavior else {
            return unsupported();
        };
        let Some(pov) = pov else {
            return unsupported();
        };
        let LocalItemUsePovResult::Block { hit, .. } = pov else {
            return LocalItemUsePrediction::Result(LocalInteractionResult::Pass);
        };
        let clicked_pos = block_pos(hit.pos);
        if self.probe_block(clicked_pos).is_none() {
            return unsupported();
        }
        if self.local_player_is_adventure()
            && !self.local_item_in_hand_can_place_on_block_in_adventure_mode(hand, clicked_pos)
        {
            return LocalItemUsePrediction::Result(LocalInteractionResult::Fail);
        }

        match behavior.fluid {
            VanillaBucketFluid::Empty => {
                self.predict_local_empty_bucket_use(hand, stack, clicked_pos, sequence)
            }
            VanillaBucketFluid::Water | VanillaBucketFluid::Lava => self
                .predict_local_filled_bucket_use(
                    hand,
                    item_type,
                    stack,
                    behavior.fluid,
                    behavior.mob_empty_sound_resource_id,
                    hit,
                    sequence,
                    secondary_use_active,
                ),
        }
    }

    fn predict_local_empty_bucket_use(
        &mut self,
        hand: InteractionHand,
        stack: &ProtocolItemStackSummary,
        pos: BlockPos,
        sequence: Option<i32>,
    ) -> LocalItemUsePrediction {
        let unsupported =
            || LocalItemUsePrediction::UnsupportedOverride(VanillaItemUseOverride::BucketItem);
        if stack.item_id != Some(VANILLA_ITEM_BUCKET_ID) || stack.count < 1 {
            return unsupported();
        }
        let Some(block) = self.probe_block(pos) else {
            return unsupported();
        };
        let Some(block_name) = block.block_name.as_deref() else {
            return unsupported();
        };
        let Some(block_type) = vanilla_block_type_for_resource_id(block_name) else {
            return unsupported();
        };
        let Some(owner) = block_type.bucket_pickup_override else {
            return LocalItemUsePrediction::Result(LocalInteractionResult::Fail);
        };
        let (filled_item_id, predicted_state_id, sound_resource_id) = match owner {
            VanillaBucketPickupOverride::BarrierBlock => {
                if !self.local_player_is_creative() {
                    return LocalItemUsePrediction::Result(LocalInteractionResult::Fail);
                }
                if block
                    .block_properties
                    .get("waterlogged")
                    .map(String::as_str)
                    != Some("true")
                {
                    return LocalItemUsePrediction::Result(LocalInteractionResult::Fail);
                }
                let Some(pickup) = self.waterlogged_bucket_pickup(&block) else {
                    return unsupported();
                };
                pickup
            }
            VanillaBucketPickupOverride::BubbleColumnBlock => {
                if !block
                    .fluid
                    .is_some_and(|fluid| fluid.kind == TerrainFluidKind::Water)
                {
                    return LocalItemUsePrediction::Result(LocalInteractionResult::Fail);
                }
                let Some(air_state_id) = self.bucket_air_block_state_id() else {
                    return unsupported();
                };
                (VANILLA_ITEM_WATER_BUCKET_ID, air_state_id, BUCKET_FILL)
            }
            VanillaBucketPickupOverride::LiquidBlock => {
                let Some(fluid) = block
                    .fluid
                    .filter(|fluid| fluid.amount == 8 && !fluid.falling)
                else {
                    return LocalItemUsePrediction::Result(LocalInteractionResult::Fail);
                };
                let (filled_item_id, sound_resource_id) = match fluid.kind {
                    TerrainFluidKind::Water => (VANILLA_ITEM_WATER_BUCKET_ID, BUCKET_FILL),
                    TerrainFluidKind::Lava => (VANILLA_ITEM_LAVA_BUCKET_ID, BUCKET_FILL_LAVA),
                };
                let Some(air_state_id) = self.bucket_air_block_state_id() else {
                    return unsupported();
                };
                (filled_item_id, air_state_id, sound_resource_id)
            }
            VanillaBucketPickupOverride::PowderSnowBlock => {
                let Some(air_state_id) = self.bucket_air_block_state_id() else {
                    return unsupported();
                };
                (
                    VANILLA_ITEM_POWDER_SNOW_BUCKET_ID,
                    air_state_id,
                    BUCKET_FILL_POWDER_SNOW,
                )
            }
            VanillaBucketPickupOverride::SimpleWaterloggedBlock => {
                if block
                    .block_properties
                    .get("waterlogged")
                    .map(String::as_str)
                    != Some("true")
                {
                    return LocalItemUsePrediction::Result(LocalInteractionResult::Fail);
                }
                if block_type.can_survive_override.is_some() {
                    return unsupported();
                }
                let Some(pickup) = self.waterlogged_bucket_pickup(&block) else {
                    return unsupported();
                };
                pickup
            }
        };
        let Some(sequence) = sequence else {
            return unsupported();
        };
        if !self.predict_local_block_state(pos, predicted_state_id, sequence) {
            return unsupported();
        }
        let Some(dropped) = self.replace_local_item_with_filled_result(
            hand,
            VANILLA_ITEM_BUCKET_ID,
            item_stack(filled_item_id),
        ) else {
            return unsupported();
        };
        bucket_success(
            dropped,
            LocalInteractionSoundAction::BucketFill { sound_resource_id },
            None,
        )
    }

    fn waterlogged_bucket_pickup(
        &self,
        block: &crate::BlockProbe,
    ) -> Option<(i32, i32, &'static str)> {
        if block
            .block_properties
            .get("waterlogged")
            .map(String::as_str)
            != Some("true")
        {
            return None;
        }
        let block_name = block.block_name.as_deref()?;
        let mut properties = block.block_properties.clone();
        properties.insert("waterlogged".to_string(), "false".to_string());
        let state_id = self
            .registries()
            .block_state_id_by_name_and_properties(block_name, &properties)?;
        Some((VANILLA_ITEM_WATER_BUCKET_ID, state_id, BUCKET_FILL))
    }

    fn predict_local_filled_bucket_use(
        &mut self,
        hand: InteractionHand,
        item_type: &ItemTypeCatalogEntry,
        stack: &ProtocolItemStackSummary,
        fluid: VanillaBucketFluid,
        mob_empty_sound_resource_id: Option<&'static str>,
        hit: ProtocolBlockHitResult,
        sequence: Option<i32>,
        secondary_use_active: bool,
    ) -> LocalItemUsePrediction {
        let unsupported =
            || LocalItemUsePrediction::UnsupportedOverride(VanillaItemUseOverride::BucketItem);
        if stack.item_id != Some(item_type.id) || stack.count < 1 {
            return unsupported();
        }
        let clicked_pos = block_pos(hit.pos);
        let Some(clicked) = self.probe_block(clicked_pos) else {
            return unsupported();
        };
        let clicked_container = clicked
            .block_name
            .as_deref()
            .and_then(vanilla_block_type_for_resource_id)
            .and_then(|block_type| block_type.liquid_block_container_override);
        let initial_pos = if fluid == VanillaBucketFluid::Water && clicked_container.is_some() {
            clicked_pos
        } else {
            relative(clicked_pos, hit.direction)
        };
        let outcome = match self.predict_local_empty_bucket_contents(
            fluid,
            mob_empty_sound_resource_id,
            initial_pos,
            Some(hit),
            sequence,
            secondary_use_active,
        ) {
            BucketEmptyAttempt::Success(outcome) => outcome,
            BucketEmptyAttempt::Fail => {
                return LocalItemUsePrediction::Result(LocalInteractionResult::Fail);
            }
            BucketEmptyAttempt::Unsupported => return unsupported(),
        };

        let empty_result = if self
            .local_player()
            .abilities
            .is_some_and(|abilities| abilities.instabuild)
        {
            stack.clone()
        } else {
            item_stack(VANILLA_ITEM_BUCKET_ID)
        };
        let Some(dropped) =
            self.replace_local_item_with_filled_result(hand, item_type.id, empty_result)
        else {
            return unsupported();
        };
        match outcome {
            BucketEmptyOutcome::Sound {
                pos,
                sound_resource_id,
                neutral_source,
            } => bucket_success(
                dropped,
                LocalInteractionSoundAction::BucketEmpty {
                    pos,
                    sound_resource_id,
                    neutral_source,
                },
                None,
            ),
            BucketEmptyOutcome::Evaporated { pos } => bucket_success(
                dropped,
                LocalInteractionSoundAction::BucketWaterEvaporate { pos },
                Some(LocalInteractionParticleAction::BucketWaterEvaporate { pos }),
            ),
        }
    }

    fn predict_local_empty_bucket_contents(
        &mut self,
        fluid: VanillaBucketFluid,
        mob_empty_sound_resource_id: Option<&'static str>,
        pos: BlockPos,
        hit: Option<ProtocolBlockHitResult>,
        sequence: Option<i32>,
        secondary_use_active: bool,
    ) -> BucketEmptyAttempt {
        let Some(block) = self.probe_block(pos) else {
            return BucketEmptyAttempt::Unsupported;
        };
        let Some(block_name) = block.block_name.as_deref() else {
            return BucketEmptyAttempt::Unsupported;
        };
        let Some(block_type) = vanilla_block_type_for_resource_id(block_name) else {
            return BucketEmptyAttempt::Unsupported;
        };
        let container = block_type.liquid_block_container_override;
        let Some(may_replace) = vanilla_block_state_fluid_replaceable(block.block_state_id) else {
            return BucketEmptyAttempt::Unsupported;
        };
        let can_place_in_container =
            match self.bucket_container_can_place(container, &block.block_properties, fluid) {
                BucketContainerPlacement::CanPlace => true,
                BucketContainerPlacement::CannotPlace => false,
                BucketContainerPlacement::Unsupported => {
                    return BucketEmptyAttempt::Unsupported;
                }
            };
        let place_liquid = may_replace || can_place_in_container;
        let is_air = block_type.id == VANILLA_BLOCK_AIR_ID;
        if !is_air && (!place_liquid || (secondary_use_active && hit.is_some())) {
            let Some(hit) = hit else {
                return BucketEmptyAttempt::Fail;
            };
            return self.predict_local_empty_bucket_contents(
                fluid,
                mob_empty_sound_resource_id,
                relative(block_pos(hit.pos), hit.direction),
                None,
                sequence,
                secondary_use_active,
            );
        }

        if fluid == VanillaBucketFluid::Water && self.water_evaporates_at(pos) {
            return BucketEmptyAttempt::Success(BucketEmptyOutcome::Evaporated { pos });
        }
        let sound_resource_id = if let Some(sound) = mob_empty_sound_resource_id {
            sound
        } else {
            match fluid {
                VanillaBucketFluid::Water => BUCKET_EMPTY,
                VanillaBucketFluid::Lava => BUCKET_EMPTY_LAVA,
                VanillaBucketFluid::Empty => return BucketEmptyAttempt::Unsupported,
            }
        };
        let neutral_source = mob_empty_sound_resource_id.is_some();
        if fluid == VanillaBucketFluid::Water && container.is_some() {
            return BucketEmptyAttempt::Success(BucketEmptyOutcome::Sound {
                pos,
                sound_resource_id,
                neutral_source,
            });
        }

        let Some(sequence) = sequence else {
            return BucketEmptyAttempt::Unsupported;
        };
        let Some(source_state_id) = self.bucket_source_state_id(fluid) else {
            return BucketEmptyAttempt::Unsupported;
        };
        if !self.predict_local_block_state(pos, source_state_id, sequence) {
            return BucketEmptyAttempt::Unsupported;
        }
        BucketEmptyAttempt::Success(BucketEmptyOutcome::Sound {
            pos,
            sound_resource_id,
            neutral_source,
        })
    }

    fn bucket_container_can_place(
        &self,
        owner: Option<VanillaLiquidBlockContainerOverride>,
        properties: &BTreeMap<String, String>,
        fluid: VanillaBucketFluid,
    ) -> BucketContainerPlacement {
        if fluid != VanillaBucketFluid::Water {
            return BucketContainerPlacement::CannotPlace;
        }
        match owner {
            Some(VanillaLiquidBlockContainerOverride::BarrierBlock) => {
                if self.local_player_is_creative() {
                    BucketContainerPlacement::CanPlace
                } else {
                    BucketContainerPlacement::CannotPlace
                }
            }
            Some(VanillaLiquidBlockContainerOverride::SimpleWaterloggedBlock) => {
                BucketContainerPlacement::CanPlace
            }
            Some(VanillaLiquidBlockContainerOverride::SlabBlock) => {
                match properties.get("type").map(String::as_str) {
                    Some("bottom" | "top") => BucketContainerPlacement::CanPlace,
                    Some("double") => BucketContainerPlacement::CannotPlace,
                    _ => BucketContainerPlacement::Unsupported,
                }
            }
            Some(
                VanillaLiquidBlockContainerOverride::KelpBlock
                | VanillaLiquidBlockContainerOverride::KelpPlantBlock
                | VanillaLiquidBlockContainerOverride::SeagrassBlock
                | VanillaLiquidBlockContainerOverride::TallSeagrassBlock,
            )
            | None => BucketContainerPlacement::CannotPlace,
        }
    }

    fn bucket_source_state_id(&self, fluid: VanillaBucketFluid) -> Option<i32> {
        let name = match fluid {
            VanillaBucketFluid::Water => "minecraft:water",
            VanillaBucketFluid::Lava => "minecraft:lava",
            VanillaBucketFluid::Empty => return None,
        };
        self.registries().block_state_id_by_name_and_properties(
            name,
            &BTreeMap::from([("level".to_string(), "0".to_string())]),
        )
    }

    fn bucket_air_block_state_id(&self) -> Option<i32> {
        self.registries()
            .block_state_id_by_name_and_properties("minecraft:air", &BTreeMap::new())
    }
}

fn bucket_success(
    dropped: bool,
    sound_action: LocalInteractionSoundAction,
    particle_action: Option<LocalInteractionParticleAction>,
) -> LocalItemUsePrediction {
    LocalItemUsePrediction::Result(LocalInteractionResult::Success(LocalInteractionSuccess {
        swing_source: LocalInteractionSwingSource::Client,
        was_item_interaction: true,
        starts_using_item: false,
        extra_swing: dropped.then_some(InteractionHand::MainHand),
        sound_action: Some(sound_action),
        particle_action,
    }))
}

fn item_stack(item_id: i32) -> ProtocolItemStackSummary {
    ProtocolItemStackSummary {
        item_id: Some(item_id),
        count: 1,
        component_patch: Default::default(),
    }
}

fn block_pos(pos: ProtocolBlockPos) -> BlockPos {
    BlockPos {
        x: pos.x,
        y: pos.y,
        z: pos.z,
    }
}

fn relative(pos: BlockPos, direction: ProtocolDirection) -> BlockPos {
    match direction {
        ProtocolDirection::Down => BlockPos {
            y: pos.y - 1,
            ..pos
        },
        ProtocolDirection::Up => BlockPos {
            y: pos.y + 1,
            ..pos
        },
        ProtocolDirection::North => BlockPos {
            z: pos.z - 1,
            ..pos
        },
        ProtocolDirection::South => BlockPos {
            z: pos.z + 1,
            ..pos
        },
        ProtocolDirection::West => BlockPos {
            x: pos.x - 1,
            ..pos
        },
        ProtocolDirection::East => BlockPos {
            x: pos.x + 1,
            ..pos
        },
    }
}
