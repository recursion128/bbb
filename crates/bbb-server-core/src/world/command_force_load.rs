//! `ServerLevel.getForceLoadedChunks` and `setChunkForced`, the two halves
//! `/forceload` reads and writes.
//!
//! `TicketStorage` is this world's [`ServerLevel::saved_chunk_tickets`], so a
//! ticket a command files is the same list a save restores, the simulation
//! tracker rebuilds from and the next save writes back.

use crate::world::LevelId;
use bbb_protocol::packets::ChunkPos;

use crate::chunk_sender::ChunkKey;

use super::{SavedChunkTicket, ServerWorld, CHUNK_ENTITY_TICKING_LEVEL};

/// `ChunkMap.FORCED_TICKET_LEVEL` (ChunkMap.java:126), which is
/// `ChunkLevel.byStatus(FullChunkStatus.ENTITY_TICKING)`.
///
/// Every `TicketType.FORCED` ticket a 26.1 server holds carries it:
/// `TicketStorage.updateChunkForced` (TicketStorage.java:383-386) is the only
/// writer, and `replaceTicketLevelOfType` is only ever asked for
/// `PLAYER_SIMULATION` (DistanceManager.java:154).
const FORCED_TICKET_LEVEL: i32 = CHUNK_ENTITY_TICKING_LEVEL;

/// Whether a persisted ticket is a `TicketType.FORCED` one.
///
/// The type itself is not kept (see [`SavedChunkTicket`]) and does not need to
/// be: `TicketStorage.packTickets` writes only the two types that persist, and
/// of those only `TicketType.PORTAL` has a timeout (`TicketType.java:22-23`),
/// so a ticket with no `ticks_left` is a `FORCED` one and nothing else.
fn is_forced(ticket: &SavedChunkTicket) -> bool {
    ticket.ticks_left.is_none()
}

impl ServerWorld {
    /// `ServerLevel.getForceLoadedChunks()` (ServerLevel.java:1493-1495):
    /// `TicketStorage.chunksWithForcedTickets` (TicketStorage.java:388-390),
    /// which `getAllChunksWithTicketThat` (`:392-405`) collects once per
    /// `FORCED` change.
    ///
    /// The level a ticket sits at is not part of the question -- the set is
    /// every column carrying the type at all -- which is why this is a wider
    /// predicate than the one [`Self::set_chunks_forced`] removes with.
    ///
    /// A dimension this world built no level for holds no ticket.
    pub(crate) fn forced_chunks_by_key(&self, dimension: &LevelId) -> Vec<ChunkPos> {
        let Some(level) = self.levels.get(dimension) else {
            return Vec::new();
        };
        // `getAllChunksWithTicketThat` breaks after the first matching ticket
        // of a column (`:399`), so a column carrying two `FORCED` tickets is
        // one member. Vanilla's `LongOpenHashSet` has no order of its own and
        // its one ordered reader sorts (`ForceLoadCommand.java:104`).
        level
            .saved_chunk_tickets
            .iter()
            .filter(|ticket| is_forced(ticket))
            .map(|ticket| ticket.chunk)
            .collect::<std::collections::BTreeSet<ChunkKey>>()
            .into_iter()
            .map(|(x, z)| ChunkPos { x, z })
            .collect()
    }

    /// `ServerLevel.setChunkForced` (ServerLevel.java:1497-1504) for every
    /// column the command walked, in its own order.
    pub(crate) fn set_chunks_forced(
        &mut self,
        dimension: LevelId,
        forced: bool,
        chunks: &[ChunkPos],
    ) {
        for chunk in chunks {
            let key = (chunk.x, chunk.z);
            let tickets = &mut self.level_mut_by_id(&dimension).saved_chunk_tickets;
            // `TicketStorage.isTicketSameTypeAndLevel` (TicketStorage.java:181-183):
            // the write matches on the type *and* the level, so a `FORCED`
            // ticket a save filed at another level is listed by
            // [`Self::forced_chunks`] and is not what `remove` takes away.
            let held = tickets.iter().position(|ticket| {
                ticket.chunk == key && is_forced(ticket) && ticket.level == FORCED_TICKET_LEVEL
            });
            match (forced, held) {
                // `addTicket` (`:147-178`): a column that already carries the
                // ticket is left as it is.
                (true, None) => tickets.push(SavedChunkTicket {
                    chunk: key,
                    level: FORCED_TICKET_LEVEL,
                    ticks_left: None,
                }),
                // `removeTicket` (`:225-267`): so is a column that carries none.
                (false, Some(index)) => {
                    tickets.remove(index);
                }
                _ => {}
            }
        }
        // `TicketStorage`'s own listener calls, which vanilla makes from inside
        // each write (`:165-171`, `:250-256`). Vanilla's `setChunkForced` then
        // loads each column on the server thread (ServerLevel.java:1499-1501);
        // here the ticket alone queues the reach, which this tick's loading
        // refresh files and later ticks commit, registering tick containers as
        // each column becomes FULL.
        self.refresh_simulation_chunk_levels_by_key(&dimension);
    }
}
