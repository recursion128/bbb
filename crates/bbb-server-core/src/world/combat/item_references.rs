use crate::connection::ConnectionId;
use crate::entity::store::EquipmentItemReference;
use crate::world::LevelId;
use bbb_protocol::equipment_slots::EquipmentSlot;
use bbb_server_types::item_stack::RuntimeItemStack;

use super::enchanted_damage::SourceEntity;
use crate::world::ServerWorld;

#[derive(Debug, Clone)]
pub(in crate::world) enum EquipmentReference {
    Stored {
        dimension: LevelId,
        reference: EquipmentItemReference,
    },
    Session {
        connection: ConnectionId,
        index: usize,
    },
}

impl ServerWorld {
    pub(in crate::world) fn capture_equipment_reference(
        &self,
        dimension: &LevelId,
        owner: SourceEntity,
        slot: EquipmentSlot,
    ) -> Result<Option<EquipmentReference>, String> {
        match owner {
            SourceEntity::Stored(id) => Ok(self
                .level_by_id(dimension)
                .entities
                .equipment_item_reference(id, slot)
                .map(|reference| EquipmentReference::Stored {
                    dimension: *dimension,
                    reference,
                })),
            SourceEntity::Session(target) => {
                let player = self.levels.player(target).ok_or("the session left")?;
                let index = player
                    .inventory
                    .equipment_slots()
                    .into_iter()
                    .find(|(at, _)| *at == slot)
                    .map(|(_, index)| index);
                let Some(index) = index else {
                    return Ok(None);
                };
                Ok(Some(EquipmentReference::Session {
                    connection: target,
                    index,
                }))
            }
        }
    }

    pub(in crate::world) fn referenced_equipment_stack(
        &self,
        reference: EquipmentReference,
    ) -> Result<RuntimeItemStack, String> {
        match reference {
            EquipmentReference::Stored {
                dimension,
                reference,
            } => self
                .level_by_id(&dimension)
                .entities
                .referenced_equipment_item(reference)
                .cloned()
                .ok_or_else(|| "the original equipment stack disappeared".to_owned()),
            EquipmentReference::Session { connection, index } => {
                let player = self
                    .levels
                    .player(connection)
                    .ok_or("the original session disappeared")?;
                Ok(player
                    .inventory
                    .slot(index)
                    .ok_or("the original session stack disappeared")?
                    .clone())
            }
        }
    }

    pub(in crate::world) fn update_session_item_reference(
        &mut self,
        reference: EquipmentReference,
        item: RuntimeItemStack,
    ) -> Result<(), String> {
        let EquipmentReference::Session { connection, index } = reference else {
            return Err("a session mutation requires its original session reference".into());
        };
        let slot = self
            .levels
            .player_mut(connection)
            .ok_or("the original session disappeared")?
            .inventory
            .slot_mut(index)
            .ok_or("the equipment slot disappeared")?;
        *slot = item;
        Ok(())
    }
}
