//! Live server entities, movement, physics, AI, and tracking.
//!
//! Each dimension owns an [`EntityStore`] holding live non-player entities,
//! player body views, spatial section indices, passenger trees, and lifecycle
//! events consumed by [`EntityTracker`]. Entity tracking uses the generated
//! 26.1 catalog's tracking range and update interval.

pub mod abstract_arrow;
pub mod ai;
pub(crate) mod allay;
mod ambient_sound;
mod animal_breeding;
mod animal_feeding;
pub mod area_effect_cloud;
pub(crate) mod armadillo;
pub mod armor_stand;
pub mod arrow;
pub mod attributes;
pub(crate) mod axolotl;
pub(crate) mod bee;
mod blaze;
mod block_spawn;
mod cat_sounds;
pub(crate) mod parrot_sounds;
pub(crate) mod sound_variants;
pub(crate) mod warden;
pub use block_spawn::{BlockOriginSpawnResult, BlockPatternMob};
pub mod block_drops;
pub mod boat;
pub(crate) mod bogged;
pub(crate) mod bucketable;
pub mod chest_boat;
mod collar;
pub mod command_data;
mod component_getter;
pub(crate) mod consume_effect;
pub(crate) mod conversion;
pub(crate) mod copper_golem;
mod cow_milk;
mod creaking;
pub mod damage;
mod daylight_burn;
pub mod death_protection;
pub mod display;
mod dolphin;
pub mod egg;
pub mod enchanted_damage;
pub mod end_crystal;
pub(crate) mod ender_dragon;
pub mod ender_pearl;
pub(crate) mod enderman;
mod endermite;
mod entity_reference;
mod environment;
pub mod equipment_modifiers;
mod equipment_wear;
pub mod evoker_fangs;
pub mod experience;
pub mod experience_bottle;
pub mod experience_orb;
mod external_player_vehicle;
pub mod eye_of_ender;
pub mod falling_block;
pub mod firework_rocket;
pub(crate) mod fish;
pub mod fishing_bobber;
mod goat_milk;
pub(crate) mod implicit_components;
pub(crate) mod inside_blocks;
#[cfg(test)]
mod test_items;
pub use fish::{FishSpawnKind, SalmonVariant, SchoolingFishState, TropicalFishVariant};
pub(crate) use inside_blocks::{
    apply_nonliving_inside_block_effects, apply_nonliving_inside_block_effects_between,
    apply_nonliving_inside_block_effects_for_last_movements,
};
pub(crate) mod fox;
pub mod game_event_listener;
pub mod general_living;
mod ghast;
mod gift_loot;
mod goat;
mod guardian;
mod happy_ghast;

pub(crate) mod camel;
pub mod hanging;
pub(crate) mod hoglin;
pub(crate) mod horse_phenotype;
pub(crate) mod horse_ride;
pub mod hurting_projectile;
pub mod interaction;
mod iron_golem;
pub mod item;
pub(crate) mod item_components;
pub(crate) mod llama;
pub use item_components::equippable_damage_on_hurt;
pub mod item_slots;
pub(crate) mod item_steering;
pub mod kill;
pub mod leash;
pub mod level;
pub mod lightning_bolt;
pub mod living;
mod living_internal_inventory;
pub mod llama_spit;
pub mod location_effects;
pub(crate) mod mace;
mod mannequin;
pub mod marker;
pub mod merchant;
pub mod minecart;
pub mod minecart_command_block;
pub mod minecart_container;
mod minecart_hopper;
pub mod minecart_spawner;
pub mod minecart_tnt;
pub mod mob_death_loot;
mod mob_pickup;
pub(crate) mod name_tag;
pub(crate) use name_tag::custom_name_from_stack;
pub mod natural_spawn;
pub mod natural_spawn_facts_26_1;
pub mod ominous_item_spawner;
mod panda;
pub(crate) mod passenger;
mod phantom;
mod pick;
pub(crate) mod piglin;
pub mod player_collision;
pub mod plugin_mob;
pub mod portal;
pub(crate) mod push;
pub(crate) use push::push_entities_from_session_player;
pub use push::{push, PlayerPushBody};
pub(crate) mod nautilus;
mod nautilus_interact;
pub mod projectile_action;
pub mod projectile_hit;
pub mod projectile_shoot;
pub(crate) mod projectile_weapon;
pub(crate) mod rabbit;
pub(crate) mod raider;
pub(crate) mod ravager;
pub mod save;
mod shear_equipment;
pub(crate) mod shearing_loot;
mod sheep;
pub(crate) mod sheep_shear;
pub(crate) mod shulker;
pub mod shulker_bullet;
mod skeleton;
pub(crate) mod skeleton_horse;
pub(crate) mod slime;
pub(crate) mod sniffer;
pub(crate) mod snow_golem;
pub mod snowball;
mod spawn_egg_offspring;
pub(crate) mod spear;
pub mod spectral_arrow;
pub mod store;
mod strider;
pub mod summon;
mod tadpole;
mod taming;
pub(crate) mod throwable_projectile;
pub mod thrown_potion;
pub mod thunder;
pub mod tnt;
pub mod tracking;
pub mod trident;
mod turtle;
mod witch;
mod wither;
mod wolf;
pub(crate) mod zombie;

use bbb_protocol::entity_poses::VANILLA_ENTITY_POSE_STANDING_ID;
use bbb_protocol::entity_synched_data::VANILLA_ENTITY_NO_GRAVITY_DATA_ID;
use bbb_protocol::entity_types::vanilla_entity_type_dimensions;
use bbb_protocol::packets::{BlockPos, Vec3d};
use bbb_protocol::RichTextComponent;
use uuid::Uuid;

mod entity_base_tick;
mod physics_queries;
mod synched_fields;
mod tick_dispatch;
pub use tick_dispatch::apply_gravity;
#[cfg(test)]
use tick_dispatch::base_tick_without_store_for_test;
pub(crate) use tick_dispatch::{
    base_tick, base_tick_hurting_on_fire, check_below_world, handle_portal, is_below_world,
    process_portal_cooldown, tick_member, tick_one_entity_checked,
};
#[cfg(test)]
pub(crate) use tick_dispatch::{tick_entities, tick_one_entity};

pub(crate) mod mooshroom;
pub(crate) mod movement;
mod named_entity_drop;
pub use crate::entity::movement::{
    body_movement_effects, body_swim_sound_volume, limit_body_piston_movement,
    BodyCrystalSoundState, BodyMovementEffect, BodyMovementEmissionState, BodyMovementFacts,
    MovementEmission, PendingPistonMove,
};
pub(crate) use crate::entity::movement::{move_entity, move_entity_by_piston_pausing};

use crate::entity::store::section_of_block;
use crate::entity::supporting_block::set_on_ground_with_movement;

