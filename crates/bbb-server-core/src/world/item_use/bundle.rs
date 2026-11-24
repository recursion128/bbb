//! `BundleItem`'s use: emptying the bundle onto the ground while held, and
//! the sounds its click overrides play.

use bbb_protocol::packets::{InteractionHand, SoundEventHolder, SoundSource, Vec3d};
use bbb_protocol::sound_event_types::vanilla_sound_event_registry_id;
use bbb_protocol::stat_types::VANILLA_STAT_TYPE_ITEM_USED_ID;
use bbb_server_types::stats::StatKey;

use super::{ItemUseContext, ItemUseResult};
use crate::connection::ConnectionId;
use crate::inventory::{bundle_remove_one, BundleSound};
use crate::world::using_item::runtime_item_in_hand;
use crate::world::{ServerWorld, WorldEvent};

/// `BundleItem.getUseDuration` (`BundleItem.java:231-233`).
pub(in crate::world) const BUNDLE_USE_DURATION: i32 = 200;

/// `BundleItem.use` (`BundleItem.java:140-143`).
pub(super) fn use_item(
    world: &mut ServerWorld,
    context: &ItemUseContext,
    _events: &mut Vec<WorldEvent>,
) -> ItemUseResult {
    world.start_using_item(context.connection_id, context.hand);
    ItemUseResult::Success
}

impl ServerWorld {
    /// `BundleItem.onUseTick` (`BundleItem.java:220-228`): one stack out on the
    /// first tick, then one every other tick after the first half second.
    pub(in crate::world) fn bundle_on_use_tick(
        &mut self,
        connection_id: ConnectionId,
        hand: InteractionHand,
        remaining: i32,
        events: &mut Vec<WorldEvent>,
    ) {
        if remaining == BUNDLE_USE_DURATION
            || (remaining < BUNDLE_USE_DURATION - 10 && remaining % 2 == 0)
        {
            self.drop_bundle_content(connection_id, hand, events);
        }
    }

    /// `BundleItem.dropContent` (`BundleItem.java:145-150`, `:192-205`).
    fn drop_bundle_content(
        &mut self,
        connection_id: ConnectionId,
        hand: InteractionHand,
        events: &mut Vec<WorldEvent>,
    ) {
        let bundle = runtime_item_in_hand(self.levels.player_at(connection_id), hand).clone();
        let Ok(Some((bundle, removed))) = bundle_remove_one(&bundle) else {
            return;
        };
        self.play_bundle_sounds(connection_id, &[BundleSound::RemoveOne], events);
        let item_id = bundle.item_id();
        self.update_hand_stack(connection_id, hand, bundle);
        if self
            .drop_player_original_stack(connection_id, removed, true)
            .is_none()
        {
            return;
        }
        let player = self.levels.player_at(connection_id);
        let pos = super::super::player_packets::block_position(player.position());
        let pitch = 0.8 + self.random.next_float() * 0.4;
        if let Some(registry_id) =
            vanilla_sound_event_registry_id("minecraft:item.bundle.drop_contents")
        {
            self.play_player_sound(
                connection_id,
                None,
                SoundEventHolder::Reference { registry_id },
                SoundSource::Players,
                Vec3d {
                    x: f64::from(pos.x) + 0.5,
                    y: f64::from(pos.y) + 0.5,
                    z: f64::from(pos.z) + 0.5,
                },
                0.8,
                pitch,
                events,
            );
        }
        if let Some(item_id) = item_id {
            self.levels[connection_id].award_stat(
                &mut self.scoreboard,
                StatKey::new(VANILLA_STAT_TYPE_ITEM_USED_ID, item_id),
                1,
            );
        }
    }

    /// `BundleItem.playRemoveOneSound` / `playInsertSound` /
    /// `playInsertFailSound` (`BundleItem.java:257-267`): `Player.playSound`,
    /// which the clicking player does not hear from the server.
    pub(in crate::world) fn play_bundle_sounds(
        &mut self,
        connection_id: ConnectionId,
        sounds: &[BundleSound],
        events: &mut Vec<WorldEvent>,
    ) {
        for sound in sounds {
            let (resource_id, volume, pitch) = match sound {
                BundleSound::Insert => (
                    "minecraft:item.bundle.insert",
                    0.8,
                    0.8 + self.random.next_float() * 0.4,
                ),
                BundleSound::RemoveOne => (
                    "minecraft:item.bundle.remove_one",
                    0.8,
                    0.8 + self.random.next_float() * 0.4,
                ),
                BundleSound::InsertFail => ("minecraft:item.bundle.insert_fail", 1.0, 1.0),
            };
            let Some(registry_id) = vanilla_sound_event_registry_id(resource_id) else {
                continue;
            };
            let position = self.levels[connection_id].position();
            self.play_player_sound(
                connection_id,
                Some(connection_id),
                SoundEventHolder::Reference { registry_id },
                SoundSource::Players,
                position,
                volume,
                pitch,
                events,
            );
        }
    }
}
