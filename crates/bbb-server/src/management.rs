//! The runtime half of the management surface.
//!
//! Vanilla owner: `MinecraftExecutorServiceImpl` (MinecraftExecutorServiceImpl.java:7-23),
//! which submits a management operation to the server thread and completes the
//! caller's future when the tick has run it, and `JsonRpcNotificationService`,
//! which pushes what changed to connected clients.
//!
//! bbb's equivalent lives here because both halves need things the world does
//! not own: the operation id counter, the wall clock a deadline is measured
//! against, and the channel an answer travels back on. The world only ever sees
//! an owned command arriving in its bounded mailbox and an outcome leaving it.

use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc, Mutex,
    },
};

use bbb_server_admin::{
    resources_changed_by, resources_changed_by_player_list, tool_by_name, AdminCall, AdminGateway,
    GatewayError, PendingOperation, ResourceChange,
};
use bbb_server_core::admin::{AdminEnvelope, AdminOutcome};
use bbb_server_core::mailbox::{WorldCommandSendError, WorldCommandSender};
use bbb_server_core::world::WorldCommand;
use tokio::sync::{broadcast, oneshot};

/// How many notifications a subscriber may fall behind before it is told it
/// lost its place. A lagged receiver is answered by re-announcing every
/// resource, so the bound costs precision rather than correctness.
const CHANGE_CHANNEL_CAPACITY: usize = 256;

pub struct RuntimeGateway {
    commands: WorldCommandSender,
    next_operation: AtomicU64,
    /// The last tick the world published, which is what a deadline is measured
    /// from.
    tick: AtomicU64,
    /// How many ticks a submitted request has to be applied within.
    deadline_ticks: u64,
    pending: Mutex<HashMap<u64, oneshot::Sender<AdminOutcome>>>,
    changes: broadcast::Sender<ResourceChange>,
}

impl RuntimeGateway {
    pub fn new(commands: WorldCommandSender, deadline_ticks: u64) -> Arc<Self> {
        Arc::new(Self {
            commands,
            next_operation: AtomicU64::new(1),
            tick: AtomicU64::new(0),
            deadline_ticks: deadline_ticks.max(1),
            pending: Mutex::new(HashMap::new()),
            changes: broadcast::channel(CHANGE_CHANNEL_CAPACITY).0,
        })
    }

    /// Records the tick the world just finished, so the next deadline is
    /// measured from where the world actually is.
    pub fn observe_tick(&self, tick: u64) {
        self.tick.store(tick, Ordering::Release);
    }

    /// Routes one applied outcome back to whoever is waiting for it, and tells
    /// subscribers what it may have changed.
    ///
    /// A caller that already gave up is not an error: the answer is dropped
    /// here, and the transport keeps its own record of the operation.
    pub fn complete(&self, outcome: AdminOutcome) {
        if outcome.result.is_ok() && is_mutation(outcome.action) {
            for uri in resources_changed_by(outcome.action) {
                let _ = self.changes.send(ResourceChange { uri });
            }
        }
        let waiting = self
            .pending
            .lock()
            .expect("management pending map poisoned")
            .remove(&outcome.operation_id);
        if let Some(waiting) = waiting {
            let _ = waiting.send(outcome);
        }
    }

    /// Announces that the player list moved for a reason no management request
    /// asked for — a join, a leave, a kick that came from the world itself.
    pub fn publish_player_list_change(&self) {
        for uri in resources_changed_by_player_list() {
            let _ = self.changes.send(ResourceChange { uri });
        }
    }

    /// Fails every waiting caller at once, which is what a stopping server owes
    /// them instead of a silent timeout.
    pub fn abandon_pending(&self) {
        self.pending
            .lock()
            .expect("management pending map poisoned")
            .clear();
    }

    pub fn pending_operations(&self) -> usize {
        self.pending
            .lock()
            .expect("management pending map poisoned")
            .len()
    }
}

impl AdminGateway for RuntimeGateway {
    fn submit(&self, call: AdminCall) -> Result<PendingOperation, GatewayError> {
        let operation_id = self.next_operation.fetch_add(1, Ordering::Relaxed);
        let envelope = AdminEnvelope {
            operation_id,
            request_id: call.request_id,
            principal: call.principal,
            scope: call.scope,
            deadline_tick: self
                .tick
                .load(Ordering::Acquire)
                .saturating_add(self.deadline_ticks),
            now: crate::runtime::epoch_millis(),
        };
        let (sender, receiver) = oneshot::channel();
        // The waiter is recorded before the command is queued: a tick that ran
        // between the two would otherwise find nobody to answer.
        self.pending
            .lock()
            .expect("management pending map poisoned")
            .insert(operation_id, sender);
        let queued = self.commands.try_send(WorldCommand::Admin {
            envelope,
            request: call.request,
        });
        if let Err(error) = queued {
            self.pending
                .lock()
                .expect("management pending map poisoned")
                .remove(&operation_id);
            return Err(match error {
                WorldCommandSendError::Full(_) => GatewayError::Busy,
                WorldCommandSendError::Closed(_) => GatewayError::Closed,
                WorldCommandSendError::Oversized { actual, max, .. } => {
                    GatewayError::Oversized { actual, max }
                }
            });
        }
        Ok(PendingOperation {
            operation_id,
            answer: receiver,
        })
    }

