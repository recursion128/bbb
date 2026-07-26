use super::{
    above, LocalBlockUseContext, LocalInteractionParticleAction, LocalInteractionResult,
    LocalInteractionSoundAction, LocalInteractionSuccess, LocalTargetUsePrediction,
};
use crate::{TerrainFluidKind, WorldStore};
use bbb_protocol::{
    block_entity_types::{
        VANILLA_BLOCK_ENTITY_TYPE_CAMPFIRE_ID, VANILLA_BLOCK_ENTITY_TYPE_CHISELED_BOOKSHELF_ID,
        VANILLA_BLOCK_ENTITY_TYPE_DECORATED_POT_ID, VANILLA_BLOCK_ENTITY_TYPE_HANGING_SIGN_ID,
        VANILLA_BLOCK_ENTITY_TYPE_SHELF_ID, VANILLA_BLOCK_ENTITY_TYPE_SIGN_ID,
    },
    block_types::{BlockTypeCatalogEntry, VanillaBlockUseItemOnOverride, VANILLA_BLOCK_TYPES},
    data_component_types::{
        VANILLA_DATA_COMPONENT_BANNER_PATTERNS_TYPE_ID, VANILLA_DATA_COMPONENT_DYED_COLOR_TYPE_ID,
        VANILLA_DATA_COMPONENT_JUKEBOX_PLAYABLE_TYPE_ID,
        VANILLA_DATA_COMPONENT_POTION_CONTENTS_TYPE_ID,
    },
    item_types::{
        vanilla_cauldron_item_interaction, vanilla_cauldron_tag_interactions,
        vanilla_compost_chance_bits, vanilla_item_type, VanillaCauldronInteraction,
        VanillaCauldronKind, VanillaItemInteractLivingEntityOverride, VanillaItemUseOnOverride,
        VANILLA_ITEM_BONE_MEAL_ID, VANILLA_ITEM_FIRE_CHARGE_ID, VANILLA_ITEM_FLINT_AND_STEEL_ID,
        VANILLA_ITEM_GLASS_BOTTLE_ID, VANILLA_ITEM_GLOWSTONE_ID, VANILLA_ITEM_GLOW_INK_SAC_ID,
        VANILLA_ITEM_HONEYCOMB_ID, VANILLA_ITEM_HONEY_BOTTLE_ID, VANILLA_ITEM_INK_SAC_ID,
        VANILLA_ITEM_SHEARS_ID,
    },
    packets::{Direction as ProtocolDirection, InteractionHand, ItemStackSummary},
    potion_types::vanilla_potion,
};
use std::collections::BTreeMap;

const VANILLA_BOOKSHELF_BOOKS_ITEM_TAG: &str = "minecraft:bookshelf_books";
const VANILLA_LECTERN_BOOKS_ITEM_TAG: &str = "minecraft:lectern_books";
const VANILLA_NOTEBLOCK_TOP_INSTRUMENTS_ITEM_TAG: &str = "minecraft:noteblock_top_instruments";
const VANILLA_AXES_ITEM_TAG: &str = "minecraft:axes";
const VANILLA_WATER_FLUID_TAG: &str = "minecraft:water";
const VANILLA_WATER_POTION: &str = "minecraft:water";

