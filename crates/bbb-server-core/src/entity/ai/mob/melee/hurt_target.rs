//! `Mob.doHurtTarget` with the overrides of the bodies
//! that attack in melee.

use bbb_protocol::equipment_slots::EquipmentSlot;
use uuid::Uuid;

use bbb_protocol::attribute_types::{
    VANILLA_ATTRIBUTE_ATTACK_DAMAGE_ID, VANILLA_ATTRIBUTE_ATTACK_KNOCKBACK_ID,
};
use bbb_protocol::entity_types::{
    VANILLA_ENTITY_TYPE_AXOLOTL_ID, VANILLA_ENTITY_TYPE_BEE_ID, VANILLA_ENTITY_TYPE_CAVE_SPIDER_ID,
    VANILLA_ENTITY_TYPE_CREAKING_ID, VANILLA_ENTITY_TYPE_CREEPER_ID,
    VANILLA_ENTITY_TYPE_DOLPHIN_ID, VANILLA_ENTITY_TYPE_HOGLIN_ID, VANILLA_ENTITY_TYPE_HUSK_ID,
    VANILLA_ENTITY_TYPE_IRON_GOLEM_ID, VANILLA_ENTITY_TYPE_PANDA_ID, VANILLA_ENTITY_TYPE_RABBIT_ID,
    VANILLA_ENTITY_TYPE_RAVAGER_ID, VANILLA_ENTITY_TYPE_WITHER_SKELETON_ID,
    VANILLA_ENTITY_TYPE_ZOGLIN_ID,
};
use bbb_protocol::packets::Vec3d;

use super::super::brain_runtime::{hurt_entity_from_body, queue_post_attack};
use super::super::ranged::PendingMobSpell;
use super::super::target::anger::NeutralAnger;
use crate::connection::ClientboundPlayPacket;
use crate::enchantment_combat_effects::CombatValue;
use crate::entity::living::mob_effects::add_active_effect_in_level;
use crate::entity::living::ActiveMobEffect;
use crate::entity::EntityCommand;
use crate::entity::{Entity, EntityStore};
use crate::level_event::{PendingHitImpulse, PendingHurtEntity, PendingPlayerHurt};
use crate::world::simulation_level::LevelCtx;
use bbb_protocol::difficulties::Difficulty;
use bbb_protocol::entity_synched_data::VANILLA_BEE_FLAGS_DATA_ID;

const MOB_ATTACK: &str = crate::entity::ai::MOB_ATTACK_DAMAGE_TYPE;

/// The entity event `Creaking`, `Ravager` and `IronGolem` broadcast as they
/// strike (`Creaking.java:147`, `Ravager.java:294`, `IronGolem.java:189`).
const ATTACK_ENTITY_EVENT: i8 = 4;

