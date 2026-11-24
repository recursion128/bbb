//! Outbound route dispatch, wire framing cache, and per-connection delivery workers.

use std::collections::{BTreeMap, BTreeSet, HashMap};
#[cfg(test)]
use std::future::Future;
use std::sync::{Arc, Mutex, PoisonError};
use std::time::Duration;

use bbb_protocol::{
    codec::ProtocolError, content_registry::ContentRegistry,
    packets::encode_play_clientbound_disconnect,
};
use bbb_server_core::admin::AdminOutcome;
use bbb_server_core::connection::{ClientboundPlayPacket, ConnectionId};
use bbb_server_core::world::{SaveTrigger, WorldCloseReason, WorldEvent};
use bbb_server_net::{
    ConnectionCommand, ConnectionCommandSender, MailboxSendError, MailboxTrySendError,
    PreparedPlayPacket,
};
use tokio::{
    sync::{oneshot, watch, OnceCell},
    time::timeout,
};

use super::packet_encoder::{encode_world_packet, world_packet_kind};
#[cfg(test)]
use super::ServerRuntimeError;
use super::TickRequests;
use crate::management::RuntimeGateway;

// A retained Noise FULL command can keep the driver compressing while the
// next command waits for its byte quota; the timeout measures no handoff, not
// the server tick's one-second overload threshold.
pub(super) const OUTBOUND_ROUTE_DRAIN_TIMEOUT: Duration = Duration::from_secs(10);
const MAX_QUEUED_ROUTE_BATCHES: usize = 512;
const MAX_QUEUED_ROUTE_EVENTS: usize = 4096;

/// Every accepted connection's command queue, written by the accept loop and
/// read by the world thread's dispatch.
pub(super) type Routes = Mutex<BTreeMap<ConnectionId, ConnectionCommandSender>>;

pub(super) enum PendingRouteEvent {
    Send(Box<ClientboundPlayPacket>),
    Shared(Arc<SharedWorldPacket>),
    SharedChunk(SharedChunkPacket),
    Close(WorldCloseReason),
}

fn is_heavy_world_packet(packet: &ClientboundPlayPacket) -> bool {
    matches!(
        packet,
        ClientboundPlayPacket::LevelChunkWithLight(_)
            | ClientboundPlayPacket::LightUpdate(_)
            | ClientboundPlayPacket::SectionBlocksUpdate(_)
            | ClientboundPlayPacket::UpdateRecipes(_)
            | ClientboundPlayPacket::Commands(_)
            | ClientboundPlayPacket::UpdateAdvancements(_)
            | ClientboundPlayPacket::MapItemData(_)
            | ClientboundPlayPacket::ContainerSetContent(_)
            | ClientboundPlayPacket::UpdateTags(_)
            | ClientboundPlayPacket::ChunksBiomes(_)
    )
}

const PRECOMPRESS_PLAY_PACKET_BYTES: usize = 64 * 1024;

fn needs_wire_compression(payload_len: usize, compression_threshold: Option<i32>) -> bool {
    compression_threshold
        .is_some_and(|threshold| threshold >= 0 && payload_len >= threshold as usize)
}

fn encode_and_prepare_play_packet(
    packet: ClientboundPlayPacket,
    compression_threshold: Option<i32>,
    content_registry: &ContentRegistry,
) -> Result<ConnectionCommand, Arc<ProtocolError>> {
    match encode_world_packet(packet, content_registry) {
        Ok(ConnectionCommand::SendPlayPacket { packet_id, payload }) => {
            PreparedPlayPacket::encode(packet_id, payload, compression_threshold)
                .map(ConnectionCommand::SendPreparedPlayPacket)
                .map_err(Arc::new)
        }
        Ok(other) => Ok(other),
        Err(error) => Err(Arc::new(error)),
    }
}

async fn run_blocking_packet_encode(
    work: impl FnOnce() -> Result<ConnectionCommand, Arc<ProtocolError>> + Send + 'static,
) -> Result<ConnectionCommand, Arc<ProtocolError>> {
    match tokio::task::spawn_blocking(work).await {
        Ok(result) => result,
        Err(error) => Err(Arc::new(ProtocolError::InvalidData(format!(
            "outbound packet encoder task failed: {error}"
        )))),
    }
}

type EncodedPacketResult = Result<ConnectionCommand, Arc<ProtocolError>>;
type InFlightEncodeReceiver = watch::Receiver<Option<EncodedPacketResult>>;

async fn await_in_flight_encode(mut rx: InFlightEncodeReceiver) -> EncodedPacketResult {
    match rx.wait_for(Option::is_some).await {
        Ok(guard) => guard
            .as_ref()
            .expect("watch signaled Some encoded packet")
            .clone(),
        Err(_) => Err(Arc::new(ProtocolError::InvalidData(
            "outbound packet encoder task dropped before completion".to_string(),
        ))),
    }
}

struct CachedChunkWire {
    compression_threshold: Option<i32>,
    encoded: OnceCell<EncodedPacketResult>,
}

impl CachedChunkWire {
    fn new(compression_threshold: Option<i32>) -> Self {
        Self {
            compression_threshold,
            encoded: OnceCell::new(),
        }
    }

    async fn command(
        &self,
        chunk: &Arc<bbb_protocol::packets::LevelChunkWithLight>,
    ) -> EncodedPacketResult {
        if let Some(encoded) = self.encoded.get() {
            return encoded.clone();
        }
        self.encoded
            .get_or_init(|| async {
                let chunk = Arc::clone(chunk);
                let compression_threshold = self.compression_threshold;
                run_blocking_packet_encode(move || {
                    encode_and_prepare_play_packet(
                        ClientboundPlayPacket::LevelChunkWithLight(chunk),
                        compression_threshold,
                        ContentRegistry::vanilla(),
                    )
                })
                .await
            })
            .await
            .clone()
    }
}

