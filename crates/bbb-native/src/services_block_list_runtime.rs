//! Generation-safe asynchronous Minecraft Services block-list lifecycle.

use std::{fmt, future::Future, pin::Pin, sync::Arc};

use bbb_net::{MinecraftUserApiClient, MinecraftUserApiErrorKind, NetEvent, UserApiBlockList};
use bbb_protocol::packets::PlayClientbound;
use bbb_world::WorldStore;
use tokio::{runtime::Handle, sync::Mutex};
use uuid::Uuid;

use crate::simulation::commands::{SimulationCommand, SimulationMailbox};

type BlockListRefreshFuture<'a> =
    Pin<Box<dyn Future<Output = ServicesBlockListRefresh> + Send + 'a>>;

trait ServicesBlockListSource: Send + Sync {
    fn refresh(&self, now_epoch_seconds: i64) -> BlockListRefreshFuture<'_>;
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

pub(crate) struct ServicesBlockListRuntime {
    handle: Handle,
    mailbox: SimulationMailbox,
    source: Option<Arc<dyn ServicesBlockListSource>>,
    generation: u64,
    online: bool,
    pending_generation: Option<u64>,
    next_refresh_epoch_seconds: Option<i64>,
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
            mailbox.enqueue(SimulationCommand::ServicesBlockListCompletion(
                ServicesBlockListCompletion {
                    generation,
                    result: refresh.result,
                    next_refresh_epoch_seconds: refresh.next_refresh_epoch_seconds,
                    completed_epoch_seconds: chrono::Utc::now().timestamp(),
                },
            ));
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
}
