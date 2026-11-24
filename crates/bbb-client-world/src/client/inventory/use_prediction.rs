use super::*;
use crate::{
    entities::{
        vanilla_bounds_for_type, vanilla_entity_metadata_abstract_horse_accessors_for_entity_type,
        vanilla_entity_metadata_age_locked_accessor_for_entity_type,
        vanilla_entity_metadata_armor_stand_accessors_for_entity_type,
        vanilla_entity_metadata_bogged_sheared_accessor_for_entity_type,
        vanilla_entity_metadata_food_predicate, vanilla_entity_metadata_interact_owner,
        vanilla_entity_metadata_interaction_accessors_for_entity_type,
        vanilla_entity_metadata_mob_interact_owner,
        vanilla_entity_metadata_sheep_wool_accessor_for_entity_type,
        vanilla_entity_metadata_snow_golem_pumpkin_accessor_for_entity_type,
        vanilla_entity_metadata_tamable_animal_accessors_for_entity_type,
        vanilla_entity_metadata_type_is_leashable, vanilla_entity_metadata_type_is_living,
        vanilla_entity_type_is_minecart, vanilla_pick_bounds_for_type, VanillaEntityFoodPredicate,
        VanillaEntityOwner,
    },
    BlockPos, LevelEventSoundRandomState,
};
use bbb_protocol::{
    attribute_types::VANILLA_ATTRIBUTE_MAX_HEALTH_ID,
    block_state_types::{
        vanilla_block_default_state_id, vanilla_block_property_values,
        vanilla_block_state_properties, vanilla_block_state_with_property,
    },
    block_types::{
        vanilla_block_type, vanilla_block_type_for_resource_id, VanillaBlockUseItemOnOverride,
        VanillaBlockUseWithoutItemOverride, VanillaCanSurviveOverride, VANILLA_BLOCK_AIR_ID,
        VANILLA_BLOCK_BEDROCK_ID, VANILLA_BLOCK_COPPER_GOLEM_STATUE_ID,
        VANILLA_BLOCK_END_PORTAL_FRAME_ID, VANILLA_BLOCK_LODESTONE_ID, VANILLA_BLOCK_OBSIDIAN_ID,
    },
    builtin_enum_component_values::normalize_vanilla_dye_color_id,
    data_component_types::{
        VANILLA_DATA_COMPONENT_BLOCKS_ATTACKS_TYPE_ID, VANILLA_DATA_COMPONENT_CUSTOM_NAME_TYPE_ID,
        VANILLA_DATA_COMPONENT_DYE_TYPE_ID, VANILLA_DATA_COMPONENT_ENCHANTMENTS_TYPE_ID,
        VANILLA_DATA_COMPONENT_EQUIPPABLE_TYPE_ID, VANILLA_DATA_COMPONENT_KINETIC_WEAPON_TYPE_ID,
        VANILLA_DATA_COMPONENT_RECIPES_TYPE_ID, VANILLA_DATA_COMPONENT_USE_COOLDOWN_TYPE_ID,
        VANILLA_DATA_COMPONENT_USE_REMAINDER_TYPE_ID,
    },
    entity_types::{
        vanilla_entity_type_is_serializable, VANILLA_ENTITY_TYPE_AXOLOTL_ID,
        VANILLA_ENTITY_TYPE_MINECART_ID, VANILLA_ENTITY_TYPE_NAUTILUS_ID,
        VANILLA_ENTITY_TYPE_WOLF_ID, VANILLA_ENTITY_TYPE_ZOMBIE_NAUTILUS_ID,
    },
    equipment_slots::VANILLA_EQUIPMENT_SLOTS,
    item_types::{
        vanilla_item_type, ItemTypeCatalogEntry, VanillaBlockItemKind, VanillaBlockTransform,
        VanillaItemInteractLivingEntityOverride, VanillaItemUseOnOverride, VanillaItemUseOverride,
        VANILLA_AXE_SCRAPEABLES, VANILLA_AXE_STRIPPABLES, VANILLA_AXE_WAX_OFF,
        VANILLA_HOE_TILLABLES, VANILLA_HONEYCOMB_WAXABLES, VANILLA_ITEM_APPLE_ID,
        VANILLA_ITEM_CARROT_ID, VANILLA_ITEM_ENCHANTED_GOLDEN_APPLE_ID,
        VANILLA_ITEM_GOLDEN_APPLE_ID, VANILLA_ITEM_GOLDEN_CARROT_ID,
        VANILLA_ITEM_GOLDEN_DANDELION_ID, VANILLA_ITEM_HAY_BLOCK_ID, VANILLA_ITEM_IRON_INGOT_ID,
        VANILLA_ITEM_LEAD_ID, VANILLA_ITEM_RED_MUSHROOM_ID, VANILLA_ITEM_SHEARS_ID,
        VANILLA_ITEM_SUGAR_ID, VANILLA_ITEM_TROPICAL_FISH_BUCKET_ID, VANILLA_ITEM_WATER_BUCKET_ID,
        VANILLA_ITEM_WHEAT_ID, VANILLA_SHOVEL_FLATTENABLES,
    },
    packets::{
        BlockPos as ProtocolBlockPos, ConsumableSummary, Direction as ProtocolDirection,
        LodestoneTargetSummary,
    },
};

mod block_use_dispatch;
mod entity_interact_dispatch;
mod item_use_dispatch;
mod mob_interact_dispatch;

mod block_placement;
mod block_use_item_on;
mod block_use_without_item;
mod bone_meal;
mod book;
mod bucket;
mod bundle;
mod comparator;
mod crossbow;
mod empty_map;
mod entity_placement;
mod equippable;
mod firework_rocket;
mod fishing_rod;
mod food_on_a_stick;
mod ignition;
mod instrument;
mod item_use_fov;
mod knowledge_book;
mod mob_interact;
mod potion;
mod pov_item;
mod redstone_wire;
mod thrown_projectile;
mod trident;

pub use pov_item::{LocalItemUsePovFluidMode, LocalItemUsePovQuery, LocalItemUsePovResult};

