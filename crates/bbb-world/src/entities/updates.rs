use bbb_protocol::packets::{
    EntityAnimation as ProtocolEntityAnimation, EntityEvent as ProtocolEntityEvent,
    HurtAnimation as ProtocolHurtAnimation, RotateHead as ProtocolRotateHead,
    SetEntityLink as ProtocolSetEntityLink, SetEntityMotion as ProtocolSetEntityMotion,
};

use crate::WorldStore;

use super::movement::entity_vec3;
use super::schema::{
    vanilla_entity_metadata_type_is_leashable, vanilla_entity_metadata_type_is_living,
    vanilla_entity_metadata_type_uses_player_entity_event_handler,
};
use super::{vanilla_entity_event_kind, VanillaEntityEventKind};

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
            if self
                .entities
                .entity_type_id(packet.id)
                .is_some_and(vanilla_entity_metadata_type_is_living)
            {
                let off_hand = packet.action == SWING_OFF_HAND_ACTION;
                let duration = self.entity_held_item_swing_duration(packet.id, off_hand);
                let _ = self
                    .entities
                    .trigger_client_animation_swing(packet.id, off_hand, duration);
            }
        }
        self.counters.entity_animation_updates_applied += 1;
        true
    }

    pub fn apply_entity_event(&mut self, packet: ProtocolEntityEvent) -> bool {
        self.counters.entity_events_received += 1;
        let permission_applied =
            self.apply_local_player_permission_entity_event(packet.entity_id, packet.event_id);
        let Some(()) = self
            .entities
            .with_transient_events_mut(packet.entity_id, |events| {
                events.last_event_id = Some(packet.event_id)
            })
        else {
            if permission_applied {
                self.counters.entity_events_applied += 1;
                return true;
            }
            self.counters.entity_events_ignored += 1;
            return false;
        };
        let event_kind = self
            .entities
            .entity_type_id(packet.entity_id)
            .and_then(|entity_type_id| vanilla_entity_event_kind(entity_type_id, packet.event_id));
        let _ = self
            .entities
            .apply_client_animation_entity_event(packet.entity_id, packet.event_id);
        if event_kind == Some(VanillaEntityEventKind::LivingDeath) {
            let _ = self
                .entities
                .apply_client_living_death_entity_event(packet.entity_id);
        }
        if self
            .entities
            .entity_type_id(packet.entity_id)
            .is_some_and(vanilla_entity_metadata_type_uses_player_entity_event_handler)
        {
            match event_kind {
                Some(VanillaEntityEventKind::PlayerFinishUsingItem)
                    if self.local_player_id() == Some(packet.entity_id) =>
                {
                    // `ServerPlayer.completeUsingItem` sends event 9 only to the completing
                    // player's connection. The authoritative stack/effect changes arrive through
                    // their own packets; this event closes the local use/movement/render gates.
                    let _ = self.take_local_using_item();
                }
                Some(VanillaEntityEventKind::PlayerEnableReducedDebug) => {
                    let _ = self.set_local_player_reduced_debug_info_from_entity_event(
                        packet.entity_id,
                        true,
                    );
                }
                Some(VanillaEntityEventKind::PlayerDisableReducedDebug) => {
                    let _ = self.set_local_player_reduced_debug_info_from_entity_event(
                        packet.entity_id,
                        false,
                    );
                }
                _ => {}
            }
        }
        if event_kind == Some(VanillaEntityEventKind::LivingSwapHandItems) {
            let _ = self.apply_living_swap_hand_items_event(packet.entity_id);
        }
        self.counters.entity_events_applied += 1;
        true
    }

    pub fn apply_hurt_animation(&mut self, packet: ProtocolHurtAnimation) -> bool {
        self.counters.entity_hurt_animations_received += 1;
        if self
            .entities
            .apply_client_animation_hurt(packet.id)
            .is_none()
        {
            self.counters.entity_hurt_animations_ignored += 1;
            return false;
        }
        let Some(()) = self
            .entities
            .with_transient_events_mut(packet.id, |events| events.last_hurt_yaw = Some(packet.yaw))
        else {
            self.counters.entity_hurt_animations_ignored += 1;
            return false;
        };
        self.counters.entity_hurt_animations_applied += 1;
        true
    }

    pub fn apply_set_entity_link(&mut self, packet: ProtocolSetEntityLink) -> bool {
        self.counters.entity_link_updates_received += 1;
        if !self
            .entities
            .entity_type_id(packet.source_id)
            .is_some_and(vanilla_entity_metadata_type_is_leashable)
        {
            self.counters.entity_link_updates_ignored += 1;
            return false;
        }
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
        let Some(()) = self.entities.lerp_head_to(packet.id, packet.y_head_rot, 3) else {
            self.counters.entity_head_rotations_ignored += 1;
            return false;
        };
        self.counters.entity_head_rotations_applied += 1;
        true
    }
}
