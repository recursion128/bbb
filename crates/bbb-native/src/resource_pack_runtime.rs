use std::{
    collections::{HashSet, VecDeque},
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicU64, Ordering},
        mpsc as std_mpsc,
    },
};

use bbb_net::{download_server_pack, NetCommand, ServerPackDownloadRequest};
use bbb_pack::{materialize_server_pack_archive, ServerPackArchiveLimits};
use bbb_protocol::packets::ResourcePackResponseAction;
use bbb_world::{ResourcePackState, WorldStore};
use tokio::{runtime::Handle, sync::mpsc};
use uuid::Uuid;

static SESSION_SERIAL: AtomicU64 = AtomicU64::new(0);

pub(crate) fn default_server_resource_pack_cache_dir() -> PathBuf {
    std::env::temp_dir().join("bbb-native-server-packs")
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct PackKey {
    id: Uuid,
    push_sequence: u64,
}

impl PackKey {
    fn from_world(pack: &ResourcePackState) -> Self {
        Self {
            id: pack.id,
            push_sequence: pack.push_sequence,
        }
    }
}

#[derive(Debug)]
enum PackDownloadState {
    Pending,
    Ready(PathBuf),
    Failed,
}

#[derive(Debug)]
struct TrackedPack {
    key: PackKey,
    state: PackDownloadState,
    removed: bool,
}

#[derive(Debug)]
enum PackTaskEvent {
    Downloaded {
        key: PackKey,
    },
    Ready {
        key: PackKey,
        root: PathBuf,
    },
    Failed {
        key: PackKey,
        action: ResourcePackResponseAction,
        error: String,
    },
}

impl PackTaskEvent {
    fn key(&self) -> PackKey {
        match self {
            Self::Downloaded { key } | Self::Ready { key, .. } | Self::Failed { key, .. } => *key,
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct DeferredResponse {
    key: PackKey,
    action: ResourcePackResponseAction,
}

/// Owns server-pack downloads and archive materialization for one connection session.
///
/// Canonical push/pop and response state stays in `WorldStore`. This runtime keys asynchronous
/// work by both UUID and monotonic push sequence so a late result from a popped or replaced pack
/// cannot mutate the newer request. Actual resource reload is a separate barrier: a pack becomes
/// ready here only after download/hash/archive/metadata validation has succeeded.
pub(crate) struct ServerResourcePackRuntime {
    runtime: Option<Handle>,
    download_cache_dir: PathBuf,
    materialized_session_dir: PathBuf,
    username: String,
    profile_id: Uuid,
    task_tx: std_mpsc::Sender<PackTaskEvent>,
    task_rx: std_mpsc::Receiver<PackTaskEvent>,
    packs: Vec<TrackedPack>,
    responses: VecDeque<DeferredResponse>,
    last_reported_ready_roots: Vec<PathBuf>,
}

impl ServerResourcePackRuntime {
    pub(crate) fn new(
        runtime: Handle,
        cache_dir: impl Into<PathBuf>,
        username: impl Into<String>,
        profile_id: Uuid,
    ) -> Self {
        Self::with_optional_runtime(Some(runtime), cache_dir.into(), username.into(), profile_id)
    }

    fn with_optional_runtime(
        runtime: Option<Handle>,
        cache_dir: PathBuf,
        username: String,
        profile_id: Uuid,
    ) -> Self {
        let session_serial = SESSION_SERIAL.fetch_add(1, Ordering::Relaxed);
        let materialized_session_dir = cache_dir
            .join("materialized")
            .join(format!("session-{}-{session_serial}", std::process::id()));
        let (task_tx, task_rx) = std_mpsc::channel();
        Self {
            runtime,
            download_cache_dir: cache_dir.join("downloads"),
            materialized_session_dir,
            username,
            profile_id,
            task_tx,
            task_rx,
            packs: Vec::new(),
            responses: VecDeque::new(),
            last_reported_ready_roots: Vec::new(),
        }
    }

    pub(crate) fn synchronize(
        &mut self,
        world: &mut WorldStore,
        commands: &Option<mpsc::Sender<NetCommand>>,
        connected: bool,
    ) {
        self.reconcile_world(world, connected);
        self.drain_task_events();
        self.cleanup_removed_packs();
        self.flush_responses(world, commands);
        self.report_ready_stack_change();
    }

    fn reconcile_world(&mut self, world: &WorldStore, connected: bool) {
        let mut canonical = world.resource_packs().values().collect::<Vec<_>>();
        canonical.sort_by_key(|pack| pack.push_sequence);
        let canonical_keys = connected
            .then(|| {
                canonical
                    .iter()
                    .map(|pack| PackKey::from_world(pack))
                    .collect::<HashSet<_>>()
            })
            .unwrap_or_default();

        for tracked in &mut self.packs {
            if !canonical_keys.contains(&tracked.key) {
                tracked.removed = true;
            }
        }

        if !connected {
            return;
        }
        for pack in canonical {
            let key = PackKey::from_world(pack);
            let accepted = pack
                .last_response
                .is_some_and(|response| response.action == ResourcePackResponseAction::Accepted);
            if !accepted || self.packs.iter().any(|tracked| tracked.key == key) {
                continue;
            }
            self.packs.push(TrackedPack {
                key,
                state: PackDownloadState::Pending,
                removed: false,
            });
            self.start_pack_task(key, pack.url.clone(), pack.hash.clone());
        }
        self.packs.sort_by_key(|pack| pack.key.push_sequence);
    }

    fn start_pack_task(&self, key: PackKey, url: String, hash: String) {
        let Some(runtime) = &self.runtime else {
            return;
        };
        let download_cache_dir = self.download_cache_dir.clone();
        let materialized_session_dir = self.materialized_session_dir.clone();
        let username = self.username.clone();
        let profile_id = self.profile_id;
        let task_tx = self.task_tx.clone();
        runtime.spawn(async move {
            let expected_sha1 = (!hash.is_empty()).then_some(hash);
            let request = ServerPackDownloadRequest::new(
                url,
                expected_sha1,
                download_cache_dir,
                username,
                profile_id,
            );
            let download = match download_server_pack(request).await {
                Ok(download) => download,
                Err(error) => {
                    let _ = task_tx.send(PackTaskEvent::Failed {
                        key,
                        action: ResourcePackResponseAction::FailedDownload,
                        error: format!("{error:#}"),
                    });
                    return;
                }
            };
            if task_tx.send(PackTaskEvent::Downloaded { key }).is_err() {
                return;
            }

            let output_dir = materialized_session_dir.join(format!(
                "{:020}-{}-{}",
                key.push_sequence, key.id, download.sha1
            ));
            let materialized = tokio::task::spawn_blocking(move || {
                std::fs::create_dir_all(&materialized_session_dir).map_err(anyhow::Error::from)?;
                materialize_server_pack_archive(
                    &download.archive_path,
                    &output_dir,
                    ServerPackArchiveLimits::default(),
                )
            })
            .await;
            match materialized {
                Ok(Ok(pack)) => {
                    let _ = task_tx.send(PackTaskEvent::Ready {
                        key,
                        root: pack.root,
                    });
                }
                Ok(Err(error)) => {
                    let _ = task_tx.send(PackTaskEvent::Failed {
                        key,
                        action: ResourcePackResponseAction::FailedReload,
                        error: format!("{error:#}"),
                    });
                }
                Err(error) => {
                    let _ = task_tx.send(PackTaskEvent::Failed {
                        key,
                        action: ResourcePackResponseAction::FailedReload,
                        error: format!("join server pack materialization worker: {error}"),
                    });
                }
            }
        });
    }

    fn drain_task_events(&mut self) {
        while let Ok(event) = self.task_rx.try_recv() {
            self.apply_task_event(event);
        }
    }

    fn apply_task_event(&mut self, event: PackTaskEvent) {
        let key = event.key();
        let Some(pack) = self.packs.iter_mut().find(|pack| pack.key == key) else {
            if let PackTaskEvent::Ready { root, .. } = event {
                remove_materialized_root(&root);
            }
            return;
        };
        if pack.removed {
            match event {
                PackTaskEvent::Downloaded { .. } => {}
                PackTaskEvent::Ready { root, .. } => {
                    remove_materialized_root(&root);
                    pack.state = PackDownloadState::Failed;
                }
                PackTaskEvent::Failed { .. } => pack.state = PackDownloadState::Failed,
            }
            return;
        }

        match event {
            PackTaskEvent::Downloaded { .. } => {
                self.responses.push_back(DeferredResponse {
                    key,
                    action: ResourcePackResponseAction::Downloaded,
                });
            }
            PackTaskEvent::Ready { root, .. } => {
                pack.state = PackDownloadState::Ready(root);
            }
            PackTaskEvent::Failed { action, error, .. } => {
                tracing::warn!(
                    pack_id = %key.id,
                    push_sequence = key.push_sequence,
                    ?action,
                    %error,
                    "server resource pack preparation failed"
                );
                pack.state = PackDownloadState::Failed;
                self.responses.push_back(DeferredResponse { key, action });
            }
        }
    }

    fn cleanup_removed_packs(&mut self) {
        self.packs.retain(|pack| {
            if !pack.removed {
                return true;
            }
            match &pack.state {
                PackDownloadState::Pending => true,
                PackDownloadState::Ready(root) => {
                    remove_materialized_root(root);
                    false
                }
                PackDownloadState::Failed => false,
            }
        });
        self.responses.retain(|response| {
            self.packs
                .iter()
                .any(|pack| pack.key == response.key && !pack.removed)
        });
    }

    fn flush_responses(
        &mut self,
        world: &mut WorldStore,
        commands: &Option<mpsc::Sender<NetCommand>>,
    ) {
        let Some(commands) = commands else {
            return;
        };
        while let Some(response) = self.responses.front().copied() {
            let is_current = world
                .resource_pack(response.key.id)
                .is_some_and(|pack| pack.push_sequence == response.key.push_sequence);
            if !is_current {
                self.responses.pop_front();
                continue;
            }
            let command = NetCommand::ResourcePackResponse {
                id: response.key.id,
                action: response.action,
            };
            match commands.try_send(command) {
                Ok(()) => {
                    world.apply_resource_pack_response(response.key.id, response.action);
                    self.responses.pop_front();
                }
                Err(mpsc::error::TrySendError::Full(_)) => break,
                Err(mpsc::error::TrySendError::Closed(_)) => break,
            }
        }
    }

    fn report_ready_stack_change(&mut self) {
        let ready_roots = self
            .packs
            .iter()
            .filter(|pack| !pack.removed)
            .filter_map(|pack| match &pack.state {
                PackDownloadState::Ready(root) => Some(root.clone()),
                PackDownloadState::Pending | PackDownloadState::Failed => None,
            })
            .collect::<Vec<_>>();
        if ready_roots == self.last_reported_ready_roots {
            return;
        }
        tracing::info!(
            ready_server_packs = ready_roots.len(),
            "server resource pack preparation stack changed"
        );
        self.last_reported_ready_roots = ready_roots;
    }
}

fn remove_materialized_root(root: &Path) {
    if let Err(error) = std::fs::remove_dir_all(root) {
        if error.kind() != std::io::ErrorKind::NotFound {
            tracing::warn!(?error, path = %root.display(), "failed to remove materialized server pack");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bbb_protocol::packets::{ResourcePackPop, ResourcePackPush};
    use std::time::{SystemTime, UNIX_EPOCH};

    fn test_runtime(label: &str) -> ServerResourcePackRuntime {
        ServerResourcePackRuntime::with_optional_runtime(
            None,
            unique_temp_dir(label),
            "bbb-client".to_string(),
            Uuid::nil(),
        )
    }

    fn push_accepted(world: &mut WorldStore, id: Uuid, url: &str) -> PackKey {
        world.apply_resource_pack_push(ResourcePackPush {
            id,
            url: url.to_string(),
            hash: String::new(),
            required: false,
            prompt: None,
        });
        world.apply_resource_pack_response(id, ResourcePackResponseAction::Accepted);
        PackKey::from_world(world.resource_pack(id).unwrap())
    }

    #[test]
    fn accepted_packs_are_tracked_in_push_order() {
        let mut world = WorldStore::new();
        let first = push_accepted(
            &mut world,
            Uuid::from_u128(1),
            "https://example.test/first.zip",
        );
        let second = push_accepted(
            &mut world,
            Uuid::from_u128(2),
            "https://example.test/second.zip",
        );
        let mut runtime = test_runtime("order");

        runtime.reconcile_world(&world, true);

        assert_eq!(
            runtime
                .packs
                .iter()
                .map(|pack| pack.key)
                .collect::<Vec<_>>(),
            vec![first, second]
        );
    }

    #[test]
    fn downloaded_response_is_queued_once_and_updates_canonical_state_after_send() {
        let mut world = WorldStore::new();
        let id = Uuid::from_u128(1);
        let key = push_accepted(&mut world, id, "https://example.test/server-pack.zip");
        let mut runtime = test_runtime("response");
        runtime.reconcile_world(&world, true);
        runtime.apply_task_event(PackTaskEvent::Downloaded { key });
        let (commands, mut command_rx) = mpsc::channel(1);

        runtime.flush_responses(&mut world, &Some(commands));

        assert_eq!(
            command_rx.try_recv().unwrap(),
            NetCommand::ResourcePackResponse {
                id,
                action: ResourcePackResponseAction::Downloaded,
            }
        );
        assert_eq!(
            world
                .resource_pack(id)
                .unwrap()
                .last_response
                .unwrap()
                .action,
            ResourcePackResponseAction::Downloaded
        );
        assert!(runtime.responses.is_empty());
    }

    #[test]
    fn popped_pack_ignores_late_materialization_and_removes_its_root() {
        let mut world = WorldStore::new();
        let id = Uuid::from_u128(1);
        let key = push_accepted(&mut world, id, "https://example.test/server-pack.zip");
        let mut runtime = test_runtime("pop");
        runtime.reconcile_world(&world, true);
        world.apply_resource_pack_pop(ResourcePackPop { id: Some(id) });
        runtime.reconcile_world(&world, true);
        let root = runtime.materialized_session_dir.join("late");
        std::fs::create_dir_all(&root).unwrap();

        runtime.apply_task_event(PackTaskEvent::Ready {
            key,
            root: root.clone(),
        });
        runtime.cleanup_removed_packs();

        assert!(!root.exists());
        assert!(runtime.packs.is_empty());
        assert!(runtime.responses.is_empty());
    }

    #[test]
    fn replaced_uuid_drops_old_result_without_mutating_new_request() {
        let mut world = WorldStore::new();
        let id = Uuid::from_u128(1);
        let old = push_accepted(&mut world, id, "https://example.test/old.zip");
        let mut runtime = test_runtime("replace");
        runtime.reconcile_world(&world, true);
        let new = push_accepted(&mut world, id, "https://example.test/new.zip");
        runtime.reconcile_world(&world, true);
        let stale_root = runtime.materialized_session_dir.join("stale");
        std::fs::create_dir_all(&stale_root).unwrap();

        runtime.apply_task_event(PackTaskEvent::Ready {
            key: old,
            root: stale_root.clone(),
        });
        runtime.cleanup_removed_packs();

        assert!(!stale_root.exists());
        assert_eq!(runtime.packs.len(), 1);
        assert_eq!(runtime.packs[0].key, new);
        assert!(matches!(runtime.packs[0].state, PackDownloadState::Pending));
    }

    fn unique_temp_dir(label: &str) -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!(
            "bbb-native-server-pack-runtime-{label}-{}-{nonce}",
            std::process::id()
        ))
    }
}