#[derive(Clone)]
pub(super) struct SharedChunkPacket {
    chunk: Arc<bbb_protocol::packets::LevelChunkWithLight>,
    wire: Arc<CachedChunkWire>,
}

impl SharedChunkPacket {
    async fn command(&self) -> EncodedPacketResult {
        self.wire.command(&self.chunk).await
    }
}

type CachedChunkWireEntry = (
    std::sync::Weak<bbb_protocol::packets::LevelChunkWithLight>,
    Arc<CachedChunkWire>,
);

pub(super) struct ChunkWireCache {
    entries: HashMap<(usize, Option<i32>), CachedChunkWireEntry>,
    next_prune: usize,
}

impl Default for ChunkWireCache {
    fn default() -> Self {
        Self {
            entries: HashMap::new(),
            next_prune: 256,
        }
    }
}

impl ChunkWireCache {
    fn prepare(
        &mut self,
        chunk: Arc<bbb_protocol::packets::LevelChunkWithLight>,
        compression_threshold: Option<i32>,
    ) -> SharedChunkPacket {
        let key = (Arc::as_ptr(&chunk) as usize, compression_threshold);
        if let Some((weak, wire)) = self.entries.get(&key) {
            if weak
                .upgrade()
                .is_some_and(|existing| Arc::ptr_eq(&existing, &chunk))
            {
                return SharedChunkPacket {
                    chunk,
                    wire: Arc::clone(wire),
                };
            }
        }
        if self.entries.len() >= self.next_prune {
            self.entries.retain(|_, (weak, _)| weak.strong_count() > 0);
            self.next_prune = (self.entries.len() * 2).max(256);
        }
        let wire = Arc::new(CachedChunkWire::new(compression_threshold));
        self.entries
            .insert(key, (Arc::downgrade(&chunk), Arc::clone(&wire)));
        SharedChunkPacket { chunk, wire }
    }
}

pub(super) struct SharedWorldPacket {
    kind: &'static str,
    compression_threshold: Option<i32>,
    content_registry: Arc<ContentRegistry>,
    pub(super) packet: Mutex<Option<ClientboundPlayPacket>>,
    in_flight: Mutex<Option<InFlightEncodeReceiver>>,
    pub(super) encoded: OnceCell<EncodedPacketResult>,
}

impl SharedWorldPacket {
    fn new(
        packet: ClientboundPlayPacket,
        compression_threshold: Option<i32>,
        content_registry: Arc<ContentRegistry>,
    ) -> Self {
        Self {
            kind: world_packet_kind(&packet),
            compression_threshold,
            content_registry,
            packet: Mutex::new(Some(packet)),
            in_flight: Mutex::new(None),
            encoded: OnceCell::new(),
        }
    }

    pub(super) async fn command(&self) -> EncodedPacketResult {
        if let Some(encoded) = self.encoded.get() {
            return encoded.clone();
        }
        let result = self
            .encoded
            .get_or_init(|| async {
                // If an earlier caller was cancelled mid-await, join its already-spawned
                // blocking task instead of losing the taken packet.
                let existing_rx = self
                    .in_flight
                    .lock()
                    .unwrap_or_else(PoisonError::into_inner)
                    .clone();
                if let Some(rx) = existing_rx {
                    return await_in_flight_encode(rx).await;
                }
                // Encode and wire-frame at the first route's turn so broadcast compression runs
                // once across recipients while failures retain each route's valid prefix.
                let packet = self
                    .packet
                    .lock()
                    .unwrap_or_else(PoisonError::into_inner)
                    .take()
                    .expect("broadcast packet encoded once");
                let compression_threshold = self.compression_threshold;
                let content_registry = Arc::clone(&self.content_registry);
                if is_heavy_world_packet(&packet) {
                    let (tx, rx) = watch::channel(None);
                    *self
                        .in_flight
                        .lock()
                        .unwrap_or_else(PoisonError::into_inner) = Some(rx.clone());
                    tokio::task::spawn_blocking(move || {
                        let _ = tx.send(Some(encode_and_prepare_play_packet(
                            packet,
                            compression_threshold,
                            &content_registry,
                        )));
                    });
                    return await_in_flight_encode(rx).await;
                }
                match encode_world_packet(packet, &content_registry) {
                    Ok(ConnectionCommand::SendPlayPacket { packet_id, payload }) => {
                        if needs_wire_compression(payload.len(), compression_threshold) {
                            let (tx, rx) = watch::channel(None);
                            *self
                                .in_flight
                                .lock()
                                .unwrap_or_else(PoisonError::into_inner) = Some(rx.clone());
                            tokio::task::spawn_blocking(move || {
                                let encoded = PreparedPlayPacket::encode(
                                    packet_id,
                                    payload,
                                    compression_threshold,
                                )
                                .map(ConnectionCommand::SendPreparedPlayPacket)
                                .map_err(Arc::new);
                                let _ = tx.send(Some(encoded));
                            });
                            await_in_flight_encode(rx).await
                        } else {
                            PreparedPlayPacket::encode(packet_id, payload, compression_threshold)
                                .map(ConnectionCommand::SendPreparedPlayPacket)
                                .map_err(Arc::new)
                        }
                    }
                    Ok(other) => Ok(other),
                    Err(error) => Err(Arc::new(error)),
                }
            })
            .await
            .clone();
        *self
            .in_flight
            .lock()
            .unwrap_or_else(PoisonError::into_inner) = None;
        result
    }
}

