//! The owned `LivingEntity` base for catalog types without a subtype runtime.

use bbb_protocol::block_state_types::vanilla_block_state_block_type_id;
use bbb_protocol::block_types::{VANILLA_BLOCK_BUBBLE_COLUMN_ID, VANILLA_BLOCK_GRASS_BLOCK_ID};
use bbb_protocol::difficulties::Difficulty;
use bbb_protocol::entity_synched_data::{
    VANILLA_ABSTRACT_HORSE_FLAGS_DATA_ID, VANILLA_ABSTRACT_HORSE_FLAG_BRED,
    VANILLA_ABSTRACT_HORSE_FLAG_EATING, VANILLA_ABSTRACT_HORSE_FLAG_STANDING,
    VANILLA_ABSTRACT_HORSE_FLAG_TAME, VANILLA_AGEABLE_MOB_BABY_DATA_ID, VANILLA_BAT_FLAGS_DATA_ID,
    VANILLA_CREEPER_SWELL_DIR_DATA_ID, VANILLA_DOLPHIN_MOISTNESS_DATA_ID,
    VANILLA_GLOW_SQUID_DARK_TICKS_DATA_ID, VANILLA_LIVING_ENTITY_HEALTH_DATA_ID,
    VANILLA_MOB_FLAG_AGGRESSIVE, VANILLA_MOB_FLAG_LEFT_HANDED, VANILLA_MOB_FLAG_NO_AI,
    VANILLA_TAMABLE_ANIMAL_FLAG_SITTING, VANILLA_TAMABLE_ANIMAL_FLAG_TAME,
};
use bbb_protocol::entity_types::{
    vanilla_entity_type, VanillaEntityMobCategory, VANILLA_ENTITY_TYPE_ARMOR_STAND_ID,
    VANILLA_ENTITY_TYPE_CAMEL_HUSK_ID, VANILLA_ENTITY_TYPE_CAMEL_ID, VANILLA_ENTITY_TYPE_CAT_ID,
    VANILLA_ENTITY_TYPE_CHICKEN_ID, VANILLA_ENTITY_TYPE_COD_ID, VANILLA_ENTITY_TYPE_DOLPHIN_ID,
    VANILLA_ENTITY_TYPE_DONKEY_ID, VANILLA_ENTITY_TYPE_FROG_ID, VANILLA_ENTITY_TYPE_GLOW_SQUID_ID,
    VANILLA_ENTITY_TYPE_HAPPY_GHAST_ID, VANILLA_ENTITY_TYPE_HORSE_ID, VANILLA_ENTITY_TYPE_LLAMA_ID,
    VANILLA_ENTITY_TYPE_MANNEQUIN_ID, VANILLA_ENTITY_TYPE_MULE_ID, VANILLA_ENTITY_TYPE_NAUTILUS_ID,
    VANILLA_ENTITY_TYPE_PARROT_ID, VANILLA_ENTITY_TYPE_PLAYER_ID,
    VANILLA_ENTITY_TYPE_PUFFERFISH_ID, VANILLA_ENTITY_TYPE_SALMON_ID,
    VANILLA_ENTITY_TYPE_SKELETON_HORSE_ID, VANILLA_ENTITY_TYPE_SQUID_ID,
    VANILLA_ENTITY_TYPE_TADPOLE_ID, VANILLA_ENTITY_TYPE_TRADER_LLAMA_ID,
    VANILLA_ENTITY_TYPE_TROPICAL_FISH_ID, VANILLA_ENTITY_TYPE_VEX_ID, VANILLA_ENTITY_TYPE_WOLF_ID,
    VANILLA_ENTITY_TYPE_ZOMBIE_HORSE_ID, VANILLA_ENTITY_TYPE_ZOMBIE_NAUTILUS_ID,
};
use bbb_protocol::equipment_slots::{EquipmentSlot, VANILLA_EQUIPMENT_SLOTS};
use bbb_protocol::packets::{BlockPos, EntityEvent, SoundSource, Vec3d};
use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue};
use uuid::Uuid;

