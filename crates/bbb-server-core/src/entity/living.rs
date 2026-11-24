//! `LivingEntity` (`net/minecraft/world/entity/LivingEntity.java`): the layer
//! every mob, player and armour stand share. [`LivingEntityState`] is the field
//! set; the free functions are the tick/hurt pieces that do not depend on the
//! subclass, which a subtype module assembles in vanilla's order.
//!
//! Failed closed, each for want of an owner: player-only and world-mutating
//! mob-effect bodies, enchantments (every `EnchantmentHelper` call answers as
//! for an unenchanted item), the combat tracker, criteria and stats, `useItem`,
//! riding, fall-flying travel, the brain.

use crate::world::simulation_level::LevelCtx;
pub mod dimensions_growth;
mod equipment;
pub use equipment::EntityEquipment;
mod climbable;
pub(crate) mod gaze;
pub(crate) mod random_teleport;
pub use climbable::on_climbable_in_blocks;
mod inside_contact;
pub(crate) mod item_use;
pub(crate) mod valid_equipment;
pub(crate) use inside_contact::apply_detached_callback as apply_detached_inside_callback;

use bbb_protocol::attribute_types::{
    VANILLA_ATTRIBUTE_GRAVITY_ID, VANILLA_ATTRIBUTE_JUMP_STRENGTH_ID,
    VANILLA_ATTRIBUTE_MAX_ABSORPTION_ID, VANILLA_ATTRIBUTE_MAX_HEALTH_ID,
    VANILLA_ATTRIBUTE_MOVEMENT_EFFICIENCY_ID, VANILLA_ATTRIBUTE_STEP_HEIGHT_ID,
    VANILLA_ATTRIBUTE_WAYPOINT_TRANSMIT_RANGE_ID,
};
use bbb_protocol::block_state_types::vanilla_block_state_block_type_id;
use bbb_protocol::difficulties::Difficulty;
use bbb_protocol::entity_synched_data::{
    VANILLA_ENTITY_SHARED_FLAG_FALL_FLYING, VANILLA_ENTITY_SHARED_FLAG_INVISIBLE,
    VANILLA_ENTITY_SHARED_FLAG_ON_FIRE, VANILLA_ENTITY_SHARED_FLAG_SPRINTING,
    VANILLA_LIVING_ENTITY_ARROW_COUNT_DATA_ID, VANILLA_LIVING_ENTITY_HEALTH_DATA_ID,
    VANILLA_LIVING_ENTITY_STINGER_COUNT_DATA_ID,
};
use bbb_protocol::entity_types::{
    vanilla_entity_type_is_fire_immune, VANILLA_ENTITY_TYPE_AXOLOTL_ID,
    VANILLA_ENTITY_TYPE_DOLPHIN_ID, VANILLA_ENTITY_TYPE_MAGMA_CUBE_ID,
    VANILLA_ENTITY_TYPE_RABBIT_ID, VANILLA_ENTITY_TYPE_SLIME_ID, VANILLA_ENTITY_TYPE_SNIFFER_ID,
};
use bbb_protocol::equipment_slots::{EquipmentSlot, EquipmentSlotType, VANILLA_EQUIPMENT_SLOTS};
use bbb_protocol::packets::{BlockPos, SoundSource, Vec3d, WaypointIcon};
use bbb_registry_codec::PersistentComponentNbtError;
use bbb_server_types::item_stack::RuntimeItemStack;
use uuid::Uuid;

use crate::entity::attributes::{AttributeMap, AttributeSupplier, PackedAttribute};
use crate::entity::level::block_state_tag_membership;
use crate::entity::projectile_action::{PendingProjectileAction, PendingProjectileHurt};
use crate::entity::store::EntityStore;
use crate::entity::{base_tick_hurting_on_fire as entity_base_tick, Entity, EntityData, MoverType};
use bbb_server_datapack::damage::{builtin_damage_type_registry, DEATH_TICKS, INVULNERABLE_TICKS};
use bbb_server_types::geometry::Vec3Ops;

mod attributes_sync;
mod combat;
mod fall_damage;
mod ground_jump;
mod impulse;
pub mod mob_effects;
mod sleep;
mod swing;
mod synched_data;
#[cfg(test)]
mod tests;

pub use attributes_sync::{
    acknowledge_equipment_updates, detect_equipment_updates, refresh_dimensions,
    refresh_dirty_attributes,
};
pub(crate) use attributes_sync::{
    publish_equipment_changes, refresh_dimensions_with_context,
    refresh_dirty_attributes_with_level, resolve_dimensions_change,
};
pub(crate) use combat::is_invulnerable_to_with_tags;
pub use combat::{
    active_main_hand_disable_blocking_seconds, actually_hurt, armor_value, clamp_f32,
    damage_after_absorb, hurt_cooldown, hurt_knockback_direction, is_invulnerable_to, knockback,
    knockback_delta, mark_dead, scaled_knockback_power,
};
pub(crate) use sleep::{snap_first_sleep, start_sleeping, tick_sleep};
pub(crate) use swing::swing_animation_duration;
pub use synched_data::{entity_synched_values, living_synched_values, SynchedValue};
pub(crate) use synched_data::{
    push_dirty_entity_synched_values, push_dirty_living_synched_values, ENTITY_SYNCHED_DATA_MASK,
    LIVING_SYNCHED_DATA_MASK,
};

pub use fall_damage::body_fall_particles;
pub(crate) use fall_damage::fall_damage_sounds;
pub use fall_damage::{
    body_block_fall_sound, calculate_body_fall_damage, calculate_fall_damage, fall_damage_sound,
    GENERIC_BIG_FALL_SOUND, GENERIC_SMALL_FALL_SOUND,
};
pub(crate) use fall_damage::{cause_fall_damage, fall_sound_for, propagate_fall_to_passengers};
pub use ground_jump::{
    body_block_jump_factor, body_block_jump_factor_with_registry, body_ground_jump_velocity,
};
pub(crate) use impulse::tick_impulse_grace;
pub use impulse::FallImpulseContext;
pub use mob_effects::ActiveMobEffect;

/// The field `LivingEntity.lastHurtByMob` is, whose lookup lives in
/// [`crate::entity::entity_reference`] beside the projection that lookup reads.
pub use crate::entity::entity_reference::LastHurtByMob;

/// `LivingEntity.MIN_MOVEMENT_DISTANCE` (`LivingEntity.java:165`): the
/// per-axis dead band `aiStep` zeroes the velocity below.
pub const MIN_MOVEMENT_DISTANCE: f64 = 0.003;

/// `LivingEntity.DEFAULT_BASE_GRAVITY` (`LivingEntity.java:166`), which is also
/// the `GRAVITY` attribute's registry default.
pub const DEFAULT_BASE_GRAVITY: f64 = 0.08;

/// `LivingEntity.DEATH_DURATION` (`LivingEntity.java:167`), the `deathTime`
/// `tickDeath` removes the entity at (`:551`).
pub const DEATH_DURATION: i32 = 20;

/// `LivingEntity.INVULNERABLE_DURATION` (`LivingEntity.java:173`).
pub const INVULNERABLE_DURATION: i32 = 20;

/// `Entity.getMaxAirSupply()` (`Entity.java:2725-2727`).
pub const MAX_AIR_SUPPLY: i32 = 300;

/// The window `LivingEntity.tick` clears a resolvable `lastHurtByMob` inside
/// (`LivingEntity.java:470-477`).
pub(crate) const LAST_HURT_BY_MOB_MEMORY_TICKS: i32 = 100;

pub(crate) fn max_air_supply(entity: &Entity) -> i32 {
    match entity.entity_type_id() {
        VANILLA_ENTITY_TYPE_DOLPHIN_ID => 4800,
        VANILLA_ENTITY_TYPE_AXOLOTL_ID => 6000,
        _ => MAX_AIR_SUPPLY,
    }
}

fn increase_air_supply(entity: &Entity, current: i32) -> i32 {
    if entity.entity_type_id() == VANILLA_ENTITY_TYPE_DOLPHIN_ID {
        4800
    } else {
        current.wrapping_add(4).min(max_air_supply(entity))
    }
}

