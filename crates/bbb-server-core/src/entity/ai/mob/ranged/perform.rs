//! `RangedAttackMob.performRangedAttack` of each shooter, and the spells, run
//! against the level once the goal phase hands its effects back.

use bbb_protocol::block_state_types::VanillaBlockSupportType;
use bbb_protocol::difficulties::Difficulty;
use bbb_protocol::entity_types::*;
use bbb_protocol::equipment_slots::EquipmentSlot;
use bbb_protocol::packets::{BlockPos, Direction, SoundSource, Vec3d};
use bbb_server_types::block_queries::face_sturdy_at;
use bbb_server_types::fluid::block_state_is_air;
use bbb_server_types::geometry::{Axis, Vec3Ops};
use bbb_server_types::item_stack::RuntimeItemStack;

use super::spell::{PendingMobSpell, Spell, SpellCast, RED};
use super::RangedTarget;
use crate::enchantment_weapon_effects::{
    pick_highest_level, process, ChargingSounds, WeaponEffectTable,
};
use crate::entity::item::plain_stack;
use crate::entity::living::{living_sound_source, ActiveMobEffect};
use crate::entity::projectile_shoot::{movement_to_shoot, ProjectileShot};
use crate::entity::projectile_weapon::{
    charged_projectiles, crossbow_shot_direction, crossbow_shot_pitch, draw, is_arrow, is_firework,
    spawn_projectile, volley_angles, volley_spread, with_charged_projectiles, Shooter,
};
use crate::entity::store::EntityStore;
use crate::entity::{Entity, EntityCommand, EntityData};
use crate::level_event::VanillaGameEvent;
use crate::random::GameRandom;
use crate::world::simulation_level::LevelCtx;

/// `Entity.getEyeY() - 0.1F`, where every mob projectile here starts.
fn launch_y(entity: &Entity) -> f64 {
    entity.position().y + f64::from(entity.eye_height()) - f64::from(0.1_f32)
}

fn difficulty_id(store: &EntityStore) -> i32 {
    store.difficulty().unwrap_or(Difficulty::Normal) as i32
}

fn play_sound(entity: &Entity, store: &mut EntityStore, resource_id: &'static str, pitch: f32) {
    if entity.is_silent() {
        return;
    }
    store.play_sound_from_entity_at(
        entity.id(),
        entity.position(),
        resource_id,
        living_sound_source(entity),
        1.0,
        pitch,
    );
}

fn item_in(entity: &Entity, slot: EquipmentSlot) -> Option<&RuntimeItemStack> {
    entity.living()?.equipment().get(slot)
}

/// `ProjectileUtil.getWeaponHoldingHand`.
fn weapon_slot(entity: &Entity, item: &str) -> EquipmentSlot {
    let main = item_in(entity, EquipmentSlot::MainHand);
    if main.and_then(RuntimeItemStack::item_resource_id) == Some(item) {
        EquipmentSlot::MainHand
    } else {
        EquipmentSlot::OffHand
    }
}

fn weapon(entity: &Entity, item: &str) -> Option<RuntimeItemStack> {
    item_in(entity, weapon_slot(entity, item)).cloned()
}

/// The held crossbow, and the slot it is held in.
fn crossbow(entity: &Entity) -> Option<(EquipmentSlot, RuntimeItemStack)> {
    let slot = weapon_slot(entity, "minecraft:crossbow");
    item_in(entity, slot)
        .filter(|stack| stack.item_resource_id() == Some("minecraft:crossbow"))
        .map(|stack| (slot, stack.clone()))
}

fn set_crossbow(entity: &mut Entity, slot: EquipmentSlot, stack: RuntimeItemStack) {
    if let Some(living) = entity.living_mut() {
        living.equipment_mut().set(slot, Some(stack));
    }
}

/// `CrossbowItem.getChargeDuration`.
pub(crate) fn crossbow_charge_duration(
    table: &WeaponEffectTable,
    stack: &RuntimeItemStack,
    random: &GameRandom,
    value_level: &crate::world::ValueEffectLevel<'_>,
) -> i32 {
    let seconds = process(
        table,
        stack.summary(),
        crate::item_durability::ValueContext::Item(stack.summary()),
        |effects| effects.crossbow_charge_time.as_ref(),
        1.25,
        random,
        value_level,
    )
    .max(0.0);
    (seconds * 20.0).floor() as i32
}

