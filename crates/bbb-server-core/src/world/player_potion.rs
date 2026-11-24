//! Potion effects landing on a session player: drinking, splashes, water
//! splashes and area effect clouds.

use bbb_protocol::packets::{SoundEventHolder, SoundSource};
use bbb_protocol::sound_event_types::vanilla_sound_event_registry_id;

use crate::commands::CommandDamageSourceEntity;
use crate::connection::ConnectionId;
use crate::entity::thrown_potion::{PendingPlayerPotion, PlayerPotionEffect};
use crate::entity::EntityData;
use crate::level_event::PendingHurtEntity;
use crate::world::combat::EnchantmentEffectEntity as SourceEntity;
use crate::world::{LevelId, ServerWorld, WorldEvent};

impl ServerWorld {
    pub(in crate::world) fn apply_player_potion(
        &mut self,
        dimension: &LevelId,
        potion: PendingPlayerPotion,
        events: &mut Vec<WorldEvent>,
    ) {
        let Some(connection_id) = self
            .level_by_id(dimension)
            .entities
            .player_connection(potion.player_entity_id)
        else {
            return;
        };
        for effect in potion.effects {
            match effect {
                PlayerPotionEffect::Instant {
                    resource_id,
                    amplifier,
                    scale,
                } => {
                    let direct = self.pending_hurt_source_entity(
                        dimension,
                        PendingHurtEntity::Stored {
                            entity_id: potion.direct_entity_id,
                        },
                    );
                    let owner = potion.owner.and_then(|uuid| {
                        if self.connection_of(uuid).is_some() {
                            self.pending_hurt_source_entity(
                                dimension,
                                PendingHurtEntity::Player { uuid },
                            )
                        } else {
                            let (owner_dimension, entity_id) =
                                self.entity_reference(dimension, uuid)?;
                            self.pending_hurt_source_entity(
                                &owner_dimension,
                                PendingHurtEntity::Stored { entity_id },
                            )
                        }
                    });
                    self.apply_player_instant_effect(
                        connection_id,
                        resource_id,
                        amplifier,
                        scale,
                        owner,
                        direct,
                        events,
                    );
                }
                PlayerPotionEffect::Add(effect) => {
                    let source = if self
                        .level_by_id(dimension)
                        .entities
                        .get(potion.direct_entity_id)
                        .is_some_and(|entity| {
                            matches!(entity.data(), EntityData::AreaEffectCloud(_))
                        }) {
                        (*dimension, SourceEntity::Stored(potion.direct_entity_id))
                    } else {
                        self.projectile_effect_source(
                            *dimension,
                            potion.owner,
                            potion.direct_entity_id,
                        )
                    };
                    self.add_player_mob_effect_from(connection_id, effect, Some(source), events);
                }
                PlayerPotionEffect::Extinguish => self.extinguish_player(connection_id, events),
            }
        }
    }

    /// `MobEffect.applyInstantenousEffect`: `HealOrHarmMobEffect`
    /// (`HealOrHarmMobEffect.java:28-48`) and `SaturationMobEffect.applyEffectTick`
    /// (`SaturationMobEffect.java:13-19`).
    #[allow(clippy::too_many_arguments)]
    pub(in crate::world) fn apply_player_instant_effect(
        &mut self,
        connection_id: ConnectionId,
        resource_id: &str,
        amplifier: i32,
        scale: f64,
        owner: Option<CommandDamageSourceEntity>,
        direct: Option<CommandDamageSourceEntity>,
        events: &mut Vec<WorldEvent>,
    ) {
        if resource_id == "minecraft:saturation" {
            if let Some(player) = self.levels.player_mut(connection_id) {
                player.food.eat(amplifier.wrapping_add(1), 1.0);
            }
            return;
        }
        let is_harm = match resource_id {
            "minecraft:instant_damage" => true,
            "minecraft:instant_health" => false,
            _ => return,
        };
        let Some(inverted) = self.player_registry_tags(connection_id).map(|tags| {
            tags.contains(
                "minecraft:entity_type",
                "minecraft:inverted_healing_and_harm",
                "minecraft:player",
            )
        }) else {
            return;
        };
        if is_harm == inverted {
            let amount = (scale * f64::from(4_i32.wrapping_shl(amplifier as u32)) + 0.5) as i32;
            if let Some(player) = self.levels.player_mut(connection_id) {
                let max_health = player.max_health();
                player.damage.heal(amount as f32, max_health);
            }
        } else {
            let amount = (scale * f64::from(6_i32.wrapping_shl(amplifier as u32)) + 0.5) as i32;
            self.hurt_connected_player(
                connection_id,
                "minecraft:indirect_magic",
                amount as f32,
                owner,
                direct,
                None,
                events,
            );
        }
    }

    /// `Entity.extinguishFire` (`Entity.java:966-972`) on a living player.
    fn extinguish_player(&mut self, connection_id: ConnectionId, events: &mut Vec<WorldEvent>) {
        let Some(player) = self.levels.player_mut(connection_id) else {
            return;
        };
        if player.damage.is_dead_or_dying() || player.remaining_fire_ticks <= 0 {
            return;
        }
        player.remaining_fire_ticks = 0;
        let position = player.position();
        let pitch = 1.6_f32 + (self.random.next_float() - self.random.next_float()) * 0.4_f32;
        let Some(registry_id) =
            vanilla_sound_event_registry_id("minecraft:entity.generic.extinguish_fire")
        else {
            return;
        };
        self.play_player_sound(
            connection_id,
            None,
            SoundEventHolder::Reference { registry_id },
            SoundSource::Players,
            position,
            0.7,
            pitch,
            events,
        );
    }
}

#[cfg(test)]
impl ServerWorld {
    pub(crate) fn player_fire_ticks_for_test(&self, connection_id: ConnectionId) -> i32 {
        self.levels[connection_id].remaining_fire_ticks
    }

    pub(crate) fn set_player_fire_ticks_for_test(
        &mut self,
        connection_id: ConnectionId,
        ticks: i32,
    ) {
        self.levels[connection_id].remaining_fire_ticks = ticks;
    }
}
