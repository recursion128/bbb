use super::*;
use crate::{
    entities::{
        vanilla_entity_metadata_abstract_horse_accessors_for_entity_type,
        vanilla_entity_metadata_age_locked_accessor_for_entity_type,
        vanilla_entity_metadata_armor_stand_accessors_for_entity_type,
        vanilla_entity_metadata_bogged_sheared_accessor_for_entity_type,
        vanilla_entity_metadata_interact_owner,
        vanilla_entity_metadata_interaction_accessors_for_entity_type,
        vanilla_entity_metadata_mob_interact_owner,
        vanilla_entity_metadata_sheep_wool_accessor_for_entity_type,
        vanilla_entity_metadata_snow_golem_pumpkin_accessor_for_entity_type,
        vanilla_entity_metadata_tamable_animal_accessors_for_entity_type,
        vanilla_entity_metadata_type_is_leashable, vanilla_entity_metadata_type_is_living,
        vanilla_entity_type_is_minecart, vanilla_pick_bounds_for_type,
    },
    BlockPos,
};
use bbb_protocol::{
    attribute_types::VANILLA_ATTRIBUTE_MAX_HEALTH_ID,
    block_types::{
        vanilla_block_type, vanilla_block_type_for_resource_id, BlockTypeCatalogEntry,
        VanillaBlockUseItemOnOverride, VanillaBlockUseWithoutItemOverride, VANILLA_BLOCK_AIR_ID,
        VANILLA_BLOCK_BEDROCK_ID, VANILLA_BLOCK_END_PORTAL_FRAME_ID, VANILLA_BLOCK_LODESTONE_ID,
        VANILLA_BLOCK_OBSIDIAN_ID,
    },
    builtin_enum_component_values::normalize_vanilla_dye_color_id,
    data_component_types::{
        VANILLA_DATA_COMPONENT_BLOCKS_ATTACKS_TYPE_ID, VANILLA_DATA_COMPONENT_CUSTOM_NAME_TYPE_ID,
        VANILLA_DATA_COMPONENT_DYE_TYPE_ID, VANILLA_DATA_COMPONENT_ENCHANTMENTS_TYPE_ID,
        VANILLA_DATA_COMPONENT_EQUIPPABLE_TYPE_ID, VANILLA_DATA_COMPONENT_KINETIC_WEAPON_TYPE_ID,
        VANILLA_DATA_COMPONENT_RECIPES_TYPE_ID, VANILLA_DATA_COMPONENT_USE_COOLDOWN_TYPE_ID,
    },
    entity_types::{
        vanilla_entity_type_is_serializable, VANILLA_ENTITY_TYPE_MINECART_ID,
        VANILLA_ENTITY_TYPE_WOLF_ID,
    },
    equipment_slots::VANILLA_EQUIPMENT_SLOTS,
    item_types::{
        VanillaBlockTransform, VanillaItemInteractLivingEntityOverride, VanillaItemUseOnOverride,
        VanillaItemUseOverride, VANILLA_AXE_SCRAPEABLES, VANILLA_AXE_STRIPPABLES,
        VANILLA_AXE_WAX_OFF, VANILLA_HOE_TILLABLES, VANILLA_HONEYCOMB_WAXABLES,
        VANILLA_ITEM_GOLDEN_DANDELION_ID, VANILLA_ITEM_IRON_INGOT_ID, VANILLA_ITEM_LEAD_ID,
        VANILLA_ITEM_SHEARS_ID, VANILLA_SHOVEL_FLATTENABLES,
    },
    packets::{
        BlockPos as ProtocolBlockPos, ConsumableSummary, Direction as ProtocolDirection,
        LodestoneTargetSummary,
    },
};

mod block_use_without_item;
mod bone_meal;
mod book;
mod bundle;
mod crossbow;
mod empty_map;
mod equippable;
mod firework_rocket;
mod fishing_rod;
mod food_on_a_stick;
mod ignition;
mod instrument;
mod item_use_fov;
mod knowledge_book;
mod potion;
mod thrown_projectile;

const VANILLA_DEFAULT_ENTITY_INTERACT_OWNER: &str = "net.minecraft.world.entity.Entity";
const VANILLA_MOB_ENTITY_INTERACT_OWNER: &str = "net.minecraft.world.entity.Mob";
const VANILLA_ARMOR_STAND_INTERACT_OWNER: &str = "net.minecraft.world.entity.decoration.ArmorStand";
const VANILLA_BOAT_INTERACT_OWNER: &str = "net.minecraft.world.entity.vehicle.boat.AbstractBoat";
const VANILLA_BOGGED_MOB_INTERACT_OWNER: &str =
    "net.minecraft.world.entity.monster.skeleton.Bogged";
const VANILLA_CAMEL_HUSK_INTERACT_OWNER: &str = "net.minecraft.world.entity.animal.camel.CamelHusk";
const VANILLA_CAMEL_MOB_INTERACT_OWNER: &str = "net.minecraft.world.entity.animal.camel.Camel";
const VANILLA_CAMEL_HUSK_FOOD_TAG: &str = "minecraft:camel_husk_food";
const VANILLA_CHEST_BOAT_INTERACT_OWNER: &str =
    "net.minecraft.world.entity.vehicle.boat.AbstractChestBoat";
const VANILLA_CREEPER_MOB_INTERACT_OWNER: &str = "net.minecraft.world.entity.monster.Creeper";
const VANILLA_CREEPER_IGNITERS_TAG: &str = "minecraft:creeper_igniters";
const VANILLA_COPPER_GOLEM_MOB_INTERACT_OWNER: &str =
    "net.minecraft.world.entity.animal.golem.CopperGolem";
const VANILLA_COPPER_GOLEM_SHEARABLE_ITEMS_TAG: &str = "minecraft:shearable_from_copper_golem";
const VANILLA_INTERACTION_ENTITY_INTERACT_OWNER: &str = "net.minecraft.world.entity.Interaction";
const VANILLA_IRON_GOLEM_MOB_INTERACT_OWNER: &str =
    "net.minecraft.world.entity.animal.golem.IronGolem";
const VANILLA_ITEM_FRAME_INTERACT_OWNER: &str = "net.minecraft.world.entity.decoration.ItemFrame";
const VANILLA_LEASH_KNOT_INTERACT_OWNER: &str =
    "net.minecraft.world.entity.decoration.LeashFenceKnotEntity";
const VANILLA_NAUTILUS_INTERACT_OWNER: &str =
    "net.minecraft.world.entity.animal.nautilus.AbstractNautilus";
const VANILLA_NAUTILUS_FOOD_TAG: &str = "minecraft:nautilus_food";
const VANILLA_NAUTILUS_TAMING_ITEMS_TAG: &str = "minecraft:nautilus_taming_items";
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
const VANILLA_MOOSHROOM_MOB_INTERACT_OWNER: &str =
    "net.minecraft.world.entity.animal.cow.MushroomCow";
const VANILLA_SHEEP_MOB_INTERACT_OWNER: &str = "net.minecraft.world.entity.animal.sheep.Sheep";
const VANILLA_SHEEP_FOOD_TAG: &str = "minecraft:sheep_food";
const VANILLA_SNOW_GOLEM_MOB_INTERACT_OWNER: &str =
    "net.minecraft.world.entity.animal.golem.SnowGolem";
const VANILLA_ZOMBIE_HORSE_INTERACT_OWNER: &str =
    "net.minecraft.world.entity.animal.equine.ZombieHorse";
const VANILLA_ZOMBIE_HORSE_FOOD_TAG: &str = "minecraft:zombie_horse_food";

