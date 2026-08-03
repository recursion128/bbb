//! Generation-safe asynchronous Minecraft Services block-list lifecycle.

use std::{fmt, future::Future, pin::Pin, sync::Arc};

use bbb_net::{
    AbuseReportRequest, MinecraftUserApiClient, MinecraftUserApiErrorKind, NetEvent,
    UserApiBlockList,
};
use bbb_protocol::packets::PlayClientbound;
use bbb_world::{BuiltPlayerReport, WorldStore};
use tokio::{runtime::Handle, sync::Mutex, task::AbortHandle};
use uuid::Uuid;

use crate::simulation::commands::{SimulationCommand, SimulationMailbox};

type BlockListRefreshFuture<'a> =
    Pin<Box<dyn Future<Output = ServicesBlockListRefresh> + Send + 'a>>;
type PlayerReportSubmissionFuture<'a> =
    Pin<Box<dyn Future<Output = Result<(), PlayerReportSubmissionFailure>> + Send + 'a>>;

trait ServicesBlockListSource: Send + Sync {
    fn refresh(&self, now_epoch_seconds: i64) -> BlockListRefreshFuture<'_>;

    fn submit_player_report(&self, report: BuiltPlayerReport) -> PlayerReportSubmissionFuture<'_> {
        let _ = report;
        Box::pin(async {
            Err(PlayerReportSubmissionFailure {
                kind: MinecraftUserApiErrorKind::ServiceUnavailable,
                status: None,
            })
        })
    }
}

struct MinecraftServicesBlockListSource {
    client: Mutex<MinecraftUserApiClient>,
}

