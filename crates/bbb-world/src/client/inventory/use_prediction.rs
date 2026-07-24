use super::*;
use crate::{
    entities::{
        vanilla_entity_metadata_interact_owner, vanilla_entity_metadata_type_is_leashable,
        vanilla_entity_metadata_type_is_living,
    },
    BlockPos,
};
use bbb_protocol::{
    block_types::{
        vanilla_block_type_for_resource_id, BlockTypeCatalogEntry, VanillaBlockUseItemOnOverride,
        VanillaBlockUseWithoutItemOverride,
    },
    data_component_types::{
        VANILLA_DATA_COMPONENT_BLOCKS_ATTACKS_TYPE_ID,
        VANILLA_DATA_COMPONENT_KINETIC_WEAPON_TYPE_ID, VANILLA_DATA_COMPONENT_USE_COOLDOWN_TYPE_ID,
    },
    item_types::{
        VanillaItemInteractLivingEntityOverride, VanillaItemUseOnOverride, VanillaItemUseOverride,
        VANILLA_ITEM_SHEARS_ID,
    },
    packets::ConsumableSummary,
};

const VANILLA_DEFAULT_ENTITY_INTERACT_OWNER: &str = "net.minecraft.world.entity.Entity";

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LocalTargetUsePrediction {
    Result(LocalInteractionResult),
    UnsupportedBlockUseItemOn(VanillaBlockUseItemOnOverride),
    UnsupportedBlockUseWithoutItem(VanillaBlockUseWithoutItemOverride),
    UnsupportedItemUseOn(VanillaItemUseOnOverride),
    UnsupportedEntityInteract(&'static str),
    UnsupportedItemInteractLivingEntity(VanillaItemInteractLivingEntityOverride),
    UnknownBlockType(Option<i32>),
    UnknownEntityType(Option<i32>),
    UnknownItemType(Option<i32>),
}

impl LocalInteractionSuccess {
    const fn consume(starts_using_item: bool) -> Self {
        Self {
            swing_source: LocalInteractionSwingSource::None,
            was_item_interaction: true,
            starts_using_item,
        }
    }

    const fn consume_without_item() -> Self {
        Self {
            swing_source: LocalInteractionSwingSource::None,
            was_item_interaction: false,
            starts_using_item: false,
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

    /// Predicts the client-side `MultiPlayerGameMode.performUseItemOn` result
    /// only when every reached vanilla behavior inherits its default
    /// implementation. Generated override ownership keeps specialized block
    /// and item behavior explicit.
    pub fn predict_local_block_use(
        &self,
        pos: BlockPos,
        hand: InteractionHand,
        secondary_use_active: bool,
    ) -> LocalTargetUsePrediction {
        if self.local_player_is_spectator() {
            return LocalTargetUsePrediction::Result(LocalInteractionResult::Success(
                LocalInteractionSuccess::consume_without_item(),
            ));
        }
        let hands_have_item = self.local_item_in_hand(InteractionHand::MainHand).is_some()
            || self.local_item_in_hand(InteractionHand::OffHand).is_some();
        if secondary_use_active && hands_have_item {
            return self.predict_local_item_use_on(hand);
        }
        let Some(block) = self.probe_block(pos) else {
            return LocalTargetUsePrediction::UnknownBlockType(None);
        };
        let Some(block_name) = block.block_name.as_deref() else {
            return LocalTargetUsePrediction::UnknownBlockType(Some(block.block_state_id));
        };
        let Some(block_type) = vanilla_block_type_for_resource_id(block_name) else {
            return LocalTargetUsePrediction::UnknownBlockType(Some(block.block_state_id));
        };
        self.predict_local_block_use_for_type(block_type, hand)
    }

    fn predict_local_block_use_for_type(
        &self,
        block_type: &BlockTypeCatalogEntry,
        hand: InteractionHand,
    ) -> LocalTargetUsePrediction {
        if !self.is_feature_enabled("minecraft:vanilla") {
            return LocalTargetUsePrediction::Result(LocalInteractionResult::Fail);
        }
        if let Some(owner) = block_type.use_item_on_override {
            return LocalTargetUsePrediction::UnsupportedBlockUseItemOn(owner);
        }
        if hand == InteractionHand::MainHand {
            if let Some(owner) = block_type.use_without_item_override {
                return LocalTargetUsePrediction::UnsupportedBlockUseWithoutItem(owner);
            }
        }
        self.predict_local_item_use_on(hand)
    }

    fn predict_local_item_use_on(&self, hand: InteractionHand) -> LocalTargetUsePrediction {
        let Some(stack) = self.local_item_in_hand(hand) else {
            return LocalTargetUsePrediction::Result(LocalInteractionResult::Pass);
        };
        if self.local_item_stack_is_on_cooldown(stack) {
            return LocalTargetUsePrediction::Result(LocalInteractionResult::Pass);
        }
        let Some(item_id) = stack.item_id else {
            return LocalTargetUsePrediction::UnknownItemType(None);
        };
        let Some(item_type) = vanilla_item_type(item_id) else {
            return LocalTargetUsePrediction::UnknownItemType(Some(item_id));
        };
        if let Some(owner) = item_type.use_on_override {
            return LocalTargetUsePrediction::UnsupportedItemUseOn(owner);
        }
        LocalTargetUsePrediction::Result(LocalInteractionResult::Pass)
    }

    /// Predicts `Player.interactOn` only across source-derived behavior paths
    /// whose client result is known. Specialized `Entity.interact` owners,
    /// leash/shears behavior, and specialized item interactions stay explicit
    /// until their local state transitions are implemented.
    pub fn predict_local_entity_interaction(
        &self,
        entity_id: i32,
        hand: InteractionHand,
    ) -> LocalTargetUsePrediction {
        if self.local_player_is_spectator() {
            return LocalTargetUsePrediction::Result(LocalInteractionResult::Pass);
        }
        let Some(entity_type_id) = self.entity_type_id(entity_id) else {
            return LocalTargetUsePrediction::UnknownEntityType(None);
        };
        let Some(owner) = vanilla_entity_metadata_interact_owner(entity_type_id) else {
            return LocalTargetUsePrediction::UnknownEntityType(Some(entity_type_id));
        };
        let held_item_id = self
            .local_item_in_hand(hand)
            .and_then(|stack| stack.item_id);
        if owner != VANILLA_DEFAULT_ENTITY_INTERACT_OWNER
            || vanilla_entity_metadata_type_is_leashable(entity_type_id)
            || held_item_id == Some(VANILLA_ITEM_SHEARS_ID)
        {
            return LocalTargetUsePrediction::UnsupportedEntityInteract(owner);
        }
        if !vanilla_entity_metadata_type_is_living(entity_type_id) {
            return LocalTargetUsePrediction::Result(LocalInteractionResult::Pass);
        }
        let Some(stack) = self.local_item_in_hand(hand) else {
            return LocalTargetUsePrediction::Result(LocalInteractionResult::Pass);
        };
        let Some(item_id) = stack.item_id else {
            return LocalTargetUsePrediction::UnknownItemType(None);
        };
        let Some(item_type) = vanilla_item_type(item_id) else {
            return LocalTargetUsePrediction::UnknownItemType(Some(item_id));
        };
        if let Some(owner) = item_type.interact_living_entity_override {
            return LocalTargetUsePrediction::UnsupportedItemInteractLivingEntity(owner);
        }
        LocalTargetUsePrediction::Result(LocalInteractionResult::Pass)
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
        block_types::{
            vanilla_block_type, VanillaBlockUseItemOnOverride, VanillaBlockUseWithoutItemOverride,
            VANILLA_BLOCK_CAKE_ID, VANILLA_BLOCK_CHEST_ID, VANILLA_BLOCK_STONE_ID,
        },
        data_component_types::{
            VANILLA_DATA_COMPONENT_FOOD_TYPE_ID, VANILLA_DATA_COMPONENT_USE_COOLDOWN_TYPE_ID,
        },
        entity_types::{
            VANILLA_ENTITY_TYPE_AREA_EFFECT_CLOUD_ID, VANILLA_ENTITY_TYPE_COW_ID,
            VANILLA_ENTITY_TYPE_MANNEQUIN_ID,
        },
        item_types::{
            VanillaItemInteractLivingEntityOverride, VanillaItemUseOnOverride,
            VANILLA_ITEM_APPLE_ID, VANILLA_ITEM_CHORUS_FRUIT_ID, VANILLA_ITEM_RED_DYE_ID,
            VANILLA_ITEM_SHEARS_ID, VANILLA_ITEM_SNOWBALL_ID, VANILLA_ITEM_STONE_ID,
        },
        packets::{
            AddEntity, Cooldown, DataComponentPatchSummary, ItemStackSummary, PlayerHealth,
            SetPlayerInventory, Vec3d,
        },
    };
    use std::collections::{BTreeMap, BTreeSet};
    use uuid::Uuid;

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

    fn add_entity(world: &mut WorldStore, id: i32, entity_type_id: i32) {
        world.apply_add_entity(AddEntity {
            id,
            uuid: Uuid::from_u128(id as u128),
            entity_type_id,
            position: Vec3d::default(),
            delta_movement: Vec3d::default(),
            x_rot: 0.0,
            y_rot: 0.0,
            y_head_rot: 0.0,
            data: 0,
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

    #[test]
    fn block_use_prediction_preserves_vanilla_dispatch_order() {
        let mut world = WorldStore::new();
        let stone = vanilla_block_type(VANILLA_BLOCK_STONE_ID).unwrap();
        assert_eq!(
            world.predict_local_block_use_for_type(stone, InteractionHand::MainHand),
            LocalTargetUsePrediction::Result(LocalInteractionResult::Pass)
        );

        let chest = vanilla_block_type(VANILLA_BLOCK_CHEST_ID).unwrap();
        assert_eq!(
            world.predict_local_block_use_for_type(chest, InteractionHand::MainHand),
            LocalTargetUsePrediction::UnsupportedBlockUseWithoutItem(
                VanillaBlockUseWithoutItemOverride::ChestBlock
            )
        );
        assert_eq!(
            world.predict_local_block_use_for_type(chest, InteractionHand::OffHand),
            LocalTargetUsePrediction::Result(LocalInteractionResult::Pass)
        );

        let cake = vanilla_block_type(VANILLA_BLOCK_CAKE_ID).unwrap();
        assert_eq!(
            world.predict_local_block_use_for_type(cake, InteractionHand::MainHand),
            LocalTargetUsePrediction::UnsupportedBlockUseItemOn(
                VanillaBlockUseItemOnOverride::CakeBlock
            )
        );

        set_main_hand(&mut world, VANILLA_ITEM_STONE_ID);
        assert_eq!(
            world.predict_local_block_use_for_type(stone, InteractionHand::MainHand),
            LocalTargetUsePrediction::UnsupportedItemUseOn(VanillaItemUseOnOverride::BlockItem)
        );
        assert_eq!(
            world.predict_local_block_use(
                BlockPos { x: 32, y: 0, z: 32 },
                InteractionHand::MainHand,
                true,
            ),
            LocalTargetUsePrediction::UnsupportedItemUseOn(VanillaItemUseOnOverride::BlockItem)
        );
    }

    #[test]
    fn entity_interaction_prediction_keeps_specialized_and_leash_edges_explicit() {
        let mut world = WorldStore::new();
        add_entity(&mut world, 1, VANILLA_ENTITY_TYPE_AREA_EFFECT_CLOUD_ID);
        add_entity(&mut world, 2, VANILLA_ENTITY_TYPE_COW_ID);
        add_entity(&mut world, 3, VANILLA_ENTITY_TYPE_MANNEQUIN_ID);

        assert_eq!(
            world.predict_local_entity_interaction(1, InteractionHand::MainHand),
            LocalTargetUsePrediction::Result(LocalInteractionResult::Pass)
        );
        assert_eq!(
            world.predict_local_entity_interaction(2, InteractionHand::MainHand),
            LocalTargetUsePrediction::UnsupportedEntityInteract("net.minecraft.world.entity.Mob")
        );
        assert_eq!(
            world.predict_local_entity_interaction(99, InteractionHand::MainHand),
            LocalTargetUsePrediction::UnknownEntityType(None)
        );

        set_main_hand(&mut world, VANILLA_ITEM_SHEARS_ID);
        assert_eq!(
            world.predict_local_entity_interaction(1, InteractionHand::MainHand),
            LocalTargetUsePrediction::UnsupportedEntityInteract(
                VANILLA_DEFAULT_ENTITY_INTERACT_OWNER
            )
        );
        set_main_hand(&mut world, VANILLA_ITEM_RED_DYE_ID);
        assert_eq!(
            world.predict_local_entity_interaction(3, InteractionHand::MainHand),
            LocalTargetUsePrediction::UnsupportedItemInteractLivingEntity(
                VanillaItemInteractLivingEntityOverride::DyeItem
            )
        );
    }
}
