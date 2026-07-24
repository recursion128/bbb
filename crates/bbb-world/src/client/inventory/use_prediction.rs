use super::*;
use bbb_protocol::{
    data_component_types::{
        VANILLA_DATA_COMPONENT_BLOCKS_ATTACKS_TYPE_ID,
        VANILLA_DATA_COMPONENT_KINETIC_WEAPON_TYPE_ID, VANILLA_DATA_COMPONENT_USE_COOLDOWN_TYPE_ID,
    },
    item_types::VanillaItemUseOverride,
    packets::ConsumableSummary,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LocalInteractionSwingSource {
    None,
    Client,
    Server,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LocalInteractionSuccess {
    pub swing_source: LocalInteractionSwingSource,
    pub was_item_interaction: bool,
    pub starts_using_item: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LocalInteractionResult {
    Success(LocalInteractionSuccess),
    Fail,
    Pass,
    TryEmptyHandInteraction,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LocalItemUsePrediction {
    Result(LocalInteractionResult),
    UnsupportedOverride(VanillaItemUseOverride),
    UnknownItemType(Option<i32>),
    MissingComponentValue(i32),
}

impl LocalInteractionSuccess {
    const fn consume(starts_using_item: bool) -> Self {
        Self {
            swing_source: LocalInteractionSwingSource::None,
            was_item_interaction: true,
            starts_using_item,
        }
    }
}

impl WorldStore {
    /// Predicts vanilla `ItemStack.use` for item classes that inherit
    /// `Item.use`. Source-derived override ownership keeps specialized item
    /// classes explicit instead of guessing their result from components.
    pub fn predict_local_item_use(&self, hand: InteractionHand) -> LocalItemUsePrediction {
        let Some(stack) = self.local_item_in_hand(hand) else {
            return LocalItemUsePrediction::Result(LocalInteractionResult::Pass);
        };
        let Some(item_id) = stack.item_id else {
            return LocalItemUsePrediction::UnknownItemType(None);
        };
        let Some(item_type) = vanilla_item_type(item_id) else {
            return LocalItemUsePrediction::UnknownItemType(Some(item_id));
        };
        if let Some(use_override) = item_type.use_override {
            return LocalItemUsePrediction::UnsupportedOverride(use_override);
        }
        if self.local_item_stack_is_on_cooldown(stack) {
            return LocalItemUsePrediction::Result(LocalInteractionResult::Pass);
        }

        if self.item_stack_has_effective_component(stack, VANILLA_DATA_COMPONENT_CONSUMABLE_TYPE_ID)
        {
            if !self.local_item_in_hand_can_start_consuming(hand) {
                return LocalItemUsePrediction::Result(LocalInteractionResult::Fail);
            }
            let Some(consumable) = self.effective_item_consumable(stack) else {
                return LocalItemUsePrediction::MissingComponentValue(
                    VANILLA_DATA_COMPONENT_CONSUMABLE_TYPE_ID,
                );
            };
            return LocalItemUsePrediction::Result(LocalInteractionResult::Success(
                LocalInteractionSuccess::consume(consumable.consume_seconds > 0.0),
            ));
        }

        // `Equippable.swapWithEquipmentSlot` depends on the target equipment
        // slot, enchantments, allowed entity set, and local inventory mutation.
        // Preserve that uncertainty until its prediction owner is implemented.
        if self.item_stack_has_effective_component(stack, VANILLA_DATA_COMPONENT_EQUIPPABLE_TYPE_ID)
        {
            return LocalItemUsePrediction::MissingComponentValue(
                VANILLA_DATA_COMPONENT_EQUIPPABLE_TYPE_ID,
            );
        }

        if self.item_stack_has_effective_component(
            stack,
            VANILLA_DATA_COMPONENT_BLOCKS_ATTACKS_TYPE_ID,
        ) || self.item_stack_has_effective_component(
            stack,
            VANILLA_DATA_COMPONENT_KINETIC_WEAPON_TYPE_ID,
        ) {
            return LocalItemUsePrediction::Result(LocalInteractionResult::Success(
                LocalInteractionSuccess::consume(true),
            ));
        }

        LocalItemUsePrediction::Result(LocalInteractionResult::Pass)
    }

    fn effective_item_consumable<'a>(
        &'a self,
        stack: &'a ProtocolItemStackSummary,
    ) -> Option<&'a ConsumableSummary> {
        if stack
            .component_patch
            .added_type_ids
            .contains(&VANILLA_DATA_COMPONENT_CONSUMABLE_TYPE_ID)
        {
            return stack.component_patch.consumable.as_ref();
        }
        stack
            .item_id
            .and_then(|item_id| self.items.default_item_consumables.get(&item_id))
    }

    fn local_item_stack_is_on_cooldown(&self, stack: &ProtocolItemStackSummary) -> bool {
        if !self
            .item_stack_has_effective_component(stack, VANILLA_DATA_COMPONENT_USE_COOLDOWN_TYPE_ID)
        {
            return false;
        }
        let effective = self.effective_item_stack(stack);
        let cooldown_group = effective
            .component_patch
            .use_cooldown_group
            .as_deref()
            .or_else(|| {
                stack
                    .item_id
                    .and_then(vanilla_item_type)
                    .map(|item| item.resource_id)
            });
        cooldown_group.is_some_and(|group| self.cooldown(group).is_some())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bbb_protocol::{
        data_component_types::{
            VANILLA_DATA_COMPONENT_FOOD_TYPE_ID, VANILLA_DATA_COMPONENT_USE_COOLDOWN_TYPE_ID,
        },
        item_types::{
            VANILLA_ITEM_APPLE_ID, VANILLA_ITEM_CHORUS_FRUIT_ID, VANILLA_ITEM_SNOWBALL_ID,
            VANILLA_ITEM_STONE_ID,
        },
        packets::{
            Cooldown, DataComponentPatchSummary, ItemStackSummary, PlayerHealth, SetPlayerInventory,
        },
    };
    use std::collections::{BTreeMap, BTreeSet};

    fn stack(item_id: i32) -> ItemStackSummary {
        ItemStackSummary {
            item_id: Some(item_id),
            count: 1,
            component_patch: DataComponentPatchSummary::default(),
        }
    }

    fn set_main_hand(world: &mut WorldStore, item_id: i32) {
        world.apply_set_player_inventory(SetPlayerInventory {
            slot: 0,
            item: stack(item_id),
        });
    }

    #[test]
    fn generic_empty_and_componentless_item_use_predict_pass() {
        let mut world = WorldStore::new();
        assert_eq!(
            world.predict_local_item_use(InteractionHand::MainHand),
            LocalItemUsePrediction::Result(LocalInteractionResult::Pass)
        );

        set_main_hand(&mut world, VANILLA_ITEM_STONE_ID);
        assert_eq!(
            world.predict_local_item_use(InteractionHand::MainHand),
            LocalItemUsePrediction::Result(LocalInteractionResult::Pass)
        );
    }

    #[test]
    fn generic_use_preserves_component_order_and_unknown_item_boundary() {
        let mut world = WorldStore::new();
        world.set_default_item_component_type_ids(BTreeMap::from([(
            VANILLA_ITEM_STONE_ID,
            BTreeSet::from([VANILLA_DATA_COMPONENT_BLOCKS_ATTACKS_TYPE_ID]),
        )]));
        set_main_hand(&mut world, VANILLA_ITEM_STONE_ID);
        assert_eq!(
            world.predict_local_item_use(InteractionHand::MainHand),
            LocalItemUsePrediction::Result(LocalInteractionResult::Success(
                LocalInteractionSuccess::consume(true)
            ))
        );

        world.set_default_item_component_type_ids(BTreeMap::from([(
            VANILLA_ITEM_STONE_ID,
            BTreeSet::from([
                VANILLA_DATA_COMPONENT_EQUIPPABLE_TYPE_ID,
                VANILLA_DATA_COMPONENT_BLOCKS_ATTACKS_TYPE_ID,
            ]),
        )]));
        assert_eq!(
            world.predict_local_item_use(InteractionHand::MainHand),
            LocalItemUsePrediction::MissingComponentValue(
                VANILLA_DATA_COMPONENT_EQUIPPABLE_TYPE_ID
            )
        );

        set_main_hand(&mut world, 99_999);
        assert_eq!(
            world.predict_local_item_use(InteractionHand::MainHand),
            LocalItemUsePrediction::UnknownItemType(Some(99_999))
        );
    }

    #[test]
    fn generic_consumable_predicts_food_gate_and_using_duration() {
        let mut world = WorldStore::new();
        world.set_default_item_component_type_ids(BTreeMap::from([(
            VANILLA_ITEM_APPLE_ID,
            BTreeSet::from([
                VANILLA_DATA_COMPONENT_CONSUMABLE_TYPE_ID,
                VANILLA_DATA_COMPONENT_FOOD_TYPE_ID,
            ]),
        )]));
        world.set_default_item_consumables(BTreeMap::from([(
            VANILLA_ITEM_APPLE_ID,
            ConsumableSummary {
                consume_seconds: 1.6,
                ..ConsumableSummary::default()
            },
        )]));
        world.set_default_item_food_properties(BTreeMap::from([(
            VANILLA_ITEM_APPLE_ID,
            ItemFoodProperties {
                nutrition: 4,
                saturation_bits: 2.4_f32.to_bits(),
                can_always_eat: false,
            },
        )]));
        set_main_hand(&mut world, VANILLA_ITEM_APPLE_ID);

        assert_eq!(
            world.predict_local_item_use(InteractionHand::MainHand),
            LocalItemUsePrediction::Result(LocalInteractionResult::Fail)
        );
        world.apply_player_health(PlayerHealth {
            health: 20.0,
            food: 10,
            saturation: 5.0,
        });
        assert_eq!(
            world.predict_local_item_use(InteractionHand::MainHand),
            LocalItemUsePrediction::Result(LocalInteractionResult::Success(
                LocalInteractionSuccess::consume(true)
            ))
        );
    }

    #[test]
    fn item_use_override_and_active_cooldown_remain_explicit() {
        let mut world = WorldStore::new();
        set_main_hand(&mut world, VANILLA_ITEM_SNOWBALL_ID);
        assert_eq!(
            world.predict_local_item_use(InteractionHand::MainHand),
            LocalItemUsePrediction::UnsupportedOverride(VanillaItemUseOverride::SnowballItem)
        );

        world.set_default_item_component_type_ids(BTreeMap::from([(
            VANILLA_ITEM_CHORUS_FRUIT_ID,
            BTreeSet::from([
                VANILLA_DATA_COMPONENT_CONSUMABLE_TYPE_ID,
                VANILLA_DATA_COMPONENT_USE_COOLDOWN_TYPE_ID,
            ]),
        )]));
        set_main_hand(&mut world, VANILLA_ITEM_CHORUS_FRUIT_ID);
        world.apply_cooldown(Cooldown {
            cooldown_group: "minecraft:chorus_fruit".to_string(),
            duration: 20,
        });
        assert_eq!(
            world.predict_local_item_use(InteractionHand::MainHand),
            LocalItemUsePrediction::Result(LocalInteractionResult::Pass)
        );
    }
}
