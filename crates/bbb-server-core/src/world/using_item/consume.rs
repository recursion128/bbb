//! `Consumable.startConsuming` and `onConsume` (`Consumable.java:62-93`) for a
//! living consumer, with the `ConsumableListener` components and consume effects they
//! run.

use crate::connection::ConnectionId;
use crate::entity::ai::mob::target::TargetRef;
use crate::entity::consume_effect::effect_summary;
use crate::entity::item_components::sound_holder;
use crate::entity::living::mob_effects::{is_instantaneous, ActiveMobEffect};
use crate::level_event::{PendingHurtEntity, VanillaGameEvent};
use crate::random::GameRandom;
use bbb_protocol::mob_effect_types::{
    vanilla_mob_effect_type, vanilla_mob_effect_type_for_resource_id,
};
use bbb_protocol::packets::{
    ConsumeEffectSummary, InteractionHand, ItemStackSummary, SoundEvent, SoundEventHolder,
    SoundSource, Vec3d,
};
use bbb_protocol::sound_event_types::vanilla_sound_event_registry_id;
use bbb_protocol::stat_types::VANILLA_STAT_TYPE_ITEM_USED_ID;
use bbb_server_types::stats::StatKey;

use super::components::{self, ConsumableFacts, FoodFacts};
use super::HeldItem;
use crate::world::advancement_triggers::CONSUME_ITEM;
use crate::world::item_use::{ItemUseContext, ItemUseResult};
use crate::world::{LevelId, ServerWorld, WorldEvent};

/// `OminousBottleAmplifier.EFFECT_DURATION` (`OminousBottleAmplifier.java:22`).
const OMINOUS_BOTTLE_EFFECT_DURATION: i32 = 120_000;

#[derive(Clone, Copy)]
enum ConsumeUser {
    Player(ConnectionId),
    Stored(LevelId, TargetRef),
}

/// `ConsumableListener` components (`Consumable.java:85`).
struct ConsumeListeners {
    food: Option<FoodFacts>,
    potion: Option<Vec<ActiveMobEffect>>,
    suspicious_stew: Option<Vec<ActiveMobEffect>>,
    ominous_bottle_amplifier: Option<i32>,
}

impl ConsumeListeners {
    fn read(stack: &ItemStackSummary) -> Result<Self, String> {
        Ok(Self {
            food: components::food(stack)?,
            potion: components::potion_effects(stack)?,
            suspicious_stew: components::suspicious_stew_effects(stack)?,
            ominous_bottle_amplifier: components::ominous_bottle_amplifier(stack)?,
        })
    }
}