impl WorldStore {
    pub(super) fn predict_local_block_use_item_on(
        &mut self,
        owner: VanillaBlockUseItemOnOverride,
        block_type: &BlockTypeCatalogEntry,
        hand: InteractionHand,
        context: LocalBlockUseContext,
    ) -> LocalTargetUsePrediction {
        let unsupported = || LocalTargetUsePrediction::UnsupportedBlockUseItemOn(owner);
        let Some(block) = self.probe_block(context.pos) else {
            return unsupported();
        };
        if block.block_name.as_deref() != Some(block_type.resource_id) {
            return unsupported();
        }

        match owner {
            VanillaBlockUseItemOnOverride::AbstractCauldronBlock => {
                let Some(cauldron) =
                    vanilla_cauldron_kind(block_type.resource_id, &block.block_properties)
                else {
                    return unsupported();
                };
                let Some(stack) = self.local_item_in_hand(hand).cloned() else {
                    return try_empty_hand();
                };
                let Some(item_id) = stack.item_id else {
                    return try_empty_hand();
                };
                for tag_interaction in vanilla_cauldron_tag_interactions(cauldron) {
                    let Some(in_tag) = self
                        .local_item_in_hand_is_in_tag(hand, tag_interaction.item_tag_resource_id)
                    else {
                        return unsupported();
                    };
                    if in_tag {
                        return self.predict_local_cauldron_interaction(
                            tag_interaction.interaction,
                            cauldron,
                            &block.block_properties,
                            &stack,
                            context.pos,
                        );
                    }
                }
                let Some(interaction) = vanilla_cauldron_item_interaction(cauldron, item_id) else {
                    return try_empty_hand();
                };
                self.predict_local_cauldron_interaction(
                    interaction,
                    cauldron,
                    &block.block_properties,
                    &stack,
                    context.pos,
                )
            }
            VanillaBlockUseItemOnOverride::ChiseledBookShelfBlock => {
                if self.block_entity_type_id_at(context.pos)
                    != Some(VANILLA_BLOCK_ENTITY_TYPE_CHISELED_BOOKSHELF_ID)
                {
                    return pass();
                }
                let Some(is_book) =
                    self.local_item_in_hand_is_in_tag(hand, VANILLA_BOOKSHELF_BOOKS_ITEM_TAG)
                else {
                    return unsupported();
                };
                if !is_book {
                    return try_empty_hand();
                }
                let Some(facing) = horizontal_direction_property(&block.block_properties, "facing")
                else {
                    return unsupported();
                };
                let Some(slot) = selectable_slot_hit(facing, context.face, context.cursor, 2, 3)
                else {
                    return pass();
                };
                match boolean_property(&block.block_properties, &format!("slot_{slot}_occupied")) {
                    Some(true) => try_empty_hand(),
                    Some(false) => success(),
                    None => unsupported(),
                }
            }
            VanillaBlockUseItemOnOverride::CakeBlock => {
                let Some(stack) = self.local_item_in_hand(hand) else {
                    return try_empty_hand();
                };
                let Some(bites) = block
                    .block_properties
                    .get("bites")
                    .and_then(|bites| bites.parse::<u8>().ok())
                    .filter(|bites| *bites <= 6)
                else {
                    return unsupported();
                };
                let Some(candle_cake) = stack.item_id.and_then(candle_cake_block_type_for_item)
                else {
                    return try_empty_hand();
                };
                if bites != 0 {
                    return try_empty_hand();
                }
                self.predict_local_block_item_replacement(
                    owner,
                    hand,
                    context,
                    candle_cake.resource_id,
                    BTreeMap::from([("lit".to_string(), "false".to_string())]),
                    Some(LocalInteractionSoundAction::SuppressedCakeAddCandle),
                )
            }
            VanillaBlockUseItemOnOverride::CandleBlock => {
                let Some(lit) = boolean_property(&block.block_properties, "lit") else {
                    return unsupported();
                };
                let Some(candles) = integer_property(&block.block_properties, "candles", 1..=4)
                else {
                    return unsupported();
                };
                if self.local_item_in_hand(hand).is_some()
                    || self.local_player_is_adventure()
                    || self.local_player_is_spectator()
                    || !lit
                {
                    return try_empty_hand();
                }
                let mut properties = block.block_properties;
                properties.insert("lit".to_string(), "false".to_string());
                let Some(state_id) = self
                    .registries
                    .block_state_id_by_name_and_properties(block_type.resource_id, &properties)
                else {
                    return unsupported();
                };
                if !self.predict_local_block_state(context.pos, state_id, context.sequence) {
                    return unsupported();
                }
                LocalTargetUsePrediction::Result(LocalInteractionResult::Success(
                    LocalInteractionSuccess {
                        sound_action: Some(LocalInteractionSoundAction::SuppressedCandleExtinguish),
                        particle_action: Some(LocalInteractionParticleAction::CandleExtinguish {
                            pos: context.pos,
                            candles,
                        }),
                        ..LocalInteractionSuccess::client_item_interaction()
                    },
                ))
            }
            VanillaBlockUseItemOnOverride::FlowerPotBlock => {
                let Some(stack) = self.local_item_in_hand(hand) else {
                    return try_empty_hand();
                };
                let Some(potted) = stack.item_id.and_then(potted_block_type_for_item) else {
                    return try_empty_hand();
                };
                if block_type.resource_id != "minecraft:flower_pot" {
                    return consume();
                }
                self.predict_local_block_item_replacement(
                    owner,
                    hand,
                    context,
                    potted.resource_id,
                    BTreeMap::new(),
                    None,
                )
            }
            VanillaBlockUseItemOnOverride::CandleCakeBlock => {
                match (
                    self.local_item_in_hand(hand).is_none(),
                    block.block_properties.get("lit").map(String::as_str),
                ) {
                    (true, Some("true")) if context.cursor[1] > 0.5 => {
                        let mut properties = block.block_properties;
                        properties.insert("lit".to_string(), "false".to_string());
                        let Some(state_id) = self.registries.block_state_id_by_name_and_properties(
                            block_type.resource_id,
                            &properties,
                        ) else {
                            return unsupported();
                        };
                        if !self.predict_local_block_state(context.pos, state_id, context.sequence)
                        {
                            return unsupported();
                        }
                        LocalTargetUsePrediction::Result(LocalInteractionResult::Success(
                            LocalInteractionSuccess {
                                sound_action: Some(
                                    LocalInteractionSoundAction::SuppressedCandleExtinguish,
                                ),
                                particle_action: Some(
                                    LocalInteractionParticleAction::CandleCakeExtinguish {
                                        pos: context.pos,
                                    },
                                ),
                                ..LocalInteractionSuccess::client_item_interaction()
                            },
                        ))
                    }
                    (_, Some("false" | "true")) => try_empty_hand(),
                    _ => unsupported(),
                }
            }
            VanillaBlockUseItemOnOverride::CampfireBlock => {
                let Some(item_id) = self
                    .local_item_in_hand(hand)
                    .and_then(|stack| stack.item_id)
                else {
                    return try_empty_hand();
                };
                if self.block_entity_type_id_at(context.pos)
                    != Some(VANILLA_BLOCK_ENTITY_TYPE_CAMPFIRE_ID)
                {
                    return try_empty_hand();
                }
                let Some(inputs) = self.recipes().property_sets.get("minecraft:campfire_input")
                else {
                    return unsupported();
                };
                if inputs.contains(&item_id) {
                    consume()
                } else {
                    try_empty_hand()
                }
            }
            VanillaBlockUseItemOnOverride::ComposterBlock => {
                let Some(level) = integer_property(&block.block_properties, "level", 0..=8) else {
                    return unsupported();
                };
                let Some(item_id) = self
                    .local_item_in_hand(hand)
                    .and_then(|stack| stack.item_id)
                else {
                    return try_empty_hand();
                };
                if level < 8 && vanilla_compost_chance_bits(item_id).is_some() {
                    success()
                } else {
                    try_empty_hand()
                }
            }
            VanillaBlockUseItemOnOverride::BeehiveBlock => {
                let Some(honey_level) =
                    integer_property(&block.block_properties, "honey_level", 0..=5)
                else {
                    return unsupported();
                };
                let Some(stack) = self.local_item_in_hand(hand) else {
                    return try_empty_hand();
                };
                if honey_level < 5 || stack.item_id != Some(VANILLA_ITEM_GLASS_BOTTLE_ID) {
                    return try_empty_hand();
                }
                let mut properties = block.block_properties;
                properties.insert("honey_level".to_string(), "0".to_string());
                let Some(state_id) = self
                    .registries
                    .block_state_id_by_name_and_properties(block_type.resource_id, &properties)
                else {
                    return unsupported();
                };
                if !self.predict_local_block_state(context.pos, state_id, context.sequence) {
                    return unsupported();
                }
                let Some(dropped) = self.replace_local_item_with_forced_filled_result(
                    hand,
                    VANILLA_ITEM_GLASS_BOTTLE_ID,
                    ItemStackSummary {
                        item_id: Some(VANILLA_ITEM_HONEY_BOTTLE_ID),
                        count: 1,
                        component_patch: Default::default(),
                    },
                ) else {
                    return unsupported();
                };
                LocalTargetUsePrediction::Result(LocalInteractionResult::Success(
                    LocalInteractionSuccess::beehive_bottle_fill(dropped),
                ))
            }
            VanillaBlockUseItemOnOverride::CopperGolemStatueBlock => {
                let Some(is_axe) = self.local_item_in_hand_is_in_tag(hand, VANILLA_AXES_ITEM_TAG)
                else {
                    return unsupported();
                };
                if is_axe {
                    return pass();
                }
                let Some(next_pose) = block
                    .block_properties
                    .get("copper_golem_pose")
                    .and_then(|pose| next_copper_golem_pose(pose))
                else {
                    return unsupported();
                };
                let mut properties = block.block_properties;
                properties.insert("copper_golem_pose".to_string(), next_pose.to_string());
                let Some(state_id) = self
                    .registries
                    .block_state_id_by_name_and_properties(block_type.resource_id, &properties)
                else {
                    return unsupported();
                };
                if !self.predict_local_block_state(context.pos, state_id, context.sequence) {
                    return unsupported();
                }
                LocalTargetUsePrediction::Result(LocalInteractionResult::Success(
                    LocalInteractionSuccess {
                        sound_action: Some(
                            LocalInteractionSoundAction::SuppressedCopperGolemBecomeStatue,
                        ),
                        ..LocalInteractionSuccess::client_item_interaction()
                    },
                ))
            }
            VanillaBlockUseItemOnOverride::DecoratedPotBlock => {
                if self.block_entity_type_id_at(context.pos)
                    == Some(VANILLA_BLOCK_ENTITY_TYPE_DECORATED_POT_ID)
                {
                    success()
                } else {
                    pass()
                }
            }
            VanillaBlockUseItemOnOverride::JukeboxBlock => {
                match block.block_properties.get("has_record").map(String::as_str) {
                    Some("true") => try_empty_hand(),
                    Some("false") => {
                        let Some(stack) = self.local_item_in_hand(hand) else {
                            return try_empty_hand();
                        };
                        if self.item_stack_has_effective_component(
                            stack,
                            VANILLA_DATA_COMPONENT_JUKEBOX_PLAYABLE_TYPE_ID,
                        ) {
                            success()
                        } else {
                            try_empty_hand()
                        }
                    }
                    _ => unsupported(),
                }
            }
            VanillaBlockUseItemOnOverride::LecternBlock => {
                match block.block_properties.get("has_book").map(String::as_str) {
                    Some("true") => try_empty_hand(),
                    Some("false") => {
                        let Some(is_book) =
                            self.local_item_in_hand_is_in_tag(hand, VANILLA_LECTERN_BOOKS_ITEM_TAG)
                        else {
                            return unsupported();
                        };
                        if is_book {
                            success()
                        } else if self.local_item_in_hand(hand).is_none()
                            && hand == InteractionHand::MainHand
                        {
                            pass()
                        } else {
                            try_empty_hand()
                        }
                    }
                    _ => unsupported(),
                }
            }
            VanillaBlockUseItemOnOverride::NoteBlock => {
                let Some(is_top_instrument) = self
                    .local_item_in_hand_is_in_tag(hand, VANILLA_NOTEBLOCK_TOP_INSTRUMENTS_ITEM_TAG)
                else {
                    return unsupported();
                };
                if is_top_instrument && context.face == ProtocolDirection::Up {
                    pass()
                } else {
                    try_empty_hand()
                }
            }
            VanillaBlockUseItemOnOverride::PumpkinBlock => {
                if self
                    .local_item_in_hand(hand)
                    .and_then(|stack| stack.item_id)
                    == Some(VANILLA_ITEM_SHEARS_ID)
                {
                    success()
                } else {
                    try_empty_hand()
                }
            }
            VanillaBlockUseItemOnOverride::RespawnAnchorBlock => {
                let charges = block
                    .block_properties
                    .get("charges")
                    .and_then(|charges| charges.parse::<u8>().ok())
                    .filter(|charges| *charges <= 4);
                let Some(charges) = charges else {
                    return unsupported();
                };
                let held_item_id = self
                    .local_item_in_hand(hand)
                    .and_then(|stack| stack.item_id);
                if held_item_id == Some(VANILLA_ITEM_GLOWSTONE_ID) && charges < 4 {
                    let mut properties = block.block_properties;
                    properties.insert("charges".to_string(), (charges + 1).to_string());
                    let Some(state_id) = self
                        .registries
                        .block_state_id_by_name_and_properties(block_type.resource_id, &properties)
                    else {
                        return unsupported();
                    };
                    if !self.predict_local_block_state(context.pos, state_id, context.sequence) {
                        return unsupported();
                    }
                    let should_shrink = !self
                        .local_player()
                        .abilities
                        .is_some_and(|abilities| abilities.instabuild);
                    if should_shrink && !self.shrink_local_item_in_hand(hand, 1) {
                        return unsupported();
                    }
                    return LocalTargetUsePrediction::Result(LocalInteractionResult::Success(
                        LocalInteractionSuccess {
                            sound_action: Some(
                                LocalInteractionSoundAction::SuppressedRespawnAnchorCharge,
                            ),
                            ..LocalInteractionSuccess::client_item_interaction()
                        },
                    ));
                }
                if hand == InteractionHand::MainHand
                    && charges < 4
                    && self
                        .local_item_in_hand(InteractionHand::OffHand)
                        .and_then(|stack| stack.item_id)
                        == Some(VANILLA_ITEM_GLOWSTONE_ID)
                {
                    pass()
                } else {
                    try_empty_hand()
                }
            }
            VanillaBlockUseItemOnOverride::ShelfBlock => {
                if self.block_entity_type_id_at(context.pos)
                    != Some(VANILLA_BLOCK_ENTITY_TYPE_SHELF_ID)
                    || hand == InteractionHand::OffHand
                {
                    return pass();
                }
                let Some(facing) = horizontal_direction_property(&block.block_properties, "facing")
                else {
                    return unsupported();
                };
                if selectable_slot_hit(facing, context.face, context.cursor, 1, 3).is_none() {
                    return pass();
                }
                if self.local_item_in_hand(InteractionHand::MainHand).is_some() {
                    success()
                } else {
                    pass()
                }
            }
            VanillaBlockUseItemOnOverride::SignBlock
            | VanillaBlockUseItemOnOverride::CeilingHangingSignBlock
            | VanillaBlockUseItemOnOverride::WallHangingSignBlock => {
                if !matches!(
                    self.block_entity_type_id_at(context.pos),
                    Some(
                        VANILLA_BLOCK_ENTITY_TYPE_SIGN_ID
                            | VANILLA_BLOCK_ENTITY_TYPE_HANGING_SIGN_ID
                    )
                ) {
                    return pass();
                }
                let Some(has_usable_applicator) =
                    self.local_item_in_hand(hand).map_or(Some(false), |stack| {
                        item_stack_is_sign_applicator(stack.item_id).map(|is_applicator| {
                            is_applicator
                                && !self.local_player_is_adventure()
                                && !self.local_player_is_spectator()
                        })
                    })
                else {
                    return unsupported();
                };
                match self.sign_text_state_at(context.pos) {
                    Some(text) if text.is_waxed => success(),
                    Some(_) if has_usable_applicator => success(),
                    Some(_) => consume(),
                    None => unsupported(),
                }
            }
            VanillaBlockUseItemOnOverride::SweetBerryBushBlock => {
                let Some(age) = integer_property(&block.block_properties, "age", 0..=3) else {
                    return unsupported();
                };
                let held_item_id = self
                    .local_item_in_hand(hand)
                    .and_then(|stack| stack.item_id);
                if age < 3 && held_item_id == Some(VANILLA_ITEM_BONE_MEAL_ID) {
                    pass()
                } else {
                    try_empty_hand()
                }
            }
            VanillaBlockUseItemOnOverride::TntBlock => {
                match self
                    .local_item_in_hand(hand)
                    .and_then(|stack| stack.item_id)
                {
                    Some(VANILLA_ITEM_FLINT_AND_STEEL_ID | VANILLA_ITEM_FIRE_CHARGE_ID) => {
                        success()
                    }
                    _ => try_empty_hand(),
                }
            }
            VanillaBlockUseItemOnOverride::VaultBlock => {
                match (
                    self.local_item_in_hand(hand).is_some(),
                    block
                        .block_properties
                        .get("vault_state")
                        .map(String::as_str),
                ) {
                    (true, Some("active")) => {
                        LocalTargetUsePrediction::Result(LocalInteractionResult::Success(
                            LocalInteractionSuccess::server_item_interaction(),
                        ))
                    }
                    (_, Some("inactive" | "unlocking" | "ejecting")) | (false, Some("active")) => {
                        try_empty_hand()
                    }
                    _ => unsupported(),
                }
            }
            _ => unsupported(),
        }
    }

