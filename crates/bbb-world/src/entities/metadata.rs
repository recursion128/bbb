use bbb_protocol::packets::{
    SetEntityData as ProtocolSetEntityData, SetEquipment as ProtocolSetEquipment,
    UpdateAttributes as ProtocolUpdateAttributes,
};

use crate::WorldStore;

use super::dimensions::vanilla_living_entity_type;

impl WorldStore {
    pub fn apply_set_entity_data(&mut self, packet: ProtocolSetEntityData) -> bool {
        self.counters.entity_data_updates_received += 1;
        self.counters.entity_data_values_received += packet.values.len();
        let id = packet.id;
        let Some(()) = self.entities.with_metadata_mut(id, |metadata| {
            for value in packet.values {
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
            return false;
        };
        let _ = self
            .entities
            .sync_client_animation_targets_from_metadata(id);
        let _ = self.entities.refresh_client_position_from_entity_data(id);
        self.counters.entity_data_updates_applied += 1;
        true
    }

    pub fn apply_set_equipment(&mut self, packet: ProtocolSetEquipment) -> bool {
        self.counters.entity_equipment_updates_received += 1;
        self.counters.entity_equipment_slots_received += packet.slots.len();
        let Some(entity_type_id) = self.entities.entity_type_id(packet.entity_id) else {
            return false;
        };
        if !vanilla_living_entity_type(entity_type_id) {
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
            return false;
        };
        self.counters.entity_equipment_updates_applied += 1;
        true
    }

    pub fn apply_update_attributes(&mut self, packet: ProtocolUpdateAttributes) -> bool {
        self.counters.entity_attribute_updates_received += 1;
        self.counters.entity_attributes_received += packet.attributes.len();
        let Some(entity_type_id) = self.entities.entity_type_id(packet.entity_id) else {
            return false;
        };
        if !vanilla_living_entity_type(entity_type_id) {
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
            return false;
        };
        self.counters.entity_attribute_updates_applied += 1;
        true
    }
}
