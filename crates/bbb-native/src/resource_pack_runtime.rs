use std::{
    collections::{HashMap, HashSet, VecDeque},
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, AtomicU64, Ordering},
        mpsc as std_mpsc, Arc,
    },
    time::{SystemTime, UNIX_EPOCH},
};

use bbb_net::{
    append_server_pack_download_log, download_server_pack_with_progress,
    initialize_server_pack_download_cache, NetCommand, ServerPackDownloadProgress,
    ServerPackDownloadProxy, ServerPackDownloadRequest,
};
use bbb_pack::{prepare_pack_archive, PreparedPackArchive, SelectedClientPack};
use bbb_protocol::packets::ResourcePackResponseAction;
use bbb_world::{
    ResourcePackDownloadPhase, ResourcePackDownloadState, ResourcePackFailureStage,
    ResourcePackRuntimeFailure, ResourcePackRuntimePhase, ResourcePackRuntimeState,
    ResourcePackState, WorldStore,
};
use tokio::{runtime::Handle, sync::mpsc};
use uuid::Uuid;

static SESSION_SERIAL: AtomicU64 = AtomicU64::new(0);

pub(crate) fn default_server_resource_pack_cache_dir(game_dir: &Path) -> PathBuf {
    game_dir.join("downloads")
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
    Ready(PreparedPackArchive),
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
    requests: Vec<PackDownloadRequest>,
    started: bool,
}

#[derive(Debug, Clone)]
struct PackDownloadRequest {
    key: PackKey,
    url: String,
    hash: String,
}

