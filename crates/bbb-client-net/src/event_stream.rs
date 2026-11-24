use std::{
    fmt,
    ops::{Deref, DerefMut},
};

use anyhow::{anyhow, Result};
#[cfg(test)]
use bbb_protocol::heightmap_types::VANILLA_HEIGHTMAP_WORLD_SURFACE_ID;
#[cfg(test)]
use bbb_protocol::menu_types::VANILLA_MENU_TYPE_MERCHANT_ID;
use bbb_protocol::packets::{self, ClientIntent, Transfer};
#[cfg(test)]
use bbb_protocol::recipe_wire_types::{
    VANILLA_RECIPE_BOOK_CATEGORY_CRAFTING_EQUIPMENT_ID,
    VANILLA_SLOT_DISPLAY_TYPE_WITH_ANY_POTION_ID,
};
#[cfg(test)]
use bbb_protocol::stat_types::{VANILLA_STAT_TYPE_BLOCK_MINED_ID, VANILLA_STAT_TYPE_CUSTOM_ID};
use tokio::sync::mpsc;

use crate::{
    connection::RawConnection,
    driver::{
        read_packet_or_drive_connection_with_deadline, ConnectionDrive, ConnectionDriveContext,
        IdleReadDeadline,
    },
    play_bundle::PlayPacketBundleAssembler,
    session::{ClientProtocolSession, RemoteDisconnectError},
    types::{
        ConnectionOptions, ConnectionState, EventStreamExit, InboundPacketMetrics, NetCommand,
        NetEvent,
    },
};

mod configuration;
mod login;
mod play;

struct EventStreamContext {
    session: ClientProtocolSession,
    events: NetEventSender,
    commands: mpsc::Receiver<NetCommand>,
}

impl EventStreamContext {
    #[cfg(test)]
    fn new(
        conn: RawConnection,
        events: NetEventSender,
        commands: mpsc::Receiver<NetCommand>,
    ) -> Self {
        Self {
            session: ClientProtocolSession::new(conn),
            events,
            commands,
        }
    }
}

impl Deref for EventStreamContext {
    type Target = ClientProtocolSession;

    fn deref(&self) -> &Self::Target {
        &self.session
    }
}

impl DerefMut for EventStreamContext {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.session
    }
}

pub async fn run_event_stream(
    options: ConnectionOptions,
    events: NetEventSender,
    commands: mpsc::Receiver<NetCommand>,
) -> Result<EventStreamExit> {
    classify_event_stream_result(run_event_stream_inner(options, events, commands, None).await)
}

#[cfg(test)]
async fn run_event_stream_with_services_endpoint(
    options: ConnectionOptions,
    events: NetEventSender,
    commands: mpsc::Receiver<NetCommand>,
    services_endpoint: String,
) -> Result<EventStreamExit> {
    classify_event_stream_result(
        run_event_stream_inner(options, events, commands, Some(services_endpoint)).await,
    )
}

fn classify_event_stream_result(result: Result<()>) -> Result<EventStreamExit> {
    match result {
        Ok(()) => Ok(EventStreamExit::LocalDisconnect),
        Err(error) => match error.downcast::<RemoteDisconnectError>() {
            Ok(remote) => Ok(EventStreamExit::RemoteDisconnect(remote.reason)),
            Err(error) => Err(error),
        },
    }
}

async fn run_event_stream_inner(
    options: ConnectionOptions,
    events: NetEventSender,
    commands: mpsc::Receiver<NetCommand>,
    services_endpoint: Option<String>,
) -> Result<()> {
    let conn = RawConnection::connect_with_gateway(
        &options.address,
        options.websocket_gateway.as_deref(),
        options.timeout,
        None,
    )
    .await?;
    let mut stream = EventStreamContext {
        session: ClientProtocolSession::from_connection_options(conn, &options),
        events,
        commands,
    };

    emit(&stream.events, NetEvent::Connected).await?;
    emit(
        &stream.events,
        NetEvent::InboundMetrics(stream.events.metrics()),
    )
    .await?;
    emit(
        &stream.events,
        NetEvent::StateChanged {
            state: stream.state,
        },
    )
    .await?;
    let _services_keys_guard = match services_endpoint {
        #[cfg(not(target_arch = "wasm32"))]
        Some(endpoint) => Some(
            crate::services_profile_keys::spawn_services_public_keys_fetch_at(
                options.timeout,
                endpoint,
                stream.events.clone(),
                false,
            ),
        ),
        #[cfg(target_arch = "wasm32")]
        Some(_) => None,
        #[cfg(not(test))]
        None => {
            crate::services_profile_keys::spawn_default_services_public_keys_fetch(
                options.timeout,
                &stream.events,
            )
            .await?
        }
        #[cfg(test)]
        None => None,
    };

    stream
        .start_login(
            &options.host,
            options.port,
            ClientIntent::Login,
            &options.username,
            options.profile_id,
        )
        .await?;

    let result = drive_event_stream(&mut stream, &options).await;
    stream.reset_content_registry();
    result
}

