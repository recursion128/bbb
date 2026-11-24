use crate::world::LevelId;
use std::collections::BTreeMap;
use std::sync::Arc;

use crate::block_entity::block_entity_update_packet_with_resolver;
use bbb_protocol::packets::{BlockPos, BlockUpdate, SectionBlocksUpdate};
use bbb_protocol::RichTextComponent;
use bbb_server_types::save::SaveError;

use super::player_tick::is_chunk_tracked;
use super::{ClientboundPlayPacket, ServerWorld, WorldEvent};

impl ServerWorld {
    pub(super) fn flush_level_light(
        &mut self,
        dimension: &LevelId,
        barrier: &str,
        events: &mut Vec<WorldEvent>,
    ) -> bool {
        // BBB uses a synchronous owner barrier before light consumers; Vanilla's
        // threaded light task scheduling does not imply that same guarantee.
        match self.level_mut_by_id(dimension).chunks.flush_live_relights() {
            Ok(()) => true,
            Err(error) => {
                self.fail_level_light(dimension, barrier, error, events);
                false
            }
        }
    }

    pub(super) fn broadcast_light_updates(
        &mut self,
        dimension: &LevelId,
        events: &mut Vec<WorldEvent>,
    ) -> bool {
        if !self.flush_level_light(dimension, "before-broadcast", events) {
            return false;
        }
        let updates = match self
            .level_mut_by_id(dimension)
            .chunks
            .take_live_light_updates_already_flushed()
        {
            Ok(updates) => updates,
            Err(error) => {
                self.fail_level_light(dimension, "packet-projection", error, events);
                return false;
            }
        };
        if updates.is_empty() || self.level_by_id(dimension).player_order.is_empty() {
            return true;
        }
        // ChunkHolder.java:174-191: light precedes blocks, including frozen ticks.
        for update in updates {
            let chunk = (update.chunk_x, update.chunk_z);
            let packet = ClientboundPlayPacket::LightUpdate(Arc::new(update));
            let mut recipients = Vec::new();
            for &connection_id in &self.level_by_id(dimension).player_order {
                if !is_chunk_tracked(&self.sessions, connection_id, chunk) {
                    continue;
                }
                // ChunkMap.java:227-245: only the already-sent tracked border.
                let border = (-1..=1).any(|dx| {
                    (-1..=1).any(|dz| {
                        (dx != 0 || dz != 0)
                            && !is_chunk_tracked(
                                &self.sessions,
                                connection_id,
                                (chunk.0.wrapping_add(dx), chunk.1.wrapping_add(dz)),
                            )
                    })
                });
                if border {
                    recipients.push(connection_id);
                }
            }
            crate::world::player_packets::broadcast_to(recipients, packet, events);
        }
        true
    }