    fn local_item_in_hand_is_in_tag(&self, hand: InteractionHand, tag: &str) -> Option<bool> {
        let Some(stack) = self.local_item_in_hand(hand) else {
            return Some(false);
        };
        let item_id = stack.item_id?;
        let item_ids = self.registries.holder_ids_for_tag("minecraft:item", tag)?;
        Some(item_ids.contains(&item_id))
    }

    fn predict_local_cauldron_interaction(
        &self,
        interaction: VanillaCauldronInteraction,
        cauldron: VanillaCauldronKind,
        properties: &BTreeMap<String, String>,
        stack: &ItemStackSummary,
        pos: crate::BlockPos,
    ) -> LocalTargetUsePrediction {
        let level = match cauldron {
            VanillaCauldronKind::Water | VanillaCauldronKind::PowderSnow => {
                let Some(level) = integer_property(properties, "level", 1..=3) else {
                    return LocalTargetUsePrediction::UnsupportedBlockUseItemOn(
                        VanillaBlockUseItemOnOverride::AbstractCauldronBlock,
                    );
                };
                Some(level)
            }
            VanillaCauldronKind::Empty | VanillaCauldronKind::Lava => None,
        };
        match interaction {
            VanillaCauldronInteraction::CleanBanner => {
                if self.item_stack_has_effective_component(
                    stack,
                    VANILLA_DATA_COMPONENT_BANNER_PATTERNS_TYPE_ID,
                ) && !stack.component_patch.banner_pattern_layers.is_empty()
                {
                    success()
                } else {
                    try_empty_hand()
                }
            }
            VanillaCauldronInteraction::CleanDyedItem => {
                if self.item_stack_has_effective_component(
                    stack,
                    VANILLA_DATA_COMPONENT_DYED_COLOR_TYPE_ID,
                ) {
                    success()
                } else {
                    try_empty_hand()
                }
            }
            VanillaCauldronInteraction::CleanShulkerBox
            | VanillaCauldronInteraction::PourWaterBucket
            | VanillaCauldronInteraction::TakeLavaBucket
            | VanillaCauldronInteraction::TakeWaterBottle => success(),
            VanillaCauldronInteraction::PourLavaBucket
            | VanillaCauldronInteraction::PourPowderSnowBucket => {
                match self.local_block_fluid_is_in_tag(above(pos), VANILLA_WATER_FLUID_TAG) {
                    Some(true) => consume(),
                    Some(false) => success(),
                    None => LocalTargetUsePrediction::UnsupportedBlockUseItemOn(
                        VanillaBlockUseItemOnOverride::AbstractCauldronBlock,
                    ),
                }
            }
            VanillaCauldronInteraction::PourWaterPotionIntoEmpty => {
                match self.item_stack_is_water_potion(stack) {
                    Some(true) => success(),
                    Some(false) => try_empty_hand(),
                    None => LocalTargetUsePrediction::UnsupportedBlockUseItemOn(
                        VanillaBlockUseItemOnOverride::AbstractCauldronBlock,
                    ),
                }
            }
            VanillaCauldronInteraction::PourWaterPotionIntoWater => {
                if level == Some(3) {
                    return try_empty_hand();
                }
                match self.item_stack_is_water_potion(stack) {
                    Some(true) => success(),
                    Some(false) => try_empty_hand(),
                    None => LocalTargetUsePrediction::UnsupportedBlockUseItemOn(
                        VanillaBlockUseItemOnOverride::AbstractCauldronBlock,
                    ),
                }
            }
            VanillaCauldronInteraction::TakePowderSnowBucket
            | VanillaCauldronInteraction::TakeWaterBucket => {
                if level == Some(3) {
                    success()
                } else {
                    try_empty_hand()
                }
            }
        }
    }

