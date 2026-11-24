//! The gamemaster F3+I readouts: block entity and entity tag queries.

use crate::connection::{ClientboundPlayPacket, ConnectionId};
use crate::entity::EntityCommandDataError;
use bbb_protocol::packets::{BlockEntityTagQuery, EntityTagQuery, TagQuery};
use bbb_protocol::{NbtSummaryValue, RichTextComponent};

use crate::commands::CommandPermission;

use super::{send, ServerWorld, WorldEvent};

impl ServerWorld {
    /// `ServerGamePacketListenerImpl.handleBlockEntityTagQuery` (:1043-1050):
    /// a position without a block entity is still answered, with the null tag.
    pub(super) fn answer_block_entity_tag_query(
        &self,
        connection_id: ConnectionId,
        packet: BlockEntityTagQuery,
        events: &mut Vec<WorldEvent>,
    ) {
        if self.player_permission(connection_id) < CommandPermission::GameMaster {
            return;
        }
        let dimension = self.player_level_id(connection_id);
        let tag = self
            .level_by_id(&dimension)
            .chunks
            .block_entities()
            .get(packet.pos)
            .map(|block_entity| {
                block_entity.save_without_metadata_with_resolver(&self.item_stack_resolver())
            });
        let tag = match tag.transpose() {
            Ok(tag) => tag,
            Err(error) => {
                events.push(WorldEvent::ConsoleMessage(RichTextComponent::plain(
                    format!("block entity tag query at {:?} failed: {error}", packet.pos,),
                )));
                return;
            }
        };
        send_tag_query(connection_id, packet.transaction_id, tag.as_ref(), events);
    }

    /// `ServerGamePacketListenerImpl.handleEntityTagQuery` (:1017-1030): the id
    /// is looked up in the asking player's own level, and an id nothing there
    /// answers to gets no reply at all.
    pub(super) fn answer_entity_tag_query(
        &self,
        connection_id: ConnectionId,
        packet: EntityTagQuery,
        events: &mut Vec<WorldEvent>,
    ) {
        if self.player_permission(connection_id) < CommandPermission::GameMaster {
            return;
        }
        let dimension = self.player_level_id(connection_id);
        let session = self
            .connection_by_entity_id(packet.entity_id)
            .and_then(|target| self.levels.player(target))
            .filter(|target| target.dimension == dimension);
        let tag = match session {
            Some(target) => {
                match self.player_save_without_id(target, &self.item_stack_resolver()) {
                    Ok(entries) => NbtSummaryValue::Compound(entries),
                    Err(error) => {
                        events.push(WorldEvent::ConsoleMessage(RichTextComponent::plain(
                            format!("player entity tag query failed: {error}"),
                        )));
                        return;
                    }
                }
            }
            None => match self
                .level_by_id(&dimension)
                .entities
                .command_data(packet.entity_id)
            {
                Ok(tag) => tag,
                Err(EntityCommandDataError::UnknownEntity(_)) => return,
                Err(error) => {
                    events.push(WorldEvent::ConsoleMessage(RichTextComponent::plain(
                        format!(
                            "entity tag query for entity {} failed: {error}",
                            packet.entity_id
                        ),
                    )));
                    return;
                }
            },
        };
        send_tag_query(connection_id, packet.transaction_id, Some(&tag), events);
    }
}

fn send_tag_query(
    connection_id: ConnectionId,
    transaction_id: i32,
    tag: Option<&NbtSummaryValue>,
    events: &mut Vec<WorldEvent>,
) {
    match TagQuery::from_tag(transaction_id, tag) {
        Ok(update) => events.push(send(connection_id, ClientboundPlayPacket::TagQuery(update))),
        Err(error) => events.push(WorldEvent::ConsoleMessage(RichTextComponent::plain(
            format!("tag query {transaction_id} could not be written: {error}"),
        ))),
    }
}
