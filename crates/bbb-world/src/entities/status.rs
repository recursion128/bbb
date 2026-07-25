use std::collections::BTreeMap;

use bbb_protocol::mob_effect_types::vanilla_mob_effect_type;
use bbb_protocol::packets::{
    Cooldown as ProtocolCooldown, DamageEvent as ProtocolDamageEvent,
    RemoveMobEffect as ProtocolRemoveMobEffect, UpdateMobEffect as ProtocolUpdateMobEffect,
    Vec3d as ProtocolVec3d,
};
use serde::{Deserialize, Serialize};

use crate::{DamageTypeEffects, WorldStore, DAMAGE_TYPE_REGISTRY};

use super::dimensions::vanilla_living_entity_type;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ItemCooldownState {
    pub cooldown_group: String,
    pub duration: i32,
    #[serde(default)]
    pub remaining_ticks: i32,
}

impl ItemCooldownState {
    pub fn percent(&self, partial_tick: f32) -> f32 {
        if self.duration == 0 {
            return 0.0;
        }
        let remaining = self.remaining_ticks as f32 - partial_tick.clamp(0.0, 1.0);
        (remaining / self.duration as f32).clamp(0.0, 1.0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct MobEffectState {
    pub effect_id: i32,
    pub amplifier: i32,
    pub duration_ticks: i32,
    pub ambient: bool,
    pub visible: bool,
    pub show_icon: bool,
    pub blend: bool,
    #[serde(default)]
    blend_factor_bits: u32,
    #[serde(default)]
    blend_factor_previous_frame_bits: u32,
}

impl From<ProtocolUpdateMobEffect> for MobEffectState {
    fn from(packet: ProtocolUpdateMobEffect) -> Self {
        let descriptor = vanilla_mob_effect_type(packet.effect_id)
            .expect("protocol validates mob-effect holder ids before world apply");
        let immediate_factor = if packet.flags.blend
            || mob_effect_ends_within(packet.duration_ticks, descriptor.blend_out_advance_ticks)
        {
            0.0_f32
        } else {
            1.0_f32
        };
        Self {
            effect_id: packet.effect_id,
            amplifier: packet.amplifier,
            duration_ticks: packet.duration_ticks,
            ambient: packet.flags.ambient,
            visible: packet.flags.visible,
            show_icon: packet.flags.show_icon,
            blend: packet.flags.blend,
            blend_factor_bits: immediate_factor.to_bits(),
            blend_factor_previous_frame_bits: immediate_factor.to_bits(),
        }
    }
}

impl MobEffectState {
    pub fn blend_factor(self, partial_tick: f32) -> f32 {
        let partial_tick = if partial_tick.is_finite() {
            partial_tick.clamp(0.0, 1.0)
        } else {
            0.0
        };
        let previous = f32::from_bits(self.blend_factor_previous_frame_bits);
        let current = f32::from_bits(self.blend_factor_bits);
        previous + (current - previous) * partial_tick
    }

    pub fn ends_within(self, ticks: i32) -> bool {
        mob_effect_ends_within(self.duration_ticks, ticks)
    }

    pub(crate) fn copy_blend_state_from(&mut self, previous: Self) {
        self.blend_factor_bits = previous.blend_factor_bits;
        self.blend_factor_previous_frame_bits = previous.blend_factor_previous_frame_bits;
    }

    pub(crate) fn advance_client_tick(&mut self) {
        if self.duration_ticks > 0 {
            self.duration_ticks -= 1;
        }

        let Some(descriptor) = vanilla_mob_effect_type(self.effect_id) else {
            return;
        };
        let current = f32::from_bits(self.blend_factor_bits);
        self.blend_factor_previous_frame_bits = current.to_bits();
        let active =
            !mob_effect_ends_within(self.duration_ticks, descriptor.blend_out_advance_ticks);
        let target = if active { 1.0 } else { 0.0 };
        let blend_duration = if active {
            descriptor.blend_in_duration_ticks
        } else {
            descriptor.blend_out_duration_ticks
        };
        let factor = if current == target {
            current
        } else if blend_duration == 0 {
            target
        } else {
            let max_delta = 1.0 / blend_duration as f32;
            current + (target - current).clamp(-max_delta, max_delta)
        };
        self.blend_factor_bits = factor.to_bits();
    }
}

fn mob_effect_ends_within(duration_ticks: i32, ticks: i32) -> bool {
    duration_ticks != -1 && duration_ticks <= ticks
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EntityDamageEventState {
    pub source_type_id: i32,
    #[serde(default)]
    pub source_type_resource_id: Option<String>,
    #[serde(default)]
    pub source_effects: Option<DamageTypeEffects>,
    pub source_cause_id: i32,
    pub source_direct_id: i32,
    pub source_position: Option<ProtocolVec3d>,
    /// Vanilla `LivingEntity.lastDamageStamp`, sampled from level game time
    /// after handling this source.
    #[serde(default)]
    pub last_damage_stamp: i64,
}

impl EntityDamageEventState {
    fn from_packet(
        packet: ProtocolDamageEvent,
        source_type_resource_id: Option<String>,
        source_effects: Option<DamageTypeEffects>,
        last_damage_stamp: i64,
    ) -> Self {
        Self {
            source_type_id: packet.source_type_id,
            source_type_resource_id,
            source_effects,
            source_cause_id: packet.source_cause_id,
            source_direct_id: packet.source_direct_id,
            source_position: packet.source_position,
            last_damage_stamp,
        }
    }
}

impl WorldStore {
    pub fn apply_cooldown(&mut self, packet: ProtocolCooldown) {
        self.counters.cooldown_packets += 1;
        if packet.duration == 0 {
            self.cooldowns.remove(&packet.cooldown_group);
        } else {
            self.cooldowns.insert(
                packet.cooldown_group.clone(),
                ItemCooldownState {
                    cooldown_group: packet.cooldown_group,
                    duration: packet.duration,
                    remaining_ticks: packet.duration,
                },
            );
        }
        self.update_cooldown_count();
    }

    pub(crate) fn add_local_item_cooldown(&mut self, cooldown_group: String, duration: i32) {
        if duration <= 0 {
            self.cooldowns.remove(&cooldown_group);
        } else {
            self.cooldowns.insert(
                cooldown_group.clone(),
                ItemCooldownState {
                    cooldown_group,
                    duration,
                    remaining_ticks: duration,
                },
            );
        }
        self.update_cooldown_count();
    }

    pub fn advance_item_cooldowns(&mut self, ticks: u32) -> usize {
        if ticks == 0 || self.cooldowns.is_empty() {
            return 0;
        }
        let ticks = i32::try_from(ticks).unwrap_or(i32::MAX);
        for cooldown in self.cooldowns.values_mut() {
            cooldown.remaining_ticks = cooldown.remaining_ticks.saturating_sub(ticks);
        }
        let before = self.cooldowns.len();
        self.cooldowns
            .retain(|_, cooldown| cooldown.remaining_ticks > 0 && cooldown.duration > 0);
        let removed = before.saturating_sub(self.cooldowns.len());
        self.update_cooldown_count();
        removed
    }

    pub fn apply_update_mob_effect(&mut self, packet: ProtocolUpdateMobEffect) -> bool {
        self.counters.update_mob_effect_packets += 1;
        let entity_id = packet.entity_id;
        let Some(entity_type_id) = self.entities.entity_type_id(entity_id) else {
            self.counters.update_mob_effects_ignored += 1;
            return false;
        };
        if !vanilla_living_entity_type(entity_type_id) {
            self.counters.update_mob_effects_ignored += 1;
            return false;
        }
        let mut effect = MobEffectState::from(packet);
        let Some(()) = self.entities.with_mob_effects_mut(entity_id, |effects| {
            if let Some(previous) = effects.effects.get(&effect.effect_id).copied() {
                effect.copy_blend_state_from(previous);
            }
            effects.effects.insert(effect.effect_id, effect);
        }) else {
            self.counters.update_mob_effects_ignored += 1;
            return false;
        };
        self.update_active_mob_effect_count();
        self.refresh_entity_active_swing_duration(entity_id);
        true
    }

    pub fn apply_remove_mob_effect(&mut self, packet: ProtocolRemoveMobEffect) -> bool {
        self.counters.remove_mob_effect_packets += 1;
        let Some(entity_type_id) = self.entities.entity_type_id(packet.entity_id) else {
            self.counters.remove_mob_effects_ignored += 1;
            return false;
        };
        if !vanilla_living_entity_type(entity_type_id) {
            self.counters.remove_mob_effects_ignored += 1;
            return false;
        }
        let Some(removed) = self
            .entities
            .with_mob_effects_mut(packet.entity_id, |effects| {
                effects.effects.remove(&packet.effect_id).is_some()
            })
        else {
            self.counters.remove_mob_effects_ignored += 1;
            return false;
        };
        self.update_active_mob_effect_count();
        if removed {
            self.refresh_entity_active_swing_duration(packet.entity_id);
        } else {
            self.counters.remove_mob_effects_ignored += 1;
        }
        removed
    }

    pub fn apply_damage_event(&mut self, packet: ProtocolDamageEvent) -> bool {
        self.counters.damage_event_packets += 1;
        let entity_id = packet.entity_id;
        let Some(entity_type_id) = self.entities.entity_type_id(entity_id) else {
            self.counters.damage_events_ignored += 1;
            return false;
        };
        if !vanilla_living_entity_type(entity_type_id) {
            self.counters.damage_events_ignored += 1;
            return false;
        }
        let source_type_resource_id = self
            .registry_resource_id_by_holder_id(DAMAGE_TYPE_REGISTRY, packet.source_type_id)
            .map(str::to_string);
        let source_effects = match self.damage_type_effects_by_holder_id(packet.source_type_id) {
            Ok(effects) => effects,
            Err(err) => {
                self.record_apply_error("damage_type", err);
                None
            }
        };
        let last_damage_stamp = self.world_time().map_or(0, |time| time.game_time);
        let damage_event = EntityDamageEventState::from_packet(
            packet,
            source_type_resource_id,
            source_effects,
            last_damage_stamp,
        );
        let Some(()) = self
            .entities
            .apply_living_damage_event_presentation(entity_id, damage_event)
        else {
            self.counters.damage_events_ignored += 1;
            return false;
        };
        self.counters.damage_events_applied += 1;
        true
    }

    pub fn cooldowns(&self) -> &BTreeMap<String, ItemCooldownState> {
        &self.cooldowns
    }

    pub fn cooldown(&self, cooldown_group: &str) -> Option<&ItemCooldownState> {
        self.cooldowns.get(cooldown_group)
    }

    pub fn item_cooldown_percent(&self, cooldown_group: &str, partial_tick: f32) -> f32 {
        self.cooldown(cooldown_group)
            .map(|cooldown| cooldown.percent(partial_tick))
            .unwrap_or(0.0)
    }

    pub fn entity_effects(&self, entity_id: i32) -> Option<BTreeMap<i32, MobEffectState>> {
        self.entities
            .mob_effects(entity_id)
            .map(|effects| effects.effects.into_value())
    }

    pub fn entity_effect(&self, entity_id: i32, effect_id: i32) -> Option<MobEffectState> {
        self.entities
            .mob_effects(entity_id)?
            .effects
            .get(&effect_id)
            .copied()
    }

    pub fn entity_last_damage(&self, entity_id: i32) -> Option<EntityDamageEventState> {
        let damage = self.entities.damage(entity_id)?.last_damage.into_value()?;
        let game_time = self.world_time().map_or(0, |time| time.game_time);
        (game_time.saturating_sub(damage.last_damage_stamp) <= 40).then_some(damage)
    }

    pub fn entity_damage_invulnerable_time(&self, entity_id: i32) -> Option<i32> {
        self.entities
            .damage(entity_id)
            .map(|damage| damage.invulnerable_time)
    }

    fn update_cooldown_count(&mut self) {
        self.counters.cooldowns_tracked = self.cooldowns.len();
    }

    pub(crate) fn update_active_mob_effect_count(&mut self) {
        self.counters.active_mob_effects_tracked = self.entities.total_mob_effects();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::RegistryPacketEntry;
    use bbb_protocol::mob_effect_types::{
        VANILLA_MOB_EFFECT_DARKNESS_ID, VANILLA_MOB_EFFECT_MINING_FATIGUE_ID,
        VANILLA_MOB_EFFECT_STRENGTH_ID,
    };
    use bbb_protocol::packets::{
        AddEntity as ProtocolAddEntity, Cooldown as ProtocolCooldown,
        DamageEvent as ProtocolDamageEvent, MobEffectFlags, PlayTime as ProtocolPlayTime,
        RemoveMobEffect, UpdateMobEffect, Vec3d as ProtocolVec3d,
    };
    use uuid::Uuid;

    #[test]
    fn cooldown_set_update_and_clear_tracks_counts() {
        let mut store = WorldStore::new();
        store.apply_cooldown(ProtocolCooldown {
            cooldown_group: "minecraft:ender_pearl".to_string(),
            duration: 20,
        });

        let cooldown = store.cooldown("minecraft:ender_pearl").unwrap();
        assert_eq!(cooldown.duration, 20);
        assert_eq!(cooldown.remaining_ticks, 20);
        assert_eq!(
            store.item_cooldown_percent("minecraft:ender_pearl", 0.0),
            1.0
        );
        assert_eq!(store.cooldowns().len(), 1);
        assert_eq!(store.counters().cooldown_packets, 1);
        assert_eq!(store.counters().cooldowns_tracked, 1);

        store.apply_cooldown(ProtocolCooldown {
            cooldown_group: "minecraft:ender_pearl".to_string(),
            duration: 0,
        });

        assert!(store.cooldown("minecraft:ender_pearl").is_none());
        assert_eq!(store.counters().cooldown_packets, 2);
        assert_eq!(store.counters().cooldowns_tracked, 0);
    }

    #[test]
    fn cooldowns_advance_with_client_ticks_and_report_vanilla_percent() {
        let mut store = WorldStore::new();
        store.apply_cooldown(ProtocolCooldown {
            cooldown_group: "minecraft:ender_pearl".to_string(),
            duration: 20,
        });

        assert_eq!(store.advance_item_cooldowns(5), 0);
        let cooldown = store.cooldown("minecraft:ender_pearl").unwrap();
        assert_eq!(cooldown.remaining_ticks, 15);
        assert_eq!(cooldown.percent(0.0), 0.75);
        assert_eq!(cooldown.percent(0.5), 0.725);
        assert_eq!(
            store.item_cooldown_percent("minecraft:ender_pearl", 1.0),
            0.7
        );
        assert_eq!(store.counters().cooldowns_tracked, 1);

        assert_eq!(store.advance_item_cooldowns(14), 0);
        assert_eq!(
            store.item_cooldown_percent("minecraft:ender_pearl", 0.0),
            0.05
        );
        assert_eq!(store.advance_item_cooldowns(1), 1);
        assert!(store.cooldown("minecraft:ender_pearl").is_none());
        assert_eq!(
            store.item_cooldown_percent("minecraft:ender_pearl", 0.0),
            0.0
        );
        assert_eq!(store.counters().cooldowns_tracked, 0);
    }

    #[test]
    fn negative_cooldown_is_added_then_expires_on_the_next_tick() {
        let mut store = WorldStore::new();
        store.apply_cooldown(ProtocolCooldown {
            cooldown_group: "minecraft:test".to_string(),
            duration: -2,
        });

        let cooldown = store.cooldown("minecraft:test").unwrap();
        assert_eq!(cooldown.duration, -2);
        assert_eq!(cooldown.remaining_ticks, -2);
        assert_eq!(cooldown.percent(0.0), 1.0);
        assert_eq!(store.advance_item_cooldowns(1), 1);
        assert!(store.cooldown("minecraft:test").is_none());
    }

    #[test]
    fn mob_effects_upsert_remove_and_ignore_unknown_entities() {
        let mut store = WorldStore::new();
        store.apply_add_entity(protocol_add_entity(7));
        store.apply_add_entity(protocol_add_entity_with_type(
            8,
            super::super::VANILLA_ENTITY_TYPE_ITEM_ID,
        ));

        let updated = store.apply_update_mob_effect(UpdateMobEffect {
            entity_id: 7,
            effect_id: VANILLA_MOB_EFFECT_MINING_FATIGUE_ID,
            amplifier: 2,
            duration_ticks: 400,
            flags: MobEffectFlags {
                raw: 0b1011,
                ambient: true,
                visible: true,
                show_icon: false,
                blend: true,
            },
        });

        assert!(updated);
        let effect = store
            .entity_effect(7, VANILLA_MOB_EFFECT_MINING_FATIGUE_ID)
            .unwrap();
        assert_eq!(effect.amplifier, 2);
        assert_eq!(effect.duration_ticks, 400);
        assert!(effect.ambient);
        assert!(effect.visible);
        assert!(!effect.show_icon);
        assert!(effect.blend);
        assert_eq!(
            store
                .entities
                .mob_effects(7)
                .unwrap()
                .effects
                .get(&VANILLA_MOB_EFFECT_MINING_FATIGUE_ID),
            Some(&effect)
        );
        assert_eq!(store.counters().update_mob_effect_packets, 1);
        assert_eq!(store.counters().active_mob_effects_tracked, 1);

        assert!(!store.apply_update_mob_effect(UpdateMobEffect {
            entity_id: 99,
            effect_id: VANILLA_MOB_EFFECT_STRENGTH_ID,
            amplifier: 0,
            duration_ticks: 100,
            flags: MobEffectFlags::default(),
        }));
        assert_eq!(store.counters().update_mob_effect_packets, 2);
        assert_eq!(store.counters().update_mob_effects_ignored, 1);
        assert_eq!(store.counters().active_mob_effects_tracked, 1);

        assert!(!store.apply_update_mob_effect(UpdateMobEffect {
            entity_id: 8,
            effect_id: VANILLA_MOB_EFFECT_STRENGTH_ID,
            amplifier: 0,
            duration_ticks: 100,
            flags: MobEffectFlags::default(),
        }));
        assert!(store.entity_effects(8).unwrap().is_empty());
        assert_eq!(store.counters().update_mob_effect_packets, 3);
        assert_eq!(store.counters().update_mob_effects_ignored, 2);
        assert_eq!(store.counters().active_mob_effects_tracked, 1);

        assert!(store.apply_remove_mob_effect(RemoveMobEffect {
            entity_id: 7,
            effect_id: VANILLA_MOB_EFFECT_MINING_FATIGUE_ID,
        }));
        assert!(store
            .entity_effect(7, VANILLA_MOB_EFFECT_MINING_FATIGUE_ID)
            .is_none());
        assert!(store.entities.mob_effects(7).unwrap().effects.is_empty());
        assert_eq!(store.counters().remove_mob_effect_packets, 1);
        assert_eq!(store.counters().active_mob_effects_tracked, 0);

        assert!(!store.apply_remove_mob_effect(RemoveMobEffect {
            entity_id: 8,
            effect_id: VANILLA_MOB_EFFECT_STRENGTH_ID,
        }));
        assert_eq!(store.counters().remove_mob_effect_packets, 2);
        assert_eq!(store.counters().remove_mob_effects_ignored, 1);
        assert_eq!(store.counters().active_mob_effects_tracked, 0);
    }

    #[test]
    fn mob_effect_client_ticks_preserve_vanilla_duration_and_blend_lifecycle() {
        let packet = |duration_ticks, blend| UpdateMobEffect {
            entity_id: 7,
            effect_id: VANILLA_MOB_EFFECT_DARKNESS_ID,
            amplifier: 0,
            duration_ticks,
            flags: MobEffectFlags {
                raw: if blend { 0b1000 } else { 0 },
                blend,
                ..MobEffectFlags::default()
            },
        };

        let mut blending = MobEffectState::from(packet(100, true));
        assert_eq!(blending.blend_factor(0.0), 0.0);
        blending.advance_client_tick();
        assert_eq!(blending.duration_ticks, 99);
        assert!((blending.blend_factor(1.0) - 1.0 / 22.0).abs() < 1.0e-6);
        assert_eq!(blending.blend_factor(0.0), 0.0);
        for _ in 1..22 {
            blending.advance_client_tick();
        }
        assert!((blending.blend_factor(1.0) - 1.0).abs() < 1.0e-6);

        let mut replacement = MobEffectState::from(packet(200, false));
        assert_eq!(replacement.blend_factor(1.0), 1.0);
        replacement.copy_blend_state_from(blending);
        assert_eq!(
            replacement.blend_factor(0.5),
            blending.blend_factor(0.5),
            "forceAddEffect copies the old instance blend state over the packet's skip flag"
        );

        let mut ending = MobEffectState::from(packet(23, false));
        assert_eq!(ending.blend_factor(1.0), 1.0);
        ending.advance_client_tick();
        assert_eq!(ending.duration_ticks, 22);
        assert!((ending.blend_factor(1.0) - 21.0 / 22.0).abs() < 1.0e-6);

        let mut infinite = MobEffectState::from(packet(-1, false));
        infinite.advance_client_tick();
        assert_eq!(infinite.duration_ticks, -1);
        assert_eq!(infinite.blend_factor(1.0), 1.0);
    }

    #[test]
    fn scheduled_living_entity_ticks_effects_without_removing_zero_duration_entries() {
        let mut store = WorldStore::new();
        store.apply_add_entity(protocol_add_entity(7));
        assert!(store.apply_update_mob_effect(UpdateMobEffect {
            entity_id: 7,
            effect_id: VANILLA_MOB_EFFECT_STRENGTH_ID,
            amplifier: 0,
            duration_ticks: 1,
            flags: MobEffectFlags::default(),
        }));

        store.advance_entity_client_animations(1);

        let effect = store
            .entity_effect(7, VANILLA_MOB_EFFECT_STRENGTH_ID)
            .unwrap();
        assert_eq!(effect.duration_ticks, 0);
        assert!(
            store
                .entity_effects(7)
                .unwrap()
                .contains_key(&VANILLA_MOB_EFFECT_STRENGTH_ID),
            "tickClient leaves expired instances until the server remove packet"
        );
    }

    #[test]
    fn damage_events_store_on_known_entities_only() {
        let mut store = WorldStore::new();
        store.apply_world_time(ProtocolPlayTime {
            game_time: 100,
            clock_updates: Vec::new(),
        });
        store.apply_add_entity(protocol_add_entity(7));
        store.apply_add_entity(protocol_add_entity_with_type(
            8,
            super::super::VANILLA_ENTITY_TYPE_ITEM_ID,
        ));

        assert!(store.apply_damage_event(ProtocolDamageEvent {
            entity_id: 7,
            source_type_id: 5,
            source_cause_id: -1,
            source_direct_id: 42,
            source_position: Some(ProtocolVec3d {
                x: 1.0,
                y: 2.0,
                z: 3.0,
            }),
        }));

        let damage = store.entity_last_damage(7).unwrap();
        assert_eq!(damage.source_type_id, 5);
        assert_eq!(
            damage.source_type_resource_id.as_deref(),
            Some("minecraft:hot_floor")
        );
        assert_eq!(damage.source_effects, Some(DamageTypeEffects::Burning));
        assert_eq!(damage.source_cause_id, -1);
        assert_eq!(damage.source_direct_id, 42);
        assert_eq!(damage.last_damage_stamp, 100);
        assert_eq!(
            damage.source_position,
            Some(ProtocolVec3d {
                x: 1.0,
                y: 2.0,
                z: 3.0,
            })
        );
        assert_eq!(
            store.entities.damage(7).unwrap().last_damage.into_value(),
            Some(damage.clone())
        );
        assert_eq!(store.counters().damage_event_packets, 1);
        assert_eq!(store.counters().damage_events_applied, 1);
        assert_eq!(store.entity_damage_invulnerable_time(7), Some(20));
        let animations = store.probe_entity(7).unwrap().client_animations;
        let walk = animations.walk_animation.unwrap();
        assert_eq!(walk.speed_old, 0.0);
        assert_eq!(walk.speed, 1.5);
        assert_eq!(walk.position, 0.0);
        assert!(animations.has_red_overlay());

        assert!(!store.apply_damage_event(ProtocolDamageEvent {
            entity_id: 8,
            source_type_id: 5,
            source_cause_id: -1,
            source_direct_id: -1,
            source_position: None,
        }));
        assert_eq!(store.entity_damage_invulnerable_time(8), Some(0));
        assert_eq!(store.probe_entity(8).unwrap().last_damage, None);

        assert!(!store.apply_damage_event(ProtocolDamageEvent {
            entity_id: 99,
            source_type_id: 5,
            source_cause_id: -1,
            source_direct_id: -1,
            source_position: None,
        }));
        assert_eq!(store.counters().damage_event_packets, 3);
        assert_eq!(store.counters().damage_events_applied, 1);
        assert_eq!(store.counters().damage_events_ignored, 2);
    }

    #[test]
    fn damage_event_timers_last_source_expiry_and_recreation_are_canonical() {
        let mut store = WorldStore::new();
        store.apply_world_time(ProtocolPlayTime {
            game_time: 200,
            clock_updates: Vec::new(),
        });
        store.apply_add_entity(protocol_add_entity(27));
        assert!(store.apply_damage_event(ProtocolDamageEvent {
            entity_id: 27,
            source_type_id: 0,
            source_cause_id: -1,
            source_direct_id: -1,
            source_position: None,
        }));
        let retained = store.publication_snapshot();

        store.advance_entity_client_animations(1);
        assert_eq!(store.entity_damage_invulnerable_time(27), Some(19));
        assert_eq!(
            retained.entity_damage_invulnerable_time(27),
            Some(20),
            "retained publication keeps the pre-tick component"
        );
        assert_eq!(
            retained
                .probe_entity(27)
                .unwrap()
                .client_animations
                .walk_animation
                .unwrap()
                .speed,
            1.5
        );

        store.apply_world_time(ProtocolPlayTime {
            game_time: 240,
            clock_updates: Vec::new(),
        });
        assert!(store.entity_last_damage(27).is_some());
        store.apply_world_time(ProtocolPlayTime {
            game_time: 241,
            clock_updates: Vec::new(),
        });
        assert!(store.entity_last_damage(27).is_none());
        assert!(store.probe_entity_status(27).unwrap().last_damage.is_none());
        assert!(
            store.probe_entity(27).unwrap().last_damage.is_some(),
            "Vanilla expires the getter without eagerly clearing the field"
        );

        store.apply_add_entity(protocol_add_entity(27));
        let recreated = store.probe_entity(27).unwrap();
        assert_eq!(recreated.last_damage, None);
        assert_eq!(recreated.damage_invulnerable_time, 0);
        assert!(recreated.client_animations.walk_animation.is_none());
        assert!(retained.entity_last_damage(27).is_some());
        assert_eq!(retained.entity_damage_invulnerable_time(27), Some(20));
    }

    #[test]
    fn damage_event_keeps_identity_when_custom_projection_is_invalid() {
        let mut store = WorldStore::new();
        store.record_registry_entries(
            DAMAGE_TYPE_REGISTRY,
            0,
            vec![RegistryPacketEntry::with_raw_data(
                "example:opaque",
                vec![10, 0],
            )],
        );
        store.apply_add_entity(protocol_add_entity(17));

        assert!(store.apply_damage_event(ProtocolDamageEvent {
            entity_id: 17,
            source_type_id: 0,
            source_cause_id: -1,
            source_direct_id: -1,
            source_position: None,
        }));
        let damage = store.entity_last_damage(17).unwrap();
        assert_eq!(
            damage.source_type_resource_id.as_deref(),
            Some("example:opaque")
        );
        assert_eq!(damage.source_effects, None);
        assert_eq!(store.counters().world_apply_errors, 1);
        let error = store.apply_diagnostics().last_apply_error.as_ref().unwrap();
        assert_eq!(error.source, "damage_type");
        assert_eq!(error.message, "message_id: missing required field");
    }

    fn protocol_add_entity(id: i32) -> ProtocolAddEntity {
        protocol_add_entity_with_type(id, 7)
    }

    fn protocol_add_entity_with_type(id: i32, entity_type_id: i32) -> ProtocolAddEntity {
        ProtocolAddEntity {
            id,
            uuid: Uuid::from_u128(0x12345678123456781234567812345678),
            entity_type_id,
            position: ProtocolVec3d {
                x: 1.0,
                y: 64.0,
                z: -2.0,
            },
            delta_movement: ProtocolVec3d {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            },
            x_rot: -10.0,
            y_rot: 20.0,
            y_head_rot: 30.0,
            data: 99,
        }
    }
}
