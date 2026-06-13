use std::collections::BTreeMap;

use anyhow::{Context, Result};
use bbb_protocol::packets::{self, ClientIntent, PlayerPositionState};
use bbb_world::{BlockPos, ChunkPos, WorldStore};
use tokio::time::{timeout, Interval};

use crate::{
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
    player_was_dead: bool,
    play_tick: Option<Interval>,
    server_cookies: BTreeMap<String, Vec<u8>>,
}

impl ProbeContext {
    fn new(conn: RawConnection) -> Self {
        Self {
            conn,
            state: ConnectionState::Login,
            world: WorldStore::new(),
            player_loaded_sent: false,
            player_position_state: PlayerPositionState::default(),
            player_was_dead: false,
            play_tick: None,
            server_cookies: BTreeMap::new(),
        }
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

        ProbeReport {
            reached_state: self.state,
            compression_threshold: self.conn.compression_threshold,
            packets_seen,
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
    let mut packets_seen = 0usize;

    let (id, payload) = packets::encode_handshake(&options.host, options.port, ClientIntent::Login);
    probe.conn.send_packet(id, &payload).await?;
    let (id, payload) = packets::encode_login_hello(&options.username, options.profile_id);
    probe.conn.send_packet(id, &payload).await?;

    let first_chunk = loop {
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
                if let Some(first_chunk) = probe.handle_play_packet(packet).await? {
                    break first_chunk;
                }
            }
            ConnectionState::Handshake | ConnectionState::Status => {
                unreachable!("probe starts at login")
            }
        }
    };

    Ok(probe.finish(packets_seen, first_chunk))
}