    fn item_stack_is_water_potion(&self, stack: &ItemStackSummary) -> Option<bool> {
        if !self.item_stack_has_effective_component(
            stack,
            VANILLA_DATA_COMPONENT_POTION_CONTENTS_TYPE_ID,
        ) {
            return Some(false);
        }
        let Some(potion_id) = self.effective_item_stack(stack).component_patch.potion_id else {
            return Some(false);
        };
        Some(vanilla_potion(potion_id)?.resource_id == VANILLA_WATER_POTION)
    }

    fn local_block_fluid_is_in_tag(&self, pos: crate::BlockPos, tag: &str) -> Option<bool> {
        let block = self.probe_block(pos)?;
        let fluid_resource_id = match block.fluid {
            None => "minecraft:empty",
            Some(fluid) => match (fluid.kind, fluid.amount == 8 && !fluid.falling) {
                (TerrainFluidKind::Water, true) => "minecraft:water",
                (TerrainFluidKind::Water, false) => "minecraft:flowing_water",
                (TerrainFluidKind::Lava, true) => "minecraft:lava",
                (TerrainFluidKind::Lava, false) => "minecraft:flowing_lava",
            },
        };
        let holder_id =
            self.registry_holder_id_by_resource_id("minecraft:fluid", fluid_resource_id)?;
        self.registry_holder_ids_for_tag("minecraft:fluid", tag)
            .map(|holder_ids| holder_ids.contains(&holder_id))
    }

