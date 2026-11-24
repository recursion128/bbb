//! Server-owned `AreaEffectCloud` state and lifecycle hooks.

use crate::world::simulation_level::LevelCtx;
use std::collections::BTreeMap;

use bbb_protocol::entity_data_serializers::{
    VANILLA_ENTITY_DATA_SERIALIZER_BOOLEAN_ID, VANILLA_ENTITY_DATA_SERIALIZER_FLOAT_ID,
    VANILLA_ENTITY_DATA_SERIALIZER_PARTICLE_ID,
};
use bbb_protocol::entity_synched_data::VANILLA_LIVING_ENTITY_HEALTH_DATA_ID;
use bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_AREA_EFFECT_CLOUD_ID;
use bbb_protocol::nbt_summary::NbtSummaryDecodeLimits;
use bbb_protocol::packets::{
    EntityDataValue, EntityDataValueKind, ParticlePayload, PotionContentsSummary, Vec3d,
};
use bbb_protocol::particle_types::VANILLA_PARTICLE_TYPE_ENTITY_EFFECT_ID;
use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue};
use bbb_registry_codec::persistent_item_stack_projection::potion_contents;
use bbb_registry_codec::RegistryElementValueRef;
use uuid::Uuid;

use crate::entity::damage::{StoredDamageCausingEntity, StoredEntityDamageSource};
use crate::entity::display::save_fields::{
    codec_uuid, entry, float, float_or, int_or, optional_field, uuid_int_array,
};
#[cfg(test)]
use crate::entity::level::builtin_registry_tag_membership;
use crate::entity::living::mob_effects::{add_active_effect_in_level, can_be_affected_in_level};
use crate::entity::living::{ActiveMobEffect, SynchedValue};
use crate::entity::store::EntityStore;
use crate::entity::throwable_projectile::discard_after_queued_hit;
use crate::entity::thrown_potion::{
    apply_instant_effect, causing_entity, contents_effects, scaled_effect_duration,
    PendingPlayerPotion, PlayerPotionEffect, PotionEffect,
};
use crate::entity::{base_tick, Entity, EntityCommand, EntityData};

/// `AreaEffectCloud.MAX_RADIUS` (`AreaEffectCloud.java:33`).
pub const MAX_RADIUS: f32 = 32.0;
/// `AreaEffectCloud.MINIMAL_RADIUS` (`AreaEffectCloud.java:39`).
pub const MINIMAL_RADIUS: f32 = 0.5;
/// `AreaEffectCloud.DEFAULT_RADIUS` (`AreaEffectCloud.java:40`).
pub const DEFAULT_RADIUS: f32 = 3.0;
/// `AreaEffectCloud.HEIGHT` (`AreaEffectCloud.java:42`).
pub const HEIGHT: f32 = 0.5;
/// `AreaEffectCloud.INFINITE_DURATION` (`AreaEffectCloud.java:43`).
pub const INFINITE_DURATION: i32 = -1;
/// `AreaEffectCloud.DEFAULT_WAIT_TIME` (`AreaEffectCloud.java:45`).
pub const DEFAULT_WAIT_TIME: i32 = 20;
/// `AreaEffectCloud.DEFAULT_REAPPLICATION_DELAY` (`AreaEffectCloud.java:46`).
pub const DEFAULT_REAPPLICATION_DELAY: i32 = 20;
/// `AreaEffectCloud.DEFAULT_DURATION_ON_USE` (`AreaEffectCloud.java:35`).
pub const DEFAULT_DURATION_ON_USE: i32 = 0;
/// `AreaEffectCloud.DEFAULT_RADIUS_ON_USE` (`AreaEffectCloud.java:36`).
pub const DEFAULT_RADIUS_ON_USE: f32 = 0.0;
/// `AreaEffectCloud.DEFAULT_RADIUS_PER_TICK` (`AreaEffectCloud.java:37`).
pub const DEFAULT_RADIUS_PER_TICK: f32 = 0.0;
/// `AreaEffectCloud.DEFAULT_POTION_DURATION_SCALE` (`AreaEffectCloud.java:38`).
pub const DEFAULT_POTION_DURATION_SCALE: f32 = 1.0;
/// `AreaEffectCloud.DEFAULT_AGE` (`AreaEffectCloud.java:34`).
pub const DEFAULT_AGE: i32 = 0;
/// `Entity.getDefaultGravity()` (`Entity.java:1432-1434`): AEC does not override.
pub const DEFAULT_GRAVITY: f64 = 0.0;

