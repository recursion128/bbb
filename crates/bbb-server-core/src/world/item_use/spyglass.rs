//! `SpyglassItem`'s use hooks.

use crate::connection::ConnectionId;
use bbb_protocol::packets::{SoundEventHolder, SoundSource};
use bbb_protocol::sound_event_types::vanilla_sound_event_registry_id;
use bbb_protocol::stat_types::VANILLA_STAT_TYPE_ITEM_USED_ID;
use bbb_server_types::stats::StatKey;

use super::{ItemUseContext, ItemUseResult};
use crate::world::{ServerWorld, WorldEvent};

/// `SpyglassItem.use` (`SpyglassItem.java:30-34`).
pub(super) fn use_item(
    world: &mut ServerWorld,
    context: &ItemUseContext,
    events: &mut Vec<WorldEvent>,
) -> ItemUseResult {
    world.play_self_excluding_sound(context.connection_id, "minecraft:item.spyglass.use", events);
    if let Some(item_id) = context.item.item_id {
        world.levels[context.connection_id].award_stat_once(
            &mut world.scoreboard,
            StatKey::new(VANILLA_STAT_TYPE_ITEM_USED_ID, item_id),
        );
    }
    // `ItemUtils.startUsingInstantly` (`ItemUtils.java:11-14`).
    world.start_using_item(context.connection_id, context.hand);
    ItemUseResult::Consume
}

impl ServerWorld {
    /// `SpyglassItem.stopUsing` (`SpyglassItem.java:48-50`).
    pub(in crate::world) fn stop_using_spyglass(
        &mut self,
        connection_id: ConnectionId,
        events: &mut Vec<WorldEvent>,
    ) {
        self.play_self_excluding_sound(connection_id, "minecraft:item.spyglass.stop_using", events);
    }

    /// `Player.playSound(sound, 1.0F, 1.0F)` (`Player.java:397-399`).
    fn play_self_excluding_sound(
        &mut self,
        connection_id: ConnectionId,
        resource_id: &str,
        events: &mut Vec<WorldEvent>,
    ) {
        let Some(registry_id) = vanilla_sound_event_registry_id(resource_id) else {
            return;
        };
        let position = self.levels[connection_id].position();
        self.play_player_sound(
            connection_id,
            Some(connection_id),
            SoundEventHolder::Reference { registry_id },
            SoundSource::Players,
            position,
            1.0,
            1.0,
            events,
        );
    }
}
