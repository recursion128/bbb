//! `ShelfBlock` (`ShelfBlock.java`): the item a click swaps with the hit
//! slot, and the whole-hotbar swap a powered shelf answers with.

use crate::world::LevelId;
use bbb_protocol::packets::{
    BlockPos, InteractionHand, SetPlayerInventory, SoundEvent, SoundEventHolder, SoundSource,
    UseItemOn, Vec3d,
};
use bbb_protocol::sound_event_types::vanilla_sound_event_registry_id;

use super::BlockUseContext;
use crate::block_behavior::shelf as shelf_block;
use crate::block_behavior::BlockUseResult;
use crate::block_entity::shelf;
use crate::connection::{ClientboundPlayPacket, ConnectionId};
use crate::level_event::VanillaGameEvent;
use crate::world::{send, ServerWorld, WorldEvent};

const WOODEN_SHELVES: &str = "minecraft:wooden_shelves";
/// `Inventory.getSelectionSize`.
const HOTBAR_SIZE: usize = 9;
#[cfg(test)]
use bbb_server_types::save::Java26BuiltinDimension;

/// The `item_on:` column of the `block_use_dispatch!` row:
/// [`ServerWorld::use_shelf_item_on`].
pub(super) fn use_item_on(
    world: &mut ServerWorld,
    context: &BlockUseContext<'_>,
    events: &mut Vec<WorldEvent>,
) -> BlockUseResult {
    world.use_shelf_item_on(
        context.connection_id,
        context.dimension,
        context.update,
        context.block_state_id,
        events,
    )
}

