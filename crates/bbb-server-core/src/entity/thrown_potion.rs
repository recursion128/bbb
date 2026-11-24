//! Stored splash and lingering potion projectiles.

use crate::world::simulation_level::LevelCtx;
use bbb_protocol::data_component_types::{
    VANILLA_DATA_COMPONENT_POTION_CONTENTS_TYPE_ID,
    VANILLA_DATA_COMPONENT_POTION_DURATION_SCALE_TYPE_ID,
};
use bbb_protocol::entity_types::{
    VANILLA_ENTITY_TYPE_AXOLOTL_ID, VANILLA_ENTITY_TYPE_LINGERING_POTION_ID,
    VANILLA_ENTITY_TYPE_SPLASH_POTION_ID,
};
use bbb_protocol::mob_effect_types::vanilla_mob_effect_type;
use bbb_protocol::packets::{
    BlockPos, DataComponentPatchSummary, Direction, EntityDataValue, EntityDataValueKind,
    MobEffectInstanceSummary, PotionContentsSummary, Vec3d,
};
use bbb_server_types::directions::{opposite, relative};
use bbb_server_types::geometry::Aabb;
use bbb_server_types::item_stack::{static_item_component, RuntimeItemStack};
use uuid::Uuid;

use crate::block_behavior::footing::is_candle_cake;
use crate::block_behavior::{campfire, candle};
use crate::entity::area_effect_cloud::{
    apply_instant_damage, apply_instant_health, hurt_in_level, AreaEffectCloudData,
};
#[cfg(test)]
use crate::entity::area_effect_cloud::{
    apply_instant_damage_builtin, apply_instant_health_builtin,
};
use crate::entity::consume_effect::potion_effects as unscaled_potion_effects;
use crate::entity::damage::{StoredDamageCausingEntity, StoredEntityDamageSource};
use crate::entity::hurting_projectile::ProjectileState;
use crate::entity::inside_blocks::extinguish::play_entity_extinguish_sound;
use crate::entity::item::{plain_stack, stack_with_count};
use crate::entity::living::mob_effects::add_active_effect_in_level;
#[cfg(test)]
use crate::entity::living::mob_effects::can_be_affected_builtin;
use crate::entity::living::{max_air_supply, ActiveMobEffect, SynchedValue};
use crate::entity::projectile_hit::{compute_margin, HitResult};
use crate::entity::projectile_shoot::ProjectileShot;

use crate::entity::store::EntityStore;
use crate::entity::throwable_projectile;
use crate::entity::{Entity, EntityCommand, EntityData};

#[derive(Debug, Clone)]
pub(crate) enum PotionEffect {
    Instant {
        resource_id: &'static str,
        amplifier: i32,
        show_particles: bool,
    },
    Active(ActiveMobEffect),
}

impl PotionEffect {
    fn from_instance(effect: ActiveMobEffect) -> Self {
        if crate::entity::living::mob_effects::is_instantaneous(effect.resource_id()) {
            Self::Instant {
                resource_id: effect.resource_id(),
                amplifier: effect.amplifier(),
                show_particles: effect.show_particles(),
            }
        } else {
            Self::Active(effect)
        }
    }
}

/// What a splash, a water splash or an area effect cloud does to an online
/// player, which only the world can apply.
#[derive(Debug, Clone, PartialEq)]
pub struct PendingPlayerPotion {
    pub player_entity_id: i32,
    /// The potion or cloud: the direct entity of an instant effect's
    /// `indirectMagic` source.
    pub direct_entity_id: i32,
    pub owner: Option<Uuid>,
    pub effects: Vec<PlayerPotionEffect>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum PlayerPotionEffect {
    /// `MobEffect.applyInstantenousEffect` at `scale`.
    Instant {
        resource_id: &'static str,
        amplifier: i32,
        scale: f64,
    },
    Add(ActiveMobEffect),
    /// `extinguishFire` when the player is alive and burning.
    Extinguish,
}

pub const DEFAULT_GRAVITY: f64 = 0.05;
const ITEM_STACK_DATA_ID: u8 = 8;
const BASE_POTION_COLOR: i32 = -13_083_194;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThrownPotionKind {
    Splash,
    Lingering,
}

impl ThrownPotionKind {
    const fn entity_type_id(self) -> i32 {
        match self {
            Self::Splash => VANILLA_ENTITY_TYPE_SPLASH_POTION_ID,
            Self::Lingering => VANILLA_ENTITY_TYPE_LINGERING_POTION_ID,
        }
    }