#[derive(Debug)]
enum PackTaskEvent {
    DownloadProgress {
        batch_id: u64,
        key: PackKey,
        current_pack: u32,
        total_packs: u32,
        progress: ServerPackDownloadProgress,
    },
    Downloaded {
        key: PackKey,
    },
    Ready {
        key: PackKey,
        materialization: PreparedPackArchive,
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
            Self::DownloadProgress { key, .. }
            | Self::Downloaded { key }
            | Self::Ready { key, .. }
            | Self::Failed { key, .. } => *key,
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
    is_recovery: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ReloadStack {
    previous: Vec<PackKey>,
    desired: Vec<PackKey>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ServerResourcePackReload {
    pub(crate) generation: u64,
    pub(crate) server_packs: Vec<SelectedClientPack>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ResourcePackReloadFinish {
    Accepted,
    RecoverySucceeded,
    RecoveryFailed,
}

/// Owns server-pack downloads, archive preparation, and lazy entry caching for one connection.
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
    download_proxy: Option<ServerPackDownloadProxy>,
    task_tx: std_mpsc::Sender<PackTaskEvent>,
    task_rx: std_mpsc::Receiver<PackTaskEvent>,
    packs: Vec<TrackedPack>,
    responses: VecDeque<DeferredResponse>,
    last_reported_ready_roots: Vec<PathBuf>,
    pack_id_serial_number: u32,
    next_reload_generation: u64,
    reload: Option<ReloadTransaction>,
    force_reload_requested: bool,
    recovery_reload_requested: bool,
    blocked_failed_stack: Option<ReloadStack>,
    next_download_batch: u64,
    download_batches: Vec<DownloadBatch>,
    required_declines_disconnected: HashSet<PackKey>,
    session_alive: Arc<AtomicBool>,
    download_presentations: Vec<ResourcePackDownloadState>,
    next_download_update_sequence: u64,
}

impl ServerResourcePackRuntime {
    pub(crate) fn new(
        runtime: Handle,
        cache_dir: impl Into<PathBuf>,
        username: impl Into<String>,
        profile_id: Uuid,
        download_proxy: Option<ServerPackDownloadProxy>,
    ) -> anyhow::Result<Self> {
        let cache_dir = cache_dir.into();
        initialize_server_pack_download_cache(&cache_dir)?;
        Ok(Self::with_optional_runtime(
            Some(runtime),
            cache_dir,
            username.into(),
            profile_id,
            download_proxy,
        ))
    }

    fn with_optional_runtime(
        runtime: Option<Handle>,
        cache_dir: PathBuf,
        username: String,
        profile_id: Uuid,
        download_proxy: Option<ServerPackDownloadProxy>,
    ) -> Self {
        let session_serial = SESSION_SERIAL.fetch_add(1, Ordering::Relaxed);
        let session_nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let materialized_session_dir = std::env::temp_dir()
            .join("bbb-native-server-pack-cache")
            .join(format!(
                "session-{}-{session_serial}-{session_nonce}",
                std::process::id()
            ));
        let (task_tx, task_rx) = std_mpsc::channel();
        Self {
            runtime,
            download_cache_dir: cache_dir,
            materialized_session_dir,
            username,
            profile_id,
            download_proxy,
            task_tx,
            task_rx,
            packs: Vec::new(),
            responses: VecDeque::new(),
            last_reported_ready_roots: Vec::new(),
            pack_id_serial_number: 0,
            next_reload_generation: 1,
            reload: None,
            force_reload_requested: false,
            recovery_reload_requested: false,
            blocked_failed_stack: None,
            next_download_batch: 1,
            download_batches: Vec::new(),
            required_declines_disconnected: HashSet::new(),
            session_alive: Arc::new(AtomicBool::new(true)),
            download_presentations: Vec::new(),
            next_download_update_sequence: 1,
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
        self.start_next_download_batch();
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
        let forced = self.force_reload_requested;
        let is_recovery = self.recovery_reload_requested;
        if !stack_changed && !forced {
            return None;
        }
        let stack = ReloadStack {
            previous: previous.clone(),
            desired: desired.clone(),
        };
        if !forced && self.blocked_failed_stack.as_ref() == Some(&stack) {
            return None;
        }
        self.blocked_failed_stack = None;
        let mut server_packs = Vec::with_capacity(desired.len());
        let mut server_pack_error = None;
        // Vanilla `loadRequestedPacks` reverses the manager's push-order list, then assigns one
        // `%08X` serial per load attempt. Repository discovery sorts those ids, preserving this
        // reverse-push order and making the oldest active server pack highest priority.
        for key in desired.iter().rev() {
            let load_serial = self.pack_id_serial_number;
            self.pack_id_serial_number = self.pack_id_serial_number.wrapping_add(1);
            let result = self
                .packs
                .iter()
                .find(|pack| pack.key == *key)
                .and_then(|pack| match &pack.state {
                    PackDownloadState::Ready(materialization) => Some(materialization),
                    PackDownloadState::Pending | PackDownloadState::Failed => None,
                })
                .ok_or_else(|| format!("ready server pack {:?} disappeared", key))
                .and_then(|materialization| {
                    SelectedClientPack::server_archive(
                        key.id.to_string(),
                        key.push_sequence,
                        load_serial,
                        materialization.clone(),
                    )
                    .map_err(|error| format!("{error:#}"))
                });
            match result {
                Ok(pack) => server_packs.push(pack),
                Err(error) => {
                    server_pack_error = Some(error);
                    break;
                }
            }
        }
        let server_packs = match server_pack_error {
            Some(error) => Err(error),
            None => Ok(server_packs),
        };
        let server_packs = match server_packs {
            Ok(packs) => packs,
            Err(error) => {
                tracing::error!(%error, "ready server pack could not enter selected stack");
                return None;
            }
        };
        self.force_reload_requested = false;
        self.recovery_reload_requested = false;
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
            is_recovery,
        });
        Some(ServerResourcePackReload {
            generation,
            server_packs,
        })
    }

    pub(crate) fn request_reload(&mut self) {
        self.force_reload_requested = true;
    }

    pub(crate) fn finish_reload(
        &mut self,
        generation: u64,
        result: Result<(), String>,
    ) -> Option<ResourcePackReloadFinish> {
        let Some(reload) = self.reload.take() else {
            return None;
        };
        if reload.generation != generation {
            self.reload = Some(reload);
            return None;
        }
        let finish = match (reload.is_recovery, result.is_ok()) {
            (true, true) => ResourcePackReloadFinish::RecoverySucceeded,
            (true, false) => ResourcePackReloadFinish::RecoveryFailed,
            (false, _) => ResourcePackReloadFinish::Accepted,
        };
        if result.is_ok() {
            self.blocked_failed_stack = None;
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
                        remove_materialized_root(&materialization.cache_root);
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
            if reload.is_recovery {
                self.blocked_failed_stack = Some(ReloadStack {
                    previous: reload.previous.clone(),
                    desired: reload.desired.clone(),
                });
            } else {
                // Vanilla rolls a failed normal reload into exactly one
                // recovery reload for the post-failure fallback stack. BBB's
                // renderer transaction already preserved the prior accepted
                // generation atomically, but the recovery pass is still the
                // bounded validation barrier. A second failure is terminal
                // until an explicit reload or a different stack arrives.
                self.force_reload_requested = true;
                self.recovery_reload_requested = true;
            }
        }
        self.cleanup_removed_packs();
        Some(finish)
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
        for (key, url, hash) in additions {
            let download_batch = self.next_download_batch;
            self.next_download_batch = self.next_download_batch.saturating_add(1);
            let request = PackDownloadRequest { key, url, hash };
            self.download_batches.push(DownloadBatch {
                id: download_batch,
                outcomes: [(key, DownloadOutcome::Pending)].into_iter().collect(),
                members: vec![key],
                requests: vec![request],
                started: false,
            });
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
        }
        self.packs.sort_by_key(|pack| pack.key.push_sequence);
    }

    fn start_next_download_batch(&mut self) {
        if self.download_batches.iter().any(|batch| batch.started) {
            return;
        }
        let Some(batch) = self
            .download_batches
            .iter_mut()
            .find(|batch| !batch.started)
        else {
            return;
        };
        let batch_id = batch.id;
        let requests = batch.requests.clone();
        batch.started = true;
        self.start_pack_batch(batch_id, requests);
    }

    fn start_pack_batch(&self, batch_id: u64, requests: Vec<PackDownloadRequest>) {
        let Some(runtime) = &self.runtime else {
            return;
        };
        let download_cache_dir = self.download_cache_dir.clone();
        let materialized_session_dir = self.materialized_session_dir.clone();
        let username = self.username.clone();
        let profile_id = self.profile_id;
        let download_proxy = self.download_proxy.clone();
        let task_tx = self.task_tx.clone();
        let session_alive = Arc::clone(&self.session_alive);
        runtime.spawn(async move {
            let total_packs = u32::try_from(requests.len()).unwrap_or(u32::MAX);
            let mut downloaded = Vec::with_capacity(requests.len());
            let mut batch_failed = false;
            for (index, request_spec) in requests.iter().cloned().enumerate() {
                if !session_alive.load(Ordering::Acquire) {
                    return;
                }
                let key = request_spec.key;
                let current_pack = u32::try_from(index + 1).unwrap_or(u32::MAX);
                let progress_tx = task_tx.clone();
                let expected_sha1 = (!request_spec.hash.is_empty()).then_some(request_spec.hash);
                let download_url = request_spec.url;
                let request = ServerPackDownloadRequest::new(
                    download_url.clone(),
                    expected_sha1.clone(),
                    download_cache_dir.join(key.id.to_string()),
                    username.clone(),
                    profile_id,
                )
                .with_proxy(download_proxy.clone());
                let result = download_server_pack_with_progress(request, move |progress| {
                    let _ = progress_tx.send(PackTaskEvent::DownloadProgress {
                        batch_id,
                        key,
                        current_pack,
                        total_packs,
                        progress,
                    });
                })
                .await;
                if let Err(error) = append_server_pack_download_log(
                    &download_cache_dir,
                    key.id,
                    &download_url,
                    expected_sha1.as_deref(),
                    result.as_ref().ok(),
                ) {
                    tracing::warn!(%error, "server pack download event could not be logged");
                }
                match result {
                    Ok(download) => {
                        if task_tx.send(PackTaskEvent::Downloaded { key }).is_err() {
                            return;
                        }
                        downloaded.push((key, download));
                    }
                    Err(error) => {
                        batch_failed = true;
                        let _ = task_tx.send(PackTaskEvent::Failed {
                            key,
                            action: ResourcePackResponseAction::FailedDownload,
                            error: format!("{error:#}"),
                        });
                    }
                }
            }
            if batch_failed || !session_alive.load(Ordering::Acquire) {
                return;
            }

            let mut prepared = downloaded.into_iter().peekable();
            while let Some((key, download)) = prepared.next() {
                let output_dir = materialized_session_dir.join(format!(
                    "{:020}-{}-{}",
                    key.push_sequence, key.id, download.sha1
                ));
                let task_session_dir = materialized_session_dir.clone();
                let materialized = tokio::task::spawn_blocking(move || {
                    std::fs::create_dir_all(&task_session_dir).map_err(anyhow::Error::from)?;
                    prepare_pack_archive(&download.archive_path, &output_dir)
                })
                .await;
                match materialized {
                    Ok(Ok(materialization)) => {
                        if !session_alive.load(Ordering::Acquire) {
                            remove_materialized_root(&materialization.cache_root);
                            break;
                        }
                        if let Err(error) = task_tx.send(PackTaskEvent::Ready {
                            key,
                            materialization,
                        }) {
                            if let PackTaskEvent::Ready {
                                materialization, ..
                            } = error.0
                            {
                                remove_materialized_root(&materialization.cache_root);
                            }
                            break;
                        }
                    }
                    Ok(Err(error)) => {
                        let message = format!("{error:#}");
                        let _ = task_tx.send(PackTaskEvent::Failed {
                            key,
                            action: ResourcePackResponseAction::FailedReload,
                            error: message.clone(),
                        });
                        for (discarded_key, _) in prepared {
                            let _ = task_tx.send(PackTaskEvent::Failed {
                                key: discarded_key,
                                action: ResourcePackResponseAction::FailedReload,
                                error: format!(
                                    "discarded because another pack in the same preparation batch failed: {message}"
                                ),
                            });
                        }
                        break;
                    }
                    Err(error) => {
                        let message = format!("join server pack preparation worker: {error}");
                        let _ = task_tx.send(PackTaskEvent::Failed {
                            key,
                            action: ResourcePackResponseAction::FailedReload,
                            error: message.clone(),
                        });
                        for (discarded_key, _) in prepared {
                            let _ = task_tx.send(PackTaskEvent::Failed {
                                key: discarded_key,
                                action: ResourcePackResponseAction::FailedReload,
                                error: format!(
                                    "discarded because another pack in the same preparation batch failed: {message}"
                                ),
                            });
                        }
                        break;
                    }
                }
            }
            if !session_alive.load(Ordering::Acquire) {
                let _ = std::fs::remove_dir_all(&materialized_session_dir);
            }
        });
    }

    fn drain_task_events(&mut self) {
        while let Ok(event) = self.task_rx.try_recv() {
            self.apply_task_event(event);
        }
    }

    fn apply_task_event(&mut self, event: PackTaskEvent) {
        if let PackTaskEvent::DownloadProgress {
            batch_id,
            key,
            current_pack,
            total_packs,
            progress,
        } = &event
        {
            self.apply_download_progress(*batch_id, *key, *current_pack, *total_packs, *progress);
            return;
        }
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
            PackTaskEvent::DownloadProgress { .. }
            | PackTaskEvent::Ready { .. }
            | PackTaskEvent::Failed { .. } => {}
        }

        let Some(pack_index) = self.packs.iter().position(|pack| pack.key == key) else {
            if let PackTaskEvent::Ready {
                materialization, ..
            } = event
            {
                remove_materialized_root(&materialization.cache_root);
            }
            return;
        };
        let pack = &mut self.packs[pack_index];
        if pack.removed {
            match event {
                PackTaskEvent::DownloadProgress { .. } => unreachable!(),
                PackTaskEvent::Downloaded { .. } => {}
                PackTaskEvent::Ready {
                    materialization, ..
                } => {
                    remove_materialized_root(&materialization.cache_root);
                    pack.state = PackDownloadState::Failed;
                }
                PackTaskEvent::Failed { .. } => pack.state = PackDownloadState::Failed,
            }
            return;
        }

        match event {
            PackTaskEvent::DownloadProgress { .. } => unreachable!(),
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

    fn apply_download_progress(
        &mut self,
        batch_id: u64,
        key: PackKey,
        current_pack: u32,
        total_packs: u32,
        progress: ServerPackDownloadProgress,
    ) {
        let current_batch = self
            .download_batches
            .iter()
            .any(|batch| batch.id == batch_id && batch.members.contains(&key));
        if !current_batch {
            return;
        }
        let previous = self
            .download_presentations
            .iter()
            .find(|state| state.batch_sequence == batch_id);
        let failed_packs = previous.map_or(0, |state| state.failed_packs);
        let update_sequence = self.next_download_update_sequence;
        self.next_download_update_sequence = self.next_download_update_sequence.saturating_add(1);
        let mut state = ResourcePackDownloadState {
            batch_sequence: batch_id,
            update_sequence,
            current_pack,
            total_packs,
            downloaded_bytes: 0,
            total_bytes: None,
            failed_packs,
            phase: ResourcePackDownloadPhase::Downloading,
            toast_action: bbb_world::ResourcePackDownloadToastAction::None,
        };
        if let Some(previous) = previous {
            state.downloaded_bytes = previous.downloaded_bytes;
            state.total_bytes = previous.total_bytes;
        }
        match progress {
            ServerPackDownloadProgress::RequestStarted => {
                // `requestStart` changes only the notifier title. Vanilla
                // retains the previous request's progress message until the
                // next `downloadStart` supplies a new size.
                state.toast_action =
                    bbb_world::ResourcePackDownloadToastAction::AddOrUpdateTitlePreserveMessage;
            }
            ServerPackDownloadProgress::DownloadStarted { total_bytes } => {
                state.downloaded_bytes = 0;
                state.total_bytes = total_bytes;
                state.toast_action =
                    bbb_world::ResourcePackDownloadToastAction::AddOrUpdateProgress;
            }
            ServerPackDownloadProgress::DownloadedBytes { bytes } => {
                state.downloaded_bytes = bytes;
                state.toast_action =
                    bbb_world::ResourcePackDownloadToastAction::AddOrUpdateProgress;
            }
            ServerPackDownloadProgress::RequestFinished { success } => {
                if !success {
                    state.failed_packs = state.failed_packs.saturating_add(1);
                }
                if current_pack == total_packs {
                    state.phase = if state.failed_packs == 0 {
                        ResourcePackDownloadPhase::Succeeded
                    } else {
                        ResourcePackDownloadPhase::Failed
                    };
                    state.toast_action = if state.failed_packs == 0 {
                        bbb_world::ResourcePackDownloadToastAction::ForceHide
                    } else {
                        bbb_world::ResourcePackDownloadToastAction::AddOrUpdateTitle
                    };
                }
            }
        }
        if let Some(current) = self
            .download_presentations
            .iter_mut()
            .find(|current| current.batch_sequence == batch_id)
        {
            *current = state;
        } else {
            self.download_presentations.push(state);
            self.download_presentations
                .sort_by_key(|state| state.batch_sequence);
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
                for pack in &mut self.packs {
                    if pack.removed || pack.active {
                        continue;
                    }
                    let key = pack.key;
                    if let PackDownloadState::Ready(materialization) = &pack.state {
                        remove_materialized_root(&materialization.cache_root);
                    }
                    pack.removed = true;
                    pack.state = PackDownloadState::Failed;
                    if !failed.contains(&key) {
                        pack.runtime_state = failed_runtime_state(
                            ResourcePackFailureStage::Download,
                            None,
                            "discarded because another server pack download failed".to_string(),
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
        let has_pending_download = self.packs.iter().any(|pack| {
            !pack.removed && !pack.active && matches!(pack.state, PackDownloadState::Pending)
        });
        if has_pending_download {
            return;
        }
        let has_preparation_failure = self.packs.iter().any(|pack| {
            !pack.removed
                && !pack.active
                && pack.pending_terminal_response == Some(ResourcePackResponseAction::FailedReload)
        });
        if !has_preparation_failure {
            return;
        }
        for pack in &mut self.packs {
            if pack.removed || pack.active {
                continue;
            }
            let key = pack.key;
            if let PackDownloadState::Ready(materialization) = &pack.state {
                remove_materialized_root(&materialization.cache_root);
            }
            pack.removed = true;
            pack.state = PackDownloadState::Failed;
            if pack.runtime_state.failure.is_none() {
                pack.runtime_state = failed_runtime_state(
                    ResourcePackFailureStage::Materialize,
                    None,
                    "discarded because another server pack preparation failed".to_string(),
                );
            }
            pack.pending_terminal_response = None;
            self.responses.push_back(DeferredResponse {
                key,
                action: ResourcePackResponseAction::FailedReload,
            });
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
                    remove_materialized_root(&materialization.cache_root);
                    false
                }
                PackDownloadState::Failed => false,
            }
        });
    }

    pub(crate) fn publish_runtime_diagnostics(&self, world: &mut WorldStore) {
        for pack in &self.packs {
            let changed = world.resource_pack(pack.key.id).is_some_and(|canonical| {
                canonical.push_sequence == pack.key.push_sequence
                    && canonical.runtime != pack.runtime_state
            });
            if changed {
                world.update_resource_pack_runtime_state(
                    pack.key.id,
                    pack.key.push_sequence,
                    pack.runtime_state.clone(),
                );
            }
        }
        if world.resource_pack_download_states() != self.download_presentations {
            world.set_resource_pack_download_states(self.download_presentations.clone());
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
                PackDownloadState::Ready(materialization) => {
                    Some(materialization.cache_root.clone())
                }
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

impl Drop for ServerResourcePackRuntime {
    fn drop(&mut self) {
        self.session_alive.store(false, Ordering::Release);
        if let Err(error) = std::fs::remove_dir_all(&self.materialized_session_dir) {
            if error.kind() != std::io::ErrorKind::NotFound {
                tracing::warn!(
                    ?error,
                    path = %self.materialized_session_dir.display(),
                    "failed to remove server pack entry-cache session"
                );
            }
        }
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
            tracing::warn!(?error, path = %root.display(), "failed to remove server pack entry cache");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bbb_protocol::packets::{ResourcePackPop, ResourcePackPush};
    use std::{
        fs::File,
        io::Write,
        time::{SystemTime, UNIX_EPOCH},
    };
    use zip::{write::SimpleFileOptions, CompressionMethod, ZipWriter};

    fn test_runtime(label: &str) -> ServerResourcePackRuntime {
        ServerResourcePackRuntime::with_optional_runtime(
            None,
            unique_temp_dir(label),
            "bbb-client".to_string(),
            Uuid::nil(),
            None,
        )
    }

    #[test]
    fn default_server_pack_cache_uses_vanilla_game_directory_downloads() {
        assert_eq!(
            default_server_resource_pack_cache_dir(Path::new("/game")),
            PathBuf::from("/game/downloads")
        );
    }

    #[test]
    fn server_pack_runtime_initializes_the_exact_vanilla_download_queue_root() {
        let async_runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let cache_dir = unique_temp_dir("queue-root");
        let runtime = ServerResourcePackRuntime::new(
            async_runtime.handle().clone(),
            &cache_dir,
            "bbb-client",
            Uuid::nil(),
            None,
        )
        .unwrap();

        assert_eq!(runtime.download_cache_dir, cache_dir);
        assert!(runtime.download_cache_dir.join("log.json").is_file());
        assert!(!runtime.download_cache_dir.join("downloads").exists());

        drop(runtime);
        std::fs::remove_dir_all(cache_dir).unwrap();
    }

    #[test]
    fn failed_runtime_download_writes_vanilla_download_queue_event() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        drop(listener);
        let url = format!("http://{address}/failed.zip");
        let pack_id = Uuid::from_u128(9);
        let mut world = WorldStore::new();
        push_accepted(&mut world, pack_id, &url);
        let async_runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let cache_dir = unique_temp_dir("queue-event");
        let mut runtime = ServerResourcePackRuntime::new(
            async_runtime.handle().clone(),
            &cache_dir,
            "bbb-client",
            Uuid::nil(),
            None,
        )
        .unwrap();

        async_runtime.block_on(async {
            runtime.synchronize(&mut world, &None, true);
            for _ in 0..200 {
                tokio::time::sleep(std::time::Duration::from_millis(5)).await;
                runtime.synchronize(&mut world, &None, true);
                if std::fs::read_to_string(cache_dir.join("log.json"))
                    .unwrap()
                    .lines()
                    .next()
                    .is_some()
                {
                    return;
                }
            }
            panic!("server pack failure did not reach the download queue event log");
        });

        let raw = std::fs::read_to_string(cache_dir.join("log.json")).unwrap();
        let entries = raw.lines().collect::<Vec<_>>();
        assert_eq!(entries.len(), 1);
        let event: serde_json::Value = serde_json::from_str(entries[0]).unwrap();
        assert_eq!(event["id"], pack_id.to_string());
        assert_eq!(event["url"], url);
        assert_eq!(event["error"], "download_failed");

        drop(runtime);
        std::fs::remove_dir_all(cache_dir).unwrap();
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
        assert_eq!(runtime.download_batches.len(), 2);
        assert_eq!(runtime.download_batches[0].members, vec![first]);
        assert_eq!(runtime.download_batches[1].members, vec![second]);
        assert_eq!(
            runtime.download_batches[0]
                .requests
                .iter()
                .map(|request| request.key)
                .collect::<Vec<_>>(),
            vec![first]
        );
        assert_eq!(runtime.download_batches[1].requests[0].key, second);
        assert!(!runtime.download_batches[0].started);
        assert!(!runtime.download_batches[1].started);
    }

    #[test]
    fn download_progress_preserves_batch_counts_and_terminal_failure() {
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
        let mut runtime = test_runtime("progress");
        runtime.reconcile_world(&world, true);
        assert_eq!(runtime.download_batches.len(), 2);
        assert_eq!(runtime.download_batches[0].members, vec![first]);
        assert_eq!(runtime.download_batches[1].members, vec![second]);
        let batch_id = runtime.download_batches[0].id;

        for progress in [
            ServerPackDownloadProgress::RequestStarted,
            ServerPackDownloadProgress::DownloadStarted {
                total_bytes: Some(1_024),
            },
            ServerPackDownloadProgress::DownloadedBytes { bytes: 512 },
            ServerPackDownloadProgress::RequestFinished { success: false },
        ] {
            runtime.apply_task_event(PackTaskEvent::DownloadProgress {
                batch_id,
                key: first,
                current_pack: 1,
                total_packs: 1,
                progress,
            });
        }
        runtime.publish_runtime_diagnostics(&mut world);

        let progress = world.resource_pack_download_state().unwrap();
        assert_eq!(progress.batch_sequence, batch_id);
        assert_eq!(progress.current_pack, 1);
        assert_eq!(progress.total_packs, 1);
        assert_eq!(progress.failed_packs, 1);
        assert_eq!(progress.phase, ResourcePackDownloadPhase::Failed);
        assert_eq!(
            progress.toast_action,
            bbb_world::ResourcePackDownloadToastAction::AddOrUpdateTitle
        );
    }

    #[test]
    fn download_progress_marks_only_vanilla_toast_mutation_callbacks() {
        let mut world = WorldStore::new();
        let key = push_accepted(
            &mut world,
            Uuid::from_u128(1),
            "https://example.test/first.zip",
        );
        let mut runtime = test_runtime("progress-toast-actions");
        runtime.reconcile_world(&world, true);
        let batch_id = runtime.download_batches[0].id;

        runtime.apply_download_progress(
            batch_id,
            key,
            1,
            2,
            ServerPackDownloadProgress::RequestStarted,
        );
        assert_eq!(
            runtime.download_presentations[0].toast_action,
            bbb_world::ResourcePackDownloadToastAction::AddOrUpdateTitlePreserveMessage
        );
        runtime.apply_download_progress(
            batch_id,
            key,
            1,
            2,
            ServerPackDownloadProgress::DownloadStarted {
                total_bytes: Some(1_024),
            },
        );
        assert_eq!(
            runtime.download_presentations[0].toast_action,
            bbb_world::ResourcePackDownloadToastAction::AddOrUpdateProgress
        );
        runtime.apply_download_progress(
            batch_id,
            key,
            1,
            2,
            ServerPackDownloadProgress::RequestFinished { success: true },
        );
        assert_eq!(
            runtime.download_presentations[0].toast_action,
            bbb_world::ResourcePackDownloadToastAction::None,
            "non-terminal requestFinished does not call updateToast"
        );
        runtime.apply_download_progress(
            batch_id,
            key,
            2,
            2,
            ServerPackDownloadProgress::RequestStarted,
        );
        let second_request = &runtime.download_presentations[0];
        assert_eq!(second_request.downloaded_bytes, 0);
        assert_eq!(second_request.total_bytes, Some(1_024));
        assert_eq!(
            second_request.toast_action,
            bbb_world::ResourcePackDownloadToastAction::AddOrUpdateTitlePreserveMessage
        );
    }

    #[test]
    fn download_progress_ignores_a_pack_from_another_sequential_batch() {
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
        let mut runtime = test_runtime("progress-partition");
        runtime.reconcile_world(&world, true);
        let first_batch = runtime.download_batches[0].id;
        let second_batch = runtime.download_batches[1].id;

        runtime.apply_task_event(PackTaskEvent::DownloadProgress {
            batch_id: first_batch,
            key: first,
            current_pack: 1,
            total_packs: 1,
            progress: ServerPackDownloadProgress::RequestStarted,
        });
        runtime.apply_task_event(PackTaskEvent::DownloadProgress {
            batch_id: first_batch,
            key: first,
            current_pack: 1,
            total_packs: 1,
            progress: ServerPackDownloadProgress::RequestFinished { success: false },
        });
        runtime.apply_task_event(PackTaskEvent::DownloadProgress {
            batch_id: first_batch,
            key: second,
            current_pack: 1,
            total_packs: 1,
            progress: ServerPackDownloadProgress::RequestStarted,
        });
        assert_eq!(runtime.download_presentations.len(), 1);
        assert_eq!(
            runtime.download_presentations[0].batch_sequence,
            first_batch
        );
        assert_eq!(
            runtime.download_presentations[0].phase,
            ResourcePackDownloadPhase::Failed
        );
        runtime.apply_task_event(PackTaskEvent::DownloadProgress {
            batch_id: second_batch,
            key: second,
            current_pack: 1,
            total_packs: 1,
            progress: ServerPackDownloadProgress::RequestStarted,
        });
        assert_eq!(
            runtime
                .download_presentations
                .last()
                .unwrap()
                .batch_sequence,
            second_batch
        );
        runtime.publish_runtime_diagnostics(&mut world);
        assert_eq!(world.resource_pack_download_states().len(), 2);
        assert_eq!(
            world.resource_pack_download_states()[0].batch_sequence,
            first_batch
        );
        assert_eq!(
            world.resource_pack_download_states()[1].batch_sequence,
            second_batch
        );
    }

    #[test]
    fn dropping_runtime_removes_materialized_session_directory() {
        let runtime = test_runtime("drop-cleanup");
        let session_dir = runtime.materialized_session_dir.clone();
        let cache_root = runtime.download_cache_dir.clone();
        std::fs::create_dir_all(&session_dir).unwrap();
        std::fs::write(session_dir.join("marker"), b"materialized").unwrap();

        drop(runtime);

        assert!(!session_dir.exists());
        if cache_root.exists() {
            std::fs::remove_dir_all(cache_root).unwrap();
        }
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
        assert_eq!(selected_roots(&reload), vec![ready_archive_path(&root)]);
        assert!(runtime
            .responses
            .iter()
            .all(|response| { response.action != ResourcePackResponseAction::SuccessfullyLoaded }));
        assert_eq!(
            runtime.finish_reload(reload.generation, Ok(())),
            Some(ResourcePackReloadFinish::Accepted)
        );
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
        assert_eq!(runtime.finish_reload(reload.generation + 1, Ok(())), None);
        assert!(!runtime.packs[0].active);
        assert!(runtime.take_reload().is_none());
        assert!(runtime
            .responses
            .iter()
            .all(|response| { response.action != ResourcePackResponseAction::SuccessfullyLoaded }));

        assert_eq!(
            runtime.finish_reload(reload.generation, Ok(())),
            Some(ResourcePackReloadFinish::Accepted)
        );
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
        assert_eq!(
            runtime.finish_reload(load.generation, Ok(())),
            Some(ResourcePackReloadFinish::Accepted)
        );
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
        assert_eq!(
            runtime.finish_reload(unload.generation, Ok(())),
            Some(ResourcePackReloadFinish::Accepted)
        );
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
        assert_eq!(
            runtime.finish_reload(load.generation, Ok(())),
            Some(ResourcePackReloadFinish::Accepted)
        );
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
        assert_eq!(
            runtime.finish_reload(unload.generation, Ok(())),
            Some(ResourcePackReloadFinish::Accepted)
        );
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
        assert_eq!(
            runtime.finish_reload(load.generation, Ok(())),
            Some(ResourcePackReloadFinish::Accepted)
        );
        runtime.responses.clear();

        world.apply_resource_pack_pop(ResourcePackPop { id: Some(id) });
        runtime.reconcile_world(&world, true);
        runtime.cleanup_removed_packs();
        assert!(root.exists());
        let unload = runtime.take_reload().unwrap();
        assert!(unload.server_packs.is_empty());

        assert_eq!(
            runtime.finish_reload(unload.generation, Ok(())),
            Some(ResourcePackReloadFinish::Accepted)
        );
        assert!(!root.exists());
        assert!(runtime.packs.is_empty());
    }

    #[test]
    fn recovery_reload_runs_once_and_second_failure_does_not_hot_loop() {
        let mut world = WorldStore::new();
        let id = Uuid::from_u128(1);
        let key = push_accepted(&mut world, id, "https://example.test/server-pack.zip");
        let mut runtime = test_runtime("bounded-recovery");
        runtime.reconcile_world(&world, true);
        let root = ready_root(&runtime, "active");
        runtime.apply_task_event(PackTaskEvent::Downloaded { key });
        runtime.apply_task_event(PackTaskEvent::Ready {
            key,
            materialization: ready_materialization(root.clone()),
        });
        runtime.resolve_download_batches();
        let load = runtime.take_reload().unwrap();
        assert_eq!(
            runtime.finish_reload(load.generation, Ok(())),
            Some(ResourcePackReloadFinish::Accepted)
        );
        runtime.responses.clear();

        world.apply_resource_pack_pop(ResourcePackPop { id: Some(id) });
        runtime.reconcile_world(&world, true);
        let unload = runtime.take_reload().unwrap();
        assert!(unload.server_packs.is_empty());
        assert!(!runtime.reload.as_ref().unwrap().is_recovery);
        assert_eq!(
            runtime.finish_reload(unload.generation, Err("first failure".to_string())),
            Some(ResourcePackReloadFinish::Accepted)
        );
        assert!(root.exists());

        let recovery = runtime.take_reload().expect("one recovery reload");
        assert!(recovery.server_packs.is_empty());
        assert!(runtime.reload.as_ref().unwrap().is_recovery);
        assert_eq!(
            runtime.finish_reload(recovery.generation, Err("recovery failure".to_string())),
            Some(ResourcePackReloadFinish::RecoveryFailed)
        );
        assert!(root.exists());
        assert!(runtime.take_reload().is_none());

        runtime.request_reload();
        let explicit_retry = runtime.take_reload().expect("explicit retry");
        assert!(!runtime.reload.as_ref().unwrap().is_recovery);
        assert_eq!(
            runtime.finish_reload(explicit_retry.generation, Ok(())),
            Some(ResourcePackReloadFinish::Accepted)
        );
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
        assert_eq!(
            runtime.finish_reload(load_old.generation, Ok(())),
            Some(ResourcePackReloadFinish::Accepted)
        );
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
            vec![ready_archive_path(&new_root), ready_archive_path(&old_root),]
        );
        assert_eq!(
            selected_ids(&reload),
            vec![
                format!("server/00000001/{new_id}"),
                format!("server/00000002/{}", old_key.id),
            ]
        );

        assert_eq!(
            runtime.finish_reload(reload.generation, Err("reload failed".to_string())),
            Some(ResourcePackReloadFinish::Accepted)
        );
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
        let recovery = runtime.take_reload().expect("fallback recovery reload");
        assert_eq!(
            selected_roots(&recovery),
            vec![ready_archive_path(&old_root)]
        );
        assert_eq!(
            selected_ids(&recovery),
            vec![format!("server/00000003/{}", old_key.id)]
        );
        assert!(runtime.reload.as_ref().unwrap().is_recovery);
        assert_eq!(
            runtime.finish_reload(recovery.generation, Ok(())),
            Some(ResourcePackReloadFinish::RecoverySucceeded)
        );
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
        assert_eq!(
            runtime.finish_reload(initial.generation, Ok(())),
            Some(ResourcePackReloadFinish::Accepted)
        );
        runtime.responses.clear();
        assert!(runtime.take_reload().is_none());

        runtime.request_reload();
        let forced = runtime.take_reload().unwrap();
        assert_eq!(selected_roots(&forced), vec![ready_archive_path(&root)]);
        assert_eq!(
            runtime.finish_reload(forced.generation, Ok(())),
            Some(ResourcePackReloadFinish::Accepted)
        );
        assert!(runtime.responses.is_empty());
        assert!(runtime.take_reload().is_none());
    }

    #[test]
    fn one_download_failure_discards_every_other_inactive_pack_across_queued_batches() {
        let mut world = WorldStore::new();
        let first = push_accepted(
            &mut world,
            Uuid::from_u128(1),
            "https://example.test/first.zip",
        );
        let mut runtime = test_runtime("batch-failure");
        runtime.reconcile_world(&world, true);
        assert_eq!(runtime.download_batches.len(), 1);

        let second = push_accepted(
            &mut world,
            Uuid::from_u128(2),
            "https://example.test/second.zip",
        );
        runtime.reconcile_world(&world, true);
        assert_eq!(runtime.download_batches.len(), 2);
        assert_ne!(
            runtime.download_batches[0].id,
            runtime.download_batches[1].id
        );

        runtime.apply_task_event(PackTaskEvent::Failed {
            key: first,
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
                (first, ResourcePackResponseAction::FailedDownload),
                (second, ResourcePackResponseAction::Discarded),
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
    fn one_preparation_failure_fails_every_new_pack_across_queued_batches() {
        let mut world = WorldStore::new();
        let first = push_accepted(
            &mut world,
            Uuid::from_u128(1),
            "https://example.test/first.zip",
        );
        let mut runtime = test_runtime("preparation-failure");
        runtime.reconcile_world(&world, true);

        let second = push_accepted(
            &mut world,
            Uuid::from_u128(2),
            "https://example.test/second.zip",
        );
        runtime.reconcile_world(&world, true);
        assert_eq!(runtime.download_batches.len(), 2);

        runtime.apply_task_event(PackTaskEvent::Downloaded { key: first });
        runtime.apply_task_event(PackTaskEvent::Failed {
            key: first,
            action: ResourcePackResponseAction::FailedReload,
            error: "invalid pack metadata".to_string(),
        });
        runtime.resolve_download_batches();
        runtime.resolve_preparation_failures();
        assert!(!runtime.packs[0].removed);
        assert!(!runtime.packs[1].removed);

        let second_root = ready_root(&runtime, "second");
        runtime.apply_task_event(PackTaskEvent::Downloaded { key: second });
        runtime.apply_task_event(PackTaskEvent::Ready {
            key: second,
            materialization: ready_materialization(second_root.clone()),
        });
        runtime.resolve_download_batches();
        runtime.resolve_preparation_failures();
        runtime.cleanup_removed_packs();

        assert!(!second_root.exists());
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

    fn ready_materialization(root: PathBuf) -> PreparedPackArchive {
        let archive_path = ready_archive_path(&root);
        let file = File::create(&archive_path).unwrap();
        let mut archive = ZipWriter::new(file);
        archive
            .start_file(
                "pack.mcmeta",
                SimpleFileOptions::default().compression_method(CompressionMethod::Stored),
            )
            .unwrap();
        archive
            .write_all(
                br#"{"pack":{"description":"test","min_format":[84,0],"max_format":[84,0]}}"#,
            )
            .unwrap();
        archive.finish().unwrap();
        prepare_pack_archive(&archive_path, &root).unwrap()
    }

    fn ready_archive_path(root: &Path) -> PathBuf {
        root.with_extension("zip")
    }

    fn selected_roots(reload: &ServerResourcePackReload) -> Vec<PathBuf> {
        reload
            .server_packs
            .iter()
            .map(|pack| pack.root.clone())
            .collect()
    }

    fn selected_ids(reload: &ServerResourcePackReload) -> Vec<String> {
        reload
            .server_packs
            .iter()
            .map(|pack| pack.id.clone())
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