///
/// `filled` answers one slot, because the two bodies that own a cover keep
/// their stacks in different places: a stored entity in its [`EntityEquipment`]
/// and a session player in its own inventory. The `total > 0` guard
/// (`:2267`) is transcribed but unreachable -- four slots always qualify.
pub fn armor_cover_percentage(filled: impl Fn(EquipmentSlot) -> bool) -> f32 {
    let mut total = 0;
    let mut count = 0;
    for entry in VANILLA_EQUIPMENT_SLOTS
        .iter()
        .filter(|entry| entry.slot_type == EquipmentSlotType::HumanoidArmor)
    {
        if filled(entry.slot) {
            count += 1;
        }
        total += 1;
    }
    if total > 0 {
        count as f32 / total as f32
    } else {
        0.0
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct LivingEntityState {
    health: f32,
    /// `DATA_LIVING_ENTITY_FLAGS` (`:180`), byte, default `0` (`:309`).
    living_entity_flags: i8,
    pub(super) sleeping_pos: Option<BlockPos>,
    swing: swing::SwingState,
    /// `hurtTime` (`:215`), `hurtDuration` (`:216`), `deathTime` (`:217`).
    pub hurt_time: i32,
    pub hurt_duration: i32,
    pub death_time: i32,
    pub invulnerable_time: i32,
    pub last_hurt: f32,
    /// `lastHurtByMobTimestamp` (`:242`), saved as `HurtByTimestamp`.
    pub last_hurt_by_mob_timestamp: i32,
    last_hurt_by_mob: Option<LastHurtByMob>,
    /// `lastHurtByPlayer` (`:228`) as the reference's UUID, and
    /// `lastHurtByPlayerMemoryTime` (`:229`).
    pub last_hurt_by_player: Option<Uuid>,
    pub last_hurt_by_player_memory_time: i32,
    /// `lastDamageSource` (`:254`) as the damage type's registry id, and the
    /// remaining ticks of the window `getLastDamageSource` answers inside
    /// (`:1394-1400`). Vanilla stamps the game time and compares; a countdown
    /// clocked by the same `tick` that reads it expires on the same tick, and
    /// is how `lastHurtByPlayerMemoryTime` above already keeps its own window.
    pub last_damage_type: Option<String>,
    /// `lastDamageSource.getEntity()` (`:254`, `:1259-1262`) as the entity id
    /// this store files it under, expired by the same window.
    pub last_damage_source_entity: Option<i32>,
    pub last_damage_memory_time: i32,
    /// `skipDropExperience` (`:263`), which `skipDropExperience` sets
    /// (`:1647-1649`) and `wasExperienceConsumed` answers (`:1651-1653`).
    /// Transient: vanilla saves neither the flag nor its cause.
    skip_drop_experience: bool,
    /// `dead` (`:230`).
    pub dead: bool,
    /// `noActionTime` (`:231`).
    pub no_action_time: i32,
    /// `absorptionAmount` (`:247`).
    absorption_amount: f32,
    /// `yBodyRot`/`yBodyRotO`/`yHeadRot`/`yHeadRotO` (`:223-226`).
    pub y_body_rot: f32,
    pub y_body_rot_o: f32,
    pub y_head_rot: f32,
    pub y_head_rot_o: f32,
    /// `speed` (`:245`), which `getSpeed` (`:2666-2668`) answers and `Mob`
    /// sets from its movement-speed attribute; zero for a type that never sets
    /// it.
    pub speed: f32,
    /// `xxa`/`yya`/`zza` (`:234-236`), the travel input.
    pub xxa: f32,
    pub yya: f32,
    pub zza: f32,
    /// `jumping` (`:233`).
    pub jumping: bool,
    /// `discardFriction` (`:210`), which `setDiscardFriction` writes
    /// (`:658-660`) and `shouldDiscardFriction` answers (`:654-656`).
    /// Transient: vanilla saves it neither.
    pub discard_friction: bool,
    no_jump_delay: i32,
    pub fall_impulse: FallImpulseContext,
    last_climbable_pos: Option<BlockPos>,
    /// `attributes` (`:205`).
    pub(super) attributes: AttributeMap,
    /// `equipment` (`:267`).
    pub(super) equipment: EntityEquipment,
    /// `locatorBarIcon` (`:268`).
    waypoint_icon: WaypointIcon,
    /// `lastEquipmentItems` (`:208`), the per-slot snapshot
    /// `collectEquipmentChanges` diffs against.
    pub(super) last_equipment_items: EntityEquipment,
    pub(crate) location_effects: crate::entity::location_effects::LocationEffectsState,
    /// `activeEffects` and `effectsDirty` (`:207-208`).
    effects: mob_effects::ActiveMobEffects,
    effect_particles: Vec<bbb_protocol::packets::ParticlePayload>,
    effect_ambience: bool,
    /// `DATA_ARROW_COUNT_ID`/`removeArrowTime` (`:184`, `:213`) and
    /// `DATA_STINGER_COUNT_ID`/`removeStingerTime` (`:185`, `:214`).
    pub(super) arrows: StuckCount,
    pub(super) stingers: StuckCount,
}

/// Arrows or stingers stuck in a body and the timer that sheds them.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct StuckCount {
    pub(crate) count: i32,
    remove_time: i32,
}

impl StuckCount {
    /// `LivingEntity.tick`'s shedding (`LivingEntity.java:2694-2716`): the
    /// more stick out, the sooner one falls off. Answers whether one did.
    pub(crate) fn tick(&mut self) -> bool {
        if self.count <= 0 {
            return false;
        }
        if self.remove_time <= 0 {
            self.remove_time = 20 * (30 - self.count);
        }
        self.remove_time -= 1;
        if self.remove_time > 0 {
            return false;
        }
        self.count -= 1;
        true
    }
}

impl LivingEntityState {
    pub const fn last_climbable_pos(&self) -> Option<BlockPos> {
        self.last_climbable_pos
    }

    pub(crate) fn set_last_climbable_pos(&mut self, pos: Option<BlockPos>) {
        self.last_climbable_pos = pos;
    }

    pub(crate) fn restore_command_transients(&mut self, previous: &Self) {
        self.living_entity_flags = previous.living_entity_flags;
        self.swing = previous.swing.clone();
        self.hurt_duration = previous.hurt_duration;
        self.invulnerable_time = previous.invulnerable_time;
        self.last_hurt = previous.last_hurt;
        self.last_damage_type = previous.last_damage_type.clone();
        self.last_damage_source_entity = previous.last_damage_source_entity;
        self.last_damage_memory_time = previous.last_damage_memory_time;
        self.skip_drop_experience = previous.skip_drop_experience;
        self.dead = previous.dead;
        self.no_action_time = previous.no_action_time;
        self.y_body_rot_o = previous.y_body_rot_o;
        self.y_head_rot_o = previous.y_head_rot_o;
        self.speed = previous.speed;
        self.xxa = previous.xxa;
        self.yya = previous.yya;
        self.zza = previous.zza;
        self.jumping = previous.jumping;
        self.discard_friction = previous.discard_friction;
        self.no_jump_delay = previous.no_jump_delay;
        self.last_climbable_pos = previous.last_climbable_pos;
        self.last_equipment_items = previous.last_equipment_items.clone();
        self.location_effects = previous.location_effects.clone();
        self.arrows = previous.arrows;
        self.stingers = previous.stingers;
    }

    ///
    /// `None` for a type `DefaultAttributes` has no supplier for, which is the
    /// `NullPointerException` vanilla would throw building the map.
    pub fn new(entity_resource_id: &str) -> Option<Self> {
        let attributes = AttributeMap::new(AttributeSupplier::for_entity_type(entity_resource_id)?);
        let mut state = Self {
            health: 1.0,
            living_entity_flags: 0,
            sleeping_pos: None,
            swing: swing::SwingState::default(),
            hurt_time: 0,
            hurt_duration: 0,
            death_time: 0,
            invulnerable_time: 0,
            last_hurt: 0.0,
            last_hurt_by_mob_timestamp: 0,
            last_hurt_by_mob: None,
            last_hurt_by_player: None,
            last_hurt_by_player_memory_time: 0,
            last_damage_type: None,
            last_damage_source_entity: None,
            last_damage_memory_time: 0,
            skip_drop_experience: false,
            dead: false,
            no_action_time: 0,
            absorption_amount: 0.0,
            y_body_rot: 0.0,
            y_body_rot_o: 0.0,
            y_head_rot: 0.0,
            y_head_rot_o: 0.0,
            speed: 0.0,
            xxa: 0.0,
            yya: 0.0,
            zza: 0.0,
            jumping: false,
            discard_friction: false,
            no_jump_delay: 0,
            fall_impulse: FallImpulseContext::default(),
            last_climbable_pos: None,
            attributes,
            equipment: EntityEquipment::new(),
            waypoint_icon: WaypointIcon {
                style: "minecraft:default".to_string(),
                color_rgb: None,
            },
            last_equipment_items: EntityEquipment::new(),
            location_effects: Default::default(),
            effects: mob_effects::ActiveMobEffects::default(),

            effect_particles: Vec::new(),
            effect_ambience: false,
            arrows: StuckCount::default(),
            stingers: StuckCount::default(),
        };
        // `:275`.
        state.set_health(state.max_health());
        Some(state)
    }

    /// Constructs a [`LivingEntityState`] for a modded entity starting from the
    /// standard Mob attribute set and overriding any attribute defaults from
    /// [`ModEntityTypeSpec::default_attributes`].
    pub fn new_modded(default_attributes: &[(i32, f64)]) -> Self {
        let supplier = AttributeSupplier::for_entity_type("minecraft:zombie")
            .expect("minecraft:zombie supplies standard LivingEntity/Mob attributes");
        let mut attributes = AttributeMap::new(supplier);
        for &(attribute_id, base_value) in default_attributes {
            if let Some(instance) = attributes.get_instance(attribute_id) {
                instance.set_base_value(base_value);
            }
        }
        let mut state = Self {
            health: 1.0,
            living_entity_flags: 0,
            sleeping_pos: None,
            swing: swing::SwingState::default(),
            hurt_time: 0,
            hurt_duration: 0,
            death_time: 0,
            invulnerable_time: 0,
            last_hurt: 0.0,
            last_hurt_by_mob_timestamp: 0,
            last_hurt_by_mob: None,
            last_hurt_by_player: None,
            last_hurt_by_player_memory_time: 0,
            last_damage_type: None,
            last_damage_source_entity: None,
            last_damage_memory_time: 0,
            skip_drop_experience: false,
            dead: false,
            no_action_time: 0,
            absorption_amount: 0.0,
            y_body_rot: 0.0,
            y_body_rot_o: 0.0,
            y_head_rot: 0.0,
            y_head_rot_o: 0.0,
            speed: 0.0,
            xxa: 0.0,
            yya: 0.0,
            zza: 0.0,
            jumping: false,
            discard_friction: false,
            no_jump_delay: 0,
            fall_impulse: FallImpulseContext::default(),
            last_climbable_pos: None,
            attributes,
            equipment: EntityEquipment::new(),
            waypoint_icon: WaypointIcon {
                style: "minecraft:default".to_string(),
                color_rgb: None,
            },
            last_equipment_items: EntityEquipment::new(),
            location_effects: Default::default(),
            effects: mob_effects::ActiveMobEffects::default(),
            effect_particles: Vec::new(),
            effect_ambience: false,
            arrows: StuckCount::default(),
            stingers: StuckCount::default(),
        };
        state.set_health(state.max_health());
        state
    }

    /// `getHealth()` (`LivingEntity.java:1142-1144`).
    pub const fn health(&self) -> f32 {
        self.health
    }

    pub fn set_health(&mut self, health: f32) {
        let max = self.max_health();
        self.health = if health < 0.0 { 0.0 } else { health.min(max) };
    }

    pub fn heal(&mut self, amount: f32) {
        let health = self.health;
        if health > 0.0 {
            self.set_health(health + amount);
        }
    }

    pub fn is_dead_or_dying(&self) -> bool {
        self.health <= 0.0
    }

    pub fn max_health(&self) -> f32 {
        self.attribute_value(VANILLA_ATTRIBUTE_MAX_HEALTH_ID) as f32
    }

    pub fn max_absorption(&self) -> f32 {
        self.attribute_value(VANILLA_ATTRIBUTE_MAX_ABSORPTION_ID) as f32
    }

    /// `getAbsorptionAmount()` (`LivingEntity.java:3329-3331`).
    pub const fn absorption_amount(&self) -> f32 {
        self.absorption_amount
    }

    pub fn set_absorption_amount(&mut self, amount: f32) {
        let max = self.max_absorption();
        self.absorption_amount = if amount < 0.0 { 0.0 } else { amount.min(max) };
    }

    pub fn internal_set_absorption_amount(&mut self, amount: f32) {
        self.absorption_amount = amount;
    }

    pub fn attribute_value(&self, attribute_id: i32) -> f64 {
        self.attributes
            .value(attribute_id)
            .or_else(|| crate::entity::attributes::attribute_default_value(attribute_id))
            .unwrap_or(0.0)
    }

    /// `getAttributes()` (`LivingEntity.java:2172-2174`).
    pub const fn attributes(&self) -> &AttributeMap {
        &self.attributes
    }

    pub fn attributes_mut(&mut self) -> &mut AttributeMap {
        &mut self.attributes
    }

    pub fn waypoint_transmit_range(&self) -> f64 {
        self.attribute_value(VANILLA_ATTRIBUTE_WAYPOINT_TRANSMIT_RANGE_ID)
    }

    pub const fn waypoint_icon(&self) -> &WaypointIcon {
        &self.waypoint_icon
    }

    pub(super) fn waypoint_icon_mut(&mut self) -> &mut WaypointIcon {
        &mut self.waypoint_icon
    }

    pub fn set_waypoint_icon(&mut self, icon: WaypointIcon) {
        self.waypoint_icon = icon;
    }

    pub fn item_by_slot(
        &self,
        slot: EquipmentSlot,
    ) -> Option<&bbb_server_types::item_stack::RuntimeItemStack> {
        self.equipment.get(slot)
    }

    pub fn has_item_in_slot(&self, slot: EquipmentSlot) -> bool {
        self.equipment.get(slot).is_some()
    }

    #[must_use]
    pub fn armor_cover_percentage(&self) -> f32 {
        armor_cover_percentage(|slot| self.has_item_in_slot(slot))
    }

    /// The equipment itself, for the save writer and the pairing snapshot.
    pub const fn equipment(&self) -> &EntityEquipment {
        &self.equipment
    }

    pub fn set_item_slot(
        &mut self,
        slot: EquipmentSlot,
        stack: Option<RuntimeItemStack>,
    ) -> Option<RuntimeItemStack> {
        self.equipment.set(slot, stack)
    }

    pub(crate) fn equipment_mut(&mut self) -> &mut EntityEquipment {
        &mut self.equipment
    }

    pub(crate) fn update_item_slot(
        &mut self,
        slot: EquipmentSlot,
        stack: RuntimeItemStack,
    ) -> Option<()> {
        self.equipment.update_item(slot, stack)
    }

    pub fn set_all_equipment(&mut self, equipment: &EntityEquipment) {
        self.equipment.set_all(equipment);
    }

    /// `DATA_LIVING_ENTITY_FLAGS` as the wire carries it.
    pub const fn living_entity_flags(&self) -> i8 {
        self.living_entity_flags
    }

    pub fn set_living_entity_flag(&mut self, flag: i8, value: bool) {
        if value {
            self.living_entity_flags |= flag;
        } else {
            self.living_entity_flags &= !flag;
        }
    }

    pub fn packed_attributes(&self) -> Vec<PackedAttribute> {
        self.attributes.pack()
    }

    pub fn apply_packed_attributes(&mut self, packed: &[PackedAttribute]) {
        self.attributes.apply(packed);
    }

    pub fn is_immobile(&self) -> bool {
        self.is_dead_or_dying()
    }

    /// `getSpeed()` (`LivingEntity.java:2666-2668`).
    pub const fn speed(&self) -> f32 {
        self.speed
    }

    pub fn set_speed(&mut self, speed: f32) {
        self.speed = speed;
    }

    pub(crate) fn begin_jump_step(&mut self) -> bool {
        if self.no_jump_delay > 0 {
            self.no_jump_delay -= 1;
        }
        if !self.jumping {
            self.no_jump_delay = 0;
        }
        self.jumping
    }

    pub(crate) const fn no_jump_delay(&self) -> i32 {
        self.no_jump_delay
    }

    pub(crate) fn set_no_jump_delay(&mut self, ticks: i32) {
        self.no_jump_delay = ticks;
    }

    /// `getYHeadRot()` (`LivingEntity.java:3306-3308`).
    pub const fn y_head_rot(&self) -> f32 {
        self.y_head_rot
    }

    /// `wasExperienceConsumed()` (`LivingEntity.java:1651-1653`).
    #[must_use]
    pub const fn was_experience_consumed(&self) -> bool {
        self.skip_drop_experience
    }

    pub fn skip_drop_experience(&mut self) {
        self.skip_drop_experience = true;
    }

    /// `getLastDamageSource().getEntity()` (`:1394-1400`), as the entity id.
    #[must_use]
    pub const fn last_damage_source_entity(&self) -> Option<i32> {
        self.last_damage_source_entity
    }

    /// `getLastHurtByMob()` (`LivingEntity.java:593-595`) as the unresolved
    /// reference, which only the caller's own entity lookup can turn into an
    /// entity.
    #[must_use]
    pub const fn last_hurt_by_mob(&self) -> Option<LastHurtByMob> {
        self.last_hurt_by_mob
    }

    pub fn set_last_hurt_by_mob(&mut self, mob: Option<LastHurtByMob>, tick_count: i32) {
        self.last_hurt_by_mob = mob;
        self.last_hurt_by_mob_timestamp = tick_count;
    }
}

impl Entity {
    /// The `LivingEntity` half of this entity, if it is one.
    pub fn living(&self) -> Option<&LivingEntityState> {
        match &self.data {
            EntityData::GeneralLiving(data) => Some(data.living()),
            EntityData::ArmorStand(data) => Some(data.living()),
            _ => None,
        }
    }

    pub fn living_mut(&mut self) -> Option<&mut LivingEntityState> {
        match &mut self.data {
            EntityData::GeneralLiving(data) => Some(data.living_mut()),
            EntityData::ArmorStand(data) => Some(data.living_mut()),
            _ => None,
        }
    }

    pub fn y_head_rot(&self) -> f32 {
        self.living().map_or(0.0, LivingEntityState::y_head_rot)
    }

    pub fn set_y_head_rot(&mut self, y_head_rot: f32) {
        if let EntityData::ArmorStand(data) = &mut self.data {
            let living = data.living_mut();
            living.y_body_rot_o = y_head_rot;
            living.y_head_rot_o = y_head_rot;
            living.y_head_rot = y_head_rot;
        } else if let Some(living) = self.living_mut() {
            living.y_head_rot = y_head_rot;
        }
    }

    pub fn set_discard_friction(&mut self, discard_friction: bool) {
        if let Some(living) = self.living_mut() {
            living.discard_friction = discard_friction;
        }
    }

    /// `Entity.isInvisible()` (`Entity.java:2672-2674`).
    pub const fn is_invisible(&self) -> bool {
        self.get_shared_flag(VANILLA_ENTITY_SHARED_FLAG_INVISIBLE)
    }

    pub fn set_invisible(&mut self, invisible: bool) {
        self.set_shared_flag(VANILLA_ENTITY_SHARED_FLAG_INVISIBLE, invisible);
    }

    /// `LivingEntity.isFallFlying()` (`LivingEntity.java:3694-3696`).
    pub const fn is_fall_flying(&self) -> bool {
        self.get_shared_flag(VANILLA_ENTITY_SHARED_FLAG_FALL_FLYING)
    }

    pub fn set_fall_flying(&mut self, fall_flying: bool) {
        self.set_shared_flag(VANILLA_ENTITY_SHARED_FLAG_FALL_FLYING, fall_flying);
    }

    pub fn set_shared_flag_on_fire(&mut self, on_fire: bool) {
        let visible = on_fire || self.has_visual_fire();
        self.set_shared_flag(VANILLA_ENTITY_SHARED_FLAG_ON_FIRE, visible);
    }

    pub fn is_on_fire(&self) -> bool {
        self.on_fire_override()
            .unwrap_or_else(|| !self.fire_immune() && self.remaining_fire_ticks > 0)
    }

    pub(crate) fn is_on_fire_in(
        &self,
        level: &LevelCtx<'_, '_>,
    ) -> Result<bool, PersistentComponentNbtError> {
        if let Some(on_fire) = self.on_fire_override() {
            return Ok(on_fire);
        }
        self.is_on_fire_with_publication(level.item_stack_resolver())
    }

    pub(crate) fn is_on_fire_with_publication(
        &self,
        publication: &'_ dyn bbb_registry_codec::PersistentItemStackHolderResolver,
    ) -> Result<bool, PersistentComponentNbtError> {
        if let Some(on_fire) = self.on_fire_override() {
            return Ok(on_fire);
        }
        // Java queries fireImmune before testing Fire, even when Fire is zero.
        Ok(!self.fire_immune_with_publication(publication)? && self.remaining_fire_ticks > 0)
    }

    fn on_fire_override(&self) -> Option<bool> {
        use bbb_protocol::entity_types::{
            VANILLA_ENTITY_TYPE_BLAZE_ID, VANILLA_ENTITY_TYPE_SHULKER_BULLET_ID,
            VANILLA_ENTITY_TYPE_STRIDER_ID, VANILLA_ENTITY_TYPE_WITHER_SKULL_ID,
        };
        match self.entity_type_id() {
            VANILLA_ENTITY_TYPE_BLAZE_ID => Some(match self.data() {
                EntityData::GeneralLiving(data) => {
                    data.blaze_retained().is_some_and(|state| state.charged())
                }
                _ => false,
            }),
            VANILLA_ENTITY_TYPE_MAGMA_CUBE_ID
            | VANILLA_ENTITY_TYPE_STRIDER_ID
            | VANILLA_ENTITY_TYPE_SHULKER_BULLET_ID
            | VANILLA_ENTITY_TYPE_WITHER_SKULL_ID => Some(false),
            _ => None,
        }
    }

    pub fn fire_immune(&self) -> bool {
        self.is_heart_bound()
            || self.plugin_mob().is_some_and(|plugin| plugin.fire_immune)
            || vanilla_entity_type_is_fire_immune(self.entity_type_id)
    }

    pub(crate) fn fire_immune_in(
        &self,
        level: &LevelCtx<'_, '_>,
    ) -> Result<bool, PersistentComponentNbtError> {
        self.fire_immune_with_publication(level.item_stack_resolver())
    }

    fn fire_immune_with_publication(
        &self,
        publication: &'_ dyn bbb_registry_codec::PersistentItemStackHolderResolver,
    ) -> Result<bool, PersistentComponentNbtError> {
        crate::entity::damage::fire_immune_with_item_query(self, |stack, damage_type| {
            Ok(!crate::entity::damage::item_resists_damage(
                stack,
                damage_type,
                publication,
            ))
        })
    }

    pub fn ignite_for_ticks(&mut self, ticks: i32) {
        if self.remaining_fire_ticks < ticks {
            self.remaining_fire_ticks = ticks;
        }
        self.set_ticks_frozen(0);
    }

    pub fn ignite_for_seconds(&mut self, seconds: f32) {
        self.ignite_for_ticks((seconds * 20.0).floor() as i32);
    }

    pub fn clear_fire(&mut self) {
        self.remaining_fire_ticks = self.remaining_fire_ticks.min(0);
    }

    pub fn set_remaining_fire_ticks(&mut self, ticks: i32) {
        self.remaining_fire_ticks = ticks;
    }
}

pub fn set_health(entity: &mut Entity, health: f32) {
    let Some(living) = entity.living_mut() else {
        return;
    };
    let before = living.health().to_bits();
    living.set_health(health);
    if living.health().to_bits() != before {
        entity.mark_synched_dirty(VANILLA_LIVING_ENTITY_HEALTH_DATA_ID);
    }
}

/// An arrow stuck in the body (`AbstractArrow.java:464-466`).
pub(crate) fn add_arrow(entity: &mut Entity) {
    if let Some(living) = entity.living_mut() {
        living.arrows.count += 1;
        entity.mark_synched_dirty(VANILLA_LIVING_ENTITY_ARROW_COUNT_DATA_ID);
    }
}

/// A bee's stinger left in the body (`Bee.java:232`).
pub(crate) fn add_stinger(entity: &mut Entity) {
    if let Some(living) = entity.living_mut() {
        living.stingers.count += 1;
        entity.mark_synched_dirty(VANILLA_LIVING_ENTITY_STINGER_COUNT_DATA_ID);
    }
}

pub(crate) fn tick_stuck_counts(entity: &mut Entity) {
    let Some(living) = entity.living_mut() else {
        return;
    };
    let (arrow, stinger) = (living.arrows.tick(), living.stingers.tick());
    if arrow {
        entity.mark_synched_dirty(VANILLA_LIVING_ENTITY_ARROW_COUNT_DATA_ID);
    }
    if stinger {
        entity.mark_synched_dirty(VANILLA_LIVING_ENTITY_STINGER_COUNT_DATA_ID);
    }
}

pub fn ignite_living_for_ticks(entity: &mut Entity, ticks: i32) {
    let Some(living) = entity.living() else {
        entity.ignite_for_ticks(ticks);
        return;
    };
    let burning_time =
        living.attribute_value(bbb_protocol::attribute_types::VANILLA_ATTRIBUTE_BURNING_TIME_ID);
    let scaled = (f64::from(ticks) * burning_time).ceil() as i32;
    entity.ignite_for_ticks(scaled);
}

pub fn ignite_living_for_seconds(entity: &mut Entity, seconds: f32) {
    ignite_living_for_ticks(entity, (seconds * 20.0).floor() as i32);
}

/// The damage the living tick asks its subtype to deal, handed back so the
/// subtype can run its own `hurtServer` with the store it owns.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LivingBaseTickDamage {
    OnFire { amount: f32 },
    Cactus { amount: f32 },
    SweetBerryBush { amount: f32 },
    InWall { amount: f32 },
    Freeze { amount: f32 },
    OutOfWorld { amount: f32 },
}