pub(crate) use crate::entity::ai::mob::navigation::ground_path_type_from_state;
pub use crate::entity::ai::mob::navigation::{GroundPath, GroundPathNode, GroundPathType};
pub use crate::entity::area_effect_cloud::{spawn_area_effect_cloud, AreaEffectCloudData};
pub use crate::entity::armor_stand::{
    spawn as spawn_armor_stand, ArmorStandData, ArmorStandInteraction, ArmorStandPose,
    InteractingPlayer, Rotations,
};
pub use crate::entity::arrow::{spawn_arrow, ArrowData, ArrowPickup};
pub use crate::entity::attributes::{
    AttributeInstance, AttributeMap, AttributeModifier, AttributeOperation, AttributeSupplier,
    PackedAttribute,
};
pub use crate::entity::boat::{spawn_boat, BoatData, BoatHurtSource, BoatStatus};
pub use crate::entity::chest_boat::{
    spawn_chest_boat, ChestBoatData, ChestBoatSlotError,
    CONTAINER_SIZE as CHEST_BOAT_CONTAINER_SIZE,
};
pub use crate::entity::command_data::EntityCommandDataError;
pub use crate::entity::creaking::CreakingHurtDecision;
pub use crate::entity::damage::{
    evaluate_stored_entity_damage, evaluate_stored_entity_damage_in_context, item_on_destroyed,
    item_resists_damage, ItemOnDestroyed, StoredAttackerViews, StoredDamageCausingEntity,
    StoredEntityDamageContext, StoredEntityDamageRules, StoredEntityDamageSource,
    StoredEntityHurtOutcome,
};
pub use crate::entity::display::{
    spawn_block_display, spawn_item_display, spawn_text_display, BlockDisplayData, DisplayData,
    ItemDisplayData, TextDisplayData,
};
pub use crate::entity::egg::{spawn_egg, EggData};
pub use crate::entity::end_crystal::{spawn_end_crystal, EndCrystalData};
pub use crate::entity::ender_pearl::{spawn_ender_pearl, EnderPearlData};
pub use crate::entity::entity_reference::{
    CrossLevelEntities, EntityReference, LastHurtByMob, ReferenceTarget,
};
pub use crate::entity::environment::water_splash::body_water_splash;
pub use crate::entity::environment::{
    body_freeze_damage_due, body_is_in_wall, body_swimming_state, can_body_freeze,
    scan_body_fluid_interaction, scan_body_fluid_interaction_with_current, tick_body_freeze,
    update_body_frost, FluidInteraction, FluidTag, TICKS_REQUIRED_TO_FREEZE,
};
pub use crate::entity::evoker_fangs::{spawn_evoker_fangs, EvokerFangsData};
pub use crate::entity::experience_bottle::{spawn_experience_bottle, ExperienceBottleData};
pub use crate::entity::experience_orb::{
    award as award_experience, player_touch as orb_player_touch, spawn_experience_orb,
    ExperienceOrbData, OrbTouch,
};
pub use crate::entity::eye_of_ender::{spawn_eye_of_ender, EyeOfEnderData};
pub(crate) use crate::entity::falling_block::falling_block_scheduled_tick;
pub use crate::entity::falling_block::{
    fallen_block_replacement, is_free, spawn_fallen_block, spawn_requested_fallen_block,
    FallingBlockData,
};
pub use crate::entity::firework_rocket::{spawn_firework_rocket, FireworkRocketData};
pub use crate::entity::fishing_bobber::{FishHookState, FishingBobberData, RetrieveOutcome};
pub use crate::entity::general_living::{
    brain_packed, configure_fox_target_goals, finalize_spawner_villager_type,
    resolve_spawner_registry_holders, schooling_fish_state, set_cat_spawn_persistence,
    set_piglin_baby, set_spawn_experience_reward, set_spawner_age, set_spawner_brain_memory,
    set_spawner_chicken_jockey, set_spawner_item, set_spawner_registry_value,
    set_spawner_simple_variant, set_tropical_fish_schooling, set_zoglin_baby, spawner_brain_memory,
    spawner_villager_finalized, tropical_fish_is_school, GeneralLivingData,
};
pub(crate) use crate::entity::general_living::{
    finalize_spawn, finalize_spawn_with, SpawnContext, SpawnGroupData, SpawnLevel, SpawnReason,
};
pub use crate::entity::hanging::{
    apply_interact as apply_hanging_interact, prepare_interact as prepare_hanging_interact,
    spawn_item_frame, spawn_leash_knot, spawn_painting, FramedMapAdmission, FramedMapFrame,
    HangHurtSource, HangInteract, ItemFrameData, LeashKnotData, PaintingData,
};
pub use crate::entity::hurting_projectile::dragon_fireball::{
    spawn_dragon_fireball, DragonFireballData,
};
pub use crate::entity::hurting_projectile::fireball::{spawn_large_fireball, LargeFireballData};
pub use crate::entity::hurting_projectile::small_fireball::{
    spawn_small_fireball, SmallFireballData,
};
pub use crate::entity::hurting_projectile::wind_charge::{
    spawn_breeze_wind_charge, spawn_wind_charge, WindChargeData,
};
pub use crate::entity::hurting_projectile::wither_skull::{
    set_dangerous as set_wither_skull_dangerous, spawn_wither_skull, WitherSkullData,
};
pub use crate::entity::hurting_projectile::{
    can_deflect_by_player_attack_with_registry_tag_view, deflect_by_player_attack,
    HurtingProjectileState, ProjectileState,
};
pub(crate) use crate::entity::inside_blocks::body_contact::BodyContactCursor;
pub use crate::entity::inside_blocks::body_contact::{
    accumulate_body_fall_distance, powder_snow_body_collision, record_body_movement,
    take_body_effect_movements, BlockContactGeometry, BodyBlockContact,
};
pub use crate::entity::inside_blocks::effects::{
    BlockEffectCollector, EnvironmentBlockKind, InsideBlockEffect, InsideBlockEffectType,
};
pub use crate::entity::inside_blocks::EntityMovement;
pub use crate::entity::interaction::{
    spawn_interaction, InteractionData, InteractionUse, PlayerAction,
};
pub use crate::entity::item::{
    spawn_advancement_reward_item, spawn_dispensed_item_entity, spawn_dropped_item_entity,
    spawn_fake_command_item, spawn_item_entity, spawn_player_dropped_item_entity, ItemEntityData,
};
pub use crate::entity::level::{
    EntityRegistryTagView, NearbyPlayer, PlayerHurtMemory, ScoreboardTeams, TargetLevelFacts,
};
pub use crate::entity::lightning_bolt::{spawn_lightning_bolt, LightningBoltData};
pub use crate::entity::living::{ActiveMobEffect, EntityEquipment, LivingEntityState};
pub use crate::entity::pick::EntityPickResult;
pub use crate::entity::spawn_egg_offspring::{
    held_is_spawn_egg, spawn_egg_entity_type_id, SpawnEggLevel,
};
pub use crate::entity::supporting_block::{
    body_on_pos, check_body_supporting_block, check_body_supporting_block_with_moving_pistons,
    SupportingBlockFacts,
};

