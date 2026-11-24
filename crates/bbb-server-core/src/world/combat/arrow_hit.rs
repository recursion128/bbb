//! The world half of `AbstractArrow.onHitEntity` (`AbstractArrow.java:419-505`):
//! everything that needs the hurt's outcome or a session.

use bbb_protocol::packets::{GameEvent, Vec3d};
use bbb_server_types::geometry::Vec3Ops;
use uuid::Uuid;

use super::super::{send, ServerWorld, WorldEvent};
use crate::connection::ClientboundPlayPacket;
use crate::enchantment_combat_effects::CombatValue;
use crate::entity::living::mob_effects::ActiveMobEffect;
use crate::entity::projectile_action::{ArrowRefusal, PendingArrowHit};
use crate::entity::store::PendingEntitySound;
use crate::level_event::PendingHurtEntity;
use crate::world::LevelId;
use bbb_protocol::attribute_types::VANILLA_ATTRIBUTE_KNOCKBACK_RESISTANCE_ID;

use super::enchanted_damage::{EnchantedDamageSource, SourceEntity};
use super::player_melee::MeleeTarget;

/// `Entity.igniteForSeconds(5.0F)` (`AbstractArrow.java:455`).
const ARROW_FIRE_TICKS: i32 = 100;

impl ServerWorld {
    /// `LivingEntity.setLastHurtMob` on a player owner (`AbstractArrow.java:454`,
    /// `AbstractWindCharge.java:83`); a non-living target clears it.
    pub(in crate::world) fn set_owner_last_hurt_mob(
        &mut self,
        dimension: &LevelId,
        owner: Option<uuid::Uuid>,
        target: &PendingHurtEntity,
    ) {
        let Some(owner) = owner.and_then(|uuid| self.connection_of(uuid)) else {
            return;
        };
        let target_ref = match *target {
            PendingHurtEntity::Player { uuid } => self
                .connection_of(uuid)
                .and_then(|id| self.levels.player(id))
                .map(|player| (player.entity_id(), uuid)),
            PendingHurtEntity::Stored { entity_id } => self
                .level_by_id(dimension)
                .entities
                .get(entity_id)
                .filter(|entity| entity.living().is_some())
                .map(|entity| (entity_id, entity.uuid())),
            PendingHurtEntity::FallingBlock { .. } => None,
        };
        if let Some(owner) = self.levels.player_mut(owner) {
            owner.set_last_hurt_mob(target_ref);
        }
    }

