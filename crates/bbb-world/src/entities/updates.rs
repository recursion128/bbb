use bbb_protocol::packets::{
    EntityAnimation as ProtocolEntityAnimation, EntityEvent as ProtocolEntityEvent,
    HurtAnimation as ProtocolHurtAnimation, RotateHead as ProtocolRotateHead,
    SetEntityLink as ProtocolSetEntityLink, SetEntityMotion as ProtocolSetEntityMotion,
};

use crate::WorldStore;

use super::movement::entity_vec3;

impl WorldStore {
    pub fn apply_entity_animation(&mut self, packet: ProtocolEntityAnimation) -> bool {
        self.counters.entity_animation_updates_received += 1;
        let Some(()) = self.entities.with_mut(packet.id, |entity| {
            entity.last_animation_action = Some(packet.action)
        }) else {
            return false;
        };
        self.counters.entity_animation_updates_applied += 1;
        true
    }

    pub fn apply_entity_event(&mut self, packet: ProtocolEntityEvent) -> bool {
        self.counters.entity_events_received += 1;
        let Some(()) = self.entities.with_mut(packet.entity_id, |entity| {
            entity.last_event_id = Some(packet.event_id)
        }) else {
            return false;
        };
        self.counters.entity_events_applied += 1;
        true
    }

    pub fn apply_hurt_animation(&mut self, packet: ProtocolHurtAnimation) -> bool {
        self.counters.entity_hurt_animations_received += 1;
        let Some(()) = self
            .entities
            .with_mut(packet.id, |entity| entity.last_hurt_yaw = Some(packet.yaw))
        else {
            return false;
        };
        self.counters.entity_hurt_animations_applied += 1;
        true
    }

    pub fn apply_set_entity_link(&mut self, packet: ProtocolSetEntityLink) -> bool {
        self.counters.entity_link_updates_received += 1;
        let Some(()) = self.entities.with_mut(packet.source_id, |entity| {
            entity.leash_holder_id = if packet.dest_id == 0 {
                None
            } else {
                Some(packet.dest_id)
            };
        }) else {
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
            return false;
        };
        self.counters.entity_head_rotations_applied += 1;
        true
    }
}