/// `body.doHurtTarget(level, target)`; `false` is a dependency this level
/// could not answer.
pub(crate) fn do_hurt_target(
    entity: &mut Entity,
    store: &mut EntityStore,
    level: &mut LevelCtx<'_, '_>,
    target_id: i32,
) -> bool {
    let entity_type_id = entity.entity_type_id();
    match entity_type_id {
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_NAUTILUS_ID
        | bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_ZOMBIE_NAUTILUS_ID => {
            return super::super::nautilus::apply_charge_hit(entity, store, level, target_id);
        }
        // `Creeper.doHurtTarget`.
        VANILLA_ENTITY_TYPE_CREEPER_ID => return true,
        VANILLA_ENTITY_TYPE_BEE_ID => return bee_sting(entity, store, level, target_id),
        // Panda marks the bite before the ordinary attempt result.
        VANILLA_ENTITY_TYPE_PANDA_ID => crate::entity::panda::before_melee_attempt(entity),
        VANILLA_ENTITY_TYPE_CREAKING_ID => {
            entity.begin_creaking_attack_animation();
            broadcast_attack_event(entity, store);
        }
        VANILLA_ENTITY_TYPE_RAVAGER_ID => {
            crate::entity::ravager::begin_attack(entity);
            broadcast_attack_event(entity, store);
            play_sound(entity, store, "minecraft:entity.ravager.attack", 1.0);
        }
        VANILLA_ENTITY_TYPE_IRON_GOLEM_ID => broadcast_attack_event(entity, store),
        bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_WARDEN_ID => {
            crate::entity::warden::sonic_boom::recover(entity);
            broadcast_attack_event(entity, store);
            if !entity.is_silent() {
                if let Some(pitch) = store
                    .mob_ai_mut(entity.id())
                    .map(|runtime| runtime.voice_pitch(false))
                {
                    store.play_entity_sound(crate::entity::PendingEntitySound {
                        position: entity.position(),
                        resource_id: "minecraft:entity.warden.attack_impact".to_owned(),
                        source: crate::entity::living::living_sound_source(entity),
                        volume: 10.0,
                        pitch,
                    });
                }
            }
        }
        VANILLA_ENTITY_TYPE_HOGLIN_ID | VANILLA_ENTITY_TYPE_ZOGLIN_ID => {
            broadcast_attack_event(entity, store);
            let sound = if entity_type_id == VANILLA_ENTITY_TYPE_HOGLIN_ID {
                "minecraft:entity.hoglin.attack"
            } else {
                "minecraft:entity.zoglin.attack"
            };
            play_sound(entity, store, sound, 1.0);
            if entity_type_id == VANILLA_ENTITY_TYPE_HOGLIN_ID {
                crate::entity::hoglin::on_hit_target(entity, store, level, target_id);
            }
        }
        _ => {}
    }
    let golem = entity_type_id == VANILLA_ENTITY_TYPE_IRON_GOLEM_ID;
    let hoglin = matches!(
        entity_type_id,
        VANILLA_ENTITY_TYPE_HOGLIN_ID | VANILLA_ENTITY_TYPE_ZOGLIN_ID
    );
    let baby =
        matches!(entity.data(), crate::entity::EntityData::GeneralLiving(data) if data.is_baby());
    let attribute = |id| {
        entity
            .living()
            .map_or(0.0, |living| living.attribute_value(id))
    };
    let attack_damage = attribute(VANILLA_ATTRIBUTE_ATTACK_DAMAGE_ID) as f32;
    let damage_type = weapon_damage_type(entity);
    let knockback = attribute(VANILLA_ATTRIBUTE_ATTACK_KNOCKBACK_ID);
    let enchantments = entity
        .living()
        .and_then(|living| living.item_by_slot(EquipmentSlot::MainHand))
        .filter(|stack| !stack.is_empty())
        .map(|stack| {
            crate::entity::enchanted_damage::stack_enchantments(stack, store.enchantment_registry())
        })
        .unwrap_or_default();
    // Mob: `EnchantmentHelper.modifyDamage` over the weapon.
    let attack_damage = weapon_value(
        entity,
        store,
        level,
        target_id,
        &enchantments,
        damage_type,
        (CombatValue::Damage, attack_damage),
    );
    // Mob: `MaceItem.getAttackDamageBonus`.
    let attack_damage = if crate::entity::mace::mob_can_smash(entity) {
        let fall_distance = entity.fall_distance();
        let per_block = weapon_value(
            entity,
            store,
            level,
            target_id,
            &enchantments,
            damage_type,
            (CombatValue::SmashDamagePerFallenBlock, 0.0),
        );
        attack_damage
            + (crate::entity::mace::smash_damage(fall_distance)
                + f64::from(per_block) * fall_distance) as f32
    } else {
        attack_damage
    };
    let amount = if golem || (hoglin && !baby) {
        let whole = attack_damage as i32;
        if whole > 0 {
            attack_damage / 2.0 + store.random().next_int(whole) as f32
        } else {
            attack_damage
        }
    } else {
        attack_damage
    };
    let impulse = if golem {
        Some(PendingHitImpulse::Lift {
            y: f64::from(0.4_f32),
        })
    } else if hoglin {
        (!baby).then_some(PendingHitImpulse::HoglinThrow {
            from: entity.position(),
            power: knockback,
        })
    } else {
        // `LivingEntity.getKnockback` and
        // `causeExtraKnockback`.
        let yaw = entity.y_rot() * (std::f32::consts::PI / 180.0);
        let knockback = weapon_value(
            entity,
            store,
            level,
            target_id,
            &enchantments,
            damage_type,
            (CombatValue::Knockback, knockback as f32),
        ) / 2.0;
        (knockback > 0.0).then(|| PendingHitImpulse::Knockback {
            power: f64::from(knockback),
            xd: f64::from(yaw.sin()),
            zd: f64::from(-yaw.cos()),
        })
    };
    let player = level
        .player_by_entity_id(target_id)
        .map(|player| player.uuid);
    let hit = if let Some(player) = level.player_by_entity_id(target_id) {
        let attacker = PendingHurtEntity::Stored {
            entity_id: entity.id(),
        };
        level.hurt_player(PendingPlayerHurt {
            player: player.uuid,
            damage_type,
            amount,
            causing: Some(attacker.clone()),
            direct: Some(attacker),
            post_attack: true,
            impulse,
        });
        // The driver decides acceptance after the entity loop; a player the
        // level's own projection already refuses cannot have been hit.
        !(player.dead_or_dying || player.spectator || player.invulnerable)
    } else {
        let Some(hurt) =
            hurt_entity_from_body(entity, store, level, target_id, damage_type, amount)
        else {
            return false;
        };
        if hurt {
            if let Some(impulse) = impulse {
                push_stored(store, target_id, impulse);
            }
            queue_post_attack(entity, level, target_id, damage_type);
        }
        hurt
    };
    if hit && matches!(impulse, Some(PendingHitImpulse::Knockback { .. })) {
        let movement = entity.delta_movement();
        entity.set_delta_movement(Vec3d {
            x: movement.x * 0.6,
            z: movement.z * 0.6,
            ..movement
        });
    }
    // Mob: `weaponItem.hurtEnemy` on a living target.
    if hit
        && (player.is_some()
            || store
                .get(target_id)
                .is_some_and(|target| target.living().is_some()))
    {
        crate::entity::mace::mob_hurt_enemy(entity, store, level, target_id);
    }
    if golem {
        play_sound(entity, store, "minecraft:entity.iron_golem.attack", 1.0);
    } else if hit {
        play_attack_sound(entity, store);
    }
    if hit {
        mark_hit_target(entity, store, level, target_id, player);
    }
    store.queue_command(crate::entity::EntityCommand::PostPiercingAttack {
        entity_id: entity.id(),
    });
    true
}

