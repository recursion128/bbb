//! Portal-arrival tickets and synchronous FULL loading.

use crate::world::LevelId;
use std::collections::BTreeSet;

use crate::chunk_sender::ChunkKey;
use bbb_protocol::packets::BlockPos;
use bbb_server_persistence::world_storage::saved_data::JAVA_26_1_TICKET_CHUNK_LIMIT;
use bbb_server_types::save::{Java26BuiltinDimension, SaveResult};

use super::chunk_lifecycle::full_preparation::{
    prepare_worldgen_entity_batch, PreparedWorldgenEntityBatch,
};
use super::{SavedChunkTicket, ServerWorld, CHUNK_FULL_LEVEL};

const PORTAL_TICKET_RADIUS: i32 = 3;
const PORTAL_TICKET_LEVEL: i32 = CHUNK_FULL_LEVEL - PORTAL_TICKET_RADIUS;
const PORTAL_TICKET_TIMEOUT: i64 = 300;

pub(super) struct PreparedPortalTicket {
    dimension: LevelId,
    worldgen_entities: Option<PreparedWorldgenEntityBatch>,
    tickets: Vec<SavedChunkTicket>,
    reach: Vec<ChunkKey>,
}

fn invalid_ticket(detail: impl Into<String>) -> bbb_server_types::save::SaveError {
    bbb_server_types::schema_support::invalid_schema("portal_ticket", detail)
}

fn checked_ticket_chunk(ticket_position: BlockPos) -> SaveResult<ChunkKey> {
    let chunk = (ticket_position.x >> 4, ticket_position.z >> 4);
    if !JAVA_26_1_TICKET_CHUNK_LIMIT.contains(&chunk.0)
        || !JAVA_26_1_TICKET_CHUNK_LIMIT.contains(&chunk.1)
    {
        return Err(invalid_ticket(format!(
            "chunk coordinates must be in {}..={}",
            JAVA_26_1_TICKET_CHUNK_LIMIT.start(),
            JAVA_26_1_TICKET_CHUNK_LIMIT.end()
        )));
    }
    Ok(chunk)
}

fn full_reach(chunk: ChunkKey) -> SaveResult<Vec<ChunkKey>> {
    let mut reach = Vec::with_capacity(((PORTAL_TICKET_RADIUS * 2 + 1).pow(2)) as usize);
    for dx in -PORTAL_TICKET_RADIUS..=PORTAL_TICKET_RADIUS {
        for dz in -PORTAL_TICKET_RADIUS..=PORTAL_TICKET_RADIUS {
            let x = chunk
                .0
                .checked_add(dx)
                .ok_or_else(|| invalid_ticket("FULL reach overflows the chunk x coordinate"))?;
            let z = chunk
                .1
                .checked_add(dz)
                .ok_or_else(|| invalid_ticket("FULL reach overflows the chunk z coordinate"))?;
            reach.push((x, z));
        }
    }
    Ok(reach)
}

impl ServerWorld {
    pub(super) fn prepare_portal_ticket_by_key(
        &mut self,
        dimension: &LevelId,
        ticket_position: BlockPos,
        additional_full_chunks: &[ChunkKey],
    ) -> SaveResult<PreparedPortalTicket> {
        self.prepare_portal_tickets_by_key(dimension, &[ticket_position], additional_full_chunks)
    }

