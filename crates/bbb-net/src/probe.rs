use std::collections::BTreeMap;

use anyhow::{Context, Result};
use bbb_protocol::packets::{self, ClientIntent, PlayerPositionState};
use bbb_world::{BlockPos, ChunkPos, WorldStore};
use tokio::time::{timeout, Interval};

use crate::{
    chunk_batch::ChunkBatchSizeCalculator,
    connection::RawConnection,
    driver::read_packet_or_send_play_tick,
    types::{ChunkProbeSummary, ConnectionOptions, ConnectionState, ProbeReport},
};

mod configuration;
mod login;
mod play;

struct ProbeContext {
    conn: RawConnection,
    state: ConnectionState,
    world: WorldStore,
    player_loaded_sent: bool,
    player_position_state: PlayerPositionState,
    play_tick: Option<Interval>,
    chunk_batch_size: ChunkBatchSizeCalculator,
    server_cookies: BTreeMap<String, Vec<u8>>,
    seen_code_of_conduct: bool,
    accepted_code_of_conduct_hash: Option<i32>,
    client_information: packets::ClientInformation,
    unsupported_packets: usize,
    last_unsupported_packet_state: Option<ConnectionState>,
    last_unsupported_packet_id: Option<i32>,
    last_unsupported_packet_len: Option<usize>,
}

#[derive(Debug)]
struct ProbeDrain {
    first_chunk: Option<ChunkPos>,
    packets_after_first_chunk: usize,
    after_first_chunk_limit: usize,
}

impl ProbeDrain {
    fn new(after_first_chunk_limit: usize) -> Self {
        Self {
            first_chunk: None,
            packets_after_first_chunk: 0,
            after_first_chunk_limit,
        }
    }

    fn observe_packet(&mut self, packet_first_chunk: Option<ChunkPos>) -> bool {
        if self.first_chunk.is_some() {
            self.packets_after_first_chunk += 1;
            return self.packets_after_first_chunk >= self.after_first_chunk_limit;
        }

        self.first_chunk = packet_first_chunk;
        self.first_chunk.is_some() && self.after_first_chunk_limit == 0
    }

    fn first_chunk(&self) -> Option<ChunkPos> {
        self.first_chunk
    }
}

impl ProbeContext {
    fn new(conn: RawConnection) -> Self {
        Self {
            conn,
            state: ConnectionState::Login,
            world: WorldStore::new(),
            player_loaded_sent: false,
            player_position_state: PlayerPositionState::default(),
            play_tick: None,
            chunk_batch_size: ChunkBatchSizeCalculator::new(),
            server_cookies: BTreeMap::new(),
            seen_code_of_conduct: false,
            accepted_code_of_conduct_hash: None,
            client_information: packets::ClientInformation::default(),
            unsupported_packets: 0,
            last_unsupported_packet_state: None,
            last_unsupported_packet_id: None,
            last_unsupported_packet_len: None,
        }
    }

    fn record_unsupported_packet(&mut self, state: ConnectionState, packet_id: i32, len: usize) {
        self.unsupported_packets += 1;
        self.last_unsupported_packet_state = Some(state);
        self.last_unsupported_packet_id = Some(packet_id);
        self.last_unsupported_packet_len = Some(len);
    }

    fn finish(self, packets_seen: usize, first_chunk: ChunkPos) -> ProbeReport {
        let first_chunk_summary = self
            .world
            .probe_chunk(first_chunk)
            .map(ChunkProbeSummary::from_column);
        let first_chunk_center_block = self.world.probe_block(BlockPos {
            x: first_chunk.x * 16 + 8,
            y: 64,
            z: first_chunk.z * 16 + 8,
        });
        let world_counters = self.world.counters();
        let world_apply_errors = self.world.world_apply_error_messages();

        ProbeReport {
            reached_state: self.state,
            compression_threshold: self.conn.compression_threshold,
            packets_seen,
            unsupported_packets: self.unsupported_packets,
            last_unsupported_packet_state: self
                .last_unsupported_packet_state
                .map(|state| format!("{state:?}")),
            last_unsupported_packet_id: self.last_unsupported_packet_id,
            last_unsupported_packet_len: self.last_unsupported_packet_len,
            registries_seen: world_counters.registries_seen,
            registry_entries_seen: world_counters.registry_entries_seen,
            registry_entries_with_data: world_counters.registry_entries_with_data,
            registry_entry_stubs: world_counters.registry_entry_stubs,
            registry_entry_payload_bytes: world_counters.registry_entry_payload_bytes,
            registry_content_registries_tracked: world_counters.registry_content_registries_tracked,
            registry_content_packets_tracked: world_counters.registry_content_packets_tracked,
            registry_content_entries_tracked: world_counters.registry_content_entries_tracked,
            registry_duplicate_entries: world_counters.registry_duplicate_entries,
            registry_duplicate_entry_ids_tracked: world_counters
                .registry_duplicate_entry_ids_tracked,
            last_registry_data_registry: world_counters.last_registry_data_registry.clone(),
            last_registry_data_entry_count: world_counters.last_registry_data_entry_count,
            first_chunk: Some(first_chunk),
            first_chunk_summary,
            first_chunk_center_block,
            world_counters,
            world_apply_errors,
            world: self.world,
        }
    }
}

