use crate::block_entity::{structure_block::StructureBlockEntity, BlockEntity};
use crate::connection::ConnectionId;
use crate::world::LevelId;
use bbb_protocol::packets::{
    BlockPos, SetStructureBlock, StructureBlockMode, StructureBlockUpdateKind, SystemChat,
};

use crate::world::{ServerWorld, WorldEvent};

impl ServerWorld {
    // ServerGamePacketListenerImpl.java:819-868: edits precede save/load/detect and feedback.
    pub(in crate::world) fn apply_set_structure_block(
        &mut self,
        connection_id: ConnectionId,
        packet: SetStructureBlock,
        events: &mut Vec<WorldEvent>,
    ) {
        if !self.can_use_game_master_blocks(connection_id) {
            return;
        }
        let dimension = self.levels[connection_id].dimension;
        let Some(block) = self
            .level_mut_by_id(&dimension)
            .chunks
            .block_entities_mut()
            .get_mut(packet.pos)
            .and_then(BlockEntity::structure_block_mut)
        else {
            return;
        };
        block.apply_edit(&packet);
        self.sync_structure_block_mode(&dimension, packet.pos, packet.mode);
        let block = self
            .structure_block_snapshot(&dimension, packet.pos)
            .expect("edited structure block");
        let name = block.name().to_owned();
        let feedback = if name.is_empty() {
            Some(("structure_block.invalid_structure_name", Some(packet.name)))
        } else {
            match packet.update {
                StructureBlockUpdateKind::UpdateData => None,
                StructureBlockUpdateKind::SaveArea => Some((
                    if block.mode() == StructureBlockMode::Save
                        && self.save_structure_block(&dimension, packet.pos, &block, true)
                    {
                        "structure_block.save_success"
                    } else {
                        "structure_block.save_failure"
                    },
                    Some(name),
                )),
                StructureBlockUpdateKind::LoadArea => {
                    let template = if block.mode() == StructureBlockMode::Load {
                        self.structure_block_template(&name)
                    } else {
                        None
                    };
                    Some((
                        if let Some(template) = template {
                            self.update_structure_block_info(&dimension, packet.pos, &template);
                            if template.size == [block.size().x, block.size().y, block.size().z] {
                                self.place_structure_block_template(
                                    dimension,
                                    offset(packet.pos, block.offset()),
                                    &block,
                                );
                                "structure_block.load_success"
                            } else {
                                "structure_block.load_prepare"
                            }
                        } else {
                            "structure_block.load_not_found"
                        },
                        Some(name),
                    ))
                }
                StructureBlockUpdateKind::ScanArea => {
                    let found = self.detect_structure_block_size(&dimension, packet.pos, &block);
                    Some((
                        if found {
                            "structure_block.size_success"
                        } else {
                            "structure_block.size_failure"
                        },
                        found.then_some(name),
                    ))
                }
            }
        };
        if let Some((key, argument)) = feedback {
            let args = argument
                .map(|name| bbb_protocol::component_literal_nbt(&name))
                .into_iter()
                .collect();
            let message = bbb_protocol::component::RichTextComponent::try_from_component_nbt(
                crate::commands::translatable(key, args),
            )
            .expect("structure block feedback component");
            self.send_system_message(
                connection_id,
                SystemChat::from_component(message, false),
                events,
            );
        }
        self.mark_structure_block_changed(&dimension, packet.pos);
    }

    pub(super) fn structure_block_snapshot(
        &self,
        dimension: &LevelId,
        pos: BlockPos,
    ) -> Option<StructureBlockEntity> {
        self.level_by_id(dimension)
            .chunks
            .block_entities()
            .get(pos)?
            .structure_block()
            .cloned()
    }

    pub(super) fn sync_structure_block_mode(
        &mut self,
        dimension: &LevelId,
        pos: BlockPos,
        mode: StructureBlockMode,
    ) {
        let state = crate::world::level_blocks::stored_block_state(
            &self.level_by_id(dimension).chunks,
            pos,
        );
        let name = match mode {
            StructureBlockMode::Save => "save",
            StructureBlockMode::Load => "load",
            StructureBlockMode::Corner => "corner",
            StructureBlockMode::Data => "data",
        };
        if let Some(updated) = crate::fluid::with_property(state, "mode", name) {
            self.set_block(dimension, pos, updated, 2);
        }
    }