    /// `ChunkHolder.broadcastChanges` for the positions
    /// `ServerLevel.sendBlockUpdated` marked (`ServerLevel.java:1119-1123`),
    /// drained where `ServerChunkCache.broadcastChangedChunks` drains them
    /// (`ServerChunkCache.java:356-368`): one section's single change is a
    /// `ClientboundBlockUpdatePacket`, several are one
    /// `ClientboundSectionBlocksUpdatePacket` (`ChunkHolder.java:200-210`).
    pub(super) fn broadcast_changed_blocks(
        &mut self,
        dimension: &LevelId,
        events: &mut Vec<WorldEvent>,
    ) {
        if self
            .level_by_id(dimension)
            .changed_block_broadcasts
            .is_empty()
        {
            return;
        }
        if self.level_by_id(dimension).player_order.is_empty() {
            self.level_mut_by_id(dimension)
                .changed_block_broadcasts
                .clear();
            return;
        }
        let broadcasts =
            std::mem::take(&mut self.level_mut_by_id(dimension).changed_block_broadcasts);
        let mut sections: BTreeMap<(i32, i32, i32), Vec<BlockUpdate>> = BTreeMap::new();
        for (x, y, z) in broadcasts {
            let Some(block_state_id) = self.level_by_id(dimension).chunks.get_block_state(x, y, z)
            else {
                continue;
            };
            sections
                .entry((x >> 4, y >> 4, z >> 4))
                .or_default()
                .push(BlockUpdate {
                    pos: BlockPos { x, y, z },
                    block_state_id,
                });
        }
        for ((section_x, section_y, section_z), updates) in sections {
            let chunk = (section_x, section_z);
            // `ChunkHolder.broadcast` reaches `ChunkMap.getPlayers(pos, ..)`,
            // which walks the level's own player tracker: a session watching
            // another dimension is not a recipient however close its
            // coordinates are.
            let recipients: Vec<_> = self
                .level_by_id(dimension)
                .player_order
                .iter()
                .copied()
                .filter(|&connection_id| {
                    self.sessions
                        .get(&connection_id)
                        .and_then(|session| session.chunk_tracking_view)
                        .is_some_and(|view| view.contains(chunk))
                })
                .collect();
            if recipients.is_empty() {
                continue;
            }
            let positions: Vec<_> = updates.iter().map(|update| update.pos).collect();
            let packet = if let [update] = updates[..] {
                ClientboundPlayPacket::BlockUpdate(update)
            } else {
                ClientboundPlayPacket::SectionBlocksUpdate(SectionBlocksUpdate {
                    section_x,
                    section_y,
                    section_z,
                    updates,
                })
            };
            crate::world::player_packets::broadcast_to(recipients.iter().copied(), packet, events);
            // `broadcastBlockEntityIfNeeded` (`ChunkHolder.java:220-223`) rides
            // the same recipient list behind the block packet.
            for pos in positions {
                if let Some(packet) = self.block_entity_update_packet(dimension, pos) {
                    crate::world::player_packets::broadcast_to(
                        recipients.iter().copied(),
                        ClientboundPlayPacket::BlockEntityData(packet),
                        events,
                    );
                }
            }
        }
    }

    fn block_entity_update_packet(
        &self,
        dimension: &LevelId,
        pos: BlockPos,
    ) -> Option<bbb_protocol::packets::BlockEntityData> {
        let entity = self
            .level_by_id(dimension)
            .chunks
            .block_entities()
            .get(pos)?;
        match block_entity_update_packet_with_resolver(entity, &self.item_stack_resolver()) {
            Ok(packet) => packet,
            Err(error) => {
                tracing::warn!("block-entity update: {error}");
                None
            }
        }
    }

    fn fail_level_light(
        &mut self,
        dimension: &LevelId,
        barrier: &str,
        error: SaveError,
        events: &mut Vec<WorldEvent>,
    ) {
        self.light_failed = true;
        events.push(WorldEvent::ConsoleMessage(RichTextComponent::plain(
            format!(
                "live light failed: dimension={} barrier={barrier}: {error}",
                self.level_key(*dimension).resource_id(),
            ),
        )));
        self.shutdown(events);
        events.push(WorldEvent::StopRequested);
    }
}

#[cfg(test)]
mod tests {
    use super::super::ServerWorldConfig;
    use super::*;
    use crate::chunk::ChunkStore;
    use bbb_protocol::block_state_types::vanilla_block_state_id_by_name_and_properties;
    use bbb_protocol::packets::BlockPos;
    use bbb_server_types::save::Java26BuiltinDimension;
    use std::collections::BTreeMap;

    const POS: BlockPos = BlockPos { x: 8, y: 319, z: 8 };

    fn full_store() -> ChunkStore {
        let mut chunks = ChunkStore::new_overworld_carvers_with_structure_generation(
            crate::chunk::noise_generator::overworld_program(true, 0),
            false,
        )
        .unwrap();
        chunks.try_ensure_served_full_column(0, 0).unwrap();
        chunks
    }

    fn glowstone() -> i32 {
        vanilla_block_state_id_by_name_and_properties("minecraft:glowstone", &BTreeMap::new())
            .unwrap()
    }