async fn drive_event_stream(
    stream: &mut EventStreamContext,
    options: &ConnectionOptions,
) -> Result<()> {
    let mut play_bundle = PlayPacketBundleAssembler::default();
    let mut idle_deadline = IdleReadDeadline::default();
    // Mirrors Vanilla `ProtocolSwapHandler.java:7-13` and
    // `UnconfiguredPipelineHandler.java:15-60`: pause inbound reads after a
    // terminal `StartConfiguration` / `FinishConfiguration` packet until the
    // matching local acknowledge command switches the protocol state.
    let mut inbound_paused = false;
    loop {
        let drive = read_packet_or_drive_connection_with_deadline(ConnectionDriveContext {
            conn: &mut stream.session.conn,
            state: stream.session.state,
            commands: &mut stream.commands,
            player_position_state: &mut stream.session.player_position_state,
            player_movement_flags: &mut stream.session.player_movement_flags,
            teleport_pending: stream.session.pending_teleports > 0,
            inbound_paused,
            content_registry: &stream.session.content_registry,
            idle_deadline: &mut idle_deadline,
        })
        .await?;
        let packet_id = match drive {
            ConnectionDrive::Packet(packet_id) => packet_id,
            ConnectionDrive::AcceptTeleport(command) => {
                stream.accept_teleport(command).await?;
                continue;
            }
            ConnectionDrive::AcknowledgePlayerRotation { y_rot, x_rot } => {
                stream.acknowledge_player_rotation(y_rot, x_rot).await?;
                continue;
            }
            ConnectionDrive::AcknowledgeConfiguration {
                pending_chat_acknowledgement,
            } => {
                stream
                    .acknowledge_start_configuration_with_chat(pending_chat_acknowledgement)
                    .await?;
                play_bundle = PlayPacketBundleAssembler::default();
                inbound_paused = false;
                idle_deadline.clear();
                continue;
            }
            ConnectionDrive::FinishConfiguration => {
                stream.finish_configuration().await?;
                inbound_paused = false;
                idle_deadline.clear();
                continue;
            }
            ConnectionDrive::DisconnectWithReason(reason) => {
                emit(
                    &stream.events,
                    NetEvent::Disconnected {
                        reason: Some(crate::DisconnectReason::from_plain(reason)),
                    },
                )
                .await?;
                return Ok(());
            }
            ConnectionDrive::Disconnect => return Ok(()),
        };
        let payload = stream.conn.payload();
        tracing::debug!(
            state = ?stream.state,
            packet_id,
            len = payload.len(),
            "clientbound packet"
        );
        stream.events.record_inbound_packet(payload.len());

        let packet_state = stream.state;
        let transfer = match packet_state {
            ConnectionState::Login => {
                let packet = packets::decode_login_clientbound(packet_id, payload)?;
                stream.handle_login_packet(packet).await?;
                None
            }
            ConnectionState::Configuration => {
                let packet = packets::decode_configuration_clientbound(packet_id, payload)?;
                if matches!(packet, packets::ConfigurationClientbound::Finish) {
                    inbound_paused = true;
                }
                stream
                    .handle_configuration_packet_with_resource_pack_policy(
                        packet,
                        options.server_resource_pack_policy,
                        &options.available_known_packs,
                    )
                    .await?
            }
            ConnectionState::Play => {
                let packet = stream.decode_play_packet(packet_id, payload)?;
                let mut transfer = None;
                if let Some(batch) = play_bundle.push(packet)? {
                    for packet in batch {
                        if transfer.is_some() {
                            if let Some(next) =
                                stream.handle_transfer_closing_play_packet(packet).await?
                            {
                                transfer = Some(next);
                            }
                        } else {
                            // `ServerGamePacketListenerImpl.java:1771-1776` swaps
                            // the server outbound encoder immediately after
                            // sending `StartConfiguration`.
                            if matches!(packet, packets::PlayClientbound::StartConfiguration) {
                                inbound_paused = true;
                            }
                            transfer = stream
                                .handle_play_packet_with_resource_pack_policy(
                                    packet,
                                    options.server_resource_pack_policy,
                                )
                                .await?;
                        }
                    }
                }
                transfer
            }
            ConnectionState::Handshake | ConnectionState::Status => {
                unreachable!("event stream starts at login")
            }
        };

        if let Some(transfer) = transfer {
            let transfer = stream
                .drain_transfer_closing_packets(packet_state, transfer)
                .await?;
            reconnect_for_transfer(stream, options, transfer).await?;
            play_bundle = PlayPacketBundleAssembler::default();
            inbound_paused = false;
            idle_deadline.clear();
        }
    }
}