    pub(in crate::world) fn apply_arrow_hit(
        &mut self,
        dimension: LevelId,
        mut hit: PendingArrowHit,
        events: &mut Vec<WorldEvent>,
    ) -> Option<()> {
        let Some(arrow) = self
            .level_by_id(&dimension)
            .entities
            .get(hit.arrow_entity_id)
            .filter(|arrow| arrow.is_alive())
        else {
            return Some(());
        };
        // A prior hit in the same tick may have bounced this non-trident arrow.
        if !hit.trident {
            let current = arrow.delta_movement();
            let prev = hit.damage.movement;
            if prev.x * current.x + prev.y * current.y + prev.z * current.z < 0.0 {
                return Some(());
            }
        }
        let victim = self.arrow_victim(&hit.hurt.target);
        hit.hurt.amount = self.arrow_damage(&dimension, &hit, victim);
        let target_player = match hit.hurt.target {
            PendingHurtEntity::Player { uuid } => self.connection_of(uuid),
            _ => None,
        };
        let owner_player = hit.owner.and_then(|uuid| self.connection_of(uuid));
        self.set_owner_last_hurt_mob(&dimension, hit.owner, &hit.hurt.target);
        let old_fire_ticks = match hit.hurt.target {
            PendingHurtEntity::Player { .. } => target_player
                .and_then(|id| self.levels.player_mut(id))
                .map(|player| {
                    let old = player.remaining_fire_ticks;
                    if hit.ignites {
                        player.ignite_environment(ARROW_FIRE_TICKS);
                    }
                    old
                }),
            PendingHurtEntity::Stored { entity_id } => self
                .level_mut_by_id(&dimension)
                .entities
                .get_mut(entity_id)
                .map(|entity| {
                    let old = entity.remaining_fire_ticks();
                    if hit.ignites {
                        crate::entity::living::ignite_living_for_ticks(entity, ARROW_FIRE_TICKS);
                    }
                    old
                }),
            PendingHurtEntity::FallingBlock { .. } => None,
        };
        let accepted = self.apply_projectile_hurt(dimension, hit.hurt.clone(), None, events)?;
        if !accepted {
            match hit.hurt.target {
                PendingHurtEntity::Player { .. } => {
                    if let (Some(old), Some(player)) = (
                        old_fire_ticks,
                        target_player.and_then(|id| self.levels.player_mut(id)),
                    ) {
                        player.set_remaining_fire_ticks(old);
                    }
                }
                PendingHurtEntity::Stored { entity_id } => {
                    if let (Some(old), Some(entity)) = (
                        old_fire_ticks,
                        self.level_mut_by_id(&dimension).entities.get_mut(entity_id),
                    ) {
                        entity.set_remaining_fire_ticks(old);
                    }
                }
                PendingHurtEntity::FallingBlock { .. } => {}
            }
            if let ArrowRefusal::Bounce { pickup } = hit.refusal {
                bounce(
                    &mut self.level_mut_by_id(&dimension).entities,
                    hit.arrow_entity_id,
                    pickup,
                );
            }
            return Some(());
        }
        if hit.enderman {
            return Some(());
        }
        let arrow_silent = self
            .level_by_id(&dimension)
            .entities
            .get(hit.arrow_entity_id)
            .is_none_or(|arrow| arrow.is_silent());
        match hit.hurt.target {
            PendingHurtEntity::Player { .. } => {
                if let Some(id) = target_player {
                    self.arrow_knockback(&dimension, &hit, victim);
                    if hit.sticks {
                        if let Some(player) = self.levels.player_mut(id) {
                            player.add_stuck_arrow();
                        }
                    }
                    if !hit.effects.is_empty() {
                        let source = self.projectile_effect_source(
                            dimension,
                            hit.owner,
                            hit.arrow_entity_id,
                        );
                        for effect in std::mem::take(&mut hit.effects) {
                            self.add_player_mob_effect_from(id, effect, Some(source), events);
                        }
                    }
                    // `ClientboundGameEventPacket.PLAY_ARROW_HIT_SOUND` (`:477-479`).
                    if let Some(owner) = owner_player.filter(|owner| *owner != id && !arrow_silent)
                    {
                        events.push(send(
                            owner,
                            ClientboundPlayPacket::GameEvent(GameEvent {
                                event_id: bbb_protocol::game_event_types::VANILLA_CLIENT_GAME_EVENT_PLAY_ARROW_HIT_SOUND_ID,
                                param: 0.0,
                            }),
                        ));
                    }
                }
            }
            PendingHurtEntity::Stored { entity_id } => {
                let living = self
                    .level_by_id(&dimension)
                    .entities
                    .get(entity_id)
                    .is_some_and(|entity| entity.living().is_some());
                if living {
                    self.arrow_knockback(&dimension, &hit, victim);
                    if hit.sticks {
                        if let Some(entity) =
                            self.level_mut_by_id(&dimension).entities.get_mut(entity_id)
                        {
                            crate::entity::living::add_arrow(entity);
                        }
                    }
                    for effect in std::mem::take(&mut hit.effects) {
                        self.add_stored_mob_effect(&dimension, entity_id, effect);
                    }
                }
            }
            PendingHurtEntity::FallingBlock { .. } => {}
        }
        let victim_living = match hit.hurt.target {
            PendingHurtEntity::Player { .. } => target_player.is_some(),
            PendingHurtEntity::Stored { entity_id } => self
                .level_by_id(&dimension)
                .entities
                .get(entity_id)
                .is_some_and(|entity| entity.living().is_some()),
            PendingHurtEntity::FallingBlock { .. } => false,
        };
        if let Some(victim) = victim.filter(|_| hit.trident || victim_living) {
            if let Err(detail) = self.arrow_post_attack(&dimension, &hit, victim, events) {
                tracing::warn!("projectile post_attack: {detail}");
            }
        }
        if let (Some(owner), Some(victim), false) = (owner_player, victim, hit.trident) {
            if victim_living {
                self.arrow_kill_trigger(&dimension, owner, victim, &hit, events);
            }
        }
        let store = &mut self.level_mut_by_id(&dimension).entities;
        if let Some((resource_id, pitch)) = hit.hit_sound {
            if let Some(arrow) = store
                .get(hit.arrow_entity_id)
                .filter(|arrow| !arrow.is_silent())
            {
                let position = arrow.position();
                store.play_entity_sound(PendingEntitySound {
                    position,
                    resource_id,
                    source: bbb_protocol::packets::SoundSource::Neutral,
                    volume: 1.0,
                    pitch,
                });
            }
        }
        if hit.discard_on_success {
            store.remove(hit.arrow_entity_id, crate::entity::RemovalReason::Discarded);
        }
        Some(())
    }