/// `Entity.isPickable()` (`Entity.java:1923-1925`), not overridden.
pub const IS_PICKABLE: bool = false;

/// After `Entity`'s eight rows (`Entity.java:255-271`). The generated
/// synched-data catalog does not name these.
pub const DATA_RADIUS_ID: u8 = 8;
pub const DATA_WAITING_ID: u8 = 9;
pub const DATA_PARTICLE_ID: u8 = 10;

fn default_particle() -> ParticlePayload {
    ParticlePayload {
        particle_type_id: VANILLA_PARTICLE_TYPE_ENTITY_EFFECT_ID,
        raw_options: (-1_i32).to_be_bytes().to_vec(),
    }
}

/// `Mth.clamp(float, 0.0F, 32.0F)` (`AreaEffectCloud.java:79`): NaN compares
/// false against both bounds and is left unchanged.
const fn mth_clamp_radius(radius: f32) -> f32 {
    if radius < 0.0 {
        0.0
    } else if radius > MAX_RADIUS {
        MAX_RADIUS
    } else {
        radius
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct AreaEffectCloudData {
    radius: f32,
    waiting: bool,
    duration: i32,
    wait_time: i32,
    reapplication_delay: i32,
    duration_on_use: i32,
    radius_on_use: f32,
    radius_per_tick: f32,
    potion_duration_scale: f32,
    owner: Option<Uuid>,
    potion_contents: Option<PotionContentsSummary>,
    custom_particle: Option<NbtSummaryValue>,
    resolved_particle: Option<ParticlePayload>,
    victims: BTreeMap<i32, i32>,
    age: i32,
}

impl Default for AreaEffectCloudData {
    fn default() -> Self {
        Self::new()
    }
}

impl AreaEffectCloudData {
    pub(crate) fn restore_command_transients(&mut self, previous: &Self) {
        self.waiting = previous.waiting;
        self.victims = previous.victims.clone();
    }

    /// Constructor defaults (`AreaEffectCloud.java:48-58`, `:72-74`).
    pub const fn new() -> Self {
        Self {
            radius: DEFAULT_RADIUS,
            waiting: false,
            duration: INFINITE_DURATION,
            wait_time: DEFAULT_WAIT_TIME,
            reapplication_delay: DEFAULT_REAPPLICATION_DELAY,
            duration_on_use: DEFAULT_DURATION_ON_USE,
            radius_on_use: DEFAULT_RADIUS_ON_USE,
            radius_per_tick: DEFAULT_RADIUS_PER_TICK,
            potion_duration_scale: DEFAULT_POTION_DURATION_SCALE,
            owner: None,
            potion_contents: None,
            custom_particle: None,
            resolved_particle: None,
            victims: BTreeMap::new(),
            age: DEFAULT_AGE,
        }
    }

    pub const fn radius(&self) -> f32 {
        self.radius
    }

    pub const fn is_waiting(&self) -> bool {
        self.waiting
    }

    pub const fn duration(&self) -> i32 {
        self.duration
    }

    pub const fn wait_time(&self) -> i32 {
        self.wait_time
    }

    pub const fn reapplication_delay(&self) -> i32 {
        self.reapplication_delay
    }

    pub const fn duration_on_use(&self) -> i32 {
        self.duration_on_use
    }

    pub const fn radius_on_use(&self) -> f32 {
        self.radius_on_use
    }

    pub const fn radius_per_tick(&self) -> f32 {
        self.radius_per_tick
    }

    pub const fn potion_duration_scale(&self) -> f32 {
        self.potion_duration_scale
    }

    pub const fn owner(&self) -> Option<Uuid> {
        self.owner
    }

    pub fn potion_contents(&self) -> Option<&PotionContentsSummary> {
        self.potion_contents.as_ref()
    }

    pub fn custom_particle(&self) -> Option<&NbtSummaryValue> {
        self.custom_particle.as_ref()
    }

    pub const fn age(&self) -> i32 {
        self.age
    }

    pub const fn with_duration(mut self, duration: i32) -> Self {
        self.duration = duration;
        self
    }

    pub const fn with_wait_time(mut self, wait_time: i32) -> Self {
        self.wait_time = wait_time;
        self
    }

    pub const fn with_radius(mut self, radius: f32) -> Self {
        self.radius = mth_clamp_radius(radius);
        self
    }

    pub const fn with_radius_per_tick(mut self, radius_per_tick: f32) -> Self {
        self.radius_per_tick = radius_per_tick;
        self
    }

    pub const fn with_reapplication_delay(mut self, reapplication_delay: i32) -> Self {
        self.reapplication_delay = reapplication_delay;
        self
    }

    pub const fn with_duration_on_use(mut self, duration_on_use: i32) -> Self {
        self.duration_on_use = duration_on_use;
        self
    }

    pub const fn with_radius_on_use(mut self, radius_on_use: f32) -> Self {
        self.radius_on_use = radius_on_use;
        self
    }

    pub const fn with_potion_duration_scale(mut self, potion_duration_scale: f32) -> Self {
        self.potion_duration_scale = potion_duration_scale;
        self
    }

    pub const fn with_owner(mut self, owner: Option<Uuid>) -> Self {
        self.owner = owner;
        self
    }

    pub fn with_potion_contents(mut self, potion_contents: Option<PotionContentsSummary>) -> Self {
        self.potion_contents = nonempty_potion_contents(potion_contents);
        self.resolved_particle =
            resolve_particle(self.custom_particle.as_ref(), self.potion_contents.as_ref());
        self
    }

    pub fn with_custom_particle(mut self, custom_particle: Option<NbtSummaryValue>) -> Self {
        self.custom_particle = custom_particle;
        self.resolved_particle =
            resolve_particle(self.custom_particle.as_ref(), self.potion_contents.as_ref());
        self
    }
}

fn nonempty_potion_contents(value: Option<PotionContentsSummary>) -> Option<PotionContentsSummary> {
    value.filter(|contents| contents != &PotionContentsSummary::default())
}

pub fn area_effect_cloud(entity: &Entity) -> Option<&AreaEffectCloudData> {
    match entity.data() {
        EntityData::AreaEffectCloud(data) => Some(data),
        _ => None,
    }
}

pub fn spawn_area_effect_cloud(
    store: &mut EntityStore,
    position: Vec3d,
    mut data: AreaEffectCloudData,
) -> Option<i32> {
    data.radius = mth_clamp_radius(data.radius);
    let uuid = store.next_uuid();
    store.restore_initialized(
        uuid,
        VANILLA_ENTITY_TYPE_AREA_EFFECT_CLOUD_ID,
        position,
        EntityData::AreaEffectCloud(data),
        apply_constructed,
    )
}

fn apply_constructed(entity: &mut Entity) {
    let Some(data) = area_effect_cloud(entity) else {
        return;
    };
    let radius = data.radius;
    let waiting = data.waiting;
    if radius.to_bits() != DEFAULT_RADIUS.to_bits() {
        entity.mark_synched_dirty(DATA_RADIUS_ID);
    }
    if waiting {
        entity.mark_synched_dirty(DATA_WAITING_ID);
    }
    entity.set_dimensions(radius * 2.0, HEIGHT);
}

pub fn set_radius(entity: &mut Entity, radius: f32) {
    let radius = mth_clamp_radius(radius);
    {
        let EntityData::AreaEffectCloud(data) = &mut entity.data else {
            return;
        };
        if data.radius.to_bits() == radius.to_bits() {
            return;
        }
        data.radius = radius;
    }
    entity.mark_synched_dirty(DATA_RADIUS_ID);
    entity.set_dimensions(radius * 2.0, HEIGHT);
}

pub fn refresh_dimensions(entity: &mut Entity) {
    let Some(data) = area_effect_cloud(entity) else {
        return;
    };
    entity.set_dimensions(data.radius * 2.0, HEIGHT);
}

pub(crate) fn tick(
    entity: &mut Entity,
    store: &mut EntityStore,
    level: &mut LevelCtx<'_, '_>,
) -> bool {
    let EntityData::AreaEffectCloud(_) = entity.data() else {
        return true;
    };
    if !base_tick(entity, store, level) {
        return false;
    }
    if entity.removal_reason().is_some() {
        return true;
    }
    server_tick(entity, store, level)
}

fn server_tick(entity: &mut Entity, store: &mut EntityStore, level: &mut LevelCtx<'_, '_>) -> bool {
    let EntityData::AreaEffectCloud(data) = entity.data() else {
        return true;
    };
    let duration = data.duration;
    let wait_time = data.wait_time;
    let radius_per_tick = data.radius_per_tick;
    let current_radius = data.radius;
    let waiting = data.waiting;
    let tick_count = entity.tick_count();
    // `:191-192`.
    if duration != INFINITE_DURATION && tick_count - wait_time >= duration {
        entity.discard();
        return true;
    }
    let should_wait = tick_count < wait_time;
    if waiting != should_wait {
        if let EntityData::AreaEffectCloud(data) = &mut entity.data {
            data.waiting = should_wait;
        }
        entity.mark_synched_dirty(DATA_WAITING_ID);
    }
    if should_wait {
        return true;
    }
    if radius_per_tick != 0.0 {
        let radius = current_radius + radius_per_tick;
        if radius < MINIMAL_RADIUS {
            entity.discard();
            return true;
        }
        set_radius(entity, radius);
    }
    if tick_count % 5 == 0 {
        return apply_effects(entity, store, level);
    }
    true
}

/// The cloud's effects, durations scaled (`PotionContents.forEachEffect`,
/// `PotionContents.java:93-103`).
fn cloud_effects(data: &AreaEffectCloudData) -> Vec<PotionEffect> {
    let Some(effects) = data.potion_contents.as_ref().and_then(contents_effects) else {
        return Vec::new();
    };
    effects
        .flatten()
        .filter_map(|effect| match effect {
            PotionEffect::Instant { .. } => Some(effect),
            PotionEffect::Active(effect) => ActiveMobEffect::from_saved(
                effect.resource_id(),
                scaled_effect_duration(effect.duration(), data.potion_duration_scale)?,
                effect.amplifier(),
                effect.ambient(),
                effect.show_particles(),
                effect.show_icon(),
            )
            .map(PotionEffect::Active),
        })
        .collect()
}

fn effect_resource_id(effect: &PotionEffect) -> &'static str {
    match effect {
        PotionEffect::Instant { resource_id, .. } => resource_id,
        PotionEffect::Active(effect) => effect.resource_id(),
    }
}

/// `AreaEffectCloud.serverTick`'s application pass (`AreaEffectCloud.java:212-256`).
fn apply_effects(
    cloud: &mut Entity,
    store: &mut EntityStore,
    level: &mut LevelCtx<'_, '_>,
) -> bool {
    let tick_count = cloud.tick_count();
    let Some(data) = area_effect_cloud_mut(cloud) else {
        return true;
    };
    data.victims.retain(|_, expiry| tick_count < *expiry);
    let effects = cloud_effects(data);
    if effects.is_empty() {
        data.victims.clear();
        return true;
    }
    let owner = data.owner;
    let area = cloud.bounding_box();
    let cloud_position = cloud.position();
    let causing = causing_entity(owner, store, &*level);
    let affected = |entity_resource_id: &str| {
        effects.iter().any(|effect| {
            can_be_affected_in_level(&*level, entity_resource_id, effect_resource_id(effect))
        })
    };
    // `getEntitiesOfClass` leaves spectators out; `isAffectedByPotions`
    // leaves the dead out.
    let mut candidates: Vec<(i32, bool, Vec3d)> = store
        .ids_in_box_sections(area)
        .into_iter()
        .filter_map(|id| store.get(id))
        .filter(|target| {
            is_affected_stored_living(target)
                && target.bounding_box().intersects(area)
                && affected(target.data().entity_resource_id())
        })
        .map(|target| (target.id(), false, target.position()))
        .collect();
    candidates.extend(
        level
            .players_in_box(area)
            .filter(|player| !player.spectator && !player.dead_or_dying)
            .filter(|_| affected("minecraft:player"))
            .map(|player| (player.entity_id, true, player.position)),
    );

    let mut queued = false;
    for (target_id, is_player, position) in candidates {
        let Some(data) = area_effect_cloud(cloud) else {
            return true;
        };
        if data.victims.contains_key(&target_id) {
            continue;
        }
        let radius = data.radius;
        let dx = position.x - cloud_position.x;
        let dz = position.z - cloud_position.z;
        if dx * dx + dz * dz > f64::from(radius * radius) {
            continue;
        }
        let delay = data.reapplication_delay;
        if let Some(data) = area_effect_cloud_mut(cloud) {
            data.victims
                .insert(target_id, tick_count.wrapping_add(delay));
        }
        if is_player {
            queued = true;
            store.queue_command(EntityCommand::PlayerPotion(PendingPlayerPotion {
                player_entity_id: target_id,
                direct_entity_id: cloud.id(),
                owner,
                effects: effects
                    .iter()
                    .map(|effect| match effect {
                        PotionEffect::Instant {
                            resource_id,
                            amplifier,
                            ..
                        } => PlayerPotionEffect::Instant {
                            resource_id,
                            amplifier: *amplifier,
                            scale: 0.5,
                        },
                        PotionEffect::Active(effect) => PlayerPotionEffect::Add(effect.clone()),
                    })
                    .collect(),
            }));
        } else if let Some(mut target) = store.detach(target_id) {
            for effect in &effects {
                let applied = match effect {
                    PotionEffect::Instant {
                        resource_id,
                        amplifier,
                        ..
                    } => level.with_detached_entity(cloud, |level| {
                        apply_instant_effect(
                            cloud.id(),
                            &mut target,
                            store,
                            level,
                            causing,
                            resource_id,
                            (*amplifier, 0.5),
                        )
                    }),
                    PotionEffect::Active(effect) => {
                        let _ =
                            add_active_effect_in_level(&mut target, store, &*level, effect.clone());
                        Some(())
                    }
                };
                if applied.is_none() {
                    store.attach(target);
                    return false;
                }
            }
            store.attach(target);
        }

        let Some(data) = area_effect_cloud(cloud) else {
            return true;
        };
        let (radius, radius_on_use) = (data.radius, data.radius_on_use);
        let (duration, duration_on_use) = (data.duration, data.duration_on_use);
        if radius_on_use != 0.0 {
            let radius = radius + radius_on_use;
            if radius < MINIMAL_RADIUS {
                discard_after_queued_hit(cloud, store, queued);
                return true;
            }
            set_radius(cloud, radius);
        }
        if duration_on_use != 0 && duration != INFINITE_DURATION {
            let duration = duration.wrapping_add(duration_on_use);
            if let Some(data) = area_effect_cloud_mut(cloud) {
                data.duration = duration;
            }
            if duration <= 0 {
                discard_after_queued_hit(cloud, store, queued);
                return true;
            }
        }
    }
    true
}

fn area_effect_cloud_mut(entity: &mut Entity) -> Option<&mut AreaEffectCloudData> {
    match &mut entity.data {
        EntityData::AreaEffectCloud(data) => Some(data),
        _ => None,
    }
}

fn is_affected_stored_living(entity: &Entity) -> bool {
    matches!(entity.data(), EntityData::GeneralLiving(_))
        && entity
            .living()
            .is_some_and(|living| !living.is_dead_or_dying())
}

pub(crate) fn apply_instant_damage(
    direct_entity_id: i32,
    target: &mut Entity,
    store: &mut EntityStore,
    level: &mut LevelCtx<'_, '_>,
    causing_entity: Option<StoredDamageCausingEntity>,
    amplifier: i32,
    scale: f64,
) -> Option<()> {
    apply_instant_effect_in_level(
        Some(direct_entity_id),
        target,
        store,
        level,
        causing_entity,
        amplifier,
        scale,
        true,
    )
}

pub(crate) fn apply_instant_health(
    direct_entity_id: i32,
    target: &mut Entity,
    store: &mut EntityStore,
    level: &mut LevelCtx<'_, '_>,
    causing_entity: Option<StoredDamageCausingEntity>,
    amplifier: i32,
    scale: f64,
) -> Option<()> {
    apply_instant_effect_in_level(
        Some(direct_entity_id),
        target,
        store,
        level,
        causing_entity,
        amplifier,
        scale,
        false,
    )
}

pub(crate) fn apply_consumed_instant_effect(
    target: &mut Entity,
    store: &mut EntityStore,
    level: &mut LevelCtx<'_, '_>,
    resource_id: &str,
    amplifier: i32,
) -> Option<()> {
    let is_harm = match resource_id {
        "minecraft:instant_health" => false,
        "minecraft:instant_damage" => true,
        _ => return Some(()),
    };
    apply_instant_effect_in_level(None, target, store, level, None, amplifier, 1.0, is_harm)
}

#[allow(clippy::too_many_arguments)]
fn apply_instant_effect_in_level(
    direct_entity_id: Option<i32>,
    target: &mut Entity,
    store: &mut EntityStore,
    level: &mut LevelCtx<'_, '_>,
    causing_entity: Option<StoredDamageCausingEntity>,
    amplifier: i32,
    scale: f64,
    is_harm: bool,
) -> Option<()> {
    let membership = level.registry_tag_membership(
        "minecraft:entity_type",
        "minecraft:inverted_healing_and_harm",
        target.data().entity_resource_id(),
    );
    apply_instant_heal_or_harm(
        direct_entity_id,
        target,
        causing_entity,
        InstantHealOrHarm {
            amplifier,
            scale,
            is_harm,
            membership,
        },
        |target, source, damage| hurt_in_level(target, store, level, source, damage),
    )
}

/// `LivingEntity.hurtServer` with the `ENTITY_DAMAGE` game event and the
/// death it may cause.
pub(crate) fn hurt_in_level(
    target: &mut Entity,
    store: &mut EntityStore,
    level: &mut LevelCtx<'_, '_>,
    source: StoredEntityDamageSource<'_>,
    damage: f32,
) -> Option<()> {
    let position = target.position();
    let effects = crate::entity::general_living::hurt_server_with_runtime_in_level(
        target,
        store,
        &mut *level,
        source,
        damage,
        crate::entity::damage::StoredEntityDamageRules::default(),
    );
    if effects.emits_entity_damage {
        level.game_event_at(
            crate::level_event::VanillaGameEvent::ENTITY_DAMAGE,
            position,
            effects.game_event_source_id,
        );
    }
    crate::entity::general_living::emit_entity_die(store, level, target, source, effects)
}

#[cfg(test)]
pub(crate) fn apply_instant_damage_builtin(
    direct_entity_id: i32,
    target: &mut Entity,
    causing_entity: Option<StoredDamageCausingEntity>,
    amplifier: i32,
    scale: f64,
) {
    let membership = builtin_registry_tag_membership(
        "minecraft:entity_type",
        "minecraft:inverted_healing_and_harm",
        target.data().entity_resource_id(),
    );
    let _ = apply_instant_heal_or_harm(
        Some(direct_entity_id),
        target,
        causing_entity,
        InstantHealOrHarm {
            amplifier,
            scale,
            is_harm: true,
            membership,
        },
        |target, source, damage| {
            let _ = crate::entity::general_living::hurt_server(
                target,
                source,
                damage,
                crate::entity::damage::StoredEntityDamageRules::default(),
                Default::default(),
            );
            Some(())
        },
    );
}

#[cfg(test)]
pub(crate) fn apply_instant_health_builtin(
    direct_entity_id: i32,
    target: &mut Entity,
    causing_entity: Option<StoredDamageCausingEntity>,
    amplifier: i32,
    scale: f64,
) {
    let membership = builtin_registry_tag_membership(
        "minecraft:entity_type",
        "minecraft:inverted_healing_and_harm",
        target.data().entity_resource_id(),
    );
    let _ = apply_instant_heal_or_harm(
        Some(direct_entity_id),
        target,
        causing_entity,
        InstantHealOrHarm {
            amplifier,
            scale,
            is_harm: false,
            membership,
        },
        |target, source, damage| {
            let _ = crate::entity::general_living::hurt_server(
                target,
                source,
                damage,
                crate::entity::damage::StoredEntityDamageRules::default(),
                Default::default(),
            );
            Some(())
        },
    );
}

#[derive(Clone, Copy)]
struct InstantHealOrHarm {
    amplifier: i32,
    scale: f64,
    is_harm: bool,
    membership: bool,
}

fn apply_instant_heal_or_harm(
    direct_entity_id: Option<i32>,
    target: &mut Entity,
    causing_entity: Option<StoredDamageCausingEntity>,
    params: InstantHealOrHarm,
    mut hurt: impl FnMut(&mut Entity, StoredEntityDamageSource<'_>, f32) -> Option<()>,
) -> Option<()> {
    let InstantHealOrHarm {
        amplifier,
        scale,
        is_harm,
        membership,
    } = params;
    let inverted = membership;
    let shift = amplifier as u32;
    if is_harm == inverted {
        let amount = (scale * f64::from(4_i32.wrapping_shl(shift)) + 0.5) as i32;
        let before = target.living().map(|living| living.health());
        if let Some(living) = target.living_mut() {
            living.heal(amount.max(0) as f32);
        }
        if before != target.living().map(|living| living.health()) {
            target.mark_synched_dirty(VANILLA_LIVING_ENTITY_HEALTH_DATA_ID);
        }
        return Some(());
    }
    let Some(damage_type) = bbb_server_datapack::damage::builtin_damage_type_registry()
        .iter()
        .find(|facts| facts.resource_id == "minecraft:indirect_magic")
    else {
        return Some(());
    };
    let source = StoredEntityDamageSource {
        damage_type,
        direct_entity_id,
        causing_entity,
        attacker_views: Default::default(),
    };
    let damage = (scale * f64::from(6_i32.wrapping_shl(shift)) + 0.5) as i32;
    hurt(target, source, damage.max(0) as f32)
}

/// `AreaEffectCloud.updateParticle` (`AreaEffectCloud.java:110-117`).
fn resolve_particle(
    custom_particle: Option<&NbtSummaryValue>,
    potion_contents: Option<&PotionContentsSummary>,
) -> Option<ParticlePayload> {
    if let Some(NbtSummaryValue::Compound(fields)) = custom_particle {
        let resource_id = fields.iter().find_map(|field| match &field.value {
            NbtSummaryValue::String(id) if field.name == "type" => Some(id.clone()),
            _ => None,
        });
        let options = NbtSummaryValue::Compound(
            fields
                .iter()
                .filter(|field| field.name != "type")
                .cloned()
                .collect(),
        );
        let startup = crate::registry_sync::builtin_datapack_startup_shared();
        if let Some(payload) = resource_id.and_then(|id| {
            let id = bbb_protocol::normalize_resource_location(id.try_to_string().ok()?).ok()?;
            bbb_server_datapack::particle_options::particle_payload_from_nbt(
                &id,
                &options,
                &bbb_server_datapack::frozen_lookup::FrozenDatapackRegistryLookup::from_startup(
                    &startup,
                ),
            )
            .ok()
        }) {
            return Some(payload);
        }
    }
    let contents = potion_contents?;
    let color = crate::entity::thrown_potion::contents_color(contents)
        .expect("cloud potion contents are valid")
        | 0xff00_0000_u32 as i32;
    Some(ParticlePayload {
        particle_type_id: VANILLA_PARTICLE_TYPE_ENTITY_EFFECT_ID,
        raw_options: color.to_be_bytes().to_vec(),
    })
}

fn particle(data: &AreaEffectCloudData) -> ParticlePayload {
    data.resolved_particle
        .clone()
        .unwrap_or_else(default_particle)
}

pub fn synched_values(entity: &Entity, data: &AreaEffectCloudData) -> Vec<SynchedValue> {
    let mut rows = crate::entity::living::entity_synched_values(entity);
    let particle = particle(data);
    let is_default_particle = particle == default_particle();
    rows.extend([
        SynchedValue {
            value: EntityDataValue {
                data_id: DATA_RADIUS_ID,
                serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_FLOAT_ID,
                value: EntityDataValueKind::Float(data.radius),
            },
            is_default: data.radius.to_bits() == DEFAULT_RADIUS.to_bits(),
        },
        SynchedValue {
            value: EntityDataValue {
                data_id: DATA_WAITING_ID,
                serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_BOOLEAN_ID,
                value: EntityDataValueKind::Boolean(data.waiting),
            },
            is_default: !data.waiting,
        },
        SynchedValue {
            value: EntityDataValue {
                data_id: DATA_PARTICLE_ID,
                serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_PARTICLE_ID,
                value: EntityDataValueKind::Particle(particle),
            },
            is_default: is_default_particle,
        },
    ]);
    rows
}

pub fn write_save_data(
    entries: &mut Vec<NbtSummaryEntry>,
    entity: &Entity,
    data: &AreaEffectCloudData,
) {
    entries.push(entry("Age", NbtSummaryValue::Int(entity.tick_count())));
    entries.push(entry("Duration", NbtSummaryValue::Int(data.duration)));
    entries.push(entry("WaitTime", NbtSummaryValue::Int(data.wait_time)));
    entries.push(entry(
        "ReapplicationDelay",
        NbtSummaryValue::Int(data.reapplication_delay),
    ));
    entries.push(entry(
        "DurationOnUse",
        NbtSummaryValue::Int(data.duration_on_use),
    ));
    entries.push(entry("RadiusOnUse", float(data.radius_on_use)));
    entries.push(entry("RadiusPerTick", float(data.radius_per_tick)));
    entries.push(entry("Radius", float(data.radius)));
    if let Some(particle) = &data.custom_particle {
        entries.push(entry("custom_particle", particle.clone()));
    }
    if let Some(owner) = data.owner {
        entries.push(entry("Owner", uuid_int_array(owner)));
    }
    if let Some(contents) = &data.potion_contents {
        entries.push(entry(
            "potion_contents",
            potion_contents::write(contents, NbtSummaryDecodeLimits::SAVED_FILE.max_depth)
                .expect("cloud potion contents are valid"),
        ));
    }
    if data.potion_duration_scale.to_bits() != DEFAULT_POTION_DURATION_SCALE.to_bits() {
        entries.push(entry(
            "potion_duration_scale",
            float(data.potion_duration_scale),
        ));
    }
}

pub fn read_save_data(entries: &[NbtSummaryEntry]) -> AreaEffectCloudData {
    let custom_particle = match optional_field(entries, "custom_particle") {
        Some(value @ NbtSummaryValue::Compound(_)) => Some(value.clone()),
        _ => None,
    };
    let potion_contents = nonempty_potion_contents(
        optional_field(entries, "potion_contents").and_then(|value| {
            potion_contents::read(
                RegistryElementValueRef::Nbt(value),
                NbtSummaryDecodeLimits::SAVED_FILE.max_depth,
            )
            .ok()
        }),
    );
    let resolved_particle = resolve_particle(custom_particle.as_ref(), potion_contents.as_ref());
    AreaEffectCloudData {
        radius: mth_clamp_radius(float_or(entries, "Radius", DEFAULT_RADIUS)),
        waiting: false,
        duration: int_or(entries, "Duration", INFINITE_DURATION),
        wait_time: int_or(entries, "WaitTime", DEFAULT_WAIT_TIME),
        reapplication_delay: int_or(entries, "ReapplicationDelay", DEFAULT_REAPPLICATION_DELAY),
        duration_on_use: int_or(entries, "DurationOnUse", DEFAULT_DURATION_ON_USE),
        radius_on_use: float_or(entries, "RadiusOnUse", DEFAULT_RADIUS_ON_USE),
        radius_per_tick: float_or(entries, "RadiusPerTick", DEFAULT_RADIUS_PER_TICK),
        potion_duration_scale: float_or(
            entries,
            "potion_duration_scale",
            DEFAULT_POTION_DURATION_SCALE,
        ),
        owner: optional_field(entries, "Owner").and_then(codec_uuid),
        potion_contents,
        custom_particle,
        resolved_particle,
        victims: BTreeMap::new(),
        age: int_or(entries, "Age", DEFAULT_AGE),
    }
}

pub fn finish_load(entity: &mut Entity) {
    let Some(data) = area_effect_cloud(entity) else {
        return;
    };
    let age = data.age;
    let radius = data.radius;
    let waiting = data.waiting;
    entity.tick_count = age;
    if radius.to_bits() != DEFAULT_RADIUS.to_bits() {
        entity.mark_synched_dirty(DATA_RADIUS_ID);
    }
    if waiting {
        entity.mark_synched_dirty(DATA_WAITING_ID);
    }
    entity.set_dimensions(radius * 2.0, HEIGHT);
}