/// Resolves a connected player's live `LivingEntity.canBeAffected` admission
/// against the exact registry-tag generation carried by a prepared action.
pub fn player_mob_effect_admitted_with_registry_tag_view(
    registry_tags: &EntityRegistryTagView,
    effect_resource_id: &str,
) -> bool {
    living::mob_effects::can_be_affected_with_registry_tag_view(
        registry_tags,
        "minecraft:player",
        effect_resource_id,
    )
}
pub use crate::entity::external_player_vehicle::{
    ExternalPlayerBoatAdmissionFacts, ExternalPlayerBoatCollisionProbe,
    ExternalPlayerBoatCollisionResolution, ExternalPlayerBoatControllerSnapshot,
    ExternalPlayerBoatCorrectionReason, ExternalPlayerBoatMoveFinalization,
    ExternalPlayerBoatMoveOutcome, ExternalPlayerBoatMoveRequest,
};
pub use crate::entity::llama_spit::{spawn_llama_spit, LlamaSpitData};
pub use crate::entity::marker::{spawn_marker, MarkerData};
pub use crate::entity::merchant::{MerchantItemCost, MerchantOffer, MerchantOffers};
pub use crate::entity::minecart::{
    add_furnace_minecart_fuel, apply_spawner_tick_decision, resolve_spawner_minecart_attempt,
    spawn_command_block_minecart, spawn_furnace_minecart, spawn_minecart, spawn_spawner_minecart,
    spawn_tnt_minecart, spawner_minecart_data_mut, FurnaceMinecartData, MinecartData,
    MinecartHurtSource,
};
pub use crate::entity::minecart_command_block::{
    begin_command_minecart_execution, edit_command_minecart, finish_command_minecart_execution,
    ActivatorRailDecision, CommandBlockExecutionSnapshot, CommandBlockMinecartData,
    CommandMinecartExecutionPlan, CommandMinecartRun, PendingCommandMinecartExecution,
};
pub use crate::entity::minecart_container::{
    spawn_minecart_container, MinecartContainerData, MinecartContainerKind,
    MinecartContainerSlotError, CHEST_CONTAINER_SIZE, HOPPER_CONTAINER_SIZE,
};
pub(crate) use crate::entity::minecart_spawner::check_spawn_placement_predicate;
pub use crate::entity::minecart_spawner::{
    difficulty_special_multiplier, effective_difficulty, equip_spawner_mob, finalize_spawner_mob,
    is_spawn_position_ok, prepare_spawn_request, AtomicSpawnPlan, CustomSpawnRules, EquipmentPlan,
    EquipmentSlotDropChance, PendingSpawnerMinecartAttempt, SpawnAttempt, SpawnEntityTree,
    SpawnFluid, SpawnHeightmap, SpawnPlacementEntry, SpawnPlacementEnvironment,
    SpawnPlacementProbe, SpawnPlacementType, SpawnPredicateKind, SpawnPredicateReason,
    SpawnRequest, SpawnerMinecartData, SpawnerRefusal, SpawnerSpawnData, SpawnerTickDecision,
    WeightedSpawnerSpawnData,
};
pub use crate::entity::minecart_tnt::{
    TntMinecartDamageSource, TntMinecartData, TntMinecartDestroyDecision,
    TntMinecartExplosionDecision, TntMinecartIgnitionSource, TntMinecartPrimeEffects,
    TntMinecartTickEffects,
};
pub use crate::entity::natural_spawn_facts_26_1::{
    vanilla_mob_category_spawn_facts_26_1, vanilla_natural_spawn_type_facts_26_1,
    vanilla_natural_spawn_type_facts_by_id_26_1, vanilla_natural_spawn_types_for_category_26_1,
    VanillaMobCategorySpawnFacts26_1, VanillaNaturalSpawnTypeFacts26_1,
    VANILLA_MOB_CATEGORY_SPAWN_FACTS_26_1, VANILLA_NATURAL_SPAWN_TYPE_FACTS_26_1,
};
pub use crate::entity::ominous_item_spawner::{spawn_ominous_item_spawner, OminousItemSpawnerData};
pub(crate) use crate::entity::passenger::{
    continue_entity_tick, continue_entity_tick_after_portal_refusal,
};
pub use crate::entity::passenger::{
    direct_controlling_player_with_registry_tags, EntityTickContinuation, EntityTickOutcome,
    ExternalPlayerCanRide, ExternalPlayerSnap, ExternalPlayerTeleportBody,
    ExternalPlayerVehiclePlacement, PendingPortalEntityTick, PlayerIdentity, TeleportApplication,
    TeleportTree,
};
pub use crate::entity::save::{
    create_fresh_entity, load_fresh_entity_tree, read_entity_chunk,
    read_entity_chunk_with_registry_tags, read_external_player_root_vehicle,
    read_external_player_root_vehicle_with_registry_tags,
    resolve_saved_last_hurt_by_mob_references, spawn_entity_tree, write_entity_chunk,
    write_external_player_root_vehicle, EntitySaveError,
};
pub use crate::entity::shulker_bullet::{ShulkerBulletData, ShulkerBulletTarget};
pub use crate::entity::snowball::{spawn_snowball, SnowballData};
pub use crate::entity::spectral_arrow::{spawn_spectral_arrow, SpectralArrowData};
pub use crate::entity::store::{
    transfer_stored_entity_subtree_cross_level,
    transfer_stored_entity_subtree_cross_level_with_context,
    transfer_stored_entity_tree_cross_level, transfer_stored_entity_tree_cross_level_with_context,
    CrossLevelEntityTreeError, CrossLevelEntityTreeOutcome, EntityCommand, EntityIdCounter,
    EntityLifecycleEvent, EntityStore, EntityVisibility, ExternalPlayerVehicleUnload,
    FramedMapRemoval, ItemPublication, PendingBredAnimals, PendingEntitySound, PendingKillingBlow,
    PendingPlayerKill, PendingPlayerStat, PendingTamedAnimal, PendingThrownItemPickup, SectionKey,
    StoredEntityTransition,
};
pub use crate::entity::thrown_potion::{
    spawn_lingering_potion, spawn_splash_potion, ThrownPotionData, ThrownPotionKind,
};
pub use crate::entity::tnt::{spawn_chain_reaction_tnt, spawn_tnt, TntData};
pub use crate::entity::tracking::{
    EntityTracker, PlayerPairingData, PlayerSyncAudience, PlayerSyncFrame, PlayerSyncPacket,
    PlayerSyncPackets, ServerEntity, TrackingViewer,
};
pub use crate::entity::trident::{spawn_trident, TridentData};
use bbb_server_types::geometry::Aabb;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MoverType {
    Self_,
    Player,
    Piston,
    ShulkerBox,
    Shulker,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RemovalReason {
    Killed,
    Discarded,
    UnloadedToChunk,
    UnloadedWithPlayer,
    ChangedDimension,
}

impl RemovalReason {
    /// `RemovalReason.shouldDestroy()` (`Entity.java:4083-4085`).
    pub const fn should_destroy(self) -> bool {
        matches!(self, Self::Killed | Self::Discarded)
    }

    /// `RemovalReason.shouldSave()` (`Entity.java:4087-4089`): only a chunk
    /// unload writes the entity back out.
    pub const fn should_save(self) -> bool {
        matches!(self, Self::UnloadedToChunk)
    }
}

/// The per-subtype half of an entity, vanilla's `Entity` subclass fields.
///
/// One variant so far. The enum rather than a trait object is deliberate: an
/// entity has to round-trip through a save and be compared for equality in a
/// differential replay, and both are far cheaper on a closed set.
// Closed entity payload stored inline per slot; boxing all large variants would add spawn heap churn.
#[allow(clippy::large_enum_variant)]
#[derive(Debug, Clone, PartialEq)]
pub enum EntityData {
    /// The shared `LivingEntity` base for a generated-catalog living type.
    GeneralLiving(GeneralLivingData),
    /// `net/minecraft/world/entity/item/FallingBlockEntity.java`.
    FallingBlock(FallingBlockData),
    /// `net/minecraft/world/entity/item/ItemEntity.java`.
    Item(ItemEntityData),
    /// `net/minecraft/world/entity/ExperienceOrb.java`.
    ExperienceOrb(ExperienceOrbData),
    /// `net/minecraft/world/entity/item/PrimedTnt.java`.
    Tnt(TntData),
    /// `net/minecraft/world/entity/decoration/painting/Painting.java`.
    Painting(PaintingData),
    /// `net/minecraft/world/entity/decoration/ItemFrame.java`.
    ItemFrame(ItemFrameData),
    /// `net/minecraft/world/entity/decoration/GlowItemFrame.java`.
    GlowItemFrame(ItemFrameData),
    /// `net/minecraft/world/entity/decoration/LeashFenceKnotEntity.java`.
    LeashKnot(LeashKnotData),
    /// `net/minecraft/world/entity/decoration/ArmorStand.java`.
    ArmorStand(ArmorStandData),
    /// `net/minecraft/world/entity/Marker.java`.
    Marker(MarkerData),
    /// `net/minecraft/world/entity/Interaction.java`.
    Interaction(InteractionData),
    /// `net/minecraft/world/entity/Display.java`, `Display.BlockDisplay`.
    BlockDisplay(BlockDisplayData),
    /// `net/minecraft/world/entity/Display.java`, `Display.ItemDisplay`.
    ItemDisplay(ItemDisplayData),
    /// `net/minecraft/world/entity/Display.java`, `Display.TextDisplay`.
    TextDisplay(TextDisplayData),
    /// `net/minecraft/world/entity/projectile/arrow/Arrow.java`.
    Arrow(ArrowData),
    /// `net/minecraft/world/entity/projectile/arrow/SpectralArrow.java`.
    SpectralArrow(SpectralArrowData),
    /// `net/minecraft/world/entity/projectile/arrow/ThrownTrident.java`.
    Trident(TridentData),
    /// `net/minecraft/world/entity/projectile/FishingHook.java`.
    FishingBobber(FishingBobberData),
    /// `net/minecraft/world/entity/projectile/throwableitemprojectile/Snowball.java`.
    Snowball(SnowballData),
    /// `net/minecraft/world/entity/projectile/throwableitemprojectile/Egg.java`.
    Egg(EggData),
    /// `net/minecraft/world/entity/projectile/throwableitemprojectile/ThrownEnderpearl.java`.
    EnderPearl(EnderPearlData),
    /// `net/minecraft/world/entity/boss/enderdragon/EndCrystal.java`.
    EndCrystal(EndCrystalData),
    /// `net/minecraft/world/entity/projectile/EyeOfEnder.java`.
    EyeOfEnder(EyeOfEnderData),
    /// `net/minecraft/world/entity/projectile/throwableitemprojectile/ThrownExperienceBottle.java`.
    ExperienceBottle(ExperienceBottleData),
    /// `net/minecraft/world/entity/projectile/FireworkRocketEntity.java`.
    FireworkRocket(FireworkRocketData),
    /// `net/minecraft/world/entity/projectile/LlamaSpit.java`.
    LlamaSpit(LlamaSpitData),
    /// `net/minecraft/world/entity/projectile/EvokerFangs.java`.
    EvokerFangs(EvokerFangsData),
    /// `net/minecraft/world/entity/LightningBolt.java`.
    LightningBolt(LightningBoltData),
    /// `net/minecraft/world/entity/projectile/hurtingprojectile/LargeFireball.java`.
    LargeFireball(LargeFireballData),
    /// `net/minecraft/world/entity/projectile/hurtingprojectile/SmallFireball.java`.
    SmallFireball(SmallFireballData),
    /// `net/minecraft/world/entity/projectile/hurtingprojectile/DragonFireball.java`.
    DragonFireball(DragonFireballData),
    /// `net/minecraft/world/entity/projectile/hurtingprojectile/WitherSkull.java`.
    WitherSkull(WitherSkullData),
    /// `net/minecraft/world/entity/projectile/hurtingprojectile/windcharge/WindCharge.java`.
    WindCharge(WindChargeData),
    /// `net/minecraft/world/entity/projectile/hurtingprojectile/windcharge/BreezeWindCharge.java`.
    BreezeWindCharge(WindChargeData),
    /// `net/minecraft/world/entity/AreaEffectCloud.java`.
    AreaEffectCloud(AreaEffectCloudData),
    /// `net/minecraft/world/entity/OminousItemSpawner.java`.
    OminousItemSpawner(OminousItemSpawnerData),
    /// `net/minecraft/world/entity/projectile/throwableitemprojectile/ThrownSplashPotion.java`.
    SplashPotion(ThrownPotionData),
    /// `net/minecraft/world/entity/projectile/throwableitemprojectile/ThrownLingeringPotion.java`.
    LingeringPotion(ThrownPotionData),
    /// `net/minecraft/world/entity/projectile/ShulkerBullet.java`.
    ShulkerBullet(ShulkerBulletData),
    Boat(BoatData),
    ChestBoat(ChestBoatData),
    /// `net/minecraft/world/entity/vehicle/minecart/Minecart.java`.
    Minecart(MinecartData),
    /// `net/minecraft/world/entity/vehicle/minecart/MinecartFurnace.java`.
    FurnaceMinecart(FurnaceMinecartData),
    ChestMinecart(MinecartContainerData),
    HopperMinecart(MinecartContainerData),
    /// `net/minecraft/world/entity/vehicle/minecart/MinecartCommandBlock.java`.
    CommandBlockMinecart(CommandBlockMinecartData),
    /// `net/minecraft/world/entity/vehicle/minecart/MinecartSpawner.java`.
    SpawnerMinecart(SpawnerMinecartData),
    /// `net/minecraft/world/entity/vehicle/minecart/MinecartTNT.java`.
    TntMinecart(TntMinecartData),
    /// An online player's body; the rest of the player rides in its store
    /// slot (`crate::world::PlayerState`).
    Player,
    /// A nonliving body owned by its dragon, outside normal store admission.
    EnderDragonPart,
}

impl EntityData {
    /// The registry name of the `EntityType` this data belongs to.
    pub const fn entity_resource_id(&self) -> &'static str {
        match self {
            Self::Player => "minecraft:player",
            Self::EnderDragonPart => "minecraft:ender_dragon",
            Self::GeneralLiving(data) => data.entity_resource_id(),
            Self::FallingBlock(_) => "minecraft:falling_block",
            Self::Item(_) => "minecraft:item",
            Self::ExperienceOrb(_) => "minecraft:experience_orb",
            Self::Tnt(_) => "minecraft:tnt",
            Self::Painting(_) => "minecraft:painting",
            Self::ItemFrame(_) => "minecraft:item_frame",
            Self::GlowItemFrame(_) => "minecraft:glow_item_frame",
            Self::LeashKnot(_) => "minecraft:leash_knot",
            Self::ArmorStand(_) => "minecraft:armor_stand",
            Self::Marker(_) => "minecraft:marker",
            Self::Interaction(_) => "minecraft:interaction",
            Self::BlockDisplay(_) => "minecraft:block_display",
            Self::ItemDisplay(_) => "minecraft:item_display",
            Self::TextDisplay(_) => "minecraft:text_display",
            Self::Arrow(_) => "minecraft:arrow",
            Self::SpectralArrow(_) => "minecraft:spectral_arrow",
            Self::Trident(_) => "minecraft:trident",
            Self::FishingBobber(_) => "minecraft:fishing_bobber",
            Self::Snowball(_) => "minecraft:snowball",
            Self::Egg(_) => "minecraft:egg",
            Self::EnderPearl(_) => "minecraft:ender_pearl",
            Self::EndCrystal(_) => "minecraft:end_crystal",
            Self::EyeOfEnder(_) => "minecraft:eye_of_ender",
            Self::ExperienceBottle(_) => "minecraft:experience_bottle",
            Self::FireworkRocket(_) => "minecraft:firework_rocket",
            Self::LlamaSpit(_) => "minecraft:llama_spit",
            Self::EvokerFangs(_) => "minecraft:evoker_fangs",
            Self::LightningBolt(_) => "minecraft:lightning_bolt",
            Self::LargeFireball(_) => "minecraft:fireball",
            Self::SmallFireball(_) => "minecraft:small_fireball",
            Self::DragonFireball(_) => "minecraft:dragon_fireball",
            Self::WitherSkull(_) => "minecraft:wither_skull",
            Self::WindCharge(_) => "minecraft:wind_charge",
            Self::BreezeWindCharge(_) => "minecraft:breeze_wind_charge",
            Self::AreaEffectCloud(_) => "minecraft:area_effect_cloud",
            Self::OminousItemSpawner(_) => "minecraft:ominous_item_spawner",
            Self::SplashPotion(_) => "minecraft:splash_potion",
            Self::LingeringPotion(_) => "minecraft:lingering_potion",
            Self::ShulkerBullet(_) => "minecraft:shulker_bullet",
            Self::Boat(data) => data.entity_resource_id(),
            Self::ChestBoat(data) => data.boat().entity_resource_id(),
            Self::Minecart(_) => "minecraft:minecart",
            Self::FurnaceMinecart(_) => "minecraft:furnace_minecart",
            Self::ChestMinecart(_) => "minecraft:chest_minecart",
            Self::HopperMinecart(_) => "minecraft:hopper_minecart",
            Self::CommandBlockMinecart(_) => "minecraft:command_block_minecart",
            Self::SpawnerMinecart(_) => "minecraft:spawner_minecart",
            Self::TntMinecart(_) => "minecraft:tnt_minecart",
        }
    }
}