impl EventStreamContext {
    async fn handle_transfer_closing_play_packet(
        &mut self,
        packet: packets::PlayClientbound,
    ) -> Result<Option<Transfer>> {
        if matches!(
            &packet,
            packets::PlayClientbound::StoreCookie(_) | packets::PlayClientbound::Transfer(_)
        ) {
            self.handle_play_packet(packet).await
        } else {
            Ok(None)
        }
    }

    async fn drain_transfer_closing_packets(
        &mut self,
        packet_state: ConnectionState,
        mut transfer: Transfer,
    ) -> Result<Transfer> {
        while let Some(packet_id) = self.conn.try_read_buffered_packet()? {
            let payload = self.conn.payload();
            tracing::debug!(
                state = ?packet_state,
                packet_id,
                len = payload.len(),
                "clientbound packet during transfer close"
            );
            self.events.record_inbound_packet(payload.len());

            let next = match packet_state {
                ConnectionState::Configuration => {
                    let packet = packets::decode_configuration_clientbound(packet_id, payload)?;
                    if matches!(
                        &packet,
                        packets::ConfigurationClientbound::StoreCookie(_)
                            | packets::ConfigurationClientbound::Transfer(_)
                    ) {
                        self.handle_configuration_packet(packet).await?
                    } else {
                        None
                    }
                }
                ConnectionState::Play => {
                    let packet = self.decode_play_packet(packet_id, payload)?;
                    self.handle_transfer_closing_play_packet(packet).await?
                }
                _ => unreachable!("transfer is only legal in configuration or play"),
            };
            if let Some(next) = next {
                transfer = next;
            }
        }
        Ok(transfer)
    }
}

async fn reconnect_for_transfer(
    stream: &mut EventStreamContext,
    options: &ConnectionOptions,
    transfer: Transfer,
) -> Result<()> {
    stream
        .session
        .reconnect_for_transfer(options, transfer)
        .await?;
    emit(&stream.events, NetEvent::Connected).await?;
    emit(
        &stream.events,
        NetEvent::StateChanged {
            state: stream.state,
        },
    )
    .await
}

/// The event stream's half of the client's one event channel: every event it
/// accepts also wakes the consumer through `wake`.
#[derive(Clone)]
pub struct NetEventSender {
    events: mpsc::Sender<NetEvent>,
    wake: std::sync::Arc<dyn Fn() + Send + Sync>,
    metrics: std::sync::Arc<InboundPacketMetrics>,
}

impl NetEventSender {
    pub fn new(events: mpsc::Sender<NetEvent>, wake: impl Fn() + Send + Sync + 'static) -> Self {
        Self {
            events,
            wake: std::sync::Arc::new(wake),
            metrics: std::sync::Arc::new(InboundPacketMetrics::default()),
        }
    }

    pub fn metrics(&self) -> std::sync::Arc<InboundPacketMetrics> {
        std::sync::Arc::clone(&self.metrics)
    }

    pub(crate) fn record_inbound_packet(&self, payload_len: usize) {
        self.metrics.record_packet(payload_len);
    }

    pub async fn send(&self, event: NetEvent) -> Result<()> {
        self.events
            .send(event)
            .await
            .map_err(|_| anyhow!("net event receiver dropped"))?;
        bbb_client_clock::replay::count(&bbb_client_clock::replay::NET_EVENTS_EMITTED);
        (self.wake)();
        Ok(())
    }
}

impl fmt::Debug for NetEventSender {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("NetEventSender")
            .finish_non_exhaustive()
    }
}

pub(super) async fn emit(events: &NetEventSender, event: NetEvent) -> Result<()> {
    events.send(event).await
}

/// Drops rejected `ResourcePackPush` payloads (`InvalidUrl` and optional
/// `Declined`) before forwarding to the world store so already-rejected pushes
/// do not retain URLs/prompts in world state (`ClientCommonPacketListenerImpl.java:175-178`,
/// `ServerPackManager.java:50-56`). Required declined packs still forward so
/// the client runtime disconnects with the required-pack message.
pub(super) fn should_forward_resource_pack_push(
    required: bool,
    action: Option<packets::ResourcePackResponseAction>,
) -> bool {
    match action {
        None | Some(packets::ResourcePackResponseAction::Accepted) => true,
        Some(packets::ResourcePackResponseAction::Declined) => required,
        Some(_) => false,
    }
}

#[cfg(test)]
mod tests;