    pub(super) fn prepare_portal_tickets_by_key(
        &mut self,
        dimension: &LevelId,
        ticket_positions: &[BlockPos],
        additional_full_chunks: &[ChunkKey],
    ) -> SaveResult<PreparedPortalTicket> {
        if !self.has_level_id(dimension) {
            return Err(invalid_ticket(format!(
                "dimension {} is not served",
                self.level_key(*dimension).resource_id()
            )));
        }
        let ticket_chunks = ticket_positions
            .iter()
            .copied()
            .map(checked_ticket_chunk)
            .collect::<SaveResult<Vec<_>>>()?;
        let mut tickets = self.level_by_id(dimension).saved_chunk_tickets.clone();
        for ticket in &tickets {
            if !JAVA_26_1_TICKET_CHUNK_LIMIT.contains(&ticket.chunk.0)
                || !JAVA_26_1_TICKET_CHUNK_LIMIT.contains(&ticket.chunk.1)
            {
                return Err(invalid_ticket(
                    "an existing ticket is outside the ticket domain",
                ));
            }
        }
        let mut reach = BTreeSet::new();
        for &chunk in &ticket_chunks {
            reach.extend(full_reach(chunk)?);
        }
        let reach = reach.into_iter().collect::<Vec<_>>();
        let full_chunks = additional_full_chunks
            .iter()
            .copied()
            .chain(reach.iter().copied())
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect::<Vec<_>>();
        self.generate_original_mob_columns(dimension, &full_chunks)?;
        let input = self.entity_nbt_read_snapshot();
        let worldgen_entities = prepare_worldgen_entity_batch(
            &mut self.level_mut_by_id(dimension).chunks,
            &full_chunks,
            input,
        )?;
        for chunk in ticket_chunks {
            if let Some(ticket) = tickets.iter_mut().find(|ticket| {
                ticket.chunk == chunk
                    && ticket.level == PORTAL_TICKET_LEVEL
                    && ticket.ticks_left.is_some()
            }) {
                ticket.ticks_left = Some(PORTAL_TICKET_TIMEOUT);
            } else {
                tickets.push(SavedChunkTicket {
                    chunk,
                    level: PORTAL_TICKET_LEVEL,
                    ticks_left: Some(PORTAL_TICKET_TIMEOUT),
                });
            }
        }
        Ok(PreparedPortalTicket {
            dimension: *dimension,
            worldgen_entities: Some(worldgen_entities),
            tickets,
            reach,
        })
    }

    pub(super) fn commit_portal_ticket(&mut self, prepared: PreparedPortalTicket) {
        self.commit_portal_ticket_state(prepared);
    }

    pub(super) fn publish_portal_destination(
        &mut self,
        mut prepared: PreparedPortalTicket,
    ) -> PreparedPortalTicket {
        self.install_portal_worldgen_entities(&mut prepared);
        prepared
    }

    fn install_portal_worldgen_entities(&mut self, prepared: &mut PreparedPortalTicket) {
        if let Some(entities) = prepared.worldgen_entities.take() {
            self.install_worldgen_entities(&prepared.dimension, entities);
        }
    }

    pub(super) fn commit_portal_ticket_state(&mut self, mut prepared: PreparedPortalTicket) {
        self.install_portal_worldgen_entities(&mut prepared);
        let PreparedPortalTicket {
            dimension,
            tickets,
            reach,
            ..
        } = prepared;
        let level = self.level_mut_by_id(&dimension);
        level.saved_chunk_tickets = tickets;
        if let Some(builtin) = dimension.builtin() {
            self.refresh_simulation_chunk_levels(builtin);
            for chunk in reach {
                self.register_chunk_tick_containers_by_key(&dimension, chunk);
            }
        } else {
            self.refresh_simulation_chunk_levels_by_key(&dimension);
            for chunk in reach {
                self.register_chunk_tick_containers_by_key(&dimension, chunk);
            }
        }
    }

    /// `Entity.placePortalTicket` (`Entity.java:3102-3104`) files a radius-three
    /// `TicketType.PORTAL`; `TicketStorage.addTicketWithRadius`
    /// (`TicketStorage.java:138-140`) maps that to FULL level 33 minus three.
    pub fn place_portal_ticket(
        &mut self,
        dimension: Java26BuiltinDimension,
        ticket_position: BlockPos,
    ) -> SaveResult<()> {
        if !self.has_level(dimension) {
            return Err(invalid_ticket(format!(
                "dimension {} is not served",
                self.level_key(dimension.into()).resource_id()
            )));
        }
        let dimension = LevelId::from(dimension);
        let prepared = self.prepare_portal_ticket_by_key(&dimension, ticket_position, &[])?;
        self.commit_portal_ticket(prepared);
        Ok(())
    }

    #[cfg(test)]
    pub(crate) fn saved_chunk_tickets_for_test(
        &self,
        dimension: Java26BuiltinDimension,
    ) -> Vec<(ChunkKey, i32, Option<i64>)> {
        self.level(dimension)
            .saved_chunk_tickets
            .iter()
            .map(|ticket| (ticket.chunk, ticket.level, ticket.ticks_left))
            .collect()
    }
}

impl ServerWorld {
    pub(in crate::world) fn publish_full_columns(
        &mut self,
        dimension: &LevelId,
        columns: &[(i32, i32)],
    ) -> SaveResult<()> {
        self.level_mut_by_id(dimension)
            .chunks
            .flush_live_relights()?;
        let prepared = self.prepare_portal_tickets_by_key(dimension, &[], columns)?;
        // Synchronous searches publish FULL chunks and original mobs, without a portal ticket.
        let _ = self.publish_portal_destination(prepared);
        Ok(())
    }
}