/// `CrossbowItem.getChargingSounds`.
pub(crate) fn crossbow_charging_sounds(
    table: &WeaponEffectTable,
    stack: &RuntimeItemStack,
) -> ChargingSounds {
    pick_highest_level(table, stack.summary(), |effects| {
        &effects.crossbow_charging_sounds
    })
    .cloned()
    .unwrap_or_else(|| ChargingSounds {
        start: crate::entity::sound_variants::registered_sound(
            "minecraft:item.crossbow.loading_start",
        ),
        mid: crate::entity::sound_variants::registered_sound(
            "minecraft:item.crossbow.loading_middle",
        ),
        end: crate::entity::sound_variants::registered_sound("minecraft:item.crossbow.loading_end"),
    })
}

/// The charge in ticks of the crossbow the mob holds.
pub(crate) fn held_crossbow_charge_duration(
    entity: &Entity,
    store: &EntityStore,
    value_level: &crate::world::ValueEffectLevel<'_>,
) -> Option<i32> {
    let (_, stack) = crossbow(entity)?;
    Some(crossbow_charge_duration(
        store.weapon_effects(),
        &stack,
        store.random(),
        value_level,
    ))
}

/// `CrossbowItem.onUseTick`'s start and middle marks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ChargeMark {
    Start,
    Mid,
}

/// `RangedCrossbowAttackGoal`'s writes to the shooter and its crossbow.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CrossbowCharge {
    /// `setChargingCrossbow(true)` as it starts using the crossbow
    Start,
    /// The charge passed a loading mark while still charging.
    Mark(ChargeMark),
    /// The full-charge `onUseTick` load into `CHARGED_PROJECTILES`
    /// then `setChargingCrossbow(false)`
    /// at release.
    Loaded,
    /// `stop` while still using it.
    Abort,
}

/// `CrossbowAttackMob.setChargingCrossbow` of the pillager and the piglin
fn set_charging_crossbow(entity: &mut Entity, charging: bool) {
    if crate::entity::raider::set_charging_crossbow(entity, charging) {
        return;
    }
    let EntityData::GeneralLiving(data) = &mut entity.data else {
        return;
    };
    let Some(current) = data.piglin_charging_crossbow_mut() else {
        return;
    };
    if *current != charging {
        *current = charging;
        entity.mark_synched_dirty(
            bbb_protocol::entity_synched_data::VANILLA_PIGLIN_CHARGING_CROSSBOW_DATA_ID,
        );
    }
}

pub(crate) fn apply_crossbow_charge(
    entity: &mut Entity,
    store: &mut EntityStore,
    step: CrossbowCharge,
    value_level: &crate::world::ValueEffectLevel<'_>,
) {
    set_charging_crossbow(
        entity,
        matches!(step, CrossbowCharge::Start | CrossbowCharge::Mark(_)),
    );
    let Some((slot, stack)) = crossbow(entity) else {
        return;
    };
    match step {
        CrossbowCharge::Start => {}
        CrossbowCharge::Mark(mark) => {
            let sounds = crossbow_charging_sounds(store.weapon_effects(), &stack);
            let sound = match mark {
                ChargeMark::Start => sounds.start,
                ChargeMark::Mid => sounds.mid,
            };
            if let Some(sound) = sound {
                crate::entity::sound_variants::play(
                    store,
                    sound,
                    entity.position(),
                    SoundSource::Players,
                    0.5,
                    1.0,
                );
            }
        }
        CrossbowCharge::Loaded => {
            if !charged_projectiles(&stack).is_empty() {
                return;
            }
            let Some(mut held) = held_projectile(entity, true) else {
                return;
            };
            let drawn = draw(
                store.weapon_effects(),
                store.random(),
                crate::item_durability::ValueContext::stored(
                    entity,
                    Some(crate::entity::store::EntityBodyReadView::new(
                        store,
                        Some(entity),
                        None,
                    )),
                ),
                &stack,
                &mut held,
                false,
                value_level,
            );
            if drawn.is_empty() {
                return;
            }
            set_crossbow(entity, slot, with_charged_projectiles(&stack, &drawn));
            if let Some(sound) = crossbow_charging_sounds(store.weapon_effects(), &stack).end {
                let pitch = 1.0 / (store.random().next_float() * 0.5 + 1.0) + 0.2;
                crate::entity::sound_variants::play(
                    store,
                    sound,
                    entity.position(),
                    living_sound_source(entity),
                    1.0,
                    pitch,
                );
            }
        }
        CrossbowCharge::Abort => set_crossbow(entity, slot, with_charged_projectiles(&stack, &[])),
    }
}