impl ServerWorld {
    // Fox.java:205-220, Fox.java:247-253.
    pub(in crate::world) fn tick_fox_mouth_food(
        &mut self,
        dimension: LevelId,
        target: TargetRef,
        events: &mut Vec<WorldEvent>,
    ) {
        let Some(level) = self.levels.get(&dimension) else {
            return;
        };
        let Some(entity) = level.entities.get(target.entity_id) else {
            return;
        };
        if entity.uuid() != target.uuid
            || !entity.is_alive_with_health()
            || !entity.on_ground()
            || !matches!(entity.data(), crate::entity::EntityData::GeneralLiving(data)
                if !data.is_no_ai() && data.fox_retained().is_some_and(|fox| !fox.is_sleeping()))
            || level
                .entities
                .mob_ai(target.entity_id)
                .is_some_and(|ai| ai.has_target())
        {
            return;
        }
        let Some(ticks) =
            crate::entity::fox::ticks_since_eaten(entity).filter(|ticks| *ticks > 560)
        else {
            return;
        };
        let Some(before) = entity
            .living()
            .and_then(|living| {
                living.item_by_slot(bbb_protocol::equipment_slots::EquipmentSlot::MainHand)
            })
            .filter(|stack| crate::entity::item_components::is_consumable_food(stack) == Ok(true))
        else {
            return;
        };
        if ticks <= 600 {
            if self.random.next_float() < 0.1 {
                let position = entity.position();
                let store = &mut self
                    .levels
                    .get_mut(&dimension)
                    .expect("Fox level exists")
                    .entities;
                store.play_entity_sound(crate::entity::store::PendingEntitySound {
                    position,
                    resource_id: "minecraft:entity.fox.eat".to_owned(),
                    source: SoundSource::Neutral,
                    volume: 1.0,
                    pitch: 1.0,
                });
                store.broadcast_to_tracking(
                    target.entity_id,
                    crate::connection::ClientboundPlayPacket::EntityEvent(
                        bbb_protocol::packets::EntityEvent {
                            entity_id: target.entity_id,
                            event_id: 45,
                        },
                    ),
                );
            }
            return;
        }
        let before = before.clone();
        let consumable = match components::consumable(before.summary()) {
            Ok(Some(consumable)) => consumable,
            Ok(None) => return,
            Err(error) => {
                tracing::warn!("Fox food: {error}");
                return;
            }
        };
        let user = ConsumeUser::Stored(dimension, target);
        self.emit_consume_feedback(user, &consumable, events);
        let consumed = ConsumeListeners::read(before.summary()).and_then(|listeners| {
            self.consume_item_components(user, listeners, &consumable, events)
        });
        if let Err(error) = consumed {
            tracing::warn!("Fox food: {error}");
            return;
        }
        let mut used = before.with_count(before.count() - 1);
        match super::remainder_created_on_use(&used, &before, false) {
            Ok(Some(remainder)) if used.is_empty() => used = remainder,
            Ok(_) => {}
            Err(error) => {
                tracing::warn!("Fox food: {error}");
                return;
            }
        }
        self.with_stored_hurt_level(&dimension, |store, level| {
            store.with_entity_mut(target.entity_id, |entity, store| {
                if entity.uuid() != target.uuid {
                    return;
                }
                if let Err(error) = crate::entity::item_slots::set_living_item_in_tick(
                    entity,
                    store,
                    level,
                    bbb_protocol::equipment_slots::EquipmentSlot::MainHand,
                    (!used.is_empty()).then_some(used),
                ) {
                    tracing::warn!("Fox food: {error}");
                    return;
                }
                crate::entity::fox::reset_ticks_since_eaten(entity);
            });
        });
        self.apply_level_entity_lifecycle_by_key(&dimension, events);
    }

    /// `Consumable.startConsuming` (`Consumable.java:62-75`).
    pub(in crate::world) fn start_consuming(
        &mut self,
        context: &ItemUseContext,
        consumable: &ConsumableFacts<'_>,
        events: &mut Vec<WorldEvent>,
    ) -> Result<(ItemUseResult, Option<HeldItem>), String> {
        let food = components::food(&context.item)?;
        let player = self.levels.player_at(context.connection_id);
        // `Consumable.canConsume` (`:95-98`) over `Player.canEat`
        // (`Player.java:1581-1583`).
        if let Some(food) = food {
            if !(player.abilities.invulnerable || food.can_always_eat || player.food.needs_food()) {
                return Ok((ItemUseResult::Fail, None));
            }
        }
        if consumable.component.consume_ticks() > 0 {
            self.start_using_item(context.connection_id, context.hand);
            return Ok((ItemUseResult::Consume, None));
        }
        self.on_consume(context.connection_id, context.hand, consumable, events)?;
        Ok((ItemUseResult::Consume, Some(HeldItem::Same)))
    }

    /// `Consumable.onConsume` (`Consumable.java:77-93`).
    pub(super) fn on_consume(
        &mut self,
        connection_id: ConnectionId,
        hand: InteractionHand,
        consumable: &ConsumableFacts<'_>,
        events: &mut Vec<WorldEvent>,
    ) -> Result<(), String> {
        let stack = super::runtime_item_in_hand(self.levels.player_at(connection_id), hand)
            .summary()
            .clone();
        let listeners = ConsumeListeners::read(&stack)?;
        self.emit_player_consume_sound(connection_id, consumable, events);
        if let Some(item_id) = stack.item_id {
            self.levels[connection_id].award_stat_once(
                &mut self.scoreboard,
                StatKey::new(VANILLA_STAT_TYPE_ITEM_USED_ID, item_id),
            );
        }
        self.trigger_item_criterion(connection_id, CONSUME_ITEM, &stack, events);
        if self.levels.player(connection_id).is_none() {
            return Ok(());
        }
        self.consume_item_components(
            ConsumeUser::Player(connection_id),
            listeners,
            consumable,
            events,
        )?;
        let Some(player) = self.levels.player(connection_id) else {
            return Ok(());
        };
        if !player.abilities.instabuild {
            let mut held = super::runtime_item_in_hand(player, hand).clone();
            held.set_count(held.count() - 1);
            self.update_hand_stack(connection_id, hand, held);
        }
        Ok(())
    }