/// `ItemStack.getDamageSource(attacker, mobAttack)`
/// of `Mob.getWeaponItem`: the main hand's `damage_type`.
pub(crate) fn weapon_damage_type(entity: &Entity) -> &'static str {
    use crate::inventory::MainHandAttackDamageType;
    let Some(stack) = entity
        .living()
        .and_then(|living| living.item_by_slot(EquipmentSlot::MainHand))
        .filter(|stack| !stack.is_empty())
    else {
        return MOB_ATTACK;
    };
    let registry = bbb_server_datapack::damage::builtin_damage_type_registry();
    let found = match crate::inventory::main_hand_attack_damage_type(stack.summary()) {
        Ok(MainHandAttackDamageType::ResourceId(resource_id)) => registry
            .iter()
            .find(|facts| facts.resource_id == resource_id),
        Ok(MainHandAttackDamageType::RegistryId(registry_id)) => registry
            .iter()
            .find(|facts| facts.registry_id == registry_id),
        // `MaceItem.getItemDamageSource`.
        Ok(MainHandAttackDamageType::PlayerAttack)
            if crate::entity::mace::mob_can_smash(entity) =>
        {
            return crate::entity::mace::MACE_SMASH_DAMAGE_TYPE;
        }
        Ok(MainHandAttackDamageType::PlayerAttack) | Err(_) => None,
    };
    found.map_or(MOB_ATTACK, |facts| facts.resource_id.as_str())
}

