use std::{
    collections::{HashMap, HashSet, VecDeque},
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicU64, Ordering},
        mpsc as std_mpsc,
    },
    time::{SystemTime, UNIX_EPOCH},
};

use bbb_net::{download_server_pack, NetCommand, ServerPackDownloadRequest};
use bbb_pack::{
    materialize_server_pack_archive, SelectedClientPack, ServerPackArchiveLimits,
    ServerPackMaterialization,
};
use bbb_protocol::packets::ResourcePackResponseAction;
use bbb_world::{
    ResourcePackFailureStage, ResourcePackRuntimeFailure, ResourcePackRuntimePhase,
    ResourcePackRuntimeState, ResourcePackState, WorldStore,
};
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
    Ready(ServerPackMaterialization),
    Failed,
}

#[derive(Debug)]
struct TrackedPack {
    key: PackKey,
    download_batch: u64,
    state: PackDownloadState,
    removed: bool,
    active: bool,
    active_generation: Option<u64>,
    pending_terminal_response: Option<ResourcePackResponseAction>,
    runtime_state: ResourcePackRuntimeState,
}

#[derive(Debug, Clone)]
enum DownloadOutcome {
    Pending,
    Succeeded,
    Failed,
}

#[derive(Debug)]
struct DownloadBatch {
    id: u64,
    members: Vec<PackKey>,
    outcomes: HashMap<PackKey, DownloadOutcome>,
}