pub(super) struct PendingRoute {
    pub(super) route: ConnectionCommandSender,
    pub(super) events: Vec<PendingRouteEvent>,
}

#[derive(Debug)]
enum OutboundRouteError {
    Encode {
        packet: &'static str,
        error: Arc<ProtocolError>,
    },
    Closed {
        packet: &'static str,
    },
    Oversized {
        packet: &'static str,
        actual: usize,
        max: usize,
    },
    Timeout,
}

impl OutboundRouteError {
    #[cfg(test)]
    fn packet(&self) -> &'static str {
        match self {
            Self::Encode { packet, .. }
            | Self::Closed { packet }
            | Self::Oversized { packet, .. } => packet,
            Self::Timeout => "timeout",
        }
    }
}

#[derive(Default)]
struct RouteDeliveryTrace {
    #[cfg(test)]
    handed_off: Vec<&'static str>,
    #[cfg(test)]
    encoded_in_flight: usize,
    #[cfg(test)]
    max_encoded_in_flight: usize,
}

impl RouteDeliveryTrace {
    fn encoded_started(&mut self) {
        #[cfg(test)]
        {
            self.encoded_in_flight += 1;
            self.max_encoded_in_flight = self.max_encoded_in_flight.max(self.encoded_in_flight);
        }
    }

    fn encoded_finished(&mut self) {
        #[cfg(test)]
        {
            self.encoded_in_flight -= 1;
        }
    }

    fn handed_off(&mut self, packet: &'static str) {
        #[cfg(test)]
        self.handed_off.push(packet);
        #[cfg(not(test))]
        let _ = packet;
    }
}

struct RouteDeliveryOutcome {
    error: Option<OutboundRouteError>,
    #[cfg_attr(not(test), allow(dead_code))]
    trace: RouteDeliveryTrace,
}

impl RouteDeliveryOutcome {
    fn failed(trace: RouteDeliveryTrace, error: OutboundRouteError) -> Self {
        Self {
            error: Some(error),
            trace,
        }
    }
}

async fn handoff_outbound_command(
    route: &ConnectionCommandSender,
    trace: &mut RouteDeliveryTrace,
    packet: &'static str,
    command: ConnectionCommand,
    drain_timeout: Duration,
) -> Result<(), OutboundRouteError> {
    trace.encoded_started();
    let command = match route.try_send(command) {
        Ok(()) => {
            trace.encoded_finished();
            trace.handed_off(packet);
            return Ok(());
        }
        Err(MailboxTrySendError::Full(command)) => command,
        Err(MailboxTrySendError::Closed(_)) => {
            trace.encoded_finished();
            return Err(OutboundRouteError::Closed { packet });
        }
        Err(MailboxTrySendError::Oversized { actual, max, .. }) => {
            trace.encoded_finished();
            return Err(OutboundRouteError::Oversized {
                packet,
                actual,
                max,
            });
        }
    };
    let result = timeout(drain_timeout, route.send(command)).await;
    trace.encoded_finished();
    match result {
        Ok(Ok(())) => {
            trace.handed_off(packet);
            Ok(())
        }
        Ok(Err(MailboxSendError::Closed(_))) => Err(OutboundRouteError::Closed { packet }),
        Ok(Err(MailboxSendError::Oversized { actual, max, .. })) => {
            Err(OutboundRouteError::Oversized {
                packet,
                actual,
                max,
            })
        }
        Err(_) => Err(OutboundRouteError::Timeout),
    }
}