    fn write_light(chunks: &mut ChunkStore) {
        assert!(chunks
            .set_live_block_state(POS.x, POS.y, POS.z, glowstone())
            .unwrap()
            .is_some());
        assert_eq!(chunks.block_light_level(POS.x, POS.y, POS.z), 0);
    }

    fn world_with(chunks: ChunkStore) -> ServerWorld {
        let mut world = ServerWorld::new(ServerWorldConfig::default()).unwrap();
        world.install_chunk_store_for_test(Java26BuiltinDimension::Overworld, chunks);
        world
    }

    fn packet_light_sample(mask: &[i64], updates: &[Vec<u8>], min_y: i32, pos: BlockPos) -> u8 {
        let section = usize::try_from(pos.y.div_euclid(16) - min_y.div_euclid(16) + 1).unwrap();
        let present = |bit: usize| {
            mask.get(bit / 64)
                .is_some_and(|word| (*word as u64) & (1_u64 << (bit % 64)) != 0)
        };
        assert!(
            present(section),
            "sampled light section must be transmitted"
        );
        let array = &updates[(0..section).filter(|&bit| present(bit)).count()];
        let index = usize::try_from(
            pos.y.rem_euclid(16) * 256 + pos.z.rem_euclid(16) * 16 + pos.x.rem_euclid(16),
        )
        .unwrap();
        (array[index / 2] >> ((index % 2) * 4)) & 15
    }

    fn assert_chunk_wire_roundtrip(packet: &bbb_protocol::packets::LevelChunkWithLight) {
        use bbb_protocol::packets::{
            decode_play_clientbound, encode_play_clientbound_level_chunk_with_light,
            PlayClientbound,
        };
        let (id, payload) = encode_play_clientbound_level_chunk_with_light(packet).unwrap();
        assert_eq!(
            decode_play_clientbound(id, &payload).unwrap(),
            PlayClientbound::LevelChunkWithLight(packet.clone())
        );
    }

    #[test]
    fn ordinary_chunk_delivery_uses_the_dimension_skylight_contract() {
        use crate::world_creation::default_overworld_level_source;
        use bbb_server_datapack::debug_generator::DebugLevelSource;
        let stone =
            vanilla_block_state_id_by_name_and_properties("minecraft:stone", &BTreeMap::new())
                .unwrap();
        let roof = BlockPos { x: 8, y: 100, z: 8 };
        let lamp = BlockPos { y: 98, ..roof };
        for has_skylight in [false, true] {
            let flat = ChunkStore::new_with_biome_registry_and_skylight(
                default_overworld_level_source().unwrap(),
                0,
                bbb_server_types::chunk_data::VANILLA_26_1_BIOME_REGISTRY_LEN,
                has_skylight,
            );
            let debug = ChunkStore::new_debug_with_biome_registry_and_skylight(
                DebugLevelSource::new(-64, 384).unwrap(),
                0,
                bbb_server_types::chunk_data::VANILLA_26_1_BIOME_REGISTRY_LEN,
                has_skylight,
            );
            for mut chunks in [flat, debug] {
                chunks.try_ensure_served_full_column(0, 0).unwrap();
                chunks
                    .set_live_block_state(roof.x, roof.y, roof.z, stone)
                    .unwrap();
                chunks
                    .set_live_block_state(lamp.x, lamp.y, lamp.z, glowstone())
                    .unwrap();
                chunks.flush_live_relights().unwrap();
                let packet = ServerWorld::encode_chunk(
                    &chunks,
                    (0, 0),
                    crate::world::registry_runtime::item_stack_resolver_for_test(),
                )
                .unwrap();
                let light = &packet.light_data;
                assert_eq!(
                    packet_light_sample(
                        &light.block_y_mask,
                        &light.block_updates,
                        chunks.min_y(),
                        lamp
                    ),
                    15
                );
                assert_eq!(
                    packet_light_sample(
                        &light.block_y_mask,
                        &light.block_updates,
                        chunks.min_y(),
                        BlockPos { y: 99, ..lamp }
                    ),
                    14
                );
                if has_skylight {
                    assert_eq!(
                        packet_light_sample(
                            &light.sky_y_mask,
                            &light.sky_updates,
                            chunks.min_y(),
                            roof
                        ),
                        0
                    );
                    assert_eq!(
                        packet_light_sample(
                            &light.sky_y_mask,
                            &light.sky_updates,
                            chunks.min_y(),
                            BlockPos { y: 101, ..roof }
                        ),
                        15
                    );
                } else {
                    assert!(light.sky_y_mask.is_empty());
                    assert!(light.empty_sky_y_mask.is_empty());
                    assert!(light.sky_updates.is_empty());
                }
                assert_eq!(packet.chunk_data.heightmaps.len(), 3);
                assert!(!packet.chunk_data.section_data.is_empty());
                assert_chunk_wire_roundtrip(&packet);
            }
        }
    }