    fn predict_local_block_item_replacement(
        &mut self,
        owner: VanillaBlockUseItemOnOverride,
        hand: InteractionHand,
        context: LocalBlockUseContext,
        target_block_name: &str,
        target_properties: BTreeMap<String, String>,
        sound_action: Option<LocalInteractionSoundAction>,
    ) -> LocalTargetUsePrediction {
        let Some(stack) = self.local_item_in_hand(hand) else {
            return LocalTargetUsePrediction::UnsupportedBlockUseItemOn(owner);
        };
        let should_shrink = !self
            .local_player()
            .abilities
            .is_some_and(|abilities| abilities.instabuild);
        if should_shrink && stack.count < 1 {
            return LocalTargetUsePrediction::UnsupportedBlockUseItemOn(owner);
        }
        let Some(state_id) = self
            .registries
            .block_state_id_by_name_and_properties(target_block_name, &target_properties)
        else {
            return LocalTargetUsePrediction::UnsupportedBlockUseItemOn(owner);
        };
        if !self.predict_local_block_state(context.pos, state_id, context.sequence) {
            return LocalTargetUsePrediction::UnsupportedBlockUseItemOn(owner);
        }
        if should_shrink && !self.shrink_local_item_in_hand(hand, 1) {
            return LocalTargetUsePrediction::UnsupportedBlockUseItemOn(owner);
        }
        LocalTargetUsePrediction::Result(LocalInteractionResult::Success(LocalInteractionSuccess {
            sound_action,
            ..LocalInteractionSuccess::client_item_interaction()
        }))
    }
}

