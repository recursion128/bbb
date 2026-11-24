//! Owned lifecycle for the external connection.
//!
//! Vanilla closes the active `Connection` before releasing the remaining
//! client backends (`ClientLevel.disconnect`, `Minecraft.destroy/close`). BBB's
//! connection runs on Tokio rather than Netty, so this owner retains both tasks,
//! gives terminal shutdown a command-queue-independent signal, and joins it
//! before the event-loop closure releases renderer/audio/window state.

use std::sync::Arc;
#[cfg(not(target_arch = "wasm32"))]
use std::{future::Future, time::Duration};

use bbb_client_net::{
    ConnectionOptions, DisconnectReason, EventStreamExit, NetCommand, NetEvent, NetEventSender,
};
#[cfg(not(target_arch = "wasm32"))]
use thiserror::Error;
use tokio::sync::{mpsc, oneshot};
#[cfg(not(target_arch = "wasm32"))]
use tokio::{runtime::Runtime, task::JoinSet, time::timeout};

use crate::simulation::commands::LoopWake;

#[cfg(not(target_arch = "wasm32"))]
const NETWORK_EVENT_CAPACITY: usize = 262_144;
#[cfg(target_arch = "wasm32")]
pub(crate) const WEB_NETWORK_EVENT_CAPACITY: usize = 16_384;
pub(crate) const NETWORK_COMMAND_CAPACITY: usize = 8_192;
#[cfg(not(target_arch = "wasm32"))]
const NETWORK_SHUTDOWN_TIMEOUT: Duration = Duration::from_secs(5);

#[derive(Debug)]
enum NetworkConnectionExit {
    Local,
    Remote(DisconnectReason),
    Failure(DisconnectReason),
}

pub(crate) struct NetworkRuntime {
    events: Option<mpsc::Receiver<NetEvent>>,
    commands: Option<mpsc::Sender<NetCommand>>,
    connection_start: Option<oneshot::Sender<()>>,
    terminal_close: Option<oneshot::Sender<()>>,
    #[cfg(not(target_arch = "wasm32"))]
    tasks: JoinSet<()>,
}

#[cfg(not(target_arch = "wasm32"))]
#[derive(Debug, Error)]
pub(crate) enum NetworkShutdownError {
    #[error("network task failed during terminal shutdown: {0}")]
    Task(String),
    #[error("network tasks exceeded the {millis} ms terminal shutdown budget")]
    Timeout { millis: u128 },
}