impl EntityData {
    fn restore_command_transients(&mut self, previous: &Self) {
        match self {
            Self::Player | Self::EnderDragonPart => {}
            Self::GeneralLiving(data) => {
                let Self::GeneralLiving(previous) = previous else {
                    unreachable!("entity type is preserved across command load")
                };
                data.restore_command_transients(previous);
            }
            Self::FallingBlock(data) => {
                let Self::FallingBlock(previous) = previous else {
                    unreachable!("entity type is preserved across command load")
                };
                data.restore_command_transients(previous);
            }
            Self::Item(data) => {
                let Self::Item(previous) = previous else {
                    unreachable!("entity type is preserved across command load")
                };
                data.restore_command_transients(previous);
            }
            Self::ExperienceOrb(data) => {
                let Self::ExperienceOrb(previous) = previous else {
                    unreachable!("entity type is preserved across command load")
                };
                data.restore_command_transients(previous);
            }
            Self::Painting(data) => {
                let Self::Painting(previous) = previous else {
                    unreachable!("entity type is preserved across command load")
                };
                data.restore_command_transients(previous);
            }
            Self::ItemFrame(data) | Self::GlowItemFrame(data) => {
                let previous = match previous {
                    Self::ItemFrame(previous) | Self::GlowItemFrame(previous) => previous,
                    _ => unreachable!("entity type is preserved across command load"),
                };
                data.restore_command_transients(previous);
            }
            Self::LeashKnot(data) => {
                let Self::LeashKnot(previous) = previous else {
                    unreachable!("entity type is preserved across command load")
                };
                data.restore_command_transients(previous);
            }
            Self::ArmorStand(data) => {
                let Self::ArmorStand(previous) = previous else {
                    unreachable!("entity type is preserved across command load")
                };
                data.restore_command_transients(previous);
            }
            Self::Arrow(data) => {
                let Self::Arrow(previous) = previous else {
                    unreachable!("entity type is preserved across command load")
                };
                data.restore_command_transients(previous);
            }
            Self::SpectralArrow(data) => {
                let Self::SpectralArrow(previous) = previous else {
                    unreachable!("entity type is preserved across command load")
                };
                data.restore_command_transients(previous);
            }
            Self::Trident(data) => {
                let Self::Trident(previous) = previous else {
                    unreachable!("entity type is preserved across command load")
                };
                data.restore_command_transients(previous);
            }
            Self::FishingBobber(data) => {
                let Self::FishingBobber(previous) = previous else {
                    unreachable!("entity type is preserved across command load")
                };
                data.restore_command_transients(previous);
            }
            Self::EndCrystal(data) => {
                let Self::EndCrystal(previous) = previous else {
                    unreachable!("entity type is preserved across command load")
                };
                data.restore_command_transients(previous);
            }
            Self::EyeOfEnder(data) => {
                let Self::EyeOfEnder(previous) = previous else {
                    unreachable!("entity type is preserved across command load")
                };
                data.restore_command_transients(previous);
            }
            Self::FireworkRocket(data) => {
                let Self::FireworkRocket(previous) = previous else {
                    unreachable!("entity type is preserved across command load")
                };
                data.restore_command_transients(previous);
            }
            Self::EvokerFangs(data) => {
                let Self::EvokerFangs(previous) = previous else {
                    unreachable!("entity type is preserved across command load")
                };
                data.restore_command_transients(previous);
            }
            Self::LightningBolt(data) => {
                let Self::LightningBolt(previous) = previous else {
                    unreachable!("entity type is preserved across command load")
                };
                *data = previous.clone();
            }
            Self::WindCharge(data) | Self::BreezeWindCharge(data) => {
                let previous = match previous {
                    Self::WindCharge(previous) | Self::BreezeWindCharge(previous) => previous,
                    _ => unreachable!("entity type is preserved across command load"),
                };
                data.restore_command_transients(previous);
            }
            Self::AreaEffectCloud(data) => {
                let Self::AreaEffectCloud(previous) = previous else {
                    unreachable!("entity type is preserved across command load")
                };
                data.restore_command_transients(previous);
            }
            Self::Boat(data) => {
                let previous = match previous {
                    Self::Boat(previous) => previous,
                    _ => unreachable!("entity type is preserved across command load"),
                };
                data.restore_command_transients(previous);
            }
            Self::ChestBoat(data) => {
                let previous = match previous {
                    Self::ChestBoat(previous) => previous,
                    _ => unreachable!("entity type is preserved across command load"),
                };
                data.restore_command_transients(previous);
            }
            Self::Minecart(data) => {
                let Self::Minecart(previous) = previous else {
                    unreachable!("entity type is preserved across command load")
                };
                data.restore_command_transients(previous);
            }
            Self::FurnaceMinecart(data) => {
                let Self::FurnaceMinecart(previous) = previous else {
                    unreachable!("entity type is preserved across command load")
                };
                data.minecart_mut()
                    .restore_command_transients(previous.minecart());
            }
            Self::ChestMinecart(data) | Self::HopperMinecart(data) => {
                let previous = match previous {
                    Self::ChestMinecart(previous) | Self::HopperMinecart(previous) => previous,
                    _ => unreachable!("entity type is preserved across command load"),
                };
                data.restore_command_transients(previous);
            }
            Self::CommandBlockMinecart(data) => {
                let Self::CommandBlockMinecart(previous) = previous else {
                    unreachable!("entity type is preserved across command load")
                };
                data.restore_command_transients(previous);
            }
            Self::SpawnerMinecart(data) => {
                let Self::SpawnerMinecart(previous) = previous else {
                    unreachable!("entity type is preserved across command load")
                };
                data.restore_command_transients(previous);
            }
            Self::TntMinecart(data) => {
                let Self::TntMinecart(previous) = previous else {
                    unreachable!("entity type is preserved across command load")
                };
                data.restore_command_transients(previous);
            }
            Self::Tnt(data) => {
                let Self::Tnt(previous) = previous else {
                    unreachable!("entity type is preserved across command load")
                };
                data.restore_command_transients(previous);
            }
            Self::Marker(_)
            | Self::OminousItemSpawner(_)
            | Self::Interaction(_)
            | Self::BlockDisplay(_)
            | Self::ItemDisplay(_)
            | Self::TextDisplay(_)
            | Self::Snowball(_)
            | Self::Egg(_)
            | Self::EnderPearl(_)
            | Self::ExperienceBottle(_)
            | Self::LlamaSpit(_)
            | Self::LargeFireball(_)
            | Self::SmallFireball(_)
            | Self::DragonFireball(_)
            | Self::WitherSkull(_)
            | Self::SplashPotion(_)
            | Self::LingeringPotion(_)
            | Self::ShulkerBullet(_) => {}
        }
    }
}