fn candle_cake_block_type_for_item(item_id: i32) -> Option<&'static BlockTypeCatalogEntry> {
    let candle = block_type_for_item(item_id)?;
    if candle.use_item_on_override != Some(VanillaBlockUseItemOnOverride::CandleBlock) {
        return None;
    }
    let candle_cake_constant = format!("{}_CAKE", candle.constant_name);
    VANILLA_BLOCK_TYPES
        .iter()
        .find(|block| block.constant_name == candle_cake_constant)
}

fn potted_block_type_for_item(item_id: i32) -> Option<&'static BlockTypeCatalogEntry> {
    let content = block_type_for_item(item_id)?;
    let potted_constant = format!("POTTED_{}", content.constant_name);
    VANILLA_BLOCK_TYPES
        .iter()
        .find(|block| block.constant_name == potted_constant)
}

fn block_type_for_item(item_id: i32) -> Option<&'static BlockTypeCatalogEntry> {
    let block_type_id = vanilla_item_type(item_id)?
        .block_item_behavior?
        .block_type_id;
    VANILLA_BLOCK_TYPES.get(usize::try_from(block_type_id).ok()?)
}

fn vanilla_cauldron_kind(
    block_name: &str,
    properties: &BTreeMap<String, String>,
) -> Option<VanillaCauldronKind> {
    match block_name {
        "minecraft:cauldron" if properties.is_empty() => Some(VanillaCauldronKind::Empty),
        "minecraft:water_cauldron" => Some(VanillaCauldronKind::Water),
        "minecraft:lava_cauldron" if properties.is_empty() => Some(VanillaCauldronKind::Lava),
        "minecraft:powder_snow_cauldron" => Some(VanillaCauldronKind::PowderSnow),
        _ => None,
    }
}