use crate::connection::ClientboundPlayPacket;
use crate::entity::damage::{
    StoredDamageCausingEntity, StoredEntityDamageRules, StoredEntityDamageSource,
    StoredEntityHurtOutcome,
};
use crate::entity::environment::{fluid_contact, FluidContact, FluidTag};
use crate::entity::level::builtin_registry_tag_membership;
use crate::entity::living::{
    self, apply_immobile, apply_input, apply_movement_dead_band, calculate_fall_damage,
    fall_damage_sounds, living_base_tick_checked, move_relative, publish_equipment_changes,
    tick_jump, travel_in_air, LastHurtByMob, LivingBaseTickDamage, LivingEntityState,
};
use crate::entity::store::{
    EntityStore, PendingEntitySound, PendingKillingBlow, PendingPlayerKill,
};
use crate::entity::{Entity, EntityData, MoverType};
use crate::explosion::{
    ExplosionDamageProfile, ExplosionInteraction, ExplosionPresentation, ExplosionSource,
    PendingExplosion,
};
use crate::level_event::VanillaGameEvent;
use bbb_server_datapack::damage::{builtin_damage_type_registry, LAST_HURT_BY_PLAYER_MEMORY_TICKS};

mod ai_runtime;
mod bat_runtime;
mod command_data;
mod construction;
mod damage_runtime;
mod data_accessors;
mod despawn;
pub(crate) mod dimensions;
mod environment_damage;
pub(crate) mod frog_travel;
pub(super) mod horse_runtime;
mod hurt_sounds;
mod level_reads;
mod mob_state;
mod mob_variant;
mod retained_state;
mod spawn_finalize;
mod spawn_finalize_animals;
pub(crate) use spawn_finalize_animals::random_rabbit_variant;
pub(crate) mod spawn_finalize_equipment;
mod spawn_finalize_monsters;
mod tick_runtime;
mod type_variants;
pub(crate) use mob_variant::{FishState, MobVariantState};
pub(crate) use spawn_finalize_monsters::zombie_handle_attributes;
pub(crate) use type_variants::finish_baby_load;

#[cfg(test)]
pub(crate) use spawn_finalize::test_level::TestLevel;
pub(crate) use spawn_finalize::{
    finalize_spawn, finalize_spawn_with, SpawnContext, SpawnGroupData, SpawnLevel, SpawnReason,
};
pub use spawn_finalize_monsters::apply_raid_buffs;
pub(crate) use type_variants::{
    add_gossip_entry, apply_parent_variant_copy, clear_villager_assign_profession_when_spawned,
    copy_villager_data, fox_target_goals, gossip_reputation, increment_schooling_fish_size,
    parent_variant_copy, pick_random_wolf_sound_variant, set_component_villager_type,
    set_schooling_fish_leader, set_villager_level, set_villager_profession,
    sound_variant_resource_id, GossipKind, ParentVariantCopy, VillagerGossip,
};
pub use type_variants::{
    brain_memory as spawner_brain_memory, brain_packed, configure_fox_target_goals,
    resolve_registry_holders as resolve_spawner_registry_holders, schooling_fish_state,
    set_brain_memory as set_spawner_brain_memory, set_cat_spawn_persistence, set_piglin_baby,
    set_registry_value as set_spawner_registry_value,
    set_simple_variant as set_spawner_simple_variant, set_spawn_experience_reward,
    set_tropical_fish_schooling, set_villager_type as finalize_spawner_villager_type,
    set_zoglin_baby, tropical_fish_is_school, villager_finalized as spawner_villager_finalized,
};

pub(crate) use despawn::check_despawn;
pub(crate) use environment_damage::{
    hurt_from_environment, hurt_from_environment_outcome, GeneralLivingHurtEffects,
};
pub(crate) use horse_runtime::{
    abstract_horse_max_temper, apply_abstract_horse_make_mad, apply_abstract_horse_random_stand,
    apply_run_around_like_crazy, do_player_ride_body_writes, handle_abstract_horse_start_jump,
    modify_abstract_horse_temper, tick_abstract_horse_ambient_sound,
};
pub(crate) use level_reads::{can_breathe_underwater_membership, eye_water_contact_from_tracker};
pub(crate) use retained_state::{
    is_abstract_horse_entity_type, is_base_animal_entity_type, is_water_animal_entity_type,
    AbstractHorseRetainedState, AgeableMobRetainedState, AgeableWaterCreatureRetainedState,
    AnimalRetainedState, EndermanRetainedState, MobRetainedState, TamableRetainedState,
    VexRetainedState, DEFAULT_MOB_EQUIPMENT_DROP_CHANCE,
};
pub(crate) use retained_state::{
    set_aggressive, set_enderman_stare_flags, set_polar_bear_standing, set_vex_charging,
};
pub(crate) use tick_runtime::tick;

