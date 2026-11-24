use bbb_protocol::mob_effect_types::{
    vanilla_mob_effect_type, MobEffectTypeCatalogEntry, VANILLA_MOB_EFFECT_TYPES,
};
use bbb_protocol::packets::{
    ConsumeEffectSummary, MobEffectDetailsSummary, MobEffectInstanceSummary, SoundEvent,
};

use crate::entity::item_components::sound_holder;
use crate::entity::living::mob_effects::{
    add_active_effect_in_level, remove_active_effect_in_level, remove_all_active_effects_in_level,
};
use crate::entity::store::EntityStore;
use crate::entity::{ActiveMobEffect, Entity, EntityCommand};
use crate::world::simulation_level::LevelCtx;

pub(crate) fn potion_effects(
    potion_id: Option<i32>,
    custom: &[MobEffectInstanceSummary],
) -> Option<impl Iterator<Item = Option<ActiveMobEffect>> + '_> {
    let potion = match potion_id {
        Some(id) => Some(bbb_protocol::potion_types::vanilla_potion(id)?),
        None => None,
    };
    Some(
        potion
            .into_iter()
            .flat_map(|potion| potion.effects)
            .map(|effect| {
                Some(ActiveMobEffect::from_catalog(
                    vanilla_mob_effect_type(effect.effect_id)?,
                    effect.duration,
                    effect.amplifier,
                    false,
                    true,
                    true,
                    None,
                ))
            })
            .chain(custom.iter().map(effect_summary)),
    )
}

pub(crate) fn effect_summary(summary: &MobEffectInstanceSummary) -> Option<ActiveMobEffect> {
    let effect = vanilla_mob_effect_type(summary.effect_id)?;
    let hidden = summary
        .hidden_effect
        .as_deref()
        .map(|hidden| effect_details_summary(effect, hidden));
    Some(ActiveMobEffect::from_catalog(
        effect,
        summary.duration,
        summary.amplifier,
        summary.ambient,
        summary.show_particles,
        summary.show_icon,
        hidden,
    ))
}

fn effect_details_summary(
    effect: &MobEffectTypeCatalogEntry,
    details: &MobEffectDetailsSummary,
) -> ActiveMobEffect {
    let hidden = details
        .hidden_effect
        .as_deref()
        .map(|hidden| effect_details_summary(effect, hidden));
    ActiveMobEffect::from_catalog(
        effect,
        details.duration,
        details.amplifier,
        details.ambient,
        details.show_particles,
        details.show_icon,
        hidden,
    )
}

/// `ConsumeEffect.apply` for a stored mob; an effect that cannot be applied is skipped.
pub(crate) fn apply_to_entity(
    entity: &mut Entity,
    store: &mut EntityStore,
    level: &mut LevelCtx<'_, '_>,
    effect: &ConsumeEffectSummary,
) -> Option<()> {
    match effect {
        ConsumeEffectSummary::ApplyEffects {
            effects,
            probability_bits,
        } => {
            let roll = store.random().next_float();
            if roll >= f32::from_bits(*probability_bits) {
                return Some(());
            }
            for effect in effects {
                let effect = effect_summary(effect).expect("validated consume effect");
                let _ = add_active_effect_in_level(entity, store, level, effect);
            }
        }
        ConsumeEffectSummary::RemoveEffects {
            effect_registry_ids,
            effect_tag,
        } => match effect_tag {
            Some(tag) => {
                let tag = tag.trim_start_matches('#');
                for entry in VANILLA_MOB_EFFECT_TYPES {
                    if level.registry_tag_membership("minecraft:mob_effect", tag, entry.resource_id)
                    {
                        let _ =
                            remove_active_effect_in_level(entity, store, level, entry.resource_id);
                    }
                }
            }
            None => {
                for id in effect_registry_ids {
                    let effect = vanilla_mob_effect_type(*id).expect("validated consume effect id");
                    let _ = remove_active_effect_in_level(entity, store, level, effect.resource_id);
                }
            }
        },
        ConsumeEffectSummary::ClearAllEffects => {
            let _ = remove_all_active_effects_in_level(entity, store, level);
        }
        ConsumeEffectSummary::TeleportRandomly { diameter_bits } => {
            store.queue_command(EntityCommand::TeleportRandomly {
                entity_id: entity.id(),
                diameter: f32::from_bits(*diameter_bits),
            });
        }
        ConsumeEffectSummary::PlaySound { sound } => {
            let position = entity.position();
            let seed = store.random().next_long();
            store.emit_entity_sound(SoundEvent {
                position: bbb_protocol::packets::Vec3d {
                    x: position.x.floor() + 0.5,
                    y: position.y.floor() + 0.5,
                    z: position.z.floor() + 0.5,
                },
                sound: sound_holder(sound).expect("validated consume effect sound"),
                source: crate::entity::living::living_sound_source(entity),
                volume: 1.0,
                pitch: 1.0,
                seed,
            });
        }
    }
    Some(())
}