async fn send_outbound_route_events(
    pending: PendingRoute,
    compression_threshold: Option<i32>,
    content_registry: &Arc<ContentRegistry>,
    drain_timeout: Duration,
) -> RouteDeliveryOutcome {
    let mut trace = RouteDeliveryTrace::default();
    for event in pending.events {
        match event {
            PendingRouteEvent::Send(packet) => {
                let packet_kind = world_packet_kind(&packet);
                let command_result = if is_heavy_world_packet(&packet) {
                    let content_registry = Arc::clone(content_registry);
                    run_blocking_packet_encode(move || {
                        encode_and_prepare_play_packet(
                            *packet,
                            compression_threshold,
                            &content_registry,
                        )
                    })
                    .await
                } else {
                    match encode_world_packet(*packet, content_registry) {
                        Ok(ConnectionCommand::SendPlayPacket { packet_id, payload })
                            if payload.len() >= PRECOMPRESS_PLAY_PACKET_BYTES
                                && needs_wire_compression(payload.len(), compression_threshold) =>
                        {
                            run_blocking_packet_encode(move || {
                                PreparedPlayPacket::encode(
                                    packet_id,
                                    payload,
                                    compression_threshold,
                                )
                                .map(ConnectionCommand::SendPreparedPlayPacket)
                                .map_err(Arc::new)
                            })
                            .await
                        }
                        other => other.map_err(Arc::new),
                    }
                };
                let command = match command_result {
                    Ok(command) => command,
                    Err(error) => {
                        return RouteDeliveryOutcome::failed(
                            trace,
                            OutboundRouteError::Encode {
                                packet: packet_kind,
                                error,
                            },
                        );
                    }
                };
                if let Err(error) = handoff_outbound_command(
                    &pending.route,
                    &mut trace,
                    packet_kind,
                    command,
                    drain_timeout,
                )
                .await
                {
                    return RouteDeliveryOutcome::failed(trace, error);
                }
            }
            PendingRouteEvent::Shared(packet) => {
                let command = match packet.command().await {
                    Ok(command) => command,
                    Err(error) => {
                        return RouteDeliveryOutcome::failed(
                            trace,
                            OutboundRouteError::Encode {
                                packet: packet.kind,
                                error,
                            },
                        )
                    }
                };
                if let Err(error) = handoff_outbound_command(
                    &pending.route,
                    &mut trace,
                    packet.kind,
                    command,
                    drain_timeout,
                )
                .await
                {
                    return RouteDeliveryOutcome::failed(trace, error);
                }
            }
            PendingRouteEvent::SharedChunk(packet) => {
                let command = match packet.command().await {
                    Ok(command) => command,
                    Err(error) => {
                        return RouteDeliveryOutcome::failed(
                            trace,
                            OutboundRouteError::Encode {
                                packet: "level_chunk_with_light",
                                error,
                            },
                        );
                    }
                };
                if let Err(error) = handoff_outbound_command(
                    &pending.route,
                    &mut trace,
                    "level_chunk_with_light",
                    command,
                    drain_timeout,
                )
                .await
                {
                    return RouteDeliveryOutcome::failed(trace, error);
                }
            }
            PendingRouteEvent::Close(reason) => {
                // `ServerCommonPacketListenerImpl.java:177-184` hands off the
                // reason before closing; this route future awaits that order.
                if let Some(component) = reason.reason_component() {
                    let (packet_id, payload) = match encode_play_clientbound_disconnect(&component)
                    {
                        Ok(encoded) => encoded,
                        Err(error) => {
                            return RouteDeliveryOutcome::failed(
                                trace,
                                OutboundRouteError::Encode {
                                    packet: "disconnect",
                                    error: Arc::new(error),
                                },
                            );
                        }
                    };
                    if let Err(error) = handoff_outbound_command(
                        &pending.route,
                        &mut trace,
                        "disconnect",
                        ConnectionCommand::SendPlayPacket {
                            packet_id,
                            payload: Arc::new(payload),
                        },
                        drain_timeout,
                    )
                    .await
                    {
                        return RouteDeliveryOutcome::failed(trace, error);
                    }
                }
                if let Err(error) = handoff_outbound_command(
                    &pending.route,
                    &mut trace,
                    "close",
                    ConnectionCommand::Close,
                    drain_timeout,
                )
                .await
                {
                    return RouteDeliveryOutcome::failed(trace, error);
                }
                return RouteDeliveryOutcome { error: None, trace };
            }
        }
    }
    RouteDeliveryOutcome { error: None, trace }
}

async fn deliver_outbound_route_events(
    connection_id: ConnectionId,
    pending: PendingRoute,
    compression_threshold: Option<i32>,
    content_registry: &Arc<ContentRegistry>,
    drain_timeout: Duration,
) -> (ConnectionId, RouteDeliveryOutcome) {
    let close_route = pending.route.clone();
    let outcome = send_outbound_route_events(
        pending,
        compression_threshold,
        content_registry,
        drain_timeout,
    )
    .await;
    if outcome.error.is_some() {
        close_route.close();
    }
    (connection_id, outcome)
}

fn log_outbound_route_error(
    connection_id: ConnectionId,
    error: OutboundRouteError,
    drain_timeout: Duration,
) {
    match error {
        OutboundRouteError::Encode { packet, error } => tracing::error!(
            event = "world.outbound_encode_failed",
            connection_id = connection_id.0,
            packet,
            %error,
        ),
        OutboundRouteError::Closed { packet } => tracing::warn!(
            event = "world.outbound_closed",
            connection_id = connection_id.0,
            packet,
        ),
        OutboundRouteError::Oversized {
            packet,
            actual,
            max,
        } => tracing::warn!(
            event = "world.outbound_oversized",
            connection_id = connection_id.0,
            packet,
            actual,
            max,
        ),
        OutboundRouteError::Timeout => tracing::warn!(
            event = "world.outbound_backpressure_timeout",
            connection_id = connection_id.0,
            timeout_millis = drain_timeout.as_millis(),
        ),
    }
}

fn complete_admin_outcome(gateway: &RuntimeGateway, outcome: AdminOutcome) {
    let error = outcome.result.as_ref().err().map(ToString::to_string);
    tracing::info!(
        event = "admin.operation",
        operation_id = outcome.operation_id,
        request_id = %outcome.request_id,
        principal = %outcome.principal,
        scope = outcome.scope.name(),
        action = outcome.action,
        tick = outcome.tick,
        replayed = outcome.replayed,
        outcome = if error.is_none() { "ok" } else { "refused" },
        refusal = error.as_deref().unwrap_or_default(),
    );
    gateway.complete(outcome);
}

struct RouteWorkBatch {
    pending: PendingRoute,
    event_count: usize,
    compression_threshold: Option<i32>,
    barrier: Option<oneshot::Sender<()>>,
}

struct RouteWorkerHandle {
    route: ConnectionCommandSender,
    sender: tokio::sync::mpsc::Sender<RouteWorkBatch>,
    queued_events: Arc<std::sync::atomic::AtomicUsize>,
    join: tokio::task::JoinHandle<()>,
}

async fn run_route_worker(
    connection_id: ConnectionId,
    mut rx: tokio::sync::mpsc::Receiver<RouteWorkBatch>,
    routes: Arc<Routes>,
    queued_events: Arc<std::sync::atomic::AtomicUsize>,
    content_registry: Arc<ContentRegistry>,
    drain_timeout: Duration,
) {
    while let Some(batch) = rx.recv().await {
        let (_, outcome) = deliver_outbound_route_events(
            connection_id,
            batch.pending,
            batch.compression_threshold,
            &content_registry,
            drain_timeout,
        )
        .await;
        queued_events.fetch_sub(batch.event_count, std::sync::atomic::Ordering::AcqRel);
        let failed = if let Some(error) = outcome.error {
            log_outbound_route_error(connection_id, error, drain_timeout);
            routes
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .remove(&connection_id);
            true
        } else {
            false
        };
        drop(batch.barrier);
        if failed {
            rx.close();
            while let Some(dropped) = rx.recv().await {
                queued_events.fetch_sub(dropped.event_count, std::sync::atomic::Ordering::AcqRel);
                drop(dropped.barrier);
            }
            break;
        }
    }
}