/// `Monster.getProjectile` over `ProjectileWeaponItem.getHeldProjectile`:
/// a supported projectile in either hand, off hand first, else a plain arrow.
fn held_projectile(entity: &Entity, firework: bool) -> Option<RuntimeItemStack> {
    [EquipmentSlot::OffHand, EquipmentSlot::MainHand]
        .into_iter()
        .filter_map(|slot| item_in(entity, slot))
        .find(|stack| is_arrow(stack) || (firework && is_firework(stack)))
        .map(|stack| stack.with_count(1))
        .or_else(|| plain_stack("minecraft:arrow", 1))
}

fn shooter(entity: &Entity) -> Shooter {
    let position = entity.position();
    Shooter {
        uuid: entity.uuid(),
        player: false,
        position,
        eye_y: position.y + f64::from(entity.eye_height()),
    }
}

pub(crate) fn ranged_attack(
    entity: &mut Entity,
    store: &mut EntityStore,
    level: &mut LevelCtx<'_, '_>,
    target: RangedTarget,
    power: f32,
) {
    match entity.entity_type_id() {
        VANILLA_ENTITY_TYPE_SKELETON_ID
        | VANILLA_ENTITY_TYPE_STRAY_ID
        | VANILLA_ENTITY_TYPE_BOGGED_ID
        | VANILLA_ENTITY_TYPE_PARCHED_ID
        | VANILLA_ENTITY_TYPE_WITHER_SKELETON_ID
        | VANILLA_ENTITY_TYPE_ILLUSIONER_ID => {
            bow_attack(entity, store, target, power, &level.value_effect_level())
        }
        VANILLA_ENTITY_TYPE_PILLAGER_ID | VANILLA_ENTITY_TYPE_PIGLIN_ID => {
            crossbow_attack(entity, store, level, target)
        }
        VANILLA_ENTITY_TYPE_DROWNED_ID => {
            trident(entity, store, target, &level.value_effect_level())
        }
        VANILLA_ENTITY_TYPE_LLAMA_ID | VANILLA_ENTITY_TYPE_TRADER_LLAMA_ID => {
            spit(entity, store, target)
        }
        VANILLA_ENTITY_TYPE_SNOW_GOLEM_ID => snowball(entity, store, target),
        VANILLA_ENTITY_TYPE_BREEZE_ID => breeze_attack(entity, store, target),
        VANILLA_ENTITY_TYPE_WITHER_ID => crate::entity::wither::main_head_attack(
            entity,
            store,
            level,
            target.position,
            target.eye_y,
        ),
        _ => {}
    }
}

/// `Shoot.tick` and the firing position
fn breeze_attack(entity: &Entity, store: &mut EntityStore, target: RangedTarget) {
    let position = Vec3d {
        y: entity.position().y + f64::from(entity.bb_height() / 2.0 + 0.3_f32),
        ..entity.position()
    };
    let fraction = if store.vehicle(target.entity_id).is_some() {
        0.8
    } else {
        0.3
    };
    let direction = Vec3d {
        x: target.position.x - position.x,
        y: target.y_at(fraction) - position.y,
        z: target.position.z - position.z,
    };
    let uncertainty = (5 - difficulty_id(store) * 4) as f32;
    let movement = movement_to_shoot(store, direction, 0.7, uncertainty);
    let _ = crate::entity::hurting_projectile::wind_charge::spawn_breeze_wind_charge(
        store,
        position,
        ProjectileShot::new(movement),
        entity.uuid(),
    );
    if !entity.is_silent() {
        store.play_sound_from_entity_at(
            entity.id(),
            entity.position(),
            "minecraft:entity.breeze.shoot",
            living_sound_source(entity),
            1.5,
            1.0,
        );
    }
}

