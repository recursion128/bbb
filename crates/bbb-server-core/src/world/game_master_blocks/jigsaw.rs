use crate::block_entity::BlockEntity;
use crate::connection::ConnectionId;
use bbb_protocol::packets::{Direction, JigsawGenerate, SetJigsawBlock};

use crate::world::ServerWorld;

impl ServerWorld {
    // JigsawBlockEntity.java:139-144; ServerGamePacketListenerImpl.java:951-958.
    pub(in crate::world) fn apply_jigsaw_generate(
        &mut self,
        connection_id: ConnectionId,
        packet: JigsawGenerate,
    ) {
        if !self.can_use_game_master_blocks(connection_id) {
            return;
        }
        let dimension = self.levels[connection_id].dimension;
        let Some(entity) = self
            .level_by_id(&dimension)
            .chunks
            .block_entities()
            .get(packet.pos)
        else {
            return;
        };
        let Some(jigsaw) = entity.jigsaw() else {
            return;
        };
        let orientation =
            crate::block_behavior::string_property(entity.block_state(), "orientation");
        let direction = match orientation
            .as_deref()
            .and_then(|value| value.split('_').next())
        {
            Some("down") => Direction::Down,
            Some("up") => Direction::Up,
            Some("south") => Direction::South,
            Some("west") => Direction::West,
            Some("east") => Direction::East,
            _ => Direction::North,
        };
        let pool = jigsaw.pool().to_owned();
        let target = jigsaw.target().to_owned();
        let origin = bbb_server_types::directions::relative(packet.pos, direction);
        let _ = self.generate_live_jigsaw(
            dimension,
            &pool,
            &target,
            packet.levels,
            origin,
            packet.keep_jigsaws,
        );
    }

    // ServerGamePacketListenerImpl.java:931-946: permission and actual block entity type precede edits.
    pub(in crate::world) fn apply_set_jigsaw_block(
        &mut self,
        connection_id: ConnectionId,
        packet: SetJigsawBlock,
    ) {
        if !self.can_use_game_master_blocks(connection_id) {
            return;
        }
        let dimension = self.levels[connection_id].dimension;
        let level = self.level_mut_by_id(&dimension);
        let Some(jigsaw) = level
            .chunks
            .block_entities_mut()
            .get_mut(packet.pos)
            .and_then(BlockEntity::jigsaw_mut)
        else {
            return;
        };
        jigsaw.apply_edit(&packet);
        level.chunks.block_entities_mut().set_changed(packet.pos);
        level
            .changed_block_broadcasts
            .insert((packet.pos.x, packet.pos.y, packet.pos.z));
    }
}