/// `EnchantmentHelper.modifyDamage` / `modifyKnockback`
/// of the attacker's weapon against `target_id`.
fn weapon_value(
    entity: &Entity,
    store: &EntityStore,
    level: &mut LevelCtx<'_, '_>,
    target_id: i32,
    enchantments: &[(String, i32)],
    damage_type: &str,
    (value, amount): (CombatValue, f32),
) -> f32 {
    use crate::entity::mob_death_loot::{stored_loot_entity, MobLootAttacker, MobLootDamageSource};
    use crate::loot_runtime::{LootEntity, LootEntityFacts};
    if enchantments.is_empty() {
        return amount;
    }
    let Some(damage_type) = bbb_server_datapack::damage::builtin_damage_type_registry()
        .iter()
        .find(|facts| facts.resource_id == damage_type)
    else {
        return amount;
    };
    let source = crate::entity::damage::StoredEntityDamageSource {
        damage_type,
        direct_entity_id: Some(entity.id()),
        causing_entity: Some(crate::entity::damage::StoredDamageCausingEntity::Mob {
            entity_id: entity.id(),
        }),
        attacker_views: Default::default(),
    };
    let player = LootEntityFacts {
        entity_type: Some("minecraft:player"),
        living: Some(true),
        ..LootEntityFacts::default()
    };
    let victim = if level.player_by_entity_id(target_id).is_some() {
        LootEntity::Facts(&player)
    } else {
        match store
            .get(target_id)
            .or_else(|| level.detached_entity(target_id))
        {
            Some(target) => stored_loot_entity(store, level, target),
            None => return amount,
        }
    };
    let attacker = stored_loot_entity(store, level, entity);
    let request = crate::entity::enchanted_damage::EnchantedDamageRequest {
        bodies: crate::entity::store::EntityBodyReadView::new(store, Some(entity), None),
        items: crate::entity::enchanted_damage::EnchantedDamageItems::Listed(enchantments),
        victim,
        causing: MobLootAttacker::Entity(attacker),
        direct: MobLootAttacker::Entity(attacker),
        damage_source: MobLootDamageSource::of(&source),
    };
    level
        .enchanted_damage_value(request, value, amount)
        .unwrap_or(amount)
}

/// The `doHurtTarget` overrides that mark the target of a landed hit:
/// `CaveSpider`, `WitherSkeleton`, and the zombie
/// family's `Zombie` with `Husk`, whose local
/// difficulty the level answers.
fn mark_hit_target(
    entity: &Entity,
    store: &mut EntityStore,
    level: &mut LevelCtx<'_, '_>,
    target_id: i32,
    player: Option<Uuid>,
) {
    let entity_type_id = entity.entity_type_id();
    let effect = match entity_type_id {
        VANILLA_ENTITY_TYPE_CAVE_SPIDER_ID => match store.difficulty() {
            Some(Difficulty::Normal) => ActiveMobEffect::new("minecraft:poison", 7 * 20, 0),
            Some(Difficulty::Hard) => ActiveMobEffect::new("minecraft:poison", 15 * 20, 0),
            _ => None,
        },
        VANILLA_ENTITY_TYPE_WITHER_SKELETON_ID => ActiveMobEffect::new("minecraft:wither", 200, 0),
        _ => None,
    };
    if let Some(effect) = effect {
        give_effect(store, level, entity.id(), target_id, player, effect);
    }
    if !crate::entity::zombie::is_zombie_hierarchy(entity_type_id) {
        return;
    }
    let empty_hand = entity
        .living()
        .and_then(|living| living.equipment().get(EquipmentSlot::MainHand))
        .is_none_or(|stack| stack.is_empty());
    let burning = entity.is_on_fire();
    let husk = entity_type_id == VANILLA_ENTITY_TYPE_HUSK_ID;
    if empty_hand && (burning || husk) {
        store.queue_command(EntityCommand::Zombie(
            crate::entity::zombie::PendingZombieAction::HitTarget {
                source_entity_id: entity.id(),
                target_id,
                player,
                pos: entity.block_position(),
                burning,
                husk,
            },
        ));
    }
}

