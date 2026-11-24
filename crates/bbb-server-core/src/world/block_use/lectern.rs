//! `LecternBlock` (`LecternBlock.java`): the book a click puts on the
//! lectern, and the menu a click on a stocked one opens.

use crate::world::LevelId;
use bbb_protocol::entity_anchors::EntityAnchor;
use bbb_protocol::packets::{
    BlockPos, InteractionHand, OpenScreen, SoundEvent, SoundEventHolder, SoundSource, Vec3d,
};
use bbb_protocol::sound_event_types::vanilla_sound_event_registry_id;
use bbb_protocol::RichTextComponent;
use bbb_server_types::item_stack::RuntimeItemStack;

use super::BlockUseContext;
use crate::block_behavior::lectern as lectern_block;
use crate::block_behavior::BlockUseResult;
use crate::block_entity::{lectern, BlockEntity};
use crate::commands::{CommandOrigin, CommandPermission, CommandSource};
use crate::connection::{ClientboundPlayPacket, ConnectionId};
use crate::inventory::{ContainerAnchor, MenuLayout};
use crate::world::using_item::runtime_item_in_hand;
use crate::world::{chest_menu_title, send, ServerWorld, WorldEvent};

/// The `item_on:` column of the `block_use_dispatch!` row:
/// [`ServerWorld::use_lectern_item_on`].
pub(super) fn use_item_on(
    world: &mut ServerWorld,
    context: &BlockUseContext<'_>,
    events: &mut Vec<WorldEvent>,
) -> BlockUseResult {
    world.use_lectern_item_on(
        context.connection_id,
        context.dimension,
        context.update.hit.pos,
        context.block_state_id,
        context.update.hand,
        events,
    )
}

/// The `without_item:` column of the `block_use_dispatch!` row:
/// [`ServerWorld::use_lectern_without_item`].
pub(super) fn use_without_item(
    world: &mut ServerWorld,
    context: &BlockUseContext<'_>,
    events: &mut Vec<WorldEvent>,
) -> bool {
    world.use_lectern_without_item(
        context.connection_id,
        context.dimension,
        context.update.hit.pos,
        context.block_state_id,
        events,
    )
}

/// The `spectator_menu:` column of the `block_use_dispatch!` row:
/// [`ServerWorld::open_lectern_menu`].
pub(super) fn open_menu(
    world: &mut ServerWorld,
    context: &BlockUseContext<'_>,
    events: &mut Vec<WorldEvent>,
) -> bool {
    world.open_lectern_menu(
        context.connection_id,
        context.dimension,
        context.update.hit.pos,
        events,
    )
}

impl ServerWorld {
    /// `LecternBlock.useItemOn` (`LecternBlock.java:216-232`).
    pub(super) fn use_lectern_item_on(
        &mut self,
        connection_id: ConnectionId,
        dimension: impl Into<LevelId>,
        pos: BlockPos,
        block_state_id: i32,
        hand: InteractionHand,
        events: &mut Vec<WorldEvent>,
    ) -> BlockUseResult {
        let dimension = dimension.into();
        if lectern_block::has_book(block_state_id) {
            return BlockUseResult::TryWithEmptyHand;
        }
        if lectern::item_is_lectern_book(
            runtime_item_in_hand(self.levels.player_at(connection_id), hand),
            self.lectern_book_tags(dimension),
        ) {
            if self.place_lectern_book(connection_id, dimension, pos, block_state_id, hand, events)
            {
                return BlockUseResult::Consumed;
            }
            return BlockUseResult::Pass;
        }
        if runtime_item_in_hand(self.levels.player_at(connection_id), hand).is_empty()
            && hand == InteractionHand::MainHand
        {
            return BlockUseResult::Pass;
        }
        BlockUseResult::TryWithEmptyHand
    }

    fn lectern_book_tags(&self, dimension: LevelId) -> &crate::registry_tags::RegistryTagSnapshot {
        self.level_by_id(&dimension)
            .chunks
            .registry_tags()
            .unwrap_or_else(|| crate::registry_tags::RegistryTagSnapshot::builtin())
    }