/// `Entity.MAX_ENTITY_TAG_COUNT` (`Entity.java:193`), the ceiling `addTag`
/// compares against (`:392`).
pub const MAX_ENTITY_TAG_COUNT: usize = 1024;

#[derive(Debug, Clone, Default, PartialEq)]
struct EntityCold {
    portal_process: Option<portal::PortalProcessor>,
    last_deflected_by: Option<projectile_hit::ProjectileTarget>,
    minecart_track: Option<minecart::MinecartTrackInterpolation>,
    waypoint_update_pending: bool,
}

#[derive(Debug, Clone, Default)]
struct EntityColdBox(Option<Box<EntityCold>>);

impl EntityColdBox {
    #[inline]
    fn as_ref(&self) -> Option<&EntityCold> {
        self.0.as_deref()
    }

    #[inline]
    fn as_mut(&mut self) -> Option<&mut EntityCold> {
        self.0.as_deref_mut()
    }

    #[inline]
    fn get_or_insert_mut(&mut self) -> &mut EntityCold {
        self.0.get_or_insert_with(Box::default)
    }
}

impl PartialEq for EntityColdBox {
    fn eq(&self, other: &Self) -> bool {
        match (&self.0, &other.0) {
            (None, None) => true,
            (Some(a), Some(b)) => a == b,
            (Some(a), None) => a.as_ref() == &EntityCold::default(),
            (None, Some(b)) => b.as_ref() == &EntityCold::default(),
        }
    }
}