const VANILLA_DEFAULT_ENTITY_INTERACT_OWNER: VanillaEntityOwner = VanillaEntityOwner::Entity;
const VANILLA_MOB_ENTITY_INTERACT_OWNER: VanillaEntityOwner = VanillaEntityOwner::Mob;
const VANILLA_AGEABLE_MOB_INTERACT_OWNER: VanillaEntityOwner = VanillaEntityOwner::AgeableMob;
const VANILLA_ANIMAL_MOB_INTERACT_OWNER: VanillaEntityOwner = VanillaEntityOwner::Animal;
const VANILLA_ARMOR_STAND_INTERACT_OWNER: VanillaEntityOwner = VanillaEntityOwner::ArmorStand;
const VANILLA_BOAT_INTERACT_OWNER: VanillaEntityOwner = VanillaEntityOwner::AbstractBoat;
const VANILLA_BOGGED_MOB_INTERACT_OWNER: VanillaEntityOwner = VanillaEntityOwner::Bogged;
const VANILLA_CAMEL_HUSK_INTERACT_OWNER: VanillaEntityOwner = VanillaEntityOwner::CamelHusk;
const VANILLA_CAMEL_MOB_INTERACT_OWNER: VanillaEntityOwner = VanillaEntityOwner::Camel;
const VANILLA_CAMEL_HUSK_FOOD_TAG: &str = "minecraft:camel_husk_food";
const VANILLA_CHEST_BOAT_INTERACT_OWNER: VanillaEntityOwner = VanillaEntityOwner::AbstractChestBoat;
const VANILLA_CREEPER_MOB_INTERACT_OWNER: VanillaEntityOwner = VanillaEntityOwner::Creeper;
const VANILLA_CREEPER_IGNITERS_TAG: &str = "minecraft:creeper_igniters";
const VANILLA_COPPER_GOLEM_MOB_INTERACT_OWNER: VanillaEntityOwner = VanillaEntityOwner::CopperGolem;
const VANILLA_COPPER_GOLEM_SHEARABLE_ITEMS_TAG: &str = "minecraft:shearable_from_copper_golem";
const VANILLA_INTERACTION_ENTITY_INTERACT_OWNER: VanillaEntityOwner =
    VanillaEntityOwner::Interaction;
const VANILLA_IRON_GOLEM_MOB_INTERACT_OWNER: VanillaEntityOwner = VanillaEntityOwner::IronGolem;
const VANILLA_ITEM_FRAME_INTERACT_OWNER: VanillaEntityOwner = VanillaEntityOwner::ItemFrame;
const VANILLA_LEASH_KNOT_INTERACT_OWNER: VanillaEntityOwner =
    VanillaEntityOwner::LeashFenceKnotEntity;
const VANILLA_NAUTILUS_INTERACT_OWNER: VanillaEntityOwner = VanillaEntityOwner::AbstractNautilus;
const VANILLA_NAUTILUS_BUCKET_FOOD_TAG: &str = "minecraft:nautilus_bucket_food";
const VANILLA_NAUTILUS_FOOD_TAG: &str = "minecraft:nautilus_food";
const VANILLA_NAUTILUS_TAMING_ITEMS_TAG: &str = "minecraft:nautilus_taming_items";
const VANILLA_MINECART_INTERACT_OWNER: VanillaEntityOwner = VanillaEntityOwner::Minecart;
const VANILLA_MINECART_CONTAINER_INTERACT_OWNER: VanillaEntityOwner =
    VanillaEntityOwner::AbstractMinecartContainer;
const VANILLA_MINECART_CHEST_INTERACT_OWNER: VanillaEntityOwner = VanillaEntityOwner::MinecartChest;
const VANILLA_MINECART_COMMAND_BLOCK_INTERACT_OWNER: VanillaEntityOwner =
    VanillaEntityOwner::MinecartCommandBlock;
const VANILLA_MINECART_FURNACE_INTERACT_OWNER: VanillaEntityOwner =
    VanillaEntityOwner::MinecartFurnace;
const VANILLA_MOOSHROOM_MOB_INTERACT_OWNER: VanillaEntityOwner = VanillaEntityOwner::MushroomCow;
const VANILLA_SHEEP_MOB_INTERACT_OWNER: VanillaEntityOwner = VanillaEntityOwner::Sheep;
const VANILLA_SNOW_GOLEM_MOB_INTERACT_OWNER: VanillaEntityOwner = VanillaEntityOwner::SnowGolem;
const VANILLA_VILLAGER_MOB_INTERACT_OWNER: VanillaEntityOwner = VanillaEntityOwner::Villager;
const VANILLA_WANDERING_TRADER_MOB_INTERACT_OWNER: VanillaEntityOwner =
    VanillaEntityOwner::WanderingTrader;
const VANILLA_ZOMBIE_HORSE_INTERACT_OWNER: VanillaEntityOwner = VanillaEntityOwner::ZombieHorse;
const VANILLA_ZOMBIE_HORSE_FOOD_TAG: &str = "minecraft:zombie_horse_food";
#[cfg(test)]
const VANILLA_CAN_WEAR_HORSE_ARMOR_TAG: &str = "minecraft:can_wear_horse_armor";
const VANILLA_CANNOT_BE_AGE_LOCKED_TAG: &str = "minecraft:cannot_be_age_locked";
const MAX_PENDING_LOCAL_INTERACTION_PARTICLE_ACTIONS: usize = 64;

#[derive(Debug, Clone, Copy, PartialEq)]
struct VanillaAbstractHorseFoodEffect {
    heal: f32,
    age_up: i32,
}

fn vanilla_abstract_horse_food_effect(item_id: i32) -> Option<VanillaAbstractHorseFoodEffect> {
    let (heal, age_up) = match item_id {
        VANILLA_ITEM_WHEAT_ID => (2.0, 20),
        VANILLA_ITEM_SUGAR_ID => (1.0, 30),
        VANILLA_ITEM_HAY_BLOCK_ID => (20.0, 180),
        VANILLA_ITEM_APPLE_ID => (3.0, 60),
        VANILLA_ITEM_RED_MUSHROOM_ID => (3.0, 0),
        VANILLA_ITEM_CARROT_ID => (3.0, 60),
        VANILLA_ITEM_GOLDEN_CARROT_ID => (4.0, 60),
        VANILLA_ITEM_GOLDEN_APPLE_ID | VANILLA_ITEM_ENCHANTED_GOLDEN_APPLE_ID => (10.0, 240),
        _ => return None,
    };
    Some(VanillaAbstractHorseFoodEffect { heal, age_up })
}

