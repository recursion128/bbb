use super::*;
use crate::{
    entities::{
        vanilla_entity_metadata_abstract_horse_accessors_for_entity_type,
        vanilla_entity_metadata_age_locked_accessor_for_entity_type,
        vanilla_entity_metadata_armor_stand_accessors_for_entity_type,
        vanilla_entity_metadata_interact_owner,
        vanilla_entity_metadata_interaction_accessors_for_entity_type,
        vanilla_entity_metadata_mob_interact_owner, vanilla_entity_metadata_type_is_leashable,
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
        VANILLA_ITEM_LEAD_ID, VANILLA_ITEM_SHEARS_ID,
    },
    packets::ConsumableSummary,
};

const VANILLA_DEFAULT_ENTITY_INTERACT_OWNER: &str = "net.minecraft.world.entity.Entity";
const VANILLA_MOB_ENTITY_INTERACT_OWNER: &str = "net.minecraft.world.entity.Mob";
const VANILLA_ARMOR_STAND_INTERACT_OWNER: &str = "net.minecraft.world.entity.decoration.ArmorStand";
const VANILLA_BOAT_INTERACT_OWNER: &str = "net.minecraft.world.entity.vehicle.boat.AbstractBoat";
const VANILLA_CAMEL_HUSK_INTERACT_OWNER: &str = "net.minecraft.world.entity.animal.camel.CamelHusk";
const VANILLA_CAMEL_MOB_INTERACT_OWNER: &str = "net.minecraft.world.entity.animal.camel.Camel";
const VANILLA_CAMEL_HUSK_FOOD_TAG: &str = "minecraft:camel_husk_food";
const VANILLA_CHEST_BOAT_INTERACT_OWNER: &str =
    "net.minecraft.world.entity.vehicle.boat.AbstractChestBoat";
const VANILLA_INTERACTION_ENTITY_INTERACT_OWNER: &str = "net.minecraft.world.entity.Interaction";
const VANILLA_ITEM_FRAME_INTERACT_OWNER: &str = "net.minecraft.world.entity.decoration.ItemFrame";
const VANILLA_LEASH_KNOT_INTERACT_OWNER: &str =
    "net.minecraft.world.entity.decoration.LeashFenceKnotEntity";
const VANILLA_NAUTILUS_INTERACT_OWNER: &str =
    "net.minecraft.world.entity.animal.nautilus.AbstractNautilus";
const VANILLA_MINECART_INTERACT_OWNER: &str =
    "net.minecraft.world.entity.vehicle.minecart.Minecart";
const VANILLA_MINECART_CONTAINER_INTERACT_OWNER: &str =
    "net.minecraft.world.entity.vehicle.minecart.AbstractMinecartContainer";
const VANILLA_MINECART_CHEST_INTERACT_OWNER: &str =
    "net.minecraft.world.entity.vehicle.minecart.MinecartChest";
const VANILLA_MINECART_COMMAND_BLOCK_INTERACT_OWNER: &str =
    "net.minecraft.world.entity.vehicle.minecart.MinecartCommandBlock";
const VANILLA_MINECART_FURNACE_INTERACT_OWNER: &str =
    "net.minecraft.world.entity.vehicle.minecart.MinecartFurnace";
const VANILLA_ZOMBIE_HORSE_INTERACT_OWNER: &str =
    "net.minecraft.world.entity.animal.equine.ZombieHorse";