pub(crate) fn living_base_tick_checked<F>(
    entity: &mut Entity,
    store: &mut EntityStore,
    level: &mut LevelCtx<'_, '_>,
    hurt: F,
) -> bool
where
    F: FnMut(&mut Entity, &mut EntityStore, &mut LevelCtx<'_, '_>, LivingBaseTickDamage) -> bool,
{
    living_base_tick_with_owned_hooks(entity, store, level, hurt)
}

fn living_base_tick_with_owned_hooks<F>(
    entity: &mut Entity,
    store: &mut EntityStore,
    level: &mut LevelCtx<'_, '_>,
    mut hurt: F,
) -> bool
where
    F: FnMut(&mut Entity, &mut EntityStore, &mut LevelCtx<'_, '_>, LivingBaseTickDamage) -> bool,
{
    snap_first_sleep(entity);
    crate::entity::location_effects::tick_effects(entity, store, level);
    let mut fire_hurt_refused = false;
    if !entity_base_tick(entity, store, level, |entity, store, level| {
        if !hurt(
            entity,
            store,
            level,
            LivingBaseTickDamage::OnFire { amount: 1.0 },
        ) {
            fire_hurt_refused = true;
        }
    }) {
        return false;
    }
    if crate::entity::is_below_world(entity, level)
        && !hurt(
            entity,
            store,
            level,
            LivingBaseTickDamage::OutOfWorld { amount: 4.0 },
        )
    {
        return false;
    }
    if !crate::entity::leash::tick_leash(entity, store, level) {
        return false;
    }
    if fire_hurt_refused {
        return false;
    }
    if entity.living().is_none() {
        return false;
    }
    if let Some(living) = entity.living_mut() {
        for entry in bbb_protocol::equipment_slots::VANILLA_EQUIPMENT_SLOTS {
            let Some(stack) = living.item_by_slot(entry.slot) else {
                continue;
            };
            if let Some(updated) = level.updated_lodestone_compass(stack) {
                let _ = living.update_item_slot(entry.slot, updated);
            }
        }
    }
    if entity.is_alive() {
        if entity.living().is_some_and(|living| living.health() > 0.0) {
            match if entity.sleeping_pos().is_some() {
                crate::entity::environment::KnownContact::Known(false)
            } else {
                crate::entity::environment::is_in_wall(entity, level)
            } {
                crate::entity::environment::KnownContact::Unknown => return false,
                crate::entity::environment::KnownContact::Known(true) => {
                    if !hurt(
                        entity,
                        store,
                        level,
                        LivingBaseTickDamage::InWall { amount: 1.0 },
                    ) {
                        return false;
                    }
                }
                crate::entity::environment::KnownContact::Known(false) => {}
            }
        }
        let Some(eye_in_water) =
            crate::entity::general_living::eye_water_contact_from_tracker(entity, level)
        else {
            return false;
        };
        let can_breathe_underwater = if eye_in_water {
            crate::entity::general_living::can_breathe_underwater_membership(level, entity)
        } else {
            false
        };
        let effect_breathes = entity.living().is_some_and(|living| {
            [
                "minecraft:water_breathing",
                "minecraft:conduit_power",
                "minecraft:breath_of_the_nautilus",
            ]
            .iter()
            .any(|resource_id| living.active_effect(resource_id).is_some())
        });
        let effects_refill_air = entity.living().is_none_or(|living| {
            living
                .active_effect("minecraft:breath_of_the_nautilus")
                .is_none()
                || living.active_effect("minecraft:water_breathing").is_some()
                || living.active_effect("minecraft:conduit_power").is_some()
        });
        match eye_in_water {
            true if !can_breathe_underwater && !effect_breathes => {
                // `LivingEntity.decreaseAirSupply` (`LivingEntity.java:565-575`).
                let oxygen_bonus = entity.living().map_or(0.0, |living| {
                    living.attribute_value(
                        bbb_protocol::attribute_types::VANILLA_ATTRIBUTE_OXYGEN_BONUS_ID,
                    )
                });
                // `IronGolem.decreaseAirSupply` (`IronGolem.java:100-102`).
                let air = if entity.entity_type_id()
                    == bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_IRON_GOLEM_ID
                    || oxygen_bonus > 0.0
                        && store.random().next_double() >= 1.0 / (oxygen_bonus + 1.0)
                {
                    entity.air_supply
                } else {
                    entity.air_supply.wrapping_sub(1)
                };
                if air <= -20 {
                    entity.set_air_supply(0);
                    let entity_id = entity.id();
                    store.broadcast_to_tracking(
                        entity_id,
                        crate::connection::ClientboundPlayPacket::EntityEvent(
                            bbb_protocol::packets::EntityEvent {
                                entity_id,
                                event_id: 67,
                            },
                        ),
                    );
                    store.queue_command(crate::entity::EntityCommand::Projectile(
                        PendingProjectileAction::Hurt(PendingProjectileHurt {
                            target: crate::level_event::PendingHurtEntity::Stored { entity_id },
                            damage_type: "minecraft:drown",
                            amount: 2.0,
                            causing: None,
                            direct: None,
                            direct_resource_id: "minecraft:drown",
                            registry_tags: level.capture_registry_tags(),
                        }),
                    ));
                } else {
                    entity.set_air_supply(air);
                }
            }
            true if effects_refill_air && entity.air_supply < max_air_supply(entity) => {
                let air = increase_air_supply(entity, entity.air_supply);
                entity.set_air_supply(air);
            }
            false if entity.air_supply < max_air_supply(entity) => {
                let air = increase_air_supply(entity, entity.air_supply);
                entity.set_air_supply(air);
            }
            _ => {}
        }
        crate::entity::location_effects::check_changed_block(entity, store, level);
    }
    let entity_id = entity.id();
    let entity_resource_id = entity.data().entity_resource_id();
    let entity_tick_count = entity.tick_count();
    let living = entity.living_mut().expect("checked above");
    // `:444-446`.
    if living.hurt_time > 0 {
        living.hurt_time -= 1;
    }
    // `:448-450`; this is never a `ServerPlayer`.
    if living.invulnerable_time > 0 {
        living.invulnerable_time -= 1;
    }
    if living.is_dead_or_dying() {
        tick_death(entity, store);
    }
    let living = entity.living_mut().expect("checked above");
    // `:456-460`.
    if living.last_hurt_by_player_memory_time > 0 {
        living.last_hurt_by_player_memory_time -= 1;
    } else {
        living.last_hurt_by_player = None;
    }
    if living.last_damage_memory_time > 0 {
        living.last_damage_memory_time -= 1;
    } else {
        living.last_damage_type = None;
        living.last_damage_source_entity = None;
    }
    // `:470-477`: the remembered attacker is dropped once the entity it
    // resolves to is dead, and the hundred-tick window closes a reference that
    // is still alive. A reference its lookup does not resolve at all keeps the
    // memory, window or not: `getLastHurtByMob` is null there and the whole
    // block is skipped. The lookup is the one `EntityReference.getEntity` runs
    // (`entity_reference.rs`), over this store, the level's own sessions and
    // the projection of the levels beside it.
    if let Some(reference) = living.last_hurt_by_mob {
        let expired =
            entity_tick_count - living.last_hurt_by_mob_timestamp > LAST_HURT_BY_MOB_MEMORY_TICKS;
        match reference.resolve(store, level) {
            Some(target) if target.alive && !expired => {}
            Some(_) => living.set_last_hurt_by_mob(None, entity_tick_count),
            None => {}
        }
    }
    let living = entity.living_mut().expect("checked above");
    let health_before = living.health().to_bits();
    let effect_tick = living.tick_active_effects(level, entity_resource_id, entity_tick_count);
    if living.health().to_bits() != health_before {
        entity.mark_synched_dirty(VANILLA_LIVING_ENTITY_HEALTH_DATA_ID);
    }
    for effect in &effect_tick.refreshed {
        mob_effects::send_effect_update(store, entity_id, effect);
    }
    mob_effects::send_effect_removals(store, entity_id, &effect_tick.removed);
    for hurt in effect_tick.hurts {
        store.queue_command(crate::entity::EntityCommand::Projectile(
            PendingProjectileAction::Hurt(PendingProjectileHurt {
                target: crate::level_event::PendingHurtEntity::Stored { entity_id },
                damage_type: hurt.damage_type,
                amount: hurt.amount,
                causing: None,
                direct: None,
                direct_resource_id: hurt.damage_type,
                registry_tags: level.capture_registry_tags(),
            }),
        ));
    }
    if refresh_dirty_attributes_with_level(entity, level, store).is_none() {
        return false;
    }
    store.refile_detached(entity);
    mob_effects::synchronize_metadata(entity);
    // LivingEntity.java:3057-3064: these effects clear fall distance even without travel.
    if entity.living().is_some_and(|living| {
        living.active_effect("minecraft:slow_falling").is_some()
            || living.active_effect("minecraft:levitation").is_some()
    }) {
        entity.reset_fall_distance();
    }
    let living = entity.living_mut().expect("checked above");
    // `:479-480`; `yRotO`/`xRotO` (`:481-482`) are not kept by [`Entity`].
    living.y_head_rot_o = living.y_head_rot;
    living.y_body_rot_o = living.y_body_rot;
    true
}