pub(super) struct RouteDispatchEnv<'a> {
    pub(super) routes: &'a Arc<Routes>,
    pub(super) compression_threshold: Option<i32>,
    pub(super) content_registry: &'a Arc<ContentRegistry>,
    pub(super) gateway: &'a Arc<RuntimeGateway>,
    pub(super) network: &'a tokio::runtime::Handle,
    pub(super) drain_timeout: Duration,
}

#[derive(Default)]
pub(super) struct AsyncRouteDispatcher {
    workers: HashMap<ConnectionId, RouteWorkerHandle>,
    finished_joins: Vec<tokio::task::JoinHandle<()>>,
    chunk_wires: ChunkWireCache,
    last_admin_barrier: Option<oneshot::Receiver<()>>,
}

impl AsyncRouteDispatcher {
    pub(super) fn dispatch(
        &mut self,
        events: Vec<WorldEvent>,
        env: RouteDispatchEnv<'_>,
        requests: &mut TickRequests,
    ) {
        let RouteDispatchEnv {
            routes,
            compression_threshold,
            content_registry,
            gateway,
            network,
            drain_timeout,
        } = env;
        self.workers
            .retain(|_, worker| !worker.sender.is_closed() && !worker.route.is_closed());
        self.finished_joins.retain(|join| !join.is_finished());
        let mut unavailable = BTreeSet::new();
        let mut sealed = BTreeSet::new();
        let mut reported_after_terminal = BTreeSet::new();
        let mut events = events.into_iter();

        loop {
            let mut admin_barrier = None;
            let mut segment_routes: BTreeMap<ConnectionId, PendingRoute> = BTreeMap::new();
            for event in events.by_ref() {
                match event {
                    WorldEvent::Admin(outcome) => {
                        admin_barrier = Some(outcome);
                        break;
                    }
                    WorldEvent::SaveRequested { flush, trigger } => {
                        requests.save = Some(merge_save_request(requests.save, flush, trigger));
                    }
                    WorldEvent::ReloadRequested { selected } => requests.reload = Some(selected),
                    WorldEvent::StopRequested => requests.stop = true,
                    WorldEvent::ConsoleMessage(message) => {
                        tracing::info!(event = "world.console_message", message = message.text())
                    }
                    WorldEvent::Broadcast { recipients, packet } => {
                        let shared = Arc::new(SharedWorldPacket::new(
                            packet,
                            compression_threshold,
                            Arc::clone(content_registry),
                        ));
                        for connection_id in recipients {
                            if unavailable.contains(&connection_id)
                                || reject_terminal_send(
                                    connection_id,
                                    &sealed,
                                    &mut reported_after_terminal,
                                )
                            {
                                continue;
                            }
                            let Some(route) = self.ensure_route(
                                connection_id,
                                "send",
                                routes,
                                &mut unavailable,
                                content_registry,
                                network,
                                drain_timeout,
                            ) else {
                                continue;
                            };
                            push_pending_event(
                                &mut segment_routes,
                                connection_id,
                                route,
                                PendingRouteEvent::Shared(Arc::clone(&shared)),
                            );
                        }
                    }
                    WorldEvent::Send {
                        connection_id,
                        packet,
                    } => {
                        if unavailable.contains(&connection_id)
                            || reject_terminal_send(
                                connection_id,
                                &sealed,
                                &mut reported_after_terminal,
                            )
                        {
                            continue;
                        }
                        let Some(route) = self.ensure_route(
                            connection_id,
                            "send",
                            routes,
                            &mut unavailable,
                            content_registry,
                            network,
                            drain_timeout,
                        ) else {
                            continue;
                        };
                        let staged = stage_unicast_packet(
                            &mut self.chunk_wires,
                            packet,
                            compression_threshold,
                        );
                        push_pending_event(&mut segment_routes, connection_id, route, staged);
                    }
                    WorldEvent::Close {
                        connection_id,
                        reason,
                    } => {
                        tracing::info!(
                            event = "world.connection_closed",
                            connection_id = connection_id.0,
                            ?reason,
                        );
                        if unavailable.contains(&connection_id) || sealed.contains(&connection_id) {
                            continue;
                        }
                        let Some(route) = self.ensure_route(
                            connection_id,
                            "close",
                            routes,
                            &mut unavailable,
                            content_registry,
                            network,
                            drain_timeout,
                        ) else {
                            continue;
                        };
                        push_pending_event(
                            &mut segment_routes,
                            connection_id,
                            route,
                            PendingRouteEvent::Close(reason),
                        );
                        routes
                            .lock()
                            .unwrap_or_else(PoisonError::into_inner)
                            .remove(&connection_id);
                        sealed.insert(connection_id);
                    }
                }
            }

            let mut segment_barriers = Vec::new();
            let need_barriers = admin_barrier.is_some();
            for (connection_id, pending) in segment_routes {
                let event_count = pending.events.len();
                let Some(worker) = self.workers.get(&connection_id) else {
                    continue;
                };
                let previous_queued = worker
                    .queued_events
                    .fetch_add(event_count, std::sync::atomic::Ordering::AcqRel);
                if previous_queued.saturating_add(event_count) > MAX_QUEUED_ROUTE_EVENTS {
                    worker
                        .queued_events
                        .fetch_sub(event_count, std::sync::atomic::Ordering::AcqRel);
                    tracing::warn!(
                        event = "world.outbound_queue_overflow",
                        connection_id = connection_id.0,
                        queued_events = previous_queued.saturating_add(event_count),
                        max_events = MAX_QUEUED_ROUTE_EVENTS,
                    );
                    self.abort_worker(connection_id, routes, &mut unavailable);
                    continue;
                }
                let (barrier_tx, barrier_rx) = if need_barriers {
                    let (tx, rx) = oneshot::channel();
                    (Some(tx), Some(rx))
                } else {
                    (None, None)
                };
                match worker.sender.try_send(RouteWorkBatch {
                    pending,
                    event_count,
                    compression_threshold,
                    barrier: barrier_tx,
                }) {
                    Ok(()) => {
                        if let Some(rx) = barrier_rx {
                            segment_barriers.push(rx);
                        }
                        if sealed.contains(&connection_id) {
                            if let Some(closed_worker) = self.workers.remove(&connection_id) {
                                self.finished_joins.push(closed_worker.join);
                            }
                        }
                    }
                    Err(tokio::sync::mpsc::error::TrySendError::Full(_)) => {
                        worker
                            .queued_events
                            .fetch_sub(event_count, std::sync::atomic::Ordering::AcqRel);
                        tracing::warn!(
                            event = "world.outbound_queue_overflow",
                            connection_id = connection_id.0,
                            queued_batches = MAX_QUEUED_ROUTE_BATCHES,
                        );
                        self.abort_worker(connection_id, routes, &mut unavailable);
                    }
                    Err(tokio::sync::mpsc::error::TrySendError::Closed(_)) => {
                        worker
                            .queued_events
                            .fetch_sub(event_count, std::sync::atomic::Ordering::AcqRel);
                        self.abort_worker(connection_id, routes, &mut unavailable);
                    }
                }
            }

            let Some(outcome) = admin_barrier else {
                break;
            };
            let prev_admin = self.last_admin_barrier.take();
            let (done_tx, done_rx) = oneshot::channel();
            self.last_admin_barrier = Some(done_rx);
            let gateway = Arc::clone(gateway);
            network.spawn(async move {
                if let Some(prev) = prev_admin {
                    let _ = prev.await;
                }
                for rx in segment_barriers {
                    let _ = rx.await;
                }
                complete_admin_outcome(&gateway, outcome);
                let _ = done_tx.send(());
            });
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn ensure_route(
        &mut self,
        connection_id: ConnectionId,
        action: &'static str,
        routes: &Arc<Routes>,
        unavailable: &mut BTreeSet<ConnectionId>,
        content_registry: &Arc<ContentRegistry>,
        network: &tokio::runtime::Handle,
        drain_timeout: Duration,
    ) -> Option<ConnectionCommandSender> {
        if let Some(worker) = self.workers.get(&connection_id) {
            if !worker.sender.is_closed() && !worker.route.is_closed() {
                return Some(worker.route.clone());
            }
            self.workers.remove(&connection_id);
        }
        let route = routes
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .get(&connection_id)
            .cloned();
        let Some(route) = route else {
            tracing::warn!(
                event = "world.route_missing",
                connection_id = connection_id.0,
                action,
            );
            unavailable.insert(connection_id);
            return None;
        };
        let (tx, rx) = tokio::sync::mpsc::channel(MAX_QUEUED_ROUTE_BATCHES);
        let queued_events = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let join = network.spawn(run_route_worker(
            connection_id,
            rx,
            Arc::clone(routes),
            Arc::clone(&queued_events),
            Arc::clone(content_registry),
            drain_timeout,
        ));
        self.workers.insert(
            connection_id,
            RouteWorkerHandle {
                route: route.clone(),
                sender: tx,
                queued_events,
                join,
            },
        );
        Some(route)
    }

    fn abort_worker(
        &mut self,
        connection_id: ConnectionId,
        routes: &Arc<Routes>,
        unavailable: &mut BTreeSet<ConnectionId>,
    ) {
        unavailable.insert(connection_id);
        if let Some(worker) = self.workers.remove(&connection_id) {
            worker.route.close();
            worker.join.abort();
        } else if let Some(route) = routes
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .get(&connection_id)
            .cloned()
        {
            route.close();
        }
        routes
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .remove(&connection_id);
    }

    pub(super) fn finish_shutdown(
        &mut self,
        routes: &Routes,
        network: &tokio::runtime::Handle,
        shutdown_timeout: Duration,
    ) {
        let mut joins = std::mem::take(&mut self.finished_joins);
        for (_, worker) in std::mem::take(&mut self.workers) {
            drop(worker.sender);
            joins.push(worker.join);
        }
        let last_admin = self.last_admin_barrier.take();
        if joins.is_empty() && last_admin.is_none() {
            return;
        }
        let abort_handles: Vec<_> = joins
            .iter()
            .map(tokio::task::JoinHandle::abort_handle)
            .collect();
        let completed = network.block_on(async move {
            timeout(shutdown_timeout, async move {
                if let Some(prev) = last_admin {
                    let _ = prev.await;
                }
                for join in joins {
                    let _ = join.await;
                }
            })
            .await
            .is_ok()
        });
        if !completed {
            for handle in abort_handles {
                handle.abort();
            }
            for route in routes
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .values()
            {
                route.close();
            }
        }
    }
}

fn push_pending_event(
    pending_routes: &mut BTreeMap<ConnectionId, PendingRoute>,
    connection_id: ConnectionId,
    route: ConnectionCommandSender,
    event: PendingRouteEvent,
) {
    pending_routes
        .entry(connection_id)
        .or_insert_with(|| PendingRoute {
            route,
            events: Vec::new(),
        })
        .events
        .push(event);
}

fn stage_unicast_packet(
    chunk_wires: &mut ChunkWireCache,
    packet: ClientboundPlayPacket,
    compression_threshold: Option<i32>,
) -> PendingRouteEvent {
    match packet {
        ClientboundPlayPacket::LevelChunkWithLight(chunk) => {
            PendingRouteEvent::SharedChunk(chunk_wires.prepare(chunk, compression_threshold))
        }
        other => PendingRouteEvent::Send(Box::new(other)),
    }
}

fn save_trigger_priority(trigger: SaveTrigger) -> u8 {
    match trigger {
        SaveTrigger::ChunkUnload => 0,
        SaveTrigger::Autosave => 1,
        SaveTrigger::Management => 2,
        SaveTrigger::Shutdown => 3,
    }
}

fn merge_save_request(
    existing: Option<(bool, SaveTrigger)>,
    flush: bool,
    trigger: SaveTrigger,
) -> (bool, SaveTrigger) {
    match existing {
        None => (flush, trigger),
        Some((prev_flush, prev_trigger)) => {
            let merged_trigger =
                if save_trigger_priority(trigger) >= save_trigger_priority(prev_trigger) {
                    trigger
                } else {
                    prev_trigger
                };
            (prev_flush || flush, merged_trigger)
        }
    }
}

fn reject_terminal_send(
    connection_id: ConnectionId,
    sealed: &BTreeSet<ConnectionId>,
    reported_after_terminal: &mut BTreeSet<ConnectionId>,
) -> bool {
    if !sealed.contains(&connection_id) {
        return false;
    }
    if reported_after_terminal.insert(connection_id) {
        tracing::warn!(
            event = "world.route_terminal",
            connection_id = connection_id.0,
            action = "send",
        );
    }
    true
}

#[cfg(test)]
pub(super) struct RouteDispatch<'a> {
    routes: &'a Routes,
    compression_threshold: Option<i32>,
    drain_timeout: Duration,
    chunk_wires: ChunkWireCache,
    pub(super) pending_routes: BTreeMap<ConnectionId, PendingRoute>,
    pub(super) senders: BTreeMap<ConnectionId, ConnectionCommandSender>,
    unavailable: BTreeSet<ConnectionId>,
    sealed: BTreeSet<ConnectionId>,
    reported_after_terminal: BTreeSet<ConnectionId>,
    pub(super) handed_off: BTreeMap<ConnectionId, Vec<&'static str>>,
    pub(super) max_encoded_in_flight: BTreeMap<ConnectionId, usize>,
    pub(super) failure_packets: BTreeMap<ConnectionId, &'static str>,
}

#[cfg(test)]
impl<'a> RouteDispatch<'a> {
    pub(super) fn is_unavailable(&self, connection_id: ConnectionId) -> bool {
        self.unavailable.contains(&connection_id)
    }

