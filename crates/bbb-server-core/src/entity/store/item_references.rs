use crate::world::simulation_level::LevelCtx;
use bbb_protocol::equipment_slots::EquipmentSlot;
use bbb_server_types::item_stack::RuntimeItemStack;
use uuid::Uuid;

use crate::entity::EntityStore;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EquipmentItemReference {
    entity_id: i32,
    uuid: Uuid,
    slot: EquipmentSlot,
}

impl EntityStore {
    pub fn equipment_item_reference(
        &self,
        id: i32,
        slot: EquipmentSlot,
    ) -> Option<EquipmentItemReference> {
        let body = self.get(id)?;
        let uuid = body.uuid();
        body.living()?.equipment().get(slot)?;
        Some(EquipmentItemReference {
            entity_id: id,
            uuid,
            slot,
        })
    }

    pub fn referenced_equipment_item(
        &self,
        reference: EquipmentItemReference,
    ) -> Option<&bbb_server_types::item_stack::RuntimeItemStack> {
        let body = self.get(reference.entity_id)?;
        if body.uuid() != reference.uuid {
            return None;
        }
        body.living()?.equipment().referenced_item(reference.slot)
    }

    pub fn update_equipment_item_reference(
        &mut self,
        reference: EquipmentItemReference,
        item: RuntimeItemStack,
    ) -> Option<()> {
        self.get_mut(reference.entity_id)
            .filter(|body| body.uuid() == reference.uuid)?
            .living_mut()?
            .equipment_mut()
            .update_item(reference.slot, item)
    }

    pub(crate) fn on_equipment_item_reference_broken(
        &mut self,
        reference: EquipmentItemReference,
        level: &mut LevelCtx<'_, '_>,
        slot: EquipmentSlot,
    ) -> Result<(), String> {
        if !self
            .get(reference.entity_id)
            .is_some_and(|body| body.uuid() == reference.uuid && body.living().is_some())
        {
            return Ok(());
        }
        self.with_entity_mut(reference.entity_id, |body, store| {
            crate::entity::daylight_burn::on_equipped_item_broken(body, store, level, slot)
        })
        .unwrap_or(Ok(()))
    }
}