impl ServerWorld {
    /// `ShelfBlock.useItemOn` (`ShelfBlock.java:156-204`).
    pub(super) fn use_shelf_item_on(
        &mut self,
        connection_id: ConnectionId,
        dimension: impl Into<LevelId>,
        update: UseItemOn,
        block_state_id: i32,
        events: &mut Vec<WorldEvent>,
    ) -> BlockUseResult {
        let dimension = dimension.into();
        if update.hand != InteractionHand::MainHand {
            return BlockUseResult::Pass;
        }
        let facing = shelf_block::facing(block_state_id);
        let Some(slot) = shelf_block::hit_slot(
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
        if shelf_block::powered(block_state_id) {
            self.swap_shelf_hotbar(connection_id, dimension, update.hit.pos, events);
            BlockUseResult::Consumed
        } else {
            self.swap_shelf_single(connection_id, dimension, update.hit.pos, slot, events)
        }
    }

    pub(super) fn swap_shelf_single(
        &mut self,
        connection_id: ConnectionId,
        dimension: impl Into<LevelId>,
        pos: BlockPos,
        slot: usize,
        events: &mut Vec<WorldEvent>,
    ) -> BlockUseResult {
        let dimension = dimension.into();
        let player = self
            .levels
            .player(connection_id)
            .expect("connection presence was checked");
        let held = player.inventory.selected_stack().clone();
        let selected_slot = usize::from(player.inventory.selected_slot());
        let creative = player.abilities.instabuild;
        let held_empty = held.is_empty();
        let removed = {
            let Some(shelf) = self
                .level_mut_by_id(&dimension)
                .chunks
                .block_entities_mut()
                .get_mut(pos)
                .and_then(|body| body.shelf_mut())
            else {
                return BlockUseResult::Pass;
            };
            let Some(removed) = shelf.swap_item(slot, held.clone()) else {
                return BlockUseResult::Pass;
            };
            removed
        };
        let item_removed = !removed.is_empty();
        let into_hand = if creative && !item_removed {
            held
        } else {
            removed
        };
        // `ShelfBlock.java:213-216`, `ShelfBlockEntity.java:87-92`.
        let vibrates = !crate::inventory::has_component(
            into_hand.summary(),
            bbb_protocol::data_component_types::VANILLA_DATA_COMPONENT_USE_EFFECTS_TYPE_ID,
        )
        .unwrap_or(false)
            || bbb_server_types::item_stack::rules::interact_vibrations(into_hand.summary());
        let update = send(
            connection_id,
            ClientboundPlayPacket::SetPlayerInventory(SetPlayerInventory {
                slot: i32::try_from(selected_slot).expect("hotbar slots are small"),
                item: into_hand.summary().clone(),
            }),
        );
        self.levels[connection_id]
            .inventory
            .set_inventory_slot(selected_slot, into_hand);
        self.shelf_changed(&dimension, pos);
        if vibrates {
            self.shelf_game_event(dimension, pos, VanillaGameEvent::ITEM_INTERACT_FINISH);
        }
        events.push(update);
        let sound = if item_removed {
            if held_empty {
                shelf_block::TAKE_ITEM_SOUND
            } else {
                shelf_block::SINGLE_SWAP_SOUND
            }
        } else if held_empty {
            return BlockUseResult::Pass;
        } else {
            shelf_block::PLACE_ITEM_SOUND
        };
        self.play_shelf_sound(dimension, pos, sound, events);
        BlockUseResult::Consumed
    }

    /// `ShelfBlock.swapHotbar` (`ShelfBlock.java:221-247`): every shelf of the
    /// powered chain takes its share of the hotbar, the rightmost the last
    /// three slots.
    pub(super) fn swap_shelf_hotbar(
        &mut self,
        connection_id: ConnectionId,
        dimension: impl Into<LevelId>,
        pos: BlockPos,
        events: &mut Vec<WorldEvent>,
    ) -> bool {
        let dimension = dimension.into();
        let chain = self.shelf_chain(&dimension, pos);
        let mut updates = Vec::new();
        let mut any = false;
        for (index, &part) in chain.iter().enumerate() {
            let first_slot =
                HOTBAR_SIZE as isize - (chain.len() - index) as isize * shelf::MAX_ITEMS as isize;
            let placed: Vec<_> = (0..shelf::MAX_ITEMS)
                .filter_map(|slot| {
                    let inventory_slot = usize::try_from(first_slot + slot as isize)
                        .ok()
                        .filter(|&s| s < HOTBAR_SIZE)?;
                    let item = self
                        .levels
                        .player(connection_id)
                        .expect("connection presence was checked")
                        .inventory
                        .slot(inventory_slot)
                        .cloned()?;
                    Some((slot, inventory_slot, item))
                })
                .collect();
            let mut replacements = Vec::with_capacity(placed.len());
            {
                let Some(shelf) = self
                    .level_mut_by_id(&dimension)
                    .chunks
                    .block_entities_mut()
                    .get_mut(part)
                    .and_then(|body| body.shelf_mut())
                else {
                    continue;
                };
                for (slot, inventory_slot, placed) in placed {
                    let placed_empty = placed.is_empty();
                    let taken = shelf.swap_item(slot, placed).expect("shelf slot");
                    if placed_empty && taken.is_empty() {
                        replacements.push((
                            inventory_slot,
                            bbb_server_types::item_stack::RuntimeItemStack::empty(),
                        ));
                        continue;
                    }
                    any = true;
                    updates.push(send(
                        connection_id,
                        ClientboundPlayPacket::SetPlayerInventory(SetPlayerInventory {
                            slot: i32::try_from(inventory_slot).expect("hotbar slots are small"),
                            item: taken.summary().clone(),
                        }),
                    ));
                    replacements.push((inventory_slot, taken));
                }
            }
            let player = self
                .levels
                .player_mut(connection_id)
                .expect("connection presence was checked");
            for (slot, item) in replacements {
                player.inventory.set_inventory_slot(slot, item);
            }
            self.shelf_changed(&dimension, part);
            // `ShelfBlock.java:245`.
            self.shelf_game_event(dimension, part, VanillaGameEvent::ENTITY_INTERACT);
        }
        events.extend(updates);
        if any {
            self.play_shelf_sound(dimension, pos, shelf_block::MULTI_SWAP_SOUND, events);
        }
        any
    }

    /// `ShelfBlockEntity.setChanged` (`ShelfBlockEntity.java:87-100`) also
    /// sends the block update that carries the new items to watchers.
    fn shelf_changed(&mut self, dimension: &LevelId, pos: BlockPos) {
        let level = self.level_mut_by_id(dimension);
        level.chunks.block_entities_mut().set_changed(pos);
        level.changed_block_broadcasts.insert((pos.x, pos.y, pos.z));
    }

    fn shelf_chain(&self, dimension: &LevelId, pos: BlockPos) -> Vec<BlockPos> {
        use crate::block_getter::BlockTags;
        let chunks = &self.level_by_id(dimension).chunks;
        let tags = chunks.block_tags();
        shelf_block::connected_chain(
            pos,
            |at| chunks.get_block_state(at.x, at.y, at.z).unwrap_or(0),
            |state| tags.block_in_tag(state, WOODEN_SHELVES) && shelf_block::powered(state),
        )
    }

    fn shelf_game_event(&mut self, dimension: LevelId, pos: BlockPos, kind: VanillaGameEvent) {
        let state = self.block_state_or_air(dimension, pos);
        self.level_mut_by_id(&dimension)
            .event_sink
            .game_event(kind, pos, Some(state));
    }

    pub(super) fn play_shelf_sound(
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shelf_swap_transports_both_originals_and_keeps_personal_ender_shared() {
        let mut world = ServerWorld::new(crate::tests::datapack_config()).unwrap();
        let (sender, receiver) = crate::tests::mailbox();
        crate::tests::join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
        let connection = ConnectionId(1);
        let pos = BlockPos { x: 0, y: 64, z: 1 };
        // Powered, so the hotbar leg below swaps with this shelf's chain.
        assert!(world.set_block(
            Java26BuiltinDimension::Overworld,
            BlockPos { y: 63, ..pos },
            redstone_block(),
            crate::fluid::SET_BLOCK_UPDATE_ALL
        ));
        let state = powered_oak_shelf();
        assert!(world.set_block(
            Java26BuiltinDimension::Overworld,
            pos,
            state,
            crate::fluid::SET_BLOCK_UPDATE_ALL
        ));
        world.set_player_game_mode(
            connection,
            bbb_protocol::game_types::GameType::Survival,
            &mut Vec::new(),
        );
        let stone = crate::tests::native_stack(bbb_protocol::packets::ItemStackSummary {
            item_id: Some(bbb_protocol::item_types::VANILLA_ITEM_STONE_ID),
            count: 3,
            component_patch: Default::default(),
        });
        let player = world.levels.player_mut(connection).unwrap();
        player.inventory.set_inventory_slot(0, stone.clone());
        let held = player.inventory.slot(0).cloned().unwrap();
        let mut events = Vec::new();
        // Flush the placement's own block update first.
        world.broadcast_changed_blocks(&Java26BuiltinDimension::Overworld.into(), &mut events);
        events.clear();
        assert_eq!(
            world.swap_shelf_single(
                connection,
                Java26BuiltinDimension::Overworld,
                pos,
                0,
                &mut events
            ),
            BlockUseResult::Consumed
        );
        let shelf = world
            .level(Java26BuiltinDimension::Overworld)
            .chunks
            .block_entities()
            .get(pos)
            .and_then(crate::block_entity::BlockEntity::shelf)
            .unwrap();
        assert_eq!(shelf.original_items()[0], held);
        let mut broadcast = Vec::new();
        world.broadcast_changed_blocks(&Java26BuiltinDimension::Overworld.into(), &mut broadcast);
        assert!(broadcast.iter().any(|event| matches!(
            event,
            WorldEvent::Send {
                connection_id,
                packet: ClientboundPlayPacket::BlockEntityData(update),
            } if *connection_id == connection && update.pos == pos
        )));
        let raised = |world: &mut ServerWorld| {
            world
                .level_mut(Java26BuiltinDimension::Overworld)
                .event_sink
                .drain_game_events()
                .into_iter()
                .filter(|event| event.pos == pos)
                .map(|event| event.game_event)
                .collect::<Vec<_>>()
        };
        assert!(raised(&mut world).contains(&VanillaGameEvent::ITEM_INTERACT_FINISH));
        world.swap_shelf_hotbar(
            connection,
            Java26BuiltinDimension::Overworld,
            pos,
            &mut events,
        );
        assert!(raised(&mut world).contains(&VanillaGameEvent::ENTITY_INTERACT));
        world.swap_shelf_hotbar(
            connection,
            Java26BuiltinDimension::Overworld,
            pos,
            &mut events,
        );
        assert_eq!(
            world.swap_shelf_single(
                connection,
                Java26BuiltinDimension::Overworld,
                pos,
                0,
                &mut events
            ),
            BlockUseResult::Consumed
        );
        assert_eq!(world.levels[connection].inventory.slot(0), Some(&held));
    }

    fn redstone_block() -> i32 {
        bbb_protocol::block_state_types::vanilla_block_default_state_id(
            bbb_protocol::block_types::VANILLA_BLOCK_REDSTONE_BLOCK_ID,
        )
        .unwrap()
    }

    fn powered_oak_shelf() -> i32 {
        crate::fluid::with_property(
            bbb_protocol::block_state_types::vanilla_block_default_state_id(
                bbb_protocol::block_types::VANILLA_BLOCK_OAK_SHELF_ID,
            )
            .unwrap(),
            "powered",
            "true",
        )
        .unwrap()
    }

    #[test]
    fn a_powered_shelf_chain_swaps_the_whole_hotbar() {
        let mut world = ServerWorld::new(crate::tests::datapack_config()).unwrap();
        let (sender, receiver) = crate::tests::mailbox();
        crate::tests::join_loaded(&sender, &receiver, &mut world, 1, "alice", 1);
        let connection = ConnectionId(1);
        let dimension = Java26BuiltinDimension::Overworld;
        // Facing north, so the chain's left end is the east one.
        let shelves = [0, 1, 2].map(|x| BlockPos { x, y: 64, z: 1 });
        for pos in shelves {
            assert!(world.set_block(
                dimension,
                BlockPos { y: 63, ..pos },
                redstone_block(),
                crate::fluid::SET_BLOCK_UPDATE_ALL
            ));
        }
        for pos in shelves {
            assert!(world.set_block(
                dimension,
                pos,
                powered_oak_shelf(),
                crate::fluid::SET_BLOCK_UPDATE_ALL
            ));
        }
        let stack = |count| {
            crate::tests::native_stack(bbb_protocol::packets::ItemStackSummary {
                item_id: Some(bbb_protocol::item_types::VANILLA_ITEM_STONE_ID),
                count,
                component_patch: Default::default(),
            })
        };
        let player = world.levels.player_mut(connection).unwrap();
        for slot in 0..9 {
            player
                .inventory
                .set_inventory_slot(slot, stack(slot as i32 + 1));
        }
        assert!(world.swap_shelf_hotbar(connection, dimension, shelves[1], &mut Vec::new()));
        let held = |world: &ServerWorld, pos| {
            world
                .level(dimension)
                .chunks
                .block_entities()
                .get(pos)
                .and_then(crate::block_entity::BlockEntity::shelf)
                .unwrap()
                .original_items()
                .iter()
                .map(|item| item.summary().count)
                .collect::<Vec<_>>()
        };
        assert_eq!(held(&world, shelves[2]), [1, 2, 3]);
        assert_eq!(held(&world, shelves[1]), [4, 5, 6]);
        assert_eq!(held(&world, shelves[0]), [7, 8, 9]);
        assert!((0..9).all(|slot| world.levels[connection]
            .inventory
            .slot(slot)
            .is_some_and(|item| item.is_empty())));
    }
}
