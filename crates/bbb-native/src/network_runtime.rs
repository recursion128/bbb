//! Owned lifecycle for the external connection and its event relay.
//!
//! Vanilla closes the active `Connection` before releasing the remaining
//! client backends (`ClientLevel.disconnect`, `Minecraft.destroy/close`). BBB's
//! connection runs on Tokio rather than Netty, so this owner retains both tasks,
//! gives terminal shutdown a command-queue-independent signal, and joins the
//! pair before the event-loop closure releases renderer/audio/window state.

use std::{future::Future, time::Duration};

use bbb_net::{ConnectionOptions, DisconnectReason, EventStreamExit, NetCommand, NetEvent};
use thiserror::Error;
use tokio::{
    runtime::Runtime,
    sync::{mpsc, oneshot},
    task::JoinSet,
    time::timeout,
};

use crate::simulation::commands::SimulationMailbox;

const NETWORK_EVENT_CAPACITY: usize = 262_144;
const SIMULATION_EVENT_CAPACITY: usize = 262_144;
const NETWORK_COMMAND_CAPACITY: usize = 8_192;
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
    tasks: JoinSet<()>,
}

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
            tasks: JoinSet::new(),
        }
    }

    pub(crate) fn start(
        runtime: &Runtime,
        options: ConnectionOptions,
        simulation_mailbox: SimulationMailbox,
    ) -> Self {
        let (network_tx, network_rx) = mpsc::channel(NETWORK_EVENT_CAPACITY);
        let (simulation_tx, simulation_rx) = mpsc::channel(SIMULATION_EVENT_CAPACITY);
        let (command_tx, command_rx) = mpsc::channel(NETWORK_COMMAND_CAPACITY);
        let final_event_tx = network_tx.clone();
        let connection = async move {
            match bbb_net::run_event_stream(options, network_tx, command_rx).await {
                Ok(EventStreamExit::LocalDisconnect) => NetworkConnectionExit::Local,
                Ok(EventStreamExit::RemoteDisconnect(reason)) => {
                    NetworkConnectionExit::Remote(reason)
                }
                Err(error) => {
                    NetworkConnectionExit::Failure(DisconnectReason::from_plain(error.to_string()))
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
            simulation_mailbox,
            network_rx,
            simulation_tx,
            simulation_rx,
            command_tx,
            final_event_tx,
            gated_connection,
        );
        owner.connection_start = Some(connection_start);
        owner
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

    pub(crate) fn shutdown_and_join(
        &mut self,
        runtime: &Runtime,
    ) -> Result<(), NetworkShutdownError> {
        self.shutdown_and_join_with_timeout(runtime, NETWORK_SHUTDOWN_TIMEOUT)
    }

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

    #[allow(clippy::too_many_arguments)]
    fn spawn_tasks<F>(
        runtime: &Runtime,
        simulation_mailbox: SimulationMailbox,
        mut network_rx: mpsc::Receiver<NetEvent>,
        simulation_tx: mpsc::Sender<NetEvent>,
        simulation_rx: mpsc::Receiver<NetEvent>,
        command_tx: mpsc::Sender<NetCommand>,
        final_event_tx: mpsc::Sender<NetEvent>,
        connection: F,
    ) -> Self
    where
        F: Future<Output = NetworkConnectionExit> + Send + 'static,
    {
        let (terminal_close, terminal_close_rx) = oneshot::channel();
        let mut tasks = JoinSet::new();
        let relay_mailbox = simulation_mailbox;
        tasks.spawn_on(
            async move {
                while let Some(event) = network_rx.recv().await {
                    if simulation_tx.send(event).await.is_err() {
                        break;
                    }
                    relay_mailbox.notify_external_work();
                }
            },
            runtime.handle(),
        );
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
            events: Some(simulation_rx),
            commands: Some(command_tx),
            connection_start: None,
            terminal_close: Some(terminal_close),
            tasks,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{
        future,
        sync::{
            atomic::{AtomicBool, Ordering},
            Arc,
        },
    };

    use super::*;

    fn test_runtime_with_connection<F>(
        runtime: &Runtime,
        connection: F,
        simulation_capacity: usize,
    ) -> NetworkRuntime
    where
        F: Future<Output = NetworkConnectionExit> + Send + 'static,
    {
        let (network_tx, network_rx) = mpsc::channel(4);
        let (simulation_tx, simulation_rx) = mpsc::channel(simulation_capacity);
        let (command_tx, command_rx) = mpsc::channel(1);
        let connection = async move {
            let _command_rx = command_rx;
            connection.await
        };
        NetworkRuntime::spawn_tasks(
            runtime,
            SimulationMailbox::default(),
            network_rx,
            simulation_tx,
            simulation_rx,
            command_tx,
            network_tx,
            connection,
        )
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
        let (network_tx, network_rx) = mpsc::channel(4);
        let (simulation_tx, simulation_rx) = mpsc::channel(4);
        let (command_tx, _command_rx) = mpsc::channel(1);
        let (connection_start, connection_start_rx) = oneshot::channel();
        let gated_connection = async move {
            if connection_start_rx.await.is_err() {
                return NetworkConnectionExit::Local;
            }
            connection.await
        };
        let mut owner = NetworkRuntime::spawn_tasks(
            &runtime,
            SimulationMailbox::default(),
            network_rx,
            simulation_tx,
            simulation_rx,
            command_tx,
            network_tx,
            gated_connection,
        );
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
    fn closed_simulation_receiver_does_not_stall_relay_join() {
        let runtime = Runtime::new().unwrap();
        let mut network = test_runtime_with_connection(&runtime, future::pending(), 1);
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
    fn shared_deadline_aborts_and_reaps_a_blocked_relay() {
        let runtime = Runtime::new().unwrap();
        let (network_tx, network_rx) = mpsc::channel(2);
        let (simulation_tx, simulation_rx) = mpsc::channel(1);
        runtime
            .block_on(simulation_tx.send(NetEvent::Disconnected { reason: None }))
            .unwrap();
        runtime
            .block_on(network_tx.send(NetEvent::Disconnected { reason: None }))
            .unwrap();
        let (command_tx, _command_rx) = mpsc::channel(1);
        let mut network = NetworkRuntime::spawn_tasks(
            &runtime,
            SimulationMailbox::default(),
            network_rx,
            simulation_tx,
            simulation_rx,
            command_tx,
            network_tx,
            future::pending(),
        );

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
    fn event_relay_preserves_order_before_natural_connection_end() {
        let runtime = Runtime::new().unwrap();
        let (network_tx, network_rx) = mpsc::channel(4);
        let (simulation_tx, simulation_rx) = mpsc::channel(4);
        let (command_tx, command_rx) = mpsc::channel(1);
        let ordered_event_tx = network_tx.clone();
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
        let mut network = NetworkRuntime::spawn_tasks(
            &runtime,
            SimulationMailbox::default(),
            network_rx,
            simulation_tx,
            simulation_rx,
            command_tx,
            network_tx,
            connection,
        );
        let (events, _commands) = network.take_channels();
        let mut events = events.unwrap();

        let reasons = (0..4)
            .map(|_| match events.blocking_recv().unwrap() {
                NetEvent::Disconnected { reason } => reason.map(|reason| reason.summary()),
                event => panic!("unexpected relayed event: {event:?}"),
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
    fn production_shutdown_has_one_owner_and_frozen_order() {
        let startup = include_str!("startup.rs");
        let main = include_str!("main.rs");
        assert!(!startup.contains("struct NetworkHandles"));
        let network_startup = startup
            .split("pub(crate) fn start_network_if_requested")
            .nth(1)
            .expect("network startup function")
            .split("fn connection_options_from_args")
            .next()
            .unwrap();
        assert!(!network_startup.contains("runtime.spawn"));
        assert_eq!(
            main.matches("network_runtime.shutdown_and_join(&runtime)")
                .count(),
            1
        );
        let loop_exit = main
            .split("if matches!(&event, Event::LoopExiting)")
            .nth(1)
            .expect("LoopExiting branch");
        let simulation_join = loop_exit
            .find("simulation_thread.shutdown_and_join()")
            .expect("simulation join");
        let network_join = loop_exit
            .find("network_runtime.shutdown_and_join(&runtime)")
            .expect("network join");
        let final_snapshot = loop_exit.find("snapshot.rcu").expect("final snapshot");
        assert!(simulation_join < network_join);
        assert!(network_join < final_snapshot);
    }
}
