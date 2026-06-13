use std::collections::BTreeMap;

use anyhow::{anyhow, Context, Result};
use bbb_protocol::{
    frame::encode_packet,
    packets::{self, ClientIntent, PlayerPositionState},
};
use tokio::{
    sync::mpsc,
    time::{timeout, Interval},
};

use crate::{
    connection::RawConnection,
    driver::{read_packet_or_drive_connection, ConnectionDrive},
    types::{ConnectionOptions, ConnectionState, NetCommand, NetEvent},
};

mod configuration;
mod login;
mod play;

struct EventStreamContext {
    conn: RawConnection,
    events: mpsc::Sender<NetEvent>,
    commands: mpsc::Receiver<NetCommand>,
    state: ConnectionState,
    player_loaded_sent: bool,
    player_position_state: PlayerPositionState,
    player_was_dead: bool,
    play_tick: Option<Interval>,
    server_cookies: BTreeMap<String, Vec<u8>>,
}

pub async fn run_offline_event_stream(
    options: ConnectionOptions,
    events: mpsc::Sender<NetEvent>,
    commands: mpsc::Receiver<NetCommand>,
) -> Result<()> {
    let conn = timeout(
        options.timeout,
        RawConnection::connect(&options.address, None),
    )
    .await
    .context("offline connect timed out")??;
    let mut stream = EventStreamContext {
        conn,
        events,
        commands,
        state: ConnectionState::Login,
        player_loaded_sent: false,
        player_position_state: PlayerPositionState::default(),
        player_was_dead: false,
        play_tick: None,
        server_cookies: BTreeMap::new(),
    };

    emit(&stream.events, NetEvent::Connected).await?;
    emit(
        &stream.events,
        NetEvent::StateChanged {
            state: stream.state,
        },
    )
    .await?;

    let (id, payload) = packets::encode_handshake(&options.host, options.port, ClientIntent::Login);
    stream.conn.send_packet(id, &payload).await?;
    let (id, payload) = packets::encode_login_hello(&options.username, options.profile_id);
    stream.conn.send_packet(id, &payload).await?;

    loop {
        let drive = read_packet_or_drive_connection(
            &mut stream.conn,
            stream.state,
            &mut stream.play_tick,
            &mut stream.commands,
            &mut stream.player_position_state,
        )
        .await?;
        let ConnectionDrive::Packet(packet_id, payload) = drive else {
            return Ok(());
        };
        tracing::debug!(
            state = ?stream.state,
            packet_id,
            len = payload.len(),
            "clientbound packet"
        );
        emit_best_effort(
            &stream.events,
            NetEvent::PacketSeen {
                state: stream.state,
                packet_id,
                len: payload.len(),
            },
        )?;

        match stream.state {
            ConnectionState::Login => {
                let packet = packets::decode_login_clientbound(packet_id, &payload)?;
                stream.handle_login_packet(packet).await?;
            }
            ConnectionState::Configuration => {
                let packet = packets::decode_configuration_clientbound(packet_id, &payload)?;
                stream.handle_configuration_packet(packet).await?;
            }
            ConnectionState::Play => {
                let packet = packets::decode_play_clientbound(packet_id, &payload)?;
                stream.handle_play_packet(packet).await?;
            }
            ConnectionState::Handshake | ConnectionState::Status => {
                unreachable!("event stream starts at login")
            }
        }
    }
}

pub(super) async fn emit(events: &mpsc::Sender<NetEvent>, event: NetEvent) -> Result<()> {
    events
        .send(event)
        .await
        .map_err(|_| anyhow!("net event receiver dropped"))
}

fn emit_best_effort(events: &mpsc::Sender<NetEvent>, event: NetEvent) -> Result<()> {
    if events.capacity() <= 1024 {
        return Ok(());
    }

    match events.try_send(event) {
        Ok(()) | Err(mpsc::error::TrySendError::Full(_)) => Ok(()),
        Err(mpsc::error::TrySendError::Closed(_)) => Err(anyhow!("net event receiver dropped")),
    }
}

#[allow(dead_code)]
fn _keep_encode_packet_reachable(packet_id: i32, payload: &[u8]) -> Vec<u8> {
    encode_packet(packet_id, payload)
}