    fn consume_item_components(
        &mut self,
        user: ConsumeUser,
        listeners: ConsumeListeners,
        consumable: &ConsumableFacts<'_>,
        events: &mut Vec<WorldEvent>,
    ) -> Result<(), String> {
        if let Some(food) = listeners.food {
            self.food_on_consume(user, consumable, food, events);
        }
        if let Some(effects) = listeners.potion {
            self.potion_on_consume(user, effects, events);
        }
        if let Some(effects) = listeners.suspicious_stew {
            // `SuspiciousStewEffects.onConsume` (`SuspiciousStewEffects.java:39-43`).
            for effect in effects {
                self.add_consumer_mob_effect(user, effect, events);
            }
        }
        if let Some(amplifier) = listeners.ominous_bottle_amplifier {
            // `OminousBottleAmplifier.onConsume` (`OminousBottleAmplifier.java:31-33`).
            if let Some(effect) = ActiveMobEffect::from_saved_with_hidden(
                "minecraft:bad_omen",
                OMINOUS_BOTTLE_EFFECT_DURATION,
                amplifier,
                false,
                false,
                true,
                None,
            ) {
                self.add_consumer_mob_effect(user, effect, events);
            }
        }
        for effect in consumable.on_consume_effects {
            match user {
                ConsumeUser::Player(connection_id) => {
                    self.apply_consume_effect_checked(connection_id, effect, events)?;
                }
                ConsumeUser::Stored(dimension, target) => {
                    let applied = self.with_stored_hurt_level(&dimension, |store, level| {
                        store.with_entity_mut(target.entity_id, |entity, store| {
                            if entity.uuid() != target.uuid {
                                return None;
                            }
                            crate::entity::consume_effect::apply_to_entity(
                                entity, store, level, effect,
                            )
                        })?
                    });
                    applied
                        .ok_or_else(|| "stored consumer could not apply its effect".to_owned())?;
                }
            }
        }
        let event = if consumable.component.is_drink() {
            VanillaGameEvent::DRINK
        } else {
            VanillaGameEvent::EAT
        };
        match user {
            ConsumeUser::Player(connection_id) => self.player_game_event(connection_id, event),
            ConsumeUser::Stored(dimension, target) => {
                if let Some((_, position, _)) = self.consume_origin(user) {
                    self.with_level(dimension, |level| {
                        level.game_event_at(event, position, Some(target.entity_id));
                    });
                }
            }
        }
        Ok(())
    }

    fn add_consumer_mob_effect(
        &mut self,
        user: ConsumeUser,
        effect: ActiveMobEffect,
        events: &mut Vec<WorldEvent>,
    ) {
        match user {
            ConsumeUser::Player(connection_id) => {
                self.add_player_mob_effect(connection_id, effect, events);
            }
            ConsumeUser::Stored(dimension, target) => {
                self.add_stored_mob_effect(&dimension, target.entity_id, effect);
            }
        }
    }

    /// `Consumable.emitParticlesAndSounds` (`Consumable.java:104-117`).
    pub(super) fn emit_player_consume_sound(
        &mut self,
        connection_id: ConnectionId,
        consumable: &ConsumableFacts<'_>,
        events: &mut Vec<WorldEvent>,
    ) {
        self.emit_consume_feedback(ConsumeUser::Player(connection_id), consumable, events);
    }

    fn consume_origin(&self, user: ConsumeUser) -> Option<(LevelId, Vec3d, SoundSource)> {
        match user {
            ConsumeUser::Player(connection_id) => {
                let player = self.levels.player(connection_id)?;
                Some((player.dimension, player.position(), SoundSource::Players))
            }
            ConsumeUser::Stored(dimension, target) => {
                let entity = self
                    .levels
                    .get(&dimension)?
                    .entities
                    .get(target.entity_id)?;
                (entity.uuid() == target.uuid).then(|| {
                    (
                        dimension,
                        entity.position(),
                        crate::entity::living::living_sound_source(entity),
                    )
                })
            }
        }
    }

