use bbb_protocol::packets::{
    SetEntityData as ProtocolSetEntityData, SetEquipment as ProtocolSetEquipment,
    UpdateAttributes as ProtocolUpdateAttributes,
};

use crate::WorldStore;

impl WorldStore {
    pub fn apply_set_entity_data(&mut self, packet: ProtocolSetEntityData) -> bool {
        self.counters.entity_data_updates_received += 1;
        self.counters.entity_data_values_received += packet.values.len();
        let Some(entity) = self
            .entities
            .iter_mut()
            .find(|entity| entity.id == packet.id)
        else {
            return false;
        };

        for value in packet.values {
            if let Some(existing) = entity
                .data_values
                .iter_mut()
                .find(|existing| existing.data_id == value.data_id)
            {
                *existing = value;
            } else {
                entity.data_values.push(value);
            }
        }
        entity.data_values.sort_by_key(|value| value.data_id);
        self.counters.entity_data_updates_applied += 1;
        true
    }

    pub fn apply_set_equipment(&mut self, packet: ProtocolSetEquipment) -> bool {
        self.counters.entity_equipment_updates_received += 1;
        self.counters.entity_equipment_slots_received += packet.slots.len();
        let Some(entity) = self
            .entities
            .iter_mut()
            .find(|entity| entity.id == packet.entity_id)
        else {
            return false;
        };

        for update in packet.slots {
            if let Some(existing) = entity
                .equipment
                .iter_mut()
                .find(|existing| existing.slot == update.slot)
            {
                *existing = update;
            } else {
                entity.equipment.push(update);
            }
        }
        entity.equipment.sort_by_key(|update| update.slot.ordinal());
        self.counters.entity_equipment_updates_applied += 1;
        true
    }

    pub fn apply_update_attributes(&mut self, packet: ProtocolUpdateAttributes) -> bool {
        self.counters.entity_attribute_updates_received += 1;
        self.counters.entity_attributes_received += packet.attributes.len();
        let Some(entity) = self
            .entities
            .iter_mut()
            .find(|entity| entity.id == packet.entity_id)
        else {
            return false;
        };

        for attribute in packet.attributes {
            if let Some(existing) = entity
                .attributes
                .iter_mut()
                .find(|existing| existing.attribute_id == attribute.attribute_id)
            {
                *existing = attribute;
            } else {
                entity.attributes.push(attribute);
            }
        }
        entity
            .attributes
            .sort_by_key(|attribute| attribute.attribute_id);
        self.counters.entity_attribute_updates_applied += 1;
        true
    }
}