pub(crate) fn apply_living_inside_block_effects_and_freezing<F>(
    entity: &mut Entity,
    store: &mut EntityStore,
    level: &mut LevelCtx<'_, '_>,
    mut hurt: F,
) -> bool
where
    F: FnMut(
        &mut Entity,
        &mut EntityStore,
        &mut LevelCtx<'_, '_>,
        &'static str,
        f32,
    ) -> Option<bool>,
{
    if !crate::entity::inside_blocks::apply_living_inside_block_effects(
        entity,
        level,
        Some(store),
        |entity, level, store, callback| {
            inside_contact::apply_callback(
                entity,
                store.expect("living contact retained actual store"),
                level,
                callback,
                &mut hurt,
            )
        },
    ) {
        return false;
    }
    mob_effects::synchronize_metadata(entity);
    match crate::entity::environment::apply_living_freeze_suffix(entity, level) {
        crate::entity::environment::KnownContact::Unknown => false,
        crate::entity::environment::KnownContact::Known(true) => {
            hurt(entity, store, level, "minecraft:freeze", 1.0).is_some()
        }
        crate::entity::environment::KnownContact::Known(false) => true,
    }
}

pub(crate) fn living_sound_source(entity: &Entity) -> SoundSource {
    use bbb_facts::entity_sound_sources::{
        vanilla_entity_sound_source_kind, VanillaEntitySoundSourceKind,
    };
    match vanilla_entity_sound_source_kind(entity.entity_type_id()) {
        Some(VanillaEntitySoundSourceKind::Static(source)) => source,
        Some(VanillaEntitySoundSourceKind::Dynamic) => {
            let evil = match entity.data() {
                EntityData::GeneralLiving(data) => data.rabbit_retained().is_some_and(|state| {
                    state.variant() == bbb_protocol::builtin_enum_component_values::VANILLA_RABBIT_VARIANT_EVIL_ID
                }),
                _ => false,
            };
            if evil {
                SoundSource::Hostile
            } else {
                SoundSource::Neutral
            }
        }
        None => SoundSource::Neutral,
    }
}