/// `AbstractSkeleton.performRangedAttack`,
/// the subclasses' `getArrow` (`Stray`, `Bogged`,
/// `Parched`, `WitherSkeleton`) and
/// `Illusioner.performRangedAttack`.
fn bow_attack(
    entity: &mut Entity,
    store: &mut EntityStore,
    target: RangedTarget,
    power: f32,
    value_level: &crate::world::ValueEffectLevel<'_>,
) {
    let Some(weapon) = weapon(entity, "minecraft:bow").filter(|stack| !stack.is_empty()) else {
        return;
    };
    let Some(mut item) = held_projectile(entity, false) else {
        return;
    };
    let effect = match entity.entity_type_id() {
        VANILLA_ENTITY_TYPE_STRAY_ID => Some(("minecraft:slowness", 600)),
        VANILLA_ENTITY_TYPE_BOGGED_ID => Some(("minecraft:poison", 100)),
        VANILLA_ENTITY_TYPE_PARCHED_ID => Some(("minecraft:weakness", 600)),
        _ => None,
    };
    if let Some((resource_id, duration)) =
        effect.filter(|_| item.item_resource_id() != Some("minecraft:spectral_arrow"))
    {
        let effect = bbb_protocol::packets::MobEffectInstanceSummary {
            effect_id: bbb_protocol::mob_effect_types::vanilla_mob_effect_type_for_resource_id(
                resource_id,
            )
            .expect("ranged mob effects are built in")
            .id,
            amplifier: 0,
            duration,
            ambient: false,
            show_particles: true,
            show_icon: true,
            hidden_effect: None,
        };
        if let Some(with_effect) =
            crate::entity::thrown_potion::with_added_custom_effect(&item, effect)
        {
            item = with_effect;
        }
    }
    let difficulty = difficulty_id(store);
    // `AbstractArrow.setBaseDamageFromMob`.
    let damage = f64::from(power * 2.0)
        + store
            .random()
            .triangle(f64::from(difficulty) * 0.11, 0.57425);
    let position = entity.position();
    let xd = target.position.x - position.x;
    let zd = target.position.z - position.z;
    let yd = target.y_at(1.0 / 3.0) - launch_y(entity);
    let distance = (xd * xd + zd * zd).sqrt();
    let direction = Vec3d {
        x: xd,
        y: yd + distance * f64::from(0.2_f32),
        z: zd,
    };
    let movement = movement_to_shoot(store, direction, 1.6, (14 - difficulty * 4) as f32);
    let wither = entity.entity_type_id() == VANILLA_ENTITY_TYPE_WITHER_SKELETON_ID;
    let _ = spawn_projectile(
        store,
        shooter(entity),
        &weapon,
        item,
        false,
        false,
        ProjectileShot::new(movement),
        value_level,
        |arrow| {
            if let Some(state) = crate::entity::abstract_arrow::state_mut(&mut arrow.data) {
                state.set_damage(damage);
            }
            // `WitherSkeleton.getArrow`.
            if wither {
                arrow.ignite_for_seconds(100.0);
            }
        },
    );
    let pitch = 1.0 / (store.random().next_float() * 0.4 + 0.8);
    play_sound(entity, store, "minecraft:entity.skeleton.shoot", pitch);
}

/// `Drowned.performRangedAttack`.
fn trident(
    entity: &Entity,
    store: &mut EntityStore,
    target: RangedTarget,
    value_level: &crate::world::ValueEffectLevel<'_>,
) {
    let Some(item) = super::super::drowned::held_trident(entity).cloned() else {
        return;
    };
    let position = Vec3d {
        y: launch_y(entity),
        ..entity.position()
    };
    let xd = target.position.x - position.x;
    let zd = target.position.z - position.z;
    let direction = Vec3d {
        x: xd,
        y: target.y_at(1.0 / 3.0) - position.y + (xd * xd + zd * zd).sqrt() * f64::from(0.2_f32),
        z: zd,
    };
    let uncertainty = (14 - difficulty_id(store) * 4) as f32;
    let movement = movement_to_shoot(store, direction, 1.6, uncertainty);
    let _ = crate::entity::trident::spawn_thrown(
        store,
        entity.uuid(),
        position,
        item,
        crate::entity::ArrowPickup::Disallowed,
        ProjectileShot::new(movement),
        value_level,
    );
    let pitch = 1.0 / (store.random().next_float() * 0.4 + 0.8);
    play_sound(entity, store, "minecraft:entity.drowned.shoot", pitch);
}