    fn now(&self) -> i64 {
        crate::runtime::epoch_millis()
    }

    fn subscribe(&self) -> broadcast::Receiver<ResourceChange> {
        self.changes.subscribe()
    }
}

/// Whether an applied action could have changed anything.
///
/// Read from the tool catalog rather than declared again here: the catalog
/// already derives it from the request the action belongs to.
fn is_mutation(action: &str) -> bool {
    tool_by_name(action).is_some_and(|tool| !tool.read_only())
}

#[cfg(test)]
mod tests {
    use bbb_server_core::admin::{AdminRequest, AdminResult, AdminScope};
    use bbb_server_core::mailbox::{
        world_command_mailbox, MailboxClassLimits, WorldMailboxLimits, WorldProducerClass,
    };

    use super::*;

    fn call(request: AdminRequest) -> AdminCall {
        AdminCall {
            principal: "loopback".to_string(),
            scope: AdminScope::Manage,
            request_id: "r-1".to_string(),
            request,
        }
    }

    fn outcome(operation_id: u64, action: &'static str, ok: bool) -> AdminOutcome {
        AdminOutcome {
            operation_id,
            request_id: "r-1".to_string(),
            principal: "loopback".to_string(),
            scope: AdminScope::Manage,
            action,
            tick: 7,
            replayed: false,
            result: if ok {
                Ok(AdminResult::Sent(true))
            } else {
                Err(bbb_server_core::admin::AdminError::invalid_params("no"))
            },
        }
    }

    #[tokio::test]
    async fn a_submitted_call_becomes_an_admin_command_and_its_answer_comes_back_by_id() {
        let (sender, receiver) = world_command_mailbox(WorldMailboxLimits::default()).unwrap();
        let gateway = RuntimeGateway::new(sender, 40);
        gateway.observe_tick(100);
        let pending = gateway.submit(call(AdminRequest::AllowlistClear)).unwrap();
        assert_eq!(pending.operation_id, 1);
        assert_eq!(gateway.pending_operations(), 1);

        let queued = receiver.drain();
        assert_eq!(queued.len(), 1);
        assert_eq!(queued[0].producer_class(), WorldProducerClass::Admin);
        let WorldCommand::Admin { envelope, request } = queued[0].command() else {
            panic!("a management call is an admin command");
        };
        assert_eq!(envelope.operation_id, 1);
        assert_eq!(envelope.request_id, "r-1");
        assert_eq!(envelope.deadline_tick, 140);
        assert!(envelope.now > 1_700_000_000_000);
        assert_eq!(*request, AdminRequest::AllowlistClear);

        gateway.complete(outcome(1, "allowlist.clear", true));
        assert_eq!(gateway.pending_operations(), 0);
        assert_eq!(pending.answer.await.unwrap().operation_id, 1);
    }

    #[tokio::test]
    async fn a_full_management_quota_is_backpressure_and_not_a_lost_request() {
        let limits = WorldMailboxLimits {
            admin: MailboxClassLimits {
                entries: 1,
                bytes: 64 * 1024,
            },
            ..WorldMailboxLimits::default()
        };
        let (sender, _receiver) = world_command_mailbox(limits).unwrap();
        let gateway = RuntimeGateway::new(sender, 40);
        assert!(gateway.submit(call(AdminRequest::BansClear)).is_ok());
        assert_eq!(
            gateway.submit(call(AdminRequest::BansClear)).err(),
            Some(GatewayError::Busy)
        );
        // The refused call left no waiter behind to be answered later.
        assert_eq!(gateway.pending_operations(), 1);
    }

    #[tokio::test]
    async fn only_an_applied_mutation_tells_subscribers_a_resource_moved() {
        let (sender, _receiver) = world_command_mailbox(WorldMailboxLimits::default()).unwrap();
        let gateway = RuntimeGateway::new(sender, 40);
        let mut changes = gateway.subscribe();

        gateway.complete(outcome(1, "players", true));
        gateway.complete(outcome(2, "allowlist.clear", false));
        gateway.complete(outcome(3, "allowlist.clear", true));
        let mut announced = Vec::new();
        while let Ok(change) = changes.try_recv() {
            announced.push(change.uri);
        }
        assert_eq!(
            announced,
            vec!["bbb://server/allowlist", "bbb://server/diagnostics"]
        );

        gateway.publish_player_list_change();
        let mut announced = Vec::new();
        while let Ok(change) = changes.try_recv() {
            announced.push(change.uri);
        }
        assert_eq!(
            announced,
            vec![
                "bbb://server/players",
                "bbb://server/status",
                "bbb://server/diagnostics",
            ]
        );
    }

    #[tokio::test]
    async fn an_answer_nobody_waits_for_is_dropped_rather_than_kept() {
        let (sender, _receiver) = world_command_mailbox(WorldMailboxLimits::default()).unwrap();
        let gateway = RuntimeGateway::new(sender, 40);
        let pending = gateway.submit(call(AdminRequest::Status)).unwrap();
        drop(pending.answer);
        gateway.complete(outcome(1, "server.status", true));
        assert_eq!(gateway.pending_operations(), 0);

        let pending = gateway.submit(call(AdminRequest::Status)).unwrap();
        gateway.abandon_pending();
        assert!(pending.answer.await.is_err());
    }
}
