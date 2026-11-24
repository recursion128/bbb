//! `ChiseledBookShelfBlock` (`ChiseledBookShelfBlock.java`): the book a
//! click puts into the hit slot, and the one a click takes back out.

use crate::world::LevelId;
use bbb_protocol::packets::{
    BlockPos, InteractionHand, SoundEvent, SoundEventHolder, SoundSource, UseItemOn, Vec3d,
};
use bbb_protocol::sound_event_types::vanilla_sound_event_registry_id;

use super::BlockUseContext;
use crate::block_behavior::chiseled_bookshelf as chiseled_bookshelf_block;
use crate::block_behavior::BlockUseResult;
use crate::block_entity::chiseled_bookshelf;
use crate::connection::ConnectionId;
use crate::fluid::SET_BLOCK_UPDATE_ALL;
use crate::world::using_item::runtime_item_in_hand;
use crate::world::{ServerWorld, WorldEvent};
use bbb_server_types::item_stack::RuntimeItemStack;

/// The `item_on:` column of the `block_use_dispatch!` row:
/// [`ServerWorld::use_chiseled_bookshelf_item_on`].
pub(super) fn use_item_on(
    world: &mut ServerWorld,
    context: &BlockUseContext<'_>,
    events: &mut Vec<WorldEvent>,
) -> BlockUseResult {
    world.use_chiseled_bookshelf_item_on(
        context.connection_id,
        context.dimension,
        context.update,
        context.block_state_id,
        events,
    )
}

/// The `without_item:` column of the `block_use_dispatch!` row:
/// [`ServerWorld::use_chiseled_bookshelf_without_item`].
pub(super) fn use_without_item(
    world: &mut ServerWorld,
    context: &BlockUseContext<'_>,
    events: &mut Vec<WorldEvent>,
) -> bool {
    world.use_chiseled_bookshelf_without_item(
        context.connection_id,
        context.dimension,
        context.update,
        context.block_state_id,
        events,
    )
}

impl ServerWorld {
    /// `ChiseledBookShelfBlock.useItemOn` (`ChiseledBookShelfBlock.java:76-104`).
    pub(super) fn use_chiseled_bookshelf_item_on(
        &mut self,
        connection_id: ConnectionId,
        dimension: impl Into<LevelId>,
        update: UseItemOn,
        block_state_id: i32,
        events: &mut Vec<WorldEvent>,
    ) -> BlockUseResult {
        let dimension = dimension.into();
        if !chiseled_bookshelf::item_is_bookshelf_book_with(
            runtime_item_in_hand(self.levels.player_at(connection_id), update.hand),
            |registry, tag, id| self.registries.registry_tags().contains(registry, tag, id),
        ) {
            return BlockUseResult::TryWithEmptyHand;
        }
        let facing = chiseled_bookshelf_block::facing(block_state_id);
        let Some(slot) = chiseled_bookshelf_block::hit_slot(
            facing,
            update.hit.direction,
            [
                update.hit.cursor_x,
                update.hit.cursor_y,
                update.hit.cursor_z,
            ],
        ) else {
            return BlockUseResult::Pass;
        };
        if chiseled_bookshelf_block::slot_occupied(block_state_id, slot) {
            return BlockUseResult::TryWithEmptyHand;
        }
        if self.insert_bookshelf_book(
            connection_id,
            dimension,
            update.hit.pos,
            update.hand,
            slot,
            events,
        ) {
            BlockUseResult::Consumed
        } else {
            BlockUseResult::Pass
        }
    }

    /// `ChiseledBookShelfBlock.useWithoutItem` (`ChiseledBookShelfBlock.java:107-125`).
    pub(super) fn use_chiseled_bookshelf_without_item(
        &mut self,
        connection_id: ConnectionId,
        dimension: impl Into<LevelId>,
        update: UseItemOn,
        block_state_id: i32,
        events: &mut Vec<WorldEvent>,
    ) -> bool {
        let dimension = dimension.into();
        let facing = chiseled_bookshelf_block::facing(block_state_id);
        let Some(slot) = chiseled_bookshelf_block::hit_slot(
            facing,
            update.hit.direction,
            [
                update.hit.cursor_x,
                update.hit.cursor_y,
                update.hit.cursor_z,
            ],
        ) else {
            return false;
        };
        if !chiseled_bookshelf_block::slot_occupied(block_state_id, slot) {
            return true;
        }
        self.remove_bookshelf_book(connection_id, dimension, update.hit.pos, slot, events);
        true
    }

