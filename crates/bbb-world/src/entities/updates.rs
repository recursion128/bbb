use bbb_protocol::packets::{
    EntityAnimation as ProtocolEntityAnimation, EntityEvent as ProtocolEntityEvent,
    HurtAnimation as ProtocolHurtAnimation, RotateHead as ProtocolRotateHead,
    SetEntityLink as ProtocolSetEntityLink, SetEntityMotion as ProtocolSetEntityMotion,
};

use crate::WorldStore;

use super::movement::entity_vec3;

/// Vanilla `ClientboundAnimatePacket.SWING_MAIN_HAND` action id: the entity swings its main hand.
const SWING_MAIN_HAND_ACTION: u8 = 0;
/// Vanilla `ClientboundAnimatePacket.SWING_OFF_HAND` action id: the entity swings its off hand.
const SWING_OFF_HAND_ACTION: u8 = 3;

impl WorldStore {
    pub fn apply_entity_animation(&mut self, packet: ProtocolEntityAnimation) -> bool {
        self.counters.entity_animation_updates_received += 1;
        let Some(()) = self
            .entities
            .with_transient_events_mut(packet.id, |events| {
                events.last_animation_action = Some(packet.action)
            })
        else {
            self.counters.entity_animation_updates_ignored += 1;
            return false;
        };
        // Vanilla `ClientboundAnimatePacket`: action `0` swings the main hand, `3` the off hand
        // (`ClientPacketListener.handleAnimate` → `LivingEntity.swing`). Both arm the melee swing.
        if packet.action == SWING_MAIN_HAND_ACTION || packet.action == SWING_OFF_HAND_ACTION {
            let _ = self
                .entities
                .trigger_client_animation_swing(packet.id, packet.action == SWING_OFF_HAND_ACTION);
        }
        self.counters.entity_animation_updates_applied += 1;
        true
    }

    pub fn apply_entity_event(&mut self, packet: ProtocolEntityEvent) -> bool {
        self.counters.entity_events_received += 1;
        let Some(()) = self
            .entities
            .with_transient_events_mut(packet.entity_id, |events| {
                events.last_event_id = Some(packet.event_id)
            })
        else {
            self.counters.entity_events_ignored += 1;
            return false;
        };
        let _ = self
            .entities
            .apply_client_animation_entity_event(packet.entity_id, packet.event_id);
        self.counters.entity_events_applied += 1;
        true
    }

    pub fn apply_hurt_animation(&mut self, packet: ProtocolHurtAnimation) -> bool {
        self.counters.entity_hurt_animations_received += 1;
        let Some(()) = self
            .entities
            .with_transient_events_mut(packet.id, |events| events.last_hurt_yaw = Some(packet.yaw))
        else {
            self.counters.entity_hurt_animations_ignored += 1;
            return false;
        };
        // Vanilla `LivingEntity.animateHurt`: a hurt animation also drives the
        // hurtTime countdown behind the red damage overlay.
        let _ = self.entities.trigger_client_animation_hurt(packet.id);
        self.counters.entity_hurt_animations_applied += 1;
        true
    }

    pub fn apply_set_entity_link(&mut self, packet: ProtocolSetEntityLink) -> bool {
        self.counters.entity_link_updates_received += 1;
        let Some(()) = self.entities.with_leash_mut(packet.source_id, |leash| {
            leash.holder_id = if packet.dest_id == 0 {
                None
            } else {
                Some(packet.dest_id)
            };
        }) else {
            self.counters.entity_link_updates_ignored += 1;
            return false;
        };
        self.counters.entity_link_updates_applied += 1;
        true
    }

    pub fn apply_set_entity_motion(&mut self, packet: ProtocolSetEntityMotion) -> bool {
        self.counters.entity_motion_updates_received += 1;
        let Some(()) = self.entities.with_transform_mut(packet.id, |transform| {
            transform.delta_movement = entity_vec3(packet.delta_movement);
        }) else {
            self.counters.entity_motion_updates_ignored += 1;
            return false;
        };
        self.counters.entity_motion_updates_applied += 1;
        true
    }

    pub fn apply_rotate_head(&mut self, packet: ProtocolRotateHead) -> bool {
        self.counters.entity_head_rotations_received += 1;
        let Some(()) = self.entities.with_transform_mut(packet.id, |transform| {
            transform.y_head_rot = packet.y_head_rot;
        }) else {
            self.counters.entity_head_rotations_ignored += 1;
            return false;
        };
        self.counters.entity_head_rotations_applied += 1;
        true
    }
}