/// `target.addEffect(effect, attacker)` on a stored mob or a session.
fn give_effect(
    store: &mut EntityStore,
    level: &mut LevelCtx<'_, '_>,
    source_entity_id: i32,
    target_id: i32,
    player: Option<Uuid>,
    effect: ActiveMobEffect,
) {
    match player {
        Some(player) => {
            store.queue_command(EntityCommand::MobSpell(PendingMobSpell::PlayerEffect {
                source_entity_id,
                player,
                effect,
            }))
        }
        None => {
            store.with_entity_mut(target_id, |victim, store| {
                add_active_effect_in_level(victim, store, level, effect)
            });
        }
    }
}

const STING: &str = "minecraft:sting";

/// `Bee.doHurtTarget`: no knockback, poison by
/// difficulty, then the bee has stung and stops being angry.
fn bee_sting(
    entity: &mut Entity,
    store: &mut EntityStore,
    level: &mut LevelCtx<'_, '_>,
    target_id: i32,
) -> bool {
    let damage = entity.living().map_or(0.0, |living| {
        living.attribute_value(VANILLA_ATTRIBUTE_ATTACK_DAMAGE_ID)
    }) as i32 as f32;
    let player = level.player_by_entity_id(target_id);
    let hurt = if let Some(player) = &player {
        let attacker = PendingHurtEntity::Stored {
            entity_id: entity.id(),
        };
        level.hurt_player(PendingPlayerHurt {
            player: player.uuid,
            damage_type: STING,
            amount: damage,
            causing: Some(attacker.clone()),
            direct: Some(attacker),
            post_attack: true,
            impulse: None,
        });
        !(player.dead_or_dying || player.spectator || player.invulnerable)
    } else {
        let Some(hurt) = hurt_entity_from_body(entity, store, level, target_id, STING, damage)
        else {
            return false;
        };
        if hurt {
            queue_post_attack(entity, level, target_id, STING);
        }
        hurt
    };
    if !hurt {
        return true;
    }
    let player = player.map(|player| player.uuid);
    match player {
        Some(player) => store.queue_command(EntityCommand::PlayerStinger(player)),
        None => {
            store.with_entity_mut(target_id, |victim, _| {
                crate::entity::living::add_stinger(victim)
            });
        }
    }
    let poison_seconds = match store.difficulty() {
        Some(Difficulty::Normal) => 10,
        Some(Difficulty::Hard) => 18,
        _ => 0,
    };
    if let Some(effect) = (poison_seconds > 0)
        .then(|| ActiveMobEffect::new("minecraft:poison", poison_seconds * 20, 0))
        .flatten()
    {
        give_effect(store, level, entity.id(), target_id, player, effect);
    }
    let entity_id = entity.id();
    if let crate::entity::EntityData::GeneralLiving(data) = &mut entity.data {
        if data
            .bee_retained_mut()
            .is_some_and(|bee| bee.set_has_stung())
        {
            entity.mark_synched_dirty(VANILLA_BEE_FLAGS_DATA_ID);
        }
    }
    // `stopBeingAngry` (`NeutralMob.java:136-141`), landed with the step's
    // target writes so they do not restore the anger.
    if let Some(runtime) = store.mob_ai_mut(entity_id) {
        let state = runtime.target_state_mut();
        state.target = None;
        let output = state.output.get_or_insert_with(Default::default);
        output.anger = Some(NeutralAnger {
            end_time: -1,
            target: None,
        });
        output.cleared_last_hurt_by_mob = true;
    }
    play_sound(entity, store, "minecraft:entity.bee.sting", 1.0);
    true
}