    /// `LecternBlock.tryPlaceBook` (`LecternBlock.java:126-137`) on the server.
    pub(super) fn place_lectern_book(
        &mut self,
        connection_id: ConnectionId,
        dimension: impl Into<LevelId>,
        pos: BlockPos,
        block_state_id: i32,
        hand: InteractionHand,
        events: &mut Vec<WorldEvent>,
    ) -> bool {
        let dimension = dimension.into();
        let player = self
            .levels
            .player(connection_id)
            .expect("connection presence was checked");
        let held = runtime_item_in_hand(player, hand);
        if !lectern::item_is_lectern_book(held, self.lectern_book_tags(dimension)) {
            return false;
        }
        let source_entity_id = player.entity_id();
        let book = held.with_count(1);
        let book = self
            .resolve_lectern_book(dimension, pos, &book, Some(connection_id))
            .unwrap_or(book);
        let Some(lectern) = self
            .level_mut_by_id(&dimension)
            .chunks
            .block_entities_mut()
            .get_mut(pos)
            .and_then(BlockEntity::lectern_mut)
        else {
            return false;
        };
        if let Err(error) = lectern.set_book(book) {
            tracing::warn!("lectern book placement: {error}");
            return false;
        }
        let player = self
            .levels
            .player_mut(connection_id)
            .expect("connection presence was checked");
        player
            .inventory
            .consume_in_hand(hand, 1, player.abilities.instabuild);
        self.level_mut_by_id(&dimension)
            .chunks
            .block_entities_mut()
            .set_changed(pos);
        self.with_level(dimension, |level| {
            lectern_block::reset_book_state(
                level,
                pos,
                block_state_id,
                Some(source_entity_id),
                true,
            );
        });
        self.play_lectern_put_sound(dimension, pos, events);
        true
    }

    /// `LecternBlock.useWithoutItem` (`LecternBlock.java:235-247`).
    ///
    /// The counter sits inside `openScreen` (`:254-260`) rather than beside the
    /// menu here, but the distinction that matters is the same one every other
    /// opener has: a spectator reaches the menu through `getMenuProvider`
    /// (`:250-252`) and `ServerPlayerGameMode.useItemOn`
    /// (`ServerPlayerGameMode.java:358-365`), which runs no `openScreen` and
    /// counts nothing.
    pub(super) fn use_lectern_without_item(
        &mut self,
        connection_id: ConnectionId,
        dimension: impl Into<LevelId>,
        pos: BlockPos,
        block_state_id: i32,
        events: &mut Vec<WorldEvent>,
    ) -> bool {
        let dimension = dimension.into();
        if lectern_block::has_book(block_state_id)
            && self.open_lectern_menu(connection_id, dimension, pos, events)
        {
            // `:258`, behind the `instanceof LecternBlockEntity` at `:256`.
            self.levels[connection_id].award_stat_once(
                &mut self.scoreboard,
                bbb_server_types::stats::custom_stat("minecraft:interact_with_lectern"),
            );
        }
        true
    }