    const fn entity_resource_id(self) -> &'static str {
        match self {
            Self::Splash => "minecraft:splash_potion",
            Self::Lingering => "minecraft:lingering_potion",
        }
    }

    pub(crate) const fn item_resource_id(self) -> &'static str {
        self.entity_resource_id()
    }
}

pub use super::throwable_projectile::ThrowableItemData as ThrownPotionData;

pub fn spawn_splash_potion(
    store: &mut EntityStore,
    position: Vec3d,
    shot: ProjectileShot,
    item: RuntimeItemStack,
    owner: Option<Uuid>,
) -> Option<i32> {
    spawn(store, position, shot, item, owner, ThrownPotionKind::Splash)
}

pub fn spawn_lingering_potion(
    store: &mut EntityStore,
    position: Vec3d,
    shot: ProjectileShot,
    item: RuntimeItemStack,
    owner: Option<Uuid>,
) -> Option<i32> {
    spawn(
        store,
        position,
        shot,
        item,
        owner,
        ThrownPotionKind::Lingering,
    )
}

pub(crate) fn registry_potion_stack(
    item_resource_id: &str,
    potion_resource_id: &str,
) -> Option<RuntimeItemStack> {
    let potion = bbb_protocol::potion_types::vanilla_potion_for_resource_id(potion_resource_id)?;
    plain_stack(item_resource_id, 1)?
        .with_component_patch(
            &DataComponentPatchSummary {
                added_type_ids: vec![VANILLA_DATA_COMPONENT_POTION_CONTENTS_TYPE_ID],
                potion_contents: PotionContentsSummary {
                    potion_id: Some(potion.id),
                    ..Default::default()
                },
                ..Default::default()
            },
            &bbb_registry_codec::UnavailableHolderResolver,
        )
        .ok()
}

fn spawn(
    store: &mut EntityStore,
    position: Vec3d,
    shot: ProjectileShot,
    item: RuntimeItemStack,
    owner: Option<Uuid>,
    kind: ThrownPotionKind,
) -> Option<i32> {
    let item = stack_with_count(&item, 1)?;
    let uuid = store.next_uuid();
    let data = ThrownPotionData::from_saved(item, ProjectileState::new(owner));
    store.restore_initialized(
        uuid,
        kind.entity_type_id(),
        position,
        match kind {
            ThrownPotionKind::Splash => EntityData::SplashPotion(data),
            ThrownPotionKind::Lingering => EntityData::LingeringPotion(data),
        },
        |entity| shot.install(entity),
    )
}

pub(crate) fn tick(
    entity: &mut Entity,
    store: &mut EntityStore,
    level: &mut LevelCtx<'_, '_>,
) -> bool {
    let kind = match entity.data() {
        EntityData::SplashPotion(_) => ThrownPotionKind::Splash,
        EntityData::LingeringPotion(_) => ThrownPotionKind::Lingering,
        _ => return true,
    };
    let hit = throwable_projectile::tick(entity, store, level);
    let Ok(hit) = hit else {
        return false;
    };
    let Some(hit) = hit else {
        return true;
    };
    let location = match hit {
        HitResult::Block { location, .. } | HitResult::Entity { location, .. } => location,
        HitResult::Miss => unreachable!("a miss does not invoke on_hit"),
    };
    let Some(data) = potion_data(entity) else {
        return true;
    };
    let mut queued = false;
    if is_water(data.item()) {
        let owner_id = data.projectile.owner.and_then(|owner| {
            level
                .player(owner)
                .map(|player| player.entity_id)
                .or_else(|| store.entity_id_for_uuid(owner))
        });
        if let HitResult::Block { pos, direction, .. } = hit {
            dowse_fire_around(level, relative(pos, direction), direction, owner_id);
        }
        if level
            .with_detached_entity(entity, |level| {
                on_hit_as_water(entity, store, level, data.projectile.owner)
            })
            .is_none()
        {
            return false;
        }
    } else {
        match kind {
            ThrownPotionKind::Splash => {
                queued = match splash(entity, store, level, data, location) {
                    Some(queued) => queued,
                    None => return false,
                };
            }
            ThrownPotionKind::Lingering => lingering(entity, store, level, data),
        }
    }
    level.level_event(
        potion_break_event(data.item()),
        entity.block_position(),
        potion_color(data.item()),
    );
    throwable_projectile::discard_after_queued_hit(entity, store, queued);
    true
}