fn copper_golem_poses() -> Option<&'static [&'static str]> {
    let block = vanilla_block_type(VANILLA_BLOCK_COPPER_GOLEM_STATUE_ID)?;
    vanilla_block_property_values(block.resource_id, "copper_golem_pose")
}

fn copper_golem_pose_ordinal(pose: &str) -> Option<i32> {
    copper_golem_poses()?
        .iter()
        .position(|candidate| *candidate == pose)
        .and_then(|index| i32::try_from(index).ok())
}

#[derive(Debug, Clone, Copy)]
struct LocalBlockUseContext {
    pos: BlockPos,
    face: ProtocolDirection,
    cursor: [f64; 3],
    inside: bool,
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
    FlintAndSteelUse {
        pos: BlockPos,
    },
    InstrumentUse {
        hand: InteractionHand,
    },
    LoomSelectPattern,
    StonecutterSelectRecipe,
    CrafterSlotStateChanged {
        enabled: bool,
    },
    BundleInsert,
    BundleRemoveOne,
    BundleInsertFail,
    KineticWeaponUse {
        hand: InteractionHand,
    },
    PiercingWeaponUse {
        hand: InteractionHand,
    },
    BlockDestroy {
        pos: BlockPos,
        block_state_id: i32,
    },
    BlockPositioned {
        pos: BlockPos,
        sound_resource_id: &'static str,
        volume_bits: u32,
        pitch_bits: u32,
    },
    BlockToggleSound {
        pos: BlockPos,
        sound_resource_id: &'static str,
    },
    EntityPositioned {
        entity_id: i32,
        sound_resource_id: &'static str,
        volume_bits: u32,
        pitch_bits: u32,
    },
    SpyglassUse,
    SpyglassStopUsing,
    BottleFill,
    BeehiveBottleFill,
    BucketFill {
        sound_resource_id: &'static str,
    },
    BucketEmpty {
        pos: BlockPos,
        sound_resource_id: &'static str,
        neutral_source: bool,
    },
    BucketWaterEvaporate {
        pos: BlockPos,
    },
    Consume {
        hand: InteractionHand,
    },
    AllayInteraction {
        entity_id: i32,
        sound_resource_id: &'static str,
    },
    CreeperIgnition {
        entity_id: i32,
        fire_charge: bool,
    },
    LocalPlayerMobInteraction {
        sound_resource_id: &'static str,
    },
    BlockPlace {
        pos: BlockPos,
        sound_resource_id: &'static str,
        volume_bits: u32,
        pitch_bits: u32,
    },
    BlockPlaceFromProfile {
        pos: BlockPos,
        block_type_id: i32,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LocalLeverAttachFace {
    Floor,
    Wall,
    Ceiling,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LocalInteractionParticleAction {
    Consume {
        hand: InteractionHand,
        count: usize,
    },
    RedstoneOre {
        pos: BlockPos,
    },
    LeverActivation {
        pos: BlockPos,
        facing: ProtocolDirection,
        face: LocalLeverAttachFace,
    },
    BucketWaterEvaporate {
        pos: BlockPos,
    },
    CandleCakeExtinguish {
        pos: BlockPos,
    },
    CandleExtinguish {
        pos: BlockPos,
        candles: u8,
    },
    EquineBabyFeeding {
        position_bits: [u64; 3],
    },
    TadpoleFeeding {
        position_bits: [u64; 3],
    },
    MooshroomSmoke {
        position_bits: [[u64; 3]; 2],
        velocity_y_bits: [u64; 2],
    },
    MooshroomEffect {
        position_bits: [[u64; 3]; 4],
        velocity_y_bits: [u64; 4],
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
    UnsupportedEntityInteract(VanillaEntityOwner),
    UnsupportedItemInteractLivingEntity(VanillaItemInteractLivingEntityOverride),
    UnsupportedItemStackInteractLivingEntityComponent(i32),
    UnknownBlockType(Option<i32>),
    UnknownEntityType(Option<i32>),
    UnknownItemType(Option<i32>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum GeneratedItemUseOnRoute {
    Tool,
    ClientOwned,
    EntitySpawning,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct LocalUsingItemTickActions {
    pub bundle_drops: u32,
}

const fn generated_item_use_on_route(owner: VanillaItemUseOnOverride) -> GeneratedItemUseOnRoute {
    match owner {
        VanillaItemUseOnOverride::AxeItem
        | VanillaItemUseOnOverride::HoneycombItem
        | VanillaItemUseOnOverride::HoeItem
        | VanillaItemUseOnOverride::ShearsItem
        | VanillaItemUseOnOverride::ShovelItem => GeneratedItemUseOnRoute::Tool,
        VanillaItemUseOnOverride::BlockItem
        | VanillaItemUseOnOverride::BoneMealItem
        | VanillaItemUseOnOverride::BrushItem
        | VanillaItemUseOnOverride::CompassItem
        | VanillaItemUseOnOverride::DebugStickItem
        | VanillaItemUseOnOverride::EnderEyeItem
        | VanillaItemUseOnOverride::FireChargeItem
        | VanillaItemUseOnOverride::FlintAndSteelItem
        | VanillaItemUseOnOverride::LeadItem
        | VanillaItemUseOnOverride::MapItem
        | VanillaItemUseOnOverride::PlaceOnWaterBlockItem
        | VanillaItemUseOnOverride::PotionItem
        | VanillaItemUseOnOverride::SolidBucketItem => GeneratedItemUseOnRoute::ClientOwned,
        VanillaItemUseOnOverride::ArmorStandItem
        | VanillaItemUseOnOverride::EndCrystalItem
        | VanillaItemUseOnOverride::FireworkRocketItem
        | VanillaItemUseOnOverride::HangingEntityItem
        | VanillaItemUseOnOverride::MinecartItem
        | VanillaItemUseOnOverride::SpawnEggItem => GeneratedItemUseOnRoute::EntitySpawning,
    }
}

impl LocalInteractionSuccess {
    fn start_consuming(hand: InteractionHand, consumable: &ConsumableSummary) -> Self {
        if consumable.consume_ticks() > 0 {
            Self::consume(true)
        } else {
            Self {
                sound_action: Some(LocalInteractionSoundAction::Consume { hand }),
                particle_action: Some(LocalInteractionParticleAction::Consume { hand, count: 16 }),
                ..Self::consume(false)
            }
        }
    }

    const fn consume(starts_using_item: bool) -> Self {
        Self {
            swing_source: LocalInteractionSwingSource::None,
            starts_using_item,
            ..Self::client_item_interaction()
        }
    }

    const fn consume_without_item() -> Self {
        Self {
            was_item_interaction: false,
            ..Self::consume(false)
        }
    }

    const fn server_item_interaction() -> Self {
        Self {
            swing_source: LocalInteractionSwingSource::Server,
            ..Self::client_item_interaction()
        }
    }

    const fn client_without_item() -> Self {
        Self {
            was_item_interaction: false,
            ..Self::client_item_interaction()
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

    const fn block_place(
        pos: BlockPos,
        sound_resource_id: &'static str,
        volume: f32,
        pitch: f32,
    ) -> Self {
        Self {
            sound_action: Some(LocalInteractionSoundAction::BlockPlace {
                pos,
                sound_resource_id,
                volume_bits: volume.to_bits(),
                pitch_bits: pitch.to_bits(),
            }),
            ..Self::client_item_interaction()
        }
    }

    const fn block_place_from_profile(pos: BlockPos, block_type_id: i32) -> Self {
        Self {
            sound_action: Some(LocalInteractionSoundAction::BlockPlaceFromProfile {
                pos,
                block_type_id,
            }),
            ..Self::client_item_interaction()
        }
    }

    const fn client_item_interaction_with_drop(dropped: bool) -> Self {
        Self {
            extra_swing: if dropped {
                Some(InteractionHand::MainHand)
            } else {
                None
            },
            ..Self::client_item_interaction()
        }
    }

    const fn ignition(sound_action: Option<LocalInteractionSoundAction>) -> Self {
        Self {
            sound_action,
            ..Self::client_item_interaction()
        }
    }

    const fn instrument_use(hand: InteractionHand) -> Self {
        Self {
            sound_action: Some(LocalInteractionSoundAction::InstrumentUse { hand }),
            ..Self::consume(true)
        }
    }

    const fn bottle_fill(dropped: bool) -> Self {
        Self {
            sound_action: Some(LocalInteractionSoundAction::BottleFill),
            ..Self::client_item_interaction_with_drop(dropped)
        }
    }

    const fn beehive_bottle_fill(dropped: bool) -> Self {
        Self {
            sound_action: Some(LocalInteractionSoundAction::BeehiveBottleFill),
            ..Self::bottle_fill(dropped)
        }
    }

    const fn spyglass_use() -> Self {
        Self {
            sound_action: Some(LocalInteractionSoundAction::SpyglassUse),
            ..Self::consume(true)
        }
    }

    const fn bundle_use() -> Self {
        Self {
            starts_using_item: true,
            ..Self::client_item_interaction()
        }
    }
}

impl WorldStore {
    /// Predicts one native no-target use after the caller has allocated the
    /// serverbound interaction sequence that owns any local block mutation.
    pub fn predict_local_item_use_with_pov_and_sequence(
        &mut self,
        hand: InteractionHand,
        pov: Option<LocalItemUsePovResult>,
        sequence: i32,
        secondary_use_active: bool,
    ) -> LocalItemUsePrediction {
        self.predict_local_item_use_with_pov_context(
            hand,
            pov,
            Some(sequence),
            secondary_use_active,
        )
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
        self.predict_local_block_use_with_hit(
            pos,
            face,
            face_center_cursor(face),
            false,
            hand,
            secondary_use_active,
            sequence,
        )
    }

    fn predict_local_tool_item_use_on(
        &mut self,
        owner: VanillaItemUseOnOverride,
        hand: InteractionHand,
        context: LocalBlockUseContext,
    ) -> Option<LocalTargetUsePrediction> {
        let block = self.block_view(context.pos)?;
        let block_name = block.block_name?;
        let block_type = vanilla_block_type_for_resource_id(block_name)?;
        let properties = block.block_properties;
        let success_with_sound = |sound_action: Option<LocalInteractionSoundAction>| {
            LocalTargetUsePrediction::Result(LocalInteractionResult::Success(
                LocalInteractionSuccess {
                    sound_action,
                    ..LocalInteractionSuccess::client_item_interaction()
                },
            ))
        };
        let success = || success_with_sound(None);
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
                let (transform, sound_resource_id, local_level_event) = if let Some(transform) =
                    find_block_transform(VANILLA_AXE_STRIPPABLES, block_type.id)
                {
                    (transform, "minecraft:item.axe.strip", None)
                } else if let Some(transform) =
                    find_block_transform(VANILLA_AXE_SCRAPEABLES, block_type.id)
                {
                    (transform, "minecraft:item.axe.scrape", Some(3005))
                } else if let Some(transform) =
                    find_block_transform(VANILLA_AXE_WAX_OFF, block_type.id)
                {
                    (transform, "minecraft:item.axe.wax_off", Some(3004))
                } else {
                    return Some(pass());
                };
                if !self.predict_local_transformed_block_state(
                    context,
                    transform,
                    block.block_state_id,
                ) {
                    return Some(LocalTargetUsePrediction::UnsupportedItemUseOn(owner));
                }
                if let Some(event_type) = local_level_event {
                    self.push_pending_local_level_event(bbb_protocol::packets::LevelEvent {
                        event_type,
                        pos: context.pos,
                        data: 0,
                        global: false,
                    });
                }
                Some(success_with_sound(Some(
                    LocalInteractionSoundAction::BlockPositioned {
                        pos: context.pos,
                        sound_resource_id,
                        volume_bits: 1.0_f32.to_bits(),
                        pitch_bits: 1.0_f32.to_bits(),
                    },
                )))
            }
            VanillaItemUseOnOverride::HoneycombItem => {
                let Some(transform) =
                    find_block_transform(VANILLA_HONEYCOMB_WAXABLES, block_type.id)
                else {
                    return Some(pass());
                };
                if !self.predict_local_transformed_block_state(
                    context,
                    transform,
                    block.block_state_id,
                ) || (!self
                    .local_player()
                    .abilities
                    .is_some_and(|abilities| abilities.instabuild)
                    && !self.shrink_local_item_in_hand(hand, 1))
                {
                    return Some(LocalTargetUsePrediction::UnsupportedItemUseOn(owner));
                }
                self.push_pending_local_level_event(bbb_protocol::packets::LevelEvent {
                    event_type: 3003,
                    pos: context.pos,
                    data: 0,
                    global: false,
                });
                Some(success_with_sound(Some(
                    LocalInteractionSoundAction::BlockPositioned {
                        pos: context.pos,
                        sound_resource_id: "minecraft:item.honeycomb.wax_on",
                        volume_bits: 1.0_f32.to_bits(),
                        pitch_bits: 1.0_f32.to_bits(),
                    },
                )))
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
                Some(success_with_sound(Some(
                    LocalInteractionSoundAction::BlockPositioned {
                        pos: context.pos,
                        sound_resource_id: "minecraft:item.hoe.till",
                        volume_bits: 1.0_f32.to_bits(),
                        pitch_bits: 1.0_f32.to_bits(),
                    },
                )))
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
                let Some(predicted_block_state_id) = vanilla_block_state_with_property(
                    block.block_state_id,
                    "age",
                    &max_age.to_string(),
                ) else {
                    return Some(LocalTargetUsePrediction::UnsupportedItemUseOn(owner));
                };
                if !self.predict_local_block_state(
                    context.pos,
                    predicted_block_state_id,
                    context.sequence,
                ) {
                    return Some(LocalTargetUsePrediction::UnsupportedItemUseOn(owner));
                }
                Some(success_with_sound(Some(
                    LocalInteractionSoundAction::BlockPositioned {
                        pos: context.pos,
                        sound_resource_id: "minecraft:block.growing_plant.crop",
                        volume_bits: 1.0_f32.to_bits(),
                        pitch_bits: 1.0_f32.to_bits(),
                    },
                )))
            }
            VanillaItemUseOnOverride::ShovelItem => {
                if context.face == ProtocolDirection::Down {
                    return Some(pass());
                }
                let flattenable = find_block_transform(VANILLA_SHOVEL_FLATTENABLES, block_type.id);
                if flattenable.is_some() && self.local_block_is_air(above(context.pos))? {
                    return Some(success_with_sound(Some(
                        LocalInteractionSoundAction::BlockPositioned {
                            pos: context.pos,
                            sound_resource_id: "minecraft:item.shovel.flatten",
                            volume_bits: 1.0_f32.to_bits(),
                            pitch_bits: 1.0_f32.to_bits(),
                        },
                    )));
                }
                let lit_campfire = block_type.use_item_on_override
                    == Some(VanillaBlockUseItemOnOverride::CampfireBlock)
                    && properties.get("lit").is_some_and(|value| value == "true");
                Some(if lit_campfire { success() } else { pass() })
            }
            _ => None,
        }
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
            if !self.content_registry().is_vanilla_only()
                && self
                    .content_registry()
                    .entity_by_id(entity_type_id)
                    .is_some()
            {
                if crate::entities::content_facts::entity_metadata_is_living(
                    entity_type_id,
                    self.content_registry(),
                ) {
                    if let Some(stack) = self.local_item_in_hand(hand) {
                        return self.predict_local_item_stack_interact_living_entity(
                            entity_id,
                            entity_type_id,
                            stack,
                        );
                    }
                }
                return LocalTargetUsePrediction::Result(LocalInteractionResult::Pass);
            }
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
        } else if let Some(result) = self.predict_local_minecart_interact(
            owner,
            entity_id,
            hand,
            held_item_id,
            secondary_use_active,
        ) {
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
            if !self.content_registry().is_vanilla_only()
                && self.content_registry().item_by_id(item_id).is_some()
            {
                return LocalTargetUsePrediction::Result(LocalInteractionResult::Pass);
            }
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
                if health <= 0.0 || health.is_nan() {
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
                        LocalInteractionSuccess {
                            sound_action: Some(LocalInteractionSoundAction::EntityPositioned {
                                entity_id,
                                sound_resource_id: "minecraft:item.dye.use",
                                volume_bits: 1.0_f32.to_bits(),
                                pitch_bits: 1.0_f32.to_bits(),
                            }),
                            ..LocalInteractionSuccess::client_item_interaction()
                        },
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
                let patch = self.items.component_source(
                    stack.item_id,
                    &stack.component_patch,
                    VANILLA_DATA_COMPONENT_CUSTOM_NAME_TYPE_ID,
                );
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
}

fn face_center_cursor(face: ProtocolDirection) -> [f64; 3] {
    match face {
        ProtocolDirection::Down => [0.5, 0.0, 0.5],
        ProtocolDirection::Up => [0.5, 1.0, 0.5],
        ProtocolDirection::North => [0.5, 0.5, 0.0],
        ProtocolDirection::South => [0.5, 0.5, 1.0],
        ProtocolDirection::West => [0.0, 0.5, 0.5],
        ProtocolDirection::East => [1.0, 0.5, 0.5],
    }
}

fn protocol_direction_axis(direction: ProtocolDirection) -> char {
    match direction {
        ProtocolDirection::West | ProtocolDirection::East => 'x',
        ProtocolDirection::Down | ProtocolDirection::Up => 'y',
        ProtocolDirection::North | ProtocolDirection::South => 'z',
    }
}

fn horizontal_property_axis(direction: &str) -> Option<char> {
    match direction {
        "west" | "east" => Some('x'),
        "north" | "south" => Some('z'),
        _ => None,
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
    use crate::vanilla_entity_metadata_hanging_direction_accessor_for_entity_type;
    use bbb_protocol::{
        block_types::{
            vanilla_block_type, vanilla_block_type_for_resource_id, VanillaBonemealTargetOverride,
            VanillaBucketPickupOverride, VanillaCanSurviveOverride,
            VANILLA_BLOCK_CRIMSON_NYLIUM_ID, VANILLA_BLOCK_DIRT_ID, VANILLA_BLOCK_FROGSPAWN_ID,
            VANILLA_BLOCK_ICE_ID, VANILLA_BLOCK_LILY_PAD_ID, VANILLA_BLOCK_RAIL_ID,
            VANILLA_BLOCK_STONE_ID, VANILLA_BLOCK_TYPES, VANILLA_BLOCK_WHITE_BANNER_ID,
        },
        builtin_enum_component_values::VANILLA_DYE_COLOR_RED_ID,
        data_component_types::{
            VANILLA_DATA_COMPONENT_CUSTOM_NAME_TYPE_ID, VANILLA_DATA_COMPONENT_DAMAGE_TYPE_ID,
            VANILLA_DATA_COMPONENT_DYE_TYPE_ID, VANILLA_DATA_COMPONENT_ENCHANTMENTS_TYPE_ID,
            VANILLA_DATA_COMPONENT_EQUIPPABLE_TYPE_ID, VANILLA_DATA_COMPONENT_FOOD_TYPE_ID,
            VANILLA_DATA_COMPONENT_INSTRUMENT_TYPE_ID,
            VANILLA_DATA_COMPONENT_LODESTONE_TRACKER_TYPE_ID,
            VANILLA_DATA_COMPONENT_MAX_DAMAGE_TYPE_ID,
            VANILLA_DATA_COMPONENT_PAINTING_VARIANT_TYPE_ID,
            VANILLA_DATA_COMPONENT_POTION_CONTENTS_TYPE_ID,
            VANILLA_DATA_COMPONENT_USE_COOLDOWN_TYPE_ID,
        },
        entity_data_serializers::{
            VANILLA_ENTITY_DATA_SERIALIZER_BOOLEAN_ID, VANILLA_ENTITY_DATA_SERIALIZER_BYTE_ID,
            VANILLA_ENTITY_DATA_SERIALIZER_DIRECTION_ID, VANILLA_ENTITY_DATA_SERIALIZER_FLOAT_ID,
            VANILLA_ENTITY_DATA_SERIALIZER_INT_ID, VANILLA_ENTITY_DATA_SERIALIZER_ITEM_STACK_ID,
            VANILLA_ENTITY_DATA_SERIALIZER_OPTIONAL_BLOCK_POS_ID,
            VANILLA_ENTITY_DATA_SERIALIZER_OPTIONAL_LIVING_ENTITY_REFERENCE_ID,
            VANILLA_ENTITY_DATA_SERIALIZER_PAINTING_VARIANT_ID,
        },
        entity_types::{
            VANILLA_ENTITY_TYPE_AREA_EFFECT_CLOUD_ID, VANILLA_ENTITY_TYPE_ARMOR_STAND_ID,
            VANILLA_ENTITY_TYPE_BOGGED_ID, VANILLA_ENTITY_TYPE_CAMEL_HUSK_ID,
            VANILLA_ENTITY_TYPE_CHEST_MINECART_ID, VANILLA_ENTITY_TYPE_CHICKEN_ID,
            VANILLA_ENTITY_TYPE_COMMAND_BLOCK_MINECART_ID, VANILLA_ENTITY_TYPE_COPPER_GOLEM_ID,
            VANILLA_ENTITY_TYPE_COW_ID, VANILLA_ENTITY_TYPE_CREEPER_ID,
            VANILLA_ENTITY_TYPE_ENDER_DRAGON_ID, VANILLA_ENTITY_TYPE_FURNACE_MINECART_ID,
            VANILLA_ENTITY_TYPE_GLOW_ITEM_FRAME_ID, VANILLA_ENTITY_TYPE_HOPPER_MINECART_ID,
            VANILLA_ENTITY_TYPE_INTERACTION_ID, VANILLA_ENTITY_TYPE_IRON_GOLEM_ID,
            VANILLA_ENTITY_TYPE_ITEM_FRAME_ID, VANILLA_ENTITY_TYPE_ITEM_ID,
            VANILLA_ENTITY_TYPE_LEASH_KNOT_ID, VANILLA_ENTITY_TYPE_MANNEQUIN_ID,
            VANILLA_ENTITY_TYPE_MINECART_ID, VANILLA_ENTITY_TYPE_MOOSHROOM_ID,
            VANILLA_ENTITY_TYPE_NAUTILUS_ID, VANILLA_ENTITY_TYPE_OAK_BOAT_ID,
            VANILLA_ENTITY_TYPE_OAK_CHEST_BOAT_ID, VANILLA_ENTITY_TYPE_PAINTING_ID,
            VANILLA_ENTITY_TYPE_PLAYER_ID, VANILLA_ENTITY_TYPE_SHEEP_ID,
            VANILLA_ENTITY_TYPE_SNOW_GOLEM_ID, VANILLA_ENTITY_TYPE_SQUID_ID,
            VANILLA_ENTITY_TYPE_VILLAGER_ID, VANILLA_ENTITY_TYPE_WANDERING_TRADER_ID,
            VANILLA_ENTITY_TYPE_ZOMBIE_HORSE_ID, VANILLA_ENTITY_TYPE_ZOMBIE_ID,
            VANILLA_ENTITY_TYPE_ZOMBIE_NAUTILUS_ID,
        },
        equipment_slots::{EquipmentSlot, VANILLA_EQUIPMENT_SLOT_BODY_ID},
        game_types::{VANILLA_GAME_TYPE_CREATIVE_ID, VANILLA_GAME_TYPE_SURVIVAL_ID},
        item_types::{
            VanillaItemInteractLivingEntityOverride, VanillaItemUseOnOverride,
            VANILLA_ITEM_APPLE_ID, VANILLA_ITEM_ARMOR_STAND_ID, VANILLA_ITEM_BONE_MEAL_ID,
            VANILLA_ITEM_BRUSH_ID, VANILLA_ITEM_BUCKET_ID, VANILLA_ITEM_CHICKEN_SPAWN_EGG_ID,
            VANILLA_ITEM_CHORUS_FRUIT_ID, VANILLA_ITEM_COAL_ID, VANILLA_ITEM_COD_BUCKET_ID,
            VANILLA_ITEM_COMPASS_ID, VANILLA_ITEM_DEBUG_STICK_ID, VANILLA_ITEM_ENDER_EYE_ID,
            VANILLA_ITEM_END_CRYSTAL_ID, VANILLA_ITEM_FILLED_MAP_ID,
            VANILLA_ITEM_FIREWORK_ROCKET_ID, VANILLA_ITEM_FIRE_CHARGE_ID,
            VANILLA_ITEM_FLINT_AND_STEEL_ID, VANILLA_ITEM_FROGSPAWN_ID,
            VANILLA_ITEM_GLASS_BOTTLE_ID, VANILLA_ITEM_GLOW_ITEM_FRAME_ID,
            VANILLA_ITEM_GOAT_HORN_ID, VANILLA_ITEM_HONEYCOMB_ID, VANILLA_ITEM_IRON_AXE_ID,
            VANILLA_ITEM_IRON_HOE_ID, VANILLA_ITEM_IRON_SHOVEL_ID, VANILLA_ITEM_ITEM_FRAME_ID,
            VANILLA_ITEM_LAVA_BUCKET_ID, VANILLA_ITEM_LEAD_ID, VANILLA_ITEM_LILY_PAD_ID,
            VANILLA_ITEM_MINECART_ID, VANILLA_ITEM_NAME_TAG_ID, VANILLA_ITEM_OAK_BOAT_ID,
            VANILLA_ITEM_PAINTING_ID, VANILLA_ITEM_POPPY_ID, VANILLA_ITEM_POTION_ID,
            VANILLA_ITEM_POWDER_SNOW_BUCKET_ID, VANILLA_ITEM_RED_DYE_ID,
            VANILLA_ITEM_RED_MUSHROOM_ID, VANILLA_ITEM_SADDLE_ID, VANILLA_ITEM_SHEARS_ID,
            VANILLA_ITEM_SHIELD_ID, VANILLA_ITEM_STONE_ID, VANILLA_ITEM_TRIDENT_ID,
            VANILLA_ITEM_TYPES, VANILLA_ITEM_WATER_BUCKET_ID, VANILLA_ITEM_WHEAT_ID,
            VANILLA_ITEM_WHEAT_SEEDS_ID,
        },
        packets::{
            AddEntity, AdventureModePredicateDataSummary, BlockHitResult,
            BlockPos as ProtocolBlockPos, BlockPredicateSummary, BlockUpdate,
            CommonPlayerSpawnInfo, Cooldown, DataComponentPatchSummary, EntityDataValue,
            EntityDataValueKind, EntityEvent, EquipmentSlotUpdate, EquippableSummary, GameProfile,
            GameType, HolderSetSummary, InstrumentSummary, ItemEnchantmentSummary,
            ItemStackSummary, PaintingVariantSummary, PlayLogin, PlayerHealth, PlayerInfoAction,
            PlayerInfoEntry, PlayerInfoUpdate, RegistryData, RegistryDataEntry, RegistryTags,
            SetChunkCacheCenter, SetEntityData, SetEntityLink, SetEquipment, SetPassengers,
            SetPlayerInventory, SoundEventSummary, TagNetworkPayload, UpdateEnabledFeatures,
            UpdateTags, Vec3d,
        },
    };
    use std::collections::{BTreeMap, BTreeSet};
    use uuid::Uuid;

    mod block_use;
    mod entity_interact;
    mod fixtures;
    mod item_use;
    use fixtures::*;

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

        world.set_default_item_component_codec_values(BTreeMap::from([(
            VANILLA_ITEM_CHORUS_FRUIT_ID,
            BTreeMap::from([
                (
                    VANILLA_DATA_COMPONENT_CONSUMABLE_TYPE_ID,
                    serde_json::json!({}),
                ),
                (
                    VANILLA_DATA_COMPONENT_USE_COOLDOWN_TYPE_ID,
                    serde_json::json!({"seconds": 1.0}),
                ),
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
    fn generated_item_use_on_owners_are_exhaustively_routed() {
        let mut route_counts = [0_usize; 3];
        let mut generated_rows = 0;
        for owner in VANILLA_ITEM_TYPES
            .iter()
            .filter_map(|item| item.use_on_override)
        {
            generated_rows += 1;
            route_counts[match generated_item_use_on_route(owner) {
                GeneratedItemUseOnRoute::Tool => 0,
                GeneratedItemUseOnRoute::ClientOwned => 1,
                GeneratedItemUseOnRoute::EntitySpawning => 2,
            }] += 1;
        }

        assert_eq!(generated_rows, 1_160);
        assert!(route_counts.into_iter().all(|count| count > 0));
    }

    #[test]
    fn item_use_pov_queries_follow_generated_owner_fluid_modes() {
        let mut world = WorldStore::new();
        for (item_id, fluid_mode) in [
            (VANILLA_ITEM_ENDER_EYE_ID, LocalItemUsePovFluidMode::None),
            (
                VANILLA_ITEM_GLASS_BOTTLE_ID,
                LocalItemUsePovFluidMode::SourceOnly,
            ),
            (
                VANILLA_ITEM_CHICKEN_SPAWN_EGG_ID,
                LocalItemUsePovFluidMode::SourceOnly,
            ),
            (VANILLA_ITEM_OAK_BOAT_ID, LocalItemUsePovFluidMode::Any),
            (VANILLA_ITEM_BUCKET_ID, LocalItemUsePovFluidMode::SourceOnly),
            (VANILLA_ITEM_WATER_BUCKET_ID, LocalItemUsePovFluidMode::None),
            (VANILLA_ITEM_COD_BUCKET_ID, LocalItemUsePovFluidMode::None),
            (
                VANILLA_ITEM_LILY_PAD_ID,
                LocalItemUsePovFluidMode::SourceOnly,
            ),
            (
                VANILLA_ITEM_FROGSPAWN_ID,
                LocalItemUsePovFluidMode::SourceOnly,
            ),
        ] {
            set_main_hand(&mut world, item_id);
            assert_eq!(
                world.local_item_use_pov_query(InteractionHand::MainHand),
                Some(LocalItemUsePovQuery {
                    fluid_mode,
                    max_distance: 4.5,
                })
            );
        }

        set_main_hand(&mut world, VANILLA_ITEM_STONE_ID);
        assert_eq!(
            world.local_item_use_pov_query(InteractionHand::MainHand),
            None
        );
    }

    #[test]
    fn trident_release_obeys_hold_threshold_and_keeps_throw_damage_server_owned() {
        let mut world = WorldStore::new();
        set_main_hand(&mut world, VANILLA_ITEM_TRIDENT_ID);
        let original = world
            .local_item_in_hand(InteractionHand::MainHand)
            .unwrap()
            .clone();

        world.set_local_using_item_with_hand(true, InteractionHand::MainHand);
        world.advance_local_using_item_ticks(9);
        assert!(world.release_local_using_item());
        assert!(world.local_player_auto_spin_attack().is_none());
        assert!(world
            .take_pending_local_interaction_sound_actions()
            .is_empty());

        world.set_local_using_item_with_hand(true, InteractionHand::MainHand);
        world.advance_local_using_item_ticks(10);
        assert!(world.release_local_using_item());
        assert!(world.local_player_auto_spin_attack().is_none());
        assert!(world
            .take_pending_local_interaction_sound_actions()
            .is_empty());
        assert_eq!(
            world.local_item_in_hand(InteractionHand::MainHand),
            Some(&original)
        );
    }

    #[test]
    fn secondary_block_use_reaches_item_placement_without_loaded_target() {
        let mut world = WorldStore::new();
        set_main_hand(&mut world, VANILLA_ITEM_STONE_ID);
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
            LocalTargetUsePrediction::UnsupportedEntityInteract(VanillaEntityOwner::AbstractCow)
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
                    tag: "minecraft:sheep_food".to_string(),
                    entries: Vec::new(),
                }],
            }],
        });
        world.set_default_item_component_codec_values(BTreeMap::from([(
            VANILLA_ITEM_RED_DYE_ID,
            BTreeMap::from([(VANILLA_DATA_COMPONENT_DYE_TYPE_ID, serde_json::json!("red"))]),
        )]));

        set_main_hand(&mut world, VANILLA_ITEM_RED_DYE_ID);
        assert_eq!(
            world.predict_local_entity_interaction(4, InteractionHand::MainHand, false),
            LocalTargetUsePrediction::UnsupportedItemInteractLivingEntity(
                VanillaItemInteractLivingEntityOverride::DyeItem
            )
        );
        world.rebuild_item_component_defaults().unwrap();

        let pass = LocalTargetUsePrediction::Result(LocalInteractionResult::Pass);
        let client_success = LocalTargetUsePrediction::Result(LocalInteractionResult::Success(
            LocalInteractionSuccess::client_item_interaction(),
        ));
        let dye_success = LocalTargetUsePrediction::Result(LocalInteractionResult::Success(
            LocalInteractionSuccess {
                sound_action: Some(LocalInteractionSoundAction::EntityPositioned {
                    entity_id: 4,
                    sound_resource_id: "minecraft:item.dye.use",
                    volume_bits: 1.0_f32.to_bits(),
                    pitch_bits: 1.0_f32.to_bits(),
                }),
                ..LocalInteractionSuccess::client_item_interaction()
            },
        ));
        assert_eq!(
            world.predict_local_entity_interaction(4, InteractionHand::MainHand, false),
            dye_success
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
                    tag: "minecraft:sheep_food".to_string(),
                    entries: vec![VANILLA_ITEM_RED_DYE_ID],
                }],
            }],
        });
        assert_eq!(
            world.predict_local_entity_interaction(4, InteractionHand::MainHand, false),
            LocalTargetUsePrediction::Result(LocalInteractionResult::Success(
                LocalInteractionSuccess::consume(false)
            ))
        );
        world.apply_update_tags(UpdateTags {
            registries: vec![RegistryTags {
                registry: "minecraft:item".to_string(),
                tags: vec![TagNetworkPayload {
                    tag: "minecraft:sheep_food".to_string(),
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
                added_type_ids: vec![VANILLA_DATA_COMPONENT_CUSTOM_NAME_TYPE_ID],
                custom_name: Some("Woolly".to_string()),
                ..DataComponentPatchSummary::default()
            }
            .into(),
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

        world.set_default_item_component_codec_values(BTreeMap::from([
            (
                VANILLA_ITEM_RED_DYE_ID,
                BTreeMap::from([(VANILLA_DATA_COMPONENT_DYE_TYPE_ID, serde_json::json!("red"))]),
            ),
            (
                VANILLA_ITEM_NAME_TAG_ID,
                BTreeMap::from([(
                    VANILLA_DATA_COMPONENT_EQUIPPABLE_TYPE_ID,
                    serde_json::json!({"slot": "chest"}),
                )]),
            ),
        ]));
        world.rebuild_item_component_defaults().unwrap();
        assert_eq!(
            world.predict_local_entity_interaction(4, InteractionHand::OffHand, false),
            client_success
        );

        let patched_equippable = equippable(false);
        world.apply_set_player_inventory(SetPlayerInventory {
            slot: VANILLA_PLAYER_INVENTORY_OFFHAND_SLOT,
            item: ItemStackSummary {
                component_patch: DataComponentPatchSummary {
                    added_type_ids: vec![
                        VANILLA_DATA_COMPONENT_CUSTOM_NAME_TYPE_ID,
                        VANILLA_DATA_COMPONENT_EQUIPPABLE_TYPE_ID,
                    ],
                    custom_name: Some("Equippable".to_string()),
                    equippable: Some(patched_equippable),
                    ..DataComponentPatchSummary::default()
                }
                .into(),
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
                    added_type_ids: vec![
                        VANILLA_DATA_COMPONENT_CUSTOM_NAME_TYPE_ID,
                        VANILLA_DATA_COMPONENT_EQUIPPABLE_TYPE_ID,
                    ],
                    custom_name: Some("Equippable".to_string()),
                    equippable: Some(patched_equippable),
                    ..DataComponentPatchSummary::default()
                }
                .into(),
                ..named_name_tag
            },
        });
        assert_eq!(
            world.predict_local_entity_interaction(4, InteractionHand::OffHand, false),
            client_success
        );
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
            LocalTargetUsePrediction::UnsupportedEntityInteract(VanillaEntityOwner::AbstractCow)
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
    fn iron_golem_interaction_applies_client_heal_before_selecting_result() {
        let mut world = WorldStore::new();
        add_entity(&mut world, 92, VANILLA_ENTITY_TYPE_IRON_GOLEM_ID);
        assert_eq!(
            world.predict_local_entity_interaction(92, InteractionHand::MainHand, false),
            LocalTargetUsePrediction::Result(LocalInteractionResult::Pass)
        );

        let mut iron_ingots = stack(VANILLA_ITEM_IRON_INGOT_ID);
        iron_ingots.count = 4;
        assert!(world.apply_set_player_inventory(SetPlayerInventory {
            slot: 0,
            item: iron_ingots,
        }));
        for expected_health in [26.0, 51.0, 76.0, 100.0] {
            assert_eq!(
                world.predict_local_entity_interaction(92, InteractionHand::MainHand, false),
                LocalTargetUsePrediction::Result(LocalInteractionResult::Success(
                    LocalInteractionSuccess::client_item_interaction()
                ))
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
}