/// `CrossbowAttackMob.performCrossbowAttack`
/// through `CrossbowItem.performShooting` with
/// power 1.6: it fires and empties what the charge loaded.
fn crossbow_attack(
    entity: &mut Entity,
    store: &mut EntityStore,
    level: &mut LevelCtx<'_, '_>,
    target: RangedTarget,
) {
    let Some((slot, weapon)) = crossbow(entity) else {
        return;
    };
    let drawn = charged_projectiles(&weapon);
    if drawn.is_empty() {
        return;
    }
    set_crossbow(entity, slot, with_charged_projectiles(&weapon, &[]));
    let spread = volley_spread(
        store.weapon_effects(),
        store.random(),
        crate::item_durability::ValueContext::stored(
            entity,
            Some(crate::entity::store::EntityBodyReadView::new(
                store,
                Some(entity),
                None,
            )),
        ),
        &weapon,
        &level.value_effect_level(),
    );
    let uncertainty = (14 - difficulty_id(store) * 4) as f32;
    let shooter = shooter(entity);
    let position = entity.position();
    let angles = volley_angles(drawn.len(), spread);
    for ((index, projectile), angle) in drawn.into_iter().enumerate().zip(angles) {
        let launch = if is_firework(&projectile) {
            shooter.eye_y - f64::from(0.15_f32)
        } else {
            shooter.eye_y - f64::from(0.1_f32)
        };
        let xd = target.position.x - position.x;
        let zd = target.position.z - position.z;
        let distance = (xd * xd + zd * zd).sqrt();
        let target_direction = Vec3d {
            x: xd,
            y: target.y_at(1.0 / 3.0) - launch + distance * f64::from(0.2_f32),
            z: zd,
        };
        let direction = crossbow_shot_direction(
            entity.x_rot(),
            entity.y_rot(),
            Some(target_direction),
            angle,
        );
        let movement = movement_to_shoot(store, direction, 1.6, uncertainty);
        let wear = crate::entity::projectile_weapon::durability_use(true, &projectile);
        let _ = spawn_projectile(
            store,
            shooter,
            &weapon,
            projectile,
            false,
            true,
            ProjectileShot::new(movement),
            &level.value_effect_level(),
            |_| {},
        );
        let broke = crate::entity::equipment_wear::hurt_and_break(entity, store, level, slot, wear)
            .unwrap_or(true);
        let pitch = crossbow_shot_pitch(store.random(), index);
        play_sound(entity, store, "minecraft:item.crossbow.shoot", pitch);
        if broke {
            break;
        }
    }
}

/// `Llama.spit` with `LlamaSpit(level, llama)`'s start
fn spit(entity: &mut Entity, store: &mut EntityStore, target: RangedTarget) {
    let position = entity.position();
    let reach = f64::from(entity.bb_width() + 1.0) * 0.5;
    let body = entity
        .living()
        .map_or(0.0, |living| living.y_body_rot)
        .to_radians();
    let origin = Vec3d {
        x: position.x - reach * f64::from(body.sin()),
        y: launch_y(entity),
        z: position.z + reach * f64::from(body.cos()),
    };
    let xd = target.position.x - position.x;
    let zd = target.position.z - position.z;
    let yd = target.y_at(1.0 / 3.0) - origin.y;
    let yo = (xd * xd + zd * zd).sqrt() * f64::from(0.2_f32);
    let movement = movement_to_shoot(
        store,
        Vec3d {
            x: xd,
            y: yd + yo,
            z: zd,
        },
        1.5,
        10.0,
    );
    if let Some(id) = crate::entity::spawn_llama_spit(store, origin, movement) {
        if let Some(spit) = store.get_mut(id) {
            if let EntityData::LlamaSpit(data) = &mut spit.data {
                data.set_owner(Some(entity.uuid()));
            }
        }
    }
    let random = store.random();
    let pitch = 1.0 + (random.next_float() - random.next_float()) * 0.2;
    play_sound(entity, store, "minecraft:entity.llama.spit", pitch);
}