/// `PotionContents.is` (`PotionContents.java:79-80`).
pub(crate) fn is_water(item: &RuntimeItemStack) -> bool {
    potion_contents(item).is_some_and(|contents| {
        contents.custom_effects.is_empty()
            && contents
                .potion_id
                .and_then(bbb_protocol::potion_types::vanilla_potion)
                .is_some_and(|potion| potion.resource_id == WATER_POTION)
    })
}

const WATER_POTION: &str = "minecraft:water";

/// `AbstractThrownPotion.onHitBlock`'s water branch
/// (`AbstractThrownPotion.java:50-67`): the cell in front of the hit face, the
/// one behind it and its four horizontal neighbours.
fn dowse_fire_around(
    level: &mut LevelCtx<'_, '_>,
    at: BlockPos,
    face: Direction,
    owner_id: Option<i32>,
) {
    dowse_fire(level, at, owner_id);
    dowse_fire(level, relative(at, opposite(face)), owner_id);
    for direction in [
        Direction::North,
        Direction::East,
        Direction::South,
        Direction::West,
    ] {
        dowse_fire(level, relative(at, direction), owner_id);
    }
}

/// `AbstractThrownPotion.dowseFire` (`AbstractThrownPotion.java:110-121`).
fn dowse_fire(level: &mut LevelCtx<'_, '_>, pos: BlockPos, owner_id: Option<i32>) {
    let state = level.block_state(pos);
    if level.block_state_in_tag(state, "minecraft:fire") {
        level.destroy_block(pos, false);
    } else if (candle::is_candle(state) || is_candle_cake(state)) && candle::is_lit(state) {
        candle::extinguish(level, pos, state, None);
    } else if campfire::is_campfire(state) {
        campfire::shovel_dowse(level, pos, state, owner_id);
    }
}

/// `AbstractThrownPotion.onHitAsWater` (`AbstractThrownPotion.java:87-106`):
/// one point of indirect magic to water-sensitive bodies within four blocks,
/// their fire put out, and every axolotl in the box rehydrated.
fn on_hit_as_water(
    potion: &Entity,
    store: &mut EntityStore,
    level: &mut LevelCtx<'_, '_>,
    owner: Option<Uuid>,
) -> Option<()> {
    let area = potion.bounding_box().inflate(4.0, 2.0, 4.0);
    let causing = causing_entity(owner, store, &*level);
    let living: Vec<i32> = store
        .ids_in_box_sections(area)
        .into_iter()
        .filter(|id| {
            store.get(*id).is_some_and(|target| {
                matches!(
                    target.data(),
                    EntityData::GeneralLiving(_) | EntityData::ArmorStand(_)
                ) && target.bounding_box().intersects(area)
            })
        })
        .collect();
    for id in living {
        let Some(mut target) = store.detach(id) else {
            continue;
        };
        let sensitive = target.is_sensitive_to_water();
        let wet = (sensitive || target.is_on_fire())
            && distance_sqr(potion.position(), target.position()) < 16.0;
        if wet && sensitive {
            let source = StoredEntityDamageSource {
                damage_type: indirect_magic()?,
                direct_entity_id: Some(potion.id()),
                causing_entity: causing,
                attacker_views: Default::default(),
            };
            hurt_in_level(&mut target, store, level, source, 1.0)?;
        }
        if wet
            && target.is_on_fire()
            && target
                .living()
                .is_some_and(|living| !living.is_dead_or_dying())
        {
            play_entity_extinguish_sound(&target, store);
            target.clear_fire();
        }
        if target.entity_type_id() == VANILLA_ENTITY_TYPE_AXOLOTL_ID {
            // `Axolotl.rehydrate` (`Axolotl.java:265-268`).
            let air = target.air_supply().saturating_add(1800);
            target.set_air_supply(air.min(max_air_supply(&target)));
        }
        store.attach(target);
    }
    // Players are never water-sensitive; a burning one within four blocks is
    // put out, spectators included (`WATER_SENSITIVE_OR_ON_FIRE` replaces the
    // spectator filter).
    for player in level.players_in_box(area) {
        if distance_sqr(potion.position(), player.position) < 16.0 {
            store.queue_command(EntityCommand::PlayerPotion(PendingPlayerPotion {
                player_entity_id: player.entity_id,
                direct_entity_id: potion.id(),
                owner,
                effects: vec![PlayerPotionEffect::Extinguish],
            }));
        }
    }
    Some(())
}

