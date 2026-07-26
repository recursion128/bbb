use super::{
    LocalBlockUseContext, LocalInteractionParticleAction, LocalInteractionResult,
    LocalInteractionSoundAction, LocalInteractionSuccess, LocalTargetUsePrediction,
};
use crate::WorldStore;
use bbb_protocol::{
    block_entity_types::{
        VANILLA_BLOCK_ENTITY_TYPE_CHISELED_BOOKSHELF_ID,
        VANILLA_BLOCK_ENTITY_TYPE_DECORATED_POT_ID, VANILLA_BLOCK_ENTITY_TYPE_HANGING_SIGN_ID,
        VANILLA_BLOCK_ENTITY_TYPE_SIGN_ID,
    },
    block_types::{BlockTypeCatalogEntry, VanillaBlockUseItemOnOverride, VANILLA_BLOCK_TYPES},
    item_types::{vanilla_item_type, VANILLA_ITEM_GLOWSTONE_ID},
    packets::InteractionHand,
};
use std::collections::BTreeMap;

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
            VanillaBlockUseItemOnOverride::ChiseledBookShelfBlock
                if self.local_item_in_hand(hand).is_none() =>
            {
                if self.block_entity_type_id_at(context.pos)
                    == Some(VANILLA_BLOCK_ENTITY_TYPE_CHISELED_BOOKSHELF_ID)
                {
                    try_empty_hand()
                } else {
                    pass()
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
            VanillaBlockUseItemOnOverride::ComposterBlock
            | VanillaBlockUseItemOnOverride::NoteBlock
            | VanillaBlockUseItemOnOverride::SweetBerryBushBlock
                if self.local_item_in_hand(hand).is_none() =>
            {
                try_empty_hand()
            }
            VanillaBlockUseItemOnOverride::DecoratedPotBlock
                if self.local_item_in_hand(hand).is_none() =>
            {
                if self.block_entity_type_id_at(context.pos)
                    == Some(VANILLA_BLOCK_ENTITY_TYPE_DECORATED_POT_ID)
                {
                    success()
                } else {
                    pass()
                }
            }
            VanillaBlockUseItemOnOverride::JukeboxBlock
                if self.local_item_in_hand(hand).is_none() =>
            {
                match block.block_properties.get("has_record").map(String::as_str) {
                    Some("false" | "true") => try_empty_hand(),
                    _ => unsupported(),
                }
            }
            VanillaBlockUseItemOnOverride::LecternBlock
                if self.local_item_in_hand(hand).is_none() =>
            {
                match block.block_properties.get("has_book").map(String::as_str) {
                    Some("true") => try_empty_hand(),
                    Some("false") if hand == InteractionHand::MainHand => pass(),
                    Some("false") => try_empty_hand(),
                    _ => unsupported(),
                }
            }
            VanillaBlockUseItemOnOverride::RespawnAnchorBlock
                if self.local_item_in_hand(hand).is_none() =>
            {
                let charges = block
                    .block_properties
                    .get("charges")
                    .and_then(|charges| charges.parse::<u8>().ok())
                    .filter(|charges| *charges <= 4);
                match charges {
                    Some(charges)
                        if hand == InteractionHand::MainHand
                            && charges < 4
                            && self
                                .local_item_in_hand(InteractionHand::OffHand)
                                .and_then(|stack| stack.item_id)
                                == Some(VANILLA_ITEM_GLOWSTONE_ID) =>
                    {
                        pass()
                    }
                    Some(_) => try_empty_hand(),
                    None => unsupported(),
                }
            }
            VanillaBlockUseItemOnOverride::SignBlock
            | VanillaBlockUseItemOnOverride::CeilingHangingSignBlock
            | VanillaBlockUseItemOnOverride::WallHangingSignBlock
                if self.local_item_in_hand(hand).is_none() =>
            {
                if !matches!(
                    self.block_entity_type_id_at(context.pos),
                    Some(
                        VANILLA_BLOCK_ENTITY_TYPE_SIGN_ID
                            | VANILLA_BLOCK_ENTITY_TYPE_HANGING_SIGN_ID
                    )
                ) {
                    return pass();
                }
                match self.sign_text_state_at(context.pos) {
                    Some(text) if text.is_waxed => success(),
                    Some(_) => consume(),
                    None => unsupported(),
                }
            }
            _ => unsupported(),
        }
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