// Items a sibling module owns but the glob-importing submodules reach here.
use bat_runtime::{apply_bat_ai_step, tick_bat_after_super, tick_bat_ambient_sound};
pub(crate) use environment_damage::cause_fall_damage_inner;
use environment_damage::refused;
use retained_state::animal_custom_server_ai_step_calls_super;
pub(crate) use retained_state::HappyGhastState;

pub(crate) fn publish_baby_change(entity: &mut Entity) {
    entity.mark_synched_dirty(VANILLA_AGEABLE_MOB_BABY_DATA_ID);
    living::refresh_dimensions(entity);
}

pub fn set_spawner_age(entity: &mut Entity, age: i32) -> bool {
    let changed = match &mut entity.data {
        EntityData::GeneralLiving(data) => data
            .ageable_retained_mut()
            .is_some_and(|state| state.set_age(age)),
        _ => return false,
    };
    if changed {
        publish_baby_change(entity);
    }
    true
}

pub fn set_spawner_chicken_jockey(entity: &mut Entity) -> bool {
    let EntityData::GeneralLiving(data) = &mut entity.data else {
        return false;
    };
    data.set_chicken_jockey(true)
}

pub fn set_spawner_item(entity: &mut Entity, slot: EquipmentSlot, resource_id: &str) -> bool {
    let Some(stack) = crate::entity::item::plain_stack(resource_id, 1) else {
        return false;
    };
    let EntityData::GeneralLiving(data) = &mut entity.data else {
        return false;
    };
    data.living_mut().set_item_slot(slot, Some(stack));
    true
}

pub(crate) use damage_runtime::emit_entity_die;
pub(crate) use damage_runtime::external_die;
#[cfg(test)]
pub(crate) use damage_runtime::hurt_server_with_runtime;
pub(crate) use damage_runtime::{
    abstract_horse_hurt_follow_up, hurt_live_entity, hurt_server, hurt_server_with_runtime_in_level,
};

use crate::entity::ai::mob::bat::BatAiStepResult;
pub(crate) use crate::entity::ai::mob::bat::BatRetainedState;
pub(crate) use crate::entity::ai::mob::creeper::CreeperRetainedState;
use crate::entity::ai::mob::run_around_like_crazy::RunAroundLikeCrazyOutcome;
use crate::entity::ai::BrainPacked;
use bbb_server_types::fluid::block_state_is_air;

/// The `LivingEntity`/`Mob` base every generated-catalog living type shares,
/// plus the one per-type variant its class adds.
#[derive(Debug, Clone, PartialEq)]
pub struct GeneralLivingData {
    entity_resource_id: &'static str,
    body: Box<GeneralLivingBody>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct GeneralLivingBody {
    living: LivingEntityState,
    /// `Mob`; absent only for the non-Mob mannequin.
    pub(crate) mob: Option<MobRetainedState>,
    ageable: Option<AgeableMobRetainedState>,
    animal: Option<AnimalRetainedState>,
    pub(crate) internal_inventory:
        Option<crate::entity::living_internal_inventory::LivingInternalInventory>,
    brain: BrainPacked,
    variants: type_variants::TypeVariants,
    neutral_anger: Option<crate::entity::ai::mob::target::anger::NeutralAnger>,
    kind: MobVariantState,
    opaque_fields: Vec<NbtSummaryEntry>,
}

impl std::ops::Deref for GeneralLivingData {
    type Target = GeneralLivingBody;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.body
    }
}

impl std::ops::DerefMut for GeneralLivingData {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.body
    }
}

impl EntityStore {
    /// Creates the shared base runtime for a generated-catalog living type.
    pub fn spawn_general_living(&mut self, entity_type_id: i32, position: Vec3d) -> Option<i32> {
        let data = GeneralLivingData::new(entity_type_id)?;
        self.spawn_general_living_data(entity_type_id, position, data)
    }

    pub(super) fn spawn_general_living_data(
        &mut self,
        entity_type_id: i32,
        position: Vec3d,
        data: GeneralLivingData,
    ) -> Option<i32> {
        let is_dolphin = entity_type_id == VANILLA_ENTITY_TYPE_DOLPHIN_ID;
        let uuid = self.next_uuid();
        let y_rot = self.random().next_float() * std::f32::consts::TAU;
        self.restore_initialized(
            uuid,
            entity_type_id,
            position,
            EntityData::GeneralLiving(data),
            |entity| {
                entity.set_rot(y_rot, 0.0);
                if is_dolphin {
                    entity.set_air_supply(4800);
                }
                crate::entity::slime::initialize_spawn(entity);
                entity
                    .living_mut()
                    .expect("general living state")
                    .y_head_rot = y_rot;
            },
        )
    }
}