const VANILLA_ZOMBIE_HORSE_FOOD_TAG: &str = "minecraft:zombie_horse_food";

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

    const fn server_item_interaction() -> Self {
        Self {
            swing_source: LocalInteractionSwingSource::Server,
            was_item_interaction: true,
            starts_using_item: false,
        }
    }

    const fn client_without_item() -> Self {
        Self {
            swing_source: LocalInteractionSwingSource::Client,
            was_item_interaction: false,
            starts_using_item: false,
        }
    }

    const fn client_item_interaction() -> Self {
        Self {
            swing_source: LocalInteractionSwingSource::Client,
            was_item_interaction: true,
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
        secondary_use_active: bool,
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
        let interact_result = if matches!(
            owner,
            VANILLA_MOB_ENTITY_INTERACT_OWNER
                | VANILLA_CAMEL_HUSK_INTERACT_OWNER
                | VANILLA_NAUTILUS_INTERACT_OWNER
                | VANILLA_ZOMBIE_HORSE_INTERACT_OWNER
        ) {
            self.predict_local_mob_interact(
                entity_id,
                entity_type_id,
                hand,
                owner,
                secondary_use_active,
            )
        } else if owner == VANILLA_DEFAULT_ENTITY_INTERACT_OWNER {
            self.predict_local_base_entity_interact(entity_id, entity_type_id, held_item_id)
        } else if owner == VANILLA_ARMOR_STAND_INTERACT_OWNER {
            self.predict_local_armor_stand_interact(entity_id, entity_type_id, held_item_id)
        } else if owner == VANILLA_INTERACTION_ENTITY_INTERACT_OWNER {
            self.predict_local_interaction_entity_interact(entity_id, entity_type_id)
        } else if owner == VANILLA_ITEM_FRAME_INTERACT_OWNER {
            self.predict_local_item_frame_interact(entity_id, entity_type_id, held_item_id)
        } else if owner == VANILLA_LEASH_KNOT_INTERACT_OWNER {
            LocalTargetUsePrediction::Result(LocalInteractionResult::Success(
                LocalInteractionSuccess::client_item_interaction(),
            ))
        } else if let Some(result) =
            self.predict_local_minecart_interact(owner, entity_id, secondary_use_active)
        {
            result
        } else if let Some(result) = self.predict_local_boat_interact(
            owner,
            entity_id,
            entity_type_id,
            held_item_id,
            secondary_use_active,
        ) {
            result
        } else {
            return LocalTargetUsePrediction::UnsupportedEntityInteract(owner);
        };
        if interact_result != LocalTargetUsePrediction::Result(LocalInteractionResult::Pass) {
            return interact_result;
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

    fn predict_local_armor_stand_interact(
        &self,
        entity_id: i32,
        entity_type_id: i32,
        held_item_id: Option<i32>,
    ) -> LocalTargetUsePrediction {
        let Some(accessors) =
            vanilla_entity_metadata_armor_stand_accessors_for_entity_type(entity_type_id)
        else {
            return LocalTargetUsePrediction::UnknownEntityType(Some(entity_type_id));
        };
        let Some(flags) = self.entities.metadata_byte_for_entity(
            entity_id,
            accessors.client_flags_data_id,
            accessors.client_flags_default,
        ) else {
            return LocalTargetUsePrediction::UnknownEntityType(Some(entity_type_id));
        };
        let held_item_type = match held_item_id {
            Some(item_id) => {
                let Some(item_type) = vanilla_item_type(item_id) else {
                    return LocalTargetUsePrediction::UnknownItemType(Some(item_id));
                };
                Some(item_type)
            }
            None => None,
        };
        let marker = flags & accessors.marker_mask != 0;
        let held_is_name_tag = held_item_type.is_some_and(|item_type| {
            item_type.interact_living_entity_override
                == Some(VanillaItemInteractLivingEntityOverride::NameTagItem)
        });
        if marker || held_is_name_tag {
            self.predict_local_base_entity_interact(entity_id, entity_type_id, held_item_id)
        } else {
            LocalTargetUsePrediction::Result(LocalInteractionResult::Success(
                LocalInteractionSuccess::server_item_interaction(),
            ))
        }
    }

    fn predict_local_interaction_entity_interact(
        &self,
        entity_id: i32,
        entity_type_id: i32,
    ) -> LocalTargetUsePrediction {
        let Some(accessors) =
            vanilla_entity_metadata_interaction_accessors_for_entity_type(entity_type_id)
        else {
            return LocalTargetUsePrediction::UnknownEntityType(Some(entity_type_id));
        };
        let Some(response) = self.entities.metadata_bool_for_entity(
            entity_id,
            accessors.response_data_id,
            accessors.response_default,
        ) else {
            return LocalTargetUsePrediction::UnknownEntityType(Some(entity_type_id));
        };
        LocalTargetUsePrediction::Result(LocalInteractionResult::Success(if response {
            LocalInteractionSuccess::client_item_interaction()
        } else {
            LocalInteractionSuccess::consume(false)
        }))
    }

    fn predict_local_item_frame_interact(
        &self,
        entity_id: i32,
        entity_type_id: i32,
        held_item_id: Option<i32>,
    ) -> LocalTargetUsePrediction {
        let Some(frame_has_item) = self.entities.item_frame_has_item(entity_id) else {
            return LocalTargetUsePrediction::UnknownEntityType(Some(entity_type_id));
        };
        if !frame_has_item && held_item_id.is_none() {
            LocalTargetUsePrediction::Result(LocalInteractionResult::Pass)
        } else {
            LocalTargetUsePrediction::Result(LocalInteractionResult::Success(
                LocalInteractionSuccess::client_item_interaction(),
            ))
        }
    }

    fn predict_local_minecart_interact(
        &self,
        owner: &'static str,
        entity_id: i32,
        secondary_use_active: bool,
    ) -> Option<LocalTargetUsePrediction> {
        let success = || {
            LocalTargetUsePrediction::Result(LocalInteractionResult::Success(
                LocalInteractionSuccess::client_item_interaction(),
            ))
        };
        match owner {
            VANILLA_MINECART_INTERACT_OWNER => Some(
                if secondary_use_active || self.entities.entity_has_passengers(entity_id) {
                    LocalTargetUsePrediction::Result(LocalInteractionResult::Pass)
                } else {
                    success()
                },
            ),
            VANILLA_MINECART_CONTAINER_INTERACT_OWNER
            | VANILLA_MINECART_CHEST_INTERACT_OWNER
            | VANILLA_MINECART_FURNACE_INTERACT_OWNER => Some(success()),
            VANILLA_MINECART_COMMAND_BLOCK_INTERACT_OWNER => {
                Some(if self.local_player_has_gamemaster_permission() {
                    success()
                } else {
                    LocalTargetUsePrediction::Result(LocalInteractionResult::Pass)
                })
            }
            _ => None,
        }
    }

    fn predict_local_boat_interact(
        &self,
        owner: &'static str,
        entity_id: i32,
        entity_type_id: i32,
        held_item_id: Option<i32>,
        secondary_use_active: bool,
    ) -> Option<LocalTargetUsePrediction> {
        if !matches!(
            owner,
            VANILLA_BOAT_INTERACT_OWNER | VANILLA_CHEST_BOAT_INTERACT_OWNER
        ) {
            return None;
        }
        let base_result =
            self.predict_local_base_entity_interact(entity_id, entity_type_id, held_item_id);
        if base_result != LocalTargetUsePrediction::Result(LocalInteractionResult::Pass) {
            return Some(base_result);
        }

        let success = || {
            LocalTargetUsePrediction::Result(LocalInteractionResult::Success(
                LocalInteractionSuccess::client_item_interaction(),
            ))
        };
        if owner == VANILLA_CHEST_BOAT_INTERACT_OWNER {
            // `AbstractChestBoat.interact` opens its container whenever
            // `AbstractBoat.interact` passes on the client: secondary use
            // intentionally skips riding, while 60 submerged ticks imply
            // `canAddPassenger == false`.
            return Some(success());
        }
        let Some(movement) = self.entities.boat_movement(entity_id) else {
            return Some(LocalTargetUsePrediction::UnknownEntityType(Some(
                entity_type_id,
            )));
        };
        Some(
            if secondary_use_active || movement.out_of_control_ticks >= 60.0 {
                LocalTargetUsePrediction::Result(LocalInteractionResult::Pass)
            } else {
                success()
            },
        )
    }

    fn predict_local_mob_interact(
        &self,
        entity_id: i32,
        entity_type_id: i32,
        hand: InteractionHand,
        interact_owner: &'static str,
        secondary_use_active: bool,
    ) -> LocalTargetUsePrediction {
        let Some(health) = self.entities.living_entity_health(entity_id) else {
            return LocalTargetUsePrediction::UnknownEntityType(Some(entity_type_id));
        };
        if health <= 0.0 {
            return LocalTargetUsePrediction::Result(LocalInteractionResult::Pass);
        }

        let held_item_id = self
            .local_item_in_hand(hand)
            .and_then(|stack| stack.item_id);
        if let Some(item_id) = held_item_id {
            let Some(item_type) = vanilla_item_type(item_id) else {
                return LocalTargetUsePrediction::UnknownItemType(Some(item_id));
            };
            if item_type.interact_living_entity_override
                == Some(VanillaItemInteractLivingEntityOverride::NameTagItem)
            {
                return LocalTargetUsePrediction::UnsupportedItemInteractLivingEntity(
                    VanillaItemInteractLivingEntityOverride::NameTagItem,
                );
            }
            if item_type.use_override == Some(VanillaItemUseOverride::SpawnEggItem) {
                return LocalTargetUsePrediction::Result(LocalInteractionResult::Success(
                    LocalInteractionSuccess::server_item_interaction(),
                ));
            }
        }

        let base_result =
            self.predict_local_base_entity_interact(entity_id, entity_type_id, held_item_id);
        if base_result != LocalTargetUsePrediction::Result(LocalInteractionResult::Pass) {
            return base_result;
        }

        let Some(owner) = vanilla_entity_metadata_mob_interact_owner(entity_type_id) else {
            return LocalTargetUsePrediction::UnknownEntityType(Some(entity_type_id));
        };
        if interact_owner == VANILLA_CAMEL_HUSK_INTERACT_OWNER
            && owner == VANILLA_CAMEL_MOB_INTERACT_OWNER
        {
            return self.predict_local_camel_husk_mob_interact(held_item_id, secondary_use_active);
        }
        if interact_owner == VANILLA_ZOMBIE_HORSE_INTERACT_OWNER
            && owner == VANILLA_ZOMBIE_HORSE_INTERACT_OWNER
        {
            return self.predict_local_zombie_horse_mob_interact(
                entity_id,
                entity_type_id,
                held_item_id,
                secondary_use_active,
            );
        }
        if owner != VANILLA_MOB_ENTITY_INTERACT_OWNER {
            return LocalTargetUsePrediction::UnsupportedEntityInteract(owner);
        }
        LocalTargetUsePrediction::Result(LocalInteractionResult::Pass)
    }

    fn predict_local_camel_husk_mob_interact(
        &self,
        held_item_id: Option<i32>,
        secondary_use_active: bool,
    ) -> LocalTargetUsePrediction {
        if secondary_use_active {
            return LocalTargetUsePrediction::Result(LocalInteractionResult::Success(
                LocalInteractionSuccess::client_item_interaction(),
            ));
        }
        let Some(item_id) = held_item_id else {
            return LocalTargetUsePrediction::Result(LocalInteractionResult::Success(
                LocalInteractionSuccess::consume(false),
            ));
        };
        let Some(item_type) = vanilla_item_type(item_id) else {
            return LocalTargetUsePrediction::UnknownItemType(Some(item_id));
        };
        if let Some(owner) = item_type.interact_living_entity_override {
            return LocalTargetUsePrediction::UnsupportedItemInteractLivingEntity(owner);
        }
        let Some(food_item_ids) =
            self.registry_holder_ids_for_tag("minecraft:item", VANILLA_CAMEL_HUSK_FOOD_TAG)
        else {
            return LocalTargetUsePrediction::UnsupportedEntityInteract(
                VANILLA_CAMEL_MOB_INTERACT_OWNER,
            );
        };
        if food_item_ids.contains(&item_id) {
            LocalTargetUsePrediction::Result(LocalInteractionResult::Success(
                LocalInteractionSuccess::server_item_interaction(),
            ))
        } else {
            LocalTargetUsePrediction::Result(LocalInteractionResult::Success(
                LocalInteractionSuccess::consume(false),
            ))
        }
    }

    fn predict_local_zombie_horse_mob_interact(
        &self,
        entity_id: i32,
        entity_type_id: i32,
        held_item_id: Option<i32>,
        secondary_use_active: bool,
    ) -> LocalTargetUsePrediction {
        let Some(is_baby) = self.entities.is_baby(entity_id) else {
            return LocalTargetUsePrediction::UnknownEntityType(Some(entity_type_id));
        };
        let Some(accessors) =
            vanilla_entity_metadata_abstract_horse_accessors_for_entity_type(entity_type_id)
        else {
            return LocalTargetUsePrediction::UnknownEntityType(Some(entity_type_id));
        };
        let Some(flags) = self.entities.metadata_byte_for_entity(
            entity_id,
            accessors.flags_data_id,
            accessors.flags_default,
        ) else {
            return LocalTargetUsePrediction::UnknownEntityType(Some(entity_type_id));
        };
        let is_tamed = flags & accessors.tame_mask != 0;
        let is_vehicle = self.entities.entity_has_passengers(entity_id);

        // `ZombieHorse.mobInteract` reaches `AbstractHorse.mobInteract` for this
        // case, which opens the inventory before consulting the held item.
        if !is_baby && !is_vehicle && is_tamed && secondary_use_active {
            return LocalTargetUsePrediction::Result(LocalInteractionResult::Success(
                LocalInteractionSuccess::client_item_interaction(),
            ));
        }

        let Some(item_id) = held_item_id else {
            return if is_baby || is_vehicle {
                LocalTargetUsePrediction::Result(LocalInteractionResult::Pass)
            } else {
                LocalTargetUsePrediction::Result(LocalInteractionResult::Success(
                    LocalInteractionSuccess::client_item_interaction(),
                ))
            };
        };
        let Some(food_item_ids) =
            self.registry_holder_ids_for_tag("minecraft:item", VANILLA_ZOMBIE_HORSE_FOOD_TAG)
        else {
            return LocalTargetUsePrediction::UnsupportedEntityInteract(
                VANILLA_ZOMBIE_HORSE_INTERACT_OWNER,
            );
        };
        if food_item_ids.contains(&item_id) {
            if !is_baby && !is_vehicle {
                return LocalTargetUsePrediction::Result(LocalInteractionResult::Success(
                    LocalInteractionSuccess::server_item_interaction(),
                ));
            }
            if is_baby {
                let Some((data_id, default)) =
                    vanilla_entity_metadata_age_locked_accessor_for_entity_type(entity_type_id)
                else {
                    return LocalTargetUsePrediction::UnknownEntityType(Some(entity_type_id));
                };
                let Some(age_locked) = self
                    .entities
                    .metadata_bool_for_entity(entity_id, data_id, default)
                else {
                    return LocalTargetUsePrediction::UnknownEntityType(Some(entity_type_id));
                };
                if !age_locked {
                    return LocalTargetUsePrediction::Result(LocalInteractionResult::Success(
                        LocalInteractionSuccess::client_item_interaction(),
                    ));
                }
            }
            return LocalTargetUsePrediction::Result(LocalInteractionResult::Success(
                LocalInteractionSuccess::consume(false),
            ));
        }

        if !is_baby && !is_vehicle {
            if !is_tamed {
                return LocalTargetUsePrediction::Result(LocalInteractionResult::Success(
                    LocalInteractionSuccess::client_item_interaction(),
                ));
            }
            let Some(item_type) = vanilla_item_type(item_id) else {
                return LocalTargetUsePrediction::UnknownItemType(Some(item_id));
            };
            if let Some(owner) = item_type.interact_living_entity_override {
                return LocalTargetUsePrediction::UnsupportedItemInteractLivingEntity(owner);
            }
            return LocalTargetUsePrediction::Result(LocalInteractionResult::Success(
                LocalInteractionSuccess::client_item_interaction(),
            ));
        }

        // Zombie horses are in `EntityTypeTags.CANNOT_BE_AGE_LOCKED`, so their
        // baby/vehicle `AgeableMob` fallback has no golden-dandelion success.
        LocalTargetUsePrediction::Result(LocalInteractionResult::Pass)
    }

    fn predict_local_base_entity_interact(
        &self,
        entity_id: i32,
        entity_type_id: i32,
        held_item_id: Option<i32>,
    ) -> LocalTargetUsePrediction {
        if held_item_id == Some(VANILLA_ITEM_SHEARS_ID) {
            return LocalTargetUsePrediction::UnsupportedEntityInteract(
                VANILLA_DEFAULT_ENTITY_INTERACT_OWNER,
            );
        }
        if !vanilla_entity_metadata_type_is_leashable(entity_type_id) {
            return LocalTargetUsePrediction::Result(LocalInteractionResult::Pass);
        }
        if let Some(holder_id) = self.entities.leash_holder_id(entity_id) {
            let Some(local_player_id) = self.local_player_id else {
                return LocalTargetUsePrediction::UnsupportedEntityInteract(
                    VANILLA_DEFAULT_ENTITY_INTERACT_OWNER,
                );
            };
            if holder_id == local_player_id {
                return LocalTargetUsePrediction::Result(LocalInteractionResult::Success(
                    LocalInteractionSuccess::client_without_item(),
                ));
            }
        }
        if held_item_id == Some(VANILLA_ITEM_LEAD_ID) {
            return LocalTargetUsePrediction::UnsupportedEntityInteract(
                VANILLA_DEFAULT_ENTITY_INTERACT_OWNER,
            );
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
        entity_data_serializers::{
            VANILLA_ENTITY_DATA_SERIALIZER_BOOLEAN_ID, VANILLA_ENTITY_DATA_SERIALIZER_BYTE_ID,
            VANILLA_ENTITY_DATA_SERIALIZER_ITEM_STACK_ID,
        },
        entity_types::{
            VANILLA_ENTITY_TYPE_AREA_EFFECT_CLOUD_ID, VANILLA_ENTITY_TYPE_ARMOR_STAND_ID,
            VANILLA_ENTITY_TYPE_CAMEL_HUSK_ID, VANILLA_ENTITY_TYPE_CHEST_MINECART_ID,
            VANILLA_ENTITY_TYPE_COMMAND_BLOCK_MINECART_ID, VANILLA_ENTITY_TYPE_COW_ID,
            VANILLA_ENTITY_TYPE_FURNACE_MINECART_ID, VANILLA_ENTITY_TYPE_HOPPER_MINECART_ID,
            VANILLA_ENTITY_TYPE_INTERACTION_ID, VANILLA_ENTITY_TYPE_ITEM_FRAME_ID,
            VANILLA_ENTITY_TYPE_LEASH_KNOT_ID, VANILLA_ENTITY_TYPE_MANNEQUIN_ID,
            VANILLA_ENTITY_TYPE_MINECART_ID, VANILLA_ENTITY_TYPE_NAUTILUS_ID,
            VANILLA_ENTITY_TYPE_OAK_BOAT_ID, VANILLA_ENTITY_TYPE_OAK_CHEST_BOAT_ID,
            VANILLA_ENTITY_TYPE_ZOMBIE_HORSE_ID, VANILLA_ENTITY_TYPE_ZOMBIE_ID,
            VANILLA_ENTITY_TYPE_ZOMBIE_NAUTILUS_ID,
        },
        item_types::{
            VanillaItemInteractLivingEntityOverride, VanillaItemUseOnOverride,
            VANILLA_ITEM_APPLE_ID, VANILLA_ITEM_CHICKEN_SPAWN_EGG_ID, VANILLA_ITEM_CHORUS_FRUIT_ID,
            VANILLA_ITEM_LEAD_ID, VANILLA_ITEM_NAME_TAG_ID, VANILLA_ITEM_RED_DYE_ID,
            VANILLA_ITEM_SHEARS_ID, VANILLA_ITEM_SNOWBALL_ID, VANILLA_ITEM_STONE_ID,
        },
        packets::{
            AddEntity, Cooldown, DataComponentPatchSummary, EntityDataValue, EntityDataValueKind,
            EntityEvent, ItemStackSummary, PlayerHealth, RegistryTags, SetEntityData,
            SetEntityLink, SetPassengers, SetPlayerInventory, TagNetworkPayload, UpdateTags, Vec3d,
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
            world.predict_local_entity_interaction(1, InteractionHand::MainHand, false),
            LocalTargetUsePrediction::Result(LocalInteractionResult::Pass)
        );
        assert_eq!(
            world.predict_local_entity_interaction(2, InteractionHand::MainHand, false),
            LocalTargetUsePrediction::UnsupportedEntityInteract(
                "net.minecraft.world.entity.animal.cow.AbstractCow"
            )
        );
        assert_eq!(
            world.predict_local_entity_interaction(99, InteractionHand::MainHand, false),
            LocalTargetUsePrediction::UnknownEntityType(None)
        );

        set_main_hand(&mut world, VANILLA_ITEM_SHEARS_ID);
        assert_eq!(
            world.predict_local_entity_interaction(1, InteractionHand::MainHand, false),
            LocalTargetUsePrediction::UnsupportedEntityInteract(
                VANILLA_DEFAULT_ENTITY_INTERACT_OWNER
            )
        );
        set_main_hand(&mut world, VANILLA_ITEM_RED_DYE_ID);
        assert_eq!(
            world.predict_local_entity_interaction(3, InteractionHand::MainHand, false),
            LocalTargetUsePrediction::UnsupportedItemInteractLivingEntity(
                VanillaItemInteractLivingEntityOverride::DyeItem
            )
        );
    }

    #[test]
    fn default_mob_interaction_predicts_pass_spawn_egg_and_local_unleash() {
        let mut world = WorldStore::new();
        add_entity(&mut world, 4, VANILLA_ENTITY_TYPE_ZOMBIE_ID);

        assert_eq!(
            world.predict_local_entity_interaction(4, InteractionHand::MainHand, false),
            LocalTargetUsePrediction::Result(LocalInteractionResult::Pass)
        );
        set_main_hand(&mut world, VANILLA_ITEM_STONE_ID);
        assert_eq!(
            world.predict_local_entity_interaction(4, InteractionHand::MainHand, false),
            LocalTargetUsePrediction::Result(LocalInteractionResult::Pass)
        );

        set_main_hand(&mut world, VANILLA_ITEM_CHICKEN_SPAWN_EGG_ID);
        assert_eq!(
            world.predict_local_entity_interaction(4, InteractionHand::MainHand, false),
            LocalTargetUsePrediction::Result(LocalInteractionResult::Success(
                LocalInteractionSuccess::server_item_interaction()
            ))
        );

        set_main_hand(&mut world, VANILLA_ITEM_LEAD_ID);
        assert_eq!(
            world.predict_local_entity_interaction(4, InteractionHand::MainHand, false),
            LocalTargetUsePrediction::UnsupportedEntityInteract(
                VANILLA_DEFAULT_ENTITY_INTERACT_OWNER
            )
        );

        world.local_player_id = Some(7);
        assert!(world.apply_set_entity_link(SetEntityLink {
            source_id: 4,
            dest_id: 7,
        }));
        world.apply_set_player_inventory(SetPlayerInventory {
            slot: 0,
            item: ItemStackSummary::empty(),
        });
        assert_eq!(
            world.predict_local_entity_interaction(4, InteractionHand::MainHand, false),
            LocalTargetUsePrediction::Result(LocalInteractionResult::Success(
                LocalInteractionSuccess::client_without_item()
            ))
        );
    }

    #[test]
    fn minecart_interaction_predicts_client_results_from_local_guards() {
        let mut world = WorldStore::new();
        add_entity(&mut world, 10, VANILLA_ENTITY_TYPE_MINECART_ID);
        add_entity(&mut world, 11, VANILLA_ENTITY_TYPE_CHEST_MINECART_ID);
        add_entity(&mut world, 12, VANILLA_ENTITY_TYPE_HOPPER_MINECART_ID);
        add_entity(&mut world, 13, VANILLA_ENTITY_TYPE_FURNACE_MINECART_ID);
        add_entity(
            &mut world,
            14,
            VANILLA_ENTITY_TYPE_COMMAND_BLOCK_MINECART_ID,
        );
        add_entity(&mut world, 99, VANILLA_ENTITY_TYPE_AREA_EFFECT_CLOUD_ID);

        let success = LocalTargetUsePrediction::Result(LocalInteractionResult::Success(
            LocalInteractionSuccess::client_item_interaction(),
        ));
        assert_eq!(
            world.predict_local_entity_interaction(10, InteractionHand::MainHand, false),
            success
        );
        assert_eq!(
            world.predict_local_entity_interaction(10, InteractionHand::MainHand, true),
            LocalTargetUsePrediction::Result(LocalInteractionResult::Pass)
        );
        assert!(world.apply_set_passengers(SetPassengers {
            vehicle_id: 10,
            passenger_ids: vec![99],
        }));
        assert_eq!(
            world.predict_local_entity_interaction(10, InteractionHand::MainHand, false),
            LocalTargetUsePrediction::Result(LocalInteractionResult::Pass)
        );

        for entity_id in [11, 12, 13] {
            assert_eq!(
                world.predict_local_entity_interaction(entity_id, InteractionHand::MainHand, false),
                success
            );
        }

        assert_eq!(
            world.predict_local_entity_interaction(14, InteractionHand::MainHand, false),
            LocalTargetUsePrediction::Result(LocalInteractionResult::Pass)
        );
        world.local_player_id = Some(7);
        assert!(world.apply_entity_event(EntityEvent {
            entity_id: 7,
            event_id: 26,
        }));
        assert_eq!(
            world.predict_local_entity_interaction(14, InteractionHand::MainHand, false),
            success
        );
    }

    #[test]
    fn boat_interaction_predicts_riding_container_and_submerged_guards() {
        let mut world = WorldStore::new();
        add_entity(&mut world, 20, VANILLA_ENTITY_TYPE_OAK_BOAT_ID);
        add_entity(&mut world, 21, VANILLA_ENTITY_TYPE_OAK_CHEST_BOAT_ID);

        let success = LocalTargetUsePrediction::Result(LocalInteractionResult::Success(
            LocalInteractionSuccess::client_item_interaction(),
        ));
        assert_eq!(
            world.predict_local_entity_interaction(20, InteractionHand::MainHand, false),
            success
        );
        assert_eq!(
            world.predict_local_entity_interaction(20, InteractionHand::MainHand, true),
            LocalTargetUsePrediction::Result(LocalInteractionResult::Pass)
        );
        world
            .entities
            .with_boat_movement_transform_and_support_mut(20, |movement, _, _| {
                movement.out_of_control_ticks = 60.0;
            })
            .unwrap();
        assert_eq!(
            world.predict_local_entity_interaction(20, InteractionHand::MainHand, false),
            LocalTargetUsePrediction::Result(LocalInteractionResult::Pass)
        );

        for secondary_use_active in [false, true] {
            assert_eq!(
                world.predict_local_entity_interaction(
                    21,
                    InteractionHand::MainHand,
                    secondary_use_active
                ),
                success
            );
        }
        world
            .entities
            .with_boat_movement_transform_and_support_mut(21, |movement, _, _| {
                movement.out_of_control_ticks = 60.0;
            })
            .unwrap();
        assert_eq!(
            world.predict_local_entity_interaction(21, InteractionHand::MainHand, false),
            success
        );

        set_main_hand(&mut world, VANILLA_ITEM_LEAD_ID);
        assert_eq!(
            world.predict_local_entity_interaction(20, InteractionHand::MainHand, false),
            LocalTargetUsePrediction::UnsupportedEntityInteract(
                VANILLA_DEFAULT_ENTITY_INTERACT_OWNER
            )
        );
    }

    #[test]
    fn interaction_entity_response_selects_consume_or_client_success() {
        let mut world = WorldStore::new();
        add_entity(&mut world, 30, VANILLA_ENTITY_TYPE_INTERACTION_ID);
        let accessors = vanilla_entity_metadata_interaction_accessors_for_entity_type(
            VANILLA_ENTITY_TYPE_INTERACTION_ID,
        )
        .unwrap();

        assert_eq!(
            world.predict_local_entity_interaction(30, InteractionHand::MainHand, false),
            LocalTargetUsePrediction::Result(LocalInteractionResult::Success(
                LocalInteractionSuccess::consume(false)
            ))
        );
        assert!(world.apply_set_entity_data(SetEntityData {
            id: 30,
            values: vec![EntityDataValue {
                data_id: accessors.response_data_id,
                serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_BOOLEAN_ID,
                value: EntityDataValueKind::Boolean(true),
            }],
        }));
        assert_eq!(
            world.predict_local_entity_interaction(30, InteractionHand::MainHand, false),
            LocalTargetUsePrediction::Result(LocalInteractionResult::Success(
                LocalInteractionSuccess::client_item_interaction()
            ))
        );
    }

    #[test]
    fn item_frame_interaction_uses_synced_frame_and_held_item_presence() {
        let mut world = WorldStore::new();
        add_entity(&mut world, 40, VANILLA_ENTITY_TYPE_ITEM_FRAME_ID);
        add_entity(&mut world, 41, VANILLA_ENTITY_TYPE_ITEM_FRAME_ID);
        let accessors =
            crate::entities::vanilla_entity_metadata_item_frame_accessors_for_entity_type(
                VANILLA_ENTITY_TYPE_ITEM_FRAME_ID,
            )
            .unwrap();
        assert!(world.apply_set_entity_data(SetEntityData {
            id: 41,
            values: vec![EntityDataValue {
                data_id: accessors.item_data_id,
                serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_ITEM_STACK_ID,
                value: EntityDataValueKind::ItemStack(stack(VANILLA_ITEM_STONE_ID)),
            }],
        }));

        assert_eq!(
            world.predict_local_entity_interaction(40, InteractionHand::MainHand, false),
            LocalTargetUsePrediction::Result(LocalInteractionResult::Pass)
        );
        assert_eq!(
            world.predict_local_entity_interaction(41, InteractionHand::MainHand, false),
            LocalTargetUsePrediction::Result(LocalInteractionResult::Success(
                LocalInteractionSuccess::client_item_interaction()
            ))
        );
        set_main_hand(&mut world, VANILLA_ITEM_STONE_ID);
        assert_eq!(
            world.predict_local_entity_interaction(40, InteractionHand::MainHand, false),
            LocalTargetUsePrediction::Result(LocalInteractionResult::Success(
                LocalInteractionSuccess::client_item_interaction()
            ))
        );
    }

    #[test]
    fn armor_stand_interaction_predicts_marker_and_name_tag_routing() {
        let mut world = WorldStore::new();
        add_entity(&mut world, 50, VANILLA_ENTITY_TYPE_ARMOR_STAND_ID);
        let accessors = vanilla_entity_metadata_armor_stand_accessors_for_entity_type(
            VANILLA_ENTITY_TYPE_ARMOR_STAND_ID,
        )
        .unwrap();

        assert_eq!(
            world.predict_local_entity_interaction(50, InteractionHand::MainHand, false),
            LocalTargetUsePrediction::Result(LocalInteractionResult::Success(
                LocalInteractionSuccess::server_item_interaction()
            ))
        );
        set_main_hand(&mut world, VANILLA_ITEM_NAME_TAG_ID);
        assert_eq!(
            world.predict_local_entity_interaction(50, InteractionHand::MainHand, false),
            LocalTargetUsePrediction::UnsupportedItemInteractLivingEntity(
                VanillaItemInteractLivingEntityOverride::NameTagItem
            )
        );

        world.apply_set_player_inventory(SetPlayerInventory {
            slot: 0,
            item: ItemStackSummary::empty(),
        });
        assert!(world.apply_set_entity_data(SetEntityData {
            id: 50,
            values: vec![EntityDataValue {
                data_id: accessors.client_flags_data_id,
                serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_BYTE_ID,
                value: EntityDataValueKind::Byte(accessors.marker_mask),
            }],
        }));
        assert_eq!(
            world.predict_local_entity_interaction(50, InteractionHand::MainHand, false),
            LocalTargetUsePrediction::Result(LocalInteractionResult::Pass)
        );
    }

    #[test]
    fn leash_knot_interaction_is_unconditional_client_success() {
        let mut world = WorldStore::new();
        add_entity(&mut world, 60, VANILLA_ENTITY_TYPE_LEASH_KNOT_ID);

        for secondary_use_active in [false, true] {
            assert_eq!(
                world.predict_local_entity_interaction(
                    60,
                    InteractionHand::MainHand,
                    secondary_use_active
                ),
                LocalTargetUsePrediction::Result(LocalInteractionResult::Success(
                    LocalInteractionSuccess::client_item_interaction()
                ))
            );
        }
        set_main_hand(&mut world, VANILLA_ITEM_SHEARS_ID);
        assert_eq!(
            world.predict_local_entity_interaction(60, InteractionHand::MainHand, false),
            LocalTargetUsePrediction::Result(LocalInteractionResult::Success(
                LocalInteractionSuccess::client_item_interaction()
            ))
        );
    }

    #[test]
    fn persistence_only_mob_interact_overrides_delegate_to_mob_flow() {
        let mut world = WorldStore::new();
        for (entity_id, entity_type_id) in [
            (70, VANILLA_ENTITY_TYPE_CAMEL_HUSK_ID),
            (71, VANILLA_ENTITY_TYPE_ZOMBIE_HORSE_ID),
            (72, VANILLA_ENTITY_TYPE_NAUTILUS_ID),
            (73, VANILLA_ENTITY_TYPE_ZOMBIE_NAUTILUS_ID),
        ] {
            add_entity(&mut world, entity_id, entity_type_id);
        }

        assert_eq!(
            world.predict_local_entity_interaction(71, InteractionHand::MainHand, false),
            LocalTargetUsePrediction::Result(LocalInteractionResult::Success(
                LocalInteractionSuccess::client_item_interaction()
            ))
        );
        for (entity_id, owner) in [
            (
                72,
                "net.minecraft.world.entity.animal.nautilus.AbstractNautilus",
            ),
            (
                73,
                "net.minecraft.world.entity.animal.nautilus.AbstractNautilus",
            ),
        ] {
            assert_eq!(
                world.predict_local_entity_interaction(entity_id, InteractionHand::MainHand, false),
                LocalTargetUsePrediction::UnsupportedEntityInteract(owner)
            );
        }

        set_main_hand(&mut world, VANILLA_ITEM_CHICKEN_SPAWN_EGG_ID);
        assert_eq!(
            world.predict_local_entity_interaction(70, InteractionHand::MainHand, false),
            LocalTargetUsePrediction::Result(LocalInteractionResult::Success(
                LocalInteractionSuccess::server_item_interaction()
            ))
        );
    }

    #[test]
    fn zombie_horse_mob_interaction_uses_synced_age_tame_vehicle_and_food_state() {
        let mut world = WorldStore::new();
        add_entity(&mut world, 81, VANILLA_ENTITY_TYPE_ZOMBIE_HORSE_ID);
        add_entity(&mut world, 82, VANILLA_ENTITY_TYPE_AREA_EFFECT_CLOUD_ID);
        let horse_accessors = vanilla_entity_metadata_abstract_horse_accessors_for_entity_type(
            VANILLA_ENTITY_TYPE_ZOMBIE_HORSE_ID,
        )
        .unwrap();
        let (baby_data_id, baby_default) =
            crate::entities::vanilla_entity_metadata_baby_accessor_for_entity_type(
                VANILLA_ENTITY_TYPE_ZOMBIE_HORSE_ID,
            )
            .unwrap();
        let (age_locked_data_id, age_locked_default) =
            vanilla_entity_metadata_age_locked_accessor_for_entity_type(
                VANILLA_ENTITY_TYPE_ZOMBIE_HORSE_ID,
            )
            .unwrap();
        assert!(!baby_default);
        assert!(!age_locked_default);

        let client_success = LocalTargetUsePrediction::Result(LocalInteractionResult::Success(
            LocalInteractionSuccess::client_item_interaction(),
        ));
        assert_eq!(
            world.predict_local_entity_interaction(81, InteractionHand::MainHand, false),
            client_success
        );

        assert!(world.apply_set_entity_data(SetEntityData {
            id: 81,
            values: vec![EntityDataValue {
                data_id: horse_accessors.flags_data_id,
                serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_BYTE_ID,
                value: EntityDataValueKind::Byte(horse_accessors.tame_mask),
            }],
        }));
        assert_eq!(
            world.predict_local_entity_interaction(81, InteractionHand::MainHand, true),
            client_success
        );

        set_main_hand(&mut world, VANILLA_ITEM_STONE_ID);
        assert_eq!(
            world.predict_local_entity_interaction(81, InteractionHand::MainHand, false),
            LocalTargetUsePrediction::UnsupportedEntityInteract(
                VANILLA_ZOMBIE_HORSE_INTERACT_OWNER
            )
        );
        world.apply_update_tags(UpdateTags {
            registries: vec![RegistryTags {
                registry: "minecraft:item".to_string(),
                tags: vec![TagNetworkPayload {
                    tag: VANILLA_ZOMBIE_HORSE_FOOD_TAG.to_string(),
                    entries: vec![VANILLA_ITEM_STONE_ID],
                }],
            }],
        });
        assert_eq!(
            world.predict_local_entity_interaction(81, InteractionHand::MainHand, false),
            LocalTargetUsePrediction::Result(LocalInteractionResult::Success(
                LocalInteractionSuccess::server_item_interaction()
            ))
        );

        set_main_hand(&mut world, VANILLA_ITEM_RED_DYE_ID);
        assert_eq!(
            world.predict_local_entity_interaction(81, InteractionHand::MainHand, false),
            LocalTargetUsePrediction::UnsupportedItemInteractLivingEntity(
                VanillaItemInteractLivingEntityOverride::DyeItem
            )
        );
        assert!(world.apply_set_entity_data(SetEntityData {
            id: 81,
            values: vec![EntityDataValue {
                data_id: horse_accessors.flags_data_id,
                serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_BYTE_ID,
                value: EntityDataValueKind::Byte(horse_accessors.flags_default),
            }],
        }));
        assert_eq!(
            world.predict_local_entity_interaction(81, InteractionHand::MainHand, false),
            client_success
        );

        assert!(world.apply_set_entity_data(SetEntityData {
            id: 81,
            values: vec![EntityDataValue {
                data_id: baby_data_id,
                serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_BOOLEAN_ID,
                value: EntityDataValueKind::Boolean(true),
            }],
        }));
        set_main_hand(&mut world, VANILLA_ITEM_STONE_ID);
        assert_eq!(
            world.predict_local_entity_interaction(81, InteractionHand::MainHand, false),
            client_success
        );
        assert!(world.apply_set_entity_data(SetEntityData {
            id: 81,
            values: vec![EntityDataValue {
                data_id: age_locked_data_id,
                serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_BOOLEAN_ID,
                value: EntityDataValueKind::Boolean(true),
            }],
        }));
        assert_eq!(
            world.predict_local_entity_interaction(81, InteractionHand::MainHand, false),
            LocalTargetUsePrediction::Result(LocalInteractionResult::Success(
                LocalInteractionSuccess::consume(false)
            ))
        );

        assert!(world.apply_set_entity_data(SetEntityData {
            id: 81,
            values: vec![EntityDataValue {
                data_id: baby_data_id,
                serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_BOOLEAN_ID,
                value: EntityDataValueKind::Boolean(false),
            }],
        }));
        assert!(world.apply_set_passengers(SetPassengers {
            vehicle_id: 81,
            passenger_ids: vec![82],
        }));
        assert_eq!(
            world.predict_local_entity_interaction(81, InteractionHand::MainHand, false),
            LocalTargetUsePrediction::Result(LocalInteractionResult::Success(
                LocalInteractionSuccess::consume(false)
            ))
        );
        set_main_hand(&mut world, VANILLA_ITEM_RED_DYE_ID);
        assert_eq!(
            world.predict_local_entity_interaction(81, InteractionHand::MainHand, true),
            LocalTargetUsePrediction::UnsupportedItemInteractLivingEntity(
                VanillaItemInteractLivingEntityOverride::DyeItem
            )
        );
        set_main_hand(&mut world, VANILLA_ITEM_APPLE_ID);
        assert_eq!(
            world.predict_local_entity_interaction(81, InteractionHand::MainHand, true),
            LocalTargetUsePrediction::Result(LocalInteractionResult::Pass)
        );
    }

    #[test]
    fn camel_husk_mob_interaction_uses_secondary_and_synced_food_tag() {
        let mut world = WorldStore::new();
        add_entity(&mut world, 80, VANILLA_ENTITY_TYPE_CAMEL_HUSK_ID);

        assert_eq!(
            world.predict_local_entity_interaction(80, InteractionHand::MainHand, false),
            LocalTargetUsePrediction::Result(LocalInteractionResult::Success(
                LocalInteractionSuccess::consume(false)
            ))
        );
        assert_eq!(
            world.predict_local_entity_interaction(80, InteractionHand::MainHand, true),
            LocalTargetUsePrediction::Result(LocalInteractionResult::Success(
                LocalInteractionSuccess::client_item_interaction()
            ))
        );

        set_main_hand(&mut world, VANILLA_ITEM_STONE_ID);
        assert_eq!(
            world.predict_local_entity_interaction(80, InteractionHand::MainHand, false),
            LocalTargetUsePrediction::UnsupportedEntityInteract(VANILLA_CAMEL_MOB_INTERACT_OWNER)
        );
        world.apply_update_tags(UpdateTags {
            registries: vec![RegistryTags {
                registry: "minecraft:item".to_string(),
                tags: vec![TagNetworkPayload {
                    tag: VANILLA_CAMEL_HUSK_FOOD_TAG.to_string(),
                    entries: vec![VANILLA_ITEM_STONE_ID],
                }],
            }],
        });
        assert_eq!(
            world.predict_local_entity_interaction(80, InteractionHand::MainHand, false),
            LocalTargetUsePrediction::Result(LocalInteractionResult::Success(
                LocalInteractionSuccess::server_item_interaction()
            ))
        );
        world.apply_update_tags(UpdateTags {
            registries: vec![RegistryTags {
                registry: "minecraft:item".to_string(),
                tags: vec![TagNetworkPayload {
                    tag: VANILLA_CAMEL_HUSK_FOOD_TAG.to_string(),
                    entries: Vec::new(),
                }],
            }],
        });
        assert_eq!(
            world.predict_local_entity_interaction(80, InteractionHand::MainHand, false),
            LocalTargetUsePrediction::Result(LocalInteractionResult::Success(
                LocalInteractionSuccess::consume(false)
            ))
        );

        set_main_hand(&mut world, VANILLA_ITEM_RED_DYE_ID);
        assert_eq!(
            world.predict_local_entity_interaction(80, InteractionHand::MainHand, false),
            LocalTargetUsePrediction::UnsupportedItemInteractLivingEntity(
                VanillaItemInteractLivingEntityOverride::DyeItem
            )
        );
    }
}