/// `SnowGolem.performRangedAttack`.
fn snowball(entity: &mut Entity, store: &mut EntityStore, target: RangedTarget) {
    let position = entity.position();
    let origin = Vec3d {
        y: launch_y(entity),
        ..position
    };
    let xd = target.position.x - position.x;
    let zd = target.position.z - position.z;
    let yd = target.eye_y - f64::from(1.1_f32);
    let yo = (xd * xd + zd * zd).sqrt() * f64::from(0.2_f32);
    let direction = Vec3d {
        x: xd,
        y: yd + yo - origin.y,
        z: zd,
    };
    let movement = movement_to_shoot(store, direction, 1.6, 12.0);
    let Some(item) = plain_stack("minecraft:snowball", 1) else {
        return;
    };
    let _ = crate::entity::spawn_snowball(
        store,
        origin,
        ProjectileShot::new(movement),
        item,
        Some(entity.uuid()),
    );
    let pitch = 0.4 / (store.random().next_float() * 0.4 + 0.8);
    play_sound(entity, store, "minecraft:entity.snow_golem.shoot", pitch);
}

/// The `LargeFireball` of `GhastShootFireballGoal.tick`.
pub(crate) fn ghast_shoot(
    entity: &Entity,
    store: &mut EntityStore,
    origin: Vec3d,
    direction: Vec3d,
) {
    let EntityData::GeneralLiving(data) = entity.data() else {
        return;
    };
    let Some(power) = data.ghast_retained().map(|state| state.explosion_power()) else {
        return;
    };
    let movement = direction.normalize().scale(
        crate::entity::hurting_projectile::HurtingProjectileState::INITIAL_ACCELERATION_POWER,
    );
    let _ = crate::entity::spawn_large_fireball(
        store,
        origin,
        ProjectileShot::new(movement),
        entity.uuid(),
        power,
    );
}

/// `new ShulkerBullet(level, shulker, target, getAttachFace().getAxis())`
pub(crate) fn shulker_shoot(
    entity: &Entity,
    store: &mut EntityStore,
    level: &LevelCtx<'_, '_>,
    target: RangedTarget,
    attach_axis: Option<u8>,
) {
    let axis = attach_axis.map(|axis| match axis {
        0 => Axis::X,
        1 => Axis::Y,
        _ => Axis::Z,
    });
    let _ = crate::entity::shulker_bullet::spawn_shulker_bullet_from(
        store,
        level,
        entity,
        crate::entity::shulker_bullet::ShulkerBulletTarget {
            uuid: target.uuid,
            position: target.position,
            height: target.height as f32,
        },
        axis,
    );
}

pub(crate) fn cast_spell(
    entity: &mut Entity,
    store: &mut EntityStore,
    level: &mut LevelCtx<'_, '_>,
    cast: SpellCast,
) {
    match cast.spell {
        Spell::Fangs => {
            if let Some(target) = cast.target {
                fangs(entity, store, level, target);
            }
        }
        Spell::SummonVex => summon_vexes(entity, store),
        Spell::Wololo => {
            if let Some(sheep) = cast.sheep.and_then(|id| store.get_mut(id)) {
                if sheep.is_alive() {
                    crate::entity::sheep::set_spawn_color(sheep, RED);
                }
            }
        }
        Spell::Disappear => {
            if let Some(effect) = ActiveMobEffect::new("minecraft:invisibility", 1200, 0) {
                crate::entity::living::mob_effects::add_active_effect_in_level(
                    entity, store, level, effect,
                );
            }
        }
        Spell::Blindness => {
            let (Some(target), Some(effect)) = (
                cast.target,
                ActiveMobEffect::new("minecraft:blindness", 400, 0),
            ) else {
                return;
            };
            if target.is_player {
                store.queue_command(EntityCommand::MobSpell(PendingMobSpell::PlayerEffect {
                    source_entity_id: entity.id(),
                    player: target.uuid,
                    effect,
                }));
            } else {
                store.with_entity_mut(target.entity_id, |victim, store| {
                    crate::entity::living::mob_effects::add_active_effect_in_level(
                        victim, store, level, effect,
                    )
                });
            }
        }
    }
}