fn indirect_magic() -> Option<&'static bbb_server_datapack::damage::DamageTypeFacts> {
    bbb_server_datapack::damage::builtin_damage_type_registry()
        .iter()
        .find(|facts| facts.resource_id == "minecraft:indirect_magic")
}

fn distance_sqr(a: Vec3d, b: Vec3d) -> f64 {
    let (dx, dy, dz) = (a.x - b.x, a.y - b.y, a.z - b.z);
    dx * dx + dy * dy + dz * dz
}

pub(crate) fn potion_data(entity: &Entity) -> Option<&ThrownPotionData> {
    match entity.data() {
        EntityData::SplashPotion(data) | EntityData::LingeringPotion(data) => Some(data),
        _ => None,
    }
}

fn splash(
    potion: &Entity,
    store: &mut EntityStore,
    level: &mut LevelCtx<'_, '_>,
    data: &ThrownPotionData,
    hit_location: Vec3d,
) -> Option<bool> {
    let Some((effects, duration_scale)) = potion_effects(data.item()) else {
        return Some(false);
    };
    let potion_box = potion.bounding_box().moved(Vec3d {
        x: hit_location.x - potion.position().x,
        y: hit_location.y - potion.position().y,
        z: hit_location.z - potion.position().z,
    });
    let effect_box = potion_box.inflate(4.0, 2.0, 4.0);
    let margin = f64::from(compute_margin(potion));
    let causing = causing_entity(data.projectile.owner, store, &*level);
    let candidates: Vec<i32> = store
        .ids_in_box_sections(effect_box)
        .into_iter()
        .filter(|id| {
            store.get(*id).is_some_and(|target| {
                matches!(target.data(), EntityData::GeneralLiving(_))
                    && target
                        .living()
                        .is_some_and(|living| !living.is_dead_or_dying())
                    && target.bounding_box().intersects(effect_box)
            })
        })
        .collect();
    for target_id in candidates {
        let Some(target) = store.get(target_id) else {
            continue;
        };
        let distance = aabb_distance_to_sqr(
            potion_box,
            target.bounding_box().inflate(margin, margin, margin),
        );
        if distance >= 16.0 {
            continue;
        }
        let scale = 1.0 - distance.sqrt() / 4.0;
        let Some(mut target) = store.detach(target_id) else {
            continue;
        };
        for effect in &effects {
            match effect {
                PotionEffect::Instant {
                    resource_id,
                    amplifier,
                    ..
                } => level.with_detached_entity(potion, |level| {
                    apply_instant_effect(
                        potion.id(),
                        &mut target,
                        store,
                        level,
                        causing,
                        resource_id,
                        (*amplifier, scale),
                    )
                })?,
                PotionEffect::Active(effect) => {
                    if let Some(scaled) = splash_scaled(effect, scale, duration_scale) {
                        let _ = add_active_effect_in_level(&mut target, store, &*level, scaled);
                    }
                }
            }
        }
        store.attach(target);
    }
    // `getEntitiesOfClass` leaves spectators out; `isAffectedByPotions`
    // leaves the dead out.
    let mut queued = false;
    for player in level.players_in_box(effect_box) {
        if player.spectator || player.dead_or_dying {
            continue;
        }
        let distance = aabb_distance_to_sqr(
            potion_box,
            player.bounding_box().inflate(margin, margin, margin),
        );
        if distance >= 16.0 {
            continue;
        }
        let scale = 1.0 - distance.sqrt() / 4.0;
        let effects = effects
            .iter()
            .filter_map(|effect| match effect {
                PotionEffect::Instant {
                    resource_id,
                    amplifier,
                    ..
                } => Some(PlayerPotionEffect::Instant {
                    resource_id,
                    amplifier: *amplifier,
                    scale,
                }),
                PotionEffect::Active(effect) => {
                    splash_scaled(effect, scale, duration_scale).map(PlayerPotionEffect::Add)
                }
            })
            .collect::<Vec<_>>();
        if !effects.is_empty() {
            queued = true;
            store.queue_command(EntityCommand::PlayerPotion(PendingPlayerPotion {
                player_entity_id: player.entity_id,
                direct_entity_id: potion.id(),
                owner: data.projectile.owner,
                effects,
            }));
        }
    }
    Some(queued)
}