fn item_stack_is_sign_applicator(item_id: Option<i32>) -> Option<bool> {
    let item = vanilla_item_type(item_id?)?;
    Some(
        item.interact_living_entity_override
            == Some(VanillaItemInteractLivingEntityOverride::DyeItem)
            || item.use_on_override == Some(VanillaItemUseOnOverride::HoneycombItem)
            || matches!(
                item.id,
                VANILLA_ITEM_INK_SAC_ID | VANILLA_ITEM_GLOW_INK_SAC_ID | VANILLA_ITEM_HONEYCOMB_ID
            ),
    )
}

fn next_copper_golem_pose(pose: &str) -> Option<&'static str> {
    match pose {
        "standing" => Some("sitting"),
        "sitting" => Some("running"),
        "running" => Some("star"),
        "star" => Some("standing"),
        _ => None,
    }
}

fn boolean_property(properties: &BTreeMap<String, String>, property: &str) -> Option<bool> {
    match properties.get(property)?.as_str() {
        "false" => Some(false),
        "true" => Some(true),
        _ => None,
    }
}

fn integer_property(
    properties: &BTreeMap<String, String>,
    property: &str,
    range: std::ops::RangeInclusive<u8>,
) -> Option<u8> {
    properties
        .get(property)?
        .parse::<u8>()
        .ok()
        .filter(|value| range.contains(value))
}