    pub(super) fn new(
        routes: &'a Routes,
        compression_threshold: Option<i32>,
        drain_timeout: Duration,
    ) -> Self {
        Self {
            routes,
            compression_threshold,
            drain_timeout,
            chunk_wires: ChunkWireCache::default(),
            pending_routes: BTreeMap::new(),
            senders: BTreeMap::new(),
            unavailable: BTreeSet::new(),
            sealed: BTreeSet::new(),
            reported_after_terminal: BTreeSet::new(),
            handed_off: BTreeMap::new(),
            max_encoded_in_flight: BTreeMap::new(),
            failure_packets: BTreeMap::new(),
        }
    }

    fn routes(&self) -> std::sync::MutexGuard<'a, BTreeMap<ConnectionId, ConnectionCommandSender>> {
        self.routes.lock().unwrap_or_else(PoisonError::into_inner)
    }

    fn route(
        &mut self,
        connection_id: ConnectionId,
        action: &'static str,
    ) -> Option<ConnectionCommandSender> {
        if let Some(route) = self.senders.get(&connection_id) {
            return Some(route.clone());
        }
        let route = self.routes().get(&connection_id).cloned();
        let Some(route) = route else {
            tracing::warn!(
                event = "world.route_missing",
                connection_id = connection_id.0,
                action,
            );
            self.fail_route(connection_id);
            return None;
        };
        self.senders.insert(connection_id, route.clone());
        Some(route)
    }

