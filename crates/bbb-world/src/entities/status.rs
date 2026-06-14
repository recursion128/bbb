use std::collections::BTreeMap;

use bbb_protocol::packets::{
    Cooldown as ProtocolCooldown, DamageEvent as ProtocolDamageEvent,
    RemoveMobEffect as ProtocolRemoveMobEffect, UpdateMobEffect as ProtocolUpdateMobEffect,
    Vec3d as ProtocolVec3d,
};
use serde::{Deserialize, Serialize};

use crate::WorldStore;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ItemCooldownState {
    pub cooldown_group: String,
    pub duration: i32,
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
}

impl From<ProtocolUpdateMobEffect> for MobEffectState {
    fn from(packet: ProtocolUpdateMobEffect) -> Self {
        Self {
            effect_id: packet.effect_id,
            amplifier: packet.amplifier,
            duration_ticks: packet.duration_ticks,
            ambient: packet.flags.ambient,
            visible: packet.flags.visible,
            show_icon: packet.flags.show_icon,
            blend: packet.flags.blend,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct EntityDamageEventState {
    pub source_type_id: i32,
    pub source_cause_id: i32,
    pub source_direct_id: i32,
    pub source_position: Option<ProtocolVec3d>,
}

impl From<ProtocolDamageEvent> for EntityDamageEventState {
    fn from(packet: ProtocolDamageEvent) -> Self {
        Self {
            source_type_id: packet.source_type_id,
            source_cause_id: packet.source_cause_id,
            source_direct_id: packet.source_direct_id,
            source_position: packet.source_position,
        }
    }
}

impl WorldStore {
    pub fn apply_cooldown(&mut self, packet: ProtocolCooldown) {
        self.counters.cooldown_packets += 1;
        if packet.duration <= 0 {
            self.cooldowns.remove(&packet.cooldown_group);
        } else {
            self.cooldowns.insert(
                packet.cooldown_group.clone(),
                ItemCooldownState {
                    cooldown_group: packet.cooldown_group,
                    duration: packet.duration,
                },
            );
        }
        self.update_cooldown_count();
    }

    pub fn apply_update_mob_effect(&mut self, packet: ProtocolUpdateMobEffect) -> bool {
        self.counters.update_mob_effect_packets += 1;
        let Some(()) = self.entities.with_mut(packet.entity_id, |entity| {
            entity
                .mob_effects
                .insert(packet.effect_id, MobEffectState::from(packet));
        }) else {
            return false;
        };
        self.update_active_mob_effect_count();
        true
    }

    pub fn apply_remove_mob_effect(&mut self, packet: ProtocolRemoveMobEffect) -> bool {
        self.counters.remove_mob_effect_packets += 1;
        let Some(removed) = self.entities.with_mut(packet.entity_id, |entity| {
            entity.mob_effects.remove(&packet.effect_id).is_some()
        }) else {
            return false;
        };
        self.update_active_mob_effect_count();
        removed
    }

    pub fn apply_damage_event(&mut self, packet: ProtocolDamageEvent) -> bool {
        self.counters.damage_event_packets += 1;
        let Some(()) = self.entities.with_mut(packet.entity_id, |entity| {
            entity.last_damage = Some(EntityDamageEventState::from(packet));
        }) else {
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

    pub fn entity_effects(&self, entity_id: i32) -> Option<&BTreeMap<i32, MobEffectState>> {
        self.probe_entity(entity_id)
            .map(|entity| &entity.mob_effects)
    }

    pub fn entity_effect(&self, entity_id: i32, effect_id: i32) -> Option<&MobEffectState> {
        self.probe_entity(entity_id)?.mob_effects.get(&effect_id)
    }

    pub fn entity_last_damage(&self, entity_id: i32) -> Option<&EntityDamageEventState> {
        self.probe_entity(entity_id)
            .and_then(|entity| entity.last_damage.as_ref())
    }

    fn update_cooldown_count(&mut self) {
        self.counters.cooldowns_tracked = self.cooldowns.len();
    }

    pub(crate) fn update_active_mob_effect_count(&mut self) {
        self.counters.active_mob_effects_tracked = self
            .entities
            .iter()
            .map(|entity| entity.mob_effects.len())
            .sum();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bbb_protocol::packets::{
        AddEntity as ProtocolAddEntity, Cooldown as ProtocolCooldown,
        DamageEvent as ProtocolDamageEvent, MobEffectFlags, RemoveMobEffect, UpdateMobEffect,
        Vec3d as ProtocolVec3d,
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
    fn mob_effects_upsert_remove_and_ignore_unknown_entities() {
        let mut store = WorldStore::new();
        store.apply_add_entity(protocol_add_entity(7));

        let updated = store.apply_update_mob_effect(UpdateMobEffect {
            entity_id: 7,
            effect_id: 3,
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
        let effect = store.entity_effect(7, 3).unwrap();
        assert_eq!(effect.amplifier, 2);
        assert_eq!(effect.duration_ticks, 400);
        assert!(effect.ambient);
        assert!(effect.visible);
        assert!(!effect.show_icon);
        assert!(effect.blend);
        assert_eq!(store.counters().update_mob_effect_packets, 1);
        assert_eq!(store.counters().active_mob_effects_tracked, 1);

        assert!(!store.apply_update_mob_effect(UpdateMobEffect {
            entity_id: 99,
            effect_id: 4,
            amplifier: 0,
            duration_ticks: 100,
            flags: MobEffectFlags::default(),
        }));
        assert_eq!(store.counters().update_mob_effect_packets, 2);
        assert_eq!(store.counters().active_mob_effects_tracked, 1);

        assert!(store.apply_remove_mob_effect(RemoveMobEffect {
            entity_id: 7,
            effect_id: 3,
        }));
        assert!(store.entity_effect(7, 3).is_none());
        assert_eq!(store.counters().remove_mob_effect_packets, 1);
        assert_eq!(store.counters().active_mob_effects_tracked, 0);
    }

    #[test]
    fn damage_events_store_on_known_entities_only() {
        let mut store = WorldStore::new();
        store.apply_add_entity(protocol_add_entity(7));

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
        assert_eq!(damage.source_cause_id, -1);
        assert_eq!(damage.source_direct_id, 42);
        assert_eq!(
            damage.source_position,
            Some(ProtocolVec3d {
                x: 1.0,
                y: 2.0,
                z: 3.0,
            })
        );
        assert_eq!(store.counters().damage_event_packets, 1);
        assert_eq!(store.counters().damage_events_applied, 1);

        assert!(!store.apply_damage_event(ProtocolDamageEvent {
            entity_id: 99,
            source_type_id: 5,
            source_cause_id: -1,
            source_direct_id: -1,
            source_position: None,
        }));
        assert_eq!(store.counters().damage_event_packets, 2);
        assert_eq!(store.counters().damage_events_applied, 1);
    }

    fn protocol_add_entity(id: i32) -> ProtocolAddEntity {
        ProtocolAddEntity {
            id,
            uuid: Uuid::from_u128(0x12345678123456781234567812345678),
            entity_type_id: 7,
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