#[derive(Debug)]
enum PackTaskEvent {
    Downloaded {
        key: PackKey,
    },
    Ready {
        key: PackKey,
        materialization: ServerPackMaterialization,
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

#[derive(Debug)]
struct ReloadTransaction {
    generation: u64,
    previous: Vec<PackKey>,
    desired: Vec<PackKey>,
    report_successful_loads: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ServerResourcePackReload {
    pub(crate) generation: u64,
    pub(crate) server_packs: Vec<SelectedClientPack>,
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
    next_reload_generation: u64,
    reload: Option<ReloadTransaction>,
    force_reload_requested: bool,
    next_download_batch: u64,
    download_batches: Vec<DownloadBatch>,
    required_declines_disconnected: HashSet<PackKey>,
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
        let session_nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let materialized_session_dir = cache_dir.join("materialized").join(format!(
            "session-{}-{session_serial}-{session_nonce}",
            std::process::id()
        ));
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
            next_reload_generation: 1,
            reload: None,
            force_reload_requested: false,
            next_download_batch: 1,
            download_batches: Vec::new(),
            required_declines_disconnected: HashSet::new(),
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
        self.resolve_download_batches();
        self.resolve_preparation_failures();
        self.publish_runtime_diagnostics(world);
        self.cleanup_removed_packs();
        self.flush_responses(world, commands);
        self.disconnect_for_required_declines(world, commands, connected);
        self.cleanup_removed_packs();
        self.report_ready_stack_change();
    }

    pub(crate) fn take_reload(&mut self) -> Option<ServerResourcePackReload> {
        if self.reload.is_some()
            || self
                .packs
                .iter()
                .any(|pack| !pack.removed && matches!(pack.state, PackDownloadState::Pending))
        {
            return None;
        }
        let previous = self
            .packs
            .iter()
            .filter(|pack| pack.active)
            .map(|pack| pack.key)
            .collect::<Vec<_>>();
        let desired = self
            .packs
            .iter()
            .filter(|pack| !pack.removed)
            .filter(|pack| matches!(pack.state, PackDownloadState::Ready(_)))
            .map(|pack| pack.key)
            .collect::<Vec<_>>();
        let stack_changed = previous != desired;
        if !stack_changed && !self.force_reload_requested {
            return None;
        }
        self.force_reload_requested = false;
        let server_packs = desired
            .iter()
            .map(|key| {
                self.packs
                    .iter()
                    .find(|pack| pack.key == *key)
                    .and_then(|pack| match &pack.state {
                        PackDownloadState::Ready(materialization) => Some(materialization),
                        PackDownloadState::Pending | PackDownloadState::Failed => None,
                    })
                    .ok_or_else(|| format!("ready server pack {:?} disappeared", key))
                    .and_then(|materialization| {
                        SelectedClientPack::server(
                            key.id.to_string(),
                            key.push_sequence,
                            materialization.root.clone(),
                            materialization.metadata.clone(),
                        )
                        .map_err(|error| format!("{error:#}"))
                    })
            })
            .collect::<Result<Vec<_>, _>>();
        let server_packs = match server_packs {
            Ok(packs) => packs,
            Err(error) => {
                tracing::error!(%error, "ready server pack could not enter selected stack");
                return None;
            }
        };
        let generation = self.next_reload_generation;
        self.next_reload_generation = self.next_reload_generation.saturating_add(1);
        for pack in &mut self.packs {
            if desired.contains(&pack.key) {
                pack.runtime_state = ResourcePackRuntimeState {
                    phase: ResourcePackRuntimePhase::Applying,
                    generation: Some(generation),
                    failure: None,
                };
            }
        }
        self.reload = Some(ReloadTransaction {
            generation,
            previous,
            desired,
            report_successful_loads: stack_changed,
        });
        Some(ServerResourcePackReload {
            generation,
            server_packs,
        })
    }

    pub(crate) fn request_reload(&mut self) {
        self.force_reload_requested = true;
    }

    pub(crate) fn finish_reload(&mut self, generation: u64, result: Result<(), String>) -> bool {
        let Some(reload) = self.reload.take() else {
            return false;
        };
        if reload.generation != generation {
            self.reload = Some(reload);
            return false;
        }
        if result.is_ok() {
            for pack in &mut self.packs {
                let should_be_active = reload.desired.contains(&pack.key);
                pack.active = should_be_active;
                if should_be_active {
                    pack.active_generation = Some(generation);
                    pack.runtime_state = ResourcePackRuntimeState {
                        phase: ResourcePackRuntimePhase::Active,
                        generation: Some(generation),
                        failure: None,
                    };
                }
                if reload.report_successful_loads && should_be_active && !pack.removed {
                    self.responses.push_back(DeferredResponse {
                        key: pack.key,
                        action: ResourcePackResponseAction::SuccessfullyLoaded,
                    });
                }
            }
        } else {
            let error = result.expect_err("failed reload result has an error");
            for pack in &mut self.packs {
                if reload.desired.contains(&pack.key) && !reload.previous.contains(&pack.key) {
                    if let PackDownloadState::Ready(materialization) = &pack.state {
                        remove_materialized_root(&materialization.root);
                    }
                    pack.removed = true;
                    pack.state = PackDownloadState::Failed;
                    pack.runtime_state = failed_runtime_state(
                        ResourcePackFailureStage::Reload,
                        Some(generation),
                        error.clone(),
                    );
                    self.responses.push_back(DeferredResponse {
                        key: pack.key,
                        action: ResourcePackResponseAction::FailedReload,
                    });
                }
                pack.active = reload.previous.contains(&pack.key);
                if pack.active {
                    pack.runtime_state = ResourcePackRuntimeState {
                        phase: ResourcePackRuntimePhase::Active,
                        generation: pack.active_generation,
                        failure: Some(ResourcePackRuntimeFailure {
                            stage: ResourcePackFailureStage::Reload,
                            generation: Some(generation),
                            error: error.clone(),
                        }),
                    };
                }
            }
        }
        self.cleanup_removed_packs();
        true
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
        let additions = canonical
            .into_iter()
            .filter_map(|pack| {
                let key = PackKey::from_world(pack);
                let accepted = pack.last_response.is_some_and(|response| {
                    response.action == ResourcePackResponseAction::Accepted
                });
                if !accepted || self.packs.iter().any(|tracked| tracked.key == key) {
                    return None;
                }
                Some((key, pack.url.clone(), pack.hash.clone()))
            })
            .collect::<Vec<_>>();
        if additions.is_empty() {
            return;
        }
        let download_batch = self.next_download_batch;
        self.next_download_batch = self.next_download_batch.saturating_add(1);
        let members = additions.iter().map(|(key, _, _)| *key).collect::<Vec<_>>();
        self.download_batches.push(DownloadBatch {
            id: download_batch,
            outcomes: members
                .iter()
                .copied()
                .map(|key| (key, DownloadOutcome::Pending))
                .collect(),
            members,
        });
        for (key, url, hash) in additions {
            self.packs.push(TrackedPack {
                key,
                download_batch,
                state: PackDownloadState::Pending,
                removed: false,
                active: false,
                active_generation: None,
                pending_terminal_response: None,
                runtime_state: ResourcePackRuntimeState {
                    phase: ResourcePackRuntimePhase::Downloading,
                    generation: None,
                    failure: None,
                },
            });
            self.start_pack_task(key, url, hash);
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
                Ok(Ok(materialization)) => {
                    let _ = task_tx.send(PackTaskEvent::Ready {
                        key,
                        materialization,
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
        match &event {
            PackTaskEvent::Downloaded { .. } => {
                self.record_download_outcome(key, DownloadOutcome::Succeeded);
            }
            PackTaskEvent::Failed {
                action: ResourcePackResponseAction::FailedDownload,
                ..
            } => {
                self.record_download_outcome(key, DownloadOutcome::Failed);
            }
            PackTaskEvent::Ready { .. } | PackTaskEvent::Failed { .. } => {}
        }

        let Some(pack_index) = self.packs.iter().position(|pack| pack.key == key) else {
            if let PackTaskEvent::Ready {
                materialization, ..
            } = event
            {
                remove_materialized_root(&materialization.root);
            }
            return;
        };
        let pack = &mut self.packs[pack_index];
        if pack.removed {
            match event {
                PackTaskEvent::Downloaded { .. } => {}
                PackTaskEvent::Ready {
                    materialization, ..
                } => {
                    remove_materialized_root(&materialization.root);
                    pack.state = PackDownloadState::Failed;
                }
                PackTaskEvent::Failed { .. } => pack.state = PackDownloadState::Failed,
            }
            return;
        }

        match event {
            PackTaskEvent::Downloaded { .. } => {
                pack.runtime_state = ResourcePackRuntimeState {
                    phase: ResourcePackRuntimePhase::Preparing,
                    generation: None,
                    failure: None,
                };
            }
            PackTaskEvent::Ready {
                materialization, ..
            } => {
                pack.state = PackDownloadState::Ready(materialization);
                pack.runtime_state = ResourcePackRuntimeState {
                    phase: ResourcePackRuntimePhase::Ready,
                    generation: None,
                    failure: None,
                };
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
                let stage = if action == ResourcePackResponseAction::FailedDownload {
                    ResourcePackFailureStage::Download
                } else {
                    ResourcePackFailureStage::Materialize
                };
                pack.runtime_state = failed_runtime_state(stage, None, error);
                if action != ResourcePackResponseAction::FailedDownload {
                    pack.pending_terminal_response = Some(action);
                }
            }
        }
    }

    fn record_download_outcome(&mut self, key: PackKey, outcome: DownloadOutcome) {
        let batch_id = self
            .packs
            .iter()
            .find(|pack| pack.key == key)
            .map(|pack| pack.download_batch);
        let batch_index = batch_id
            .and_then(|id| {
                self.download_batches
                    .iter()
                    .position(|batch| batch.id == id)
            })
            .or_else(|| {
                self.download_batches
                    .iter()
                    .position(|batch| batch.members.contains(&key))
            });
        if let Some(batch_index) = batch_index {
            self.download_batches[batch_index]
                .outcomes
                .insert(key, outcome);
        }
    }

    fn resolve_download_batches(&mut self) {
        let resolved_ids = self
            .download_batches
            .iter()
            .filter(|batch| {
                batch
                    .outcomes
                    .values()
                    .all(|outcome| !matches!(outcome, DownloadOutcome::Pending))
            })
            .map(|batch| batch.id)
            .collect::<Vec<_>>();
        for batch_id in &resolved_ids {
            let Some(batch) = self
                .download_batches
                .iter()
                .find(|batch| batch.id == *batch_id)
            else {
                continue;
            };
            let members = batch.members.clone();
            let failed = batch
                .outcomes
                .iter()
                .filter_map(|(key, outcome)| {
                    matches!(outcome, DownloadOutcome::Failed).then_some(*key)
                })
                .collect::<HashSet<_>>();
            if failed.is_empty() {
                for key in members {
                    let Some(pack) = self.packs.iter_mut().find(|pack| pack.key == key) else {
                        continue;
                    };
                    if pack.removed {
                        continue;
                    }
                    self.responses.push_back(DeferredResponse {
                        key,
                        action: ResourcePackResponseAction::Downloaded,
                    });
                }
            } else {
                for key in members {
                    let Some(pack) = self.packs.iter_mut().find(|pack| pack.key == key) else {
                        continue;
                    };
                    if pack.removed || pack.active {
                        continue;
                    }
                    if let PackDownloadState::Ready(materialization) = &pack.state {
                        remove_materialized_root(&materialization.root);
                    }
                    pack.removed = true;
                    pack.state = PackDownloadState::Failed;
                    if !failed.contains(&key) {
                        pack.runtime_state = failed_runtime_state(
                            ResourcePackFailureStage::Download,
                            None,
                            "discarded because another pack in the same download batch failed"
                                .to_string(),
                        );
                    }
                    pack.pending_terminal_response = None;
                    self.responses.push_back(DeferredResponse {
                        key,
                        action: if failed.contains(&key) {
                            ResourcePackResponseAction::FailedDownload
                        } else {
                            ResourcePackResponseAction::Discarded
                        },
                    });
                }
            }
        }
        self.download_batches
            .retain(|batch| !resolved_ids.contains(&batch.id));
    }

    fn resolve_preparation_failures(&mut self) {
        let failed_batches = self
            .packs
            .iter()
            .filter(|pack| {
                !pack.removed
                    && !pack.active
                    && pack.pending_terminal_response
                        == Some(ResourcePackResponseAction::FailedReload)
            })
            .map(|pack| pack.download_batch)
            .collect::<HashSet<_>>();
        for batch_id in failed_batches {
            let waiting = self.packs.iter().any(|pack| {
                pack.download_batch == batch_id
                    && !pack.removed
                    && !pack.active
                    && matches!(pack.state, PackDownloadState::Pending)
            });
            if waiting {
                continue;
            }
            let keys = self
                .packs
                .iter()
                .filter(|pack| pack.download_batch == batch_id && !pack.removed && !pack.active)
                .map(|pack| pack.key)
                .collect::<Vec<_>>();
            for key in keys {
                let Some(pack) = self.packs.iter_mut().find(|pack| pack.key == key) else {
                    continue;
                };
                if let PackDownloadState::Ready(materialization) = &pack.state {
                    remove_materialized_root(&materialization.root);
                }
                pack.removed = true;
                pack.state = PackDownloadState::Failed;
                if pack.runtime_state.failure.is_none() {
                    pack.runtime_state = failed_runtime_state(
                        ResourcePackFailureStage::Materialize,
                        None,
                        "discarded because another pack in the same preparation batch failed"
                            .to_string(),
                    );
                }
                pack.pending_terminal_response = None;
                self.responses.push_back(DeferredResponse {
                    key,
                    action: ResourcePackResponseAction::FailedReload,
                });
            }
        }
    }

    fn cleanup_removed_packs(&mut self) {
        let pending_responses = self
            .responses
            .iter()
            .map(|response| response.key)
            .collect::<HashSet<_>>();
        self.packs.retain(|pack| {
            if !pack.removed || pack.active {
                return true;
            }
            if pending_responses.contains(&pack.key) {
                return true;
            }
            match &pack.state {
                PackDownloadState::Pending => true,
                PackDownloadState::Ready(materialization) => {
                    remove_materialized_root(&materialization.root);
                    false
                }
                PackDownloadState::Failed => false,
            }
        });
    }

    pub(crate) fn publish_runtime_diagnostics(&self, world: &mut WorldStore) {
        for pack in &self.packs {
            world.update_resource_pack_runtime_state(
                pack.key.id,
                pack.key.push_sequence,
                pack.runtime_state.clone(),
            );
        }
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

    fn disconnect_for_required_declines(
        &mut self,
        world: &WorldStore,
        commands: &Option<mpsc::Sender<NetCommand>>,
        connected: bool,
    ) {
        if !connected {
            self.required_declines_disconnected.clear();
            return;
        }
        let Some(commands) = commands else {
            return;
        };
        let declined = world
            .resource_packs()
            .values()
            .filter(|pack| {
                pack.required
                    && pack.last_response.is_some_and(|response| {
                        response.action == ResourcePackResponseAction::Declined
                    })
            })
            .map(PackKey::from_world)
            .find(|key| !self.required_declines_disconnected.contains(key));
        let Some(key) = declined else {
            return;
        };
        match commands.try_send(NetCommand::Disconnect) {
            Ok(()) => {
                self.required_declines_disconnected.insert(key);
                tracing::warn!(
                    pack_id = %key.id,
                    push_sequence = key.push_sequence,
                    "disconnecting because a required server resource pack was declined"
                );
            }
            Err(mpsc::error::TrySendError::Full(_)) | Err(mpsc::error::TrySendError::Closed(_)) => {
            }
        }
    }

    fn report_ready_stack_change(&mut self) {
        let ready_roots = self
            .packs
            .iter()
            .filter(|pack| !pack.removed)
            .filter_map(|pack| match &pack.state {
                PackDownloadState::Ready(materialization) => Some(materialization.root.clone()),
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

fn failed_runtime_state(
    stage: ResourcePackFailureStage,
    generation: Option<u64>,
    error: String,
) -> ResourcePackRuntimeState {
    ResourcePackRuntimeState {
        phase: ResourcePackRuntimePhase::Failed,
        generation,
        failure: Some(ResourcePackRuntimeFailure {
            stage,
            generation,
            error,
        }),
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
        runtime.resolve_download_batches();
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
    fn required_decline_queues_one_local_disconnect() {
        let mut world = WorldStore::new();
        let id = Uuid::from_u128(1);
        world.apply_resource_pack_push(ResourcePackPush {
            id,
            url: "https://example.test/required.zip".to_string(),
            hash: String::new(),
            required: true,
            prompt: Some("Required pack".to_string()),
        });
        world.apply_resource_pack_response(id, ResourcePackResponseAction::Declined);
        let mut runtime = test_runtime("required-decline");
        let (commands, mut command_rx) = mpsc::channel(2);

        runtime.disconnect_for_required_declines(&world, &Some(commands.clone()), true);
        runtime.disconnect_for_required_declines(&world, &Some(commands), true);

        assert_eq!(command_rx.try_recv().unwrap(), NetCommand::Disconnect);
        assert!(command_rx.try_recv().is_err());
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
            materialization: ready_materialization(root.clone()),
        });
        runtime.resolve_download_batches();
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
            materialization: ready_materialization(stale_root.clone()),
        });
        runtime.cleanup_removed_packs();

        assert!(!stale_root.exists());
        assert_eq!(runtime.packs.len(), 1);
        assert_eq!(runtime.packs[0].key, new);
        assert!(matches!(runtime.packs[0].state, PackDownloadState::Pending));
    }

    #[test]
    fn ready_pack_reload_reports_success_only_after_barrier_completion() {
        let mut world = WorldStore::new();
        let id = Uuid::from_u128(1);
        let key = push_accepted(&mut world, id, "https://example.test/server-pack.zip");
        let mut runtime = test_runtime("reload-success");
        runtime.reconcile_world(&world, true);
        let root = ready_root(&runtime, "ready");
        runtime.apply_task_event(PackTaskEvent::Downloaded { key });
        runtime.apply_task_event(PackTaskEvent::Ready {
            key,
            materialization: ready_materialization(root.clone()),
        });
        runtime.resolve_download_batches();

        let reload = runtime.take_reload().unwrap();
        assert_eq!(selected_roots(&reload), vec![root]);
        assert!(runtime
            .responses
            .iter()
            .all(|response| { response.action != ResourcePackResponseAction::SuccessfullyLoaded }));
        assert!(runtime.finish_reload(reload.generation, Ok(())));
        assert!(runtime.packs[0].active);
        assert_eq!(
            runtime
                .responses
                .iter()
                .map(|response| response.action)
                .collect::<Vec<_>>(),
            vec![
                ResourcePackResponseAction::Downloaded,
                ResourcePackResponseAction::SuccessfullyLoaded,
            ]
        );
    }

    #[test]
    fn stale_reload_ack_does_not_commit_or_discard_the_current_barrier() {
        let mut world = WorldStore::new();
        let id = Uuid::from_u128(1);
        let key = push_accepted(&mut world, id, "https://example.test/server-pack.zip");
        let mut runtime = test_runtime("stale-reload-ack");
        runtime.reconcile_world(&world, true);
        let root = ready_root(&runtime, "ready");
        runtime.apply_task_event(PackTaskEvent::Downloaded { key });
        runtime.apply_task_event(PackTaskEvent::Ready {
            key,
            materialization: ready_materialization(root),
        });
        runtime.resolve_download_batches();

        let reload = runtime.take_reload().unwrap();
        assert!(!runtime.finish_reload(reload.generation + 1, Ok(())));
        assert!(!runtime.packs[0].active);
        assert!(runtime.take_reload().is_none());
        assert!(runtime
            .responses
            .iter()
            .all(|response| { response.action != ResourcePackResponseAction::SuccessfullyLoaded }));

        assert!(runtime.finish_reload(reload.generation, Ok(())));
        assert!(runtime.packs[0].active);
    }

    #[test]
    fn accepted_pack_runs_download_apply_pop_restore_lifecycle() {
        let mut world = WorldStore::new();
        let id = Uuid::from_u128(1);
        let key = push_accepted(&mut world, id, "https://example.test/server-pack.zip");
        let mut runtime = test_runtime("full-lifecycle");
        runtime.reconcile_world(&world, true);
        let root = ready_root(&runtime, "ready");
        runtime.apply_task_event(PackTaskEvent::Downloaded { key });
        runtime.apply_task_event(PackTaskEvent::Ready {
            key,
            materialization: ready_materialization(root.clone()),
        });
        runtime.resolve_download_batches();
        runtime.publish_runtime_diagnostics(&mut world);
        assert_eq!(
            world.resource_pack(id).unwrap().runtime.phase,
            ResourcePackRuntimePhase::Ready
        );

        let (commands, mut command_rx) = mpsc::channel(4);
        runtime.flush_responses(&mut world, &Some(commands.clone()));
        assert_eq!(
            command_rx.try_recv().unwrap(),
            NetCommand::ResourcePackResponse {
                id,
                action: ResourcePackResponseAction::Downloaded,
            }
        );

        let load = runtime.take_reload().unwrap();
        runtime.publish_runtime_diagnostics(&mut world);
        let applying = &world.resource_pack(id).unwrap().runtime;
        assert_eq!(applying.phase, ResourcePackRuntimePhase::Applying);
        assert_eq!(applying.generation, Some(load.generation));
        assert!(runtime.finish_reload(load.generation, Ok(())));
        runtime.publish_runtime_diagnostics(&mut world);
        runtime.flush_responses(&mut world, &Some(commands));
        assert_eq!(
            command_rx.try_recv().unwrap(),
            NetCommand::ResourcePackResponse {
                id,
                action: ResourcePackResponseAction::SuccessfullyLoaded,
            }
        );
        let active = &world.resource_pack(id).unwrap().runtime;
        assert_eq!(active.phase, ResourcePackRuntimePhase::Active);
        assert_eq!(active.generation, Some(load.generation));
        assert_eq!(active.failure, None);

        world.apply_resource_pack_pop(ResourcePackPop { id: Some(id) });
        runtime.reconcile_world(&world, true);
        let unload = runtime.take_reload().unwrap();
        assert!(unload.server_packs.is_empty());
        assert!(runtime.finish_reload(unload.generation, Ok(())));
        runtime.cleanup_removed_packs();
        assert!(!root.exists());
        assert!(runtime.packs.is_empty());
        assert!(command_rx.try_recv().is_err());
    }

    #[test]
    fn transfer_keeps_active_pack_while_terminal_disconnect_unloads_it() {
        let mut world = WorldStore::new();
        let id = Uuid::from_u128(1);
        let key = push_accepted(&mut world, id, "https://example.test/server-pack.zip");
        let mut runtime = test_runtime("session-replacement");
        runtime.reconcile_world(&world, true);
        let root = ready_root(&runtime, "active");
        runtime.apply_task_event(PackTaskEvent::Downloaded { key });
        runtime.apply_task_event(PackTaskEvent::Ready {
            key,
            materialization: ready_materialization(root.clone()),
        });
        runtime.resolve_download_batches();
        let load = runtime.take_reload().unwrap();
        assert!(runtime.finish_reload(load.generation, Ok(())));
        runtime.responses.clear();

        world.apply_transfer(bbb_protocol::packets::Transfer {
            host: "next.example.com".to_string(),
            port: 25566,
        });
        world.begin_connection_session(true);
        runtime.reconcile_world(&world, true);
        assert!(runtime.take_reload().is_none());
        assert!(root.exists());
        assert!(runtime.packs[0].active);

        world.end_connection_session();
        runtime.reconcile_world(&world, false);
        let unload = runtime.take_reload().unwrap();
        assert!(unload.server_packs.is_empty());
        assert!(runtime.finish_reload(unload.generation, Ok(())));
        assert!(!root.exists());
        assert!(runtime.packs.is_empty());
    }

    #[test]
    fn active_pop_keeps_root_until_unload_reload_succeeds() {
        let mut world = WorldStore::new();
        let id = Uuid::from_u128(1);
        let key = push_accepted(&mut world, id, "https://example.test/server-pack.zip");
        let mut runtime = test_runtime("reload-pop");
        runtime.reconcile_world(&world, true);
        let root = ready_root(&runtime, "active");
        runtime.apply_task_event(PackTaskEvent::Downloaded { key });
        runtime.apply_task_event(PackTaskEvent::Ready {
            key,
            materialization: ready_materialization(root.clone()),
        });
        runtime.resolve_download_batches();
        let load = runtime.take_reload().unwrap();
        assert!(runtime.finish_reload(load.generation, Ok(())));
        runtime.responses.clear();

        world.apply_resource_pack_pop(ResourcePackPop { id: Some(id) });
        runtime.reconcile_world(&world, true);
        runtime.cleanup_removed_packs();
        assert!(root.exists());
        let unload = runtime.take_reload().unwrap();
        assert!(unload.server_packs.is_empty());

        assert!(runtime.finish_reload(unload.generation, Ok(())));
        assert!(!root.exists());
        assert!(runtime.packs.is_empty());
    }

    #[test]
    fn failed_reload_keeps_previous_active_stack_and_fails_only_new_pack() {
        let mut world = WorldStore::new();
        let old_id = Uuid::from_u128(1);
        let old_key = push_accepted(&mut world, old_id, "https://example.test/old.zip");
        let mut runtime = test_runtime("reload-failure");
        runtime.reconcile_world(&world, true);
        let old_root = ready_root(&runtime, "old");
        runtime.apply_task_event(PackTaskEvent::Downloaded { key: old_key });
        runtime.apply_task_event(PackTaskEvent::Ready {
            key: old_key,
            materialization: ready_materialization(old_root.clone()),
        });
        runtime.resolve_download_batches();
        let load_old = runtime.take_reload().unwrap();
        assert!(runtime.finish_reload(load_old.generation, Ok(())));
        runtime.responses.clear();

        let new_id = Uuid::from_u128(2);
        let new_key = push_accepted(&mut world, new_id, "https://example.test/new.zip");
        runtime.reconcile_world(&world, true);
        let new_root = ready_root(&runtime, "new");
        runtime.apply_task_event(PackTaskEvent::Downloaded { key: new_key });
        runtime.apply_task_event(PackTaskEvent::Ready {
            key: new_key,
            materialization: ready_materialization(new_root.clone()),
        });
        runtime.resolve_download_batches();
        let reload = runtime.take_reload().unwrap();
        assert_eq!(
            selected_roots(&reload),
            vec![old_root.clone(), new_root.clone()]
        );

        assert!(runtime.finish_reload(reload.generation, Err("reload failed".to_string())));
        runtime.publish_runtime_diagnostics(&mut world);
        assert!(old_root.exists());
        assert!(!new_root.exists());
        assert_eq!(
            runtime
                .responses
                .iter()
                .map(|response| response.action)
                .collect::<Vec<_>>(),
            vec![
                ResourcePackResponseAction::Downloaded,
                ResourcePackResponseAction::FailedReload,
            ]
        );
        assert_eq!(runtime.packs.len(), 2);
        assert!(runtime
            .packs
            .iter()
            .any(|pack| pack.key == old_key && pack.active));
        assert_eq!(
            world.resource_pack(old_id).unwrap().runtime,
            ResourcePackRuntimeState {
                phase: ResourcePackRuntimePhase::Active,
                generation: Some(load_old.generation),
                failure: Some(ResourcePackRuntimeFailure {
                    stage: ResourcePackFailureStage::Reload,
                    generation: Some(reload.generation),
                    error: "reload failed".to_string(),
                }),
            }
        );
        let failed = &world.resource_pack(new_id).unwrap().runtime;
        assert_eq!(failed.phase, ResourcePackRuntimePhase::Failed);
        assert_eq!(failed.generation, Some(reload.generation));
        assert_eq!(
            failed.failure.as_ref().unwrap().stage,
            ResourcePackFailureStage::Reload
        );
        assert_eq!(
            failed.failure.as_ref().unwrap().generation,
            Some(reload.generation)
        );
        assert_eq!(failed.failure.as_ref().unwrap().error, "reload failed");
        runtime.responses.clear();
        runtime.cleanup_removed_packs();
        assert_eq!(runtime.packs.len(), 1);
        assert_eq!(runtime.packs[0].key, old_key);
        assert!(runtime.take_reload().is_none());
    }

    #[test]
    fn explicit_reload_rebuilds_unchanged_active_stack_once() {
        let mut world = WorldStore::new();
        let id = Uuid::from_u128(1);
        let key = push_accepted(&mut world, id, "https://example.test/server-pack.zip");
        let mut runtime = test_runtime("explicit-reload");
        runtime.reconcile_world(&world, true);
        let root = ready_root(&runtime, "active");
        runtime.apply_task_event(PackTaskEvent::Downloaded { key });
        runtime.apply_task_event(PackTaskEvent::Ready {
            key,
            materialization: ready_materialization(root.clone()),
        });
        runtime.resolve_download_batches();
        let initial = runtime.take_reload().unwrap();
        assert!(runtime.finish_reload(initial.generation, Ok(())));
        runtime.responses.clear();
        assert!(runtime.take_reload().is_none());

        runtime.request_reload();
        let forced = runtime.take_reload().unwrap();
        assert_eq!(selected_roots(&forced), vec![root]);
        assert!(runtime.finish_reload(forced.generation, Ok(())));
        assert!(runtime.responses.is_empty());
        assert!(runtime.take_reload().is_none());
    }

    #[test]
    fn one_download_failure_discards_other_inactive_pack_in_the_same_batch() {
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
        let mut runtime = test_runtime("batch-failure");
        runtime.reconcile_world(&world, true);

        runtime.apply_task_event(PackTaskEvent::Downloaded { key: first });
        runtime.apply_task_event(PackTaskEvent::Failed {
            key: second,
            action: ResourcePackResponseAction::FailedDownload,
            error: "connection reset".to_string(),
        });
        runtime.resolve_download_batches();
        runtime.cleanup_removed_packs();

        assert_eq!(runtime.packs.len(), 2);
        assert!(runtime.packs.iter().all(|pack| pack.removed));
        assert_eq!(
            runtime
                .responses
                .iter()
                .map(|response| (response.key, response.action))
                .collect::<Vec<_>>(),
            vec![
                (first, ResourcePackResponseAction::Discarded),
                (second, ResourcePackResponseAction::FailedDownload),
            ]
        );
        runtime.reconcile_world(&world, true);
        assert_eq!(runtime.packs.len(), 2);
        runtime.responses.clear();
        runtime.cleanup_removed_packs();
        assert!(runtime.packs.is_empty());
        assert!(runtime.take_reload().is_none());
    }

    #[test]
    fn one_preparation_failure_fails_every_new_pack_in_the_same_batch() {
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
        let mut runtime = test_runtime("preparation-failure");
        runtime.reconcile_world(&world, true);
        let first_root = ready_root(&runtime, "first");

        runtime.apply_task_event(PackTaskEvent::Downloaded { key: first });
        runtime.apply_task_event(PackTaskEvent::Ready {
            key: first,
            materialization: ready_materialization(first_root.clone()),
        });
        runtime.apply_task_event(PackTaskEvent::Downloaded { key: second });
        runtime.apply_task_event(PackTaskEvent::Failed {
            key: second,
            action: ResourcePackResponseAction::FailedReload,
            error: "invalid pack metadata".to_string(),
        });
        runtime.resolve_download_batches();
        runtime.resolve_preparation_failures();
        runtime.cleanup_removed_packs();

        assert!(!first_root.exists());
        assert_eq!(runtime.packs.len(), 2);
        assert!(runtime.packs.iter().all(|pack| pack.removed));
        assert_eq!(
            runtime
                .responses
                .iter()
                .map(|response| (response.key, response.action))
                .collect::<Vec<_>>(),
            vec![
                (first, ResourcePackResponseAction::Downloaded),
                (second, ResourcePackResponseAction::Downloaded),
                (first, ResourcePackResponseAction::FailedReload),
                (second, ResourcePackResponseAction::FailedReload),
            ]
        );
        runtime.reconcile_world(&world, true);
        assert_eq!(runtime.packs.len(), 2);
        runtime.responses.clear();
        runtime.cleanup_removed_packs();
        assert!(runtime.packs.is_empty());
        assert!(runtime.take_reload().is_none());
    }

    fn ready_root(runtime: &ServerResourcePackRuntime, name: &str) -> PathBuf {
        let root = runtime.materialized_session_dir.join(name);
        std::fs::create_dir_all(&root).unwrap();
        root
    }

    fn ready_materialization(root: PathBuf) -> ServerPackMaterialization {
        ServerPackMaterialization {
            root,
            archive_bytes: 0,
            entries: 0,
            files: 0,
            uncompressed_bytes: 0,
            metadata: bbb_pack::ClientPackMetadata::parse(
                br#"{"pack":{"description":"test","min_format":[84,0],"max_format":[84,0]}}"#,
            )
            .unwrap(),
        }
    }

    fn selected_roots(reload: &ServerResourcePackReload) -> Vec<PathBuf> {
        reload
            .server_packs
            .iter()
            .map(|pack| pack.root.clone())
            .collect()
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