pub(in crate::entity::ai::mob) fn push_stored(
    store: &mut EntityStore,
    target_id: i32,
    impulse: PendingHitImpulse,
) {
    let Some(mut target) = store.detach(target_id) else {
        return;
    };
    match impulse {
        PendingHitImpulse::Knockback { power, xd, zd } => {
            crate::entity::living::knockback(&mut target, store, power, xd, zd);
        }
        PendingHitImpulse::Lift { y } => {
            let resistance = target.living().map_or(0.0, |living| {
                living.attribute_value(
                    bbb_protocol::attribute_types::VANILLA_ATTRIBUTE_KNOCKBACK_RESISTANCE_ID,
                )
            });
            let movement = target.delta_movement();
            target.set_delta_movement(Vec3d {
                y: movement.y + y * (1.0 - resistance).max(0.0),
                ..movement
            });
            target.set_needs_sync(true);
        }
        PendingHitImpulse::HoglinThrow { from, power } => {
            let resistance = target.living().map_or(0.0, |living| {
                living.attribute_value(
                    bbb_protocol::attribute_types::VANILLA_ATTRIBUTE_KNOCKBACK_RESISTANCE_ID,
                )
            });
            if let Some(push) = crate::entity::hoglin::throw_impulse(
                from,
                target.position(),
                power,
                resistance,
                store.random(),
            ) {
                target.set_delta_movement(Vec3d {
                    x: target.delta_movement().x + push.x,
                    y: target.delta_movement().y + push.y,
                    z: target.delta_movement().z + push.z,
                });
                target.set_needs_sync(true);
                target.mark_hurt();
            }
        }
    }
    store.attach(target);
}

fn broadcast_attack_event(entity: &Entity, store: &mut EntityStore) {
    store.broadcast_to_tracking(
        entity.id(),
        ClientboundPlayPacket::EntityEvent(bbb_protocol::packets::EntityEvent {
            entity_id: entity.id(),
            event_id: ATTACK_ENTITY_EVENT,
        }),
    );
}

/// `Mob.playAttackSound` and its overrides (`Creaking`).
pub(in crate::entity::ai::mob) fn play_attack_sound(entity: &Entity, store: &mut EntityStore) {
    let resource_id = match entity.entity_type_id() {
        VANILLA_ENTITY_TYPE_CREAKING_ID => "minecraft:entity.creaking.attack",
        VANILLA_ENTITY_TYPE_PANDA_ID => "minecraft:entity.panda.bite",
        VANILLA_ENTITY_TYPE_DOLPHIN_ID => "minecraft:entity.dolphin.attack",
        VANILLA_ENTITY_TYPE_AXOLOTL_ID => "minecraft:entity.axolotl.attack",
        VANILLA_ENTITY_TYPE_RABBIT_ID if crate::entity::rabbit::is_evil(entity) => {
            let random = store.random();
            let pitch = (random.next_float() - random.next_float()) * 0.2 + 1.0;
            play_sound(entity, store, "minecraft:entity.rabbit.attack", pitch);
            return;
        }
        _ => return,
    };
    play_sound(entity, store, resource_id, 1.0);
}

fn play_sound(entity: &Entity, store: &mut EntityStore, resource_id: &'static str, pitch: f32) {
    if entity.is_silent() {
        return;
    }
    store.play_entity_sound(crate::entity::PendingEntitySound {
        position: entity.position(),
        resource_id: resource_id.to_owned(),
        source: crate::entity::living::living_sound_source(entity),
        volume: 1.0,
        pitch,
    });
}