/// `ThrownSplashPotion.onHitAsPotion`'s duration mapping; `None` for an effect
/// that would end within a second (`ThrownSplashPotion.java:62-68`).
fn splash_scaled(
    effect: &ActiveMobEffect,
    scale: f64,
    duration_scale: f32,
) -> Option<ActiveMobEffect> {
    let duration = if effect.duration() == -1 || effect.duration() == 0 {
        effect.duration()
    } else {
        (scale * f64::from(effect.duration()) * f64::from(duration_scale) + 0.5) as i32
    };
    if duration != -1 && duration <= 20 {
        return None;
    }
    ActiveMobEffect::from_saved(
        effect.resource_id(),
        duration,
        effect.amplifier(),
        effect.ambient(),
        effect.show_particles(),
        effect.show_icon(),
    )
}

fn lingering(
    potion: &Entity,
    store: &mut EntityStore,
    level: &LevelCtx<'_, '_>,
    data: &ThrownPotionData,
) {
    let Some((effects, duration_scale)) = potion_effects(data.item()) else {
        return;
    };
    if effects.is_empty() {
        return;
    }
    let owner = data.projectile.owner.filter(|owner| {
        level.player(*owner).is_some()
            || store
                .entity_id_for_uuid(*owner)
                .and_then(|id| store.get(id))
                .is_some_and(|entity| matches!(entity.data(), EntityData::GeneralLiving(_)))
    });
    let contents = potion_contents(data.item()).cloned();
    let cloud = AreaEffectCloudData::new()
        .with_radius(3.0)
        .with_radius_on_use(-0.5)
        .with_duration(600)
        .with_wait_time(10)
        .with_radius_per_tick(-3.0 / 600.0)
        .with_owner(owner)
        .with_potion_duration_scale(duration_scale)
        .with_potion_contents(contents);
    let _ = crate::entity::spawn_area_effect_cloud(store, potion.position(), cloud);
}

pub(crate) fn causing_entity(
    owner: Option<Uuid>,
    store: &EntityStore,
    level: &LevelCtx<'_, '_>,
) -> Option<StoredDamageCausingEntity> {
    let owner = owner?;
    if let Some(player) = level.player(owner) {
        return Some(StoredDamageCausingEntity::Player {
            uuid: player.uuid,
            entity_id: player.entity_id,
            instabuild: player.instabuild,
        });
    }
    let id = store.entity_id_for_uuid(owner)?;
    let entity = store.get(id)?;
    match entity.data() {
        EntityData::GeneralLiving(data) if data.is_mob() => {
            Some(StoredDamageCausingEntity::Mob { entity_id: id })
        }
        EntityData::GeneralLiving(_) => Some(StoredDamageCausingEntity::Other { entity_id: id }),
        _ => None,
    }
}