    /// `EnchantmentHelper.doPostAttackEffectsWithItemSource` under
    /// `arrow(this, owner)` / `trident(this, owner)` with the projectile's
    /// weapon (`AbstractArrow.java:474-476`, `ThrownTrident.java:134-138`).
    fn arrow_post_attack(
        &mut self,
        level: &LevelId,
        hit: &PendingArrowHit,
        victim: MeleeTarget,
        events: &mut Vec<WorldEvent>,
    ) -> Result<(), String> {
        let registry = std::sync::Arc::clone(&self.cold.damage_types);
        let damage_type = registry
            .iter()
            .find(|entry| entry.resource_id == hit.hurt.damage_type)
            .ok_or_else(|| format!("{} is not registered", hit.hurt.damage_type))?;
        let direct = SourceEntity::Stored(hit.arrow_entity_id);
        let causing = match hit.hurt.causing {
            Some(PendingHurtEntity::Player { uuid }) => {
                self.connection_of(uuid).map(SourceEntity::Session)
            }
            Some(PendingHurtEntity::Stored { entity_id }) => Some(SourceEntity::Stored(entity_id)),
            _ => None,
        };
        let source = super::player_damage_reduction::PlayerHurtSource {
            damage_type,
            causing: causing.or(Some(direct)),
            direct: Some(direct),
        };
        self.projectile_weapon_post_attack(
            level,
            victim,
            source,
            hit.arrow_entity_id,
            hit.damage.weapon.as_ref(),
            hit.trident,
            events,
        )
    }

    /// `CriteriaTriggers.KILLED_BY_ARROW` for the owning player
    /// (`AbstractArrow.java:483-493`).
    fn arrow_kill_trigger(
        &mut self,
        level: &LevelId,
        owner: crate::connection::ConnectionId,
        victim: MeleeTarget,
        hit: &PendingArrowHit,
        events: &mut Vec<WorldEvent>,
    ) {
        let (victim_id, dead) = match victim {
            MeleeTarget::Session(id) => match self.levels.player(id) {
                Some(player) => (player.entity_id(), player.damage.is_dead_or_dying()),
                None => return,
            },
            MeleeTarget::Stored(id) => (
                id,
                self.level_by_id(level)
                    .entities
                    .get(id)
                    .is_none_or(|entity| !entity.is_alive_with_health()),
            ),
        };
        let victims = if hit.piercing {
            match self
                .level_mut_by_id(level)
                .entities
                .get_mut(hit.arrow_entity_id)
            {
                Some(arrow) => crate::entity::abstract_arrow::add_pierced_and_killed(
                    arrow,
                    dead.then_some(victim_id),
                ),
                None => return,
            }
        } else if dead {
            vec![victim_id]
        } else {
            return;
        };
        self.trigger_killed_by_arrow(owner, level, &victims, hit.damage.weapon.as_ref(), events);
    }