    pub(super) fn mark_structure_block_changed(&mut self, dimension: &LevelId, pos: BlockPos) {
        let level = self.level_mut_by_id(dimension);
        level.chunks.block_entities_mut().set_changed(pos);
        level.changed_block_broadcasts.insert((pos.x, pos.y, pos.z));
    }

    fn structure_block_template(
        &self,
        name: &str,
    ) -> Option<bbb_server_datapack::structure_template::StructureTemplate> {
        self.cold
            .structure_templates
            .get(name, self.datapacks().map(|packs| &packs.structures))
    }

    fn update_structure_block_info(
        &mut self,
        dimension: &LevelId,
        pos: BlockPos,
        template: &bbb_server_datapack::structure_template::StructureTemplate,
    ) {
        if let Some(block) = self
            .level_mut_by_id(dimension)
            .chunks
            .block_entities_mut()
            .get_mut(pos)
            .and_then(BlockEntity::structure_block_mut)
        {
            block.set_author(template.author().to_owned());
            block.set_size(BlockPos {
                x: template.size[0],
                y: template.size[1],
                z: template.size[2],
            });
        }
        self.level_mut_by_id(dimension)
            .chunks
            .block_entities_mut()
            .set_changed(pos);
    }

    // StructureBlockEntity.java:264-316: one corner includes this block; multiple corners exclude it.
    fn detect_structure_block_size(
        &mut self,
        dimension: &LevelId,
        pos: BlockPos,
        block: &StructureBlockEntity,
    ) -> bool {
        if block.mode() != StructureBlockMode::Save {
            return false;
        }
        let mut corners = Vec::new();
        for z in ((pos.z - 80) >> 4)..=((pos.z + 80) >> 4) {
            for x in ((pos.x - 80) >> 4)..=((pos.x + 80) >> 4) {
                let _ = self.block_state_by_key(
                    dimension,
                    BlockPos {
                        x: x << 4,
                        y: pos.y,
                        z: z << 4,
                    },
                );
                for entity in self
                    .level_by_id(dimension)
                    .chunks
                    .block_entities()
                    .chunk((x, z))
                {
                    let corner = entity.block_pos();
                    if corner.x < pos.x - 80
                        || corner.x > pos.x + 80
                        || corner.z < pos.z - 80
                        || corner.z > pos.z + 80
                    {
                        continue;
                    }
                    if entity.structure_block().is_some_and(|other| {
                        other.mode() == StructureBlockMode::Corner && other.name() == block.name()
                    }) {
                        corners.push(corner);
                    }
                }
            }
        }
        if corners.is_empty() {
            return false;
        }
        if corners.len() == 1 {
            corners.push(pos);
        }
        let min = BlockPos {
            x: corners.iter().map(|p| p.x).min().unwrap(),
            y: corners.iter().map(|p| p.y).min().unwrap(),
            z: corners.iter().map(|p| p.z).min().unwrap(),
        };
        let max = BlockPos {
            x: corners.iter().map(|p| p.x).max().unwrap(),
            y: corners.iter().map(|p| p.y).max().unwrap(),
            z: corners.iter().map(|p| p.z).max().unwrap(),
        };
        let size = BlockPos {
            x: max.x - min.x - 1,
            y: max.y - min.y - 1,
            z: max.z - min.z - 1,
        };
        if size.x < 1 || size.y < 1 || size.z < 1 {
            return false;
        }
        let actual = self
            .level_mut_by_id(dimension)
            .chunks
            .block_entities_mut()
            .get_mut(pos)
            .and_then(BlockEntity::structure_block_mut)
            .expect("detecting structure block");
        actual.set_offset(BlockPos {
            x: min.x - pos.x + 1,
            y: min.y - pos.y + 1,
            z: min.z - pos.z + 1,
        });
        actual.set_size(size);
        self.mark_structure_block_changed(dimension, pos);
        true
    }
}

pub(super) fn offset(a: BlockPos, b: BlockPos) -> BlockPos {
    BlockPos {
        x: a.x.wrapping_add(b.x),
        y: a.y.wrapping_add(b.y),
        z: a.z.wrapping_add(b.z),
    }
}
