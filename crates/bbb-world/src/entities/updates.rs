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
        let Some(entity) = self
            .entities
            .iter_mut()
            .find(|entity| entity.id == packet.id)
        else {
            return false;
        };

        entity.last_animation_action = Some(packet.action);
        self.counters.entity_animation_updates_applied += 1;
        true
    }

    pub fn apply_entity_event(&mut self, packet: ProtocolEntityEvent) -> bool {
        self.counters.entity_events_received += 1;
        let Some(entity) = self
            .entities
            .iter_mut()
            .find(|entity| entity.id == packet.entity_id)
        else {
            return false;
        };

        entity.last_event_id = Some(packet.event_id);
        self.counters.entity_events_applied += 1;
        true
    }

    pub fn apply_hurt_animation(&mut self, packet: ProtocolHurtAnimation) -> bool {
        self.counters.entity_hurt_animations_received += 1;
        let Some(entity) = self
            .entities
            .iter_mut()
            .find(|entity| entity.id == packet.id)
        else {
            return false;
        };

        entity.last_hurt_yaw = Some(packet.yaw);
        self.counters.entity_hurt_animations_applied += 1;
        true
    }

    pub fn apply_set_entity_link(&mut self, packet: ProtocolSetEntityLink) -> bool {
        self.counters.entity_link_updates_received += 1;
        let Some(entity) = self
            .entities
            .iter_mut()
            .find(|entity| entity.id == packet.source_id)
        else {
            return false;
        };

        entity.leash_holder_id = if packet.dest_id == 0 {
            None
        } else {
            Some(packet.dest_id)
        };
        self.counters.entity_link_updates_applied += 1;
        true
    }

    pub fn apply_set_entity_motion(&mut self, packet: ProtocolSetEntityMotion) -> bool {
        self.counters.entity_motion_updates_received += 1;
        let Some(entity) = self
            .entities
            .iter_mut()
            .find(|entity| entity.id == packet.id)
        else {
            return false;
        };

        entity.delta_movement = entity_vec3(packet.delta_movement);
        self.counters.entity_motion_updates_applied += 1;
        true
    }

    pub fn apply_rotate_head(&mut self, packet: ProtocolRotateHead) -> bool {
        self.counters.entity_head_rotations_received += 1;
        let Some(entity) = self
            .entities
            .iter_mut()
            .find(|entity| entity.id == packet.id)
        else {
            return false;
        };

        entity.y_head_rot = packet.y_head_rot;
        self.counters.entity_head_rotations_applied += 1;
        true
    }
}
