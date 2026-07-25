use super::*;
use crate::{TerrainFluidKind, WorldStore};
use bbb_protocol::{
    block_state_types::{
        vanilla_block_state_fluid_replaceable, vanilla_block_state_support_facts,
        VanillaBlockSupportDirection, VanillaBlockSupportType,
    },
    block_types::{
        vanilla_block_type_for_resource_id, VanillaBucketPickupOverride, VanillaCanSurviveOverride,
        VanillaLiquidBlockContainerOverride, VANILLA_BLOCK_AIR_ID, VANILLA_BLOCK_BIG_DRIPLEAF_ID,
        VANILLA_BLOCK_BIG_DRIPLEAF_STEM_ID, VANILLA_BLOCK_POINTED_DRIPSTONE_ID,
        VANILLA_BLOCK_SCAFFOLDING_ID, VANILLA_BLOCK_SMALL_DRIPLEAF_ID,
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

#[derive(Debug, Clone, Copy)]
enum BucketCanSurviveRule {
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
                let Some((filled_item_id, dry_state_id, sound_resource_id)) =
                    self.waterlogged_bucket_pickup(&block)
                else {
                    return unsupported();
                };
                let predicted_state_id = match block_type.can_survive_override {
                    None => dry_state_id,
                    Some(owner) => {
                        let Some(survives) = self.bucket_pickup_block_can_survive(&block, owner)
                        else {
                            return unsupported();
                        };
                        if survives {
                            dry_state_id
                        } else {
                            let Some(air_state_id) = self.bucket_air_block_state_id() else {
                                return unsupported();
                            };
                            air_state_id
                        }
                    }
                };
                (filled_item_id, predicted_state_id, sound_resource_id)
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

    fn bucket_pickup_block_can_survive(
        &self,
        block: &crate::BlockProbe,
        owner: VanillaCanSurviveOverride,
    ) -> Option<bool> {
        let rule = bucket_can_survive_rule(owner)?;
        let pos = block.pos;
        match rule {
            BucketCanSurviveRule::FacingFull => {
                let facing = direction_property(block, "facing")?;
                let support = self.probe_block(relative(pos, opposite(facing)))?;
                self.bucket_face_sturdy(&support, facing, VanillaBlockSupportType::Full)
            }
            BucketCanSurviveRule::BelowFull => {
                let below = self.probe_block(relative(pos, ProtocolDirection::Down))?;
                self.bucket_face_sturdy(
                    &below,
                    ProtocolDirection::Up,
                    VanillaBlockSupportType::Full,
                )
            }
            BucketCanSurviveRule::BelowRigid => {
                let below = self.probe_block(relative(pos, ProtocolDirection::Down))?;
                self.bucket_face_sturdy(
                    &below,
                    ProtocolDirection::Up,
                    VanillaBlockSupportType::Rigid,
                )
            }
            BucketCanSurviveRule::BigDripleaf => {
                let below = self.probe_block(relative(pos, ProtocolDirection::Down))?;
                let below_type = block_type_id(&below)?;
                if matches!(
                    below_type,
                    VANILLA_BLOCK_BIG_DRIPLEAF_ID | VANILLA_BLOCK_BIG_DRIPLEAF_STEM_ID
                ) {
                    Some(true)
                } else {
                    self.bucket_block_is_in_tag(&below, "minecraft:supports_big_dripleaf")
                }
            }
            BucketCanSurviveRule::BigDripleafStem => {
                let below = self.probe_block(relative(pos, ProtocolDirection::Down))?;
                let below_type = block_type_id(&below)?;
                let supported_below = if below_type == VANILLA_BLOCK_BIG_DRIPLEAF_STEM_ID {
                    true
                } else {
                    self.bucket_block_is_in_tag(&below, "minecraft:supports_big_dripleaf")?
                };
                if !supported_below {
                    return Some(false);
                }
                let above = self.probe_block(relative(pos, ProtocolDirection::Up))?;
                Some(matches!(
                    block_type_id(&above)?,
                    VANILLA_BLOCK_BIG_DRIPLEAF_ID | VANILLA_BLOCK_BIG_DRIPLEAF_STEM_ID
                ))
            }
            BucketCanSurviveRule::BelowCenter => {
                let below = self.probe_block(relative(pos, ProtocolDirection::Down))?;
                self.bucket_face_sturdy(
                    &below,
                    ProtocolDirection::Up,
                    VanillaBlockSupportType::Center,
                )
            }
            BucketCanSurviveRule::AboveCenter => {
                let above = self.probe_block(relative(pos, ProtocolDirection::Up))?;
                self.bucket_face_sturdy(
                    &above,
                    ProtocolDirection::Down,
                    VanillaBlockSupportType::Center,
                )
            }
            BucketCanSurviveRule::AboveFull => {
                let above = self.probe_block(relative(pos, ProtocolDirection::Up))?;
                self.bucket_face_sturdy(
                    &above,
                    ProtocolDirection::Down,
                    VanillaBlockSupportType::Full,
                )
            }
            BucketCanSurviveRule::Lantern => {
                let hanging = boolean_property(block, "hanging")?;
                let (support_direction, sturdy_direction) = if hanging {
                    (ProtocolDirection::Up, ProtocolDirection::Down)
                } else {
                    (ProtocolDirection::Down, ProtocolDirection::Up)
                };
                let support = self.probe_block(relative(pos, support_direction))?;
                if hanging
                    && self.bucket_block_is_in_tag(&support, "minecraft:unstable_bottom_center")?
                {
                    return Some(false);
                }
                self.bucket_face_sturdy(&support, sturdy_direction, VanillaBlockSupportType::Center)
            }
            BucketCanSurviveRule::MangrovePropagule => {
                let hanging = boolean_property(block, "hanging")?;
                let (support_direction, tag) = if hanging {
                    (
                        ProtocolDirection::Up,
                        "minecraft:supports_hanging_mangrove_propagule",
                    )
                } else {
                    (
                        ProtocolDirection::Down,
                        "minecraft:supports_mangrove_propagule",
                    )
                };
                let support = self.probe_block(relative(pos, support_direction))?;
                self.bucket_block_is_in_tag(&support, tag)
            }
            BucketCanSurviveRule::Multiface => {
                let mut has_face = false;
                for (direction, property) in [
                    (ProtocolDirection::Down, "down"),
                    (ProtocolDirection::Up, "up"),
                    (ProtocolDirection::North, "north"),
                    (ProtocolDirection::South, "south"),
                    (ProtocolDirection::West, "west"),
                    (ProtocolDirection::East, "east"),
                ] {
                    let Some(value) = block.block_properties.get(property) else {
                        continue;
                    };
                    let face = parse_boolean_property(value)?;
                    if !face {
                        continue;
                    }
                    has_face = true;
                    let support = self.probe_block(relative(pos, direction))?;
                    if !self.bucket_full_support_or_collision_face(&support, opposite(direction))? {
                        return Some(false);
                    }
                }
                Some(has_face)
            }
            BucketCanSurviveRule::PointedDripstone => {
                let tip_direction = direction_property(block, "vertical_direction")?;
                let behind = self.probe_block(relative(pos, opposite(tip_direction)))?;
                if block_type_id(&behind)? == VANILLA_BLOCK_POINTED_DRIPSTONE_ID {
                    if direction_property(&behind, "vertical_direction")? == tip_direction {
                        return Some(true);
                    }
                }
                self.bucket_face_sturdy(&behind, tip_direction, VanillaBlockSupportType::Full)
            }
            BucketCanSurviveRule::Scaffolding => {
                let below = self.probe_block(relative(pos, ProtocolDirection::Down))?;
                let mut distance = 7;
                if block_type_id(&below)? == VANILLA_BLOCK_SCAFFOLDING_ID {
                    distance = integer_property(&below, "distance")?;
                } else if self.bucket_face_sturdy(
                    &below,
                    ProtocolDirection::Up,
                    VanillaBlockSupportType::Full,
                )? {
                    return Some(true);
                }

                for direction in [
                    ProtocolDirection::North,
                    ProtocolDirection::East,
                    ProtocolDirection::South,
                    ProtocolDirection::West,
                ] {
                    let neighbour = self.probe_block(relative(pos, direction))?;
                    if block_type_id(&neighbour)? == VANILLA_BLOCK_SCAFFOLDING_ID {
                        distance = distance.min(integer_property(&neighbour, "distance")? + 1);
                        if distance == 1 {
                            break;
                        }
                    }
                }
                Some(distance < 7)
            }
            BucketCanSurviveRule::SeaPickle => {
                let below = self.probe_block(relative(pos, ProtocolDirection::Down))?;
                let collision =
                    self.bucket_collision_face_nonempty(&below, ProtocolDirection::Up)?;
                if collision {
                    Some(true)
                } else {
                    self.bucket_face_sturdy(
                        &below,
                        ProtocolDirection::Up,
                        VanillaBlockSupportType::Full,
                    )
                }
            }
            BucketCanSurviveRule::SmallDripleaf => {
                match block.block_properties.get("half").map(String::as_str) {
                    Some("upper") => {
                        let below = self.probe_block(relative(pos, ProtocolDirection::Down))?;
                        Some(
                            block_type_id(&below)? == VANILLA_BLOCK_SMALL_DRIPLEAF_ID
                                && below
                                    .block_properties
                                    .get("half")
                                    .is_some_and(|half| half == "lower"),
                        )
                    }
                    Some("lower") => {
                        // SimpleWaterloggedBlock installs the dry state before
                        // calling canSurvive, so mayPlaceOn's current-position
                        // source-water branch is false on this pickup path.
                        let below = self.probe_block(relative(pos, ProtocolDirection::Down))?;
                        self.bucket_block_is_in_tag(&below, "minecraft:supports_small_dripleaf")
                    }
                    _ => None,
                }
            }
            BucketCanSurviveRule::BelowLegacySolid => {
                let below = self.probe_block(relative(pos, ProtocolDirection::Down))?;
                vanilla_block_state_support_facts(below.block_state_id)
                    .map(|facts| facts.legacy_solid())
            }
            BucketCanSurviveRule::FacingLegacySolid => {
                let facing = direction_property(block, "facing")?;
                let support = self.probe_block(relative(pos, opposite(facing)))?;
                vanilla_block_state_support_facts(support.block_state_id)
                    .map(|facts| facts.legacy_solid())
            }
        }
    }

    fn bucket_face_sturdy(
        &self,
        block: &crate::BlockProbe,
        direction: ProtocolDirection,
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

    fn bucket_collision_face_nonempty(
        &self,
        block: &crate::BlockProbe,
        direction: ProtocolDirection,
    ) -> Option<bool> {
        let facts = vanilla_block_state_support_facts(block.block_state_id)?;
        Some(if facts.dynamic_shape() {
            self.block_has_nonempty_collision_face(block, direction)
        } else {
            facts.collision_face_nonempty(support_direction(direction))
        })
    }

    fn bucket_full_support_or_collision_face(
        &self,
        block: &crate::BlockProbe,
        direction: ProtocolDirection,
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

    fn bucket_block_is_in_tag(&self, block: &crate::BlockProbe, tag: &str) -> Option<bool> {
        let block_type_id = block_type_id(block)?;
        self.registry_holder_ids_for_tag("minecraft:block", tag)
            .map(|holder_ids| holder_ids.contains(&block_type_id))
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

fn opposite(direction: ProtocolDirection) -> ProtocolDirection {
    match direction {
        ProtocolDirection::Down => ProtocolDirection::Up,
        ProtocolDirection::Up => ProtocolDirection::Down,
        ProtocolDirection::North => ProtocolDirection::South,
        ProtocolDirection::South => ProtocolDirection::North,
        ProtocolDirection::West => ProtocolDirection::East,
        ProtocolDirection::East => ProtocolDirection::West,
    }
}

fn support_direction(direction: ProtocolDirection) -> VanillaBlockSupportDirection {
    match direction {
        ProtocolDirection::Down => VanillaBlockSupportDirection::Down,
        ProtocolDirection::Up => VanillaBlockSupportDirection::Up,
        ProtocolDirection::North => VanillaBlockSupportDirection::North,
        ProtocolDirection::South => VanillaBlockSupportDirection::South,
        ProtocolDirection::West => VanillaBlockSupportDirection::West,
        ProtocolDirection::East => VanillaBlockSupportDirection::East,
    }
}

fn sturdy_face_index(direction: ProtocolDirection) -> usize {
    match direction {
        ProtocolDirection::West => 0,
        ProtocolDirection::South => 1,
        ProtocolDirection::East => 2,
        ProtocolDirection::North => 3,
        ProtocolDirection::Down => 4,
        ProtocolDirection::Up => 5,
    }
}

fn direction_property(block: &crate::BlockProbe, property: &str) -> Option<ProtocolDirection> {
    match block.block_properties.get(property)?.as_str() {
        "down" => Some(ProtocolDirection::Down),
        "up" => Some(ProtocolDirection::Up),
        "north" => Some(ProtocolDirection::North),
        "south" => Some(ProtocolDirection::South),
        "west" => Some(ProtocolDirection::West),
        "east" => Some(ProtocolDirection::East),
        _ => None,
    }
}

fn boolean_property(block: &crate::BlockProbe, property: &str) -> Option<bool> {
    parse_boolean_property(block.block_properties.get(property)?)
}

fn parse_boolean_property(value: &str) -> Option<bool> {
    match value {
        "true" => Some(true),
        "false" => Some(false),
        _ => None,
    }
}

fn integer_property(block: &crate::BlockProbe, property: &str) -> Option<i32> {
    block.block_properties.get(property)?.parse().ok()
}

fn block_type_id(block: &crate::BlockProbe) -> Option<i32> {
    block
        .block_name
        .as_deref()
        .and_then(vanilla_block_type_for_resource_id)
        .map(|block_type| block_type.id)
}

#[cfg(test)]
pub(super) const fn bucket_can_survive_owner_supported(owner: VanillaCanSurviveOverride) -> bool {
    bucket_can_survive_rule(owner).is_some()
}

const fn bucket_can_survive_rule(owner: VanillaCanSurviveOverride) -> Option<BucketCanSurviveRule> {
    Some(match owner {
        VanillaCanSurviveOverride::AmethystClusterBlock
        | VanillaCanSurviveOverride::BaseCoralWallFanBlock
        | VanillaCanSurviveOverride::LadderBlock => BucketCanSurviveRule::FacingFull,
        VanillaCanSurviveOverride::BaseCoralPlantTypeBlock => BucketCanSurviveRule::BelowFull,
        VanillaCanSurviveOverride::BaseRailBlock => BucketCanSurviveRule::BelowRigid,
        VanillaCanSurviveOverride::BigDripleafBlock => BucketCanSurviveRule::BigDripleaf,
        VanillaCanSurviveOverride::BigDripleafStemBlock => BucketCanSurviveRule::BigDripleafStem,
        VanillaCanSurviveOverride::CandleBlock => BucketCanSurviveRule::BelowCenter,
        VanillaCanSurviveOverride::CeilingHangingSignBlock => BucketCanSurviveRule::AboveCenter,
        VanillaCanSurviveOverride::HangingRootsBlock => BucketCanSurviveRule::AboveFull,
        VanillaCanSurviveOverride::LanternBlock => BucketCanSurviveRule::Lantern,
        VanillaCanSurviveOverride::MangrovePropaguleBlock => {
            BucketCanSurviveRule::MangrovePropagule
        }
        VanillaCanSurviveOverride::MultifaceBlock => BucketCanSurviveRule::Multiface,
        VanillaCanSurviveOverride::PointedDripstoneBlock => BucketCanSurviveRule::PointedDripstone,
        VanillaCanSurviveOverride::ScaffoldingBlock => BucketCanSurviveRule::Scaffolding,
        VanillaCanSurviveOverride::SeaPickleBlock => BucketCanSurviveRule::SeaPickle,
        VanillaCanSurviveOverride::SmallDripleafBlock => BucketCanSurviveRule::SmallDripleaf,
        VanillaCanSurviveOverride::StandingSignBlock => BucketCanSurviveRule::BelowLegacySolid,
        VanillaCanSurviveOverride::WallSignBlock => BucketCanSurviveRule::FacingLegacySolid,
        _ => return None,
    })
}