    fn fail_route(&mut self, connection_id: ConnectionId) {
        if !self.unavailable.insert(connection_id) {
            return;
        }
        self.pending_routes.remove(&connection_id);
        if let Some(route) = self
            .senders
            .get(&connection_id)
            .cloned()
            .or_else(|| self.routes().get(&connection_id).cloned())
        {
            route.close();
        }
        self.routes().remove(&connection_id);
    }

    pub(super) fn stage_send(
        &mut self,
        connection_id: ConnectionId,
        packet: ClientboundPlayPacket,
    ) {
        if self.unavailable.contains(&connection_id)
            || reject_terminal_send(
                connection_id,
                &self.sealed,
                &mut self.reported_after_terminal,
            )
        {
            return;
        }
        let Some(route) = self.route(connection_id, "send") else {
            return;
        };
        let staged =
            stage_unicast_packet(&mut self.chunk_wires, packet, self.compression_threshold);
        push_pending_event(&mut self.pending_routes, connection_id, route, staged);
    }

    pub(super) fn stage_broadcast(
        &mut self,
        recipients: Vec<ConnectionId>,
        packet: ClientboundPlayPacket,
    ) {
        let packet = Arc::new(SharedWorldPacket::new(
            packet,
            self.compression_threshold,
            Arc::clone(ContentRegistry::vanilla()),
        ));
        for connection_id in recipients {
            if self.unavailable.contains(&connection_id)
                || reject_terminal_send(
                    connection_id,
                    &self.sealed,
                    &mut self.reported_after_terminal,
                )
            {
                continue;
            }
            let Some(route) = self.route(connection_id, "send") else {
                continue;
            };
            push_pending_event(
                &mut self.pending_routes,
                connection_id,
                route,
                PendingRouteEvent::Shared(Arc::clone(&packet)),
            );
        }
    }