/// One entity: the fields `Entity` itself owns, plus its subtype's.
#[derive(Debug, Clone, PartialEq)]
pub struct Entity {
    id: i32,
    uuid: Uuid,
    entity_type_id: i32,
    width: f32,
    height: f32,
    eye_height: f32,
    position: Vec3d,
    old_position: Vec3d,
    block_position: BlockPos,
    filed_section: SectionKey,
    bounding_box: Aabb,
    delta_movement: Vec3d,
    y_rot: f32,
    x_rot: f32,
    on_ground: bool,
    main_supporting_block_pos: Option<BlockPos>,
    on_ground_no_blocks: bool,
    horizontal_collision: bool,
    vertical_collision: bool,
    vertical_collision_below: bool,
    minor_horizontal_collision: bool,
    stuck_speed_multiplier: Vec3d,
    piston_deltas: [f64; 3],
    piston_deltas_game_time: i64,
    move_dist: f32,
    fly_dist: f32,
    next_step: f32,
    crystal_sound: BodyCrystalSoundState,
    fall_distance: f64,
    remaining_fire_ticks: i32,
    air_supply: i32,
    invulnerable: bool,
    portal_cooldown: i32,
    no_physics: bool,
    no_gravity: bool,
    tick_count: i32,
    boarding_cooldown: i32,
    first_tick: bool,
    /// The previous base tick's water-contact result, read by projectile
    /// inertia before `Entity.baseTick` refreshes it.
    was_touching_water: bool,
    was_eye_in_water: bool,
    fluid_interaction: environment::FluidInteraction,
    is_in_powder_snow: bool,
    was_in_powder_snow: bool,
    movement_this_tick: std::collections::VecDeque<inside_blocks::EntityMovement>,
    final_movements_this_tick: Vec<inside_blocks::EntityMovement>,
    needs_sync: bool,
    hurt_marked: bool,
    removal_reason: Option<RemovalReason>,
    shared_flags: i8,
    custom_name: Option<RichTextComponent>,
    // Marker retains its payload owner; other entities keep the base value here.
    custom_data: Option<Vec<bbb_protocol::NbtSummaryEntry>>,
    custom_name_visible: bool,
    silent: bool,
    has_glowing_tag: bool,
    ticks_frozen: i32,
    has_visual_fire: bool,
    pose_id: i32,
    synched_dirty: [u64; 4],
    tags: Vec<bbb_protocol::JavaString>,
    cold: EntityColdBox,
    data: EntityData,
}

fn entity_bounding_box(data: &EntityData, position: Vec3d, width: f32, height: f32) -> Aabb {
    if let Some(bounds) = shulker::bounding_box(data, position, width) {
        return bounds;
    }
    let bounding_box = Aabb::of_entity(position, width, height);
    if matches!(
        data,
        EntityData::WindCharge(_) | EntityData::BreezeWindCharge(_)
    ) {
        bounding_box.moved(Vec3d {
            x: 0.0,
            y: -f64::from(0.15_f32),
            z: 0.0,
        })
    } else {
        bounding_box
    }
}

#[must_use]
pub fn piston_push_reaction(
    entity: &Entity,
) -> bbb_protocol::block_state_types::VanillaPistonPushReaction {
    use bbb_protocol::block_state_types::VanillaPistonPushReaction;
    match entity.data() {
        EntityData::Marker(_)
        | EntityData::Interaction(_)
        | EntityData::BlockDisplay(_)
        | EntityData::ItemDisplay(_)
        | EntityData::TextDisplay(_)
        | EntityData::AreaEffectCloud(_)
        | EntityData::OminousItemSpawner(_) => VanillaPistonPushReaction::Ignore,
        EntityData::ArmorStand(data) if data.is_marker() => VanillaPistonPushReaction::Ignore,
        _ => VanillaPistonPushReaction::Normal,
    }
}

impl Entity {
    ///
    /// The declared size comes from the generated 26.1 entity catalog, which is
    /// `EntityType`'s own `dimensions` field; an unknown type collapses to a
    /// zero-sized box, and [`EntityStore::restore`] refuses to file one, so no
    /// entity in a store ever carries zero dimensions.
    #[inline]
    pub fn new(
        id: i32,
        uuid: Uuid,
        entity_type_id: i32,
        position: Vec3d,
        data: EntityData,
    ) -> Self {
        let [width, height, eye_height] = general_living::dimensions::geometry(
            &data,
            entity_type_id,
            VANILLA_ENTITY_POSE_STANDING_ID,
        )
        .or_else(|| match &data {
            EntityData::ArmorStand(data) => Some(armor_stand::geometry(data)),
            _ => None,
        })
        .or_else(|| {
            let [width, height] = vanilla_entity_type_dimensions(entity_type_id)?;
            let eye = bbb_protocol::entity_types::vanilla_entity_type_eye_height(entity_type_id)
                .unwrap_or(height * 0.85_f32);
            Some([width, height, eye])
        })
        .unwrap_or([0.0, 0.0, 0.0]);
        let block_position = containing_block(position);
        let bounding_box = entity_bounding_box(&data, position, width, height);
        Self {
            id,
            uuid,
            entity_type_id,
            width,
            height,
            eye_height,
            position,
            old_position: position,
            block_position,
            filed_section: section_of_block(block_position),
            bounding_box,
            delta_movement: Vec3d {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            },
            y_rot: 0.0,
            x_rot: 0.0,
            on_ground: false,
            main_supporting_block_pos: None,
            on_ground_no_blocks: false,
            horizontal_collision: false,
            vertical_collision: false,
            vertical_collision_below: false,
            minor_horizontal_collision: false,
            stuck_speed_multiplier: Vec3d::default(),
            piston_deltas: [0.0; 3],
            piston_deltas_game_time: 0,
            move_dist: 0.0,
            fly_dist: 0.0,
            next_step: 1.0,
            crystal_sound: BodyCrystalSoundState::default(),
            fall_distance: 0.0,
            remaining_fire_ticks: 0,
            air_supply: 300,
            invulnerable: false,
            portal_cooldown: 0,
            no_physics: matches!(
                &data,
                EntityData::ShulkerBullet(_)
                    | EntityData::OminousItemSpawner(_)
                    | EntityData::BlockDisplay(_)
                    | EntityData::ItemDisplay(_)
                    | EntityData::TextDisplay(_)
            ),
            no_gravity: false,
            tick_count: 0,
            boarding_cooldown: 0,
            first_tick: true,
            was_touching_water: false,
            was_eye_in_water: false,
            fluid_interaction: environment::FluidInteraction::default(),
            is_in_powder_snow: false,
            was_in_powder_snow: false,
            movement_this_tick: std::collections::VecDeque::new(),
            final_movements_this_tick: Vec::new(),
            needs_sync: false,
            hurt_marked: false,
            removal_reason: None,
            shared_flags: 0,
            custom_name: None,
            custom_data: (!matches!(&data, EntityData::Marker(_))).then(Vec::new),
            custom_name_visible: false,
            silent: false,
            has_glowing_tag: false,
            ticks_frozen: 0,
            has_visual_fire: false,
            pose_id: VANILLA_ENTITY_POSE_STANDING_ID,
            synched_dirty: [0; 4],
            tags: Vec::new(),
            cold: EntityColdBox::default(),
            data,
        }
    }