pub(crate) fn apply_eyeblossom_effect(
    entity: &mut Entity,
    store: &mut EntityStore,
    level: &LevelCtx<'_, '_>,
    state: i32,
    duration: i32,
) -> bool {
    let Some(difficulty) = store.difficulty() else {
        return false;
    };
    if difficulty == Difficulty::Peaceful {
        return true;
    }
    let Some(block_name) = bbb_protocol::block_state_types::vanilla_block_state_name(state) else {
        return false;
    };
    if !level.registry_tag_membership("minecraft:block", "minecraft:bee_attractive", block_name) {
        return true;
    }
    let Some(living) = entity.living() else {
        return false;
    };
    if living.active_effect("minecraft:poison").is_some() {
        return true;
    }
    let effect = ActiveMobEffect::new("minecraft:poison", duration, 0)
        .expect("the built-in poison duration is valid");
    mob_effects::add_active_effect_in_level(entity, store, level, effect).is_some()
}

pub(crate) fn apply_wither_rose_effect(
    entity: &mut Entity,
    store: &mut EntityStore,
    level: &mut LevelCtx<'_, '_>,
    duration: i32,
) -> bool {
    let Some(difficulty) = store.difficulty() else {
        return false;
    };
    if difficulty == Difficulty::Peaceful {
        return true;
    }
    let Some(wither) = builtin_damage_type_registry()
        .iter()
        .find(|facts| facts.resource_id == "minecraft:wither")
    else {
        return false;
    };
    let member = |registry_id, tag_id, resource_id| {
        Some(level.registry_tag_membership(registry_id, tag_id, resource_id))
    };
    let Some(bypasses_invulnerability) = member(
        "minecraft:damage_type",
        "minecraft:bypasses_invulnerability",
        "minecraft:wither",
    ) else {
        return false;
    };
    let Some(is_fire) = member(
        "minecraft:damage_type",
        "minecraft:is_fire",
        "minecraft:wither",
    ) else {
        return false;
    };
    let Some(is_fall) = member(
        "minecraft:damage_type",
        "minecraft:is_fall",
        "minecraft:wither",
    ) else {
        return false;
    };
    let fall_damage_immune = if is_fall {
        let Some(immune) = member(
            "minecraft:entity_type",
            "minecraft:fall_damage_immune",
            entity.data().entity_resource_id(),
        ) else {
            return false;
        };
        immune
    } else {
        false
    };
    if is_invulnerable_to_with_tags(
        entity,
        false,
        fall_damage_immune,
        bypasses_invulnerability,
        is_fire,
        is_fall,
    ) {
        return true;
    }
    let worn = match crate::entity::enchanted_damage::worn_enchantments(store, entity) {
        Ok(worn) => worn,
        Err(_) => return false,
    };
    if !worn.is_empty() {
        let source = crate::entity::damage::StoredEntityDamageSource::without_attacker_views(
            wither, None, None,
        );
        match crate::entity::mob_death_loot::with_enchanted_damage_request(
            store,
            level,
            entity,
            &source,
            crate::entity::enchanted_damage::EnchantedDamageItems::Listed(&worn),
            |level, request| level.enchanted_damage_immunity(request),
        ) {
            Ok(true) => return true,
            Ok(false) => {}
            Err(_) => return false,
        }
    }
    let effect = ActiveMobEffect::new("minecraft:wither", duration, 0)
        .expect("the built-in wither duration is valid");
    mob_effects::add_active_effect_in_level(entity, store, level, effect).is_some()
}