impl ServicesBlockListSource for MinecraftServicesBlockListSource {
    fn refresh(&self, now_epoch_seconds: i64) -> BlockListRefreshFuture<'_> {
        Box::pin(async move {
            let mut client = self.client.lock().await;
            let result = client.refresh_block_list(now_epoch_seconds).await;
            let next_refresh_epoch_seconds = client.next_block_list_refresh_epoch_seconds();
            let result = match result {
                Ok(_) => Ok(client
                    .cached_block_list()
                    .map(UserApiBlockList::blocked_profiles)
                    .unwrap_or_default()
                    .to_vec()),
                Err(error) => Err(ServicesBlockListFailure {
                    kind: error.kind(),
                    status: error.status(),
                }),
            };
            ServicesBlockListRefresh {
                result,
                next_refresh_epoch_seconds,
            }
        })
    }

    fn submit_player_report(&self, report: BuiltPlayerReport) -> PlayerReportSubmissionFuture<'_> {
        Box::pin(async move {
            let request = AbuseReportRequest::try_from(report).map_err(|error| {
                PlayerReportSubmissionFailure {
                    kind: error.kind(),
                    status: error.status(),
                }
            })?;
            let client = self.client.lock().await;
            client.submit_abuse_report(&request).await.map_err(|error| {
                PlayerReportSubmissionFailure {
                    kind: error.kind(),
                    status: error.status(),
                }
            })
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ServicesBlockListFailure {
    pub(crate) kind: MinecraftUserApiErrorKind,
    pub(crate) status: Option<u16>,
}

#[derive(Debug)]
pub(crate) struct ServicesBlockListCompletion {
    pub(crate) generation: u64,
    pub(crate) result: Result<Vec<Uuid>, ServicesBlockListFailure>,
    pub(crate) next_refresh_epoch_seconds: Option<i64>,
    pub(crate) completed_epoch_seconds: i64,
}

struct ServicesBlockListRefresh {
    result: Result<Vec<Uuid>, ServicesBlockListFailure>,
    next_refresh_epoch_seconds: Option<i64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct PlayerReportSubmissionFailure {
    pub(crate) kind: MinecraftUserApiErrorKind,
    pub(crate) status: Option<u16>,
}

#[derive(Debug)]
pub(crate) struct PlayerReportSubmissionCompletion {
    pub(crate) service_generation: u64,
    pub(crate) report_id: Uuid,
    pub(crate) attempt: u64,
    pub(crate) result: Result<(), PlayerReportSubmissionFailure>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct PendingPlayerReportSubmission {
    service_generation: u64,
    report_id: Uuid,
    attempt: u64,
}

pub(crate) struct ServicesBlockListRuntime {
    handle: Handle,
    mailbox: SimulationMailbox,
    source: Option<Arc<dyn ServicesBlockListSource>>,
    generation: u64,
    online: bool,
    pending_generation: Option<u64>,
    next_refresh_epoch_seconds: Option<i64>,
    pending_player_report: Option<PendingPlayerReportSubmission>,
    pending_player_report_abort: Option<AbortHandle>,
}

impl fmt::Debug for ServicesBlockListRuntime {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ServicesBlockListRuntime")
            .field("authenticated", &self.source.is_some())
            .field("generation", &self.generation)
            .field("online", &self.online)
            .field("pending_generation", &self.pending_generation)
            .field(
                "next_refresh_epoch_seconds",
                &self.next_refresh_epoch_seconds,
            )
            .field("pending_player_report", &self.pending_player_report)
            .finish_non_exhaustive()
    }
}

impl ServicesBlockListRuntime {
    pub(crate) fn new(
        handle: Handle,
        mailbox: SimulationMailbox,
        client: Option<MinecraftUserApiClient>,
    ) -> Self {
        let source = client.map(|client| {
            Arc::new(MinecraftServicesBlockListSource {
                client: Mutex::new(client),
            }) as Arc<dyn ServicesBlockListSource>
        });
        Self {
            handle,
            mailbox,
            source,
            generation: 0,
            online: false,
            pending_generation: None,
            next_refresh_epoch_seconds: None,
            pending_player_report: None,
            pending_player_report_abort: None,
        }
    }

    pub(crate) fn observe_net_event(
        &mut self,
        event: &NetEvent,
        world: &mut WorldStore,
        now_epoch_seconds: i64,
    ) {
        match event {
            NetEvent::StateChanged {
                state: bbb_net::ConnectionState::Play,
            } => self.begin_online_mode(world, now_epoch_seconds),
            NetEvent::Connected
            | NetEvent::StartConfiguration
            | NetEvent::Transfer(_)
            | NetEvent::Play(PlayClientbound::Transfer(_))
            | NetEvent::Disconnected { .. } => self.stop_online_mode(world),
            _ => {}
        }
    }

    pub(crate) fn tick(&mut self, now_epoch_seconds: i64) {
        if self.online
            && self
                .next_refresh_epoch_seconds
                .is_some_and(|deadline| now_epoch_seconds > deadline)
        {
            self.start_refresh(now_epoch_seconds, false);
        }
    }

    pub(crate) fn synchronize_player_report_submissions(
        &mut self,
        input: &mut crate::input::ClientInputState,
        world: &mut WorldStore,
    ) {
        for (report_id, attempt) in input.take_cancelled_player_reporting_submissions() {
            if self
                .pending_player_report
                .is_some_and(|pending| pending.report_id == report_id && pending.attempt == attempt)
            {
                self.pending_player_report = None;
                if let Some(abort) = self.pending_player_report_abort.take() {
                    abort.abort();
                }
            }
        }
        if self
            .pending_player_report
            .is_some_and(|pending| !self.online || pending.service_generation != self.generation)
        {
            let pending = self
                .pending_player_report
                .take()
                .expect("checked pending submission");
            if let Some(abort) = self.pending_player_report_abort.take() {
                abort.abort();
            }
            input.apply_player_reporting_submission_result(
                world,
                pending.service_generation,
                pending.report_id,
                pending.attempt,
                false,
            );
        }

        for request in input.take_player_reporting_submissions() {
            let report_id = request.report.id;
            let pending = PendingPlayerReportSubmission {
                service_generation: self.generation,
                report_id,
                attempt: request.attempt,
            };
            if self.pending_player_report.is_some()
                || !world.begin_player_report_submission(
                    pending.service_generation,
                    report_id,
                    request.attempt,
                )
            {
                continue;
            }
            let Some(source) = self.source.as_ref().map(Arc::clone).filter(|_| self.online) else {
                input.apply_player_reporting_submission_result(
                    world,
                    pending.service_generation,
                    report_id,
                    request.attempt,
                    false,
                );
                continue;
            };
            self.pending_player_report = Some(pending);
            let mailbox = self.mailbox.clone();
            let task = self.handle.spawn(async move {
                let result = source.submit_player_report(request.report).await;
                mailbox.enqueue_or_report(
                    SimulationCommand::PlayerReportSubmissionCompletion(
                        PlayerReportSubmissionCompletion {
                            service_generation: pending.service_generation,
                            report_id,
                            attempt: request.attempt,
                            result,
                        },
                    ),
                    "player report submission completion",
                );
            });
            self.pending_player_report_abort = Some(task.abort_handle());
        }
    }

    pub(crate) fn apply_player_report_submission_completion(
        &mut self,
        completion: PlayerReportSubmissionCompletion,
        input: &mut crate::input::ClientInputState,
        world: &mut WorldStore,
    ) -> bool {
        let expected = PendingPlayerReportSubmission {
            service_generation: completion.service_generation,
            report_id: completion.report_id,
            attempt: completion.attempt,
        };
        if self.pending_player_report != Some(expected)
            || !self.online
            || completion.service_generation != self.generation
        {
            return false;
        }
        self.pending_player_report = None;
        self.pending_player_report_abort = None;
        if let Err(error) = completion.result {
            tracing::warn!(
                kind = ?error.kind,
                status = ?error.status,
                report_id = %completion.report_id,
                attempt = completion.attempt,
                "Minecraft Services player-report submission failed"
            );
            return input.apply_player_reporting_submission_result(
                world,
                completion.service_generation,
                completion.report_id,
                completion.attempt,
                false,
            );
        }
        input.apply_player_reporting_submission_result(
            world,
            completion.service_generation,
            completion.report_id,
            completion.attempt,
            true,
        )
    }

    pub(crate) fn apply_completion(
        &mut self,
        completion: ServicesBlockListCompletion,
        world: &mut WorldStore,
    ) -> bool {
        if self.pending_generation != Some(completion.generation) {
            return false;
        }
        self.pending_generation = None;
        self.next_refresh_epoch_seconds = completion.next_refresh_epoch_seconds;

        if !self.online || completion.generation != self.generation {
            if self.online {
                self.start_refresh(completion.completed_epoch_seconds, true);
            }
            return false;
        }

        let blocked_players = match completion.result {
            Ok(blocked_players) => blocked_players,
            Err(error) => {
                tracing::warn!(
                    kind = ?error.kind,
                    status = ?error.status,
                    "Minecraft Services block-list refresh failed"
                );
                Vec::new()
            }
        };
        world.replace_services_blocked_players(self.generation, blocked_players)
    }

    fn begin_online_mode(&mut self, world: &mut WorldStore, now_epoch_seconds: i64) {
        self.advance_generation();
        if self.source.is_none() {
            self.online = false;
            self.next_refresh_epoch_seconds = None;
            world.stop_services_block_list_session(self.generation);
            return;
        }
        self.online = true;
        self.next_refresh_epoch_seconds = None;
        world.begin_services_block_list_session(self.generation);
        self.start_refresh(now_epoch_seconds, true);
    }

    fn stop_online_mode(&mut self, world: &mut WorldStore) {
        self.advance_generation();
        self.online = false;
        self.next_refresh_epoch_seconds = None;
        world.stop_services_block_list_session(self.generation);
    }

    fn advance_generation(&mut self) {
        self.generation = self
            .generation
            .checked_add(1)
            .expect("Services block-list generation exhausted");
    }

    fn start_refresh(&mut self, now_epoch_seconds: i64, force: bool) {
        if self.pending_generation.is_some() || !self.online {
            return;
        }
        if !force
            && self
                .next_refresh_epoch_seconds
                .is_none_or(|deadline| now_epoch_seconds <= deadline)
        {
            return;
        }
        let Some(source) = self.source.as_ref().map(Arc::clone) else {
            return;
        };
        let generation = self.generation;
        self.pending_generation = Some(generation);
        let mailbox = self.mailbox.clone();
        self.handle.spawn(async move {
            let refresh = source.refresh(now_epoch_seconds).await;
            mailbox.enqueue_or_report(
                SimulationCommand::ServicesBlockListCompletion(ServicesBlockListCompletion {
                    generation,
                    result: refresh.result,
                    next_refresh_epoch_seconds: refresh.next_refresh_epoch_seconds,
                    completed_epoch_seconds: chrono::Utc::now().timestamp(),
                }),
                "services block-list completion",
            );
        });
    }
}

#[cfg(test)]
mod tests {
    use std::{
        collections::VecDeque,
        sync::atomic::{AtomicUsize, Ordering},
    };

    use bbb_net::ConnectionState;
    use bbb_protocol::packets::{
        GameProfile, GameType, PlayerInfoAction, PlayerInfoEntry, PlayerInfoUpdate,
    };
    use bbb_renderer::{HudPlayerReportingFocus, HudPlayerReportingHitTarget};
    use bbb_world::{PlayerReportEnvironment, PlayerReportSubmissionStatus};
    use winit::event::{ElementState, MouseButton};

    use super::*;

    struct FakeSource {
        calls: AtomicUsize,
        refreshes: Mutex<VecDeque<ServicesBlockListRefresh>>,
    }

    impl FakeSource {
        fn new(refreshes: impl IntoIterator<Item = ServicesBlockListRefresh>) -> Self {
            Self {
                calls: AtomicUsize::new(0),
                refreshes: Mutex::new(refreshes.into_iter().collect()),
            }
        }
    }

    impl ServicesBlockListSource for FakeSource {
        fn refresh(&self, _now_epoch_seconds: i64) -> BlockListRefreshFuture<'_> {
            Box::pin(async move {
                self.calls.fetch_add(1, Ordering::SeqCst);
                self.refreshes
                    .lock()
                    .await
                    .pop_front()
                    .expect("fake refresh result")
            })
        }
    }

    struct FakeSubmissionSource {
        submissions: Mutex<VecDeque<Result<(), PlayerReportSubmissionFailure>>>,
        calls: AtomicUsize,
    }

    impl ServicesBlockListSource for FakeSubmissionSource {
        fn refresh(&self, _now_epoch_seconds: i64) -> BlockListRefreshFuture<'_> {
            Box::pin(async {
                ServicesBlockListRefresh {
                    result: Ok(Vec::new()),
                    next_refresh_epoch_seconds: None,
                }
            })
        }

        fn submit_player_report(
            &self,
            _report: BuiltPlayerReport,
        ) -> PlayerReportSubmissionFuture<'_> {
            Box::pin(async move {
                self.calls.fetch_add(1, Ordering::SeqCst);
                self.submissions
                    .lock()
                    .await
                    .pop_front()
                    .expect("fake submission result")
            })
        }
    }

    fn runtime_with_source(
        mailbox: SimulationMailbox,
        source: Arc<dyn ServicesBlockListSource>,
    ) -> ServicesBlockListRuntime {
        ServicesBlockListRuntime {
            handle: Handle::current(),
            mailbox,
            source: Some(source),
            generation: 0,
            online: false,
            pending_generation: None,
            next_refresh_epoch_seconds: None,
            pending_player_report: None,
            pending_player_report_abort: None,
        }
    }

    async fn take_completion(mailbox: &SimulationMailbox) -> ServicesBlockListCompletion {
        for _ in 0..100 {
            if let Some(command) = mailbox.drain_through_barrier().into_iter().next() {
                let SimulationCommand::ServicesBlockListCompletion(completion) = command.command
                else {
                    panic!("expected Services block-list completion");
                };
                return completion;
            }
            tokio::task::yield_now().await;
        }
        panic!("Services block-list completion was not queued");
    }

    async fn take_report_completion(
        mailbox: &SimulationMailbox,
    ) -> PlayerReportSubmissionCompletion {
        for _ in 0..100 {
            for command in mailbox.drain_through_barrier() {
                if let SimulationCommand::PlayerReportSubmissionCompletion(completion) =
                    command.command
                {
                    return completion;
                }
            }
            tokio::task::yield_now().await;
        }
        panic!("player-report submission completion was not queued");
    }

    fn success(
        blocked_players: Vec<Uuid>,
        next_refresh_epoch_seconds: i64,
    ) -> ServicesBlockListRefresh {
        ServicesBlockListRefresh {
            result: Ok(blocked_players),
            next_refresh_epoch_seconds: Some(next_refresh_epoch_seconds),
        }
    }

    #[tokio::test]
    async fn services_block_list_success_and_strict_cooldown_apply_through_mailbox() {
        let blocked = Uuid::from_u128(10);
        let source = Arc::new(FakeSource::new([
            success(vec![blocked], 220),
            success(vec![], 341),
        ]));
        let mailbox = SimulationMailbox::default();
        let mut runtime = runtime_with_source(mailbox.clone(), source.clone());
        let mut world = WorldStore::new();

        runtime.observe_net_event(
            &NetEvent::StateChanged {
                state: ConnectionState::Play,
            },
            &mut world,
            100,
        );
        let completion = take_completion(&mailbox).await;
        assert!(runtime.apply_completion(completion, &mut world));
        assert!(world.social_player_is_blocked(blocked));
        assert_eq!(source.calls.load(Ordering::SeqCst), 1);

        runtime.tick(220);
        tokio::task::yield_now().await;
        assert_eq!(source.calls.load(Ordering::SeqCst), 1);
        runtime.tick(221);
        let completion = take_completion(&mailbox).await;
        assert!(runtime.apply_completion(completion, &mut world));
        assert!(!world.social_player_is_blocked(blocked));
        assert_eq!(source.calls.load(Ordering::SeqCst), 2);
    }

    #[tokio::test]
    async fn services_block_list_failure_replaces_current_services_set() {
        let blocked = Uuid::from_u128(11);
        let configured = Uuid::from_u128(16);
        let failure = ServicesBlockListFailure {
            kind: MinecraftUserApiErrorKind::Http,
            status: Some(503),
        };
        let source = Arc::new(FakeSource::new([
            success(vec![blocked], 220),
            ServicesBlockListRefresh {
                result: Err(failure),
                next_refresh_epoch_seconds: Some(341),
            },
        ]));
        let mailbox = SimulationMailbox::default();
        let mut runtime = runtime_with_source(mailbox.clone(), source);
        let mut world = WorldStore::new();
        let mut presentation = world.chat_presentation_config().clone();
        presentation.blocked_players.push(configured);
        world.set_chat_presentation_config(presentation);
        runtime.observe_net_event(
            &NetEvent::StateChanged {
                state: ConnectionState::Play,
            },
            &mut world,
            100,
        );
        assert!(runtime.apply_completion(take_completion(&mailbox).await, &mut world));
        assert!(world.social_player_is_blocked(blocked));

        runtime.tick(221);
        assert!(runtime.apply_completion(take_completion(&mailbox).await, &mut world));
        assert!(!world.social_player_is_blocked(blocked));
        assert!(world.social_player_is_blocked(configured));
        assert!(world.services_block_list().active);
    }

    #[tokio::test]
    async fn services_block_list_transfer_reconnect_rejects_stale_completion() {
        let old_blocked = Uuid::from_u128(12);
        let new_blocked = Uuid::from_u128(13);
        let source = Arc::new(FakeSource::new([
            success(vec![old_blocked], 220),
            success(vec![new_blocked], 220),
        ]));
        let mailbox = SimulationMailbox::default();
        let mut runtime = runtime_with_source(mailbox.clone(), source);
        let mut world = WorldStore::new();
        runtime.observe_net_event(
            &NetEvent::StateChanged {
                state: ConnectionState::Play,
            },
            &mut world,
            100,
        );
        let stale = take_completion(&mailbox).await;
        runtime.observe_net_event(
            &NetEvent::Play(PlayClientbound::Transfer(bbb_protocol::packets::Transfer {
                host: "next.example".to_string(),
                port: 25565,
            })),
            &mut world,
            101,
        );
        runtime.observe_net_event(
            &NetEvent::StateChanged {
                state: ConnectionState::Play,
            },
            &mut world,
            102,
        );

        assert!(!runtime.apply_completion(stale, &mut world));
        assert!(!world.social_player_is_blocked(old_blocked));
        let current = take_completion(&mailbox).await;
        assert!(runtime.apply_completion(current, &mut world));
        assert!(world.social_player_is_blocked(new_blocked));
    }

    #[tokio::test]
    async fn services_block_list_play_reconfiguration_invalidates_pending_generation() {
        let stale_blocked = Uuid::from_u128(14);
        let current_blocked = Uuid::from_u128(15);
        let source = Arc::new(FakeSource::new([
            success(vec![stale_blocked], 220),
            success(vec![current_blocked], 220),
        ]));
        let mailbox = SimulationMailbox::default();
        let mut runtime = runtime_with_source(mailbox.clone(), source);
        let mut world = WorldStore::new();
        runtime.observe_net_event(
            &NetEvent::StateChanged {
                state: ConnectionState::Play,
            },
            &mut world,
            100,
        );
        let stale = take_completion(&mailbox).await;

        runtime.observe_net_event(&NetEvent::StartConfiguration, &mut world, 101);
        assert!(!world.services_block_list().active);
        runtime.observe_net_event(
            &NetEvent::StateChanged {
                state: ConnectionState::Play,
            },
            &mut world,
            102,
        );
        assert!(world.services_block_list().active);

        assert!(!runtime.apply_completion(stale, &mut world));
        assert!(!world.social_player_is_blocked(stale_blocked));
        assert!(runtime.apply_completion(take_completion(&mailbox).await, &mut world));
        assert!(world.social_player_is_blocked(current_blocked));
    }

    #[tokio::test]
    async fn services_block_list_offline_play_and_disconnect_stay_inactive() {
        let mailbox = SimulationMailbox::default();
        let mut runtime = ServicesBlockListRuntime::new(Handle::current(), mailbox.clone(), None);
        let mut world = WorldStore::new();

        runtime.observe_net_event(
            &NetEvent::StateChanged {
                state: ConnectionState::Play,
            },
            &mut world,
            100,
        );
        assert!(!world.services_block_list().active);
        assert_eq!(mailbox.pending_len(), 0);

        runtime.observe_net_event(&NetEvent::Disconnected { reason: None }, &mut world, 101);
        assert!(!world.services_block_list().active);
        assert_eq!(mailbox.pending_len(), 0);
    }

    #[tokio::test]
    async fn player_reporting_submission_runs_async_and_applies_only_matching_attempt() {
        let failure = PlayerReportSubmissionFailure {
            kind: MinecraftUserApiErrorKind::Http,
            status: Some(503),
        };
        let source = Arc::new(FakeSubmissionSource {
            submissions: Mutex::new([Err(failure), Ok(())].into_iter().collect()),
            calls: AtomicUsize::new(0),
        });
        let mailbox = SimulationMailbox::default();
        let mut runtime = runtime_with_source(mailbox.clone(), source.clone());
        runtime.online = true;
        runtime.generation = 7;
        let (mut world, remote) = reporting_world();
        let mut input = crate::input::ClientInputState::new(true);
        assert!(input.open_social_interactions_screen(&mut world, false));
        assert!(input.open_player_reporting_screen(&mut world, remote));
        click_reporting(
            &mut input,
            &mut world,
            HudPlayerReportingFocus::ReportType(2),
        );
        click_reporting(&mut input, &mut world, HudPlayerReportingFocus::Attestation);
        click_reporting(&mut input, &mut world, HudPlayerReportingFocus::Send);

        runtime.synchronize_player_report_submissions(&mut input, &mut world);
        let pending = runtime.pending_player_report.expect("pending report");
        assert_eq!(
            world.player_reporting().submission,
            Some(bbb_world::PlayerReportSubmissionState {
                service_generation: 7,
                report_id: pending.report_id,
                attempt: pending.attempt,
                status: PlayerReportSubmissionStatus::Sending,
            })
        );
        let completion = take_report_completion(&mailbox).await;
        let stale = PlayerReportSubmissionCompletion {
            service_generation: completion.service_generation,
            report_id: completion.report_id,
            attempt: completion.attempt + 1,
            result: completion.result,
        };
        assert!(!runtime.apply_player_report_submission_completion(stale, &mut input, &mut world,));
        assert!(runtime.pending_player_report.is_some());

        let matching = PlayerReportSubmissionCompletion {
            service_generation: pending.service_generation,
            report_id: pending.report_id,
            attempt: pending.attempt,
            result: Err(failure),
        };
        assert!(
            runtime.apply_player_report_submission_completion(matching, &mut input, &mut world,)
        );
        assert_eq!(
            world.player_reporting().submission.unwrap().status,
            PlayerReportSubmissionStatus::Failed
        );
        assert!(world.player_reporting().draft.is_some());

        click_reporting(&mut input, &mut world, HudPlayerReportingFocus::Retry);
        runtime.synchronize_player_report_submissions(&mut input, &mut world);
        let completion = take_report_completion(&mailbox).await;
        assert!(
            runtime.apply_player_report_submission_completion(completion, &mut input, &mut world,)
        );
        assert_eq!(source.calls.load(Ordering::SeqCst), 2);
        assert_eq!(
            world.player_reporting().submission.unwrap().status,
            PlayerReportSubmissionStatus::Succeeded
        );
        assert!(world.player_reporting().draft.is_none());
    }

    #[tokio::test]
    async fn player_reporting_submission_cancel_aborts_task_and_rejects_late_completion() {
        let source = Arc::new(FakeSubmissionSource {
            submissions: Mutex::new([Ok(())].into_iter().collect()),
            calls: AtomicUsize::new(0),
        });
        let mailbox = SimulationMailbox::default();
        let mut runtime = runtime_with_source(mailbox, source);
        runtime.online = true;
        runtime.generation = 8;
        let (mut world, remote) = reporting_world();
        let mut input = crate::input::ClientInputState::new(true);
        assert!(input.open_social_interactions_screen(&mut world, false));
        assert!(input.open_player_reporting_screen(&mut world, remote));
        click_reporting(
            &mut input,
            &mut world,
            HudPlayerReportingFocus::ReportType(2),
        );
        click_reporting(&mut input, &mut world, HudPlayerReportingFocus::Attestation);
        click_reporting(&mut input, &mut world, HudPlayerReportingFocus::Send);
        runtime.synchronize_player_report_submissions(&mut input, &mut world);
        let pending = runtime.pending_player_report.expect("pending report");
        assert!(runtime.pending_player_report_abort.is_some());

        click_reporting(&mut input, &mut world, HudPlayerReportingFocus::Cancel);
        runtime.synchronize_player_report_submissions(&mut input, &mut world);
        assert!(runtime.pending_player_report.is_none());
        assert!(runtime.pending_player_report_abort.is_none());
        assert!(world.player_reporting().submission.is_none());
        assert!(!runtime.apply_player_report_submission_completion(
            PlayerReportSubmissionCompletion {
                service_generation: pending.service_generation,
                report_id: pending.report_id,
                attempt: pending.attempt,
                result: Ok(()),
            },
            &mut input,
            &mut world,
        ));
        assert!(world.player_reporting().draft.is_some());
    }

    fn reporting_world() -> (WorldStore, Uuid) {
        let local = Uuid::from_u128(101);
        let remote = Uuid::from_u128(102);
        let mut world = WorldStore::new();
        world.set_local_player_profile_id(local);
        world.set_player_report_environment(Some(PlayerReportEnvironment::third_party(
            "26.1",
            "en-US",
            "example.org:25565",
        )));
        world.apply_player_info_update(PlayerInfoUpdate {
            actions: vec![PlayerInfoAction::AddPlayer],
            entries: vec![
                player_info_entry(local, "Local"),
                player_info_entry(remote, "Remote"),
            ],
        });
        (world, remote)
    }

    fn player_info_entry(profile_id: Uuid, name: &str) -> PlayerInfoEntry {
        PlayerInfoEntry {
            profile_id,
            profile: Some(GameProfile {
                uuid: profile_id,
                name: name.to_string(),
                properties: Vec::new(),
            }),
            listed: true,
            latency: 0,
            game_mode: GameType::default(),
            display_name: None,
            show_hat: true,
            list_order: 0,
            chat_session: None,
        }
    }

    fn click_reporting(
        input: &mut crate::input::ClientInputState,
        world: &mut WorldStore,
        focus: HudPlayerReportingFocus,
    ) {
        let generation = input
            .player_reporting_runtime()
            .expect("reporting runtime")
            .generation();
        assert!(input.handle_player_reporting_mouse_input(
            world,
            MouseButton::Left,
            ElementState::Pressed,
            Some(HudPlayerReportingHitTarget { generation, focus }),
        ));
    }
}
