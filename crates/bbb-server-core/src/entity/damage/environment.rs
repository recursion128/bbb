use crate::level_event::VanillaGameEvent;
use crate::world::simulation_level::LevelCtx;
use bbb_server_datapack::damage::builtin_damage_type_registry;

use super::{spawn_destroyed_drops, StoredEntityDamageRules, StoredEntityDamageSource};
use crate::entity::{Entity, EntityData, EntityStore};

pub(crate) fn supports_detached_environment_hurt(data: &EntityData) -> bool {
    matches!(
        data,
        EntityData::Painting(_)
            | EntityData::ItemFrame(_)
            | EntityData::GlowItemFrame(_)
            | EntityData::LeashKnot(_)
            | EntityData::Item(_)
            | EntityData::ShulkerBullet(_)
            | EntityData::ExperienceOrb(_)
            | EntityData::Arrow(_)
            | EntityData::SpectralArrow(_)
            | EntityData::Trident(_)
            | EntityData::Snowball(_)
            | EntityData::Egg(_)
            | EntityData::EnderPearl(_)
            | EntityData::ExperienceBottle(_)
            | EntityData::SplashPotion(_)
            | EntityData::LingeringPotion(_)
            | EntityData::FishingBobber(_)
            | EntityData::FireworkRocket(_)
            | EntityData::LlamaSpit(_)
            | EntityData::FallingBlock(_)
            | EntityData::Tnt(_)
            | EntityData::EyeOfEnder(_)
            | EntityData::LightningBolt(_)
            | EntityData::EvokerFangs(_)
            | EntityData::Marker(_)
            | EntityData::Interaction(_)
            | EntityData::BlockDisplay(_)
            | EntityData::ItemDisplay(_)
            | EntityData::TextDisplay(_)
            | EntityData::AreaEffectCloud(_)
            | EntityData::OminousItemSpawner(_)
            | EntityData::LargeFireball(_)
            | EntityData::SmallFireball(_)
            | EntityData::DragonFireball(_)
            | EntityData::WitherSkull(_)
            | EntityData::WindCharge(_)
            | EntityData::BreezeWindCharge(_)
    )
}

pub(crate) fn base_tick_from_environment(
    entity: &mut Entity,
    store: &mut EntityStore,
    level: &mut LevelCtx<'_, '_>,
) -> bool {
    let mut resolved = true;
    let ticked =
        crate::entity::base_tick_hurting_on_fire(entity, store, level, |entity, store, level| {
            resolved =
                hurt_detached_from_environment(entity, store, level, "minecraft:on_fire", 1.0)
                    .is_some();
        });
    ticked && resolved
}

pub(crate) fn hurt_detached_from_environment(
    entity: &mut Entity,
    store: &mut EntityStore,
    level: &mut LevelCtx<'_, '_>,
    damage_type: &'static str,
    amount: f32,
) -> Option<bool> {
    if !supports_detached_environment_hurt(entity.data()) {
        return entity
            .living()
            .is_none()
            .then(|| queue_environment_hurt(entity, store, level, damage_type, amount));
    }
    if matches!(entity.data(), EntityData::ShulkerBullet(_)) {
        crate::entity::shulker_bullet::hurt_server(entity, store);
        level.game_event_at(
            VanillaGameEvent::ENTITY_DAMAGE,
            entity.position(),
            Some(entity.id()),
        );
        return Some(true);
    }
    let damage_type = builtin_damage_type_registry()
        .iter()
        .find(|facts| facts.resource_id == damage_type)?;
    let registry_tags = level.capture_registry_tags();
    let rules = StoredEntityDamageRules {
        mob_griefing: level.mob_griefing(),
        entity_drops: level.entity_drops(),
        tnt_explodes: level.tnt_explodes(),
        ..Default::default()
    };
    let mutation = crate::entity::projectile_action::hurt_detached_nonliving_with_facts(
        entity,
        store,
        &registry_tags,
        StoredEntityDamageSource::without_attacker_views(damage_type, None, None),
        amount,
        rules,
        level.item_stack_resolver(),
    )?;
    let mutation = match mutation {
        crate::entity::projectile_action::DetachedNonlivingHurt::Mutation(mutation) => mutation,
        crate::entity::projectile_action::DetachedNonlivingHurt::Hanging(effects) => {
            if effects.emits_entity_die {
                store.publish_detached_hanging_removal(entity)?;
                crate::entity::kill::raise_entity_die(entity, level);
            }
            crate::entity::hanging::apply_hanging_post_actions_in_level(
                effects.actions,
                store,
                level,
                None,
            );
            return Some(effects.outcome);
        }
    };
    if mutation.emits_entity_damage {
        level.game_event_at(
            VanillaGameEvent::ENTITY_DAMAGE,
            entity.position(),
            mutation.game_event_source_id,
        );
    }
    spawn_destroyed_drops(store, entity.position(), mutation.destroyed_drops)?;
    Some(mutation.outcome.succeeded)
}

#[cfg(test)]
mod hanging_tests;

/// Vehicles and end crystals cannot be hurt while detached for their own tick;
/// the hurt lands through the entity-command queue right after that tick.
/// Their `hurtServer` succeeds unless the body is invulnerable
/// (`VehicleEntity.java:35-59`, `EndCrystal.hurtServer`).
fn queue_environment_hurt(
    entity: &Entity,
    store: &mut EntityStore,
    level: &mut LevelCtx<'_, '_>,
    damage_type: &'static str,
    amount: f32,
) -> bool {
    store.queue_command(crate::entity::EntityCommand::Projectile(
        crate::entity::projectile_action::PendingProjectileAction::Hurt(
            crate::entity::projectile_action::PendingProjectileHurt {
                target: crate::level_event::PendingHurtEntity::Stored {
                    entity_id: entity.id(),
                },
                damage_type,
                amount,
                causing: None,
                direct: None,
                direct_resource_id: entity.data().entity_resource_id(),
                registry_tags: level.capture_registry_tags(),
            },
        ),
    ));
    !entity.invulnerable()
}