/// `EntityEvent.POOF` (`net/minecraft/world/entity/EntityEvent.java:55`).
pub const ENTITY_EVENT_POOF: i8 = 60;

/// `EntityEvent.DEATH` (`net/minecraft/world/entity/EntityEvent.java:6`), the
/// animation `LivingEntity.die` broadcasts after the emission gate (`:1452`).
pub const ENTITY_EVENT_DEATH: i8 = 3;

fn tick_death(entity: &mut Entity, store: &mut EntityStore) {
    // `EnderDragon.tickDeath` replaces this; the dragon's own tick runs it.
    if entity.entity_type_id() == bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_ENDER_DRAGON_ID {
        return;
    }
    let tearing_down = entity.is_heart_bound() && entity.creaking_is_tearing_down();
    let Some(living) = entity.living_mut() else {
        return;
    };
    living.death_time += 1;
    if tearing_down {
        return;
    }
    if living.death_time >= DEATH_DURATION && entity.is_alive() {
        store.broadcast_to_tracking(
            entity.id(),
            crate::connection::ClientboundPlayPacket::EntityEvent(
                bbb_protocol::packets::EntityEvent {
                    entity_id: entity.id(),
                    event_id: ENTITY_EVENT_POOF,
                },
            ),
        );
        entity.set_removed(crate::entity::RemovalReason::Killed);
    }
}

pub fn apply_movement_dead_band(entity: &mut Entity) {
    let movement = entity.delta_movement();
    let dx = if movement.x.abs() < MIN_MOVEMENT_DISTANCE {
        0.0
    } else {
        movement.x
    };
    let dy = if movement.y.abs() < MIN_MOVEMENT_DISTANCE {
        0.0
    } else {
        movement.y
    };
    let dz = if movement.z.abs() < MIN_MOVEMENT_DISTANCE {
        0.0
    } else {
        movement.z
    };
    entity.set_delta_movement(Vec3d {
        x: dx,
        y: dy,
        z: dz,
    });
}

pub fn apply_input(living: &mut LivingEntityState) {
    living.xxa *= 0.98;
    living.zza *= 0.98;
}

pub fn apply_immobile(living: &mut LivingEntityState) {
    living.jumping = false;
    living.xxa = 0.0;
    living.zza = 0.0;
}

pub(crate) const fn base_ground_jump_supported(entity_type_id: i32) -> bool {
    !matches!(
        entity_type_id,
        VANILLA_ENTITY_TYPE_RABBIT_ID
            | VANILLA_ENTITY_TYPE_SLIME_ID
            | VANILLA_ENTITY_TYPE_MAGMA_CUBE_ID
            | VANILLA_ENTITY_TYPE_SNIFFER_ID
    )
}

/// `Entity.getFluidJumpThreshold` (`Entity.java:3600-3601`) and
/// `Breeze.getFluidJumpThreshold` (`Breeze.java:268-269`).
pub(crate) fn fluid_jump_threshold(entity: &Entity) -> f64 {
    let eye_height = f64::from(entity.eye_height());
    if entity.entity_type_id() == bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_BREEZE_ID {
        eye_height
    } else if eye_height < 0.4 {
        0.0
    } else {
        0.4
    }
}

/// `LivingEntity.aiStep` (`LivingEntity.java:3023-3047`), including
/// `jumpInLiquid` (`LivingEntity.java:2361-2362`).
pub(crate) fn tick_jump(entity: &mut Entity, level: &LevelCtx<'_, '_>) {
    let Some(living) = entity.living_mut() else {
        return;
    };
    if living.no_jump_delay > 0 {
        living.no_jump_delay -= 1;
    }
    if !living.jumping {
        living.no_jump_delay = 0;
        return;
    }
    let on_ground = entity.on_ground();
    let in_lava = entity.is_in_lava();
    let fluid_height = entity.fluid_height(if in_lava {
        crate::entity::environment::FluidTag::Lava
    } else {
        crate::entity::environment::FluidTag::Water
    });
    let in_water = entity.is_in_water() && fluid_height > 0.0;
    let threshold = fluid_jump_threshold(entity);
    if (in_water || in_lava) && (!on_ground || fluid_height > threshold) {
        entity.set_delta_movement(Vec3d {
            y: entity.delta_movement().y + f64::from(0.04_f32),
            ..entity.delta_movement()
        });
        return;
    }
    if !(on_ground || (in_water && fluid_height <= threshold))
        || entity.living().expect("checked above").no_jump_delay > 0
    {
        return;
    }

    if base_ground_jump_supported(entity.entity_type_id()) {
        jump_from_ground(entity, level);
    }
    entity.living_mut().expect("checked above").no_jump_delay = 10;
}