    pub(super) fn insert_bookshelf_book(
        &mut self,
        connection_id: ConnectionId,
        dimension: impl Into<LevelId>,
        pos: BlockPos,
        hand: InteractionHand,
        slot: usize,
        events: &mut Vec<WorldEvent>,
    ) -> bool {
        let dimension = dimension.into();
        let player = self
            .levels
            .player(connection_id)
            .expect("connection presence was checked");
        let held = runtime_item_in_hand(player, hand);
        if !chiseled_bookshelf::item_is_bookshelf_book_with(held, |registry, tag, id| {
            self.registries.registry_tags().contains(registry, tag, id)
        }) {
            return false;
        }
        let enchanted =
            held.item_id() == Some(bbb_protocol::item_types::VANILLA_ITEM_ENCHANTED_BOOK_ID);
        let book = held.with_count(1);
        {
            let Some(bookshelf) = self
                .level_mut_by_id(&dimension)
                .chunks
                .block_entities_mut()
                .get_mut(pos)
                .and_then(|body| body.chiseled_bookshelf_mut())
            else {
                return false;
            };
            if !bookshelf.set_original_slot(slot, book, true) {
                return false;
            }
        }
        let player = self
            .levels
            .player_mut(connection_id)
            .expect("connection presence was checked");
        let item_id = runtime_item_in_hand(player, hand).item_id();
        if let Some(item_id) = item_id {
            player.award_stat_once(
                &mut self.scoreboard,
                bbb_server_types::stats::StatKey::new(
                    bbb_protocol::stat_types::VANILLA_STAT_TYPE_ITEM_USED_ID,
                    item_id,
                ),
            );
        }
        player
            .inventory
            .consume_in_hand(hand, 1, player.abilities.instabuild);
        self.level_mut_by_id(&dimension)
            .chunks
            .block_entities_mut()
            .set_changed(pos);
        self.sync_bookshelf_occupancy_by_key(&dimension, pos);
        let sound = if enchanted {
            chiseled_bookshelf_block::INSERT_ENCHANTED_SOUND
        } else {
            chiseled_bookshelf_block::INSERT_SOUND
        };
        self.play_bookshelf_sound(dimension, pos, sound, events);
        true
    }

    pub(super) fn remove_bookshelf_book(
        &mut self,
        connection_id: ConnectionId,
        dimension: impl Into<LevelId>,
        pos: BlockPos,
        slot: usize,
        events: &mut Vec<WorldEvent>,
    ) {
        let resolver = self.item_stack_resolver();
        let dimension = dimension.into();
        let mut taken = {
            let Some(bookshelf) = self
                .level_mut_by_id(&dimension)
                .chunks
                .block_entities_mut()
                .get_mut(pos)
                .and_then(|body| body.chiseled_bookshelf_mut())
            else {
                return;
            };
            let Some(taken) = bookshelf.original_items().get(slot).cloned() else {
                return;
            };
            if taken.is_empty() || !bookshelf.set_slot(slot, RuntimeItemStack::empty()) {
                return;
            }
            taken
        };
        let enchanted =
            taken.item_id() == Some(bbb_protocol::item_types::VANILLA_ITEM_ENCHANTED_BOOK_ID);
        self.level_mut_by_id(&dimension)
            .chunks
            .block_entities_mut()
            .set_changed(pos);
        self.sync_bookshelf_occupancy_by_key(&dimension, pos);
        let sound = if enchanted {
            chiseled_bookshelf_block::PICKUP_ENCHANTED_SOUND
        } else {
            chiseled_bookshelf_block::PICKUP_SOUND
        };
        self.play_bookshelf_sound(dimension, pos, sound, events);
        let player = self
            .levels
            .player_mut(connection_id)
            .expect("connection presence was checked");
        let creative = player.abilities.instabuild;
        let added = player
            .inventory
            .add_anywhere(&resolver, &mut taken, creative);
        let refusal = match added {
            Ok(false) if !taken.is_empty() => self
                .drop_player_original_stack(connection_id, taken, false)
                .is_none()
                .then(|| "original book drop refused".to_string()),
            Ok(_) => None,
            Err(error) => Some(error.to_string()),
        };
        if let Some(detail) = refusal {
            tracing::warn!("bookshelf book removal: {detail}");
            return;
        }
        self.player_game_event_at(
            connection_id,
            crate::level_event::VanillaGameEvent::BLOCK_CHANGE,
            Vec3d {
                x: f64::from(pos.x) + 0.5,
                y: f64::from(pos.y) + 0.5,
                z: f64::from(pos.z) + 0.5,
            },
            None,
        );
    }

    pub(in crate::world) fn sync_bookshelf_occupancy_by_key(
        &mut self,
        dimension: &LevelId,
        pos: BlockPos,
    ) {
        let Some(block_state_id) = self.block_state_by_key(dimension, pos) else {
            return;
        };
        let Some(bookshelf) = self
            .level_by_id(dimension)
            .chunks
            .block_entities()
            .get(pos)
            .and_then(|body| body.chiseled_bookshelf())
        else {
            return;
        };
        let Some(next) =
            chiseled_bookshelf_block::with_occupancy(block_state_id, bookshelf.original_items())
        else {
            return;
        };
        if next != block_state_id {
            let _ = self.set_block(dimension, pos, next, SET_BLOCK_UPDATE_ALL);
        }
        self.level_mut_by_id(dimension)
            .event_sink
            .game_event_with_source(
                crate::level_event::VanillaGameEvent::BLOCK_CHANGE,
                pos,
                None,
                Some(next),
            );
    }

    pub(super) fn play_bookshelf_sound(
        &mut self,
        dimension: impl Into<LevelId>,
        pos: BlockPos,
        sound_resource_id: &str,
        events: &mut Vec<WorldEvent>,
    ) {
        let dimension = dimension.into();
        let Some(registry_id) = vanilla_sound_event_registry_id(sound_resource_id) else {
            return;
        };
        let sound = SoundEvent {
            sound: SoundEventHolder::Reference { registry_id },
            source: SoundSource::Blocks,
            position: Vec3d {
                x: f64::from(pos.x) + 0.5,
                y: f64::from(pos.y) + 0.5,
                z: f64::from(pos.z) + 0.5,
            },
            volume: 1.0,
            pitch: 1.0,
            seed: self.random.next_long(),
        };
        self.broadcast_level_sound_by_key(&dimension, &sound, None, events);
    }
}