/// `EvokerAttackSpellGoal.performSpellCasting`.
fn fangs(
    entity: &Entity,
    store: &mut EntityStore,
    level: &mut LevelCtx<'_, '_>,
    target: RangedTarget,
) {
    let position = entity.position();
    let min_y = target.position.y.min(position.y);
    let max_y = target.position.y.max(position.y) + 1.0;
    let angle = (target.position.z - position.z).atan2(target.position.x - position.x) as f32;
    let mut place = |x: f64, z: f64, angle: f32, delay: i32| {
        create_fangs(entity, store, level, x, z, min_y, max_y, angle, delay);
    };
    if target.position.distance_to_sqr(position) < 9.0 {
        for i in 0..5 {
            let angle = angle + i as f32 * std::f32::consts::PI * 0.4;
            place(
                position.x + f64::from(angle.cos()) * 1.5,
                position.z + f64::from(angle.sin()) * 1.5,
                angle,
                0,
            );
        }
        for i in 0..8 {
            let angle = angle
                + i as f32 * std::f32::consts::PI * 2.0 / 8.0
                + (std::f64::consts::PI * 2.0 / 5.0) as f32;
            place(
                position.x + f64::from(angle.cos()) * 2.5,
                position.z + f64::from(angle.sin()) * 2.5,
                angle,
                3,
            );
        }
    } else {
        for i in 0..16 {
            let reach = 1.25 * f64::from(i + 1);
            place(
                position.x + f64::from(angle.cos()) * reach,
                position.z + f64::from(angle.sin()) * reach,
                angle,
                i,
            );
        }
    }
}

/// `EvokerAttackSpellGoal.createSpellEntity`.
#[allow(clippy::too_many_arguments)]
fn create_fangs(
    entity: &Entity,
    store: &mut EntityStore,
    level: &mut LevelCtx<'_, '_>,
    x: f64,
    z: f64,
    min_y: f64,
    max_y: f64,
    angle: f32,
    delay: i32,
) {
    let mut pos = BlockPos {
        x: x.floor() as i32,
        y: max_y.floor() as i32,
        z: z.floor() as i32,
    };
    let mut top_offset = 0.0;
    loop {
        let below = BlockPos {
            y: pos.y - 1,
            ..pos
        };
        if face_sturdy_at(
            level.block_state(below),
            below,
            Direction::Up,
            VanillaBlockSupportType::Full,
        ) {
            if !block_state_is_air(level.block_state(pos)) {
                if let Some(top) = level
                    .block_collision_boxes(pos)
                    .iter()
                    .map(|shape| shape.max_y)
                    .reduce(f64::max)
                {
                    top_offset = top;
                }
            }
            break;
        }
        pos = below;
        if pos.y < min_y.floor() as i32 - 1 {
            return;
        }
    }
    let spawn = Vec3d {
        x,
        y: f64::from(pos.y) + top_offset,
        z,
    };
    if crate::entity::spawn_evoker_fangs(store, spawn, angle, delay, Some(entity.uuid())).is_some()
    {
        level.game_event_at(VanillaGameEvent::ENTITY_PLACE, spawn, Some(entity.id()));
    }
}

/// `EvokerSummonSpellGoal.performSpellCasting`: the
/// draws here, the spawn in the world.
fn summon_vexes(entity: &Entity, store: &mut EntityStore) {
    let origin = entity.block_position();
    let vexes = (0..3)
        .map(|_| {
            let random = store.random();
            let x = -2 + random.next_int(5);
            let z = -2 + random.next_int(5);
            let life = 20 * (30 + random.next_int(90));
            (
                BlockPos {
                    x: origin.x + x,
                    y: origin.y + 1,
                    z: origin.z + z,
                },
                life,
            )
        })
        .collect();
    store.queue_command(EntityCommand::MobSpell(PendingMobSpell::SummonVexes {
        owner: entity.uuid(),
        vexes,
    }));
}