    pub(crate) fn has_minecart_track_interpolation(&self) -> bool {
        self.cold
            .as_ref()
            .is_some_and(|cold| cold.minecart_track.is_some())
    }

    /// Marks this minecart as using `NewMinecartBehavior`-style track
    /// interpolation packets (`ClientboundMoveMinecartPacket`).
    pub(crate) fn enable_minecart_track_interpolation(&mut self) {
        if minecart::is_minecart(&self.data) {
            let cold = self.cold.get_or_insert_mut();
            if cold.minecart_track.is_none() {
                cold.minecart_track = Some(minecart::MinecartTrackInterpolation::default());
            }
        }
    }

    /// Appends one track-interpolation step for `ServerEntity.handleMinecartPosRot`.
    pub(crate) fn push_minecart_lerp_step(&mut self, step: bbb_protocol::packets::MinecartStep) {
        if minecart::is_minecart(&self.data) {
            self.cold
                .get_or_insert_mut()
                .minecart_track
                .get_or_insert_with(minecart::MinecartTrackInterpolation::default)
                .lerp_steps
                .push(step);
        }
    }

    /// Drains pending track-interpolation steps when this entity is a minecart
    /// with active track interpolation (`ServerEntity.java:134-135`, `:226-255`).
    pub(crate) fn take_minecart_track_steps(
        &mut self,
        minecart_improvements: bool,
    ) -> Option<Vec<bbb_protocol::packets::MinecartStep>> {
        if !minecart::is_minecart(&self.data) {
            return None;
        }
        if minecart_improvements {
            self.enable_minecart_track_interpolation();
        }
        let state = self.cold.as_mut()?.minecart_track.as_mut()?;
        Some(std::mem::take(&mut state.lerp_steps))
    }

    pub const fn id(&self) -> i32 {
        self.id
    }

    pub const fn uuid(&self) -> Uuid {
        self.uuid
    }

    pub const fn entity_type_id(&self) -> i32 {
        self.entity_type_id
    }

    pub const fn position(&self) -> Vec3d {
        self.position
    }

    pub fn is_first_tick(&self) -> bool {
        minecart::first_tick(self).unwrap_or(self.first_tick)
    }

    /// `Entity.oldPosition()` (`Entity.java:1750-1752`).
    pub const fn old_position(&self) -> Vec3d {
        self.old_position
    }

    pub fn set_old_pos_and_rot(&mut self) {
        self.old_position = self.position;
    }

    /// `Entity.blockPosition()` (`Entity.java:3634-3636`), the floor of the
    /// position kept in step with it by `setPosRaw` (`:3729-3733`).
    pub const fn block_position(&self) -> BlockPos {
        self.block_position
    }

    pub const fn bounding_box(&self) -> Aabb {
        self.bounding_box
    }

    // Shulker.java:307-313: shulkers report zero velocity and ignore velocity writes.
    pub const fn delta_movement(&self) -> Vec3d {
        if self.entity_type_id == bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_SHULKER_ID {
            Vec3d {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            }
        } else {
            self.delta_movement
        }
    }

    pub const fn y_rot(&self) -> f32 {
        self.y_rot
    }

    pub const fn x_rot(&self) -> f32 {
        self.x_rot
    }

    pub const fn on_ground(&self) -> bool {
        self.on_ground
    }

    pub(crate) fn set_on_ground(&mut self, on_ground: bool) {
        self.on_ground = on_ground;
    }

    pub const fn horizontal_collision(&self) -> bool {
        self.horizontal_collision
    }

    pub(crate) fn set_horizontal_collision(&mut self, horizontal_collision: bool) {
        if self.horizontal_collision != horizontal_collision {
            self.horizontal_collision = horizontal_collision;
            if self.is_spider() {
                self.mark_synched_dirty(
                    bbb_protocol::entity_synched_data::VANILLA_SPIDER_FLAGS_DATA_ID,
                );
            }
        }
    }

    pub const fn vertical_collision(&self) -> bool {
        self.vertical_collision
    }

    pub const fn vertical_collision_below(&self) -> bool {
        self.vertical_collision_below
    }

    pub const fn minor_horizontal_collision(&self) -> bool {
        self.minor_horizontal_collision
    }

    pub const fn fall_distance(&self) -> f64 {
        self.fall_distance
    }

    pub(crate) fn is_spider(&self) -> bool {
        matches!(
            self.entity_type_id(),
            bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_SPIDER_ID
                | bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_CAVE_SPIDER_ID
        )
    }

    /// `Entity.stuckSpeedMultiplier` (`Entity.java:234`).
    pub const fn stuck_speed_multiplier(&self) -> Vec3d {
        self.stuck_speed_multiplier
    }

    pub fn make_stuck_in_block(&mut self, speed_multiplier: Vec3d) {
        // `WitherBoss.makeStuckInBlock` (`WitherBoss.java:366-367`) is empty.
        if self.entity_type_id() == bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_WITHER_ID {
            return;
        }
        self.reset_fall_distance();
        self.stuck_speed_multiplier = speed_multiplier;
    }

    pub const fn remaining_fire_ticks(&self) -> i32 {
        self.remaining_fire_ticks
    }

    pub const fn air_supply(&self) -> i32 {
        self.air_supply
    }

    pub const fn invulnerable(&self) -> bool {
        self.invulnerable
    }

    pub const fn portal_cooldown(&self) -> i32 {
        self.portal_cooldown
    }

    pub fn portal_process(&self) -> Option<&portal::PortalProcessor> {
        self.cold.as_ref()?.portal_process.as_ref()
    }

    pub fn set_as_inside_portal(
        &mut self,
        kind: portal::PortalKind,
        entry_position: BlockPos,
        dimension_change_delay: i32,
    ) {
        if self.portal_cooldown > 0 {
            self.portal_cooldown = dimension_change_delay;
            return;
        }
        let cold = self.cold.get_or_insert_mut();
        portal::record_portal_contact(
            &mut self.portal_cooldown,
            &mut cold.portal_process,
            kind,
            entry_position,
            dimension_change_delay,
        );
    }

    pub fn process_portal(
        &mut self,
        transition_time: i32,
        allowed: bool,
        dimension_change_delay: i32,
    ) -> Option<portal::PortalTransitionRequest> {
        let allowed = allowed && self.sleeping_pos().is_none();
        let cold = self.cold.as_mut()?;
        portal::advance_portal_process(
            &mut self.portal_cooldown,
            &mut cold.portal_process,
            transition_time,
            allowed,
            dimension_change_delay,
        )
    }

    pub(crate) fn restore_portal_process(
        &mut self,
        portal_process: Option<portal::PortalProcessor>,
    ) {
        if let Some(process) = portal_process {
            self.cold.get_or_insert_mut().portal_process = Some(process);
        } else if let Some(cold) = self.cold.as_mut() {
            cold.portal_process = None;
        }
    }

    pub fn set_portal_cooldown(&mut self) {
        self.portal_cooldown = portal::ENTITY_PORTAL_COOLDOWN;
    }

    /// `Entity.isNoGravity()` (`Entity.java:1424-1426`).
    pub const fn no_gravity(&self) -> bool {
        self.no_gravity
    }

