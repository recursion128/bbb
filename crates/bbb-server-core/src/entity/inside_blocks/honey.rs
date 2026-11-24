use crate::block_behavior::honey;
use crate::connection::ClientboundPlayPacket;
use crate::entity::{living::living_sound_source, Entity, EntityStore, PendingEntitySound};
use crate::world::simulation_level::LevelCtx;
use bbb_protocol::packets::EntityEvent;

pub(crate) fn slide_effects(
    entity: &Entity,
    store: &mut EntityStore,
    level: &mut LevelCtx<'_, '_>,
) -> Option<()> {
    let (sound, event) = honey::slide_effects(|| level.level_random().next_int(5));
    if sound && !entity.is_silent() {
        store.play_entity_sound(PendingEntitySound {
            position: entity.position(),
            resource_id: "minecraft:block.honey_block.slide".to_owned(),
            source: living_sound_source(entity),
            volume: 1.0,
            pitch: 1.0,
        });
    }
    if event {
        let entity_id = entity.id();
        store.broadcast_to_tracking(
            entity_id,
            ClientboundPlayPacket::EntityEvent(EntityEvent {
                entity_id,
                event_id: 53,
            }),
        );
    }
    Some(())
}