// LivingEntity.java:2330-2355.
pub(crate) fn jump_from_ground(entity: &mut Entity, level: &LevelCtx<'_, '_>) {
    let Some(living) = entity.living() else {
        return;
    };
    if let Some(movement) = body_ground_jump_velocity(
        entity.delta_movement(),
        living.attribute_value(VANILLA_ATTRIBUTE_JUMP_STRENGTH_ID),
        entity_block_jump_factor(entity, level),
        living
            .active_effect("minecraft:jump_boost")
            .map(|effect| effect.amplifier()),
        entity.get_shared_flag(VANILLA_ENTITY_SHARED_FLAG_SPRINTING),
        entity.y_rot(),
    ) {
        entity.set_delta_movement(movement);
        entity.set_needs_sync(true);
    }
}

pub(crate) fn entity_block_jump_factor(entity: &Entity, level: &LevelCtx<'_, '_>) -> f32 {
    let known_or_air = |state| {
        if vanilla_block_state_block_type_id(state).is_some() {
            state
        } else {
            0
        }
    };
    body_block_jump_factor(
        known_or_air(level.block_state(entity.block_position())),
        known_or_air(level.block_state(entity.block_pos_below_that_affects_my_movement())),
    )
    .expect("unknown states use the existing ordinary jump factor")
}

fn input_vector(input: Vec3d, speed: f32, y_rot: f32) -> Vec3d {
    let length = input.length_sqr();
    if length < 1.0e-7 {
        return Vec3d {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        };
    }
    let movement = if length > 1.0 {
        input.normalize()
    } else {
        input
    }
    .scale(f64::from(speed));
    // `Mth.sin`/`Mth.cos` are the table lookups; the argument is
    // `yRot * (float)(Math.PI / 180.0)` in `float`.
    let radians = f64::from(y_rot * (std::f32::consts::PI / 180.0));
    let sin = f64::from(bbb_server_types::geometry::mth_sin(radians));
    let cos = f64::from(bbb_server_types::geometry::mth_cos(radians));
    Vec3d {
        x: movement.x * cos - movement.z * sin,
        y: movement.y,
        z: movement.z * cos + movement.x * sin,
    }
}