fn aabb_distance_to_sqr(left: Aabb, right: Aabb) -> f64 {
    let dx = (left.min_x - right.max_x)
        .max(right.min_x - left.max_x)
        .max(0.0);
    let dy = (left.min_y - right.max_y)
        .max(right.min_y - left.max_y)
        .max(0.0);
    let dz = (left.min_z - right.max_z)
        .max(right.min_z - left.max_z)
        .max(0.0);
    dx * dx + dy * dy + dz * dz
}

pub(crate) fn apply_instant_effect(
    direct_entity_id: i32,
    target: &mut Entity,
    store: &mut EntityStore,
    level: &mut LevelCtx<'_, '_>,
    causing_entity: Option<StoredDamageCausingEntity>,
    resource_id: &str,
    (amplifier, scale): (i32, f64),
) -> Option<()> {
    match resource_id {
        "minecraft:instant_damage" => apply_instant_damage(
            direct_entity_id,
            target,
            store,
            level,
            causing_entity,
            amplifier,
            scale,
        ),
        "minecraft:instant_health" => apply_instant_health(
            direct_entity_id,
            target,
            store,
            level,
            causing_entity,
            amplifier,
            scale,
        ),
        _ => Some(()),
    }
}

#[cfg(test)]
pub(crate) fn apply_registry_potion(target: &mut Entity, item: &RuntimeItemStack) -> Option<()> {
    let (effects, duration_scale) = potion_effects(item)?;
    for effect in effects {
        match effect {
            PotionEffect::Instant {
                resource_id,
                amplifier,
                ..
            } => match resource_id {
                "minecraft:instant_damage" => {
                    apply_instant_damage_builtin(target.id(), target, None, amplifier, 1.0)
                }
                "minecraft:instant_health" => {
                    apply_instant_health_builtin(target.id(), target, None, amplifier, 1.0)
                }
                _ => {}
            },
            PotionEffect::Active(effect) => {
                let duration = scaled_effect_duration(effect.duration(), duration_scale)?;
                let scaled = ActiveMobEffect::from_saved(
                    effect.resource_id(),
                    duration,
                    effect.amplifier(),
                    effect.ambient(),
                    effect.show_particles(),
                    effect.show_icon(),
                )?;
                if can_be_affected_builtin(target.data().entity_resource_id(), scaled.resource_id())
                {
                    if let Some(living) = target.living_mut() {
                        // The passenger send lives on the store-bearing
                        // sibling, `apply_registry_potion_in_level`.
                        living.add_active_effect(scaled);
                    }
                }
            }
        }
    }
    Some(())
}

pub(crate) fn apply_registry_potion_in_level(
    target: &mut Entity,
    store: &mut EntityStore,
    level: &mut LevelCtx<'_, '_>,
    item: &RuntimeItemStack,
) -> Option<()> {
    let (effects, duration_scale) = potion_effects(item)?;
    for effect in effects {
        match effect {
            PotionEffect::Instant {
                resource_id,
                amplifier,
                ..
            } => crate::entity::area_effect_cloud::apply_consumed_instant_effect(
                target,
                store,
                level,
                resource_id,
                amplifier,
            )?,
            PotionEffect::Active(effect) => {
                let duration = scaled_effect_duration(effect.duration(), duration_scale)?;
                let scaled = ActiveMobEffect::from_saved(
                    effect.resource_id(),
                    duration,
                    effect.amplifier(),
                    effect.ambient(),
                    effect.show_particles(),
                    effect.show_icon(),
                )?;
                let _ = add_active_effect_in_level(target, store, &*level, scaled);
            }
        }
    }
    Some(())
}