    pub(super) fn stage_close(&mut self, connection_id: ConnectionId, reason: WorldCloseReason) {
        if self.unavailable.contains(&connection_id) || self.sealed.contains(&connection_id) {
            return;
        }
        let Some(route) = self.route(connection_id, "close") else {
            return;
        };
        push_pending_event(
            &mut self.pending_routes,
            connection_id,
            route,
            PendingRouteEvent::Close(reason),
        );
        self.routes().remove(&connection_id);
        self.sealed.insert(connection_id);
    }

    pub(super) async fn flush(&mut self) -> Result<(), ServerRuntimeError> {
        if self.pending_routes.is_empty() {
            return Ok(());
        }
        let pending_routes = std::mem::take(&mut self.pending_routes);
        type RouteFuture =
            std::pin::Pin<Box<dyn Future<Output = (ConnectionId, RouteDeliveryOutcome)> + Send>>;
        let mut futures: Vec<Option<RouteFuture>> = pending_routes
            .into_iter()
            .map(|(connection_id, pending)| {
                let compression_threshold = self.compression_threshold;
                let drain_timeout = self.drain_timeout;
                Some(Box::pin(async move {
                    deliver_outbound_route_events(
                        connection_id,
                        pending,
                        compression_threshold,
                        ContentRegistry::vanilla(),
                        drain_timeout,
                    )
                    .await
                }) as RouteFuture)
            })
            .collect();
        let mut outcomes = Vec::with_capacity(futures.len());
        std::future::poll_fn(|cx| {
            let mut any_pending = false;
            for slot in &mut futures {
                if let Some(fut) = slot.as_mut() {
                    match fut.as_mut().poll(cx) {
                        std::task::Poll::Ready(outcome) => {
                            outcomes.push(outcome);
                            *slot = None;
                        }
                        std::task::Poll::Pending => {
                            any_pending = true;
                        }
                    }
                }
            }
            if any_pending {
                std::task::Poll::Pending
            } else {
                std::task::Poll::Ready(())
            }
        })
        .await;
        for (connection_id, outcome) in outcomes {
            self.handed_off
                .entry(connection_id)
                .or_default()
                .extend(outcome.trace.handed_off.iter().copied());
            self.max_encoded_in_flight
                .entry(connection_id)
                .and_modify(|maximum| {
                    *maximum = (*maximum).max(outcome.trace.max_encoded_in_flight)
                })
                .or_insert(outcome.trace.max_encoded_in_flight);
            if let Some(error) = outcome.error {
                self.failure_packets.insert(connection_id, error.packet());
                log_outbound_route_error(connection_id, error, self.drain_timeout);
                self.fail_route(connection_id);
            }
        }
        Ok(())
    }
}

#[cfg(test)]
#[derive(Debug)]
pub(super) struct RouteDispatchStats {
    pub(super) handed_off: BTreeMap<ConnectionId, Vec<&'static str>>,
    pub(super) unavailable: BTreeSet<ConnectionId>,
    pub(super) rejected_after_terminal: BTreeSet<ConnectionId>,
    pub(super) failure_packets: BTreeMap<ConnectionId, &'static str>,
}

#[cfg(test)]
pub(super) async fn dispatch_world_events(
    events: Vec<WorldEvent>,
    routes: &Routes,
    compression_threshold: Option<i32>,
    requests: &mut TickRequests,
    gateway: &RuntimeGateway,
    drain_timeout: Duration,
) -> Result<RouteDispatchStats, ServerRuntimeError> {
    let mut dispatch = RouteDispatch::new(routes, compression_threshold, drain_timeout);
    let mut events = events.into_iter();
    loop {
        let mut admin_barrier = None;
        for event in events.by_ref() {
            match event {
                WorldEvent::Admin(outcome) => {
                    admin_barrier = Some(outcome);
                    break;
                }
                WorldEvent::SaveRequested { flush, trigger } => {
                    requests.save = Some(merge_save_request(requests.save, flush, trigger));
                }
                WorldEvent::ReloadRequested { selected } => requests.reload = Some(selected),
                WorldEvent::StopRequested => requests.stop = true,
                WorldEvent::ConsoleMessage(message) => {
                    tracing::info!(event = "world.console_message", message = message.text(),)
                }
                WorldEvent::Broadcast { recipients, packet } => {
                    dispatch.stage_broadcast(recipients, packet)
                }
                WorldEvent::Send {
                    connection_id,
                    packet,
                } => dispatch.stage_send(connection_id, packet),
                WorldEvent::Close {
                    connection_id,
                    reason,
                } => {
                    tracing::info!(
                        event = "world.connection_closed",
                        connection_id = connection_id.0,
                        ?reason,
                    );
                    dispatch.stage_close(connection_id, reason);
                }
            }
        }

        dispatch.flush().await?;
        let Some(outcome) = admin_barrier else {
            break;
        };
        complete_admin_outcome(gateway, outcome);
    }
    Ok(RouteDispatchStats {
        handed_off: dispatch.handed_off,
        unavailable: dispatch.unavailable,
        rejected_after_terminal: dispatch.reported_after_terminal,
        failure_packets: dispatch.failure_packets,
    })
}