pub async fn run_offline_probe(options: ConnectionOptions) -> Result<ProbeReport> {
    timeout(options.timeout, run_offline_probe_inner(options))
        .await
        .context("offline probe timed out")?
}

async fn run_offline_probe_inner(options: ConnectionOptions) -> Result<ProbeReport> {
    let mut probe = ProbeContext::new(RawConnection::connect(&options.address, None).await?);
    probe.accepted_code_of_conduct_hash = options.accepted_code_of_conduct_hash;
    probe.client_information = options.client_information.clone();
    let mut packets_seen = 0usize;

    let (id, payload) = packets::encode_handshake(&options.host, options.port, ClientIntent::Login);
    probe.conn.send_packet(id, &payload).await?;
    let (id, payload) = packets::encode_login_hello(&options.username, options.profile_id);
    probe.conn.send_packet(id, &payload).await?;

    let mut drain = ProbeDrain::new(options.probe_after_first_chunk_packets);
    loop {
        let (packet_id, payload) =
            read_packet_or_send_play_tick(&mut probe.conn, probe.state, &mut probe.play_tick)
                .await?;
        packets_seen += 1;
        tracing::debug!(
            state = ?probe.state,
            packet_id,
            len = payload.len(),
            "clientbound packet"
        );

        let mut packet_first_chunk = None;
        match probe.state {
            ConnectionState::Login => {
                let packet = packets::decode_login_clientbound(packet_id, &payload)?;
                probe.handle_login_packet(packet).await?;
            }
            ConnectionState::Configuration => {
                let packet = packets::decode_configuration_clientbound(packet_id, &payload)?;
                probe.handle_configuration_packet(packet).await?;
            }
            ConnectionState::Play => {
                let packet = packets::decode_play_clientbound(packet_id, &payload)?;
                packet_first_chunk = probe.handle_play_packet(packet).await?;
            }
            ConnectionState::Handshake | ConnectionState::Status => {
                unreachable!("probe starts at login")
            }
        }

        if drain.observe_packet(packet_first_chunk) {
            break;
        }
    }

    let first_chunk = drain
        .first_chunk()
        .expect("probe exits only after the first chunk is observed");
    Ok(probe.finish(packets_seen, first_chunk))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn probe_drain_finishes_on_first_chunk_by_default() {
        let first_chunk = ChunkPos { x: 1, z: -2 };
        let mut drain = ProbeDrain::new(0);

        assert!(!drain.observe_packet(None));
        assert!(drain.observe_packet(Some(first_chunk)));
        assert_eq!(drain.first_chunk(), Some(first_chunk));
        assert_eq!(drain.packets_after_first_chunk, 0);
    }

    #[test]
    fn probe_drain_counts_bounded_packets_after_first_chunk() {
        let first_chunk = ChunkPos { x: 1, z: -2 };
        let later_chunk = ChunkPos { x: 3, z: 4 };
        let mut drain = ProbeDrain::new(2);

        assert!(!drain.observe_packet(None));
        assert!(!drain.observe_packet(Some(first_chunk)));
        assert_eq!(drain.first_chunk(), Some(first_chunk));
        assert_eq!(drain.packets_after_first_chunk, 0);

        assert!(!drain.observe_packet(None));
        assert_eq!(drain.packets_after_first_chunk, 1);
        assert!(drain.observe_packet(Some(later_chunk)));
        assert_eq!(drain.packets_after_first_chunk, 2);
        assert_eq!(drain.first_chunk(), Some(first_chunk));
    }
}