    /// `LecternBlock.openScreen` (`LecternBlock.java:254-260`).
    pub(super) fn open_lectern_menu(
        &mut self,
        connection_id: ConnectionId,
        dimension: impl Into<LevelId>,
        pos: BlockPos,
        events: &mut Vec<WorldEvent>,
    ) -> bool {
        let resolver = self.item_stack_resolver();
        let dimension = dimension.into();
        let Some(block_state_id) = self.block_state(dimension, pos) else {
            return false;
        };
        if !lectern_block::has_book(block_state_id) {
            return false;
        }
        let Some(book) = self
            .level_by_id(&dimension)
            .chunks
            .block_entities()
            .get(pos)
            .and_then(BlockEntity::lectern)
            .map(lectern::LecternBlockEntity::original_book)
        else {
            return false;
        };
        // The `loadAdditional` resolution (`LecternBlockEntity.java:206`) of a
        // book that loaded unresolved, before anyone reads it.
        if let Some(resolved) = self.resolve_lectern_book(dimension, pos, &book, None) {
            if let Some(lectern) = self
                .level_mut_by_id(&dimension)
                .chunks
                .block_entities_mut()
                .get_mut(pos)
                .and_then(BlockEntity::lectern_mut)
            {
                lectern.set_resolved_book(resolved);
            }
            self.level_mut_by_id(&dimension)
                .chunks
                .block_entities_mut()
                .set_changed(pos);
        }
        // ServerPlayer.java:1324-1329: close and reserve precede provider admission.
        let Some(instances) = self.capture_container_instances(dimension, &[pos]) else {
            return false;
        };
        let Some(container_id) = self.begin_container_open(connection_id, events) else {
            return false;
        };
        let prepared = (|| {
            let body =
                instances[0].resolve_instance(self.level_by_id(&dimension).chunks.block_entities());
            let Some(lectern) = body.lectern() else {
                return Ok(None);
            };
            let book = lectern.original_book();
            let inventory = &mut self.levels[connection_id].inventory;
            let Some(opened) = inventory.prepare_reserved_menu(
                &resolver,
                None,
                container_id,
                MenuLayout::Lectern,
                ContainerAnchor {
                    positions: vec![pos],
                },
                vec![book],
            )?
            else {
                return Ok(None);
            };
            Ok::<_, bbb_server_types::ServerWorldError>(Some(opened))
        })();
        let opened = match prepared {
            Ok(Some(prepared)) => prepared,
            Ok(None) => return false,
            Err(error) => {
                tracing::warn!("lectern menu preparation: {error}");
                return false;
            }
        };
        let Some(title) = chest_menu_title("container.lectern") else {
            return false;
        };
        self.levels[connection_id]
            .inventory
            .bind_container_instances(instances);
        events.push(send(
            connection_id,
            ClientboundPlayPacket::OpenScreen(OpenScreen {
                container_id: opened.container_id,
                menu_type_id: opened.menu_type_id,
                title,
            }),
        ));
        self.initialize_player_menu(connection_id, events);
        true
    }

    pub(super) fn play_lectern_put_sound(
        &mut self,
        dimension: impl Into<LevelId>,
        pos: BlockPos,
        events: &mut Vec<WorldEvent>,
    ) {
        let dimension = dimension.into();
        let Some(registry_id) = vanilla_sound_event_registry_id(lectern_block::BOOK_PUT_SOUND)
        else {
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

    /// `LecternBlockEntity.resolveBook` (`LecternBlockEntity.java:177-184`)
    /// with `createCommandSourceStack` (`:186-200`): the player, or `Lectern`,
    /// at the lectern's centre with gamemaster permission.
    pub(in crate::world) fn resolve_lectern_book(
        &mut self,
        dimension: LevelId,
        pos: BlockPos,
        book: &RuntimeItemStack,
        player: Option<ConnectionId>,
    ) -> Option<RuntimeItemStack> {
        let (source, recipient) = match player {
            Some(connection_id) => {
                let profile = &self.levels[connection_id].profile;
                (
                    CommandSource::player(
                        &profile.name,
                        profile.uuid,
                        CommandPermission::GameMaster,
                    ),
                    Some(profile.uuid),
                )
            }
            None => (
                CommandSource::block(RichTextComponent::plain("Lectern")),
                None,
            ),
        };
        let origin = CommandOrigin {
            position: [
                f64::from(pos.x) + 0.5,
                f64::from(pos.y) + 0.5,
                f64::from(pos.z) + 0.5,
            ],
            rotation: [0.0, 0.0],
            level: self.level_key(dimension).clone(),
            anchor: EntityAnchor::Feet,
            eye_height: None,
        };
        self.resolve_written_book_stack(book, &source, &origin, recipient)
    }
}
