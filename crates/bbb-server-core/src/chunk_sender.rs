//! Per-player chunk delivery pacing.
//!
//! Transcribed from `PlayerChunkSender` (PlayerChunkSender.java:24-121). The
//! server never pushes chunks as fast as it can produce them: it wraps each
//! group in a batch the client acknowledges, and the client's answer sets how
//! many chunks per tick the server may send next. This is the whole reason a
//! joining player's world streams in smoothly instead of stalling the client.
//!
//! This owns only the pacing decision. Which chunks exist and how they encode
//! stays with the chunk store, and turning a batch into packets stays with the
//! world.

use std::collections::BTreeSet;

/// `PlayerChunkSender.MIN_CHUNKS_PER_TICK` (PlayerChunkSender.java:25).
pub const MIN_CHUNKS_PER_TICK: f32 = 0.01;
/// `PlayerChunkSender.MAX_CHUNKS_PER_TICK` (:26).
pub const MAX_CHUNKS_PER_TICK: f32 = 64.0;
/// `PlayerChunkSender.START_CHUNKS_PER_TICK` (:27).
pub const START_CHUNKS_PER_TICK: f32 = 9.0;
/// `PlayerChunkSender.MAX_UNACKNOWLEDGED_BATCHES` (:28), which only takes
/// effect after the client has acknowledged its first batch.
pub const MAX_UNACKNOWLEDGED_BATCHES: i32 = 10;

/// A chunk column position, in chunk coordinates.
pub type ChunkKey = (i32, i32);

#[derive(Debug, Clone)]
pub struct PlayerChunkSender {
    pending_chunks: BTreeSet<ChunkKey>,
    desired_chunks_per_tick: f32,
    batch_quota: f32,
    unacknowledged_batches: i32,
    max_unacknowledged_batches: i32,
}

impl Default for PlayerChunkSender {
    fn default() -> Self {
        Self::new()
    }
}

impl PlayerChunkSender {
    /// A network connection, which is the only kind bbb serves: vanilla's
    /// `memoryConnection` flag is the integrated-server path and stays false
    /// here, so the nearest-first bound on a batch always applies.
    pub fn new() -> Self {
        Self {
            pending_chunks: BTreeSet::new(),
            desired_chunks_per_tick: START_CHUNKS_PER_TICK,
            batch_quota: 0.0,
            unacknowledged_batches: 0,
            max_unacknowledged_batches: 1,
        }
    }

    /// `markChunkPendingToSend` (:40-42).
    pub fn mark_chunk_pending_to_send(&mut self, chunk: ChunkKey) {
        self.pending_chunks.insert(chunk);
    }

    /// `isPending` (:124-126), which `ChunkMap.isChunkTracked` (:227-229) asks
    /// before it lets an entity be paired: a column the session is still
    /// waiting for is not one it can place an entity in.
    pub fn is_pending(&self, chunk: ChunkKey) -> bool {
        self.pending_chunks.contains(&chunk)
    }

    /// `dropChunk` (:44-48). Returns whether the client has to be told to
    /// forget the column: a chunk still waiting to be sent is simply taken off
    /// the queue, and only one the client already has needs the packet.
    ///
    /// Vanilla's `player.isAlive()` guard is not reproduced because this world
    /// has no dead players; every session it drops a chunk for is one it is
    /// still writing to.
    pub fn drop_chunk(&mut self, chunk: ChunkKey) -> bool {
        !self.pending_chunks.remove(&chunk)
    }

    /// The batch to send this tick, empty when the pacing says to wait.
    ///
    /// `sendNextChunks` (:50-72) minus the packet writing: the caller opens the
    /// batch, sends each chunk and closes it. The returned order is
    /// `collectChunksToSend` (:84-110) — nearest to the player first, bounded
    /// by the quota once more chunks are pending than the quota allows.
    pub fn next_batch(
        &mut self,
        player_chunk: ChunkKey,
        ready: impl Fn(ChunkKey) -> bool,
    ) -> Vec<ChunkKey> {
        if self.unacknowledged_batches >= self.max_unacknowledged_batches {
            return Vec::new();
        }
        let max_batch_size = self.desired_chunks_per_tick.max(1.0);
        self.batch_quota = (self.batch_quota + self.desired_chunks_per_tick).min(max_batch_size);
        if self.batch_quota < 1.0 || self.pending_chunks.is_empty() {
            return Vec::new();
        }
        let chunks = self.collect_chunks_to_send(player_chunk, ready);
        if chunks.is_empty() {
            return Vec::new();
        }
        self.unacknowledged_batches += 1;
        self.batch_quota -= chunks.len() as f32;
        chunks
    }

    /// `handleChunkBatchReceived` (ServerGamePacketListenerImpl.java:2138-2141)
    /// into `onChunkBatchReceivedByClient` (:112-119). A NaN rate collapses to
    /// the minimum rather than poisoning the quota arithmetic, and the first
    /// acknowledgement is what lifts the one-batch-in-flight bound.
    ///
    /// Returns `false` if no batch was unacknowledged, preventing a hostile
    /// client from driving `unacknowledged_batches` negative to bypass throttling
    /// or trigger integer underflow.
    pub fn on_chunk_batch_received_by_client(&mut self, desired_chunks_per_tick: f32) -> bool {
        if self.unacknowledged_batches <= 0 {
            return false;
        }
        self.unacknowledged_batches -= 1;
        self.desired_chunks_per_tick = if desired_chunks_per_tick.is_nan() {
            MIN_CHUNKS_PER_TICK
        } else {
            desired_chunks_per_tick.clamp(MIN_CHUNKS_PER_TICK, MAX_CHUNKS_PER_TICK)
        };
        if self.unacknowledged_batches == 0 {
            self.batch_quota = 1.0;
        }
        self.max_unacknowledged_batches = MAX_UNACKNOWLEDGED_BATCHES;
        true
    }