    fn arrow_victim(&self, target: &PendingHurtEntity) -> Option<MeleeTarget> {
        match *target {
            PendingHurtEntity::Player { uuid } => {
                self.connection_of(uuid).map(MeleeTarget::Session)
            }
            PendingHurtEntity::Stored { entity_id } => Some(MeleeTarget::Stored(entity_id)),
            PendingHurtEntity::FallingBlock { .. } => None,
        }
    }

    /// The weapon's damage or knockback enchantments under `arrow(this, owner)`
    /// (`AbstractArrow.java:425-431`, `:507-516`).
    fn arrow_weapon_value(
        &mut self,
        level: &LevelId,
        hit: &PendingArrowHit,
        victim: Option<MeleeTarget>,
        value: CombatValue,
        amount: f32,
    ) -> f32 {
        let (Some(weapon), Some(victim)) = (&hit.damage.weapon, victim) else {
            return amount;
        };
        let enchantments = crate::entity::enchanted_damage::stack_enchantments(
            weapon,
            &self.datapack_tables.enchantment_slots,
        );
        if enchantments.is_empty() {
            return amount;
        }
        let direct = SourceEntity::Stored(hit.arrow_entity_id);
        let causing = match hit.hurt.causing {
            Some(PendingHurtEntity::Player { uuid }) => {
                self.connection_of(uuid).map(SourceEntity::Session)
            }
            Some(PendingHurtEntity::Stored { entity_id }) => Some(SourceEntity::Stored(entity_id)),
            _ => None,
        }
        .unwrap_or(direct);
        let source = EnchantedDamageSource {
            damage_type: hit.hurt.damage_type,
            is_direct: causing == direct,
            causing: Some(causing),
            direct: Some(direct),
        };
        self.enchanted_damage_value(level, &enchantments, victim, &source, value, amount)
            .unwrap_or_else(|detail| {
                tracing::warn!("projectile weapon enchantment {value:?}: {detail}");
                amount
            })
    }

    fn arrow_damage(
        &mut self,
        level: &LevelId,
        hit: &PendingArrowHit,
        victim: Option<MeleeTarget>,
    ) -> f32 {
        let base =
            self.arrow_weapon_value(level, hit, victim, CombatValue::Damage, hit.damage.base);
        let Some(speed) = hit.damage.speed else {
            return base;
        };
        let mut damage = (f64::from(speed) * f64::from(base))
            .clamp(0.0, 2.147_483_647e9)
            .ceil() as i32;
        if hit.damage.crit {
            let increase = i64::from(self.random.next_int(damage / 2 + 2));
            damage = (increase + i64::from(damage)).min(i64::from(i32::MAX)) as i32;
        }
        damage as f32
    }