    fn emit_consume_feedback(
        &mut self,
        user: ConsumeUser,
        consumable: &ConsumableFacts<'_>,
        events: &mut Vec<WorldEvent>,
    ) {
        let Some((dimension, position, source)) = self.consume_origin(user) else {
            return;
        };
        let random = &self.random;
        let eat_volume = if random.next_boolean() { 0.5 } else { 1.0 };
        let eat_pitch = triangle(random, 1.0, 0.2);
        let drink_pitch = random_between(random, 0.9, 1.0);
        let (volume, pitch) = if consumable.component.is_drink() {
            (0.5, drink_pitch)
        } else {
            (eat_volume, eat_pitch)
        };
        // `Player.playSound` (`Player.java:397-399`) skips the player itself.
        let except = match user {
            ConsumeUser::Player(connection_id) => Some(connection_id),
            ConsumeUser::Stored(..) => None,
        };
        let sound = SoundEvent {
            sound: sound_holder(consumable.sound).expect("validated consumable sound"),
            source,
            position,
            volume,
            pitch,
            seed: self.random.next_long(),
        };
        self.broadcast_level_sound_by_key(&dimension, &sound, except, events);
    }

    /// `FoodProperties.onConsume` (`FoodProperties.java:40-48`).
    fn food_on_consume(
        &mut self,
        user: ConsumeUser,
        consumable: &ConsumableFacts<'_>,
        food: FoodFacts,
        events: &mut Vec<WorldEvent>,
    ) {
        let Some((dimension, position, _)) = self.consume_origin(user) else {
            return;
        };
        let pitch = triangle(&self.random, 1.0, 0.4);
        let sound = SoundEvent {
            sound: sound_holder(consumable.sound).expect("validated consumable sound"),
            source: SoundSource::Neutral,
            position,
            volume: 1.0,
            pitch,
            seed: self.random.next_long(),
        };
        self.broadcast_level_sound_by_key(&dimension, &sound, None, events);
        let ConsumeUser::Player(connection_id) = user else {
            return;
        };
        let player = self
            .levels
            .player_mut(connection_id)
            .expect("the consuming session is connected");
        player
            .food
            .eat_food_properties(food.nutrition, food.saturation);
        let pitch = random_between(&self.random, 0.9, 1.0);
        let position = player.position();
        if let Some(registry_id) = vanilla_sound_event_registry_id("minecraft:entity.player.burp") {
            self.play_player_sound(
                connection_id,
                None,
                SoundEventHolder::Reference { registry_id },
                SoundSource::Players,
                position,
                0.5,
                pitch,
                events,
            );
        }
    }

    /// `PotionContents.applyToLivingEntity` (`PotionContents.java:154-165`)
    /// with a player source only when the consumer is a player.
    fn potion_on_consume(
        &mut self,
        user: ConsumeUser,
        effects: Vec<ActiveMobEffect>,
        events: &mut Vec<WorldEvent>,
    ) {
        for effect in effects {
            if !is_instantaneous(effect.resource_id()) {
                self.add_consumer_mob_effect(user, effect, events);
                continue;
            }
            match user {
                ConsumeUser::Stored(dimension, target) => {
                    self.with_stored_hurt_level(&dimension, |store, level| {
                        store.with_entity_mut(target.entity_id, |entity, store| {
                            if entity.uuid() == target.uuid {
                                crate::entity::area_effect_cloud::apply_consumed_instant_effect(
                                    entity,
                                    store,
                                    level,
                                    effect.resource_id(),
                                    effect.amplifier(),
                                );
                            }
                        });
                    });
                }
                ConsumeUser::Player(connection_id) => {
                    let player = self.levels.player_at(connection_id);
                    let dimension = player.dimension;
                    let source = self.pending_hurt_source_entity(
                        &dimension,
                        PendingHurtEntity::Player {
                            uuid: player.profile.uuid,
                        },
                    );
                    self.apply_player_instant_effect(
                        connection_id,
                        effect.resource_id(),
                        effect.amplifier(),
                        1.0,
                        source.clone(),
                        source,
                        events,
                    );
                }
            }
        }
    }

    /// `ConsumeEffect.apply` for the five built-in types.
    pub(super) fn apply_consume_effect(
        &mut self,
        connection_id: ConnectionId,
        effect: &ConsumeEffectSummary,
        events: &mut Vec<WorldEvent>,
    ) {
        if let ConsumeEffectSummary::TeleportRandomly { diameter_bits } = effect {
            self.teleport_randomly(connection_id, f32::from_bits(*diameter_bits), events);
        } else {
            let _ = self.apply_consume_effect_checked(connection_id, effect, events);
        }
    }