pub fn move_relative(entity: &mut Entity, speed: f32, input: Vec3d) {
    let delta = input_vector(input, speed, entity.y_rot());
    let movement = entity.delta_movement();
    entity.set_delta_movement(movement.plus(delta));
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FlyingContact {
    Water,
    Lava,
    Air,
    Unknown,
}

// LivingEntity.java:2401-2418.
pub(crate) fn travel_flying(
    entity: &mut Entity,
    store: &mut EntityStore,
    level: &mut LevelCtx<'_, '_>,
    input: Vec3d,
    contact: FlyingContact,
    air_speed: f32,
    fluid_speed: f32,
) -> Result<(), ()> {
    let (speed, drag) = match contact {
        FlyingContact::Water => (fluid_speed, f64::from(0.8_f32)),
        FlyingContact::Lava => (fluid_speed, 0.5),
        FlyingContact::Air => (air_speed, f64::from(0.91_f32)),
        FlyingContact::Unknown => return Err(()),
    };
    move_relative(entity, speed, input);
    let movement = entity.delta_movement();
    crate::entity::movement::move_living_entity(entity, store, level, MoverType::Self_, movement)?;
    entity.set_delta_movement(entity.delta_movement().scale(drag));
    Ok(())
}

fn friction_influenced_speed(
    living: &LivingEntityState,
    on_ground: bool,
    block_friction: f32,
) -> f32 {
    if on_ground {
        living.speed * (0.216_000_02_f32 / (block_friction * block_friction * block_friction))
    } else {
        0.02
    }
}

pub fn gravity(entity: &Entity) -> f64 {
    if entity.no_gravity() {
        return 0.0;
    }
    entity.living().map_or(DEFAULT_BASE_GRAVITY, |living| {
        living.attribute_value(VANILLA_ATTRIBUTE_GRAVITY_ID)
    })
}

pub fn effective_gravity(entity: &Entity) -> f64 {
    let gravity = gravity(entity);
    if entity.delta_movement().y <= 0.0
        && entity
            .living()
            .is_some_and(|living| living.active_effect("minecraft:slow_falling").is_some())
    {
        gravity.min(0.01)
    } else {
        gravity
    }
}

/// `getWaterSlowDown` (`LivingEntity.java:2365-2367`, `PolarBear.java:232-234`,
/// `SkeletonHorse.java:148-150`).
fn water_slow_down(entity_type_id: i32) -> f32 {
    match entity_type_id {
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_POLAR_BEAR_ID => 0.98,
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_SKELETON_HORSE_ID => 0.96,
        _ => 0.8,
    }
}

pub(crate) fn travel_in_water(
    entity: &mut Entity,
    store: &mut EntityStore,
    level: &mut LevelCtx<'_, '_>,
    input: Vec3d,
) -> Result<(), ()> {
    // LivingEntity.java:2458-2484.
    let old_y = entity.position().y;
    let falling = entity.delta_movement().y <= 0.0;
    let gravity = crate::entity::living::effective_gravity(entity);
    let living = entity.living().ok_or(())?;
    let mut drag = if entity.is_sprinting() {
        0.9_f32
    } else {
        water_slow_down(entity.entity_type_id())
    };
    let mut speed = 0.02_f32;
    let mut efficiency = living.attribute_value(
        bbb_protocol::attribute_types::VANILLA_ATTRIBUTE_WATER_MOVEMENT_EFFICIENCY_ID,
    ) as f32;
    if !entity.on_ground() {
        efficiency *= 0.5;
    }
    if efficiency > 0.0 {
        drag += (0.54600006 - drag) * efficiency;
        speed += (living.speed() - speed) * efficiency;
    }
    if living.active_effect("minecraft:dolphins_grace").is_some() {
        drag = 0.96;
    }
    move_relative(entity, speed, input);
    let movement = entity.delta_movement();
    crate::entity::movement::move_living_entity(entity, store, level, MoverType::Self_, movement)?;
    let mut movement = entity.delta_movement();
    if entity.horizontal_collision() {
        match climbable_contact_with_history(entity, level) {
            ClimbableContact::Known(true) => movement.y = 0.2,
            ClimbableContact::Known(false) => {}
            ClimbableContact::Unknown => return Err(()),
        }
    }
    let y = fluid_falling_y(entity, gravity, falling, movement.y * f64::from(0.8_f32));
    entity.set_delta_movement(Vec3d {
        x: movement.x * f64::from(drag),
        y,
        z: movement.z * f64::from(drag),
    });
    jump_out_of_fluid(entity, store, level, old_y)
}

/// `LivingEntity.travelInLava` (`LivingEntity.java:2487-2502`).
pub(crate) fn travel_in_lava(
    entity: &mut Entity,
    store: &mut EntityStore,
    level: &mut LevelCtx<'_, '_>,
    input: Vec3d,
) -> Result<(), ()> {
    let old_y = entity.position().y;
    let falling = entity.delta_movement().y <= 0.0;
    let gravity = effective_gravity(entity);
    move_relative(entity, 0.02, input);
    let movement = entity.delta_movement();
    crate::entity::movement::move_living_entity(entity, store, level, MoverType::Self_, movement)?;
    let movement = entity.delta_movement();
    let mut movement = if entity.fluid_height(crate::entity::environment::FluidTag::Lava)
        <= fluid_jump_threshold(entity)
    {
        Vec3d {
            x: movement.x * 0.5,
            y: fluid_falling_y(entity, gravity, falling, movement.y * f64::from(0.8_f32)),
            z: movement.z * 0.5,
        }
    } else {
        movement.scale(0.5)
    };
    movement.y -= gravity / 4.0;
    entity.set_delta_movement(movement);
    jump_out_of_fluid(entity, store, level, old_y)
}

/// `LivingEntity.getFluidFallingAdjustedMovement` (`LivingEntity.java:2626-2637`).
fn fluid_falling_y(entity: &Entity, gravity: f64, falling: bool, y: f64) -> f64 {
    if gravity == 0.0 || entity.is_sprinting() {
        y
    } else if falling && (y - 0.005).abs() >= 0.003 && (y - gravity / 16.0).abs() < 0.003 {
        -0.003
    } else {
        y - gravity / 16.0
    }
}

fn jump_out_of_fluid(
    entity: &mut Entity,
    store: &mut EntityStore,
    level: &mut LevelCtx<'_, '_>,
    old_y: f64,
) -> Result<(), ()> {
    // LivingEntity.java:2505-2509; Entity.java:653-658.
    if entity.horizontal_collision() {
        let movement = entity.delta_movement();
        let bounds = entity.bounding_box().moved(Vec3d {
            x: movement.x,
            y: movement.y + f64::from(0.6_f32) - entity.position().y + old_y,
            z: movement.z,
        });
        let clear = bbb_server_types::geometry::no_block_collision_with_scratch(
            bounds,
            |pos| level.is_moving_piston_at(pos),
            |pos, out| level.push_block_collision_boxes_for_entity(entity, pos, out),
        )
        .ok_or(())?;
        let clear = clear
            && level.world_border().ok_or(())?.no_collision_at(
                entity.position(),
                bounds,
                level.game_time(),
            );
        let mut clear = clear;
        for id in store.ids_in_box_sections(bounds) {
            let Some(other) = store.get(id) else {
                continue;
            };
            if id != entity.id()
                && other.can_be_collided_with()
                && other.bounding_box().intersects(bounds)
                && !store.checked_same_vehicle(entity, id).ok_or(())?
            {
                clear = false;
                break;
            }
        }
        let dry = !random_teleport::contains_any_liquid(level, bounds).ok_or(())?;
        if clear && dry {
            entity.set_delta_movement(Vec3d {
                y: f64::from(0.3_f32),
                ..movement
            });
        }
    }
    Ok(())
}

///
/// Kept: the block friction read off `getBlockPosBelowThatAffectsMyMovement`
/// when on the ground (`:2422-2423`), `moveRelative` with the
/// friction-influenced speed (`:2615`), the move (`:2617`), the gravity
/// subtraction (`:2430-2431`, the server arm), and the `friction`/`0.98F`
/// scaling (`:2441-2443`).
///
pub(crate) fn travel_in_air(
    entity: &mut Entity,
    store: &mut EntityStore,
    level: &mut LevelCtx<'_, '_>,
    input: Vec3d,
) -> Result<(), ()> {
    if entity.living().is_none() {
        return Ok(());
    }
    let climbable = match climbable_contact_with_history(entity, level) {
        ClimbableContact::Known(climbable) => climbable,
        ClimbableContact::Unknown => return Err(()),
    };
    let living = entity.living().expect("checked before climbable lookup");
    let on_ground = entity.on_ground();
    // `:2422-2423`.
    let pos_below = entity.block_pos_below_that_affects_my_movement();
    let block_friction = if on_ground {
        level.block_friction(pos_below)
    } else {
        1.0
    };
    // `:2424`, a `float` product.
    let friction = block_friction * 0.91_f32;
    // `handleRelativeFrictionAndCalculateMovement` (`:2614-2624`).
    let speed = friction_influenced_speed(living, on_ground, block_friction);
    move_relative(entity, speed, input);
    if climbable {
        entity.reset_fall_distance();
        let movement = entity.delta_movement();
        entity.set_delta_movement(Vec3d {
            x: movement.x.clamp(-0.15, 0.15),
            y: movement.y.max(-0.15),
            z: movement.z.clamp(-0.15, 0.15),
        });
    }
    let delta = entity.delta_movement();
    crate::entity::movement::move_living_entity(entity, store, level, MoverType::Self_, delta)?;
    let mut movement = entity.delta_movement();
    // LivingEntity.java:2619-2621.
    if entity.horizontal_collision() || entity.living().is_some_and(|living| living.jumping) {
        let climbable = match climbable_contact_with_history(entity, level) {
            ClimbableContact::Known(climbable) => climbable,
            ClimbableContact::Unknown => return Err(()),
        };
        if climbable
            || (entity.was_in_powder_snow()
                && crate::entity::inside_blocks::body_contact::can_walk_on_powder_snow(
                    entity.data().entity_resource_id(),
                    entity
                        .living()
                        .and_then(|living| living.item_by_slot(EquipmentSlot::Feet))
                        .and_then(|item| item.item_id()),
                    |registry, tag, name| level.registry_tag_membership(registry, tag, name),
                ))
        {
            movement.y = 0.2;
        }
    }
    // `:2426-2431`.
    let movement_y = entity
        .living()
        .and_then(|living| living.active_effect("minecraft:levitation"))
        .map_or_else(
            || movement.y - effective_gravity(entity),
            |effect| movement.y + (0.05 * f64::from(effect.amplifier() + 1) - movement.y) * 0.2,
        );
    let (x, y, z) = if entity
        .living()
        .is_some_and(|living| living.discard_friction)
    {
        (movement.x, movement_y, movement.z)
    } else {
        let vertical_friction = if matches!(
            entity.entity_type_id(),
            bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_PARROT_ID
                | bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_BEE_ID
        ) {
            friction
        } else {
            0.98_f32
        };
        (
            movement.x * f64::from(friction),
            movement_y * f64::from(vertical_friction),
            movement.z * f64::from(friction),
        )
    };
    entity.set_delta_movement(Vec3d { x, y, z });
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ClimbableContact {
    Known(bool),
    Unknown,
}

pub(crate) fn on_climbable(entity: &mut Entity, level: &LevelCtx<'_, '_>) -> Option<bool> {
    match climbable_contact_with_history(entity, level) {
        ClimbableContact::Known(climbable) => Some(climbable),
        ClimbableContact::Unknown => None,
    }
}

fn climbable_contact_with_history(
    entity: &mut Entity,
    level: &LevelCtx<'_, '_>,
) -> ClimbableContact {
    let contact = climbable_contact(entity, level);
    if contact == ClimbableContact::Known(true) {
        let pos = entity.block_position();
        if let Some(living) = entity.living_mut() {
            living.set_last_climbable_pos(Some(pos));
        }
    }
    contact
}

fn climbable_contact(entity: &Entity, level: &LevelCtx<'_, '_>) -> ClimbableContact {
    // `Spider.onClimbable` and `tick` (`Spider.java:81-86`, `:113-115`).
    if entity.is_spider() {
        return ClimbableContact::Known(entity.horizontal_collision());
    }
    climbable_contact_at(level, entity.block_position(), entity.is_fall_flying())
}

/// [`on_climbable`] for a body the store does not hold, read off its block
/// position and fall-flying flag.
pub(crate) fn on_climbable_at(
    level: &LevelCtx<'_, '_>,
    pos: BlockPos,
    fall_flying: bool,
) -> Option<bool> {
    match climbable_contact_at(level, pos, fall_flying) {
        ClimbableContact::Known(climbable) => Some(climbable),
        ClimbableContact::Unknown => None,
    }
}

fn climbable_contact_at(
    level: &LevelCtx<'_, '_>,
    pos: BlockPos,
    fall_flying: bool,
) -> ClimbableContact {
    match on_climbable_in_blocks(
        pos,
        fall_flying,
        |pos| Some(level.block_state(pos)),
        |state, tag| block_state_tag_membership(level, state, tag),
    ) {
        Some(climbable) => ClimbableContact::Known(climbable),
        None => ClimbableContact::Unknown,
    }
}

pub fn tick_head_turn(entity: &mut Entity, y_body_rot_t: f32, max_head_rotation: f32) {
    let y_rot = entity.y_rot();
    let Some(living) = entity.living_mut() else {
        return;
    };
    let y_body_rot_d = wrap_degrees(y_body_rot_t - living.y_body_rot);
    living.y_body_rot += y_body_rot_d * 0.3;
    let head_diff = wrap_degrees(y_rot - living.y_body_rot);
    if head_diff.abs() > max_head_rotation {
        living.y_body_rot += head_diff - head_diff.signum() * max_head_rotation;
    }
}

pub fn wrap_degrees(angle: f32) -> f32 {
    let mut normalized = angle % 360.0;
    if normalized >= 180.0 {
        normalized -= 360.0;
    }
    if normalized < -180.0 {
        normalized += 360.0;
    }
    normalized
}

pub fn living_block_speed_factor(living: &LivingEntityState, base: f32) -> f32 {
    let efficiency = living.attribute_value(VANILLA_ATTRIBUTE_MOVEMENT_EFFICIENCY_ID) as f32;
    body_movement_efficiency_speed_factor(efficiency, base)
}

pub fn body_movement_efficiency_speed_factor(efficiency: f32, base: f32) -> f32 {
    base + efficiency * (1.0 - base)
}

pub fn max_up_step(living: &LivingEntityState) -> f32 {
    living.attribute_value(VANILLA_ATTRIBUTE_STEP_HEIGHT_ID) as f32
}

pub fn voice_pitch(store: &mut EntityStore, baby: bool) -> f32 {
    let first = store.random().next_float();
    let second = store.random().next_float();
    if baby {
        (first - second) * 0.2 + 1.5
    } else {
        (first - second) * 0.2 + 1.0
    }
}

const _: () = {
    // `bbb_server_datapack::damage` spells the same three constants for the player
    // pipeline; the living layer reads its own names and pins them equal.
    assert!(DEATH_DURATION == DEATH_TICKS);
    assert!(INVULNERABLE_DURATION == INVULNERABLE_TICKS);
};
