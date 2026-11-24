//! `net/minecraft/world/entity/decoration/ArmorStand.java`, the store's first
//! `LivingEntity`. The `LivingEntity` behaviour it reaches lives in
//! [`crate::entity::living`]; this module holds what `ArmorStand` itself
//! overrides or adds.

use crate::world::simulation_level::LevelCtx;
use bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_EQUIPPABLE_TYPE_ID;
use bbb_protocol::entity_data_serializers::{
    VANILLA_ENTITY_DATA_SERIALIZER_BYTE_ID, VANILLA_ENTITY_DATA_SERIALIZER_ROTATIONS_ID,
};
use bbb_protocol::entity_synched_data::{
    VANILLA_ARMOR_STAND_BODY_POSE_DATA_ID, VANILLA_ARMOR_STAND_CLIENT_FLAGS_DATA_ID,
    VANILLA_ARMOR_STAND_CLIENT_FLAG_MARKER, VANILLA_ARMOR_STAND_CLIENT_FLAG_NO_BASEPLATE,
    VANILLA_ARMOR_STAND_CLIENT_FLAG_SHOW_ARMS, VANILLA_ARMOR_STAND_CLIENT_FLAG_SMALL,
    VANILLA_ARMOR_STAND_HEAD_POSE_DATA_ID, VANILLA_ARMOR_STAND_LEFT_ARM_POSE_DATA_ID,
    VANILLA_ARMOR_STAND_LEFT_LEG_POSE_DATA_ID, VANILLA_ARMOR_STAND_RIGHT_ARM_POSE_DATA_ID,
    VANILLA_ARMOR_STAND_RIGHT_LEG_POSE_DATA_ID,
};
use bbb_protocol::entity_types::{
    vanilla_entity_type_dimensions, VANILLA_ENTITY_TYPE_ARMOR_STAND_ID,
};
use bbb_protocol::equipment_slots::{
    vanilla_equipment_slot_by_ordinal, EquipmentSlot, EquipmentSlotType, VANILLA_EQUIPMENT_SLOTS,
};
use bbb_protocol::packets::{
    BlockPos, EntityDataValue, EntityDataValueKind, EntityEvent, ItemStackSummary, SoundSource,
    Vec3d,
};
use bbb_server_types::item_stack::{static_item_component, RuntimeItemStack};

use crate::entity::block_drops::item_entity_height;
use crate::entity::damage::{
    StoredDamageCausingEntity, StoredEntityDamageContext, StoredEntityDamageRules,
    StoredEntityDamageSource, StoredEntityHurtOutcome,
};
use crate::entity::item::plain_stack;
use crate::entity::living::{
    self, apply_immobile, apply_input, apply_movement_dead_band, calculate_fall_damage,
    fall_damage_sounds, ignite_living_for_seconds, living_base_tick_checked,
    publish_equipment_changes, travel_in_air, LivingBaseTickDamage, LivingEntityState,
    SynchedValue,
};
use crate::entity::store::{EntityStore, PendingEntitySound};
use crate::entity::{Entity, EntityData, RemovalReason};
use crate::level_event::VanillaGameEvent;
use bbb_server_datapack::damage::builtin_damage_type_registry;

/// `ArmorStand.WOBBLE_TIME` (`ArmorStand.java:53`).
pub const WOBBLE_TIME: i64 = 5;
/// `ArmorStand.DISABLE_TAKING_OFFSET`/`DISABLE_PUTTING_OFFSET`
/// (`ArmorStand.java:67-68`).
pub const DISABLE_TAKING_OFFSET: i32 = 8;
pub const DISABLE_PUTTING_OFFSET: i32 = 16;
/// The entity event `ArmorStand.hurtServer` broadcasts for a wobble
/// (`ArmorStand.java:330`, handled at `:344-354`).
pub const ENTITY_EVENT_ARMOR_STAND_HIT: i8 = 32;

/// `SoundEvents.ARMOR_STAND_BREAK`/`ARMOR_STAND_FALL`/`ARMOR_STAND_HIT`
/// (`net/minecraft/sounds/SoundEvents.java:105-107`).
pub const ARMOR_STAND_BREAK_SOUND: &str = "minecraft:entity.armor_stand.break";
pub const ARMOR_STAND_FALL_SOUND: &str = "minecraft:entity.armor_stand.fall";
pub const ARMOR_STAND_HIT_SOUND: &str = "minecraft:entity.armor_stand.hit";

const ARMOR_STAND_RESOURCE_ID: &str = "minecraft:armor_stand";
const ARMOR_STAND_ITEM_RESOURCE_ID: &str = "minecraft:armor_stand";

#[derive(Debug, Clone, Copy)]
pub struct Rotations {
    x: f32,
    y: f32,
    z: f32,
}

impl Rotations {
    pub const fn new(x: f32, y: f32, z: f32) -> Self {
        Self {
            x: Self::canonical(x),
            y: Self::canonical(y),
            z: Self::canonical(z),
        }
    }

    const fn canonical(value: f32) -> f32 {
        if value.is_finite() {
            value % 360.0
        } else {
            0.0
        }
    }

    pub const fn x(self) -> f32 {
        self.x
    }

    pub const fn y(self) -> f32 {
        self.y
    }

    pub const fn z(self) -> f32 {
        self.z
    }

    pub fn components(self) -> [f32; 3] {
        [self.x, self.y, self.z]
    }

    fn data_value(self) -> EntityDataValueKind {
        EntityDataValueKind::Rotations {
            x: self.x,
            y: self.y,
            z: self.z,
        }
    }
}

impl PartialEq for Rotations {
    fn eq(&self, other: &Self) -> bool {
        self.x.to_bits() == other.x.to_bits()
            && self.y.to_bits() == other.y.to_bits()
            && self.z.to_bits() == other.z.to_bits()
    }
}

impl Eq for Rotations {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ArmorStandPose {
    pub head: Rotations,
    pub body: Rotations,
    pub left_arm: Rotations,
    pub right_arm: Rotations,
    pub left_leg: Rotations,
    pub right_leg: Rotations,
}

impl ArmorStandPose {
    /// `ArmorStandPose.DEFAULT` over the six defaults (`ArmorStand.java:55-60`).
    pub const DEFAULT: Self = Self {
        head: Rotations::new(0.0, 0.0, 0.0),
        body: Rotations::new(0.0, 0.0, 0.0),
        left_arm: Rotations::new(-10.0, 0.0, -10.0),
        right_arm: Rotations::new(-15.0, 0.0, 10.0),
        left_leg: Rotations::new(-1.0, 0.0, -1.0),
        right_leg: Rotations::new(1.0, 0.0, 1.0),
    };