    pub(crate) fn set_no_gravity(&mut self, no_gravity: bool) {
        if self.no_gravity != no_gravity {
            self.no_gravity = no_gravity;
            self.mark_synched_dirty(VANILLA_ENTITY_NO_GRAVITY_DATA_ID);
        }
    }

    pub(crate) fn set_no_physics(&mut self, no_physics: bool) {
        self.no_physics = no_physics;
    }

    /// `Entity.noPhysics` (`Entity.java:1784`), the term `Entity.push` refuses
    /// a pair on.
    pub(crate) const fn no_physics(&self) -> bool {
        self.no_physics
    }

    pub const fn tick_count(&self) -> i32 {
        self.tick_count
    }

    pub const fn boarding_cooldown(&self) -> i32 {
        self.boarding_cooldown
    }

    pub(crate) fn set_boarding_cooldown(&mut self, ticks: i32) {
        self.boarding_cooldown = ticks.max(0);
    }

    /// `Entity.needsSync` (`Entity.java:274`).
    pub const fn needs_sync(&self) -> bool {
        self.needs_sync
    }

    pub fn set_needs_sync(&mut self, needs_sync: bool) {
        self.needs_sync = needs_sync;
    }

    pub fn mark_hurt(&mut self) {
        self.hurt_marked = true;
    }

    /// Takes the one-shot `hurtMarked` velocity resend flag.
    ///
    pub fn take_hurt_marked(&mut self) -> bool {
        std::mem::take(&mut self.hurt_marked)
    }

    /// Whether damage has requested an unconditional velocity resend.
    #[must_use]
    pub const fn hurt_marked(&self) -> bool {
        self.hurt_marked
    }

    pub fn set_invulnerable(&mut self, invulnerable: bool) {
        self.invulnerable = invulnerable;
    }

    /// `Entity.isRemoved()`: set, this entity is gone and only its save form is
    /// still of interest.
    pub const fn removal_reason(&self) -> Option<RemovalReason> {
        self.removal_reason
    }

    /// `Entity.isAlive()` (`Entity.java:2146-2148`).
    pub const fn is_alive(&self) -> bool {
        self.removal_reason.is_none()
    }

    /// `isAlive()` with the `LivingEntity` override (`LivingEntity.java:1718-1720`).
    pub fn is_alive_with_health(&self) -> bool {
        self.is_alive() && self.living().is_none_or(|living| living.health() > 0.0)
    }

    pub(crate) fn last_deflected_by(&self) -> Option<projectile_hit::ProjectileTarget> {
        self.cold.as_ref()?.last_deflected_by
    }

    pub(crate) fn set_last_deflected_by(&mut self, target: projectile_hit::ProjectileTarget) {
        self.cold.get_or_insert_mut().last_deflected_by = Some(target);
    }

    pub const fn data(&self) -> &EntityData {
        &self.data
    }

    pub const fn filed_section(&self) -> SectionKey {
        self.filed_section
    }

    pub fn set_filed_section(&mut self, section: SectionKey) {
        self.filed_section = section;
    }

    pub fn set_pos(&mut self, position: Vec3d) {
        let waypoint_updated = self.position != position
            && !self.first_tick
            && self.removal_reason.is_none()
            && self
                .living()
                .is_some_and(|living| living.waypoint_transmit_range() > 0.0);
        self.position = position;
        self.block_position = containing_block(position);
        self.bounding_box = entity_bounding_box(&self.data, position, self.width, self.height);
        if waypoint_updated {
            self.mark_waypoint_update_pending();
        }
    }

    pub fn set_pos_raw_and_bounding_box(&mut self, position: Vec3d, bounding_box: Aabb) {
        let waypoint_updated = self.position != position
            && !self.first_tick
            && self.removal_reason.is_none()
            && self
                .living()
                .is_some_and(|living| living.waypoint_transmit_range() > 0.0);
        self.position = position;
        self.block_position = containing_block(position);
        self.bounding_box = bounding_box;
        self.needs_sync = true;
        if waypoint_updated {
            self.mark_waypoint_update_pending();
        }
    }

    pub(crate) fn mark_waypoint_update_pending(&mut self) {
        self.cold.get_or_insert_mut().waypoint_update_pending = true;
    }

    pub(crate) fn take_waypoint_update_pending(&mut self) -> bool {
        self.cold
            .as_mut()
            .is_some_and(|cold| std::mem::take(&mut cold.waypoint_update_pending))
    }

    /// `Entity.setDeltaMovement(Vec3)`.
    pub fn set_delta_movement(&mut self, delta: Vec3d) {
        if self.entity_type_id != bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_SHULKER_ID {
            self.delta_movement = delta;
        }
    }

    pub const fn was_touching_water(&self) -> bool {
        self.was_touching_water
    }

    pub fn set_was_touching_water(&mut self, value: bool) {
        self.was_touching_water = value;
    }

    /// `Entity.setRot(float, float)` (`Entity.java:3811-3829`): non-finite angles zero out.
    pub fn set_rot(&mut self, y_rot: f32, x_rot: f32) {
        self.y_rot = if y_rot.is_finite() { y_rot } else { 0.0 };
        self.x_rot = if x_rot.is_finite() { x_rot } else { 0.0 };
    }

    pub fn set_removed(&mut self, reason: RemovalReason) {
        if self.removal_reason.is_none() {
            self.removal_reason = Some(reason);
        }
    }

    pub fn discard(&mut self) {
        self.set_removed(RemovalReason::Discarded);
    }

    /// `Entity.resetFallDistance()`.
    pub fn reset_fall_distance(&mut self) {
        self.fall_distance = 0.0;
    }

    pub(crate) fn set_fall_distance(&mut self, fall_distance: f64) {
        self.fall_distance = fall_distance;
    }

    /// `Entity.getBbWidth()` (`Entity.java:3604-3606`).
    pub const fn bb_width(&self) -> f32 {
        self.width
    }

    /// `Entity.getBbHeight()` (`Entity.java:3608-3610`).
    pub const fn bb_height(&self) -> f32 {
        self.height
    }

    pub fn eye_height(&self) -> f32 {
        self.eye_height
    }

    pub fn set_dimensions(&mut self, width: f32, height: f32) {
        self.set_dimensions_and_eye(width, height, height * 0.85_f32);
    }

    pub(crate) fn set_dimensions_and_eye(&mut self, width: f32, height: f32, eye_height: f32) {
        self.width = width;
        self.height = height;
        self.eye_height = eye_height;
        self.set_pos(self.position);
    }
}

fn containing_block(position: Vec3d) -> BlockPos {
    BlockPos {
        x: position.x.floor() as i32,
        y: position.y.floor() as i32,
        z: position.z.floor() as i32,
    }
}

///
/// The tolerance is written `1.0E-5F` in both overloads, so the `double` one
/// compares against a widened `float`, which is `9.999999747378752E-6` and not
/// `1.0E-5`.
fn mth_equal(a: f64, b: f64) -> bool {
    (b - a).abs() < f64::from(1.0e-5_f32)
}

#[cfg(test)]
pub mod tests;

pub(crate) mod supporting_block;
pub use supporting_block::{set_spawn_on_ground, set_spawn_on_ground_with_moving_pistons};
pub(crate) mod villager;
pub(crate) mod villager_life;
mod wandering_trader;
pub use wandering_trader::create_trader_event_root;
pub mod warden_spawn_tracker;
pub use warden_spawn_tracker::WardenSpawnTracker;

pub use general_living::apply_raid_buffs;
pub use raider::{
    clear_raider_raid, join_raid, patrol_spawn_state, prepare_patrol_leader, resolve_raider_raid,
    set_patrol_leader, RaiderRaidJoin,
};

mod zombification;