#[derive(Debug, Clone, Copy)]
struct LocalBlockUseContext {
    pos: BlockPos,
    face: ProtocolDirection,
    sequence: i32,
    secondary_use_active: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LocalInteractionSwingSource {
    None,
    Client,
    Server,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LocalInteractionSoundAction {
    FlintAndSteelUse { pos: BlockPos },
    InstrumentUse { hand: InteractionHand },
    BundleRemoveOne,
    SuppressedBundleDropContents,
    SuppressedFishingRodUse { retrieving: bool },
    SpyglassUse,
    SpyglassStopUsing,
    SuppressedBowRelease { power_bits: u32 },
    SuppressedProjectileThrow,
    SuppressedFireChargeUse,
    SuppressedLodestoneCompassUse,
    SuppressedPotionMudConversion,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LocalLeverAttachFace {
    Floor,
    Wall,
    Ceiling,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LocalInteractionParticleAction {
    LeverActivation {
        pos: BlockPos,
        facing: ProtocolDirection,
        face: LocalLeverAttachFace,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LocalInteractionSuccess {
    pub swing_source: LocalInteractionSwingSource,
    pub was_item_interaction: bool,
    pub starts_using_item: bool,
    pub extra_swing: Option<InteractionHand>,
    pub sound_action: Option<LocalInteractionSoundAction>,
    pub particle_action: Option<LocalInteractionParticleAction>,
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
    MissingRegistryTag(&'static str),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LocalTargetUsePrediction {
    Result(LocalInteractionResult),
    UnsupportedBlockUseItemOn(VanillaBlockUseItemOnOverride),
    UnsupportedBlockUseWithoutItem(VanillaBlockUseWithoutItemOverride),
    UnsupportedItemUseOn(VanillaItemUseOnOverride),
    UnsupportedEntityInteract(&'static str),
    UnsupportedItemInteractLivingEntity(VanillaItemInteractLivingEntityOverride),
    UnsupportedItemStackInteractLivingEntityComponent(i32),
    UnknownBlockType(Option<i32>),
    UnknownEntityType(Option<i32>),
    UnknownItemType(Option<i32>),
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct LocalUsingItemTickActions {
    pub bundle_drops: u32,
}

impl LocalInteractionSuccess {
    const fn consume(starts_using_item: bool) -> Self {
        Self {
            swing_source: LocalInteractionSwingSource::None,
            was_item_interaction: true,
            starts_using_item,
            extra_swing: None,
            sound_action: None,
            particle_action: None,
        }
    }

    const fn consume_without_item() -> Self {
        Self {
            swing_source: LocalInteractionSwingSource::None,
            was_item_interaction: false,
            starts_using_item: false,
            extra_swing: None,
            sound_action: None,
            particle_action: None,
        }
    }

    const fn server_item_interaction() -> Self {
        Self {
            swing_source: LocalInteractionSwingSource::Server,
            was_item_interaction: true,
            starts_using_item: false,
            extra_swing: None,
            sound_action: None,
            particle_action: None,
        }
    }

    const fn client_without_item() -> Self {
        Self {
            swing_source: LocalInteractionSwingSource::Client,
            was_item_interaction: false,
            starts_using_item: false,
            extra_swing: None,
            sound_action: None,
            particle_action: None,
        }
    }

    const fn client_item_interaction() -> Self {
        Self {
            swing_source: LocalInteractionSwingSource::Client,
            was_item_interaction: true,
            starts_using_item: false,
            extra_swing: None,
            sound_action: None,
            particle_action: None,
        }
    }

    const fn lodestone_compass_lock(dropped: bool) -> Self {
        Self {
            swing_source: LocalInteractionSwingSource::Client,
            was_item_interaction: true,
            starts_using_item: false,
            extra_swing: if dropped {
                Some(InteractionHand::MainHand)
            } else {
                None
            },
            sound_action: Some(LocalInteractionSoundAction::SuppressedLodestoneCompassUse),
            particle_action: None,
        }
    }

    const fn ignition(sound_action: LocalInteractionSoundAction) -> Self {
        Self {
            swing_source: LocalInteractionSwingSource::Client,
            was_item_interaction: true,
            starts_using_item: false,
            extra_swing: None,
            sound_action: Some(sound_action),
            particle_action: None,
        }
    }

    const fn potion_mud_conversion(dropped: bool) -> Self {
        Self {
            swing_source: LocalInteractionSwingSource::Client,
            was_item_interaction: true,
            starts_using_item: false,
            extra_swing: if dropped {
                Some(InteractionHand::MainHand)
            } else {
                None
            },
            sound_action: Some(LocalInteractionSoundAction::SuppressedPotionMudConversion),
            particle_action: None,
        }
    }

    const fn instrument_use(hand: InteractionHand) -> Self {
        Self {
            swing_source: LocalInteractionSwingSource::None,
            was_item_interaction: true,
            starts_using_item: true,
            extra_swing: None,
            sound_action: Some(LocalInteractionSoundAction::InstrumentUse { hand }),
            particle_action: None,
        }
    }

    const fn equippable_swap(dropped: bool) -> Self {
        Self {
            swing_source: LocalInteractionSwingSource::Client,
            was_item_interaction: true,
            starts_using_item: false,
            extra_swing: if dropped {
                Some(InteractionHand::MainHand)
            } else {
                None
            },
            sound_action: None,
            particle_action: None,
        }
    }

    const fn thrown_projectile_use() -> Self {
        Self {
            swing_source: LocalInteractionSwingSource::Client,
            was_item_interaction: true,
            starts_using_item: false,
            extra_swing: None,
            sound_action: Some(LocalInteractionSoundAction::SuppressedProjectileThrow),
            particle_action: None,
        }
    }

    const fn spyglass_use() -> Self {
        Self {
            swing_source: LocalInteractionSwingSource::None,
            was_item_interaction: true,
            starts_using_item: true,
            extra_swing: None,
            sound_action: Some(LocalInteractionSoundAction::SpyglassUse),
            particle_action: None,
        }
    }

    const fn bundle_use() -> Self {
        Self {
            swing_source: LocalInteractionSwingSource::Client,
            was_item_interaction: true,
            starts_using_item: true,
            extra_swing: None,
            sound_action: None,
            particle_action: None,
        }
    }

    const fn fishing_rod_use(retrieving: bool) -> Self {
        Self {
            swing_source: LocalInteractionSwingSource::Client,
            was_item_interaction: true,
            starts_using_item: false,
            extra_swing: None,
            sound_action: Some(LocalInteractionSoundAction::SuppressedFishingRodUse { retrieving }),
            particle_action: None,
        }
    }
}

impl WorldStore {
    /// Predicts vanilla `ItemStack.use` for the default implementation and
    /// implemented source-derived specialized owners. Remaining specialized
    /// classes stay explicit instead of being guessed from component shape.
    pub fn predict_local_item_use(&mut self, hand: InteractionHand) -> LocalItemUsePrediction {
        let Some(stack) = self.local_item_in_hand(hand).cloned() else {
            return LocalItemUsePrediction::Result(LocalInteractionResult::Pass);
        };
        let Some(item_id) = stack.item_id else {
            return LocalItemUsePrediction::UnknownItemType(None);
        };
        let Some(item_type) = vanilla_item_type(item_id) else {
            return LocalItemUsePrediction::UnknownItemType(Some(item_id));
        };
        if self.local_item_stack_is_on_cooldown(&stack) {
            return LocalItemUsePrediction::Result(LocalInteractionResult::Pass);
        }
        if let Some(use_override) = item_type.use_override {
            return match use_override {
                VanillaItemUseOverride::InstrumentItem => {
                    self.predict_local_instrument_item_use(hand, &stack)
                }
                VanillaItemUseOverride::BowItem => self.predict_local_bow_item_use(),
                VanillaItemUseOverride::WritableBookItem
                | VanillaItemUseOverride::WrittenBookItem => self.predict_local_book_item_use(),
                VanillaItemUseOverride::BundleItem => LocalItemUsePrediction::Result(
                    LocalInteractionResult::Success(LocalInteractionSuccess::bundle_use()),
                ),
                VanillaItemUseOverride::CrossbowItem => {
                    self.predict_local_crossbow_item_use(&stack)
                }
                VanillaItemUseOverride::EmptyMapItem => self.predict_local_empty_map_item_use(),
                VanillaItemUseOverride::FireworkRocketItem => {
                    self.predict_local_firework_rocket_item_use()
                }
                VanillaItemUseOverride::FishingRodItem => self.predict_local_fishing_rod_item_use(),
                VanillaItemUseOverride::FoodOnAStickItem => {
                    self.predict_local_food_on_a_stick_item_use()
                }
                VanillaItemUseOverride::KnowledgeBookItem => {
                    self.predict_local_knowledge_book_item_use(hand, &stack)
                }
                VanillaItemUseOverride::SpyglassItem => LocalItemUsePrediction::Result(
                    LocalInteractionResult::Success(LocalInteractionSuccess::spyglass_use()),
                ),
                VanillaItemUseOverride::EggItem
                | VanillaItemUseOverride::EnderpearlItem
                | VanillaItemUseOverride::ExperienceBottleItem
                | VanillaItemUseOverride::LingeringPotionItem
                | VanillaItemUseOverride::SnowballItem
                | VanillaItemUseOverride::SplashPotionItem
                | VanillaItemUseOverride::WindChargeItem => {
                    self.predict_local_thrown_projectile_item_use(hand, &stack)
                }
                _ => LocalItemUsePrediction::UnsupportedOverride(use_override),
            };
        }

        if self
            .item_stack_has_effective_component(&stack, VANILLA_DATA_COMPONENT_CONSUMABLE_TYPE_ID)
        {
            if !self.local_item_in_hand_can_start_consuming(hand) {
                return LocalItemUsePrediction::Result(LocalInteractionResult::Fail);
            }
            let Some(consumable) = self.effective_item_consumable(&stack) else {
                return LocalItemUsePrediction::MissingComponentValue(
                    VANILLA_DATA_COMPONENT_CONSUMABLE_TYPE_ID,
                );
            };
            return LocalItemUsePrediction::Result(LocalInteractionResult::Success(
                LocalInteractionSuccess::consume(consumable.consume_seconds > 0.0),
            ));
        }

        if let Some(prediction) = self.predict_local_swappable_equippable_use(hand, &stack) {
            return prediction;
        }

        if self.item_stack_has_effective_component(
            &stack,
            VANILLA_DATA_COMPONENT_BLOCKS_ATTACKS_TYPE_ID,
        ) || self.item_stack_has_effective_component(
            &stack,
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
        &mut self,
        pos: BlockPos,
        face: ProtocolDirection,
        hand: InteractionHand,
        secondary_use_active: bool,
        sequence: i32,
    ) -> LocalTargetUsePrediction {
        if self.local_player_is_spectator() {
            return LocalTargetUsePrediction::Result(LocalInteractionResult::Success(
                LocalInteractionSuccess::consume_without_item(),
            ));
        }
        let context = LocalBlockUseContext {
            pos,
            face,
            sequence,
            secondary_use_active,
        };
        let hands_have_item = self.local_item_in_hand(InteractionHand::MainHand).is_some()
            || self.local_item_in_hand(InteractionHand::OffHand).is_some();
        if secondary_use_active && hands_have_item {
            return self.predict_local_item_use_on(hand, Some(context));
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
        self.predict_local_block_use_for_type(block_type, hand, Some(context))
    }

    fn predict_local_block_use_for_type(
        &mut self,
        block_type: &BlockTypeCatalogEntry,
        hand: InteractionHand,
        context: Option<LocalBlockUseContext>,
    ) -> LocalTargetUsePrediction {
        if !self.is_feature_enabled("minecraft:vanilla") {
            return LocalTargetUsePrediction::Result(LocalInteractionResult::Fail);
        }
        let ignition_reaches_item_use =
            self.local_ignition_item_reaches_use_on(block_type.use_item_on_override, hand);
        if let Some(owner) = block_type.use_item_on_override {
            if !ignition_reaches_item_use {
                return LocalTargetUsePrediction::UnsupportedBlockUseItemOn(owner);
            }
        }
        if hand == InteractionHand::MainHand {
            if let Some(owner) = block_type.use_without_item_override {
                if !ignition_reaches_item_use {
                    let Some(context) = context else {
                        return LocalTargetUsePrediction::UnsupportedBlockUseWithoutItem(owner);
                    };
                    let prediction =
                        self.predict_local_block_use_without_item(owner, block_type, context);
                    if !matches!(
                        prediction,
                        LocalTargetUsePrediction::Result(
                            LocalInteractionResult::Pass
                                | LocalInteractionResult::TryEmptyHandInteraction
                        )
                    ) {
                        return prediction;
                    }
                }
            }
        }
        self.predict_local_item_use_on(hand, context)
    }

    fn local_ignition_item_reaches_use_on(
        &self,
        block_owner: Option<VanillaBlockUseItemOnOverride>,
        hand: InteractionHand,
    ) -> bool {
        let Some(item_id) = self
            .local_item_in_hand(hand)
            .and_then(|stack| stack.item_id)
        else {
            return false;
        };
        let ignition = vanilla_item_type(item_id)
            .and_then(|item| item.use_on_override)
            .is_some_and(|owner| {
                matches!(
                    owner,
                    VanillaItemUseOnOverride::FlintAndSteelItem
                        | VanillaItemUseOnOverride::FireChargeItem
                )
            });
        if !ignition {
            return false;
        }
        match block_owner {
            Some(VanillaBlockUseItemOnOverride::CandleBlock)
            | Some(VanillaBlockUseItemOnOverride::CandleCakeBlock) => true,
            Some(VanillaBlockUseItemOnOverride::CampfireBlock) => !self
                .recipes()
                .property_sets
                .get("minecraft:campfire_input")
                .is_some_and(|items| items.contains(&item_id)),
            _ => false,
        }
    }

    fn predict_local_item_use_on(
        &mut self,
        hand: InteractionHand,
        context: Option<LocalBlockUseContext>,
    ) -> LocalTargetUsePrediction {
        let Some(stack) = self.local_item_in_hand(hand) else {
            return LocalTargetUsePrediction::Result(LocalInteractionResult::Pass);
        };
        if self.local_item_stack_is_on_cooldown(stack) {
            return LocalTargetUsePrediction::Result(LocalInteractionResult::Pass);
        }
        if let Some(context) = context {
            if self.local_player_is_adventure()
                && !self.local_item_in_hand_can_place_on_block_in_adventure_mode(hand, context.pos)
            {
                return LocalTargetUsePrediction::Result(LocalInteractionResult::Pass);
            }
        }
        let Some(item_id) = stack.item_id else {
            return LocalTargetUsePrediction::UnknownItemType(None);
        };
        let Some(item_type) = vanilla_item_type(item_id) else {
            return LocalTargetUsePrediction::UnknownItemType(Some(item_id));
        };
        if let Some(owner) = item_type.use_on_override {
            if let Some(context) = context {
                if let Some(result) = self.predict_local_tool_item_use_on(owner, hand, context) {
                    return result;
                }
                if let Some(result) =
                    self.predict_local_client_owned_item_use_on(owner, hand, context)
                {
                    return result;
                }
                if let Some(result) =
                    self.predict_local_entity_spawning_item_use_on(owner, hand, context)
                {
                    return result;
                }
            }
            return LocalTargetUsePrediction::UnsupportedItemUseOn(owner);
        }
        LocalTargetUsePrediction::Result(LocalInteractionResult::Pass)
    }

    fn predict_local_tool_item_use_on(
        &mut self,
        owner: VanillaItemUseOnOverride,
        hand: InteractionHand,
        context: LocalBlockUseContext,
    ) -> Option<LocalTargetUsePrediction> {
        let block = self.probe_block(context.pos)?;
        let block_name = block.block_name.as_deref()?;
        let block_type = vanilla_block_type_for_resource_id(block_name)?;
        let properties = block.block_properties;
        let success = || {
            LocalTargetUsePrediction::Result(LocalInteractionResult::Success(
                LocalInteractionSuccess::client_item_interaction(),
            ))
        };
        let pass = || LocalTargetUsePrediction::Result(LocalInteractionResult::Pass);
        match owner {
            VanillaItemUseOnOverride::AxeItem => {
                if hand == InteractionHand::MainHand
                    && !context.secondary_use_active
                    && self
                        .local_item_in_hand(InteractionHand::OffHand)
                        .is_some_and(|stack| {
                            self.item_stack_has_effective_component(
                                stack,
                                VANILLA_DATA_COMPONENT_BLOCKS_ATTACKS_TYPE_ID,
                            )
                        })
                {
                    return Some(pass());
                }
                let Some((transform, copy_axis_only)) =
                    find_block_transform(VANILLA_AXE_STRIPPABLES, block_type.id)
                        .map(|transform| (transform, true))
                        .or_else(|| {
                            find_block_transform(VANILLA_AXE_SCRAPEABLES, block_type.id)
                                .map(|transform| (transform, false))
                        })
                        .or_else(|| {
                            find_block_transform(VANILLA_AXE_WAX_OFF, block_type.id)
                                .map(|transform| (transform, false))
                        })
                else {
                    return Some(pass());
                };
                if !self.predict_local_transformed_block_state(
                    context,
                    transform,
                    properties,
                    copy_axis_only,
                ) {
                    return Some(LocalTargetUsePrediction::UnsupportedItemUseOn(owner));
                }
                Some(success())
            }
            VanillaItemUseOnOverride::HoneycombItem => {
                let Some(transform) =
                    find_block_transform(VANILLA_HONEYCOMB_WAXABLES, block_type.id)
                else {
                    return Some(pass());
                };
                if !self
                    .predict_local_transformed_block_state(context, transform, properties, false)
                    || (!self
                        .local_player()
                        .abilities
                        .is_some_and(|abilities| abilities.instabuild)
                        && !self.shrink_local_item_in_hand(hand, 1))
                {
                    return Some(LocalTargetUsePrediction::UnsupportedItemUseOn(owner));
                }
                Some(success())
            }
            VanillaItemUseOnOverride::HoeItem => {
                let Some(tillable) = VANILLA_HOE_TILLABLES
                    .iter()
                    .find(|entry| entry.transform.source_block_type_id == block_type.id)
                else {
                    return Some(pass());
                };
                if tillable.requires_air_above
                    && (context.face == ProtocolDirection::Down
                        || !self.local_block_is_air(above(context.pos))?)
                {
                    return Some(pass());
                }
                Some(success())
            }
            VanillaItemUseOnOverride::ShearsItem => {
                if !block_type.growing_plant_head {
                    return Some(pass());
                }
                let Some(current_age) = properties
                    .get("age")
                    .and_then(|value| value.parse::<i32>().ok())
                else {
                    return Some(LocalTargetUsePrediction::UnsupportedItemUseOn(owner));
                };
                let Some(max_age) = self
                    .registries
                    .maximum_numeric_block_property_value(block_name, "age")
                else {
                    return Some(LocalTargetUsePrediction::UnsupportedItemUseOn(owner));
                };
                if current_age >= max_age {
                    return Some(pass());
                }
                let mut predicted_properties = properties;
                predicted_properties.insert("age".to_string(), max_age.to_string());
                let Some(predicted_block_state_id) = self
                    .registries
                    .block_state_id_by_name_and_properties(block_name, &predicted_properties)
                else {
                    return Some(LocalTargetUsePrediction::UnsupportedItemUseOn(owner));
                };
                if !self.predict_local_block_state(
                    context.pos,
                    predicted_block_state_id,
                    context.sequence,
                ) {
                    return Some(LocalTargetUsePrediction::UnsupportedItemUseOn(owner));
                }
                Some(success())
            }
            VanillaItemUseOnOverride::ShovelItem => {
                if context.face == ProtocolDirection::Down {
                    return Some(pass());
                }
                let flattenable = find_block_transform(VANILLA_SHOVEL_FLATTENABLES, block_type.id);
                if flattenable.is_some() && self.local_block_is_air(above(context.pos))? {
                    return Some(success());
                }
                let lit_campfire = block_type.use_item_on_override
                    == Some(VanillaBlockUseItemOnOverride::CampfireBlock)
                    && properties.get("lit").is_some_and(|value| value == "true");
                Some(if lit_campfire { success() } else { pass() })
            }
            _ => None,
        }
    }

    fn predict_local_client_owned_item_use_on(
        &mut self,
        owner: VanillaItemUseOnOverride,
        hand: InteractionHand,
        context: LocalBlockUseContext,
    ) -> Option<LocalTargetUsePrediction> {
        let client_success = || {
            LocalTargetUsePrediction::Result(LocalInteractionResult::Success(
                LocalInteractionSuccess::client_item_interaction(),
            ))
        };
        let consume = || {
            LocalTargetUsePrediction::Result(LocalInteractionResult::Success(
                LocalInteractionSuccess::consume(true),
            ))
        };
        let pass = || LocalTargetUsePrediction::Result(LocalInteractionResult::Pass);
        match owner {
            // `BrushItem.useOn` always consumes the action and its block hit
            // starts the fixed-duration brushing use animation.
            VanillaItemUseOnOverride::BrushItem => Some(consume()),
            // The permission, component, and block-property work is confined
            // to `ServerPlayer`; the client returns success immediately.
            VanillaItemUseOnOverride::DebugStickItem => Some(client_success()),
            // `LeadItem` only binds mobs on the logical server, while the
            // targeted entry point of `PlaceOnWaterBlockItem` always passes.
            VanillaItemUseOnOverride::LeadItem
            | VanillaItemUseOnOverride::PlaceOnWaterBlockItem => Some(pass()),
            VanillaItemUseOnOverride::CompassItem => {
                let block = self.probe_block(context.pos)?;
                let block_name = block.block_name.as_deref()?;
                let block_type = vanilla_block_type_for_resource_id(block_name)?;
                if block_type.id != VANILLA_BLOCK_LODESTONE_ID {
                    return Some(pass());
                }
                let Some(dimension) = self.level_info().map(|level| level.dimension.clone()) else {
                    return Some(LocalTargetUsePrediction::UnsupportedItemUseOn(owner));
                };
                let Some(dropped) = self.lock_local_compass_to_lodestone(
                    hand,
                    LodestoneTargetSummary {
                        dimension,
                        pos: ProtocolBlockPos {
                            x: context.pos.x,
                            y: context.pos.y,
                            z: context.pos.z,
                        },
                    },
                ) else {
                    return Some(LocalTargetUsePrediction::UnsupportedItemUseOn(owner));
                };
                Some(LocalTargetUsePrediction::Result(
                    LocalInteractionResult::Success(
                        LocalInteractionSuccess::lodestone_compass_lock(dropped),
                    ),
                ))
            }
            VanillaItemUseOnOverride::FlintAndSteelItem
            | VanillaItemUseOnOverride::FireChargeItem => {
                Some(self.predict_local_ignition_item_use_on(owner, hand, context))
            }
            VanillaItemUseOnOverride::BoneMealItem => {
                Some(self.predict_local_bone_meal_item_use_on(owner, context))
            }
            VanillaItemUseOnOverride::PotionItem => {
                Some(self.predict_local_potion_item_use_on(owner, hand, context))
            }
            VanillaItemUseOnOverride::EnderEyeItem => {
                let block = self.probe_block(context.pos)?;
                let block_name = block.block_name.as_deref()?;
                let block_type = vanilla_block_type_for_resource_id(block_name)?;
                if block_type.id != VANILLA_BLOCK_END_PORTAL_FRAME_ID {
                    return Some(pass());
                }
                match block.block_properties.get("eye").map(String::as_str) {
                    Some("false") => Some(client_success()),
                    Some("true") => Some(pass()),
                    _ => Some(LocalTargetUsePrediction::UnsupportedItemUseOn(owner)),
                }
            }
            VanillaItemUseOnOverride::MapItem => {
                let block = self.probe_block(context.pos)?;
                let block_name = block.block_name.as_deref()?;
                let block_type = vanilla_block_type_for_resource_id(block_name)?;
                let Some(banners) =
                    self.registry_holder_ids_for_tag("minecraft:block", "minecraft:banners")
                else {
                    return Some(LocalTargetUsePrediction::UnsupportedItemUseOn(owner));
                };
                Some(if banners.contains(&block_type.id) {
                    client_success()
                } else {
                    pass()
                })
            }
            _ => None,
        }
    }

    fn predict_local_entity_spawning_item_use_on(
        &mut self,
        owner: VanillaItemUseOnOverride,
        hand: InteractionHand,
        context: LocalBlockUseContext,
    ) -> Option<LocalTargetUsePrediction> {
        let client_success = || {
            LocalTargetUsePrediction::Result(LocalInteractionResult::Success(
                LocalInteractionSuccess::client_item_interaction(),
            ))
        };
        let pass = || LocalTargetUsePrediction::Result(LocalInteractionResult::Pass);
        let fail = || LocalTargetUsePrediction::Result(LocalInteractionResult::Fail);
        match owner {
            // `SpawnEggItem.useOn` returns before all server-only type,
            // spawner, difficulty, spawn, and stack-mutation work.
            VanillaItemUseOnOverride::SpawnEggItem => Some(client_success()),
            VanillaItemUseOnOverride::FireworkRocketItem => {
                let fall_flying = self
                    .local_player_id()
                    .and_then(|id| self.entities.is_fall_flying(id))
                    .unwrap_or(false);
                Some(if fall_flying {
                    pass()
                } else {
                    client_success()
                })
            }
            VanillaItemUseOnOverride::EndCrystalItem => {
                let block = self.probe_block(context.pos)?;
                let block_name = block.block_name.as_deref()?;
                let block_type = vanilla_block_type_for_resource_id(block_name)?;
                if !matches!(
                    block_type.id,
                    VANILLA_BLOCK_OBSIDIAN_ID | VANILLA_BLOCK_BEDROCK_ID
                ) {
                    return Some(fail());
                }
                let spawn_pos = above(context.pos);
                if !self.local_block_is_air(spawn_pos)? {
                    return Some(fail());
                }
                let query_min = [
                    f64::from(spawn_pos.x),
                    f64::from(spawn_pos.y),
                    f64::from(spawn_pos.z),
                ];
                let query_max = [query_min[0] + 1.0, query_min[1] + 2.0, query_min[2] + 1.0];
                match self
                    .entities
                    .any_entity_bounds_intersecting(query_min, query_max, |_| true)
                {
                    Some(true) => Some(fail()),
                    Some(false) if self.shrink_local_item_in_hand(hand, 1) => {
                        Some(client_success())
                    }
                    Some(false) => Some(LocalTargetUsePrediction::UnsupportedItemUseOn(owner)),
                    None => Some(LocalTargetUsePrediction::UnsupportedItemUseOn(owner)),
                }
            }
            VanillaItemUseOnOverride::MinecartItem => {
                let block = self.probe_block(context.pos)?;
                let block_name = block.block_name.as_deref()?;
                let block_type = vanilla_block_type_for_resource_id(block_name)?;
                let Some(rails) =
                    self.registry_holder_ids_for_tag("minecraft:block", "minecraft:rails")
                else {
                    return Some(LocalTargetUsePrediction::UnsupportedItemUseOn(owner));
                };
                if !rails.contains(&block_type.id) {
                    return Some(fail());
                }
                if self.is_feature_enabled("minecraft:minecart_improvements") {
                    let bounds = vanilla_pick_bounds_for_type(VANILLA_ENTITY_TYPE_MINECART_ID)?;
                    let slope_offset = block
                        .block_properties
                        .get("shape")
                        .is_some_and(|shape| shape.starts_with("ascending_"))
                        .then_some(0.5)
                        .unwrap_or(0.0);
                    let spawn_position = [
                        f64::from(context.pos.x) + 0.5,
                        f64::from(context.pos.y) + 0.0625 + slope_offset,
                        f64::from(context.pos.z) + 0.5,
                    ];
                    let query_min = std::array::from_fn(|axis| {
                        spawn_position[axis] + f64::from(bounds.min[axis])
                    });
                    let query_max = std::array::from_fn(|axis| {
                        spawn_position[axis] + f64::from(bounds.max[axis])
                    });
                    match self.entities.any_entity_bounds_intersecting(
                        query_min,
                        query_max,
                        vanilla_entity_type_is_minecart,
                    ) {
                        Some(true) => return Some(fail()),
                        Some(false) => {}
                        None => {
                            return Some(LocalTargetUsePrediction::UnsupportedItemUseOn(owner));
                        }
                    }
                }
                Some(if self.shrink_local_item_in_hand(hand, 1) {
                    client_success()
                } else {
                    LocalTargetUsePrediction::UnsupportedItemUseOn(owner)
                })
            }
            _ => None,
        }
    }

    fn predict_local_transformed_block_state(
        &mut self,
        context: LocalBlockUseContext,
        transform: &VanillaBlockTransform,
        source_properties: BTreeMap<String, String>,
        copy_axis_only: bool,
    ) -> bool {
        let Some(target) = vanilla_block_type(transform.target_block_type_id) else {
            return false;
        };
        let target_properties = if copy_axis_only {
            source_properties
                .get("axis")
                .map(|axis| BTreeMap::from([("axis".to_string(), axis.clone())]))
                .unwrap_or_default()
        } else {
            source_properties
        };
        let Some(predicted_block_state_id) = self
            .registries
            .block_state_id_by_name_and_properties(target.resource_id, &target_properties)
        else {
            return false;
        };
        self.predict_local_block_state(context.pos, predicted_block_state_id, context.sequence)
    }

    fn local_block_is_air(&self, pos: BlockPos) -> Option<bool> {
        let block = self.probe_block(pos)?;
        let block_name = block.block_name.as_deref()?;
        Some(vanilla_block_type_for_resource_id(block_name)?.id == VANILLA_BLOCK_AIR_ID)
    }

    /// Predicts `Player.interactOn` across source-derived behavior paths whose client result is
    /// known, applying the local entity state changes that Vanilla performs while computing that
    /// result. The shared `Entity` leash/shears path is canonical here; unimplemented specialized
    /// entity and item interaction owners remain explicit.
    pub fn predict_local_entity_interaction(
        &mut self,
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
            self.predict_local_base_entity_interact(
                entity_id,
                entity_type_id,
                held_item_id,
                secondary_use_active,
            )
        } else if owner == VANILLA_ARMOR_STAND_INTERACT_OWNER {
            self.predict_local_armor_stand_interact(
                entity_id,
                entity_type_id,
                held_item_id,
                secondary_use_active,
            )
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
        self.predict_local_item_stack_interact_living_entity(entity_id, entity_type_id, stack)
    }

    fn predict_local_item_stack_interact_living_entity(
        &self,
        entity_id: i32,
        entity_type_id: i32,
        stack: &bbb_protocol::packets::ItemStackSummary,
    ) -> LocalTargetUsePrediction {
        if let Some(prediction) =
            self.predict_local_equippable_equip_on_target(entity_id, entity_type_id, stack)
        {
            if prediction != LocalTargetUsePrediction::Result(LocalInteractionResult::Pass) {
                return prediction;
            }
        }
        let Some(item_id) = stack.item_id else {
            return LocalTargetUsePrediction::UnknownItemType(None);
        };
        let Some(item_type) = vanilla_item_type(item_id) else {
            return LocalTargetUsePrediction::UnknownItemType(Some(item_id));
        };
        let Some(owner) = item_type.interact_living_entity_override else {
            return LocalTargetUsePrediction::Result(LocalInteractionResult::Pass);
        };
        match owner {
            VanillaItemInteractLivingEntityOverride::DyeItem => {
                let Some((data_id, default, sheared_mask, color_mask)) =
                    vanilla_entity_metadata_sheep_wool_accessor_for_entity_type(entity_type_id)
                else {
                    return LocalTargetUsePrediction::Result(LocalInteractionResult::Pass);
                };
                let Some(health) = self.entities.living_entity_health(entity_id) else {
                    return LocalTargetUsePrediction::UnknownEntityType(Some(entity_type_id));
                };
                if !(health > 0.0) {
                    return LocalTargetUsePrediction::Result(LocalInteractionResult::Pass);
                }
                let Some(wool) = self
                    .entities
                    .metadata_byte_for_entity(entity_id, data_id, default)
                else {
                    return LocalTargetUsePrediction::UnknownEntityType(Some(entity_type_id));
                };
                if wool & sheared_mask != 0
                    || !self.item_stack_has_effective_component(
                        stack,
                        VANILLA_DATA_COMPONENT_DYE_TYPE_ID,
                    )
                {
                    return LocalTargetUsePrediction::Result(LocalInteractionResult::Pass);
                }
                let effective = self.effective_item_stack(stack);
                let Some(dye_color_id) = effective.component_patch.dye_color_id else {
                    return LocalTargetUsePrediction::UnsupportedItemInteractLivingEntity(owner);
                };
                let dye_color_id = normalize_vanilla_dye_color_id(dye_color_id);
                let wool_color_id = i32::from((wool as u8) & color_mask);
                if wool_color_id == dye_color_id {
                    LocalTargetUsePrediction::Result(LocalInteractionResult::Pass)
                } else {
                    LocalTargetUsePrediction::Result(LocalInteractionResult::Success(
                        LocalInteractionSuccess::client_item_interaction(),
                    ))
                }
            }
            VanillaItemInteractLivingEntityOverride::NameTagItem => {
                if !self.item_stack_has_effective_component(
                    stack,
                    VANILLA_DATA_COMPONENT_CUSTOM_NAME_TYPE_ID,
                ) || !vanilla_entity_type_is_serializable(entity_type_id)
                {
                    return LocalTargetUsePrediction::Result(LocalInteractionResult::Pass);
                }
                let effective = self.effective_item_stack(stack);
                let patch = &effective.component_patch;
                if patch.custom_name.is_none()
                    && patch.custom_name_styled.is_none()
                    && patch.custom_name_nbt.is_none()
                {
                    return LocalTargetUsePrediction::UnsupportedItemInteractLivingEntity(owner);
                }
                LocalTargetUsePrediction::Result(LocalInteractionResult::Success(
                    LocalInteractionSuccess::client_item_interaction(),
                ))
            }
        }
    }

    fn predict_local_armor_stand_interact(
        &mut self,
        entity_id: i32,
        entity_type_id: i32,
        held_item_id: Option<i32>,
        secondary_use_active: bool,
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
            self.predict_local_base_entity_interact(
                entity_id,
                entity_type_id,
                held_item_id,
                secondary_use_active,
            )
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
        &mut self,
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
        let base_result = self.predict_local_base_entity_interact(
            entity_id,
            entity_type_id,
            held_item_id,
            secondary_use_active,
        );
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
        &mut self,
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

        let held_stack = self.local_item_in_hand(hand).cloned();
        let held_item_id = held_stack.as_ref().and_then(|stack| stack.item_id);
        if let Some(item_id) = held_item_id {
            let Some(item_type) = vanilla_item_type(item_id) else {
                return LocalTargetUsePrediction::UnknownItemType(Some(item_id));
            };
            if item_type.interact_living_entity_override
                == Some(VanillaItemInteractLivingEntityOverride::NameTagItem)
            {
                let result = self.predict_local_item_stack_interact_living_entity(
                    entity_id,
                    entity_type_id,
                    held_stack
                        .as_ref()
                        .expect("held item id came from this stack"),
                );
                if result != LocalTargetUsePrediction::Result(LocalInteractionResult::Pass) {
                    return result;
                }
            }
            if item_type.use_override == Some(VanillaItemUseOverride::SpawnEggItem) {
                return LocalTargetUsePrediction::Result(LocalInteractionResult::Success(
                    LocalInteractionSuccess::server_item_interaction(),
                ));
            }
        }

        let base_result = self.predict_local_base_entity_interact(
            entity_id,
            entity_type_id,
            held_item_id,
            secondary_use_active,
        );
        if base_result != LocalTargetUsePrediction::Result(LocalInteractionResult::Pass) {
            return base_result;
        }

        let Some(owner) = vanilla_entity_metadata_mob_interact_owner(entity_type_id) else {
            return LocalTargetUsePrediction::UnknownEntityType(Some(entity_type_id));
        };
        if interact_owner == VANILLA_CAMEL_HUSK_INTERACT_OWNER
            && owner == VANILLA_CAMEL_MOB_INTERACT_OWNER
        {
            return self.predict_local_camel_husk_mob_interact(
                entity_id,
                entity_type_id,
                hand,
                held_item_id,
                secondary_use_active,
            );
        }
        if interact_owner == VANILLA_ZOMBIE_HORSE_INTERACT_OWNER
            && owner == VANILLA_ZOMBIE_HORSE_INTERACT_OWNER
        {
            return self.predict_local_zombie_horse_mob_interact(
                entity_id,
                entity_type_id,
                hand,
                held_item_id,
                secondary_use_active,
            );
        }
        if interact_owner == VANILLA_NAUTILUS_INTERACT_OWNER
            && owner == VANILLA_NAUTILUS_INTERACT_OWNER
        {
            return self.predict_local_nautilus_mob_interact(
                entity_id,
                entity_type_id,
                hand,
                held_item_id,
                secondary_use_active,
            );
        }
        if held_item_id == Some(VANILLA_ITEM_SHEARS_ID) {
            if let Some(result) = self.predict_local_specialized_shearable_mob_interact(
                entity_id,
                entity_type_id,
                owner,
            ) {
                return result;
            }
        }
        if owner == VANILLA_IRON_GOLEM_MOB_INTERACT_OWNER {
            return self.predict_local_iron_golem_mob_interact(
                entity_id,
                entity_type_id,
                held_item_id,
            );
        }
        if owner == VANILLA_CREEPER_MOB_INTERACT_OWNER {
            return self.predict_local_tagged_client_success_mob_interact(
                held_item_id,
                VANILLA_CREEPER_IGNITERS_TAG,
                VANILLA_CREEPER_MOB_INTERACT_OWNER,
            );
        }
        if owner == VANILLA_SHEEP_MOB_INTERACT_OWNER {
            if let Some(item_id) = held_item_id {
                let Some(food_item_ids) =
                    self.registry_holder_ids_for_tag("minecraft:item", VANILLA_SHEEP_FOOD_TAG)
                else {
                    return LocalTargetUsePrediction::UnsupportedEntityInteract(owner);
                };
                if food_item_ids.contains(&item_id) {
                    // `Animal.mobInteract` depends on age and love state and
                    // consumes Sheep food before `ItemStack.interactLivingEntity`.
                    return LocalTargetUsePrediction::UnsupportedEntityInteract(owner);
                }
            }
            return LocalTargetUsePrediction::Result(LocalInteractionResult::Pass);
        }
        if owner != VANILLA_MOB_ENTITY_INTERACT_OWNER {
            return LocalTargetUsePrediction::UnsupportedEntityInteract(owner);
        }
        LocalTargetUsePrediction::Result(LocalInteractionResult::Pass)
    }

    fn predict_local_specialized_shearable_mob_interact(
        &self,
        entity_id: i32,
        entity_type_id: i32,
        owner: &'static str,
    ) -> Option<LocalTargetUsePrediction> {
        let client_success = || {
            LocalTargetUsePrediction::Result(LocalInteractionResult::Success(
                LocalInteractionSuccess::client_item_interaction(),
            ))
        };
        let pass = || LocalTargetUsePrediction::Result(LocalInteractionResult::Pass);
        Some(match owner {
            VANILLA_BOGGED_MOB_INTERACT_OWNER => {
                let Some((data_id, default)) =
                    vanilla_entity_metadata_bogged_sheared_accessor_for_entity_type(entity_type_id)
                else {
                    return Some(LocalTargetUsePrediction::UnknownEntityType(Some(
                        entity_type_id,
                    )));
                };
                let Some(sheared) = self
                    .entities
                    .metadata_bool_for_entity(entity_id, data_id, default)
                else {
                    return Some(LocalTargetUsePrediction::UnknownEntityType(Some(
                        entity_type_id,
                    )));
                };
                if sheared {
                    pass()
                } else {
                    client_success()
                }
            }
            VANILLA_COPPER_GOLEM_MOB_INTERACT_OWNER => {
                let Some(antenna) = self
                    .entities
                    .equipment_item(entity_id, EquipmentSlot::Saddle)
                else {
                    return Some(pass());
                };
                let Some(antenna_item_id) = antenna.item_id else {
                    return Some(pass());
                };
                let Some(item_ids) = self.registry_holder_ids_for_tag(
                    "minecraft:item",
                    VANILLA_COPPER_GOLEM_SHEARABLE_ITEMS_TAG,
                ) else {
                    return Some(LocalTargetUsePrediction::UnsupportedEntityInteract(owner));
                };
                if item_ids.contains(&antenna_item_id) {
                    client_success()
                } else {
                    pass()
                }
            }
            VANILLA_MOOSHROOM_MOB_INTERACT_OWNER => {
                let Some(is_baby) = self.entities.is_baby(entity_id) else {
                    return Some(LocalTargetUsePrediction::UnknownEntityType(Some(
                        entity_type_id,
                    )));
                };
                if is_baby {
                    // A baby falls through to `AbstractCow`/`Animal`, whose food-tag behavior
                    // remains part of the wider specialized mob-interaction owner.
                    LocalTargetUsePrediction::UnsupportedEntityInteract(owner)
                } else {
                    client_success()
                }
            }
            VANILLA_SHEEP_MOB_INTERACT_OWNER => LocalTargetUsePrediction::Result(
                LocalInteractionResult::Success(LocalInteractionSuccess::consume(false)),
            ),
            VANILLA_SNOW_GOLEM_MOB_INTERACT_OWNER => {
                let Some((data_id, default, pumpkin_mask)) =
                    vanilla_entity_metadata_snow_golem_pumpkin_accessor_for_entity_type(
                        entity_type_id,
                    )
                else {
                    return Some(LocalTargetUsePrediction::UnknownEntityType(Some(
                        entity_type_id,
                    )));
                };
                let Some(flags) = self
                    .entities
                    .metadata_byte_for_entity(entity_id, data_id, default)
                else {
                    return Some(LocalTargetUsePrediction::UnknownEntityType(Some(
                        entity_type_id,
                    )));
                };
                if flags & pumpkin_mask != 0 {
                    client_success()
                } else {
                    pass()
                }
            }
            _ => return None,
        })
    }

    fn predict_local_camel_husk_mob_interact(
        &self,
        entity_id: i32,
        entity_type_id: i32,
        hand: InteractionHand,
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
        let Some(stack) = self.local_item_in_hand(hand) else {
            return LocalTargetUsePrediction::UnknownItemType(Some(item_id));
        };
        let item_interaction =
            self.predict_local_item_stack_interact_living_entity(entity_id, entity_type_id, stack);
        if item_interaction != LocalTargetUsePrediction::Result(LocalInteractionResult::Pass) {
            return item_interaction;
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
        hand: InteractionHand,
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
            let Some(stack) = self.local_item_in_hand(hand) else {
                return LocalTargetUsePrediction::UnknownItemType(Some(item_id));
            };
            let item_interaction = self.predict_local_item_stack_interact_living_entity(
                entity_id,
                entity_type_id,
                stack,
            );
            if item_interaction != LocalTargetUsePrediction::Result(LocalInteractionResult::Pass) {
                return item_interaction;
            }
            return LocalTargetUsePrediction::Result(LocalInteractionResult::Success(
                LocalInteractionSuccess::client_item_interaction(),
            ));
        }

        // Zombie horses are in `EntityTypeTags.CANNOT_BE_AGE_LOCKED`, so their
        // baby/vehicle `AgeableMob` fallback has no golden-dandelion success.
        LocalTargetUsePrediction::Result(LocalInteractionResult::Pass)
    }

    fn predict_local_nautilus_mob_interact(
        &mut self,
        entity_id: i32,
        entity_type_id: i32,
        hand: InteractionHand,
        held_item_id: Option<i32>,
        secondary_use_active: bool,
    ) -> LocalTargetUsePrediction {
        let Some(is_baby) = self.entities.is_baby(entity_id) else {
            return LocalTargetUsePrediction::UnknownEntityType(Some(entity_type_id));
        };
        let Some(accessors) =
            vanilla_entity_metadata_tamable_animal_accessors_for_entity_type(entity_type_id)
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

        if !is_baby && is_tamed && secondary_use_active {
            return LocalTargetUsePrediction::Result(LocalInteractionResult::Success(
                LocalInteractionSuccess::client_item_interaction(),
            ));
        }
        let Some(item_id) = held_item_id else {
            return if !is_baby && is_tamed {
                LocalTargetUsePrediction::Result(LocalInteractionResult::Success(
                    LocalInteractionSuccess::client_item_interaction(),
                ))
            } else {
                LocalTargetUsePrediction::Result(LocalInteractionResult::Pass)
            };
        };
        let food_tag = if !is_baby && !is_tamed {
            VANILLA_NAUTILUS_TAMING_ITEMS_TAG
        } else {
            VANILLA_NAUTILUS_FOOD_TAG
        };
        let Some(food_item_ids) = self.registry_holder_ids_for_tag("minecraft:item", food_tag)
        else {
            return LocalTargetUsePrediction::UnsupportedEntityInteract(
                VANILLA_NAUTILUS_INTERACT_OWNER,
            );
        };
        let is_food = food_item_ids.contains(&item_id);

        if is_baby {
            if is_food {
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
                return LocalTargetUsePrediction::Result(LocalInteractionResult::Success(
                    if age_locked {
                        LocalInteractionSuccess::consume(false)
                    } else {
                        LocalInteractionSuccess::client_item_interaction()
                    },
                ));
            }
            if item_id == VANILLA_ITEM_GOLDEN_DANDELION_ID {
                let Some(timer_ticks) = self.entities.age_lock_particle_timer_ticks(entity_id)
                else {
                    return LocalTargetUsePrediction::UnknownEntityType(Some(entity_type_id));
                };
                if timer_ticks == 0 {
                    if self.entities.apply_local_age_lock_use(entity_id).is_none() {
                        return LocalTargetUsePrediction::UnknownEntityType(Some(entity_type_id));
                    }
                    return LocalTargetUsePrediction::Result(LocalInteractionResult::Success(
                        LocalInteractionSuccess::client_item_interaction(),
                    ));
                }
            }
            return LocalTargetUsePrediction::Result(LocalInteractionResult::Pass);
        }

        if is_food {
            let Some(health) = self.entities.living_entity_health(entity_id) else {
                return LocalTargetUsePrediction::UnknownEntityType(Some(entity_type_id));
            };
            let Some(max_health) = self
                .entities
                .attribute_value(entity_id, VANILLA_ATTRIBUTE_MAX_HEALTH_ID)
            else {
                return LocalTargetUsePrediction::UnknownEntityType(Some(entity_type_id));
            };
            if health < max_health as f32 {
                return LocalTargetUsePrediction::Result(LocalInteractionResult::Success(
                    LocalInteractionSuccess::client_item_interaction(),
                ));
            }
        }
        let Some(stack) = self.local_item_in_hand(hand) else {
            return LocalTargetUsePrediction::UnknownItemType(Some(item_id));
        };
        let item_interaction =
            self.predict_local_item_stack_interact_living_entity(entity_id, entity_type_id, stack);
        if item_interaction != LocalTargetUsePrediction::Result(LocalInteractionResult::Pass) {
            return item_interaction;
        }
        if is_food {
            return LocalTargetUsePrediction::Result(LocalInteractionResult::Success(
                LocalInteractionSuccess::consume(false),
            ));
        }
        if is_tamed && !secondary_use_active {
            return LocalTargetUsePrediction::Result(LocalInteractionResult::Success(
                LocalInteractionSuccess::client_item_interaction(),
            ));
        }
        LocalTargetUsePrediction::Result(LocalInteractionResult::Pass)
    }

    fn predict_local_iron_golem_mob_interact(
        &mut self,
        entity_id: i32,
        entity_type_id: i32,
        held_item_id: Option<i32>,
    ) -> LocalTargetUsePrediction {
        if held_item_id != Some(VANILLA_ITEM_IRON_INGOT_ID) {
            return LocalTargetUsePrediction::Result(LocalInteractionResult::Pass);
        }
        let Some(healed) = self.entities.apply_local_living_heal(entity_id, 25.0) else {
            return LocalTargetUsePrediction::UnknownEntityType(Some(entity_type_id));
        };
        if healed {
            LocalTargetUsePrediction::Result(LocalInteractionResult::Success(
                LocalInteractionSuccess::client_item_interaction(),
            ))
        } else {
            LocalTargetUsePrediction::Result(LocalInteractionResult::Pass)
        }
    }

    fn predict_local_tagged_client_success_mob_interact(
        &self,
        held_item_id: Option<i32>,
        item_tag: &'static str,
        owner: &'static str,
    ) -> LocalTargetUsePrediction {
        let Some(item_id) = held_item_id else {
            return LocalTargetUsePrediction::Result(LocalInteractionResult::Pass);
        };
        let Some(item_ids) = self.registry_holder_ids_for_tag("minecraft:item", item_tag) else {
            return LocalTargetUsePrediction::UnsupportedEntityInteract(owner);
        };
        if item_ids.contains(&item_id) {
            LocalTargetUsePrediction::Result(LocalInteractionResult::Success(
                LocalInteractionSuccess::client_item_interaction(),
            ))
        } else {
            LocalTargetUsePrediction::Result(LocalInteractionResult::Pass)
        }
    }

    fn predict_local_base_entity_interact(
        &mut self,
        entity_id: i32,
        entity_type_id: i32,
        held_item_id: Option<i32>,
        secondary_use_active: bool,
    ) -> LocalTargetUsePrediction {
        if held_item_id == Some(VANILLA_ITEM_SHEARS_ID) {
            let dropped_leashes = self.entities.drop_local_leash_connections(entity_id);
            match dropped_leashes {
                Some(true) => {
                    return LocalTargetUsePrediction::Result(LocalInteractionResult::Success(
                        LocalInteractionSuccess::client_item_interaction(),
                    ));
                }
                None => {
                    return LocalTargetUsePrediction::UnsupportedEntityInteract(
                        VANILLA_DEFAULT_ENTITY_INTERACT_OWNER,
                    );
                }
                Some(false) => {}
            }
            if vanilla_entity_metadata_mob_interact_owner(entity_type_id).is_some()
                && !secondary_use_active
            {
                let Some(can_shear_equipment) =
                    self.local_mob_can_shear_equipment(entity_id, entity_type_id)
                else {
                    return LocalTargetUsePrediction::UnsupportedEntityInteract(
                        VANILLA_DEFAULT_ENTITY_INTERACT_OWNER,
                    );
                };
                if !can_shear_equipment {
                    return self.predict_local_base_leash_interact(
                        entity_id,
                        entity_type_id,
                        held_item_id,
                    );
                }
                for slot in VANILLA_EQUIPMENT_SLOTS.iter().map(|entry| entry.slot) {
                    let Some(item) = self.entities.equipment_item(entity_id, slot) else {
                        continue;
                    };
                    let Some(can_be_sheared) = self.effective_equipment_can_be_sheared(&item)
                    else {
                        return LocalTargetUsePrediction::UnsupportedEntityInteract(
                            VANILLA_DEFAULT_ENTITY_INTERACT_OWNER,
                        );
                    };
                    if !can_be_sheared {
                        continue;
                    }
                    if !self.local_player_is_creative() {
                        let Some(prevents_armor_change) =
                            self.equipment_prevents_armor_change(&item)
                        else {
                            return LocalTargetUsePrediction::UnsupportedEntityInteract(
                                VANILLA_DEFAULT_ENTITY_INTERACT_OWNER,
                            );
                        };
                        if prevents_armor_change {
                            continue;
                        }
                    }
                    if !self.entities.clear_local_equipment_item(entity_id, slot) {
                        return LocalTargetUsePrediction::UnsupportedEntityInteract(
                            VANILLA_DEFAULT_ENTITY_INTERACT_OWNER,
                        );
                    }
                    return LocalTargetUsePrediction::Result(LocalInteractionResult::Success(
                        LocalInteractionSuccess::client_item_interaction(),
                    ));
                }
            }
        }
        self.predict_local_base_leash_interact(entity_id, entity_type_id, held_item_id)
    }

    fn predict_local_base_leash_interact(
        &self,
        entity_id: i32,
        entity_type_id: i32,
        held_item_id: Option<i32>,
    ) -> LocalTargetUsePrediction {
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

    fn effective_equipment_can_be_sheared(&self, item: &ProtocolItemStackSummary) -> Option<bool> {
        let patch = &item.component_patch;
        if patch
            .removed_type_ids
            .contains(&VANILLA_DATA_COMPONENT_EQUIPPABLE_TYPE_ID)
        {
            return Some(false);
        }
        if patch
            .added_type_ids
            .contains(&VANILLA_DATA_COMPONENT_EQUIPPABLE_TYPE_ID)
        {
            return patch
                .equippable
                .as_ref()
                .map(|equippable| equippable.can_be_sheared);
        }
        Some(item.item_id.is_some_and(|item_id| {
            self.items
                .default_shearable_equipment_item_ids
                .contains(&item_id)
        }))
    }

    fn local_mob_can_shear_equipment(&self, entity_id: i32, entity_type_id: i32) -> Option<bool> {
        if entity_type_id != VANILLA_ENTITY_TYPE_WOLF_ID {
            return Some(!self.entities.entity_has_passengers(entity_id));
        }
        let accessors =
            vanilla_entity_metadata_tamable_animal_accessors_for_entity_type(entity_type_id)?;
        let owner = self
            .entities
            .metadata_optional_living_entity_reference_for_entity(
                entity_id,
                accessors.owner_data_id,
                accessors.owner_default,
            )?;
        match owner {
            None => Some(false),
            Some(owner) => self
                .local_player_profile_id()
                .map(|local_player| local_player == owner),
        }
    }

    fn equipment_prevents_armor_change(&self, item: &ProtocolItemStackSummary) -> Option<bool> {
        let patch = &item.component_patch;
        if patch
            .removed_type_ids
            .contains(&VANILLA_DATA_COMPONENT_ENCHANTMENTS_TYPE_ID)
            || patch.enchantments.is_empty()
        {
            return Some(false);
        }
        for enchantment in &patch.enchantments {
            if self.enchantment_prevents_armor_change(enchantment.holder_id)? {
                return Some(true);
            }
        }
        Some(false)
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
        self.local_item_stack_cooldown_group(stack)
            .is_some_and(|group| self.cooldown(&group).is_some())
    }

    fn local_item_stack_cooldown_group(&self, stack: &ProtocolItemStackSummary) -> Option<String> {
        let effective = self.effective_item_stack(stack);
        let configured_group = self
            .item_stack_has_effective_component(stack, VANILLA_DATA_COMPONENT_USE_COOLDOWN_TYPE_ID)
            .then(|| effective.component_patch.use_cooldown_group.as_deref())
            .flatten();
        configured_group
            .or_else(|| {
                stack
                    .item_id
                    .and_then(vanilla_item_type)
                    .map(|item| item.resource_id)
            })
            .map(str::to_owned)
    }
}

fn find_block_transform(
    transforms: &'static [VanillaBlockTransform],
    source_block_type_id: i32,
) -> Option<&'static VanillaBlockTransform> {
    transforms
        .iter()
        .find(|transform| transform.source_block_type_id == source_block_type_id)
}

fn above(pos: BlockPos) -> BlockPos {
    BlockPos {
        x: pos.x,
        y: pos.y.saturating_add(1),
        z: pos.z,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bbb_protocol::{
        block_types::{
            vanilla_block_type, vanilla_block_type_for_resource_id, VanillaBlockUseItemOnOverride,
            VanillaBlockUseWithoutItemOverride, VanillaBonemealTargetOverride,
            VANILLA_BLOCK_CAKE_ID, VANILLA_BLOCK_CHEST_ID, VANILLA_BLOCK_CRIMSON_NYLIUM_ID,
            VANILLA_BLOCK_DIRT_ID, VANILLA_BLOCK_RAIL_ID, VANILLA_BLOCK_STONE_ID,
            VANILLA_BLOCK_WHITE_BANNER_ID,
        },
        builtin_enum_component_values::VANILLA_DYE_COLOR_RED_ID,
        data_component_types::{
            VANILLA_DATA_COMPONENT_CUSTOM_NAME_TYPE_ID, VANILLA_DATA_COMPONENT_DYE_TYPE_ID,
            VANILLA_DATA_COMPONENT_ENCHANTMENTS_TYPE_ID, VANILLA_DATA_COMPONENT_EQUIPPABLE_TYPE_ID,
            VANILLA_DATA_COMPONENT_FOOD_TYPE_ID, VANILLA_DATA_COMPONENT_INSTRUMENT_TYPE_ID,
            VANILLA_DATA_COMPONENT_LODESTONE_TRACKER_TYPE_ID,
            VANILLA_DATA_COMPONENT_POTION_CONTENTS_TYPE_ID,
            VANILLA_DATA_COMPONENT_USE_COOLDOWN_TYPE_ID,
        },
        entity_data_serializers::{
            VANILLA_ENTITY_DATA_SERIALIZER_BOOLEAN_ID, VANILLA_ENTITY_DATA_SERIALIZER_BYTE_ID,
            VANILLA_ENTITY_DATA_SERIALIZER_FLOAT_ID, VANILLA_ENTITY_DATA_SERIALIZER_ITEM_STACK_ID,
            VANILLA_ENTITY_DATA_SERIALIZER_OPTIONAL_LIVING_ENTITY_REFERENCE_ID,
        },
        entity_types::{
            VANILLA_ENTITY_TYPE_AREA_EFFECT_CLOUD_ID, VANILLA_ENTITY_TYPE_ARMOR_STAND_ID,
            VANILLA_ENTITY_TYPE_BOGGED_ID, VANILLA_ENTITY_TYPE_CAMEL_HUSK_ID,
            VANILLA_ENTITY_TYPE_CHEST_MINECART_ID, VANILLA_ENTITY_TYPE_COMMAND_BLOCK_MINECART_ID,
            VANILLA_ENTITY_TYPE_COPPER_GOLEM_ID, VANILLA_ENTITY_TYPE_COW_ID,
            VANILLA_ENTITY_TYPE_CREEPER_ID, VANILLA_ENTITY_TYPE_FURNACE_MINECART_ID,
            VANILLA_ENTITY_TYPE_HOPPER_MINECART_ID, VANILLA_ENTITY_TYPE_INTERACTION_ID,
            VANILLA_ENTITY_TYPE_IRON_GOLEM_ID, VANILLA_ENTITY_TYPE_ITEM_FRAME_ID,
            VANILLA_ENTITY_TYPE_LEASH_KNOT_ID, VANILLA_ENTITY_TYPE_MANNEQUIN_ID,
            VANILLA_ENTITY_TYPE_MINECART_ID, VANILLA_ENTITY_TYPE_MOOSHROOM_ID,
            VANILLA_ENTITY_TYPE_NAUTILUS_ID, VANILLA_ENTITY_TYPE_OAK_BOAT_ID,
            VANILLA_ENTITY_TYPE_OAK_CHEST_BOAT_ID, VANILLA_ENTITY_TYPE_PLAYER_ID,
            VANILLA_ENTITY_TYPE_SHEEP_ID, VANILLA_ENTITY_TYPE_SNOW_GOLEM_ID,
            VANILLA_ENTITY_TYPE_ZOMBIE_HORSE_ID, VANILLA_ENTITY_TYPE_ZOMBIE_ID,
            VANILLA_ENTITY_TYPE_ZOMBIE_NAUTILUS_ID,
        },
        equipment_slots::EquipmentSlot,
        game_types::{VANILLA_GAME_TYPE_CREATIVE_ID, VANILLA_GAME_TYPE_SURVIVAL_ID},
        item_types::{
            VanillaItemInteractLivingEntityOverride, VanillaItemUseOnOverride,
            VANILLA_ITEM_APPLE_ID, VANILLA_ITEM_BONE_MEAL_ID, VANILLA_ITEM_BRUSH_ID,
            VANILLA_ITEM_CHICKEN_SPAWN_EGG_ID, VANILLA_ITEM_CHORUS_FRUIT_ID,
            VANILLA_ITEM_COMPASS_ID, VANILLA_ITEM_DEBUG_STICK_ID, VANILLA_ITEM_ENDER_EYE_ID,
            VANILLA_ITEM_END_CRYSTAL_ID, VANILLA_ITEM_FILLED_MAP_ID,
            VANILLA_ITEM_FIREWORK_ROCKET_ID, VANILLA_ITEM_FIRE_CHARGE_ID,
            VANILLA_ITEM_FLINT_AND_STEEL_ID, VANILLA_ITEM_GLASS_BOTTLE_ID,
            VANILLA_ITEM_GOAT_HORN_ID, VANILLA_ITEM_HONEYCOMB_ID, VANILLA_ITEM_IRON_AXE_ID,
            VANILLA_ITEM_IRON_HOE_ID, VANILLA_ITEM_IRON_SHOVEL_ID, VANILLA_ITEM_LEAD_ID,
            VANILLA_ITEM_LILY_PAD_ID, VANILLA_ITEM_MINECART_ID, VANILLA_ITEM_NAME_TAG_ID,
            VANILLA_ITEM_POPPY_ID, VANILLA_ITEM_POTION_ID, VANILLA_ITEM_RED_DYE_ID,
            VANILLA_ITEM_SADDLE_ID, VANILLA_ITEM_SHEARS_ID, VANILLA_ITEM_SHIELD_ID,
            VANILLA_ITEM_STONE_ID,
        },
        packets::{
            AddEntity, AdventureModePredicateDataSummary, BlockHolderSetSummary,
            BlockPos as ProtocolBlockPos, BlockPredicateSummary, BlockUpdate,
            CommonPlayerSpawnInfo, Cooldown, DataComponentPatchSummary, EntityDataValue,
            EntityDataValueKind, EntityEvent, EquipmentSlotUpdate, EquippableSummary,
            InstrumentSummary, ItemEnchantmentSummary, ItemStackSummary, PlayLogin, PlayerHealth,
            RegistryData, RegistryDataEntry, RegistryTags, SetChunkCacheCenter, SetEntityData,
            SetEntityLink, SetEquipment, SetPassengers, SetPlayerInventory, SoundEventSummary,
            TagNetworkPayload, UpdateEnabledFeatures, UpdateTags, Vec3d,
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

    fn potion_stack(potion_id: i32, count: i32) -> ItemStackSummary {
        ItemStackSummary {
            item_id: Some(VANILLA_ITEM_POTION_ID),
            count,
            component_patch: DataComponentPatchSummary {
                added: 1,
                added_type_ids: vec![VANILLA_DATA_COMPONENT_POTION_CONTENTS_TYPE_ID],
                potion_id: Some(potion_id),
                ..DataComponentPatchSummary::default()
            },
        }
    }

    fn direct_instrument_stack(
        use_duration: f32,
        range: f32,
        cooldown_group: Option<&str>,
    ) -> ItemStackSummary {
        let mut added_type_ids = vec![VANILLA_DATA_COMPONENT_INSTRUMENT_TYPE_ID];
        if cooldown_group.is_some() {
            added_type_ids.push(VANILLA_DATA_COMPONENT_USE_COOLDOWN_TYPE_ID);
        }
        ItemStackSummary {
            item_id: Some(VANILLA_ITEM_GOAT_HORN_ID),
            count: 1,
            component_patch: DataComponentPatchSummary {
                added: added_type_ids.len(),
                added_type_ids,
                instrument_direct: Some(Box::new(InstrumentSummary {
                    sound_event: SoundEventSummary {
                        registry_id: None,
                        sound_id: Some("example:direct_horn".to_string()),
                        fixed_range_bits: Some(24.0_f32.to_bits()),
                    },
                    use_duration_bits: use_duration.to_bits(),
                    range_bits: range.to_bits(),
                    description: "Direct horn".to_string(),
                    description_styled: None,
                    description_nbt: None,
                })),
                use_cooldown_group: cooldown_group.map(str::to_owned),
                ..DataComponentPatchSummary::default()
            },
        }
    }

    fn install_convertable_to_mud_tag(world: &mut WorldStore) {
        world.apply_update_tags(UpdateTags {
            registries: vec![RegistryTags {
                registry: "minecraft:block".to_string(),
                tags: vec![TagNetworkPayload {
                    tag: "minecraft:convertable_to_mud".to_string(),
                    entries: vec![VANILLA_BLOCK_DIRT_ID],
                }],
            }],
        });
    }

    fn set_offhand(world: &mut WorldStore, item: ItemStackSummary) {
        world.apply_set_player_inventory(SetPlayerInventory {
            slot: VANILLA_PLAYER_INVENTORY_OFFHAND_SLOT,
            item,
        });
    }

    fn player_inventory_item(world: &WorldStore, slot: i32) -> &ItemStackSummary {
        &world
            .inventory()
            .player_slots
            .iter()
            .find(|entry| entry.slot == slot)
            .unwrap_or_else(|| panic!("missing player inventory slot {slot}"))
            .item
    }

    fn block_state_id<const N: usize>(name: &str, properties: [(&str, &str); N]) -> i32 {
        crate::vanilla_block_state_id(
            name,
            &properties
                .into_iter()
                .map(|(name, value)| (name.to_string(), value.to_string()))
                .collect(),
        )
        .unwrap()
    }

    fn tool_use_world(initial_block_state_id: i32) -> WorldStore {
        tool_use_world_with_login(initial_block_state_id, None)
    }

    fn tool_use_world_with_login(
        initial_block_state_id: i32,
        login: Option<(&str, i8)>,
    ) -> WorldStore {
        let single_value = |domain, entry_count, global_id| crate::PalettedContainerData {
            domain,
            bits_per_entry: 0,
            palette_kind: crate::PaletteKind::SingleValue,
            palette_global_ids: vec![global_id],
            packed_data: Vec::new(),
            entry_count,
        };
        let air = block_state_id("minecraft:air", []);
        let mut world = WorldStore::with_dimension(crate::WorldDimension {
            min_y: 0,
            height: 16,
        });
        if let Some((dimension, game_type)) = login {
            world.apply_login(&PlayLogin {
                player_id: 7,
                hardcore: false,
                levels: vec![dimension.to_string()],
                max_players: 20,
                chunk_radius: 8,
                simulation_distance: 6,
                reduced_debug_info: false,
                show_death_screen: true,
                do_limited_crafting: false,
                common_spawn_info: CommonPlayerSpawnInfo {
                    dimension_type_id: 0,
                    dimension: dimension.to_string(),
                    seed: 0,
                    game_type,
                    previous_game_type: -1,
                    is_debug: false,
                    is_flat: false,
                    last_death_location: None,
                    portal_cooldown: 0,
                    sea_level: 63,
                },
                enforces_secure_chat: true,
            });
            world.apply_set_chunk_cache_center(SetChunkCacheCenter {
                chunk_x: 0,
                chunk_z: 0,
            });
        }
        let section_count = usize::try_from(world.dimension().height.div_euclid(16)).unwrap();
        world.insert_decoded_chunk(crate::ChunkColumn {
            pos: crate::ChunkPos { x: 0, z: 0 },
            state: crate::ChunkState::Decoded,
            heightmaps: Vec::new(),
            sections: (0..section_count)
                .map(|_| crate::ChunkSection {
                    non_empty_block_count: 0,
                    fluid_count: 0,
                    block_states: single_value(crate::PaletteDomain::BlockStates, 4096, air),
                    biomes: single_value(crate::PaletteDomain::Biomes, 64, 0),
                })
                .collect(),
            block_entities: Vec::new(),
            light: crate::LightData::default(),
        });
        assert!(world.apply_block_update(BlockUpdate {
            pos: ProtocolBlockPos { x: 0, y: 0, z: 0 },
            block_state_id: initial_block_state_id,
        }));
        world
    }

    fn add_entity(world: &mut WorldStore, id: i32, entity_type_id: i32) {
        add_entity_at(world, id, entity_type_id, Vec3d::default());
    }

    fn set_block(world: &mut WorldStore, pos: BlockPos, block_state_id: i32) {
        assert!(world.apply_block_update(BlockUpdate {
            pos: ProtocolBlockPos {
                x: pos.x,
                y: pos.y,
                z: pos.z,
            },
            block_state_id,
        }));
    }

    fn add_entity_at(world: &mut WorldStore, id: i32, entity_type_id: i32, position: Vec3d) {
        world.apply_add_entity(AddEntity {
            id,
            uuid: Uuid::from_u128(id as u128),
            entity_type_id,
            position,
            delta_movement: Vec3d::default(),
            x_rot: 0.0,
            y_rot: 0.0,
            y_head_rot: 0.0,
            data: 0,
        });
    }

    fn equippable(can_be_sheared: bool) -> EquippableSummary {
        let sound = || SoundEventSummary {
            registry_id: Some(0),
            sound_id: None,
            fixed_range_bits: None,
        };
        EquippableSummary {
            slot_id: 0,
            equip_sound: sound(),
            asset_id: None,
            camera_overlay: None,
            allowed_entities: None,
            dispensable: true,
            swappable: true,
            damage_on_hurt: true,
            equip_on_interact: false,
            can_be_sheared,
            shearing_sound: sound(),
        }
    }

    fn install_test_enchantment_registry(world: &mut WorldStore) {
        world.record_registry_data(RegistryData {
            registry: "minecraft:enchantment".to_string(),
            entries: vec![
                RegistryDataEntry {
                    id: "minecraft:binding_curse".to_string(),
                    raw_data: None,
                },
                RegistryDataEntry {
                    id: "minecraft:sharpness".to_string(),
                    raw_data: None,
                },
            ],
            raw_payload_len: 0,
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
        world.set_default_swappable_equipment_item_ids(BTreeSet::from([VANILLA_ITEM_STONE_ID]));
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
        set_main_hand(&mut world, VANILLA_ITEM_LILY_PAD_ID);
        assert_eq!(
            world.predict_local_item_use(InteractionHand::MainHand),
            LocalItemUsePrediction::UnsupportedOverride(
                VanillaItemUseOverride::PlaceOnWaterBlockItem
            )
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
    fn instrument_use_resolves_named_session_holder_and_applies_item_cooldown() {
        let mut world = WorldStore::new();
        world.record_registry_entries(
            crate::INSTRUMENT_REGISTRY,
            0,
            vec![
                crate::RegistryPacketEntry::stub("example:other_horn"),
                crate::RegistryPacketEntry::with_known_pack_data_for_test(
                    "example:network_horn",
                    br#"{
                        "sound_event":"minecraft:item.goat_horn.sound.0",
                        "use_duration":7.05,
                        "range":128.0,
                        "description":"Network horn"
                    }"#
                    .to_vec(),
                ),
            ],
        );
        world.apply_set_player_inventory(SetPlayerInventory {
            slot: 0,
            item: ItemStackSummary {
                item_id: Some(VANILLA_ITEM_GOAT_HORN_ID),
                count: 1,
                component_patch: DataComponentPatchSummary {
                    added: 1,
                    added_type_ids: vec![VANILLA_DATA_COMPONENT_INSTRUMENT_TYPE_ID],
                    instrument_id: Some(1),
                    ..DataComponentPatchSummary::default()
                },
            },
        });
        let instrument = world
            .local_instrument_use_state(
                world.local_item_in_hand(InteractionHand::MainHand).unwrap(),
            )
            .unwrap();
        assert_eq!(
            instrument.sound_event.registry_id,
            bbb_protocol::sound_event_types::vanilla_sound_event_registry_id(
                "minecraft:item.goat_horn.sound.0"
            )
        );
        assert_eq!(instrument.sound_event.sound_id, None);

        assert_eq!(
            world.predict_local_item_use(InteractionHand::MainHand),
            LocalItemUsePrediction::Result(LocalInteractionResult::Success(
                LocalInteractionSuccess::instrument_use(InteractionHand::MainHand)
            ))
        );
        assert_eq!(
            world
                .cooldown("minecraft:goat_horn")
                .map(|cooldown| (cooldown.duration, cooldown.remaining_ticks)),
            Some((141, 141))
        );
        assert_eq!(world.counters().cooldown_packets, 0);
        assert_eq!(
            world.predict_local_item_use(InteractionHand::MainHand),
            LocalItemUsePrediction::Result(LocalInteractionResult::Pass)
        );
    }

    #[test]
    fn instrument_use_resolves_direct_payload_sound_and_custom_cooldown_group() {
        let mut expected_seed = crate::SoundSeedRandomState::default();
        let expected_seed = expected_seed.next_long();
        let mut world = tool_use_world_with_login(
            block_state_id("minecraft:air", []),
            Some(("minecraft:overworld", VANILLA_GAME_TYPE_SURVIVAL_ID as i8)),
        );
        world.set_local_player_pose(crate::LocalPlayerPoseState {
            position: Vec3d {
                x: 1.25,
                y: 64.0,
                z: -3.5,
            },
            ..crate::LocalPlayerPoseState::default()
        });
        world.apply_set_player_inventory(SetPlayerInventory {
            slot: 0,
            item: direct_instrument_stack(3.25, 96.0, Some("example:horns")),
        });

        assert_eq!(
            world.predict_local_item_use(InteractionHand::MainHand),
            LocalItemUsePrediction::Result(LocalInteractionResult::Success(
                LocalInteractionSuccess::instrument_use(InteractionHand::MainHand)
            ))
        );
        assert_eq!(
            world
                .cooldown("example:horns")
                .map(|cooldown| cooldown.duration),
            Some(65)
        );
        let effect = world
            .apply_local_interaction_sound_action(
                LocalInteractionSoundAction::InstrumentUse {
                    hand: InteractionHand::MainHand,
                },
                &mut crate::LevelEventSoundRandomState::default(),
            )
            .unwrap();
        let crate::LocalInteractionSoundEffect::Entity { state, position } = effect else {
            panic!("instrument use must produce an entity-bound sound");
        };
        assert_eq!(state.sound.kind, "direct");
        assert_eq!(state.sound.location.as_deref(), Some("example:direct_horn"));
        assert_eq!(state.sound.fixed_range, Some(24.0));
        assert_eq!(state.source, "record");
        assert_eq!(state.entity_id, 7);
        assert_eq!(state.volume, 6.0);
        assert_eq!(state.pitch, 1.0);
        assert_eq!(state.seed, expected_seed);
        assert!(!state.looping);
        assert!(state.linear_attenuation);
        assert!(!state.tickable);
        assert_eq!(
            position,
            Some(Vec3d {
                x: 1.25,
                y: 64.0,
                z: -3.5,
            })
        );
        assert_eq!(world.last_sound_entity(), Some(&state));
    }

    #[test]
    fn instrument_use_without_effective_component_fails_and_invalid_payload_stays_explicit() {
        let mut world = WorldStore::new();
        set_main_hand(&mut world, VANILLA_ITEM_GOAT_HORN_ID);
        assert_eq!(
            world.predict_local_item_use(InteractionHand::MainHand),
            LocalItemUsePrediction::Result(LocalInteractionResult::Fail)
        );

        world.apply_set_player_inventory(SetPlayerInventory {
            slot: 0,
            item: ItemStackSummary {
                item_id: Some(VANILLA_ITEM_GOAT_HORN_ID),
                count: 1,
                component_patch: DataComponentPatchSummary {
                    added: 1,
                    added_type_ids: vec![VANILLA_DATA_COMPONENT_INSTRUMENT_TYPE_ID],
                    instrument_id: Some(0),
                    instrument_direct: direct_instrument_stack(1.0, 16.0, None)
                        .component_patch
                        .instrument_direct,
                    ..DataComponentPatchSummary::default()
                },
            },
        });
        assert_eq!(
            world.predict_local_item_use(InteractionHand::MainHand),
            LocalItemUsePrediction::MissingComponentValue(
                VANILLA_DATA_COMPONENT_INSTRUMENT_TYPE_ID
            )
        );
        assert!(world.cooldowns().is_empty());
    }

    #[test]
    fn block_use_prediction_preserves_vanilla_dispatch_order() {
        let mut world = WorldStore::new();
        let stone = vanilla_block_type(VANILLA_BLOCK_STONE_ID).unwrap();
        assert_eq!(
            world.predict_local_block_use_for_type(stone, InteractionHand::MainHand, None),
            LocalTargetUsePrediction::Result(LocalInteractionResult::Pass)
        );

        let chest = vanilla_block_type(VANILLA_BLOCK_CHEST_ID).unwrap();
        assert_eq!(
            world.predict_local_block_use_for_type(chest, InteractionHand::MainHand, None),
            LocalTargetUsePrediction::UnsupportedBlockUseWithoutItem(
                VanillaBlockUseWithoutItemOverride::ChestBlock
            )
        );
        assert_eq!(
            world.predict_local_block_use_for_type(chest, InteractionHand::OffHand, None),
            LocalTargetUsePrediction::Result(LocalInteractionResult::Pass)
        );

        let cake = vanilla_block_type(VANILLA_BLOCK_CAKE_ID).unwrap();
        assert_eq!(
            world.predict_local_block_use_for_type(cake, InteractionHand::MainHand, None),
            LocalTargetUsePrediction::UnsupportedBlockUseItemOn(
                VanillaBlockUseItemOnOverride::CakeBlock
            )
        );

        set_main_hand(&mut world, VANILLA_ITEM_STONE_ID);
        assert_eq!(
            world.predict_local_block_use_for_type(stone, InteractionHand::MainHand, None),
            LocalTargetUsePrediction::UnsupportedItemUseOn(VanillaItemUseOnOverride::BlockItem)
        );
        assert_eq!(
            world.predict_local_block_use(
                BlockPos { x: 32, y: 0, z: 32 },
                ProtocolDirection::Up,
                InteractionHand::MainHand,
                true,
                1,
            ),
            LocalTargetUsePrediction::UnsupportedItemUseOn(VanillaItemUseOnOverride::BlockItem)
        );
    }

    #[test]
    fn water_potion_mud_conversion_predicts_block_and_filled_result() {
        let mut world = tool_use_world(block_state_id("minecraft:air", []));
        install_convertable_to_mud_tag(&mut world);
        let clicked = BlockPos { x: 8, y: 8, z: 8 };
        let dirt = block_state_id("minecraft:dirt", []);
        let mud = block_state_id("minecraft:mud", []);
        set_block(&mut world, clicked, dirt);
        world.apply_set_player_inventory(SetPlayerInventory {
            slot: 0,
            item: potion_stack(0, 1),
        });

        assert_eq!(
            world.predict_local_block_use(
                clicked,
                ProtocolDirection::Up,
                InteractionHand::MainHand,
                false,
                31,
            ),
            LocalTargetUsePrediction::Result(LocalInteractionResult::Success(
                LocalInteractionSuccess::potion_mud_conversion(false)
            ))
        );
        assert_eq!(world.probe_block(clicked).unwrap().block_state_id, mud);
        let hand = player_inventory_item(&world, 0);
        assert_eq!(hand.item_id, Some(VANILLA_ITEM_GLASS_BOTTLE_ID));
        assert_eq!(hand.count, 1);
        assert_eq!(world.local_block_predictions().len(), 1);
        assert_eq!(world.local_block_predictions()[0].sequence, 31);
    }

    #[test]
    fn potion_mud_conversion_preserves_live_tag_holder_and_face_gates() {
        let clicked = BlockPos { x: 8, y: 8, z: 8 };
        let dirt = block_state_id("minecraft:dirt", []);
        let mut world = tool_use_world(block_state_id("minecraft:air", []));
        set_block(&mut world, clicked, dirt);
        world.apply_set_player_inventory(SetPlayerInventory {
            slot: 0,
            item: potion_stack(0, 1),
        });

        assert_eq!(
            world.predict_local_block_use(
                clicked,
                ProtocolDirection::Up,
                InteractionHand::MainHand,
                false,
                1,
            ),
            LocalTargetUsePrediction::UnsupportedItemUseOn(VanillaItemUseOnOverride::PotionItem)
        );

        install_convertable_to_mud_tag(&mut world);
        world.apply_set_player_inventory(SetPlayerInventory {
            slot: 0,
            item: potion_stack(1, 1),
        });
        assert_eq!(
            world.predict_local_block_use(
                clicked,
                ProtocolDirection::Up,
                InteractionHand::MainHand,
                false,
                2,
            ),
            LocalTargetUsePrediction::Result(LocalInteractionResult::Pass)
        );

        world.apply_set_player_inventory(SetPlayerInventory {
            slot: 0,
            item: potion_stack(0, 1),
        });
        assert_eq!(
            world.predict_local_block_use(
                clicked,
                ProtocolDirection::Down,
                InteractionHand::MainHand,
                false,
                3,
            ),
            LocalTargetUsePrediction::Result(LocalInteractionResult::Pass)
        );
        assert_eq!(world.probe_block(clicked).unwrap().block_state_id, dirt);
        assert_eq!(
            player_inventory_item(&world, 0).item_id,
            Some(VANILLA_ITEM_POTION_ID)
        );
        assert!(world.local_block_predictions().is_empty());
    }

    #[test]
    fn potion_filled_result_tracks_creative_and_full_inventory_branches() {
        let clicked = BlockPos { x: 8, y: 8, z: 8 };
        let dirt = block_state_id("minecraft:dirt", []);
        let mut creative = tool_use_world(block_state_id("minecraft:air", []));
        install_convertable_to_mud_tag(&mut creative);
        set_block(&mut creative, clicked, dirt);
        creative.apply_player_abilities(bbb_protocol::packets::PlayerAbilities {
            invulnerable: true,
            flying: false,
            can_fly: true,
            instabuild: true,
            flying_speed: 0.05,
            walking_speed: 0.1,
        });
        creative.apply_set_player_inventory(SetPlayerInventory {
            slot: 0,
            item: potion_stack(0, 1),
        });
        assert_eq!(
            creative.predict_local_block_use(
                clicked,
                ProtocolDirection::North,
                InteractionHand::MainHand,
                false,
                4,
            ),
            LocalTargetUsePrediction::Result(LocalInteractionResult::Success(
                LocalInteractionSuccess::potion_mud_conversion(false)
            ))
        );
        assert_eq!(
            player_inventory_item(&creative, 0).item_id,
            Some(VANILLA_ITEM_POTION_ID)
        );
        assert_eq!(
            player_inventory_item(&creative, 1).item_id,
            Some(VANILLA_ITEM_GLASS_BOTTLE_ID)
        );
        assert_eq!(creative.sync_ended_local_block_predictions(4), 1);
        assert_eq!(creative.probe_block(clicked).unwrap().block_state_id, dirt);
        assert_eq!(
            creative.predict_local_block_use(
                clicked,
                ProtocolDirection::North,
                InteractionHand::MainHand,
                false,
                5,
            ),
            LocalTargetUsePrediction::Result(LocalInteractionResult::Success(
                LocalInteractionSuccess::potion_mud_conversion(false)
            ))
        );
        assert_eq!(player_inventory_item(&creative, 1).count, 1);
        assert!(creative
            .inventory()
            .player_slots
            .iter()
            .all(|slot| slot.slot == 1 || slot.item.item_id != Some(VANILLA_ITEM_GLASS_BOTTLE_ID)));

        let mut stacked = tool_use_world(block_state_id("minecraft:air", []));
        install_convertable_to_mud_tag(&mut stacked);
        set_block(&mut stacked, clicked, dirt);
        stacked.apply_set_player_inventory(SetPlayerInventory {
            slot: 0,
            item: potion_stack(0, 2),
        });
        assert_eq!(
            stacked.predict_local_block_use(
                clicked,
                ProtocolDirection::North,
                InteractionHand::MainHand,
                false,
                6,
            ),
            LocalTargetUsePrediction::Result(LocalInteractionResult::Success(
                LocalInteractionSuccess::potion_mud_conversion(false)
            ))
        );
        assert_eq!(player_inventory_item(&stacked, 0).count, 1);
        assert_eq!(
            player_inventory_item(&stacked, 1).item_id,
            Some(VANILLA_ITEM_GLASS_BOTTLE_ID)
        );

        let mut full = tool_use_world(block_state_id("minecraft:air", []));
        install_convertable_to_mud_tag(&mut full);
        set_block(&mut full, clicked, dirt);
        for slot in 0..36 {
            full.apply_set_player_inventory(SetPlayerInventory {
                slot,
                item: stack(VANILLA_ITEM_STONE_ID),
            });
        }
        full.apply_set_player_inventory(SetPlayerInventory {
            slot: 0,
            item: potion_stack(0, 2),
        });
        assert_eq!(
            full.predict_local_block_use(
                clicked,
                ProtocolDirection::North,
                InteractionHand::MainHand,
                false,
                7,
            ),
            LocalTargetUsePrediction::Result(LocalInteractionResult::Success(
                LocalInteractionSuccess::potion_mud_conversion(true)
            ))
        );
        assert_eq!(player_inventory_item(&full, 0).count, 1);
        assert!(full
            .inventory()
            .player_slots
            .iter()
            .all(|slot| { slot.item.item_id != Some(VANILLA_ITEM_GLASS_BOTTLE_ID) }));
    }

    #[test]
    fn bone_meal_water_growth_predicts_client_result_without_local_mutation() {
        let mut world = tool_use_world(block_state_id("minecraft:air", []));
        set_main_hand(&mut world, VANILLA_ITEM_BONE_MEAL_ID);
        let clicked = BlockPos { x: 8, y: 8, z: 8 };
        let water = BlockPos { x: 9, y: 8, z: 8 };
        let above_clicked = BlockPos { x: 8, y: 9, z: 8 };
        let source_water = block_state_id("minecraft:water", [("level", "0")]);
        let stone = block_state_id("minecraft:stone", []);
        set_block(&mut world, clicked, stone);
        set_block(&mut world, water, source_water);

        assert_eq!(
            world.predict_local_block_use(
                clicked,
                ProtocolDirection::East,
                InteractionHand::MainHand,
                false,
                1,
            ),
            LocalTargetUsePrediction::Result(LocalInteractionResult::Success(
                LocalInteractionSuccess::client_item_interaction()
            ))
        );
        assert_eq!(world.probe_block(clicked).unwrap().block_state_id, stone);
        assert_eq!(
            world.probe_block(water).unwrap().block_state_id,
            source_water
        );
        assert_eq!(player_inventory_item(&world, 0).count, 1);

        let grass = block_state_id("minecraft:grass_block", [("snowy", "false")]);
        set_block(&mut world, clicked, grass);
        assert_eq!(
            world.predict_local_block_use(
                clicked,
                ProtocolDirection::East,
                InteractionHand::MainHand,
                false,
                2,
            ),
            LocalTargetUsePrediction::Result(LocalInteractionResult::Pass)
        );

        set_block(&mut world, above_clicked, stone);
        assert_eq!(
            world.predict_local_block_use(
                clicked,
                ProtocolDirection::East,
                InteractionHand::MainHand,
                false,
                3,
            ),
            LocalTargetUsePrediction::Result(LocalInteractionResult::Success(
                LocalInteractionSuccess::client_item_interaction()
            ))
        );

        set_block(
            &mut world,
            water,
            block_state_id("minecraft:water", [("level", "1")]),
        );
        assert_eq!(
            world.predict_local_block_use(
                clicked,
                ProtocolDirection::East,
                InteractionHand::MainHand,
                false,
                4,
            ),
            LocalTargetUsePrediction::Result(LocalInteractionResult::Pass)
        );
        assert_eq!(player_inventory_item(&world, 0).count, 1);
    }

    #[test]
    fn bone_meal_netherrack_target_uses_live_nylium_tag_and_skylight_rule() {
        let mut world = tool_use_world(block_state_id("minecraft:air", []));
        set_main_hand(&mut world, VANILLA_ITEM_BONE_MEAL_ID);
        world.apply_update_tags(UpdateTags {
            registries: vec![RegistryTags {
                registry: "minecraft:block".to_string(),
                tags: vec![TagNetworkPayload {
                    tag: "minecraft:nylium".to_string(),
                    entries: vec![VANILLA_BLOCK_CRIMSON_NYLIUM_ID],
                }],
            }],
        });
        let clicked = BlockPos { x: 8, y: 8, z: 8 };
        let water = BlockPos { x: 9, y: 8, z: 8 };
        let neighbour = BlockPos { x: 7, y: 8, z: 8 };
        let above_clicked = BlockPos { x: 8, y: 9, z: 8 };
        set_block(
            &mut world,
            clicked,
            block_state_id("minecraft:netherrack", []),
        );
        set_block(
            &mut world,
            water,
            block_state_id("minecraft:water", [("level", "0")]),
        );
        set_block(
            &mut world,
            neighbour,
            block_state_id("minecraft:crimson_nylium", []),
        );

        assert_eq!(
            world.predict_local_block_use(
                clicked,
                ProtocolDirection::East,
                InteractionHand::MainHand,
                false,
                1,
            ),
            LocalTargetUsePrediction::Result(LocalInteractionResult::Pass)
        );

        set_block(
            &mut world,
            above_clicked,
            block_state_id("minecraft:tinted_glass", []),
        );
        assert_eq!(
            world.predict_local_block_use(
                clicked,
                ProtocolDirection::East,
                InteractionHand::MainHand,
                false,
                2,
            ),
            LocalTargetUsePrediction::Result(LocalInteractionResult::Success(
                LocalInteractionSuccess::client_item_interaction()
            ))
        );

        set_block(
            &mut world,
            above_clicked,
            block_state_id("minecraft:glass", []),
        );
        assert_eq!(
            world.predict_local_block_use(
                clicked,
                ProtocolDirection::East,
                InteractionHand::MainHand,
                false,
                3,
            ),
            LocalTargetUsePrediction::Result(LocalInteractionResult::Pass)
        );
    }

    #[test]
    fn bone_meal_mossy_carpet_target_tracks_replaceability_and_side_support() {
        let mut world = tool_use_world(block_state_id("minecraft:air", []));
        set_main_hand(&mut world, VANILLA_ITEM_BONE_MEAL_ID);
        let clicked = BlockPos { x: 8, y: 8, z: 8 };
        let support = BlockPos { x: 8, y: 9, z: 7 };
        set_block(
            &mut world,
            clicked,
            block_state_id(
                "minecraft:pale_moss_carpet",
                [
                    ("bottom", "true"),
                    ("east", "none"),
                    ("north", "low"),
                    ("south", "none"),
                    ("west", "none"),
                ],
            ),
        );
        set_block(
            &mut world,
            BlockPos { x: 8, y: 7, z: 8 },
            block_state_id("minecraft:water", [("level", "0")]),
        );
        set_block(&mut world, support, block_state_id("minecraft:stone", []));

        assert_eq!(
            world.predict_local_block_use(
                clicked,
                ProtocolDirection::Down,
                InteractionHand::MainHand,
                false,
                1,
            ),
            LocalTargetUsePrediction::Result(LocalInteractionResult::Pass)
        );

        set_block(
            &mut world,
            BlockPos { x: 8, y: 9, z: 8 },
            block_state_id("minecraft:stone", []),
        );
        assert_eq!(
            world.predict_local_block_use(
                clicked,
                ProtocolDirection::Down,
                InteractionHand::MainHand,
                false,
                2,
            ),
            LocalTargetUsePrediction::Result(LocalInteractionResult::Success(
                LocalInteractionSuccess::client_item_interaction()
            ))
        );

        set_block(
            &mut world,
            BlockPos { x: 8, y: 9, z: 8 },
            block_state_id("minecraft:air", []),
        );
        set_block(&mut world, support, block_state_id("minecraft:air", []));
        assert_eq!(
            world.predict_local_block_use(
                clicked,
                ProtocolDirection::Down,
                InteractionHand::MainHand,
                false,
                3,
            ),
            LocalTargetUsePrediction::Result(LocalInteractionResult::Success(
                LocalInteractionSuccess::client_item_interaction()
            ))
        );
    }

    #[test]
    fn bone_meal_non_sturdy_and_missing_water_targets_preserve_pass_boundary() {
        let mut world = tool_use_world(block_state_id("minecraft:air", []));
        set_main_hand(&mut world, VANILLA_ITEM_BONE_MEAL_ID);
        let crop = BlockPos { x: 8, y: 8, z: 8 };
        set_block(
            &mut world,
            crop,
            block_state_id("minecraft:wheat", [("age", "0")]),
        );
        set_block(
            &mut world,
            BlockPos { x: 9, y: 8, z: 8 },
            block_state_id("minecraft:water", [("level", "0")]),
        );
        assert_eq!(
            world.predict_local_block_use(
                crop,
                ProtocolDirection::East,
                InteractionHand::MainHand,
                false,
                1,
            ),
            LocalTargetUsePrediction::Result(LocalInteractionResult::Pass)
        );

        let edge = BlockPos { x: 15, y: 8, z: 8 };
        set_block(&mut world, edge, block_state_id("minecraft:stone", []));
        assert_eq!(
            world.predict_local_block_use(
                edge,
                ProtocolDirection::East,
                InteractionHand::MainHand,
                false,
                2,
            ),
            LocalTargetUsePrediction::UnsupportedItemUseOn(VanillaItemUseOnOverride::BoneMealItem)
        );
    }

    #[test]
    fn bonemeal_generated_owners_with_sturdy_faces_are_evaluated() {
        let air = block_state_id("minecraft:air", []);
        let mut world = tool_use_world(air);
        let pos = BlockPos { x: 8, y: 8, z: 8 };
        let state_count = world.registries().block_state_count();
        let mut reached_evaluators = [false; 6];
        let mut missing_evaluators = Vec::new();
        for state_id in 0..state_count {
            let Some(state) = world
                .registries()
                .block_state(i32::try_from(state_id).unwrap())
                .cloned()
            else {
                continue;
            };
            let Some(block_type) = vanilla_block_type_for_resource_id(&state.name) else {
                continue;
            };
            let Some(owner) = block_type.bonemeal_target_override else {
                continue;
            };
            set_block(&mut world, pos, state.id);
            let probe = world.probe_block(pos).unwrap();
            if world
                .debug_block_sturdy_faces(&probe)
                .into_iter()
                .any(|face| face)
            {
                let evaluator = match owner {
                    VanillaBonemealTargetOverride::BonemealableFeaturePlacerBlock => 0,
                    VanillaBonemealTargetOverride::GrassBlock => 1,
                    VanillaBonemealTargetOverride::NetherrackBlock => 2,
                    VanillaBonemealTargetOverride::NyliumBlock => 3,
                    VanillaBonemealTargetOverride::RootedDirtBlock => 4,
                    VanillaBonemealTargetOverride::MossyCarpetBlock => 5,
                    VanillaBonemealTargetOverride::AzaleaBlock
                    | VanillaBonemealTargetOverride::MushroomBlock
                    | VanillaBonemealTargetOverride::SaplingBlock => continue,
                    _ => {
                        missing_evaluators
                            .push(format!("{} state {} ({owner:?})", state.name, state.id));
                        continue;
                    }
                };
                reached_evaluators[evaluator] = true;
            }
        }
        assert!(
            missing_evaluators.is_empty(),
            "sturdy bonemeal states without evaluators: {missing_evaluators:#?}"
        );
        assert_eq!(reached_evaluators, [true; 6]);
    }

    #[test]
    fn axe_use_on_predicts_blocking_guard_and_local_state_transforms() {
        let oak_log = block_state_id("minecraft:oak_log", [("axis", "x")]);
        let mut world = tool_use_world(oak_log);
        set_main_hand(&mut world, VANILLA_ITEM_IRON_AXE_ID);
        set_offhand(
            &mut world,
            ItemStackSummary {
                component_patch: DataComponentPatchSummary {
                    added_type_ids: vec![VANILLA_DATA_COMPONENT_BLOCKS_ATTACKS_TYPE_ID],
                    ..DataComponentPatchSummary::default()
                },
                ..stack(VANILLA_ITEM_SHIELD_ID)
            },
        );
        let pos = BlockPos { x: 0, y: 0, z: 0 };

        assert_eq!(
            world.predict_local_block_use(
                pos,
                ProtocolDirection::Up,
                InteractionHand::MainHand,
                false,
                1,
            ),
            LocalTargetUsePrediction::Result(LocalInteractionResult::Pass)
        );
        assert_eq!(world.probe_block(pos).unwrap().block_state_id, oak_log);

        assert_eq!(
            world.predict_local_block_use(
                pos,
                ProtocolDirection::Up,
                InteractionHand::MainHand,
                true,
                2,
            ),
            LocalTargetUsePrediction::Result(LocalInteractionResult::Success(
                LocalInteractionSuccess::client_item_interaction()
            ))
        );
        let stripped = world.probe_block(pos).unwrap();
        assert_eq!(
            stripped.block_name.as_deref(),
            Some("minecraft:stripped_oak_log")
        );
        assert_eq!(
            stripped.block_properties.get("axis").map(String::as_str),
            Some("x")
        );
        assert_eq!(world.local_block_predictions().len(), 1);
        assert_eq!(world.local_block_predictions()[0].sequence, 2);
        assert_eq!(
            world.local_block_predictions()[0].server_block_state_id,
            oak_log
        );
    }

    #[test]
    fn wax_and_shears_use_on_predict_client_mutations_and_inventory() {
        let copper_bulb = block_state_id(
            "minecraft:copper_bulb",
            [("lit", "true"), ("powered", "false")],
        );
        let mut world = tool_use_world(copper_bulb);
        world.apply_set_player_inventory(SetPlayerInventory {
            slot: 0,
            item: ItemStackSummary {
                count: 2,
                ..stack(VANILLA_ITEM_HONEYCOMB_ID)
            },
        });
        let pos = BlockPos { x: 0, y: 0, z: 0 };
        assert_eq!(
            world.predict_local_block_use(
                pos,
                ProtocolDirection::Up,
                InteractionHand::MainHand,
                true,
                3,
            ),
            LocalTargetUsePrediction::Result(LocalInteractionResult::Success(
                LocalInteractionSuccess::client_item_interaction()
            ))
        );
        let waxed = world.probe_block(pos).unwrap();
        assert_eq!(
            waxed.block_name.as_deref(),
            Some("minecraft:waxed_copper_bulb")
        );
        assert_eq!(
            waxed.block_properties,
            BTreeMap::from([
                ("lit".to_string(), "true".to_string()),
                ("powered".to_string(), "false".to_string()),
            ])
        );
        assert_eq!(
            world
                .local_item_in_hand(InteractionHand::MainHand)
                .unwrap()
                .count,
            1
        );

        let cave_vines =
            block_state_id("minecraft:cave_vines", [("age", "3"), ("berries", "true")]);
        let mut world = tool_use_world(cave_vines);
        set_main_hand(&mut world, VANILLA_ITEM_SHEARS_ID);
        assert_eq!(
            world.predict_local_block_use(
                pos,
                ProtocolDirection::North,
                InteractionHand::MainHand,
                true,
                4,
            ),
            LocalTargetUsePrediction::Result(LocalInteractionResult::Success(
                LocalInteractionSuccess::client_item_interaction()
            ))
        );
        let max_age = world.probe_block(pos).unwrap();
        assert_eq!(max_age.block_name.as_deref(), Some("minecraft:cave_vines"));
        assert_eq!(
            max_age.block_properties.get("age").map(String::as_str),
            Some("25")
        );
        assert_eq!(
            max_age.block_properties.get("berries").map(String::as_str),
            Some("true")
        );
        assert_eq!(
            world.predict_local_block_use(
                pos,
                ProtocolDirection::North,
                InteractionHand::MainHand,
                true,
                5,
            ),
            LocalTargetUsePrediction::Result(LocalInteractionResult::Pass)
        );

        let mut creative_world = tool_use_world(copper_bulb);
        creative_world.apply_game_event(bbb_protocol::packets::GameEvent {
            event_id: bbb_protocol::game_event_types::VANILLA_CLIENT_GAME_EVENT_CHANGE_GAME_MODE_ID,
            param: bbb_protocol::game_types::VANILLA_GAME_TYPE_CREATIVE_ID as f32,
        });
        creative_world.apply_player_abilities(bbb_protocol::packets::PlayerAbilities {
            invulnerable: true,
            flying: false,
            can_fly: true,
            instabuild: true,
            flying_speed: 0.05,
            walking_speed: 0.1,
        });
        creative_world.apply_set_player_inventory(SetPlayerInventory {
            slot: 0,
            item: ItemStackSummary {
                count: 2,
                ..stack(VANILLA_ITEM_HONEYCOMB_ID)
            },
        });
        assert_eq!(
            creative_world.predict_local_block_use(
                pos,
                ProtocolDirection::Up,
                InteractionHand::MainHand,
                true,
                6,
            ),
            LocalTargetUsePrediction::Result(LocalInteractionResult::Success(
                LocalInteractionSuccess::client_item_interaction()
            ))
        );
        assert_eq!(
            creative_world
                .local_item_in_hand(InteractionHand::MainHand)
                .unwrap()
                .count,
            2
        );

        let mut offhand_world = tool_use_world(copper_bulb);
        set_offhand(
            &mut offhand_world,
            ItemStackSummary {
                count: 2,
                ..stack(VANILLA_ITEM_HONEYCOMB_ID)
            },
        );
        assert_eq!(
            offhand_world.predict_local_block_use(
                pos,
                ProtocolDirection::Up,
                InteractionHand::OffHand,
                true,
                7,
            ),
            LocalTargetUsePrediction::Result(LocalInteractionResult::Success(
                LocalInteractionSuccess::client_item_interaction()
            ))
        );
        assert_eq!(
            offhand_world
                .local_item_in_hand(InteractionHand::OffHand)
                .unwrap()
                .count,
            1
        );
    }

    #[test]
    fn hoe_and_shovel_use_on_predict_source_owned_client_result_gates() {
        let grass = block_state_id("minecraft:grass_block", [("snowy", "false")]);
        let mut world = tool_use_world(grass);
        let pos = BlockPos { x: 0, y: 0, z: 0 };
        set_main_hand(&mut world, VANILLA_ITEM_IRON_HOE_ID);
        assert_eq!(
            world.predict_local_block_use(
                pos,
                ProtocolDirection::Up,
                InteractionHand::MainHand,
                false,
                6,
            ),
            LocalTargetUsePrediction::Result(LocalInteractionResult::Success(
                LocalInteractionSuccess::client_item_interaction()
            ))
        );
        assert_eq!(world.probe_block(pos).unwrap().block_state_id, grass);
        assert!(world.local_block_predictions().is_empty());
        assert_eq!(
            world.predict_local_block_use(
                pos,
                ProtocolDirection::Down,
                InteractionHand::MainHand,
                false,
                7,
            ),
            LocalTargetUsePrediction::Result(LocalInteractionResult::Pass)
        );

        set_main_hand(&mut world, VANILLA_ITEM_IRON_SHOVEL_ID);
        assert_eq!(
            world.predict_local_block_use(
                pos,
                ProtocolDirection::Up,
                InteractionHand::MainHand,
                false,
                8,
            ),
            LocalTargetUsePrediction::Result(LocalInteractionResult::Success(
                LocalInteractionSuccess::client_item_interaction()
            ))
        );
        assert_eq!(world.probe_block(pos).unwrap().block_state_id, grass);
        assert_eq!(
            world.predict_local_block_use(
                pos,
                ProtocolDirection::Down,
                InteractionHand::MainHand,
                false,
                9,
            ),
            LocalTargetUsePrediction::Result(LocalInteractionResult::Pass)
        );

        set_main_hand(&mut world, VANILLA_ITEM_IRON_AXE_ID);
        assert_eq!(
            world.predict_local_block_use(
                pos,
                ProtocolDirection::Up,
                InteractionHand::MainHand,
                false,
                10,
            ),
            LocalTargetUsePrediction::Result(LocalInteractionResult::Pass)
        );

        let rooted_dirt = block_state_id("minecraft:rooted_dirt", []);
        let mut rooted_world = tool_use_world(rooted_dirt);
        assert!(rooted_world.apply_block_update(BlockUpdate {
            pos: ProtocolBlockPos { x: 0, y: 1, z: 0 },
            block_state_id: block_state_id("minecraft:stone", []),
        }));
        set_main_hand(&mut rooted_world, VANILLA_ITEM_IRON_HOE_ID);
        assert_eq!(
            rooted_world.predict_local_block_use(
                pos,
                ProtocolDirection::Down,
                InteractionHand::MainHand,
                false,
                11,
            ),
            LocalTargetUsePrediction::Result(LocalInteractionResult::Success(
                LocalInteractionSuccess::client_item_interaction()
            ))
        );
        assert_eq!(
            rooted_world.probe_block(pos).unwrap().block_state_id,
            rooted_dirt
        );

        let campfire = block_state_id(
            "minecraft:campfire",
            [
                ("facing", "north"),
                ("lit", "true"),
                ("signal_fire", "false"),
                ("waterlogged", "false"),
            ],
        );
        let mut campfire_world = tool_use_world(campfire);
        set_main_hand(&mut campfire_world, VANILLA_ITEM_IRON_SHOVEL_ID);
        assert_eq!(
            campfire_world.predict_local_block_use(
                pos,
                ProtocolDirection::Up,
                InteractionHand::MainHand,
                true,
                12,
            ),
            LocalTargetUsePrediction::Result(LocalInteractionResult::Success(
                LocalInteractionSuccess::client_item_interaction()
            ))
        );
        assert_eq!(
            campfire_world.probe_block(pos).unwrap().block_state_id,
            campfire
        );
    }

    #[test]
    fn item_use_on_respects_adventure_can_place_on_before_tool_owner() {
        let oak_log = block_state_id("minecraft:oak_log", [("axis", "y")]);
        let mut world = tool_use_world(oak_log);
        let pos = BlockPos { x: 0, y: 0, z: 0 };
        world.apply_game_event(bbb_protocol::packets::GameEvent {
            event_id: bbb_protocol::game_event_types::VANILLA_CLIENT_GAME_EVENT_CHANGE_GAME_MODE_ID,
            param: bbb_protocol::game_types::VANILLA_GAME_TYPE_ADVENTURE_ID as f32,
        });
        set_main_hand(&mut world, VANILLA_ITEM_IRON_AXE_ID);
        assert_eq!(
            world.predict_local_block_use(
                pos,
                ProtocolDirection::Up,
                InteractionHand::MainHand,
                true,
                13,
            ),
            LocalTargetUsePrediction::Result(LocalInteractionResult::Pass)
        );
        assert_eq!(world.probe_block(pos).unwrap().block_state_id, oak_log);

        let oak_log_holder = world
            .registry_holder_id_by_resource_id("minecraft:block", "minecraft:oak_log")
            .unwrap();
        world.apply_set_player_inventory(SetPlayerInventory {
            slot: 0,
            item: ItemStackSummary {
                component_patch: DataComponentPatchSummary {
                    added_type_ids: vec![
                        bbb_protocol::data_component_types::
                            VANILLA_DATA_COMPONENT_CAN_PLACE_ON_TYPE_ID,
                    ],
                    can_place_on_persistent: Some(AdventureModePredicateDataSummary {
                        predicates: vec![BlockPredicateSummary {
                            blocks: Some(BlockHolderSetSummary {
                                registry_ids: vec![oak_log_holder],
                                tag: None,
                            }),
                            ..BlockPredicateSummary::default()
                        }],
                    }),
                    ..DataComponentPatchSummary::default()
                },
                ..stack(VANILLA_ITEM_IRON_AXE_ID)
            },
        });
        assert_eq!(
            world.predict_local_block_use(
                pos,
                ProtocolDirection::Up,
                InteractionHand::MainHand,
                true,
                14,
            ),
            LocalTargetUsePrediction::Result(LocalInteractionResult::Success(
                LocalInteractionSuccess::client_item_interaction()
            ))
        );
        assert_eq!(
            world.probe_block(pos).unwrap().block_name.as_deref(),
            Some("minecraft:stripped_oak_log")
        );
    }

    #[test]
    fn spawn_egg_and_firework_use_on_predict_client_short_circuits() {
        let pos = BlockPos { x: 0, y: 0, z: 0 };
        let mut world = tool_use_world(block_state_id("minecraft:stone", []));
        let client_success = LocalTargetUsePrediction::Result(LocalInteractionResult::Success(
            LocalInteractionSuccess::client_item_interaction(),
        ));

        set_main_hand(&mut world, VANILLA_ITEM_CHICKEN_SPAWN_EGG_ID);
        assert_eq!(
            world.predict_local_block_use(
                pos,
                ProtocolDirection::Up,
                InteractionHand::MainHand,
                false,
                1,
            ),
            client_success
        );
        assert_eq!(
            world
                .local_item_in_hand(InteractionHand::MainHand)
                .unwrap()
                .count,
            1
        );

        set_main_hand(&mut world, VANILLA_ITEM_FIREWORK_ROCKET_ID);
        assert_eq!(
            world.predict_local_block_use(
                pos,
                ProtocolDirection::Up,
                InteractionHand::MainHand,
                false,
                2,
            ),
            client_success
        );
        add_entity(&mut world, 7, VANILLA_ENTITY_TYPE_PLAYER_ID);
        world.local_player_id = Some(7);
        let accessors = crate::entities::vanilla_entity_metadata_entity_accessors().unwrap();
        assert!(world.apply_set_entity_data(SetEntityData {
            id: 7,
            values: vec![EntityDataValue {
                data_id: accessors.shared_flags_data_id,
                serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_BYTE_ID,
                value: EntityDataValueKind::Byte(
                    accessors.shared_flags_default | accessors.fall_flying_mask,
                ),
            }],
        }));
        assert_eq!(
            world.predict_local_block_use(
                pos,
                ProtocolDirection::Up,
                InteractionHand::MainHand,
                false,
                3,
            ),
            LocalTargetUsePrediction::Result(LocalInteractionResult::Pass)
        );
    }

    #[test]
    fn client_owned_item_use_on_predicts_fixed_and_block_gated_results() {
        let pos = BlockPos { x: 0, y: 0, z: 0 };
        let client_success = LocalTargetUsePrediction::Result(LocalInteractionResult::Success(
            LocalInteractionSuccess::client_item_interaction(),
        ));
        let consume = LocalTargetUsePrediction::Result(LocalInteractionResult::Success(
            LocalInteractionSuccess::consume(true),
        ));
        let pass = LocalTargetUsePrediction::Result(LocalInteractionResult::Pass);
        let mut world = tool_use_world(block_state_id("minecraft:stone", []));

        for (item_id, expected) in [
            (VANILLA_ITEM_BRUSH_ID, consume),
            (VANILLA_ITEM_DEBUG_STICK_ID, client_success),
            (VANILLA_ITEM_LEAD_ID, pass),
            (VANILLA_ITEM_LILY_PAD_ID, pass),
        ] {
            set_main_hand(&mut world, item_id);
            assert_eq!(
                world.predict_local_block_use(
                    pos,
                    ProtocolDirection::Up,
                    InteractionHand::MainHand,
                    false,
                    1,
                ),
                expected
            );
        }

        let mut frame_world = tool_use_world(block_state_id(
            "minecraft:end_portal_frame",
            [("eye", "false"), ("facing", "north")],
        ));
        set_main_hand(&mut frame_world, VANILLA_ITEM_ENDER_EYE_ID);
        assert_eq!(
            frame_world.predict_local_block_use(
                pos,
                ProtocolDirection::Up,
                InteractionHand::MainHand,
                false,
                2,
            ),
            client_success
        );
        assert_eq!(
            frame_world
                .local_item_in_hand(InteractionHand::MainHand)
                .unwrap()
                .count,
            1
        );
        assert!(frame_world.apply_block_update(BlockUpdate {
            pos: ProtocolBlockPos { x: 0, y: 0, z: 0 },
            block_state_id: block_state_id(
                "minecraft:end_portal_frame",
                [("eye", "true"), ("facing", "north")],
            ),
        }));
        assert_eq!(
            frame_world.predict_local_block_use(
                pos,
                ProtocolDirection::Up,
                InteractionHand::MainHand,
                false,
                3,
            ),
            pass
        );

        let mut map_world = tool_use_world(block_state_id(
            "minecraft:white_banner",
            [("rotation", "0")],
        ));
        set_main_hand(&mut map_world, VANILLA_ITEM_FILLED_MAP_ID);
        assert_eq!(
            map_world.predict_local_block_use(
                pos,
                ProtocolDirection::Up,
                InteractionHand::MainHand,
                false,
                4,
            ),
            LocalTargetUsePrediction::UnsupportedItemUseOn(VanillaItemUseOnOverride::MapItem)
        );
        map_world.apply_update_tags(UpdateTags {
            registries: vec![RegistryTags {
                registry: "minecraft:block".to_string(),
                tags: vec![TagNetworkPayload {
                    tag: "minecraft:banners".to_string(),
                    entries: vec![VANILLA_BLOCK_WHITE_BANNER_ID],
                }],
            }],
        });
        assert_eq!(
            map_world.predict_local_block_use(
                pos,
                ProtocolDirection::Up,
                InteractionHand::MainHand,
                false,
                5,
            ),
            client_success
        );
        assert!(map_world.apply_block_update(BlockUpdate {
            pos: ProtocolBlockPos { x: 0, y: 0, z: 0 },
            block_state_id: block_state_id("minecraft:stone", []),
        }));
        assert_eq!(
            map_world.predict_local_block_use(
                pos,
                ProtocolDirection::Up,
                InteractionHand::MainHand,
                false,
                6,
            ),
            pass
        );
    }

    #[test]
    fn lodestone_compass_use_on_predicts_inventory_and_drop_edges_without_local_sound() {
        let pos = BlockPos { x: 0, y: 0, z: 0 };
        let lodestone = block_state_id("minecraft:lodestone", []);
        let expected_success = |dropped| {
            LocalTargetUsePrediction::Result(LocalInteractionResult::Success(
                LocalInteractionSuccess::lodestone_compass_lock(dropped),
            ))
        };

        let mut singleton = tool_use_world_with_login(
            lodestone,
            Some(("example:dimension", VANILLA_GAME_TYPE_SURVIVAL_ID as i8)),
        );
        let mut removed_tracker = stack(VANILLA_ITEM_COMPASS_ID);
        removed_tracker
            .component_patch
            .removed_type_ids
            .push(VANILLA_DATA_COMPONENT_LODESTONE_TRACKER_TYPE_ID);
        singleton.apply_set_player_inventory(SetPlayerInventory {
            slot: 0,
            item: removed_tracker,
        });
        let result = singleton.predict_local_block_use(
            pos,
            ProtocolDirection::Up,
            InteractionHand::MainHand,
            false,
            1,
        );
        assert_eq!(result, expected_success(false));
        let locked = player_inventory_item(&singleton, 0);
        assert_eq!(locked.count, 1);
        assert!(locked
            .component_patch
            .added_type_ids
            .contains(&VANILLA_DATA_COMPONENT_LODESTONE_TRACKER_TYPE_ID));
        assert!(!locked
            .component_patch
            .removed_type_ids
            .contains(&VANILLA_DATA_COMPONENT_LODESTONE_TRACKER_TYPE_ID));
        assert_eq!(
            locked.component_patch.lodestone_target,
            Some(LodestoneTargetSummary {
                dimension: "example:dimension".to_string(),
                pos: ProtocolBlockPos { x: 0, y: 0, z: 0 },
            })
        );
        assert_eq!(locked.component_patch.lodestone_tracked, Some(true));
        assert!(
            singleton.client_audio().last_sound.is_none(),
            "CompassItem passes null as the excluded player, so ClientLevel does not play locally"
        );

        assert!(singleton.apply_block_update(BlockUpdate {
            pos: ProtocolBlockPos { x: 0, y: 0, z: 0 },
            block_state_id: block_state_id("minecraft:stone", []),
        }));
        assert_eq!(
            singleton.predict_local_block_use(
                pos,
                ProtocolDirection::Up,
                InteractionHand::MainHand,
                false,
                2,
            ),
            LocalTargetUsePrediction::Result(LocalInteractionResult::Pass)
        );

        let mut stacked = tool_use_world_with_login(
            lodestone,
            Some(("minecraft:overworld", VANILLA_GAME_TYPE_SURVIVAL_ID as i8)),
        );
        stacked.apply_set_player_inventory(SetPlayerInventory {
            slot: 0,
            item: ItemStackSummary {
                count: 2,
                ..stack(VANILLA_ITEM_COMPASS_ID)
            },
        });
        assert_eq!(
            stacked.predict_local_block_use(
                pos,
                ProtocolDirection::Up,
                InteractionHand::MainHand,
                false,
                3,
            ),
            expected_success(false)
        );
        assert_eq!(player_inventory_item(&stacked, 0).count, 1);
        assert!(!player_inventory_item(&stacked, 0)
            .component_patch
            .added_type_ids
            .contains(&VANILLA_DATA_COMPONENT_LODESTONE_TRACKER_TYPE_ID));
        assert_eq!(player_inventory_item(&stacked, 1).count, 1);
        assert!(player_inventory_item(&stacked, 1)
            .component_patch
            .added_type_ids
            .contains(&VANILLA_DATA_COMPONENT_LODESTONE_TRACKER_TYPE_ID));

        let mut creative = tool_use_world_with_login(
            lodestone,
            Some(("minecraft:overworld", VANILLA_GAME_TYPE_CREATIVE_ID as i8)),
        );
        creative.apply_set_player_inventory(SetPlayerInventory {
            slot: 0,
            item: ItemStackSummary {
                count: 2,
                ..stack(VANILLA_ITEM_COMPASS_ID)
            },
        });
        assert_eq!(
            creative.predict_local_block_use(
                pos,
                ProtocolDirection::Up,
                InteractionHand::MainHand,
                false,
                4,
            ),
            expected_success(false)
        );
        assert_eq!(player_inventory_item(&creative, 0).count, 2);
        assert_eq!(player_inventory_item(&creative, 1).count, 1);
        assert!(player_inventory_item(&creative, 1)
            .component_patch
            .added_type_ids
            .contains(&VANILLA_DATA_COMPONENT_LODESTONE_TRACKER_TYPE_ID));

        let mut full = tool_use_world_with_login(
            lodestone,
            Some(("minecraft:overworld", VANILLA_GAME_TYPE_SURVIVAL_ID as i8)),
        );
        for slot in 0..36 {
            full.apply_set_player_inventory(SetPlayerInventory {
                slot,
                item: stack(VANILLA_ITEM_STONE_ID),
            });
        }
        full.apply_set_player_inventory(SetPlayerInventory {
            slot: 0,
            item: ItemStackSummary {
                count: 2,
                ..stack(VANILLA_ITEM_COMPASS_ID)
            },
        });
        assert_eq!(
            full.predict_local_block_use(
                pos,
                ProtocolDirection::Up,
                InteractionHand::MainHand,
                false,
                5,
            ),
            expected_success(true)
        );
        assert_eq!(player_inventory_item(&full, 0).count, 1);
        assert!(full.inventory().player_slots.iter().all(|slot| {
            !slot
                .item
                .component_patch
                .added_type_ids
                .contains(&VANILLA_DATA_COMPONENT_LODESTONE_TRACKER_TYPE_ID)
        }));
    }

    #[test]
    fn ignition_items_light_blocks_with_exact_client_mutation_and_consumption() {
        let pos = BlockPos { x: 0, y: 0, z: 0 };
        let unlit_campfire = block_state_id(
            "minecraft:campfire",
            [
                ("facing", "north"),
                ("lit", "false"),
                ("signal_fire", "false"),
                ("waterlogged", "false"),
            ],
        );
        let lit_campfire = block_state_id(
            "minecraft:campfire",
            [
                ("facing", "north"),
                ("lit", "true"),
                ("signal_fire", "false"),
                ("waterlogged", "false"),
            ],
        );
        let mut flint = tool_use_world(unlit_campfire);
        set_main_hand(&mut flint, VANILLA_ITEM_FLINT_AND_STEEL_ID);
        assert_eq!(
            flint.predict_local_block_use(
                pos,
                ProtocolDirection::Up,
                InteractionHand::MainHand,
                false,
                1,
            ),
            LocalTargetUsePrediction::Result(LocalInteractionResult::Success(
                LocalInteractionSuccess::ignition(LocalInteractionSoundAction::FlintAndSteelUse {
                    pos
                })
            ))
        );
        assert_eq!(flint.probe_block(pos).unwrap().block_state_id, lit_campfire);
        assert_eq!(player_inventory_item(&flint, 0).count, 1);

        let mut charge = tool_use_world(unlit_campfire);
        charge.apply_set_player_inventory(SetPlayerInventory {
            slot: 0,
            item: ItemStackSummary {
                count: 2,
                ..stack(VANILLA_ITEM_FIRE_CHARGE_ID)
            },
        });
        assert_eq!(
            charge.predict_local_block_use(
                pos,
                ProtocolDirection::Up,
                InteractionHand::MainHand,
                false,
                2,
            ),
            LocalTargetUsePrediction::Result(LocalInteractionResult::Success(
                LocalInteractionSuccess::ignition(
                    LocalInteractionSoundAction::SuppressedFireChargeUse
                )
            ))
        );
        assert_eq!(
            charge.probe_block(pos).unwrap().block_state_id,
            lit_campfire
        );
        assert_eq!(player_inventory_item(&charge, 0).count, 1);

        let mut creative = tool_use_world_with_login(
            unlit_campfire,
            Some(("minecraft:overworld", VANILLA_GAME_TYPE_CREATIVE_ID as i8)),
        );
        creative.apply_set_player_inventory(SetPlayerInventory {
            slot: 0,
            item: ItemStackSummary {
                count: 2,
                ..stack(VANILLA_ITEM_FIRE_CHARGE_ID)
            },
        });
        assert!(matches!(
            creative.predict_local_block_use(
                pos,
                ProtocolDirection::Up,
                InteractionHand::MainHand,
                false,
                3,
            ),
            LocalTargetUsePrediction::Result(LocalInteractionResult::Success(_))
        ));
        assert_eq!(player_inventory_item(&creative, 0).count, 2);

        let lightable_states = [
            (
                block_state_id(
                    "minecraft:candle",
                    [("candles", "1"), ("lit", "false"), ("waterlogged", "false")],
                ),
                block_state_id(
                    "minecraft:candle",
                    [("candles", "1"), ("lit", "true"), ("waterlogged", "false")],
                ),
            ),
            (
                block_state_id("minecraft:candle_cake", [("lit", "false")]),
                block_state_id("minecraft:candle_cake", [("lit", "true")]),
            ),
        ];
        for (unlit, lit) in lightable_states {
            let mut world = tool_use_world(unlit);
            set_main_hand(&mut world, VANILLA_ITEM_FLINT_AND_STEEL_ID);
            assert!(matches!(
                world.predict_local_block_use(
                    pos,
                    ProtocolDirection::Up,
                    InteractionHand::MainHand,
                    false,
                    4,
                ),
                LocalTargetUsePrediction::Result(LocalInteractionResult::Success(_))
            ));
            assert_eq!(world.probe_block(pos).unwrap().block_state_id, lit);
        }
    }

    #[test]
    fn ignition_items_predict_ordinary_and_soul_fire_or_fail_closed() {
        let clicked = BlockPos { x: 8, y: 0, z: 8 };
        let fire_pos = BlockPos { x: 8, y: 1, z: 8 };
        let mut ordinary = tool_use_world_with_login(
            block_state_id("minecraft:air", []),
            Some(("minecraft:the_end", VANILLA_GAME_TYPE_SURVIVAL_ID as i8)),
        );
        set_block(
            &mut ordinary,
            clicked,
            block_state_id("minecraft:oak_planks", []),
        );
        set_main_hand(&mut ordinary, VANILLA_ITEM_FLINT_AND_STEEL_ID);
        assert_eq!(
            ordinary.predict_local_block_use(
                clicked,
                ProtocolDirection::Up,
                InteractionHand::MainHand,
                false,
                3,
            ),
            LocalTargetUsePrediction::Result(LocalInteractionResult::Success(
                LocalInteractionSuccess::ignition(LocalInteractionSoundAction::FlintAndSteelUse {
                    pos: fire_pos
                })
            ))
        );
        assert_eq!(
            ordinary.probe_block(fire_pos).unwrap().block_state_id,
            block_state_id(
                "minecraft:fire",
                [
                    ("age", "0"),
                    ("east", "false"),
                    ("north", "false"),
                    ("south", "false"),
                    ("up", "false"),
                    ("west", "false"),
                ],
            )
        );

        let mut soul = tool_use_world_with_login(
            block_state_id("minecraft:air", []),
            Some(("minecraft:the_end", VANILLA_GAME_TYPE_SURVIVAL_ID as i8)),
        );
        set_block(
            &mut soul,
            clicked,
            block_state_id("minecraft:soul_sand", []),
        );
        set_main_hand(&mut soul, VANILLA_ITEM_FLINT_AND_STEEL_ID);
        assert!(matches!(
            soul.predict_local_block_use(
                clicked,
                ProtocolDirection::Up,
                InteractionHand::MainHand,
                false,
                4,
            ),
            LocalTargetUsePrediction::Result(LocalInteractionResult::Success(_))
        ));
        assert_eq!(
            soul.probe_block(fire_pos).unwrap().block_state_id,
            block_state_id("minecraft:soul_fire", [])
        );

        let mut missing_level = tool_use_world(block_state_id("minecraft:air", []));
        set_block(
            &mut missing_level,
            clicked,
            block_state_id("minecraft:stone", []),
        );
        set_main_hand(&mut missing_level, VANILLA_ITEM_FLINT_AND_STEEL_ID);
        assert_eq!(
            missing_level.predict_local_block_use(
                clicked,
                ProtocolDirection::East,
                InteractionHand::MainHand,
                false,
                5,
            ),
            LocalTargetUsePrediction::UnsupportedItemUseOn(
                VanillaItemUseOnOverride::FlintAndSteelItem
            ),
            "without level identity the client must not guess portal eligibility"
        );

        let mut fail = tool_use_world_with_login(
            block_state_id("minecraft:air", []),
            Some(("minecraft:the_end", VANILLA_GAME_TYPE_SURVIVAL_ID as i8)),
        );
        set_block(&mut fail, clicked, block_state_id("minecraft:stone", []));
        set_main_hand(&mut fail, VANILLA_ITEM_FLINT_AND_STEEL_ID);
        assert_eq!(
            fail.predict_local_block_use(
                clicked,
                ProtocolDirection::East,
                InteractionHand::MainHand,
                false,
                6,
            ),
            LocalTargetUsePrediction::Result(LocalInteractionResult::Fail)
        );
    }

    #[test]
    fn ignition_creates_empty_obsidian_portal_with_vanilla_axis_priority() {
        let air = block_state_id("minecraft:air", []);
        let obsidian = block_state_id("minecraft:obsidian", []);
        let mut world = tool_use_world_with_login(
            air,
            Some(("minecraft:overworld", VANILLA_GAME_TYPE_SURVIVAL_ID as i8)),
        );
        // Axis.X uses WEST as PortalShape.rightDir. The two-wide interior is
        // x=7,6 with sides at x=8,5 and a three-block interior height.
        for x in 5..=8 {
            set_block(&mut world, BlockPos { x, y: 0, z: 8 }, obsidian);
            set_block(&mut world, BlockPos { x, y: 4, z: 8 }, obsidian);
        }
        for y in 1..=3 {
            set_block(&mut world, BlockPos { x: 5, y, z: 8 }, obsidian);
            set_block(&mut world, BlockPos { x: 8, y, z: 8 }, obsidian);
        }
        world.apply_set_player_inventory(SetPlayerInventory {
            slot: 0,
            item: ItemStackSummary {
                count: 2,
                ..stack(VANILLA_ITEM_FIRE_CHARGE_ID)
            },
        });
        assert_eq!(
            world.predict_local_block_use(
                BlockPos { x: 7, y: 0, z: 8 },
                ProtocolDirection::Up,
                InteractionHand::MainHand,
                false,
                7,
            ),
            LocalTargetUsePrediction::Result(LocalInteractionResult::Success(
                LocalInteractionSuccess::ignition(
                    LocalInteractionSoundAction::SuppressedFireChargeUse
                )
            ))
        );
        let portal = block_state_id("minecraft:nether_portal", [("axis", "x")]);
        for x in [6, 7] {
            for y in 1..=3 {
                assert_eq!(
                    world
                        .probe_block(BlockPos { x, y, z: 8 })
                        .unwrap()
                        .block_state_id,
                    portal,
                    "missing portal prediction at x={x}, y={y}"
                );
            }
        }
        assert_eq!(player_inventory_item(&world, 0).count, 1);
        assert_eq!(world.local_block_predictions().len(), 6);
    }

    #[test]
    fn end_crystal_and_minecart_use_on_predict_client_gates_and_shrink() {
        let pos = BlockPos { x: 0, y: 0, z: 0 };
        let client_success = LocalTargetUsePrediction::Result(LocalInteractionResult::Success(
            LocalInteractionSuccess::client_item_interaction(),
        ));
        let fail = LocalTargetUsePrediction::Result(LocalInteractionResult::Fail);

        let mut crystal_world = tool_use_world(block_state_id("minecraft:obsidian", []));
        crystal_world.apply_set_player_inventory(SetPlayerInventory {
            slot: 0,
            item: ItemStackSummary {
                count: 2,
                ..stack(VANILLA_ITEM_END_CRYSTAL_ID)
            },
        });
        assert_eq!(
            crystal_world.predict_local_block_use(
                pos,
                ProtocolDirection::Up,
                InteractionHand::MainHand,
                false,
                1,
            ),
            client_success
        );
        assert_eq!(
            crystal_world
                .local_item_in_hand(InteractionHand::MainHand)
                .unwrap()
                .count,
            1
        );
        assert!(crystal_world.apply_block_update(BlockUpdate {
            pos: ProtocolBlockPos { x: 0, y: 1, z: 0 },
            block_state_id: block_state_id("minecraft:stone", []),
        }));
        assert_eq!(
            crystal_world.predict_local_block_use(
                pos,
                ProtocolDirection::Up,
                InteractionHand::MainHand,
                false,
                2,
            ),
            fail
        );

        let mut occupied_crystal_world = tool_use_world(block_state_id("minecraft:obsidian", []));
        add_entity_at(
            &mut occupied_crystal_world,
            8,
            VANILLA_ENTITY_TYPE_AREA_EFFECT_CLOUD_ID,
            Vec3d {
                x: 0.5,
                y: 1.0,
                z: 0.5,
            },
        );
        set_main_hand(&mut occupied_crystal_world, VANILLA_ITEM_END_CRYSTAL_ID);
        assert_eq!(
            occupied_crystal_world.predict_local_block_use(
                pos,
                ProtocolDirection::Up,
                InteractionHand::MainHand,
                false,
                3,
            ),
            fail
        );
        let stone = block_state_id("minecraft:stone", []);
        assert!(occupied_crystal_world.apply_block_update(BlockUpdate {
            pos: ProtocolBlockPos { x: 0, y: 0, z: 0 },
            block_state_id: stone,
        }));
        assert_eq!(
            occupied_crystal_world.predict_local_block_use(
                pos,
                ProtocolDirection::Up,
                InteractionHand::MainHand,
                false,
                4,
            ),
            fail
        );

        let ascending_rail = block_state_id(
            "minecraft:rail",
            [("shape", "ascending_east"), ("waterlogged", "false")],
        );
        let mut minecart_world = tool_use_world(ascending_rail);
        minecart_world.apply_update_tags(UpdateTags {
            registries: vec![RegistryTags {
                registry: "minecraft:block".to_string(),
                tags: vec![TagNetworkPayload {
                    tag: "minecraft:rails".to_string(),
                    entries: vec![VANILLA_BLOCK_RAIL_ID],
                }],
            }],
        });
        minecart_world.apply_set_player_inventory(SetPlayerInventory {
            slot: 0,
            item: ItemStackSummary {
                count: 2,
                ..stack(VANILLA_ITEM_MINECART_ID)
            },
        });
        assert_eq!(
            minecart_world.predict_local_block_use(
                pos,
                ProtocolDirection::Up,
                InteractionHand::MainHand,
                false,
                5,
            ),
            client_success
        );
        assert_eq!(
            minecart_world
                .local_item_in_hand(InteractionHand::MainHand)
                .unwrap()
                .count,
            1
        );

        minecart_world.apply_update_enabled_features(UpdateEnabledFeatures {
            features: vec![
                "minecraft:vanilla".to_string(),
                "minecraft:minecart_improvements".to_string(),
            ],
        });
        add_entity_at(
            &mut minecart_world,
            9,
            VANILLA_ENTITY_TYPE_AREA_EFFECT_CLOUD_ID,
            Vec3d {
                x: 0.5,
                y: 1.0,
                z: 0.5,
            },
        );
        assert_eq!(
            minecart_world.predict_local_block_use(
                pos,
                ProtocolDirection::Up,
                InteractionHand::MainHand,
                false,
                6,
            ),
            client_success
        );
        set_main_hand(&mut minecart_world, VANILLA_ITEM_MINECART_ID);
        add_entity_at(
            &mut minecart_world,
            10,
            VANILLA_ENTITY_TYPE_MINECART_ID,
            Vec3d {
                x: 0.5,
                y: 1.0,
                z: 0.5,
            },
        );
        assert_eq!(
            minecart_world.predict_local_block_use(
                pos,
                ProtocolDirection::Up,
                InteractionHand::MainHand,
                false,
                7,
            ),
            fail
        );
        assert!(minecart_world.apply_block_update(BlockUpdate {
            pos: ProtocolBlockPos { x: 0, y: 0, z: 0 },
            block_state_id: stone,
        }));
        assert_eq!(
            minecart_world.predict_local_block_use(
                pos,
                ProtocolDirection::Up,
                InteractionHand::MainHand,
                false,
                8,
            ),
            fail
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
            LocalTargetUsePrediction::Result(LocalInteractionResult::Pass)
        );
        set_main_hand(&mut world, VANILLA_ITEM_RED_DYE_ID);
        assert_eq!(
            world.predict_local_entity_interaction(3, InteractionHand::MainHand, false),
            LocalTargetUsePrediction::Result(LocalInteractionResult::Pass)
        );
    }

    #[test]
    fn item_stack_interact_living_entity_predicts_dye_name_tag_and_equippable_boundaries() {
        let mut world = WorldStore::new();
        add_entity(&mut world, 4, VANILLA_ENTITY_TYPE_SHEEP_ID);
        world.apply_update_tags(UpdateTags {
            registries: vec![RegistryTags {
                registry: "minecraft:item".to_string(),
                tags: vec![TagNetworkPayload {
                    tag: VANILLA_SHEEP_FOOD_TAG.to_string(),
                    entries: Vec::new(),
                }],
            }],
        });
        world.set_default_item_component_type_ids(BTreeMap::from([(
            VANILLA_ITEM_RED_DYE_ID,
            BTreeSet::from([VANILLA_DATA_COMPONENT_DYE_TYPE_ID]),
        )]));

        set_main_hand(&mut world, VANILLA_ITEM_RED_DYE_ID);
        assert_eq!(
            world.predict_local_entity_interaction(4, InteractionHand::MainHand, false),
            LocalTargetUsePrediction::UnsupportedItemInteractLivingEntity(
                VanillaItemInteractLivingEntityOverride::DyeItem
            )
        );
        world.set_default_item_dye_color_ids(BTreeMap::from([(
            VANILLA_ITEM_RED_DYE_ID,
            VANILLA_DYE_COLOR_RED_ID,
        )]));

        let pass = LocalTargetUsePrediction::Result(LocalInteractionResult::Pass);
        let client_success = LocalTargetUsePrediction::Result(LocalInteractionResult::Success(
            LocalInteractionSuccess::client_item_interaction(),
        ));
        assert_eq!(
            world.predict_local_entity_interaction(4, InteractionHand::MainHand, false),
            client_success
        );
        assert_eq!(
            world
                .local_item_in_hand(InteractionHand::MainHand)
                .unwrap()
                .count,
            1
        );
        world.apply_update_tags(UpdateTags {
            registries: vec![RegistryTags {
                registry: "minecraft:item".to_string(),
                tags: vec![TagNetworkPayload {
                    tag: VANILLA_SHEEP_FOOD_TAG.to_string(),
                    entries: vec![VANILLA_ITEM_RED_DYE_ID],
                }],
            }],
        });
        assert_eq!(
            world.predict_local_entity_interaction(4, InteractionHand::MainHand, false),
            LocalTargetUsePrediction::UnsupportedEntityInteract(VANILLA_SHEEP_MOB_INTERACT_OWNER)
        );
        world.apply_update_tags(UpdateTags {
            registries: vec![RegistryTags {
                registry: "minecraft:item".to_string(),
                tags: vec![TagNetworkPayload {
                    tag: VANILLA_SHEEP_FOOD_TAG.to_string(),
                    entries: Vec::new(),
                }],
            }],
        });

        let (wool_data_id, wool_default, sheared_mask, _) =
            vanilla_entity_metadata_sheep_wool_accessor_for_entity_type(
                VANILLA_ENTITY_TYPE_SHEEP_ID,
            )
            .unwrap();
        assert!(world.apply_set_entity_data(SetEntityData {
            id: 4,
            values: vec![EntityDataValue {
                data_id: wool_data_id,
                serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_BYTE_ID,
                value: EntityDataValueKind::Byte(VANILLA_DYE_COLOR_RED_ID as i8),
            }],
        }));
        assert_eq!(
            world.predict_local_entity_interaction(4, InteractionHand::MainHand, false),
            pass
        );
        assert!(world.apply_set_entity_data(SetEntityData {
            id: 4,
            values: vec![EntityDataValue {
                data_id: wool_data_id,
                serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_BYTE_ID,
                value: EntityDataValueKind::Byte(wool_default | sheared_mask),
            }],
        }));
        assert_eq!(
            world.predict_local_entity_interaction(4, InteractionHand::MainHand, false),
            pass
        );

        let living_accessors =
            crate::entities::vanilla_entity_metadata_living_entity_accessors_for_entity_type(
                VANILLA_ENTITY_TYPE_SHEEP_ID,
            )
            .unwrap();
        assert!(world.apply_set_entity_data(SetEntityData {
            id: 4,
            values: vec![
                EntityDataValue {
                    data_id: wool_data_id,
                    serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_BYTE_ID,
                    value: EntityDataValueKind::Byte(wool_default),
                },
                EntityDataValue {
                    data_id: living_accessors.health_data_id,
                    serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_FLOAT_ID,
                    value: EntityDataValueKind::Float(0.0),
                },
            ],
        }));
        assert_eq!(
            world.predict_local_entity_interaction(4, InteractionHand::MainHand, false),
            pass
        );

        set_main_hand(&mut world, VANILLA_ITEM_NAME_TAG_ID);
        assert_eq!(
            world.predict_local_entity_interaction(4, InteractionHand::MainHand, false),
            pass
        );
        let named_name_tag = ItemStackSummary {
            item_id: Some(VANILLA_ITEM_NAME_TAG_ID),
            count: 1,
            component_patch: DataComponentPatchSummary {
                added: 1,
                added_type_ids: vec![VANILLA_DATA_COMPONENT_CUSTOM_NAME_TYPE_ID],
                custom_name: Some("Woolly".to_string()),
                ..DataComponentPatchSummary::default()
            },
        };
        set_offhand(&mut world, named_name_tag.clone());
        assert_eq!(
            world.predict_local_entity_interaction(4, InteractionHand::OffHand, false),
            client_success
        );
        assert_eq!(
            world
                .local_item_in_hand(InteractionHand::OffHand)
                .unwrap()
                .count,
            1
        );

        world.set_default_item_component_type_ids(BTreeMap::from([
            (
                VANILLA_ITEM_RED_DYE_ID,
                BTreeSet::from([VANILLA_DATA_COMPONENT_DYE_TYPE_ID]),
            ),
            (
                VANILLA_ITEM_NAME_TAG_ID,
                BTreeSet::from([VANILLA_DATA_COMPONENT_EQUIPPABLE_TYPE_ID]),
            ),
        ]));
        assert_eq!(
            world.predict_local_entity_interaction(4, InteractionHand::OffHand, false),
            client_success
        );

        let patched_equippable = equippable(false);
        world.apply_set_player_inventory(SetPlayerInventory {
            slot: VANILLA_PLAYER_INVENTORY_OFFHAND_SLOT,
            item: ItemStackSummary {
                component_patch: DataComponentPatchSummary {
                    added: 2,
                    added_type_ids: vec![
                        VANILLA_DATA_COMPONENT_CUSTOM_NAME_TYPE_ID,
                        VANILLA_DATA_COMPONENT_EQUIPPABLE_TYPE_ID,
                    ],
                    custom_name: Some("Equippable".to_string()),
                    equippable: Some(patched_equippable),
                    ..DataComponentPatchSummary::default()
                },
                ..named_name_tag.clone()
            },
        });
        assert_eq!(
            world.predict_local_entity_interaction(4, InteractionHand::OffHand, false),
            client_success
        );

        let mut patched_equippable = equippable(false);
        patched_equippable.equip_on_interact = true;
        world.apply_set_player_inventory(SetPlayerInventory {
            slot: VANILLA_PLAYER_INVENTORY_OFFHAND_SLOT,
            item: ItemStackSummary {
                component_patch: DataComponentPatchSummary {
                    added: 2,
                    added_type_ids: vec![
                        VANILLA_DATA_COMPONENT_CUSTOM_NAME_TYPE_ID,
                        VANILLA_DATA_COMPONENT_EQUIPPABLE_TYPE_ID,
                    ],
                    custom_name: Some("Equippable".to_string()),
                    equippable: Some(patched_equippable),
                    ..DataComponentPatchSummary::default()
                },
                ..named_name_tag
            },
        });
        assert_eq!(
            world.predict_local_entity_interaction(4, InteractionHand::OffHand, false),
            client_success
        );
    }

    #[test]
    fn shears_clear_local_leash_connections_inside_vanilla_search_box() {
        let mut world = WorldStore::new();
        add_entity(&mut world, 100, VANILLA_ENTITY_TYPE_COW_ID);
        add_entity_at(
            &mut world,
            101,
            VANILLA_ENTITY_TYPE_COW_ID,
            Vec3d {
                x: 15.0,
                y: 0.0,
                z: 0.0,
            },
        );
        add_entity_at(
            &mut world,
            102,
            VANILLA_ENTITY_TYPE_COW_ID,
            Vec3d {
                x: 17.0,
                y: 0.0,
                z: 0.0,
            },
        );
        for (source_id, dest_id) in [(100, 999), (101, 100), (102, 100)] {
            assert!(world.apply_set_entity_link(SetEntityLink { source_id, dest_id }));
        }
        set_main_hand(&mut world, VANILLA_ITEM_SHEARS_ID);

        assert_eq!(
            world.predict_local_entity_interaction(100, InteractionHand::MainHand, true),
            LocalTargetUsePrediction::Result(LocalInteractionResult::Success(
                LocalInteractionSuccess::client_item_interaction()
            ))
        );
        assert_eq!(world.entities.leash_holder_id(100), None);
        assert_eq!(world.entities.leash_holder_id(101), None);
        assert_eq!(world.entities.leash_holder_id(102), Some(100));
    }

    #[test]
    fn shears_clear_first_eligible_mob_equipment_in_vanilla_slot_order() {
        let mut world = WorldStore::new();
        add_entity(&mut world, 110, VANILLA_ENTITY_TYPE_COW_ID);
        install_test_enchantment_registry(&mut world);
        world.set_default_shearable_equipment_item_ids(BTreeSet::from([VANILLA_ITEM_SADDLE_ID]));
        let binding_curse = world
            .registry_holder_id_by_resource_id("minecraft:enchantment", "minecraft:binding_curse")
            .unwrap();
        let patched_item = |can_be_sheared, enchantments| ItemStackSummary {
            item_id: Some(VANILLA_ITEM_STONE_ID),
            count: 1,
            component_patch: DataComponentPatchSummary {
                added_type_ids: vec![
                    VANILLA_DATA_COMPONENT_EQUIPPABLE_TYPE_ID,
                    VANILLA_DATA_COMPONENT_ENCHANTMENTS_TYPE_ID,
                ],
                equippable: Some(equippable(can_be_sheared)),
                enchantments,
                ..DataComponentPatchSummary::default()
            },
        };
        let removed_default = ItemStackSummary {
            component_patch: DataComponentPatchSummary {
                removed_type_ids: vec![VANILLA_DATA_COMPONENT_EQUIPPABLE_TYPE_ID],
                ..DataComponentPatchSummary::default()
            },
            ..stack(VANILLA_ITEM_SADDLE_ID)
        };
        assert!(world.apply_set_equipment(SetEquipment {
            entity_id: 110,
            slots: vec![
                EquipmentSlotUpdate {
                    slot: EquipmentSlot::MainHand,
                    item: removed_default,
                },
                EquipmentSlotUpdate {
                    slot: EquipmentSlot::OffHand,
                    item: patched_item(false, Vec::new()),
                },
                EquipmentSlotUpdate {
                    slot: EquipmentSlot::Feet,
                    item: patched_item(
                        true,
                        vec![ItemEnchantmentSummary {
                            holder_id: binding_curse,
                            level: 1,
                        }],
                    ),
                },
                EquipmentSlotUpdate {
                    slot: EquipmentSlot::Legs,
                    item: stack(VANILLA_ITEM_SADDLE_ID),
                },
            ],
        }));
        set_main_hand(&mut world, VANILLA_ITEM_SHEARS_ID);

        assert_eq!(
            world.predict_local_entity_interaction(110, InteractionHand::MainHand, false),
            LocalTargetUsePrediction::Result(LocalInteractionResult::Success(
                LocalInteractionSuccess::client_item_interaction()
            ))
        );
        assert!(world.equipment_item(110, EquipmentSlot::MainHand).is_some());
        assert!(world.equipment_item(110, EquipmentSlot::OffHand).is_some());
        assert!(world.equipment_item(110, EquipmentSlot::Feet).is_some());
        assert!(world.equipment_item(110, EquipmentSlot::Legs).is_none());
    }

    #[test]
    fn shearing_mob_equipment_honors_vehicle_secondary_and_creative_guards() {
        let mut world = WorldStore::new();
        install_test_enchantment_registry(&mut world);
        for id in [120, 121, 122] {
            add_entity(&mut world, id, VANILLA_ENTITY_TYPE_ZOMBIE_ID);
            assert!(world.apply_set_equipment(SetEquipment {
                entity_id: id,
                slots: vec![EquipmentSlotUpdate {
                    slot: EquipmentSlot::Body,
                    item: stack(VANILLA_ITEM_SADDLE_ID),
                }],
            }));
        }
        add_entity(&mut world, 123, VANILLA_ENTITY_TYPE_AREA_EFFECT_CLOUD_ID);
        world.set_default_shearable_equipment_item_ids(BTreeSet::from([VANILLA_ITEM_SADDLE_ID]));
        set_main_hand(&mut world, VANILLA_ITEM_SHEARS_ID);
        assert!(world.apply_set_passengers(SetPassengers {
            vehicle_id: 120,
            passenger_ids: vec![123],
        }));

        for (entity_id, secondary_use_active) in [(120, false), (121, true)] {
            assert_eq!(
                world.predict_local_entity_interaction(
                    entity_id,
                    InteractionHand::MainHand,
                    secondary_use_active,
                ),
                LocalTargetUsePrediction::Result(LocalInteractionResult::Pass)
            );
            assert!(world
                .equipment_item(entity_id, EquipmentSlot::Body)
                .is_some());
        }

        let binding_curse = world
            .registry_holder_id_by_resource_id("minecraft:enchantment", "minecraft:binding_curse")
            .unwrap();
        assert!(world.apply_set_equipment(SetEquipment {
            entity_id: 122,
            slots: vec![EquipmentSlotUpdate {
                slot: EquipmentSlot::Body,
                item: ItemStackSummary {
                    component_patch: DataComponentPatchSummary {
                        added_type_ids: vec![VANILLA_DATA_COMPONENT_ENCHANTMENTS_TYPE_ID],
                        enchantments: vec![ItemEnchantmentSummary {
                            holder_id: binding_curse,
                            level: 1,
                        }],
                        ..DataComponentPatchSummary::default()
                    },
                    ..stack(VANILLA_ITEM_SADDLE_ID)
                },
            }],
        }));
        assert_eq!(
            world.predict_local_entity_interaction(122, InteractionHand::MainHand, false),
            LocalTargetUsePrediction::Result(LocalInteractionResult::Pass)
        );
        world.apply_game_event(bbb_protocol::packets::GameEvent {
            event_id: bbb_protocol::game_event_types::VANILLA_CLIENT_GAME_EVENT_CHANGE_GAME_MODE_ID,
            param: bbb_protocol::game_types::VANILLA_GAME_TYPE_CREATIVE_ID as f32,
        });
        assert_eq!(
            world.predict_local_entity_interaction(122, InteractionHand::MainHand, false),
            LocalTargetUsePrediction::Result(LocalInteractionResult::Success(
                LocalInteractionSuccess::client_item_interaction()
            ))
        );
        assert!(world.equipment_item(122, EquipmentSlot::Body).is_none());
    }

    #[test]
    fn wolf_equipment_shearing_requires_the_synced_owner() {
        let mut world = WorldStore::new();
        add_entity(&mut world, 130, VANILLA_ENTITY_TYPE_WOLF_ID);
        add_entity(&mut world, 131, VANILLA_ENTITY_TYPE_AREA_EFFECT_CLOUD_ID);
        world.set_default_shearable_equipment_item_ids(BTreeSet::from([VANILLA_ITEM_SADDLE_ID]));
        assert!(world.apply_set_equipment(SetEquipment {
            entity_id: 130,
            slots: vec![EquipmentSlotUpdate {
                slot: EquipmentSlot::Body,
                item: stack(VANILLA_ITEM_SADDLE_ID),
            }],
        }));
        assert!(world.apply_set_passengers(SetPassengers {
            vehicle_id: 130,
            passenger_ids: vec![131],
        }));
        set_main_hand(&mut world, VANILLA_ITEM_SHEARS_ID);

        assert!(matches!(
            world.predict_local_entity_interaction(130, InteractionHand::MainHand, false),
            LocalTargetUsePrediction::UnsupportedEntityInteract(_)
        ));
        assert!(world.equipment_item(130, EquipmentSlot::Body).is_some());

        let local_player = Uuid::from_u128(0xabc);
        world.set_local_player_profile_id(local_player);
        let accessors = vanilla_entity_metadata_tamable_animal_accessors_for_entity_type(
            VANILLA_ENTITY_TYPE_WOLF_ID,
        )
        .unwrap();
        assert!(world.apply_set_entity_data(SetEntityData {
            id: 130,
            values: vec![EntityDataValue {
                data_id: accessors.owner_data_id,
                serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_OPTIONAL_LIVING_ENTITY_REFERENCE_ID,
                value: EntityDataValueKind::OptionalLivingEntityReference(Some(local_player)),
            }],
        }));
        assert_eq!(
            world.predict_local_entity_interaction(130, InteractionHand::MainHand, false),
            LocalTargetUsePrediction::Result(LocalInteractionResult::Success(
                LocalInteractionSuccess::client_item_interaction()
            ))
        );
        assert!(world.equipment_item(130, EquipmentSlot::Body).is_none());
    }

    #[test]
    fn specialized_shearable_mobs_follow_their_exact_client_result_gates() {
        let mut world = WorldStore::new();
        for (entity_id, entity_type_id) in [
            (140, VANILLA_ENTITY_TYPE_BOGGED_ID),
            (141, VANILLA_ENTITY_TYPE_COPPER_GOLEM_ID),
            (142, VANILLA_ENTITY_TYPE_MOOSHROOM_ID),
            (143, VANILLA_ENTITY_TYPE_SHEEP_ID),
            (144, VANILLA_ENTITY_TYPE_SNOW_GOLEM_ID),
        ] {
            add_entity(&mut world, entity_id, entity_type_id);
        }
        set_main_hand(&mut world, VANILLA_ITEM_SHEARS_ID);
        let client_success = LocalTargetUsePrediction::Result(LocalInteractionResult::Success(
            LocalInteractionSuccess::client_item_interaction(),
        ));
        let pass = LocalTargetUsePrediction::Result(LocalInteractionResult::Pass);

        assert_eq!(
            world.predict_local_entity_interaction(140, InteractionHand::MainHand, false),
            client_success
        );
        let (bogged_data_id, bogged_default) =
            vanilla_entity_metadata_bogged_sheared_accessor_for_entity_type(
                VANILLA_ENTITY_TYPE_BOGGED_ID,
            )
            .unwrap();
        assert!(!bogged_default);
        assert!(world.apply_set_entity_data(SetEntityData {
            id: 140,
            values: vec![EntityDataValue {
                data_id: bogged_data_id,
                serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_BOOLEAN_ID,
                value: EntityDataValueKind::Boolean(true),
            }],
        }));
        assert_eq!(
            world.predict_local_entity_interaction(140, InteractionHand::MainHand, false),
            pass
        );

        assert!(world.apply_set_equipment(SetEquipment {
            entity_id: 141,
            slots: vec![EquipmentSlotUpdate {
                slot: EquipmentSlot::Saddle,
                item: stack(VANILLA_ITEM_POPPY_ID),
            }],
        }));
        assert_eq!(
            world.predict_local_entity_interaction(141, InteractionHand::MainHand, false),
            LocalTargetUsePrediction::UnsupportedEntityInteract(
                VANILLA_COPPER_GOLEM_MOB_INTERACT_OWNER
            )
        );
        world.apply_update_tags(UpdateTags {
            registries: vec![RegistryTags {
                registry: "minecraft:item".to_string(),
                tags: vec![TagNetworkPayload {
                    tag: VANILLA_COPPER_GOLEM_SHEARABLE_ITEMS_TAG.to_string(),
                    entries: vec![VANILLA_ITEM_POPPY_ID],
                }],
            }],
        });
        assert_eq!(
            world.predict_local_entity_interaction(141, InteractionHand::MainHand, false),
            client_success
        );
        world.apply_update_tags(UpdateTags {
            registries: vec![RegistryTags {
                registry: "minecraft:item".to_string(),
                tags: vec![TagNetworkPayload {
                    tag: VANILLA_COPPER_GOLEM_SHEARABLE_ITEMS_TAG.to_string(),
                    entries: Vec::new(),
                }],
            }],
        });
        assert_eq!(
            world.predict_local_entity_interaction(141, InteractionHand::MainHand, false),
            pass
        );

        assert_eq!(
            world.predict_local_entity_interaction(142, InteractionHand::MainHand, false),
            client_success
        );
        let (mooshroom_baby_data_id, mooshroom_baby_default) =
            crate::entities::vanilla_entity_metadata_baby_accessor_for_entity_type(
                VANILLA_ENTITY_TYPE_MOOSHROOM_ID,
            )
            .unwrap();
        assert!(!mooshroom_baby_default);
        assert!(world.apply_set_entity_data(SetEntityData {
            id: 142,
            values: vec![EntityDataValue {
                data_id: mooshroom_baby_data_id,
                serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_BOOLEAN_ID,
                value: EntityDataValueKind::Boolean(true),
            }],
        }));
        assert_eq!(
            world.predict_local_entity_interaction(142, InteractionHand::MainHand, false),
            LocalTargetUsePrediction::UnsupportedEntityInteract(
                VANILLA_MOOSHROOM_MOB_INTERACT_OWNER
            )
        );

        let (sheep_baby_data_id, _) =
            crate::entities::vanilla_entity_metadata_baby_accessor_for_entity_type(
                VANILLA_ENTITY_TYPE_SHEEP_ID,
            )
            .unwrap();
        let (sheep_wool_data_id, sheep_wool_default, sheep_sheared_mask, _) =
            crate::entities::vanilla_entity_metadata_sheep_wool_accessor_for_entity_type(
                VANILLA_ENTITY_TYPE_SHEEP_ID,
            )
            .unwrap();
        assert!(world.apply_set_entity_data(SetEntityData {
            id: 143,
            values: vec![
                EntityDataValue {
                    data_id: sheep_baby_data_id,
                    serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_BOOLEAN_ID,
                    value: EntityDataValueKind::Boolean(true),
                },
                EntityDataValue {
                    data_id: sheep_wool_data_id,
                    serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_BYTE_ID,
                    value: EntityDataValueKind::Byte(sheep_wool_default | sheep_sheared_mask),
                },
            ],
        }));
        assert_eq!(
            world.predict_local_entity_interaction(143, InteractionHand::MainHand, false),
            LocalTargetUsePrediction::Result(LocalInteractionResult::Success(
                LocalInteractionSuccess::consume(false)
            ))
        );

        assert_eq!(
            world.predict_local_entity_interaction(144, InteractionHand::MainHand, false),
            client_success
        );
        let (snow_data_id, snow_default, pumpkin_mask) =
            vanilla_entity_metadata_snow_golem_pumpkin_accessor_for_entity_type(
                VANILLA_ENTITY_TYPE_SNOW_GOLEM_ID,
            )
            .unwrap();
        assert_ne!(snow_default & pumpkin_mask, 0);
        assert!(world.apply_set_entity_data(SetEntityData {
            id: 144,
            values: vec![EntityDataValue {
                data_id: snow_data_id,
                serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_BYTE_ID,
                value: EntityDataValueKind::Byte(snow_default & !pumpkin_mask),
            }],
        }));
        assert_eq!(
            world.predict_local_entity_interaction(144, InteractionHand::MainHand, false),
            pass
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
            LocalTargetUsePrediction::Result(LocalInteractionResult::Pass)
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
        for entity_id in [72, 73] {
            assert_eq!(
                world.predict_local_entity_interaction(entity_id, InteractionHand::MainHand, false),
                LocalTargetUsePrediction::Result(LocalInteractionResult::Pass)
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
    fn nautilus_mob_interaction_uses_synced_tame_age_health_and_food_state() {
        let mut world = WorldStore::new();
        add_entity(&mut world, 90, VANILLA_ENTITY_TYPE_NAUTILUS_ID);
        add_entity(&mut world, 91, VANILLA_ENTITY_TYPE_ZOMBIE_NAUTILUS_ID);
        let tame_accessors = vanilla_entity_metadata_tamable_animal_accessors_for_entity_type(
            VANILLA_ENTITY_TYPE_NAUTILUS_ID,
        )
        .unwrap();
        let living_accessors =
            crate::entities::vanilla_entity_metadata_living_entity_accessors_for_entity_type(
                VANILLA_ENTITY_TYPE_NAUTILUS_ID,
            )
            .unwrap();
        let (baby_data_id, baby_default) =
            crate::entities::vanilla_entity_metadata_baby_accessor_for_entity_type(
                VANILLA_ENTITY_TYPE_NAUTILUS_ID,
            )
            .unwrap();
        let (age_locked_data_id, age_locked_default) =
            vanilla_entity_metadata_age_locked_accessor_for_entity_type(
                VANILLA_ENTITY_TYPE_NAUTILUS_ID,
            )
            .unwrap();
        assert!(!baby_default);
        assert!(!age_locked_default);

        let client_success = LocalTargetUsePrediction::Result(LocalInteractionResult::Success(
            LocalInteractionSuccess::client_item_interaction(),
        ));
        let consume = LocalTargetUsePrediction::Result(LocalInteractionResult::Success(
            LocalInteractionSuccess::consume(false),
        ));
        assert_eq!(
            world.predict_local_entity_interaction(90, InteractionHand::MainHand, false),
            LocalTargetUsePrediction::Result(LocalInteractionResult::Pass)
        );
        set_main_hand(&mut world, VANILLA_ITEM_STONE_ID);
        assert_eq!(
            world.predict_local_entity_interaction(90, InteractionHand::MainHand, false),
            LocalTargetUsePrediction::UnsupportedEntityInteract(VANILLA_NAUTILUS_INTERACT_OWNER)
        );
        world.apply_update_tags(UpdateTags {
            registries: vec![RegistryTags {
                registry: "minecraft:item".to_string(),
                tags: vec![
                    TagNetworkPayload {
                        tag: VANILLA_NAUTILUS_TAMING_ITEMS_TAG.to_string(),
                        entries: vec![VANILLA_ITEM_STONE_ID],
                    },
                    TagNetworkPayload {
                        tag: VANILLA_NAUTILUS_FOOD_TAG.to_string(),
                        entries: vec![VANILLA_ITEM_APPLE_ID],
                    },
                ],
            }],
        });
        assert_eq!(
            world.predict_local_entity_interaction(90, InteractionHand::MainHand, false),
            client_success
        );
        assert_eq!(
            world.predict_local_entity_interaction(91, InteractionHand::MainHand, false),
            client_success
        );
        assert!(world.apply_set_entity_data(SetEntityData {
            id: 90,
            values: vec![EntityDataValue {
                data_id: living_accessors.health_data_id,
                serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_FLOAT_ID,
                value: EntityDataValueKind::Float(15.0),
            }],
        }));
        assert_eq!(
            world.predict_local_entity_interaction(90, InteractionHand::MainHand, false),
            consume
        );

        set_main_hand(&mut world, VANILLA_ITEM_RED_DYE_ID);
        assert_eq!(
            world.predict_local_entity_interaction(90, InteractionHand::MainHand, false),
            LocalTargetUsePrediction::Result(LocalInteractionResult::Pass)
        );
        assert!(world.apply_set_entity_data(SetEntityData {
            id: 90,
            values: vec![EntityDataValue {
                data_id: tame_accessors.flags_data_id,
                serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_BYTE_ID,
                value: EntityDataValueKind::Byte(tame_accessors.tame_mask),
            }],
        }));
        assert_eq!(
            world.predict_local_entity_interaction(90, InteractionHand::MainHand, true),
            client_success
        );
        assert_eq!(
            world.predict_local_entity_interaction(90, InteractionHand::MainHand, false),
            client_success
        );
        set_main_hand(&mut world, VANILLA_ITEM_APPLE_ID);
        assert_eq!(
            world.predict_local_entity_interaction(90, InteractionHand::MainHand, false),
            consume
        );
        assert!(world.apply_set_entity_data(SetEntityData {
            id: 90,
            values: vec![EntityDataValue {
                data_id: living_accessors.health_data_id,
                serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_FLOAT_ID,
                value: EntityDataValueKind::Float(1.0),
            }],
        }));
        assert_eq!(
            world.predict_local_entity_interaction(90, InteractionHand::MainHand, false),
            client_success
        );
        set_main_hand(&mut world, VANILLA_ITEM_STONE_ID);
        assert_eq!(
            world.predict_local_entity_interaction(90, InteractionHand::MainHand, false),
            client_success
        );

        assert!(world.apply_set_entity_data(SetEntityData {
            id: 90,
            values: vec![
                EntityDataValue {
                    data_id: baby_data_id,
                    serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_BOOLEAN_ID,
                    value: EntityDataValueKind::Boolean(true),
                },
                EntityDataValue {
                    data_id: age_locked_data_id,
                    serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_BOOLEAN_ID,
                    value: EntityDataValueKind::Boolean(true),
                },
            ],
        }));
        set_main_hand(&mut world, VANILLA_ITEM_APPLE_ID);
        assert_eq!(
            world.predict_local_entity_interaction(90, InteractionHand::MainHand, false),
            consume
        );
        set_main_hand(&mut world, VANILLA_ITEM_GOLDEN_DANDELION_ID);
        assert_eq!(
            world.predict_local_entity_interaction(90, InteractionHand::MainHand, false),
            client_success
        );
        assert_eq!(
            world.predict_local_entity_interaction(90, InteractionHand::MainHand, false),
            LocalTargetUsePrediction::Result(LocalInteractionResult::Pass)
        );
        assert_eq!(
            world
                .entities
                .metadata_bool_for_entity(90, age_locked_data_id, age_locked_default),
            Some(false)
        );
        set_main_hand(&mut world, VANILLA_ITEM_APPLE_ID);
        assert_eq!(
            world.predict_local_entity_interaction(90, InteractionHand::MainHand, false),
            client_success
        );
    }

    #[test]
    fn age_lock_particle_timer_expires_after_forty_client_ticks() {
        let mut state = crate::entities::EntityClientAnimationState::default();
        state.start_age_lock_particle_timer();
        assert_eq!(state.age_lock_particle_timer_ticks(), 40);
        for expected in (0..40).rev() {
            state.advance_age_lock_particle_timer_tick();
            assert_eq!(state.age_lock_particle_timer_ticks(), expected);
        }
    }

    #[test]
    fn iron_golem_interaction_applies_client_heal_before_selecting_result() {
        let mut world = WorldStore::new();
        add_entity(&mut world, 92, VANILLA_ENTITY_TYPE_IRON_GOLEM_ID);
        assert_eq!(
            world.predict_local_entity_interaction(92, InteractionHand::MainHand, false),
            LocalTargetUsePrediction::Result(LocalInteractionResult::Pass)
        );

        let client_success = LocalTargetUsePrediction::Result(LocalInteractionResult::Success(
            LocalInteractionSuccess::client_item_interaction(),
        ));
        set_main_hand(&mut world, VANILLA_ITEM_IRON_INGOT_ID);
        for expected_health in [26.0, 51.0, 76.0, 100.0] {
            assert_eq!(
                world.predict_local_entity_interaction(92, InteractionHand::MainHand, false),
                client_success
            );
            assert_eq!(
                world.entities.living_entity_health(92),
                Some(expected_health)
            );
        }
        assert_eq!(
            world.predict_local_entity_interaction(92, InteractionHand::MainHand, false),
            LocalTargetUsePrediction::Result(LocalInteractionResult::Pass)
        );

        set_main_hand(&mut world, VANILLA_ITEM_RED_DYE_ID);
        assert_eq!(
            world.predict_local_entity_interaction(92, InteractionHand::MainHand, false),
            LocalTargetUsePrediction::Result(LocalInteractionResult::Pass)
        );
        let living_accessors =
            crate::entities::vanilla_entity_metadata_living_entity_accessors_for_entity_type(
                VANILLA_ENTITY_TYPE_IRON_GOLEM_ID,
            )
            .unwrap();
        assert!(world.apply_set_entity_data(SetEntityData {
            id: 92,
            values: vec![EntityDataValue {
                data_id: living_accessors.health_data_id,
                serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_FLOAT_ID,
                value: EntityDataValueKind::Float(0.0),
            }],
        }));
        set_main_hand(&mut world, VANILLA_ITEM_IRON_INGOT_ID);
        assert_eq!(
            world.predict_local_entity_interaction(92, InteractionHand::MainHand, false),
            LocalTargetUsePrediction::Result(LocalInteractionResult::Pass)
        );
        assert_eq!(world.entities.living_entity_health(92), Some(0.0));
    }

    #[test]
    fn creeper_interaction_uses_synced_igniter_tag_for_client_success() {
        let mut world = WorldStore::new();
        add_entity(&mut world, 93, VANILLA_ENTITY_TYPE_CREEPER_ID);
        assert_eq!(
            world.predict_local_entity_interaction(93, InteractionHand::MainHand, false),
            LocalTargetUsePrediction::Result(LocalInteractionResult::Pass)
        );

        set_main_hand(&mut world, VANILLA_ITEM_STONE_ID);
        assert_eq!(
            world.predict_local_entity_interaction(93, InteractionHand::MainHand, false),
            LocalTargetUsePrediction::UnsupportedEntityInteract(VANILLA_CREEPER_MOB_INTERACT_OWNER)
        );
        world.apply_update_tags(UpdateTags {
            registries: vec![RegistryTags {
                registry: "minecraft:item".to_string(),
                tags: vec![TagNetworkPayload {
                    tag: VANILLA_CREEPER_IGNITERS_TAG.to_string(),
                    entries: vec![VANILLA_ITEM_STONE_ID],
                }],
            }],
        });
        assert_eq!(
            world.predict_local_entity_interaction(93, InteractionHand::MainHand, false),
            LocalTargetUsePrediction::Result(LocalInteractionResult::Success(
                LocalInteractionSuccess::client_item_interaction()
            ))
        );

        set_main_hand(&mut world, VANILLA_ITEM_RED_DYE_ID);
        assert_eq!(
            world.predict_local_entity_interaction(93, InteractionHand::MainHand, false),
            LocalTargetUsePrediction::Result(LocalInteractionResult::Pass)
        );
        set_main_hand(&mut world, VANILLA_ITEM_APPLE_ID);
        assert_eq!(
            world.predict_local_entity_interaction(93, InteractionHand::MainHand, false),
            LocalTargetUsePrediction::Result(LocalInteractionResult::Pass)
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
            client_success
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
            LocalTargetUsePrediction::Result(LocalInteractionResult::Pass)
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
            LocalTargetUsePrediction::Result(LocalInteractionResult::Success(
                LocalInteractionSuccess::consume(false)
            ))
        );
    }
}