    #[test]
    fn ordinary_chunk_delivery_handles_signed_and_multiword_light_masks() {
        let mut chunks = ChunkStore::new_debug_with_biome_registry_and_skylight(
            bbb_server_datapack::debug_generator::DebugLevelSource::new(-2032, 4064).unwrap(),
            0,
            bbb_server_types::chunk_data::VANILLA_26_1_BIOME_REGISTRY_LEN,
            true,
        );
        chunks.try_ensure_served_full_column(0, 0).unwrap();
        let lamps = [
            BlockPos {
                x: 8,
                y: -1040,
                z: 8,
            },
            BlockPos { x: 8, y: 100, z: 8 },
        ];
        for pos in lamps {
            chunks
                .set_live_block_state(pos.x, pos.y, pos.z, glowstone())
                .unwrap();
        }
        chunks.flush_live_relights().unwrap();
        let packet = ServerWorld::encode_chunk(
            &chunks,
            (0, 0),
            crate::world::registry_runtime::item_stack_resolver_for_test(),
        )
        .unwrap();
        assert!(packet.light_data.block_y_mask[0] < 0);
        assert!(packet.light_data.block_y_mask.len() > 1);
        for pos in lamps {
            assert_eq!(
                packet_light_sample(
                    &packet.light_data.block_y_mask,
                    &packet.light_data.block_updates,
                    chunks.min_y(),
                    pos
                ),
                15
            );
        }
        assert_chunk_wire_roundtrip(&packet);
    }

    #[test]
    fn live_relight_tick_without_players_commits_and_drains_even_when_frozen() {
        for frozen in [false, true] {
            let mut chunks = full_store();
            write_light(&mut chunks);
            let mut world = world_with(chunks);
            world.tick_rate_mut().set_frozen(frozen);
            let tick = world.tick(Vec::new());
            assert!(!world.light_failed);
            let chunks = &mut world.level_mut(Java26BuiltinDimension::Overworld).chunks;
            assert_eq!(chunks.block_light_level(POS.x, POS.y, POS.z), 15);
            assert!(chunks.take_live_light_updates().unwrap().is_empty());
            assert!(!tick
                .events
                .iter()
                .any(|event| matches!(event, WorldEvent::StopRequested)));
            assert_eq!(
                world
                    .level_light_facts_for_test(Java26BuiltinDimension::Overworld, POS)
                    .0,
                15
            );
            let missing = BlockPos {
                x: 100_000,
                y: 319,
                z: 100_000,
            };
            let count = world
                .level(Java26BuiltinDimension::Overworld)
                .chunks
                .column_count();
            assert_eq!(
                world
                    .level_light_facts_for_test(Java26BuiltinDimension::Overworld, missing)
                    .0,
                0
            );
            assert_eq!(
                world
                    .level(Java26BuiltinDimension::Overworld)
                    .chunks
                    .column_count(),
                count
            );
            assert_eq!(world.game_time(), i64::from(!frozen));
        }
    }
}