    /// `collectChunksToSend` (:84-110), which also removes what it returns.
    ///
    /// Vanilla's two branches differ only in whether the nearest-first order is
    /// truncated to the quota; both hand back the same comparator's order, so
    /// this sorts once and truncates when the bound applies. Vanilla's
    /// comparator is not a total order over equally distant chunks, so the
    /// chunk key breaks ties here to keep the batch deterministic.
    pub fn collect_chunks_to_send(
        &mut self,
        player_chunk: ChunkKey,
        ready: impl Fn(ChunkKey) -> bool,
    ) -> Vec<ChunkKey> {
        let max_batch_size = self.batch_quota.floor().max(0.0) as usize;
        let mut chunks: Vec<ChunkKey> = self
            .pending_chunks
            .iter()
            .copied()
            .filter(|&chunk| ready(chunk))
            .collect();
        chunks.sort_by_key(|&chunk| (distance_squared(player_chunk, chunk), chunk));
        if chunks.len() > max_batch_size {
            chunks.truncate(max_batch_size);
        }
        for chunk in &chunks {
            self.pending_chunks.remove(chunk);
        }
        chunks
    }
}

/// `ChunkPos.distanceSquared` in chunk units, which is what vanilla's
/// comparator orders by.
pub fn distance_squared(from: ChunkKey, to: ChunkKey) -> i64 {
    let dx = i64::from(from.0) - i64::from(to.0);
    let dz = i64::from(from.1) - i64::from(to.1);
    dx * dx + dz * dz
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_first_batch_carries_everything_the_start_rate_allows() {
        let mut sender = PlayerChunkSender::new();
        for x in -2..=2 {
            for z in -2..=2 {
                sender.mark_chunk_pending_to_send((x, z));
            }
        }

        // Quota starts at 0 and gains the 9.0 start rate, so the first batch is
        // 9 of the 25 pending chunks, nearest first.
        let batch = sender.next_batch((0, 0), |_| true);
        assert_eq!(batch.len(), 9);
        assert_eq!(batch[0], (0, 0));
        assert!(batch.iter().all(|&(x, z)| x.abs() <= 1 && z.abs() <= 1));

        // One batch may be in flight until the client answers, so the next tick
        // sends nothing at all.
        assert!(sender.next_batch((0, 0), |_| true).is_empty());
    }

    #[test]
    fn a_slow_client_makes_the_quota_accumulate_across_ticks() {
        let mut sender = PlayerChunkSender::new();
        sender.mark_chunk_pending_to_send((0, 0));
        assert_eq!(sender.next_batch((0, 0), |_| true), vec![(0, 0)]);

        // A client that reports a quarter chunk per tick caps each batch at one
        // chunk and makes three of every four ticks send nothing at all.
        sender.on_chunk_batch_received_by_client(0.25);
        for z in 1..=2 {
            sender.mark_chunk_pending_to_send((0, z));
        }
        assert_eq!(sender.next_batch((0, 0), |_| true), vec![(0, 1)]);
        for _ in 0..3 {
            assert!(sender.next_batch((0, 0), |_| true).is_empty());
        }
        assert_eq!(sender.next_batch((0, 0), |_| true), vec![(0, 2)]);
    }

    #[test]
    fn an_acknowledged_client_may_run_more_than_one_batch_ahead() {
        let mut sender = PlayerChunkSender::new();
        for z in 0..20 {
            sender.mark_chunk_pending_to_send((0, z));
        }
        assert_eq!(sender.next_batch((0, 0), |_| true).len(), 9);
        // Until the first acknowledgement only one batch may be in flight.
        assert!(sender.next_batch((0, 0), |_| true).is_empty());

        sender.on_chunk_batch_received_by_client(9.0);
        // With the bound lifted the rest flows without waiting for each batch
        // to be acknowledged in turn.
        assert_eq!(sender.next_batch((0, 0), |_| true).len(), 9);
        assert_eq!(sender.next_batch((0, 0), |_| true).len(), 2);
        assert!(sender.next_batch((0, 0), |_| true).is_empty());
    }

    #[test]
    fn a_hostile_rate_cannot_poison_the_quota() {
        let mut sender = PlayerChunkSender::new();
        for (index, rate, expected) in [
            (0, f32::NAN, MIN_CHUNKS_PER_TICK),
            (1, f32::INFINITY, MAX_CHUNKS_PER_TICK),
            (2, -1.0, MIN_CHUNKS_PER_TICK),
        ] {
            sender.mark_chunk_pending_to_send((index, 0));
            assert_eq!(sender.next_batch((0, 0), |_| true).len(), 1);
            assert!(sender.on_chunk_batch_received_by_client(rate));
            assert_eq!(sender.desired_chunks_per_tick, expected);
        }

        // An unexpected acknowledgement when no batch is in flight is refused.
        assert!(!sender.on_chunk_batch_received_by_client(64.0));
        assert_eq!(sender.unacknowledged_batches, 0);

        // Nothing is pending, so no rate produces a batch.
        assert!(sender.next_batch((0, 0), |_| true).is_empty());
    }
}