    fn apply_consume_effect_checked(
        &mut self,
        connection_id: ConnectionId,
        effect: &ConsumeEffectSummary,
        events: &mut Vec<WorldEvent>,
    ) -> Result<(), String> {
        match effect {
            // `ApplyStatusEffectsConsumeEffect.apply`
            // (`ApplyStatusEffectsConsumeEffect.java:49-63`).
            ConsumeEffectSummary::ApplyEffects {
                effects,
                probability_bits,
            } => {
                let roll = self.random.next_float();
                if roll >= f32::from_bits(*probability_bits) {
                    return Ok(());
                }
                for effect in effects {
                    self.add_player_mob_effect(
                        connection_id,
                        effect_summary(effect).expect("validated consume effect"),
                        events,
                    );
                }
            }
            // `RemoveStatusEffectsConsumeEffect.apply`
            // (`RemoveStatusEffectsConsumeEffect.java:36-46`).
            ConsumeEffectSummary::RemoveEffects {
                effect_registry_ids,
                effect_tag,
            } => {
                if let Some(tag) = effect_tag {
                    let members: Vec<&str> = self
                        .player_registry_tags(connection_id)
                        .map(|tags| {
                            tags.members("minecraft:mob_effect", tag.trim_start_matches('#'))
                                .iter()
                                .filter_map(|id| vanilla_mob_effect_type_for_resource_id(id))
                                .map(|entry| entry.resource_id)
                                .collect()
                        })
                        .unwrap_or_default();
                    for id in members {
                        self.remove_player_mob_effect(connection_id, id, events);
                    }
                } else {
                    for id in effect_registry_ids {
                        let entry =
                            vanilla_mob_effect_type(*id).expect("validated consume effect id");
                        self.remove_player_mob_effect(connection_id, entry.resource_id, events);
                    }
                }
            }
            // `ClearAllStatusEffectsConsumeEffect.apply`
            // (`ClearAllStatusEffectsConsumeEffect.java:21-23`).
            ConsumeEffectSummary::ClearAllEffects => {
                self.remove_all_player_mob_effects(connection_id, events);
            }
            ConsumeEffectSummary::TeleportRandomly { diameter_bits } => {
                self.teleport_randomly_checked(
                    connection_id,
                    f32::from_bits(*diameter_bits),
                    events,
                )?;
            }
            // `PlaySoundConsumeEffect.apply` (`PlaySoundConsumeEffect.java:27-30`)
            // at the block centre `Level.playSound(.., BlockPos, ..)`
            // (`Level.java:389-393`) resolves.
            ConsumeEffectSummary::PlaySound { sound } => {
                let position = self.levels[connection_id].position();
                let centre = Vec3d {
                    x: position.x.floor() + 0.5,
                    y: position.y.floor() + 0.5,
                    z: position.z.floor() + 0.5,
                };
                self.play_player_sound(
                    connection_id,
                    None,
                    sound_holder(sound).expect("validated consume effect sound"),
                    SoundSource::Players,
                    centre,
                    1.0,
                    1.0,
                    events,
                );
            }
        }
        Ok(())
    }

    /// `ServerLevel.playSeededSound` (`ServerLevel.java:1023-1045`) from a
    /// session's level with the level's own seed.
    #[allow(clippy::too_many_arguments)]
    pub(in crate::world) fn play_player_sound(
        &mut self,
        connection_id: ConnectionId,
        except: Option<ConnectionId>,
        sound: SoundEventHolder,
        source: SoundSource,
        position: Vec3d,
        volume: f32,
        pitch: f32,
        events: &mut Vec<WorldEvent>,
    ) {
        let Some(player) = self.levels.player(connection_id) else {
            return;
        };
        let dimension = player.dimension;
        let sound = SoundEvent {
            sound,
            source,
            position,
            volume,
            pitch,
            seed: self.random.next_long(),
        };
        self.broadcast_level_sound_by_key(&dimension, &sound, except, events);
    }
}

/// `RandomSource.triangle(float, float)` (`RandomSource.java:63-65`).
fn triangle(random: &GameRandom, mean: f32, spread: f32) -> f32 {
    mean + spread * (random.next_float() - random.next_float())
}

/// `Mth.randomBetween` (`Mth.java:668-670`).
fn random_between(random: &GameRandom, min: f32, max_exclusive: f32) -> f32 {
    random.next_float() * (max_exclusive - min) + min
}
