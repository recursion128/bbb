use crate::entity::{Entity, EntityCommand, EntityData, EntityStore};

pub(crate) fn try_consume_items(entity: &Entity, store: &mut EntityStore) {
    if entity.is_alive()
        && matches!(entity.data(), EntityData::HopperMinecart(data) if data.is_enabled())
    {
        store.queue_command(EntityCommand::MinecartHopperSuck {
            entity_id: entity.id(),
        });
    }
}