fn horizontal_direction_property(
    properties: &BTreeMap<String, String>,
    property: &str,
) -> Option<ProtocolDirection> {
    match properties.get(property)?.as_str() {
        "north" => Some(ProtocolDirection::North),
        "south" => Some(ProtocolDirection::South),
        "west" => Some(ProtocolDirection::West),
        "east" => Some(ProtocolDirection::East),
        _ => None,
    }
}

fn selectable_slot_hit(
    facing: ProtocolDirection,
    clicked_direction: ProtocolDirection,
    cursor: [f64; 3],
    rows: usize,
    columns: usize,
) -> Option<usize> {
    if facing != clicked_direction || rows == 0 || columns == 0 {
        return None;
    }
    let relative_x = match clicked_direction {
        ProtocolDirection::North => 1.0 - cursor[0] as f32,
        ProtocolDirection::South => cursor[0] as f32,
        ProtocolDirection::West => cursor[2] as f32,
        ProtocolDirection::East => 1.0 - cursor[2] as f32,
        ProtocolDirection::Down | ProtocolDirection::Up => return None,
    };
    let row = selectable_slot_section(1.0 - cursor[1] as f32, rows);
    let column = selectable_slot_section(relative_x, columns);
    Some(column + row * columns)
}

fn selectable_slot_section(relative_coordinate: f32, sections: usize) -> usize {
    let targeted_pixel = relative_coordinate * 16.0;
    let section_size = 16.0 / sections as f32;
    (targeted_pixel / section_size)
        .floor()
        .clamp(0.0, (sections - 1) as f32) as usize
}

fn pass() -> LocalTargetUsePrediction {
    LocalTargetUsePrediction::Result(LocalInteractionResult::Pass)
}

fn success() -> LocalTargetUsePrediction {
    LocalTargetUsePrediction::Result(LocalInteractionResult::Success(
        LocalInteractionSuccess::client_item_interaction(),
    ))
}

fn consume() -> LocalTargetUsePrediction {
    LocalTargetUsePrediction::Result(LocalInteractionResult::Success(
        LocalInteractionSuccess::consume(false),
    ))
}

fn try_empty_hand() -> LocalTargetUsePrediction {
    LocalTargetUsePrediction::Result(LocalInteractionResult::TryEmptyHandInteraction)
}
