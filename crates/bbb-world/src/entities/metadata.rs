use bbb_protocol::packets::{
    EntityDataValueKind, InteractionHand, SetEntityData as ProtocolSetEntityData,
    SetEquipment as ProtocolSetEquipment, UpdateAttributes as ProtocolUpdateAttributes,
};

use crate::WorldStore;

use super::dimensions::{vanilla_living_entity_type, VANILLA_POSE_SLEEPING_ID};

const VANILLA_LIVING_ENTITY_FLAGS_DATA_ID: u8 = 8;
const LIVING_ENTITY_FLAG_IS_USING: i8 = 0x01;
const LIVING_ENTITY_FLAG_OFF_HAND: i8 = 0x02;

impl WorldStore {
    pub fn apply_set_entity_data(&mut self, packet: ProtocolSetEntityData) -> bool {
        self.counters.entity_data_updates_received += 1;
        self.counters.entity_data_values_received += packet.values.len();
        let id = packet.id;
        let is_local_player = self.local_player_id == Some(id);
        let updated_values = packet.values.clone();
        let mut local_living_entity_flags = None;
        let Some(()) = self.entities.with_metadata_mut(id, |metadata| {
            for value in packet.values {
                if is_local_player && value.data_id == VANILLA_LIVING_ENTITY_FLAGS_DATA_ID {
                    if let EntityDataValueKind::Byte(flags) = &value.value {
                        local_living_entity_flags = Some(*flags);
                    }
                }
                if let Some(existing) = metadata
                    .data_values
                    .iter_mut()
                    .find(|existing| existing.data_id == value.data_id)
                {
                    *existing = value;
                } else {
                    metadata.data_values.push(value);
                }
            }
            metadata.data_values.sort_by_key(|value| value.data_id);
        }) else {
            self.counters.entity_data_updates_ignored += 1;
            return false;
        };
        let _ = self
            .entities
            .sync_client_animation_targets_from_metadata(id);
        let _ = self
            .entities
            .sync_client_animation_events_from_metadata_update(id, &updated_values);
        let _ = self.entities.refresh_client_position_from_entity_data(id);
        if let Some(flags) = local_living_entity_flags {
            self.sync_local_using_item_from_living_entity_flags(flags);
        }
        self.counters.entity_data_updates_applied += 1;
        true
    }

    fn sync_local_using_item_from_living_entity_flags(&mut self, flags: i8) {
        let server_using_item = flags & LIVING_ENTITY_FLAG_IS_USING != 0;
        if !server_using_item {
            if self.local_player.interaction.using_item {
                self.take_local_using_item();
            }
            return;
        }

        if self.local_player.interaction.using_item {
            return;
        }

        let hand = if flags & LIVING_ENTITY_FLAG_OFF_HAND != 0 {
            InteractionHand::OffHand
        } else {
            InteractionHand::MainHand
        };
        if self.local_item_in_hand_is_non_empty(hand) {
            self.set_local_using_item_with_hand(true, hand);
        }
    }

    pub fn local_player_is_sleeping(&self) -> bool {
        self.local_player_id
            .and_then(|id| self.entities.pose(id))
            .is_some_and(|pose| pose == VANILLA_POSE_SLEEPING_ID)
    }

    pub fn apply_set_equipment(&mut self, packet: ProtocolSetEquipment) -> bool {
        self.counters.entity_equipment_updates_received += 1;
        self.counters.entity_equipment_slots_received += packet.slots.len();
        let Some(entity_type_id) = self.entities.entity_type_id(packet.entity_id) else {
            self.counters.entity_equipment_updates_ignored += 1;
            return false;
        };
        if !vanilla_living_entity_type(entity_type_id) {
            self.counters.entity_equipment_updates_ignored += 1;
            return false;
        }
        let Some(()) = self
            .entities
            .with_equipment_mut(packet.entity_id, |equipment| {
                for update in packet.slots {
                    if let Some(existing) = equipment
                        .equipment
                        .iter_mut()
                        .find(|existing| existing.slot == update.slot)
                    {
                        *existing = update;
                    } else {
                        equipment.equipment.push(update);
                    }
                }
                equipment
                    .equipment
                    .sort_by_key(|update| update.slot.ordinal());
            })
        else {
            self.counters.entity_equipment_updates_ignored += 1;
            return false;
        };
        self.counters.entity_equipment_updates_applied += 1;
        self.refresh_entity_active_swing_duration(packet.entity_id);
        true
    }

    pub fn apply_update_attributes(&mut self, packet: ProtocolUpdateAttributes) -> bool {
        self.counters.entity_attribute_updates_received += 1;
        self.counters.entity_attributes_received += packet.attributes.len();
        let Some(entity_type_id) = self.entities.entity_type_id(packet.entity_id) else {
            self.counters.entity_attribute_updates_ignored += 1;
            return false;
        };
        if !vanilla_living_entity_type(entity_type_id) {
            self.counters.entity_attribute_updates_ignored += 1;
            return false;
        }
        let Some(()) = self
            .entities
            .with_attributes_mut(packet.entity_id, |attributes| {
                for attribute in packet.attributes {
                    if let Some(existing) = attributes
                        .attributes
                        .iter_mut()
                        .find(|existing| existing.attribute_id == attribute.attribute_id)
                    {
                        *existing = attribute;
                    } else {
                        attributes.attributes.push(attribute);
                    }
                }
                attributes
                    .attributes
                    .sort_by_key(|attribute| attribute.attribute_id);
            })
        else {
            self.counters.entity_attribute_updates_ignored += 1;
            return false;
        };
        self.counters.entity_attribute_updates_applied += 1;
        true
    }
}