    /// `AbstractArrow.doKnockback` (`AbstractArrow.java:507-516`).
    fn arrow_knockback(
        &mut self,
        level: &LevelId,
        hit: &PendingArrowHit,
        victim: Option<MeleeTarget>,
    ) {
        let knockback = self.arrow_weapon_value(level, hit, victim, CombatValue::Knockback, 0.0);
        if knockback <= 0.0 {
            return;
        }
        let resistance = match victim {
            Some(MeleeTarget::Session(id)) => self.levels[id]
                .attributes
                .value(VANILLA_ATTRIBUTE_KNOCKBACK_RESISTANCE_ID)
                .unwrap_or(0.0),
            Some(MeleeTarget::Stored(id)) => self
                .level_by_id(level)
                .entities
                .get(id)
                .and_then(|entity| entity.living())
                .map_or(0.0, |living| {
                    living.attribute_value(VANILLA_ATTRIBUTE_KNOCKBACK_RESISTANCE_ID)
                }),
            None => return,
        };
        let push = hit
            .damage
            .movement
            .multiply(1.0, 0.0, 1.0)
            .normalize()
            .scale(f64::from(knockback) * 0.6 * (1.0 - resistance).max(0.0));
        if push.length_sqr() <= 0.0 {
            return;
        }
        match victim {
            Some(MeleeTarget::Session(id)) => {
                let player = &mut self.levels[id];
                player.set_delta_movement(player.delta_movement().plus(Vec3d { y: 0.1, ..push }));
                player.hurt_marked = true;
            }
            Some(MeleeTarget::Stored(id)) => {
                if let Some(entity) = self.level_mut_by_id(level).entities.get_mut(id) {
                    crate::entity::push(entity, push.x, 0.1, push.z);
                }
            }
            None => {}
        }
    }

    pub(in crate::world) fn projectile_effect_source(
        &self,
        dimension: LevelId,
        owner: Option<Uuid>,
        projectile_entity_id: i32,
    ) -> (LevelId, SourceEntity) {
        owner
            .and_then(|owner| {
                self.player_reference(owner)
                    .and_then(|id| {
                        self.levels
                            .player(id)
                            .map(|player| (player.dimension, SourceEntity::Session(id)))
                    })
                    .or_else(|| {
                        self.entity_reference(&dimension, owner)
                            .map(|(dimension, entity_id)| {
                                (dimension, SourceEntity::Stored(entity_id))
                            })
                    })
            })
            .unwrap_or((dimension, SourceEntity::Stored(projectile_entity_id)))
    }

    /// `HurtThenMobEffect` on a session: the levitation of a shulker bullet or
    /// the wither of a skull lands on players too.
    pub(in crate::world) fn apply_player_projectile_effect(
        &mut self,
        dimension: LevelId,
        target: &PendingHurtEntity,
        effect: &crate::entity::projectile_action::PendingProjectileMobEffect,
        events: &mut Vec<WorldEvent>,
    ) {
        let PendingHurtEntity::Player { uuid } = *target else {
            return;
        };
        let Some(id) = self.connection_of(uuid) else {
            return;
        };
        let duration = effect.duration_for(Some(self.difficulty()));
        if let Some(instance) = ActiveMobEffect::new(effect.resource_id, duration, effect.amplifier)
        {
            let source = self.projectile_effect_source(
                dimension,
                effect.source_owner,
                effect.projectile_entity_id,
            );
            self.add_player_mob_effect_from(id, instance, Some(source), events);
        }
    }
}

/// `deflect(REVERSE)` then `scale(0.2)`; a stalled arrow drops its pickup
/// 0.1 above itself and goes (`AbstractArrow.java:494-503`).
fn bounce(
    store: &mut crate::entity::store::EntityStore,
    arrow_entity_id: i32,
    pickup: Option<bbb_server_types::item_stack::RuntimeItemStack>,
) {
    let stalled = store.with_entity_mut(arrow_entity_id, |arrow, store| {
        crate::entity::abstract_arrow::reverse(arrow, store);
        arrow.set_delta_movement(arrow.delta_movement().scale(0.2));
        (arrow.delta_movement().length_sqr() < 1.0e-7).then(|| arrow.position())
    });
    let Some(Some(position)) = stalled else {
        return;
    };
    if let Some(item) = pickup {
        crate::entity::item::spawn_dropped_item_entity(
            store,
            Vec3d {
                x: position.x,
                y: position.y + 0.1,
                z: position.z,
            },
            item,
        );
    }
    store.remove(arrow_entity_id, crate::entity::RemovalReason::Discarded);
}