impl NetworkRuntime {
    pub(crate) fn idle() -> Self {
        Self {
            events: None,
            commands: None,
            connection_start: None,
            terminal_close: None,
            #[cfg(not(target_arch = "wasm32"))]
            tasks: JoinSet::new(),
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn start(
        runtime: &Runtime,
        options: ConnectionOptions,
        wake: Arc<LoopWake>,
    ) -> Self {
        let (event_tx, event_rx) = mpsc::channel(NETWORK_EVENT_CAPACITY);
        let network_tx = NetEventSender::new(event_tx, move || wake.wake());
        let (command_tx, command_rx) = mpsc::channel(NETWORK_COMMAND_CAPACITY);
        let final_event_tx = network_tx.clone();
        let connection = async move {
            match bbb_client_net::run_event_stream(options, network_tx, command_rx).await {
                Ok(EventStreamExit::LocalDisconnect) => NetworkConnectionExit::Local,
                Ok(EventStreamExit::RemoteDisconnect(reason)) => {
                    NetworkConnectionExit::Remote(reason)
                }
                Err(error) => {
                    NetworkConnectionExit::Failure(format_network_disconnect_reason(&error))
                }
            }
        };
        let (connection_start, connection_start_rx) = oneshot::channel();
        let gated_connection = async move {
            if connection_start_rx.await.is_err() {
                return NetworkConnectionExit::Local;
            }
            connection.await
        };
        let mut owner = Self::spawn_tasks(
            runtime,
            event_rx,
            command_tx,
            final_event_tx,
            gated_connection,
        );
        owner.connection_start = Some(connection_start);
        owner
    }

    #[cfg(target_arch = "wasm32")]
    pub(crate) fn start_web(options: ConnectionOptions, wake: Arc<LoopWake>) -> Self {
        if let Some((event_rx, command_tx, connection_start, terminal_close)) =
            crate::web::net_worker::try_start_web_session(options.clone(), Arc::clone(&wake))
        {
            return Self {
                events: Some(event_rx),
                commands: Some(command_tx),
                connection_start: Some(connection_start),
                terminal_close: Some(terminal_close),
            };
        }

        let (event_tx, event_rx) = mpsc::channel(WEB_NETWORK_EVENT_CAPACITY);
        let network_tx = NetEventSender::new(event_tx, move || wake.wake());
        let (command_tx, command_rx) = mpsc::channel(NETWORK_COMMAND_CAPACITY);
        let final_event_tx = network_tx.clone();
        let (connection_start, connection_start_rx) = oneshot::channel();
        let (terminal_close, terminal_close_rx) = oneshot::channel();

        wasm_bindgen_futures::spawn_local(async move {
            let connection = async move {
                if connection_start_rx.await.is_err() {
                    return NetworkConnectionExit::Local;
                }
                match bbb_client_net::run_event_stream(options, network_tx, command_rx).await {
                    Ok(EventStreamExit::LocalDisconnect) => NetworkConnectionExit::Local,
                    Ok(EventStreamExit::RemoteDisconnect(reason)) => {
                        NetworkConnectionExit::Remote(reason)
                    }
                    Err(error) => {
                        NetworkConnectionExit::Failure(format_network_disconnect_reason(&error))
                    }
                }
            };
            let completion = tokio::select! {
                biased;
                _ = terminal_close_rx => NetworkConnectionExit::Local,
                completion = connection => completion,
            };
            let reason = match completion {
                NetworkConnectionExit::Local => None,
                NetworkConnectionExit::Remote(reason) | NetworkConnectionExit::Failure(reason) => {
                    Some(reason)
                }
            };
            if let Some(reason) = reason {
                let _ = final_event_tx
                    .send(NetEvent::Disconnected {
                        reason: Some(reason),
                    })
                    .await;
            }
        });

        Self {
            events: Some(event_rx),
            commands: Some(command_tx),
            connection_start: Some(connection_start),
            terminal_close: Some(terminal_close),
        }
    }

    #[cfg(target_arch = "wasm32")]
    pub(crate) fn signal_close_web(&mut self) {
        if let Some(terminal_close) = self.terminal_close.take() {
            let _ = terminal_close.send(());
        }
        self.connection_start.take();
    }

    pub(crate) fn take_channels(
        &mut self,
    ) -> (
        Option<mpsc::Receiver<NetEvent>>,
        Option<mpsc::Sender<NetCommand>>,
    ) {
        (self.events.take(), self.commands.take())
    }

    /// Release the external connection only after the native initial-load
    /// owner has installed the simulation consumer and saved its clean marker.
    /// An idle runtime has no activation sender and is already active.
    pub(crate) fn activate(&mut self) -> bool {
        self.connection_start
            .take()
            .is_none_or(|start| start.send(()).is_ok())
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn shutdown_and_join(
        &mut self,
        runtime: &Runtime,
    ) -> Result<(), NetworkShutdownError> {
        self.shutdown_and_join_with_timeout(runtime, NETWORK_SHUTDOWN_TIMEOUT)
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn shutdown_and_join_with_timeout(
        &mut self,
        runtime: &Runtime,
        shutdown_timeout: Duration,
    ) -> Result<(), NetworkShutdownError> {
        if self.tasks.is_empty() {
            self.connection_start.take();
            self.terminal_close.take();
            return Ok(());
        }

        if let Some(terminal_close) = self.terminal_close.take() {
            let _ = terminal_close.send(());
        }
        self.connection_start.take();

        runtime.block_on(async {
            let mut first_task_failure = None;
            let joined = timeout(shutdown_timeout, async {
                while let Some(result) = self.tasks.join_next().await {
                    if let Err(error) = result {
                        first_task_failure.get_or_insert_with(|| {
                            if error.is_panic() {
                                "network task panicked".to_string()
                            } else if error.is_cancelled() {
                                "network task was cancelled before terminal shutdown".to_string()
                            } else {
                                "network task ended with an unknown join failure".to_string()
                            }
                        });
                    }
                }
            })
            .await;

            if joined.is_err() {
                // JoinSet::shutdown aborts every remaining task and waits until
                // each cancellation has been observed, so the Tokio runtime
                // cannot retain a detached prior-session task.
                self.tasks.shutdown().await;
                return Err(NetworkShutdownError::Timeout {
                    millis: shutdown_timeout.as_millis(),
                });
            }

            first_task_failure
                .map(NetworkShutdownError::Task)
                .map_or(Ok(()), Err)
        })
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn spawn_tasks<F>(
        runtime: &Runtime,
        events: mpsc::Receiver<NetEvent>,
        command_tx: mpsc::Sender<NetCommand>,
        final_event_tx: NetEventSender,
        connection: F,
    ) -> Self
    where
        F: Future<Output = NetworkConnectionExit> + Send + 'static,
    {
        let (terminal_close, terminal_close_rx) = oneshot::channel();
        let mut tasks = JoinSet::new();
        tasks.spawn_on(
            async move {
                let completion = tokio::select! {
                    biased;
                    _ = terminal_close_rx => NetworkConnectionExit::Local,
                    completion = connection => completion,
                };
                let reason = match completion {
                    NetworkConnectionExit::Local => None,
                    NetworkConnectionExit::Remote(reason)
                    | NetworkConnectionExit::Failure(reason) => Some(reason),
                };
                if let Some(reason) = reason {
                    let _ = final_event_tx
                        .send(NetEvent::Disconnected {
                            reason: Some(reason),
                        })
                        .await;
                }
            },
            runtime.handle(),
        );

        Self {
            events: Some(events),
            commands: Some(command_tx),
            connection_start: None,
            terminal_close: Some(terminal_close),
            tasks,
        }
    }
}

/// Formats a connection or network stream failure with full cause context and
/// user-friendly Vanilla-style headings for common `std::io::ErrorKind`
/// transport failures.
pub(crate) fn format_network_disconnect_reason(error: &anyhow::Error) -> DisconnectReason {
    let chain = format!("{error:#}");
    let category = classify_network_error(error, &chain);
    let text = match category {
        Some(prefix) => format!("{prefix}: {chain}"),
        None => chain,
    };
    DisconnectReason::from_plain(text)
}

fn classify_network_error(error: &anyhow::Error, chain: &str) -> Option<&'static str> {
    for cause in error.chain() {
        if let Some(io_err) = cause.downcast_ref::<std::io::Error>() {
            match io_err.kind() {
                std::io::ErrorKind::ConnectionRefused => return Some("Connection refused"),
                std::io::ErrorKind::TimedOut => return Some("Connection timed out"),
                std::io::ErrorKind::AddrNotAvailable => {
                    return Some("Network address not available")
                }
                _ if is_dns_lookup_io_error(io_err) => return Some("Unknown host"),
                _ => {}
            }
        }
        if cause.is::<tokio::time::error::Elapsed>() {
            return Some("Connection timed out");
        }
    }
    if is_dns_failure_message(chain) {
        return Some("Unknown host");
    }
    None
}

fn is_dns_lookup_io_error(err: &std::io::Error) -> bool {
    if matches!(err.raw_os_error(), Some(11001 | 11002 | 11004)) {
        return true;
    }
    is_dns_failure_message(&err.to_string())
}

fn is_dns_failure_message(message: &str) -> bool {
    let lower = message.to_ascii_lowercase();
    [
        "failed to lookup address information",
        "name or service not known",
        "temporary failure in name resolution",
        "no such host is known",
        "nodename nor servname provided",
        "no address associated with hostname",
        "unknown host",
    ]
    .iter()
    .any(|needle| lower.contains(needle))
}

#[cfg(test)]
mod tests {
    use std::{
        future,
        sync::atomic::{AtomicBool, Ordering},
    };

    use super::*;

    fn test_sender(capacity: usize) -> (NetEventSender, mpsc::Receiver<NetEvent>) {
        let (tx, rx) = mpsc::channel(capacity);
        (NetEventSender::new(tx, || {}), rx)
    }

    fn test_runtime_with_connection<F>(
        runtime: &Runtime,
        connection: F,
        event_capacity: usize,
    ) -> NetworkRuntime
    where
        F: Future<Output = NetworkConnectionExit> + Send + 'static,
    {
        let (event_tx, event_rx) = test_sender(event_capacity);
        let (command_tx, command_rx) = mpsc::channel(1);
        let connection = async move {
            let _command_rx = command_rx;
            connection.await
        };
        NetworkRuntime::spawn_tasks(runtime, event_rx, command_tx, event_tx, connection)
    }

    #[test]
    fn idle_shutdown_is_idempotent() {
        let runtime = Runtime::new().unwrap();
        let mut network = NetworkRuntime::idle();

        network.shutdown_and_join(&runtime).unwrap();
        network.shutdown_and_join(&runtime).unwrap();

        assert!(network.tasks.is_empty());
    }

    #[test]
    fn external_connection_is_not_polled_until_initial_load_activation() {
        let runtime = Runtime::new().unwrap();
        let polled = Arc::new(AtomicBool::new(false));
        let connection_polled = Arc::clone(&polled);
        let connection = future::poll_fn(move |_| {
            connection_polled.store(true, Ordering::SeqCst);
            std::task::Poll::Pending
        });
        let (event_tx, event_rx) = test_sender(4);
        let (command_tx, _command_rx) = mpsc::channel(1);
        let (connection_start, connection_start_rx) = oneshot::channel();
        let gated_connection = async move {
            if connection_start_rx.await.is_err() {
                return NetworkConnectionExit::Local;
            }
            connection.await
        };
        let mut owner =
            NetworkRuntime::spawn_tasks(&runtime, event_rx, command_tx, event_tx, gated_connection);
        owner.connection_start = Some(connection_start);

        runtime.block_on(tokio::task::yield_now());
        assert!(!polled.load(Ordering::SeqCst));
        assert!(owner.activate());
        runtime
            .block_on(async {
                timeout(Duration::from_secs(1), async {
                    while !polled.load(Ordering::SeqCst) {
                        tokio::task::yield_now().await;
                    }
                })
                .await
            })
            .expect("activated connection should be polled");
        assert!(polled.load(Ordering::SeqCst));
        owner.shutdown_and_join(&runtime).unwrap();
    }

    #[test]
    fn terminal_signal_cancels_connection_independently_of_command_queue() {
        let runtime = Runtime::new().unwrap();
        let mut network = test_runtime_with_connection(&runtime, future::pending(), 4);
        let (_events, commands) = network.take_channels();
        let commands = commands.unwrap();
        commands
            .try_send(NetCommand::Disconnect)
            .expect("fill the only command slot");

        network.shutdown_and_join(&runtime).unwrap();

        assert!(network.tasks.is_empty());
    }

    #[test]
    fn terminal_close_does_not_fabricate_remote_disconnect() {
        let runtime = Runtime::new().unwrap();
        let mut network = test_runtime_with_connection(&runtime, future::pending(), 4);
        let (events, _commands) = network.take_channels();
        let mut events = events.unwrap();

        network.shutdown_and_join(&runtime).unwrap();

        assert!(events.blocking_recv().is_none());
        assert!(events.try_recv().is_err());
        network.shutdown_and_join(&runtime).unwrap();
    }

    #[test]
    fn closed_event_receiver_does_not_stall_join() {
        let runtime = Runtime::new().unwrap();
        let mut network = test_runtime_with_connection(
            &runtime,
            future::ready(NetworkConnectionExit::Remote(DisconnectReason::from_plain(
                "gone",
            ))),
            1,
        );
        let (events, _commands) = network.take_channels();
        drop(events);

        network.shutdown_and_join(&runtime).unwrap();

        assert!(network.tasks.is_empty());
    }

    #[test]
    fn task_panic_is_returned_after_the_other_task_joins() {
        let runtime = Runtime::new().unwrap();
        let (started_tx, started_rx) = oneshot::channel();
        let mut network = test_runtime_with_connection(
            &runtime,
            async move {
                let _ = started_tx.send(());
                panic!("synthetic connection panic");
            },
            1,
        );
        let (events, _commands) = network.take_channels();
        drop(events);
        runtime.block_on(started_rx).unwrap();

        let error = network.shutdown_and_join(&runtime).unwrap_err();

        assert!(matches!(error, NetworkShutdownError::Task(_)));
        assert!(error.to_string().contains("panicked"));
        assert!(network.tasks.is_empty());
    }

    #[test]
    fn shared_deadline_aborts_and_reaps_a_blocked_final_event() {
        let runtime = Runtime::new().unwrap();
        let (event_tx, event_rx) = test_sender(1);
        runtime
            .block_on(event_tx.send(NetEvent::Disconnected { reason: None }))
            .unwrap();
        let (command_tx, _command_rx) = mpsc::channel(1);
        let (ended_tx, ended_rx) = oneshot::channel();
        let connection = async move {
            let _ = ended_tx.send(());
            NetworkConnectionExit::Remote(DisconnectReason::from_plain("full"))
        };
        let mut network =
            NetworkRuntime::spawn_tasks(&runtime, event_rx, command_tx, event_tx, connection);
        let (_events, _commands) = network.take_channels();
        runtime.block_on(ended_rx).unwrap();

        let error = network
            .shutdown_and_join_with_timeout(&runtime, Duration::from_millis(20))
            .unwrap_err();

        assert!(matches!(
            error,
            NetworkShutdownError::Timeout { millis: 20 }
        ));
        assert!(network.tasks.is_empty());
    }

    #[test]
    fn events_keep_order_before_natural_connection_end() {
        let runtime = Runtime::new().unwrap();
        let (event_tx, event_rx) = test_sender(4);
        let (command_tx, command_rx) = mpsc::channel(1);
        let ordered_event_tx = event_tx.clone();
        let connection = async move {
            let _command_rx = command_rx;
            for reason in ["first", "second", "third"] {
                ordered_event_tx
                    .send(NetEvent::Disconnected {
                        reason: Some(DisconnectReason::from_plain(reason)),
                    })
                    .await
                    .unwrap();
            }
            NetworkConnectionExit::Remote(DisconnectReason::from_plain("natural end"))
        };
        let mut network =
            NetworkRuntime::spawn_tasks(&runtime, event_rx, command_tx, event_tx, connection);
        let (events, _commands) = network.take_channels();
        let mut events = events.unwrap();

        let reasons = (0..4)
            .map(|_| match events.blocking_recv().unwrap() {
                NetEvent::Disconnected { reason } => reason.map(|reason| reason.summary()),
                event => panic!("unexpected event: {event:?}"),
            })
            .collect::<Vec<_>>();

        assert_eq!(
            reasons,
            vec![
                Some("first".to_string()),
                Some("second".to_string()),
                Some("third".to_string()),
                Some("natural end".to_string()),
            ]
        );
        network.shutdown_and_join(&runtime).unwrap();
    }

    #[test]
    fn disconnect_reason_formats_full_chain_and_maps_common_io_errors() {
        let refused = anyhow::Error::new(std::io::Error::new(
            std::io::ErrorKind::ConnectionRefused,
            "os error 111",
        ))
        .context("connect 127.0.0.1:25565");
        let refused_text = format_network_disconnect_reason(&refused).summary();
        assert!(refused_text.contains("Connection refused"));
        assert!(refused_text.contains("connect 127.0.0.1:25565"));
        assert!(refused_text.contains("os error 111"));

        let timed_out = anyhow::Error::new(std::io::Error::new(
            std::io::ErrorKind::TimedOut,
            "deadline elapsed",
        ))
        .context("connect example.org:25565");
        let timeout_text = format_network_disconnect_reason(&timed_out).summary();
        assert!(timeout_text.contains("Connection timed out"));
        assert!(timeout_text.contains("connect example.org:25565"));
        assert!(timeout_text.contains("deadline elapsed"));

        let addr_unavail = anyhow::Error::new(std::io::Error::new(
            std::io::ErrorKind::AddrNotAvailable,
            "cannot assign requested address",
        ))
        .context("connect 192.0.2.1:25565");
        let addr_text = format_network_disconnect_reason(&addr_unavail).summary();
        assert!(addr_text.contains("Network address not available"));
        assert!(addr_text.contains("cannot assign requested address"));

        let dns = anyhow::Error::new(std::io::Error::other(
            "failed to lookup address information: Name or service not known",
        ))
        .context("connect invalid.example:25565");
        let dns_text = format_network_disconnect_reason(&dns).summary();
        assert!(dns_text.contains("Unknown host"));
        assert!(dns_text.contains("connect invalid.example:25565"));
        assert!(dns_text.contains("Name or service not known"));

        let nested = anyhow::anyhow!("inner handshake decode error").context("login phase");
        assert_eq!(
            format_network_disconnect_reason(&nested).summary(),
            "login phase: inner handshake decode error"
        );
    }
}