    pub fn members(&self) -> [(&'static str, Rotations, Rotations); 6] {
        [
            ("Head", self.head, Self::DEFAULT.head),
            ("Body", self.body, Self::DEFAULT.body),
            ("LeftArm", self.left_arm, Self::DEFAULT.left_arm),
            ("RightArm", self.right_arm, Self::DEFAULT.right_arm),
            ("LeftLeg", self.left_leg, Self::DEFAULT.left_leg),
            ("RightLeg", self.right_leg, Self::DEFAULT.right_leg),
        ]
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ArmorStandData {
    living: Box<LivingEntityState>,
    invisible: bool,
    /// `ArmorStand.lastHit` (`:88`).
    last_hit: i64,
    /// `ArmorStand.disabledSlots` (`:89`).
    disabled_slots: i32,
    /// `DATA_CLIENT_FLAGS` (`:73`).
    client_flags: i8,
    pose: ArmorStandPose,
}

impl ArmorStandData {
    pub(crate) fn prepare_command_flags(&mut self, previous: &Self) {
        self.client_flags = previous.client_flags;
    }

    pub(crate) fn restore_command_transients(&mut self, previous: &Self) {
        self.last_hit = previous.last_hit;
        self.living.restore_command_transients(&previous.living);
    }

    pub fn new() -> Option<Self> {
        Some(Self {
            living: Box::new(LivingEntityState::new(ARMOR_STAND_RESOURCE_ID)?),
            invisible: false,
            last_hit: 0,
            disabled_slots: 0,
            client_flags: 0,
            pose: ArmorStandPose::DEFAULT,
        })
    }

    pub fn living(&self) -> &LivingEntityState {
        &self.living
    }

    pub fn living_mut(&mut self) -> &mut LivingEntityState {
        &mut self.living
    }

    /// `ArmorStand.isSmall()` (`ArmorStand.java:485-487`).
    pub const fn is_small(&self) -> bool {
        self.client_flags & VANILLA_ARMOR_STAND_CLIENT_FLAG_SMALL != 0
    }

    /// `ArmorStand.showArms()` (`:493-495`).
    pub const fn show_arms(&self) -> bool {
        self.client_flags & VANILLA_ARMOR_STAND_CLIENT_FLAG_SHOW_ARMS != 0
    }

    /// `ArmorStand.showBasePlate()` (`:501-503`).
    pub const fn show_base_plate(&self) -> bool {
        self.client_flags & VANILLA_ARMOR_STAND_CLIENT_FLAG_NO_BASEPLATE == 0
    }

    /// `ArmorStand.isMarker()` (`:509-511`).
    pub const fn is_marker(&self) -> bool {
        self.client_flags & VANILLA_ARMOR_STAND_CLIENT_FLAG_MARKER != 0
    }

    /// `ArmorStand.isBaby()` (`:455-458`).
    pub const fn is_baby(&self) -> bool {
        self.is_small()
    }

    pub const fn client_flags(&self) -> i8 {
        self.client_flags
    }

    pub const fn disabled_slots(&self) -> i32 {
        self.disabled_slots
    }

    pub const fn last_hit(&self) -> i64 {
        self.last_hit
    }

    pub const fn invisible(&self) -> bool {
        self.invisible
    }

    pub const fn pose(&self) -> &ArmorStandPose {
        &self.pose
    }

    /// `ArmorStand.hasPhysics()` (`ArmorStand.java:113-115`).
    pub const fn has_physics(&self, no_gravity: bool) -> bool {
        !self.is_marker() && !no_gravity
    }

    pub fn can_use_slot(&self, slot: EquipmentSlot) -> bool {
        slot != EquipmentSlot::Body && slot != EquipmentSlot::Saddle && !self.is_disabled(slot)
    }

    fn is_disabled(&self, slot: EquipmentSlot) -> bool {
        self.disabled_slots & (1 << filter_bit(slot, 0)) != 0
            || slot_type(slot) == EquipmentSlotType::Hand && !self.show_arms()
    }
}

fn slot_entry(
    slot: EquipmentSlot,
) -> &'static bbb_protocol::equipment_slots::EquipmentSlotCatalogEntry {
    vanilla_equipment_slot_by_ordinal(slot.ordinal())
        .expect("every EquipmentSlot variant has a catalog row")
}

fn filter_bit(slot: EquipmentSlot, offset: i32) -> i32 {
    slot_entry(slot).id + offset
}

/// `EquipmentSlot.getType()`.
fn slot_type(slot: EquipmentSlot) -> EquipmentSlotType {
    slot_entry(slot).slot_type
}

pub fn armor_stand(entity: &Entity) -> Option<&ArmorStandData> {
    match entity.data() {
        EntityData::ArmorStand(data) => Some(data),
        _ => None,
    }
}

pub fn armor_stand_mut(entity: &mut Entity) -> Option<&mut ArmorStandData> {
    match &mut entity.data {
        EntityData::ArmorStand(data) => Some(data),
        _ => None,
    }
}

pub fn dimensions(data: &ArmorStandData) -> [f32; 2] {
    let [width, height, _] = geometry(data);
    [width, height]
}

pub(crate) fn geometry(data: &ArmorStandData) -> [f32; 3] {
    if data.is_marker() {
        return [0.0, 0.0, 0.0];
    }
    let [width, height] = vanilla_entity_type_dimensions(VANILLA_ENTITY_TYPE_ARMOR_STAND_ID)
        .expect("the generated catalog carries minecraft:armor_stand");
    let [width, height, eye_height] = if data.is_baby() {
        [width * 0.5, height * 0.5, 0.9875]
    } else {
        [
            width,
            height,
            bbb_protocol::entity_types::vanilla_entity_type_eye_height(
                VANILLA_ENTITY_TYPE_ARMOR_STAND_ID,
            )
            .expect("generated armor stand eye height"),
        ]
    };
    let scale = data
        .living
        .attribute_value(bbb_protocol::attribute_types::VANILLA_ATTRIBUTE_SCALE_ID)
        as f32;
    if scale == 1.0 {
        [width, height, eye_height]
    } else {
        [width * scale, height * scale, eye_height * scale]
    }
}

pub fn refresh_dimensions(entity: &mut Entity) {
    if entity.pose_id() == bbb_protocol::entity_poses::VANILLA_ENTITY_POSE_SLEEPING_ID {
        entity.set_dimensions_and_eye(0.2, 0.2, 0.2);
        return;
    }
    let Some(data) = armor_stand(entity) else {
        return;
    };
    let [width, height, eye_height] = geometry(data);
    entity.set_dimensions_and_eye(width, height, eye_height);
}

fn set_client_flag(entity: &mut Entity, bit: i8, value: bool) {
    let Some(data) = armor_stand_mut(entity) else {
        return;
    };
    let flags = if value {
        data.client_flags | bit
    } else {
        data.client_flags & !bit
    };
    if flags == data.client_flags {
        return;
    }
    data.client_flags = flags;
    entity.mark_synched_dirty(VANILLA_ARMOR_STAND_CLIENT_FLAGS_DATA_ID);
    refresh_dimensions(entity);
}

pub fn set_small(entity: &mut Entity, value: bool) {
    set_client_flag(entity, VANILLA_ARMOR_STAND_CLIENT_FLAG_SMALL, value);
}

/// `ArmorStand.setShowArms` (`:489-491`).
pub fn set_show_arms(entity: &mut Entity, value: bool) {
    set_client_flag(entity, VANILLA_ARMOR_STAND_CLIENT_FLAG_SHOW_ARMS, value);
}

/// `ArmorStand.setNoBasePlate` (`:497-499`).
pub fn set_no_base_plate(entity: &mut Entity, value: bool) {
    set_client_flag(entity, VANILLA_ARMOR_STAND_CLIENT_FLAG_NO_BASEPLATE, value);
}

/// `ArmorStand.setMarker` (`:505-507`).
pub fn set_marker(entity: &mut Entity, value: bool) {
    set_client_flag(entity, VANILLA_ARMOR_STAND_CLIENT_FLAG_MARKER, value);
}

pub fn set_invisible(entity: &mut Entity, value: bool) {
    if let Some(data) = armor_stand_mut(entity) {
        data.invisible = value;
    }
    entity.set_invisible(value);
}

pub fn set_disabled_slots(entity: &mut Entity, disabled_slots: i32) {
    if let Some(data) = armor_stand_mut(entity) {
        data.disabled_slots = disabled_slots;
    }
}

fn set_pose_member(
    entity: &mut Entity,
    data_id: u8,
    member: fn(&mut ArmorStandPose) -> &mut Rotations,
    value: Rotations,
) {
    let Some(data) = armor_stand_mut(entity) else {
        return;
    };
    let slot = member(&mut data.pose);
    if *slot == value {
        return;
    }
    *slot = value;
    entity.mark_synched_dirty(data_id);
}

pub fn set_head_pose(entity: &mut Entity, value: Rotations) {
    set_pose_member(
        entity,
        VANILLA_ARMOR_STAND_HEAD_POSE_DATA_ID,
        |pose| &mut pose.head,
        value,
    );
}

/// `ArmorStand.setBodyPose` (`:527-529`).
pub fn set_body_pose(entity: &mut Entity, value: Rotations) {
    set_pose_member(
        entity,
        VANILLA_ARMOR_STAND_BODY_POSE_DATA_ID,
        |pose| &mut pose.body,
        value,
    );
}

/// `ArmorStand.setLeftArmPose` (`:531-533`).
pub fn set_left_arm_pose(entity: &mut Entity, value: Rotations) {
    set_pose_member(
        entity,
        VANILLA_ARMOR_STAND_LEFT_ARM_POSE_DATA_ID,
        |pose| &mut pose.left_arm,
        value,
    );
}

/// `ArmorStand.setRightArmPose` (`:535-537`).
pub fn set_right_arm_pose(entity: &mut Entity, value: Rotations) {
    set_pose_member(
        entity,
        VANILLA_ARMOR_STAND_RIGHT_ARM_POSE_DATA_ID,
        |pose| &mut pose.right_arm,
        value,
    );
}

/// `ArmorStand.setLeftLegPose` (`:539-541`).
pub fn set_left_leg_pose(entity: &mut Entity, value: Rotations) {
    set_pose_member(
        entity,
        VANILLA_ARMOR_STAND_LEFT_LEG_POSE_DATA_ID,
        |pose| &mut pose.left_leg,
        value,
    );
}

/// `ArmorStand.setRightLegPose` (`:543-545`).
pub fn set_right_leg_pose(entity: &mut Entity, value: Rotations) {
    set_pose_member(
        entity,
        VANILLA_ARMOR_STAND_RIGHT_LEG_POSE_DATA_ID,
        |pose| &mut pose.right_leg,
        value,
    );
}

pub fn set_armor_stand_pose(entity: &mut Entity, pose: ArmorStandPose) {
    set_head_pose(entity, pose.head);
    set_body_pose(entity, pose.body);
    set_left_arm_pose(entity, pose.left_arm);
    set_right_arm_pose(entity, pose.right_arm);
    set_left_leg_pose(entity, pose.left_leg);
    set_right_leg_pose(entity, pose.right_leg);
}

/// What the caller applies over the rotation the constructor drew.
enum SpawnRotation {
    Constructor,
    Snapped(f32),
    Loaded,
}

pub fn spawn(store: &mut EntityStore, position: Vec3d) -> Option<i32> {
    spawn_initialized(store, position, SpawnRotation::Constructor)
}

pub fn spawn_placed(
    store: &mut EntityStore,
    final_position: Vec3d,
    final_y_rot: f32,
) -> Option<i32> {
    spawn_initialized(store, final_position, SpawnRotation::Snapped(final_y_rot))
}

pub fn spawn_summoned(store: &mut EntityStore, position: Vec3d) -> Option<i32> {
    spawn_initialized(store, position, SpawnRotation::Loaded)
}

fn spawn_initialized(
    store: &mut EntityStore,
    position: Vec3d,
    rotation: SpawnRotation,
) -> Option<i32> {
    let data = ArmorStandData::new()?;
    let uuid = store.next_uuid();
    let constructor_y_rot = store.random().next_float() * std::f32::consts::TAU;
    store.restore_initialized(
        uuid,
        VANILLA_ENTITY_TYPE_ARMOR_STAND_ID,
        position,
        EntityData::ArmorStand(data),
        |entity| {
            let (y_rot, y_head_rot) = match rotation {
                SpawnRotation::Constructor => (constructor_y_rot, constructor_y_rot),
                SpawnRotation::Snapped(y_rot) => (y_rot, constructor_y_rot),
                SpawnRotation::Loaded => (0.0, 0.0),
            };
            entity.set_rot(y_rot, 0.0);
            if let Some(data) = armor_stand_mut(entity) {
                data.living.y_head_rot = y_head_rot;
            }
        },
    )
}

///
/// Not reached here: `updatingUsingItem`/`updateSwimAmount` (`:2692-2693`),
/// arrow and stinger counts (`:2695-2716`, never set), the combat tracker
/// (`:2720-2722`), sleeping (`:2724-2726`), the body-rotation interpolation
/// fields `yRotO`/`xRotO`/`yBodyRotO`/`yHeadRotO` (`:2730-2785`, client
/// interpolation only and never saved or sent), `fallFlyTicks`
/// (`:2788-2792`) and the elytra animation (`:2799`).
pub(crate) fn tick(
    entity: &mut Entity,
    store: &mut EntityStore,
    level: &mut LevelCtx<'_, '_>,
) -> bool {
    let block_drops = level.block_drops();
    crate::entity::living::snap_first_sleep(entity);
    if !living_base_tick_checked(entity, store, level, |entity, store, level, damage| {
        let (damage_type, amount) = match damage {
            LivingBaseTickDamage::OnFire { amount } => ("minecraft:on_fire", amount),
            LivingBaseTickDamage::Cactus { amount } => ("minecraft:cactus", amount),
            LivingBaseTickDamage::SweetBerryBush { amount } => {
                ("minecraft:sweet_berry_bush", amount)
            }
            LivingBaseTickDamage::InWall { amount } => ("minecraft:in_wall", amount),
            LivingBaseTickDamage::Freeze { amount } => ("minecraft:freeze", amount),
            LivingBaseTickDamage::OutOfWorld { amount } => ("minecraft:out_of_world", amount),
        };
        hurt_from_environment(entity, store, level, block_drops, damage_type, amount)
    }) {
        return false;
    }
    // `:2719`.
    publish_equipment_changes(entity, store, level);
    if !living::tick_sleep(entity, Some(&mut *store), level) {
        return false;
    }
    // `:2727-2729`.
    if entity.is_alive() {
        if !ai_step(entity, store, level) {
            return false;
        }
        if !living::apply_living_inside_block_effects_and_freezing(
            entity,
            store,
            level,
            |entity, store, level, damage_type, amount| {
                hurt_from_environment_outcome(
                    entity,
                    store,
                    level,
                    block_drops,
                    damage_type,
                    amount,
                )
            },
        ) {
            return false;
        }
    }
    let y_rot = entity.y_rot();
    if let Some(data) = armor_stand_mut(entity) {
        data.living.y_body_rot = y_rot;
    }
    // `:2798`.
    if entity.sleeping_pos().is_some() {
        entity.set_rot(entity.y_rot(), 0.0);
    }
    if living::refresh_dirty_attributes_with_level(entity, level, store).is_none() {
        return false;
    }
    // `:2800-2802`.
    if let Some(data) = armor_stand_mut(entity) {
        living::tick_impulse_grace(&mut data.living.fall_impulse.reset_grace_time);
    }
    true
}

fn hurt_from_environment(
    entity: &mut Entity,
    store: &mut EntityStore,
    level: &mut LevelCtx<'_, '_>,
    block_drops: bool,
    damage_type: &str,
    amount: f32,
) -> bool {
    hurt_from_environment_outcome(entity, store, level, block_drops, damage_type, amount).is_some()
}

pub(crate) fn hurt_from_environment_outcome(
    entity: &mut Entity,
    store: &mut EntityStore,
    level: &mut LevelCtx<'_, '_>,
    block_drops: bool,
    damage_type: &str,
    amount: f32,
) -> Option<bool> {
    let damage_type = builtin_damage_type_registry()
        .iter()
        .find(|facts| facts.resource_id == damage_type)?;
    let source = StoredEntityDamageSource {
        damage_type,
        direct_entity_id: None,
        causing_entity: None,
        attacker_views: Default::default(),
    };
    let decision = hurt_decision_in_level(
        entity,
        source,
        StoredEntityDamageRules::default(),
        StoredEntityDamageContext {
            block_drops,
            ..StoredEntityDamageContext::default()
        },
        level,
    )?;
    let effects = hurt_server_with_decision(entity, store, decision, block_drops)?;
    if effects.emits_entity_damage {
        level.game_event_at(VanillaGameEvent::ENTITY_DAMAGE, entity.position(), None);
    }
    if effects.emits_entity_die {
        crate::entity::kill::raise_entity_die(entity, level);
    }
    let _ = amount;
    Some(effects.outcome.succeeded)
}

fn ai_step(entity: &mut Entity, store: &mut EntityStore, level: &mut LevelCtx<'_, '_>) -> bool {
    // `:2984-3006`.
    apply_movement_dead_band(entity);
    let no_gravity = entity.no_gravity();
    let Some(data) = armor_stand_mut(entity) else {
        return true;
    };
    // `:3009`.
    apply_input(&mut data.living);
    // `:3010-3017`; `serverAiStep` is empty (`:3153-3154`).
    if data.living.is_immobile() {
        apply_immobile(&mut data.living);
    }
    // `:3055`.
    let input = Vec3d {
        x: f64::from(data.living.xxa),
        y: f64::from(data.living.yya),
        z: f64::from(data.living.zza),
    };
    if data.has_physics(no_gravity) {
        let Ok(()) = travel_in_air(entity, store, level, input) else {
            return false;
        };
    }
    crate::entity::push::armor_stand_push_minecarts(entity, store);
    true
}

pub(crate) fn cause_fall_damage_inner(
    entity: &mut Entity,
    store: &mut EntityStore,
    level: &mut LevelCtx<'_, '_>,
    fall_distance: f64,
    modifier: f32,
    damage_type: &'static str,
) -> Option<bool> {
    let damage_resource = damage_type;
    let data = armor_stand(entity)?;
    let mut impulse = data.living.fall_impulse;
    let effective_distance = impulse
        .adjust_fall_distance(fall_distance, entity.position().y)
        .0;
    let fall_damage_immune = level.registry_tag_membership(
        "minecraft:entity_type",
        "minecraft:fall_damage_immune",
        ARMOR_STAND_RESOURCE_ID,
    );
    let damage = calculate_fall_damage(
        &data.living,
        effective_distance,
        modifier,
        fall_damage_immune,
    );
    entity.living_mut()?.fall_impulse = impulse;
    if !fall_damage_immune {
        crate::entity::living::propagate_fall_to_passengers(
            entity,
            store,
            level,
            effective_distance,
            modifier,
            damage_resource,
        )?;
    }
    if damage <= 0 {
        return Some(false);
    }
    entity.living_mut()?.fall_impulse.reset();
    fall_damage_sounds(entity, store, level, ARMOR_STAND_FALL_SOUND);
    let block_drops = level.block_drops();
    hurt_from_environment_outcome(
        entity,
        store,
        level,
        block_drops,
        damage_resource,
        damage as f32,
    )?;
    Some(true)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HurtDecision {
    /// Returns false without a mutation (`:279-281`, `:283-285`, `:292-294`,
    /// `:316-317`, `:318-319`).
    Refused,
    /// `:287-290`.
    KillOnly,
    /// `:296-300`.
    ExplosionBreak,
    /// `:302-310`.
    IgniteOrSmoulder,
    /// `:311-313`.
    Burn,
    /// `:321-326`.
    CreativeBreak,
    /// `:328-339`.
    PlayerHit { always_kills: bool },
}

impl HurtDecision {
    pub const fn outcome(self) -> StoredEntityHurtOutcome {
        match self {
            Self::Refused => StoredEntityHurtOutcome {
                applied: false,
                succeeded: false,
            },
            Self::KillOnly | Self::ExplosionBreak | Self::IgniteOrSmoulder | Self::Burn => {
                StoredEntityHurtOutcome {
                    applied: true,
                    succeeded: false,
                }
            }
            Self::CreativeBreak | Self::PlayerHit { .. } => StoredEntityHurtOutcome {
                applied: true,
                succeeded: true,
            },
        }
    }
}

pub fn hurt_decision(
    entity: &Entity,
    source: StoredEntityDamageSource<'_>,
    rules: StoredEntityDamageRules,
    context: StoredEntityDamageContext,
) -> HurtDecision {
    let Some(data) = armor_stand(entity) else {
        return HurtDecision::Refused;
    };
    let facts = source.damage_type;
    // `:279-281`.
    if !entity.is_alive() {
        return HurtDecision::Refused;
    }
    // `:283-285`.
    if !rules.mob_griefing
        && source
            .causing_entity
            .is_some_and(|causing| matches!(causing, StoredDamageCausingEntity::Mob { .. }))
    {
        return HurtDecision::Refused;
    }
    // `:287-290`.
    if facts.bypasses_invulnerability {
        return HurtDecision::KillOnly;
    }
    let creative_player = source.causing_entity.is_some_and(|causing| {
        matches!(
            causing,
            StoredDamageCausingEntity::Player {
                instabuild: true,
                ..
            }
        )
    });
    if living::is_invulnerable_to(entity, facts, creative_player, rules.fall_damage_immune)
        || data.invisible
        || data.is_marker()
    {
        return HurtDecision::Refused;
    }
    // `:296-300`.
    if facts.is_explosion {
        return HurtDecision::ExplosionBreak;
    }
    // `:302-310`.
    if facts.ignites_armor_stands {
        return HurtDecision::IgniteOrSmoulder;
    }
    // `:311-313`.
    if facts.burns_armor_stands && data.living.health() > 0.5 {
        return HurtDecision::Burn;
    }
    // `:315-319`.
    let allow_incremental_breaking = facts.can_break_armor_stand;
    let always_kills = facts.always_kills_armor_stands;
    if !allow_incremental_breaking && !always_kills {
        return HurtDecision::Refused;
    }
    if matches!(
        source.causing_entity,
        Some(StoredDamageCausingEntity::Player { .. })
    ) && !context.attacker_may_build
    {
        return HurtDecision::Refused;
    }
    // `:321-326`.
    if creative_player {
        return HurtDecision::CreativeBreak;
    }
    HurtDecision::PlayerHit { always_kills }
}

pub(crate) fn hurt_decision_in_level(
    entity: &Entity,
    source: StoredEntityDamageSource<'_>,
    rules: StoredEntityDamageRules,
    context: StoredEntityDamageContext,
    level: &LevelCtx<'_, '_>,
) -> Option<HurtDecision> {
    let context =
        if let Some(StoredDamageCausingEntity::Player { uuid, .. }) = source.causing_entity {
            StoredEntityDamageContext {
                attacker_may_build: context.attacker_may_build && level.player_may_build(uuid),
                ..context
            }
        } else {
            context
        };
    hurt_decision_with_membership(entity, source, rules, context, |registry, tag, element| {
        level.registry_tag_membership(registry, tag, element)
    })
}

pub(crate) fn hurt_decision_with_registry_tags(
    entity: &Entity,
    source: StoredEntityDamageSource<'_>,
    rules: StoredEntityDamageRules,
    context: StoredEntityDamageContext,
    registry_tags: &crate::entity::EntityRegistryTagView,
) -> Option<HurtDecision> {
    hurt_decision_with_membership(entity, source, rules, context, |registry, tag, element| {
        registry_tags.contains(registry, tag, element)
    })
}

fn hurt_decision_with_membership(
    entity: &Entity,
    source: StoredEntityDamageSource<'_>,
    rules: StoredEntityDamageRules,
    context: StoredEntityDamageContext,
    mut membership: impl FnMut(&str, &str, &str) -> bool,
) -> Option<HurtDecision> {
    let data = armor_stand(entity)?;
    if !entity.is_alive() {
        return Some(HurtDecision::Refused);
    }
    if !rules.mob_griefing
        && source
            .causing_entity
            .is_some_and(|causing| matches!(causing, StoredDamageCausingEntity::Mob { .. }))
    {
        return Some(HurtDecision::Refused);
    }
    let mut member = |registry_id: &str, tag_id: &str, resource_id: &str| {
        membership(registry_id, tag_id, resource_id)
    };
    let damage_type = &source.damage_type.resource_id;
    let bypasses = member(
        "minecraft:damage_type",
        "minecraft:bypasses_invulnerability",
        damage_type,
    );
    if bypasses {
        return Some(HurtDecision::KillOnly);
    }
    let creative_player = source.causing_entity.is_some_and(|causing| {
        matches!(
            causing,
            StoredDamageCausingEntity::Player {
                instabuild: true,
                ..
            }
        )
    });
    if entity.invulnerable() && !creative_player {
        return Some(HurtDecision::Refused);
    }
    let is_fire = member("minecraft:damage_type", "minecraft:is_fire", damage_type);
    if is_fire && entity.fire_immune() {
        return Some(HurtDecision::Refused);
    }
    let is_fall = member("minecraft:damage_type", "minecraft:is_fall", damage_type);
    if is_fall
        && member(
            "minecraft:entity_type",
            "minecraft:fall_damage_immune",
            ARMOR_STAND_RESOURCE_ID,
        )
    {
        return Some(HurtDecision::Refused);
    }
    if data.invisible || data.is_marker() {
        return Some(HurtDecision::Refused);
    }
    if member(
        "minecraft:damage_type",
        "minecraft:is_explosion",
        damage_type,
    ) {
        return Some(HurtDecision::ExplosionBreak);
    }
    if member(
        "minecraft:damage_type",
        "minecraft:ignites_armor_stands",
        damage_type,
    ) {
        return Some(HurtDecision::IgniteOrSmoulder);
    }
    if member(
        "minecraft:damage_type",
        "minecraft:burns_armor_stands",
        damage_type,
    ) && data.living.health() > 0.5
    {
        return Some(HurtDecision::Burn);
    }
    let can_break = member(
        "minecraft:damage_type",
        "minecraft:can_break_armor_stand",
        damage_type,
    );
    let always_kills = member(
        "minecraft:damage_type",
        "minecraft:always_kills_armor_stands",
        damage_type,
    );
    if !can_break && !always_kills {
        return Some(HurtDecision::Refused);
    }
    if matches!(
        source.causing_entity,
        Some(StoredDamageCausingEntity::Player { .. })
    ) && !context.attacker_may_build
    {
        return Some(HurtDecision::Refused);
    }
    if creative_player {
        return Some(HurtDecision::CreativeBreak);
    }
    Some(HurtDecision::PlayerHit { always_kills })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ArmorStandHurtEffects {
    pub outcome: StoredEntityHurtOutcome,
    pub emits_entity_damage: bool,
    pub emits_entity_die: bool,
}

pub fn hurt_server(
    entity: &mut Entity,
    store: &mut EntityStore,
    source: StoredEntityDamageSource<'_>,
    _damage: f32,
    rules: StoredEntityDamageRules,
    context: StoredEntityDamageContext,
) -> Option<ArmorStandHurtEffects> {
    let decision = hurt_decision(entity, source, rules, context);
    hurt_server_with_decision(entity, store, decision, context.block_drops)
}

pub(crate) fn hurt_server_with_decision(
    entity: &mut Entity,
    store: &mut EntityStore,
    decision: HurtDecision,
    block_drops: bool,
) -> Option<ArmorStandHurtEffects> {
    let mut emits_entity_damage = false;
    let mut emits_entity_die = false;
    match decision {
        HurtDecision::Refused => {}
        HurtDecision::KillOnly => {
            kill(entity);
            emits_entity_die = true;
        }
        HurtDecision::ExplosionBreak => {
            broken_by_anything(entity, store, block_drops)?;
            kill(entity);
            emits_entity_die = true;
        }
        HurtDecision::IgniteOrSmoulder => {
            if entity.is_on_fire() {
                let effects = cause_damage(entity, store, 0.15, block_drops)?;
                emits_entity_damage = effects.emits_entity_damage;
                emits_entity_die = effects.emits_entity_die;
            } else {
                ignite_living_for_seconds(entity, 5.0);
            }
        }
        HurtDecision::Burn => {
            let effects = cause_damage(entity, store, 4.0, block_drops)?;
            emits_entity_damage = effects.emits_entity_damage;
            emits_entity_die = effects.emits_entity_die;
        }
        HurtDecision::CreativeBreak => {
            play_broken_sound(entity, store);
            show_breaking_particles(entity, store)?;
            kill(entity);
            emits_entity_die = true;
        }
        HurtDecision::PlayerHit { always_kills } => {
            let time = store.game_time();
            let last_hit = armor_stand(entity).map_or(0, ArmorStandData::last_hit);
            if time - last_hit > WOBBLE_TIME && !always_kills {
                store.broadcast_to_tracking(
                    entity.id(),
                    crate::connection::ClientboundPlayPacket::EntityEvent(EntityEvent {
                        entity_id: entity.id(),
                        event_id: ENTITY_EVENT_ARMOR_STAND_HIT,
                    }),
                );
                emits_entity_damage = true;
                if let Some(data) = armor_stand_mut(entity) {
                    data.last_hit = time;
                }
            } else {
                broken_by_player(entity, store, block_drops)?;
                show_breaking_particles(entity, store)?;
                kill(entity);
                emits_entity_die = true;
            }
        }
    }
    Some(ArmorStandHurtEffects {
        outcome: decision.outcome(),
        emits_entity_damage,
        emits_entity_die,
    })
}

fn kill(entity: &mut Entity) {
    entity.set_removed(RemovalReason::Killed);
}

fn cause_damage(
    entity: &mut Entity,
    store: &mut EntityStore,
    damage: f32,
    block_drops: bool,
) -> Option<CauseDamage> {
    let Some(data) = armor_stand(entity) else {
        return Some(CauseDamage::none());
    };
    let health = data.living.health() - damage;
    if health <= 0.5 {
        broken_by_anything(entity, store, block_drops)?;
        kill(entity);
        Some(CauseDamage {
            emits_entity_damage: false,
            emits_entity_die: true,
        })
    } else {
        living::set_health(entity, health);
        Some(CauseDamage {
            emits_entity_damage: true,
            emits_entity_die: false,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct CauseDamage {
    emits_entity_damage: bool,
    emits_entity_die: bool,
}

impl CauseDamage {
    const fn none() -> Self {
        Self {
            emits_entity_damage: false,
            emits_entity_die: false,
        }
    }
}

fn broken_by_player(entity: &mut Entity, store: &mut EntityStore, block_drops: bool) -> Option<()> {
    if let Some(mut stack) = plain_stack(ARMOR_STAND_ITEM_RESOURCE_ID, 1) {
        // ArmorStand.java:396-400 preserves the entity's full custom name.
        if let Some(name) = entity.custom_name() {
            stack = stack
                .with_component_nbt(
                    bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_CUSTOM_NAME_TYPE_ID,
                    Some(name.component()),
                    store.item_resolver(),
                )
                .ok()?;
        }
        pop_resource(store, entity.block_position(), &stack, block_drops)?;
    }
    broken_by_anything(entity, store, block_drops)
}

fn broken_by_anything(
    entity: &mut Entity,
    store: &mut EntityStore,
    block_drops: bool,
) -> Option<()> {
    play_broken_sound(entity, store);
    let above = BlockPos {
        x: entity.block_position().x,
        y: entity.block_position().y + 1,
        z: entity.block_position().z,
    };
    for entry in VANILLA_EQUIPMENT_SLOTS {
        let data = armor_stand_mut(entity)?;
        let Some(original) = data.living.equipment_mut().set(entry.slot, None) else {
            continue;
        };
        if original.is_empty()
            || stack_enchantment_ids(&original, store)
                .iter()
                .any(|id| store.prevents_equipment_drop(id))
        {
            continue;
        }
        pop_original_resource(store, above, original, block_drops)?;
    }
    Some(())
}

// ArmorStand.java:367-382.
fn show_breaking_particles(entity: &Entity, store: &mut EntityStore) -> Option<()> {
    let state = bbb_protocol::block_state_types::vanilla_block_default_state_id(
        bbb_protocol::block_types::VANILLA_BLOCK_OAK_PLANKS_ID,
    )?;
    let mut options = bbb_protocol::codec::Encoder::new();
    options.write_var_i32(state);
    let position = entity.position();
    store.send_particles(bbb_protocol::packets::LevelParticles {
        override_limiter: false,
        always_show: false,
        position: Vec3d {
            y: position.y + f64::from(entity.bb_height()) * (2.0 / 3.0),
            ..position
        },
        offset: Vec3d {
            x: f64::from(entity.bb_width() / 4.0),
            y: f64::from(entity.bb_height() / 4.0),
            z: f64::from(entity.bb_width() / 4.0),
        },
        max_speed: 0.05,
        count: 10,
        particle: bbb_protocol::packets::ParticlePayload {
            particle_type_id: bbb_protocol::particle_types::VANILLA_PARTICLE_TYPE_BLOCK_ID,
            raw_options: options.into_inner(),
        },
    });
    Some(())
}

fn play_broken_sound(entity: &Entity, store: &mut EntityStore) {
    store.play_entity_sound(PendingEntitySound {
        position: entity.position(),
        resource_id: ARMOR_STAND_BREAK_SOUND.to_owned(),
        source: SoundSource::Neutral,
        volume: 1.0,
        pitch: 1.0,
    });
}

fn pop_resource(
    store: &mut EntityStore,
    pos: BlockPos,
    stack: &RuntimeItemStack,
    block_drops: bool,
) -> Option<()> {
    pop_original_resource(store, pos, stack.clone(), block_drops)
}

fn pop_original_resource(
    store: &mut EntityStore,
    pos: BlockPos,
    original: bbb_server_types::item_stack::RuntimeItemStack,
    block_drops: bool,
) -> Option<()> {
    let half_height = f64::from(item_entity_height()) / 2.0;
    let x = f64::from(pos.x) + 0.5 + next_spread(store);
    let y = f64::from(pos.y) + 0.5 + next_spread(store) - half_height;
    let z = f64::from(pos.z) + 0.5 + next_spread(store);
    if original.is_empty() || !block_drops {
        return Some(());
    }
    crate::entity::item::spawn_dropped_item_entity(store, Vec3d { x, y, z }, original)?;
    Some(())
}

fn next_spread(store: &mut EntityStore) -> f64 {
    store.random().next_double() * 0.5 + -0.25
}

/// `EnchantmentHelper.has(stack, effect)` reads the stack's enchantment ids.
pub(crate) fn stack_enchantment_ids(stack: &RuntimeItemStack, store: &EntityStore) -> Vec<String> {
    crate::entity::item_components::enchantments(stack, store.enchantment_registry())
        .into_iter()
        .map(|(id, _)| id)
        .collect()
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct EquippableFacts {
    pub slot: EquipmentSlot,
    pub equip_sound: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InvalidEquippableComponent;

fn equippable_of_summary(
    stack: &ItemStackSummary,
) -> Result<Option<EquippableFacts>, InvalidEquippableComponent> {
    let Some(equippable) = static_item_component(stack, VANILLA_DATA_COMPONENT_EQUIPPABLE_TYPE_ID)
        .map_err(|_| InvalidEquippableComponent)?
        .and_then(|patch| patch.equippable.as_ref())
    else {
        return Ok(None);
    };
    let slot = bbb_protocol::equipment_slots::vanilla_equipment_slot_by_id(equippable.slot_id)
        .ok_or(InvalidEquippableComponent)?
        .slot;
    let equip_sound = crate::entity::item_components::sound_resource_id(&equippable.equip_sound)
        .map(str::to_owned);
    Ok(Some(EquippableFacts { slot, equip_sound }))
}

pub fn equip_sound_entering_slot(
    stack: &ItemStackSummary,
    slot: EquipmentSlot,
) -> Result<Option<String>, InvalidEquippableComponent> {
    Ok(equippable_of_summary(stack)?
        .filter(|facts| facts.slot == slot)
        .and_then(|facts| facts.equip_sound))
}

pub(crate) fn equippable_of_persistent(
    stack: &RuntimeItemStack,
) -> Result<Option<EquippableFacts>, InvalidEquippableComponent> {
    equippable_of_summary(stack.summary())
}

fn equipment_slot_for_item(
    data: &ArmorStandData,
    equippable: Option<&EquippableFacts>,
) -> EquipmentSlot {
    match equippable {
        Some(facts) if data.can_use_slot(facts.slot) => facts.slot,
        _ => EquipmentSlot::MainHand,
    }
}

#[derive(Debug)]
pub struct InteractingPlayer<'a> {
    pub hand_stack: &'a mut bbb_server_types::item_stack::RuntimeItemStack,
    /// `Player.isSpectator()`.
    pub spectator: bool,
    pub infinite_materials: bool,
}

/// `InteractionResult` as `ArmorStand.interact` answers it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArmorStandInteraction {
    Pass,
    /// `InteractionResult.SUCCESS` (`:191`).
    Success,
    /// `InteractionResult.SUCCESS_SERVER`.
    SuccessServer,
    /// `InteractionResult.FAIL`.
    Fail,
}

pub fn interact(
    entity: &mut Entity,
    store: &mut EntityStore,
    mut player: InteractingPlayer<'_>,
    location: Vec3d,
) -> ArmorStandInteraction {
    let Some(data) = armor_stand(entity) else {
        return ArmorStandInteraction::Pass;
    };
    let hand_is_name_tag =
        player.hand_stack.item_id() == Some(bbb_protocol::item_types::VANILLA_ITEM_NAME_TAG_ID);
    // `:187-189`.
    if data.is_marker() || hand_is_name_tag {
        return ArmorStandInteraction::Pass;
    }
    // `:191-193`.
    if player.spectator {
        return ArmorStandInteraction::Success;
    }
    // `:199`.
    let Ok(equippable) = equippable_of_persistent(player.hand_stack) else {
        return ArmorStandInteraction::Fail;
    };
    let item_in_hand_slot = equipment_slot_for_item(data, equippable.as_ref());
    if player.hand_stack.is_empty() {
        // `:200-206`.
        let clicked_slot = clicked_slot(data, location);
        let target_slot = if data.is_disabled(clicked_slot) {
            item_in_hand_slot
        } else {
            clicked_slot
        };
        if data.living.has_item_in_slot(target_slot)
            && swap_item(entity, store, &mut player, target_slot)
        {
            return ArmorStandInteraction::SuccessServer;
        }
    } else {
        // `:208-210`.
        if data.is_disabled(item_in_hand_slot) {
            return ArmorStandInteraction::Fail;
        }
        // `:212-214`.
        if slot_type(item_in_hand_slot) == EquipmentSlotType::Hand && !data.show_arms() {
            return ArmorStandInteraction::Fail;
        }
        // `:216-218`.
        if swap_item(entity, store, &mut player, item_in_hand_slot) {
            return ArmorStandInteraction::SuccessServer;
        }
    }
    // `:221`.
    ArmorStandInteraction::Pass
}

fn clicked_slot(data: &ArmorStandData, location: Vec3d) -> EquipmentSlot {
    let mut slot_clicked = EquipmentSlot::MainHand;
    let small = data.is_small();
    let scale = data
        .living
        .attribute_value(bbb_protocol::attribute_types::VANILLA_ATTRIBUTE_SCALE_ID)
        as f32;
    let age_scale = if data.is_baby() { 0.5_f32 } else { 1.0_f32 };
    let click_y = location.y / f64::from(scale * age_scale);
    let has = |slot| data.living.has_item_in_slot(slot);
    if click_y >= 0.1 && click_y < 0.1 + if small { 0.8 } else { 0.45 } && has(EquipmentSlot::Feet)
    {
        slot_clicked = EquipmentSlot::Feet;
    } else if click_y >= 0.9 + if small { 0.3 } else { 0.0 }
        && click_y < 0.9 + if small { 1.0 } else { 0.7 }
        && has(EquipmentSlot::Chest)
    {
        slot_clicked = EquipmentSlot::Chest;
    } else if click_y >= 0.4
        && click_y < 0.4 + if small { 1.0 } else { 0.8 }
        && has(EquipmentSlot::Legs)
    {
        slot_clicked = EquipmentSlot::Legs;
    } else if click_y >= 1.6 && has(EquipmentSlot::Head) {
        slot_clicked = EquipmentSlot::Head;
    } else if !has(EquipmentSlot::MainHand) && has(EquipmentSlot::OffHand) {
        slot_clicked = EquipmentSlot::OffHand;
    }
    slot_clicked
}

fn swap_item(
    entity: &mut Entity,
    store: &mut EntityStore,
    player: &mut InteractingPlayer<'_>,
    slot: EquipmentSlot,
) -> bool {
    let Some(data) = armor_stand(entity) else {
        return false;
    };
    let item_stack = data.living.equipment().get(slot).cloned();
    let stand_has_item = item_stack.as_ref().is_some_and(|stack| !stack.is_empty());
    // `:250-252`.
    if stand_has_item && data.disabled_slots & (1 << filter_bit(slot, DISABLE_TAKING_OFFSET)) != 0 {
        return false;
    }
    // `:254-256`.
    if !stand_has_item && data.disabled_slots & (1 << filter_bit(slot, DISABLE_PUTTING_OFFSET)) != 0
    {
        return false;
    }
    let player_stack = &mut *player.hand_stack;
    let player_stack_empty = player_stack.is_empty();
    // `:258-261`.
    if player.infinite_materials && !stand_has_item && !player_stack_empty {
        set_item_slot(entity, store, slot, Some(player_stack.with_count(1)));
        return true;
    }
    // `:263-267`.
    if player_stack_empty || player_stack.count() <= 1 {
        let new_stack = (!player_stack_empty).then(|| player_stack.clone());
        let hand = item_stack.unwrap_or_else(bbb_server_types::item_stack::RuntimeItemStack::empty);
        set_item_slot(entity, store, slot, new_stack);
        *player_stack = hand;
        return true;
    }
    // `:269-271`.
    if stand_has_item {
        return false;
    }
    // `:273-274`: `split(1)` takes one and leaves the rest in hand.
    let one = player_stack.split(1);
    set_item_slot(entity, store, slot, Some(one));
    true
}

pub fn set_item_slot(
    entity: &mut Entity,
    store: &mut EntityStore,
    slot: EquipmentSlot,
    stack: Option<RuntimeItemStack>,
) {
    if armor_stand(entity).is_none() {
        return;
    }
    if let Ok(effects) = crate::entity::item_slots::set_living_item(entity, slot, stack) {
        if let Some(sound) = effects.sound {
            let seed = store.random().next_long();
            store.play_entity_sound_with_seed(sound, seed);
        }
    }
}

/// `ItemStack.isSameItemSameComponents` over the persistent form: same item
/// and the same component patch, count aside.
pub(crate) fn is_same_item_same_components(
    left: Option<&RuntimeItemStack>,
    right: Option<&RuntimeItemStack>,
) -> bool {
    let left = left.filter(|stack| !stack.is_empty());
    let right = right.filter(|stack| !stack.is_empty());
    match (left, right) {
        (None, None) => true,
        (Some(left), Some(right)) => left.same_item_same_components(right),
        _ => false,
    }
}

pub fn synched_values(entity: &Entity) -> Vec<SynchedValue> {
    let mut values = living::entity_synched_values(entity);
    let Some(data) = armor_stand(entity) else {
        return values;
    };
    values.extend(living::living_synched_values(entity, &data.living));
    values.push(SynchedValue {
        value: EntityDataValue {
            data_id: VANILLA_ARMOR_STAND_CLIENT_FLAGS_DATA_ID,
            serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_BYTE_ID,
            value: EntityDataValueKind::Byte(data.client_flags),
        },
        is_default: data.client_flags == 0,
    });
    let pose_ids = [
        VANILLA_ARMOR_STAND_HEAD_POSE_DATA_ID,
        VANILLA_ARMOR_STAND_BODY_POSE_DATA_ID,
        VANILLA_ARMOR_STAND_LEFT_ARM_POSE_DATA_ID,
        VANILLA_ARMOR_STAND_RIGHT_ARM_POSE_DATA_ID,
        VANILLA_ARMOR_STAND_LEFT_LEG_POSE_DATA_ID,
        VANILLA_ARMOR_STAND_RIGHT_LEG_POSE_DATA_ID,
    ];
    for (data_id, (_, rotations, default)) in pose_ids.into_iter().zip(data.pose.members()) {
        values.push(SynchedValue {
            value: EntityDataValue {
                data_id,
                serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_ROTATIONS_ID,
                value: rotations.data_value(),
            },
            is_default: rotations == default,
        });
    }
    values
}

#[cfg(test)]
mod tests {
    use super::*;
    use bbb_protocol::NbtSummaryValue;

    #[test]
    fn rotations_canonicalise_like_the_record_constructor() {
        let wrapped = Rotations::new(370.0, -370.0, f32::NAN);
        assert_eq!(wrapped.components(), [10.0, -10.0, 0.0]);
        assert_eq!(Rotations::new(0.0, 0.0, 0.0), ArmorStandPose::DEFAULT.head);
    }

    #[test]
    fn dimensions_follow_marker_small_and_scale() {
        let mut data = ArmorStandData::new().expect("armor stand attributes");
        let [width, height] =
            vanilla_entity_type_dimensions(VANILLA_ENTITY_TYPE_ARMOR_STAND_ID).unwrap();
        assert_eq!(dimensions(&data), [width, height]);
        data.client_flags = VANILLA_ARMOR_STAND_CLIENT_FLAG_SMALL;
        assert_eq!(dimensions(&data), [width * 0.5, height * 0.5]);
        data.client_flags = VANILLA_ARMOR_STAND_CLIENT_FLAG_MARKER;
        assert_eq!(dimensions(&data), [0.0, 0.0]);
    }

    #[test]
    fn disabled_slots_read_the_filter_bit_and_the_arms_flag() {
        let mut data = ArmorStandData::new().expect("armor stand attributes");
        assert!(data.is_disabled(EquipmentSlot::MainHand));
        assert!(!data.is_disabled(EquipmentSlot::Head));
        data.disabled_slots = 1 << filter_bit(EquipmentSlot::Head, 0);
        assert!(data.is_disabled(EquipmentSlot::Head));
        assert!(!data.can_use_slot(EquipmentSlot::Body));
        data.client_flags = VANILLA_ARMOR_STAND_CLIENT_FLAG_SHOW_ARMS;
        assert!(!data.is_disabled(EquipmentSlot::MainHand));
    }

    #[test]
    fn vanishing_curse_prevents_the_equipment_drop() {
        let cursed = bbb_server_types::item_stack::RuntimeItemStack::try_read(
            &NbtSummaryValue::Compound(vec![
                bbb_protocol::NbtSummaryEntry {
                    name: "id".into(),
                    value: NbtSummaryValue::string("minecraft:iron_helmet"),
                },
                bbb_protocol::NbtSummaryEntry {
                    name: "count".into(),
                    value: NbtSummaryValue::Int(1),
                },
                bbb_protocol::NbtSummaryEntry {
                    name: "components".into(),
                    value: NbtSummaryValue::Compound(vec![bbb_protocol::NbtSummaryEntry {
                        name: "minecraft:enchantments".into(),
                        value: NbtSummaryValue::Compound(vec![bbb_protocol::NbtSummaryEntry {
                            name: "minecraft:vanishing_curse".into(),
                            value: NbtSummaryValue::Int(1),
                        }]),
                    }]),
                },
            ]),
            &crate::entity::test_items::BuiltinItems,
        )
        .expect("a valid stack");
        let store = EntityStore::new(7, crate::entity::EntityIdCounter::new());
        assert!(stack_enchantment_ids(&cursed, &store)
            .iter()
            .any(|id| store.prevents_equipment_drop(id)));
        let plain = plain_stack("minecraft:iron_helmet", 1).unwrap();
        assert!(!stack_enchantment_ids(&plain, &store)
            .iter()
            .any(|id| store.prevents_equipment_drop(id)));
    }

    #[test]
    fn helmets_equip_to_the_head_and_name_the_generic_equip_sound() {
        let helmet = plain_stack("minecraft:iron_helmet", 1).unwrap();
        let facts = equippable_of_persistent(&helmet).unwrap().unwrap();
        assert_eq!(facts.slot, EquipmentSlot::Head);
        assert_eq!(
            facts.equip_sound.as_deref(),
            Some("minecraft:item.armor.equip_iron")
        );
        let stone = plain_stack("minecraft:stone", 1).unwrap();
        assert_eq!(equippable_of_persistent(&stone).unwrap(), None);
    }
}
