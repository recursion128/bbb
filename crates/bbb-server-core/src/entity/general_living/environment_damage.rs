//! Causes that become a hurt without naming an attacker: fluid, fuse, starve,
//! fall and the cramming `LivingEntity.pushEntities` charges its own body.

use super::level_reads::entity_type_tag_membership;
use super::*;
use crate::world::simulation_level::LevelCtx;

impl Entity {
    /// `LivingEntity.isSensitiveToWater` overrides: blaze, enderman, snow golem, strider.
    pub(crate) fn is_sensitive_to_water(&self) -> bool {
        use bbb_protocol::entity_types::{
            VANILLA_ENTITY_TYPE_BLAZE_ID, VANILLA_ENTITY_TYPE_ENDERMAN_ID,
            VANILLA_ENTITY_TYPE_SNOW_GOLEM_ID, VANILLA_ENTITY_TYPE_STRIDER_ID,
        };
        matches!(
            self.entity_type_id(),
            VANILLA_ENTITY_TYPE_BLAZE_ID
                | VANILLA_ENTITY_TYPE_ENDERMAN_ID
                | VANILLA_ENTITY_TYPE_SNOW_GOLEM_ID
                | VANILLA_ENTITY_TYPE_STRIDER_ID
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct GeneralLivingHurtEffects {
    pub outcome: StoredEntityHurtOutcome,
    pub emits_entity_damage: bool,
    pub game_event_source_id: Option<i32>,
    pub died: bool,
    pub perished: bool,
}

pub(crate) fn hurt_from_environment(
    entity: &mut Entity,
    store: &mut EntityStore,
    level: &mut LevelCtx<'_, '_>,
    damage_type: &str,
    amount: f32,
) -> bool {
    hurt_from_environment_outcome(entity, store, level, damage_type, amount).is_some()
}

pub(crate) fn hurt_from_environment_outcome(
    entity: &mut Entity,
    store: &mut EntityStore,
    level: &mut LevelCtx<'_, '_>,
    damage_type: &str,
    amount: f32,
) -> Option<bool> {
    let damage_type = builtin_damage_type_registry()
        .iter()
        .find(|facts| facts.resource_id == damage_type)?;
    let source = StoredEntityDamageSource::without_attacker_views(damage_type, None, None);
    let position = entity.position();
    let effects = hurt_server_with_runtime_in_level(
        entity,
        store,
        level,
        source,
        amount,
        StoredEntityDamageRules::default(),
    );
    if effects.emits_entity_damage {
        level.game_event_at(
            VanillaGameEvent::ENTITY_DAMAGE,
            position,
            effects.game_event_source_id,
        );
    }
    emit_entity_die(store, level, entity, source, effects)?;
    Some(effects.outcome.succeeded)
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
    let horse = is_abstract_horse_entity_type(entity.entity_type_id());
    let mut impulse = entity.living()?.fall_impulse;
    let effective_distance = if horse {
        fall_distance
    } else {
        impulse
            .adjust_fall_distance(fall_distance, entity.position().y)
            .0
    };
    let damage_type = builtin_damage_type_registry()
        .iter()
        .find(|facts| facts.resource_id == damage_type)?;
    let fall_damage_immune =
        entity_type_tag_membership(level, entity, "minecraft:fall_damage_immune");
    let damage = entity.living().map_or(0, |living| {
        calculate_fall_damage(living, effective_distance, modifier, fall_damage_immune)
    });
    let damage = crate::entity::goat::adjust_fall_damage(entity, damage);
    let damage = if entity.entity_type_id() == VANILLA_ENTITY_TYPE_FROG_ID {
        damage - 5
    } else {
        damage
    };
    let llama = matches!(
        entity.entity_type_id(),
        VANILLA_ENTITY_TYPE_LLAMA_ID | VANILLA_ENTITY_TYPE_TRADER_LLAMA_ID
    );
    let needs_hurt = damage > 0 && (!llama || fall_distance >= 6.0);
    let propagates = if horse {
        needs_hurt
    } else {
        !fall_damage_immune
    };
    let source = StoredEntityDamageSource::without_attacker_views(damage_type, None, None);
    let rules = StoredEntityDamageRules {
        fall_damage_immune,
        ..StoredEntityDamageRules::default()
    };
    if !horse {
        entity.living_mut()?.fall_impulse = impulse;
    }
    // Resolve every required dependency before these otherwise unconditional prefixes.
    if !entity.is_silent() {
        let prefix = if entity.entity_type_id()
            == bbb_protocol::entity_types::VANILLA_ENTITY_TYPE_BREEZE_ID
            && fall_distance > 3.0
        {
            Some(("minecraft:entity.breeze.land", SoundSource::Hostile, 1.0))
        } else if horse && !llama && fall_distance > 1.0 {
            let baby = entity.entity_type_id() != VANILLA_ENTITY_TYPE_CAMEL_HUSK_ID
                && matches!(entity.data(), EntityData::GeneralLiving(data) if data.is_baby());
            let sound = if baby {
                "minecraft:entity.baby_horse.land"
            } else {
                "minecraft:entity.horse.land"
            };
            Some((sound, SoundSource::Neutral, 0.4))
        } else {
            None
        };
        if let Some((resource_id, source, volume)) = prefix {
            store.play_entity_sound(PendingEntitySound {
                position: entity.position(),
                resource_id: resource_id.to_owned(),
                source,
                volume,
                pitch: 1.0,
            });
        }
    }
    if propagates && !horse {
        crate::entity::living::propagate_fall_to_passengers(
            entity,
            store,
            level,
            effective_distance,
            modifier,
            damage_resource,
        )?;
    }
    if damage > 0 {
        if !horse {
            entity.living_mut()?.fall_impulse.reset();
            fall_damage_sounds(
                entity,
                store,
                level,
                crate::entity::living::fall_sound_for(entity, damage > 4),
            );
        }
        if needs_hurt
            && entity.creaking_hurt_decision(
                damage_type.bypasses_invulnerability,
                false,
                false,
                false,
            ) != crate::entity::CreakingHurtDecision::Refuse
        {
            let position = entity.position();
            let effects = hurt_server_with_runtime_in_level(
                entity,
                store,
                level,
                source,
                damage as f32,
                rules,
            );
            if effects.emits_entity_damage {
                level.game_event_at(
                    VanillaGameEvent::ENTITY_DAMAGE,
                    position,
                    effects.game_event_source_id,
                );
            }
            emit_entity_die(store, level, entity, source, effects)?;
        }
        if propagates && horse {
            crate::entity::living::propagate_fall_to_passengers(
                entity,
                store,
                level,
                fall_distance,
                modifier,
                damage_resource,
            )?;
        }
        if horse && !entity.is_silent() {
            if let Some(sound) = crate::entity::living::body_block_fall_sound(
                entity.position(),
                SoundSource::Neutral,
                |pos| level.block_state(pos),
            ) {
                store.play_entity_sound(sound);
            }
        }
    }
    if let EntityData::GeneralLiving(data) = &mut entity.data {
        if let Some(creeper) = data.creeper_retained_mut() {
            creeper.apply_fall_distance(fall_distance);
        }
    }
    Some(damage > 0)
}

pub(super) fn refused() -> GeneralLivingHurtEffects {
    GeneralLivingHurtEffects {
        outcome: StoredEntityHurtOutcome {
            applied: false,
            succeeded: false,
        },
        emits_entity_damage: false,
        game_event_source_id: None,
        died: false,
        perished: true,
    }
}