/// `PotionContents.forEachEffect`'s effects (`PotionContents.java`): the
/// base potion's, then the custom ones.
fn potion_effects(item: &RuntimeItemStack) -> Option<(Vec<PotionEffect>, f32)> {
    let contents = potion_contents(item)?;
    let duration_scale = potion_duration_scale(item)?;
    let effects = contents_effects(contents)?.collect::<Option<Vec<_>>>()?;
    Some((effects, duration_scale))
}

/// The unscaled effects of a `potion_contents` component.
pub(crate) fn contents_effects(
    contents: &PotionContentsSummary,
) -> Option<impl Iterator<Item = Option<PotionEffect>> + '_> {
    Some(
        unscaled_potion_effects(contents.potion_id, &contents.custom_effects)?
            .map(|effect| effect.map(PotionEffect::from_instance)),
    )
}

/// The effects an arrow's potion gives what it hits (`Arrow.doPostHurtEffects`,
/// `Arrow.java:113-119`), scaled by the item's `potion_duration_scale`; an
/// instantaneous one lasts the single tick that applies it.
pub(crate) fn potion_mob_effects(item: &RuntimeItemStack) -> Vec<ActiveMobEffect> {
    let Some((effects, scale)) = potion_effects(item) else {
        return Vec::new();
    };
    effects
        .into_iter()
        .filter_map(|effect| match effect {
            PotionEffect::Instant {
                resource_id,
                amplifier,
                show_particles,
            } => ActiveMobEffect::from_saved(
                resource_id,
                1,
                amplifier,
                false,
                show_particles,
                show_particles,
            ),
            PotionEffect::Active(effect) => ActiveMobEffect::from_saved(
                effect.resource_id(),
                scaled_effect_duration(effect.duration(), scale)?,
                effect.amplifier(),
                effect.ambient(),
                effect.show_particles(),
                effect.show_icon(),
            ),
        })
        .collect()
}

/// `MobEffectInstance.withScaledDuration` (`MobEffectInstance.java:189-193`).
pub(crate) fn scaled_effect_duration(duration: i32, scale: f32) -> Option<i32> {
    if duration == -1 || duration == 0 {
        return Some(duration);
    }
    scale
        .is_finite()
        .then(|| ((duration as f32 * scale).floor() as i32).max(1))
}

/// `Arrow.addEffect` (`Arrow.java:63-65`): the effect joins the arrow item's
/// `potion_contents` custom effects.
pub(crate) fn with_added_custom_effect(
    item: &RuntimeItemStack,
    effect: MobEffectInstanceSummary,
) -> Option<RuntimeItemStack> {
    let mut contents = potion_contents(item).cloned().unwrap_or_default();
    contents.custom_effects.push(effect);
    item.clone()
        .with_component_patch(
            &DataComponentPatchSummary {
                added_type_ids: vec![VANILLA_DATA_COMPONENT_POTION_CONTENTS_TYPE_ID],
                potion_contents: contents,
                ..Default::default()
            },
            &bbb_registry_codec::UnavailableHolderResolver,
        )
        .ok()
}

fn potion_contents(item: &RuntimeItemStack) -> Option<&PotionContentsSummary> {
    static_item_component(
        item.summary(),
        VANILLA_DATA_COMPONENT_POTION_CONTENTS_TYPE_ID,
    )
    .ok()
    .flatten()
    .map(|patch| &patch.potion_contents)
}

fn potion_duration_scale(item: &RuntimeItemStack) -> Option<f32> {
    let Some(component) = static_item_component(
        item.summary(),
        VANILLA_DATA_COMPONENT_POTION_DURATION_SCALE_TYPE_ID,
    )
    .ok()?
    else {
        return Some(1.0);
    };
    let scale = f32::from_bits(component.potion_duration_scale_bits?);
    (scale.is_finite() && scale >= 0.0 && !scale.is_sign_negative()).then_some(scale)
}

