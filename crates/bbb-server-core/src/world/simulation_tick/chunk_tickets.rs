//! The simulation levels the ticket set projects: which columns tick blocks
//! and which tick entities, plus the stale-ticket purge.

use super::*;
use crate::world::LevelId;

impl ServerWorld {
    /// Simulation tickets from saved sources, players, pearls and the dragon fight.
    /// `DistanceManager.java:109-116,131-133,151-156` sets player levels from
    /// the live simulation distance; spectator membership is filtered upstream.
    pub(in crate::world) fn refresh_simulation_chunk_levels(
        &mut self,
        dimension: Java26BuiltinDimension,
    ) {
        self.refresh_simulation_chunk_levels_by_key(&dimension.into());
    }

    pub(in crate::world) fn refresh_simulation_chunk_levels_by_key(&mut self, dimension: &LevelId) {
        let player_ticket_level =
            (CHUNK_ENTITY_TICKING_LEVEL - self.admin.settings().simulation_distance).max(0);
        // Only the sessions in *this* level file a ticket in it: the
        // `PLAYER_SIMULATION` ticket is added by the level's own
        // `DistanceManager` (`ChunkMap.updatePlayerPos` through
        // `DistanceManager.addPlayer`), so a player in the nether leaves the
        // overworld's tracker untouched.
        let level = self.level_mut_by_id(dimension);
        let players = level
            .player_chunk_membership
            .centres()
            .map(|chunk| (chunk, player_ticket_level));
        let dragon = level
            .dragon_fight
            .as_ref()
            .and_then(|fight| fight.chunk_ticket());
        let sources = level
            .saved_chunk_tickets
            .iter()
            .map(|ticket| (ticket.chunk, ticket.level))
            .chain(level.chunk_lifecycle.ender_pearl_sources())
            .chain(players)
            .chain(dragon);
        level.simulation_levels.rebuild(sources);
    }

    /// `TicketStorage.purgeStaleTickets` (`TicketStorage.java:279-289`): every
    /// ticket the type lets expire loses one tick, and one that has gone below
    /// zero is dropped (`Ticket.decreaseTicksLeft`, `Ticket.java:60-64`, and
    /// `isTimedOut`, `:66-68`).
    ///
    /// `canTicketExpire` (`TicketStorage.java:291-302`) has three answers. The
    /// first is the type's own `hasTimeout` (`TicketType.java:51-53`), which is
    /// [`SavedChunkTicket::ticks_left`] being `Some` here. The second,
    /// `canExpireIfUnloaded` (`TicketType.java:47-49`), is false for both
    /// persisted types — flags 15 has no `FLAG_CAN_EXPIRE_IF_UNLOADED`
    /// (`TicketType.java:16`, `:22-23`). The third is
    /// `updatingChunk == null || updatingChunk.isReadyForSaving()`, and it is
    /// true for every column of this world: `ChunkHolder.isReadyForSaving`
    /// (`ChunkHolder.java:110-112`) asks whether the holder's save future is
    /// done, that future starts completed (`ChunkHolder.java:52`), and the only
    /// things that replace it are an outstanding full-chunk promotion
    /// (`:281-282`, `:294-295`, `:311-312`) and an outstanding generation step
    /// (`GenerationChunkHolder.increaseGenerationRefCount`,
    /// `GenerationChunkHolder.java:236-241`). This server has neither: it holds
    /// no promotion futures and [`ChunkStore::load_column`] generates in the
    /// call.
    ///
    /// Vanilla runs this at the head of `ServerChunkCache.tick`
    /// (`ServerChunkCache.java:320-323`), before `runDistanceManagerUpdates`
    /// (`:325`), so a ticket that ran out this tick is already gone from the
    /// levels the same tick reads. Its `tickRateManager().runsNormally()` guard
    /// (`:321`) is the caller's: a frozen world does not age its tickets.
    ///
    /// The subtraction saturates because `ticks_left` came off a save and
    /// `Codec.LONG` (`Ticket.java:15`) accepts every `long`. Saturating changes
    /// no answer -- a ticket already below zero is dropped on this same pass --
    /// and keeps a hostile save from deciding between a panic and a wrap.
    pub(super) fn purge_stale_tickets(&mut self, dimension: &LevelId) {
        self.level_mut_by_id(dimension)
            .chunk_lifecycle
            .purge_transient_tickets();
        self.level_mut_by_id(dimension)
            .saved_chunk_tickets
            .retain_mut(|ticket| {
                let Some(ticks_left) = ticket.ticks_left.as_mut() else {
                    return true;
                };
                *ticks_left = ticks_left.saturating_sub(1);
                *ticks_left >= 0
            });
    }

    /// `DistanceManager.forEachEntityTickingChunk` (`DistanceManager.java:173-181`):
    /// simulation level and the loading lifecycle must both permit ticking.
    #[cfg(test)]
    pub(crate) fn entity_ticking_chunks(&self, dimension: Java26BuiltinDimension) -> Vec<ChunkKey> {
        self.entity_ticking_chunks_by_key(&dimension.into())
    }

    pub(crate) fn entity_ticking_chunks_by_key(&self, dimension: &LevelId) -> Vec<ChunkKey> {
        self.simulating_chunks(dimension, CHUNK_ENTITY_TICKING_LEVEL)
    }

    /// The same for `ChunkLevel.BLOCK_TICKING_LEVEL`, which is one level wider:
    /// the gate `ServerLevel.shouldTickBlocksAt` (`ServerLevel.java:457-459`)
    /// and therefore both scheduled-tick containers (`:207-208`) read.
    #[cfg(test)]
    pub(crate) fn block_ticking_chunks(&self, dimension: Java26BuiltinDimension) -> Vec<ChunkKey> {
        self.block_ticking_chunks_by_key(&dimension.into())
    }

    pub(crate) fn block_ticking_chunks_by_key(&self, dimension: &LevelId) -> Vec<ChunkKey> {
        self.simulating_chunks(dimension, CHUNK_BLOCK_TICKING_LEVEL)
    }

    pub(super) fn simulating_chunks(&self, dimension: &LevelId, bound: i32) -> Vec<ChunkKey> {
        let level = self.level_by_id(dimension);
        level
            .simulation_levels
            .chunks_at_most(bound)
            .filter(|&(chunk_x, chunk_z)| {
                level.chunks.is_loaded(chunk_x, chunk_z)
                    && level.chunk_lifecycle.status((chunk_x, chunk_z))
                        >= super::super::chunk_lifecycle::FullChunkStatus::BlockTicking
                    && (bound != CHUNK_BLOCK_TICKING_LEVEL
                        || level.chunk_lifecycle.entities_loaded((chunk_x, chunk_z)))
            })
            .collect()
    }
}