fn potion_break_event(item: &RuntimeItemStack) -> i32 {
    let has_instant_base = potion_contents(item)
        .and_then(|contents| contents.potion_id)
        .and_then(bbb_protocol::potion_types::vanilla_potion)
        .is_some_and(|potion| {
            potion.effects.iter().any(|effect| {
                vanilla_mob_effect_type(effect.effect_id).is_some_and(|facts| {
                    matches!(
                        facts.resource_id,
                        "minecraft:instant_damage" | "minecraft:instant_health"
                    )
                })
            })
        });
    if has_instant_base {
        2007
    } else {
        2002
    }
}

fn potion_color(item: &RuntimeItemStack) -> i32 {
    item_color(item, BASE_POTION_COLOR).unwrap_or(BASE_POTION_COLOR)
}

/// `PotionContents.getColor` (`PotionContents.java:113-119`).
pub(crate) fn arrow_item_color(item: &RuntimeItemStack) -> Result<i32, String> {
    item_color(item, -1)
}

fn item_color(item: &RuntimeItemStack, empty_color: i32) -> Result<i32, String> {
    let Some(contents) = static_item_component(
        item.summary(),
        VANILLA_DATA_COMPONENT_POTION_CONTENTS_TYPE_ID,
    )
    .map_err(|error| error.to_string())?
    else {
        return Ok(empty_color);
    };
    let contents = &contents.potion_contents;
    if contents == &PotionContentsSummary::default() {
        return Ok(empty_color);
    }
    contents_color(contents)
}

pub(crate) fn contents_color(contents: &PotionContentsSummary) -> Result<i32, String> {
    if let Some(color) = contents.custom_color {
        return Ok(color);
    }
    let potion = contents
        .potion_id
        .map(|id| {
            bbb_protocol::potion_types::vanilla_potion(id).ok_or("unreadable Native potion effects")
        })
        .transpose()?;
    let base = potion
        .into_iter()
        .flat_map(|potion| potion.effects)
        .map(|effect| {
            (
                vanilla_mob_effect_type(effect.effect_id).map(|facts| facts.color),
                effect.amplifier,
                true,
            )
        });
    let custom = contents.custom_effects.iter().map(|effect| {
        (
            effect
                .show_particles
                .then_some(effect.effect_id)
                .and_then(vanilla_mob_effect_type)
                .map(|facts| facts.color),
            effect.amplifier,
            effect.show_particles,
        )
    });
    color_from_rows(base.chain(custom))
}

fn color_from_rows(
    rows: impl IntoIterator<Item = (Option<u32>, i32, bool)>,
) -> Result<i32, String> {
    let mut red = 0i32;
    let mut green = 0i32;
    let mut blue = 0i32;
    let mut total = 0i32;
    for (color, amplifier, visible) in rows {
        if !visible {
            continue;
        }
        let color = color.ok_or("unreadable Native potion effect color")?;
        let weight = amplifier.wrapping_add(1);
        red = red.wrapping_add((((color >> 16) & 255) as i32).wrapping_mul(weight));
        green = green.wrapping_add((((color >> 8) & 255) as i32).wrapping_mul(weight));
        blue = blue.wrapping_add(((color & 255) as i32).wrapping_mul(weight));
        total = total.wrapping_add(weight);
    }
    Ok(if total == 0 {
        BASE_POTION_COLOR
    } else {
        0xff00_0000u32 as i32 | (red / total) << 16 | (green / total) << 8 | (blue / total)
    })
}

pub fn synched_values(
    entity: &Entity,
    data: &ThrownPotionData,
    kind: ThrownPotionKind,
) -> Vec<SynchedValue> {
    use bbb_protocol::entity_data_serializers::VANILLA_ENTITY_DATA_SERIALIZER_ITEM_STACK_ID;

    let mut rows = crate::entity::living::entity_synched_values(entity);
    let default = plain_stack(kind.item_resource_id(), 1);
    rows.push(SynchedValue {
        value: EntityDataValue {
            data_id: ITEM_STACK_DATA_ID,
            serializer_id: VANILLA_ENTITY_DATA_SERIALIZER_ITEM_STACK_ID,
            value: EntityDataValueKind::ItemStack(data.item().summary().clone()),
        },
        is_default: default.as_ref() == Some(data.item()),
    });
    rows
}
