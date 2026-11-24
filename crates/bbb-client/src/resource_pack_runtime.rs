use bbb_client_clock::UNIX_EPOCH;
use std::{
    collections::{HashMap, HashSet, VecDeque},
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, AtomicU64, Ordering},
        mpsc as std_mpsc, Arc,
    },
};

use bbb_client_assets::{prepare_pack_archive, PreparedPackArchive, SelectedClientPack};
#[cfg(not(target_arch = "wasm32"))]
use bbb_client_net::initialize_server_pack_download_cache;
use bbb_client_net::{
    append_server_pack_download_log, download_server_pack_with_progress, NetCommand,
    ServerPackDownloadProgress, ServerPackDownloadProxy, ServerPackDownloadRequest,
};
use bbb_client_world::{
    ResourcePackDownloadPhase, ResourcePackDownloadState, ResourcePackFailureStage,
    ResourcePackRuntimeFailure, ResourcePackRuntimePhase, ResourcePackRuntimeState,
    ResourcePackState, WorldStore,
};
use bbb_protocol::packets::ResourcePackResponseAction;
#[cfg(not(target_arch = "wasm32"))]
use tokio::runtime::Handle;
use tokio::sync::mpsc;
use uuid::Uuid;

static SESSION_SERIAL: AtomicU64 = AtomicU64::new(0);

#[cfg(any(not(target_arch = "wasm32"), test))]
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
    #[cfg(any(target_arch = "wasm32", test))]
    local_pack_change: bool,
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
    #[cfg(any(target_arch = "wasm32", test))]
    pub(crate) local_pack_change: bool,
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
    #[cfg(not(target_arch = "wasm32"))]
    runtime: Option<Handle>,
    #[cfg(target_arch = "wasm32")]
    async_enabled: bool,
    download_cache_dir: PathBuf,
    materialized_session_dir: PathBuf,
    username: String,
    profile_id: Uuid,
    download_proxy: Option<ServerPackDownloadProxy>,
    task_tx: std_mpsc::Sender<PackTaskEvent>,
    task_rx: std_mpsc::Receiver<PackTaskEvent>,
    packs: Vec<TrackedPack>,
    pack_indices: bbb_client_world::FxHashMap<PackKey, usize>,
    responses: VecDeque<DeferredResponse>,
    last_reported_ready_roots: Vec<PathBuf>,
    pack_id_serial_number: u32,
    next_reload_generation: u64,
    reload: Option<ReloadTransaction>,
    force_reload_requested: bool,
    recovery_reload_requested: bool,
    #[cfg(any(target_arch = "wasm32", test))]
    local_pack_reload_requested: bool,
    blocked_failed_stack: Option<ReloadStack>,
    next_download_batch: u64,
    download_batches: Vec<DownloadBatch>,
    required_declines_disconnected: HashSet<PackKey>,
    session_alive: Arc<AtomicBool>,
    download_presentations: Vec<ResourcePackDownloadState>,
    next_download_update_sequence: u64,
}

fn required_pack_disconnect_reason(pack: &ResourcePackState) -> String {
    let mut message = String::from("This server requires its resource pack. ");
    if let Some(prompt) = &pack.prompt {
        message.push_str(prompt);
        message.push(' ');
    }
    #[cfg(not(target_arch = "wasm32"))]
    message.push_str("Reconnect and accept, or start with --server-resource-pack-policy accept.");
    #[cfg(target_arch = "wasm32")]
    message.push_str("Reconnect and accept, or set server_resource_pack_policy=accept in the URL.");
    message
}

impl ServerResourcePackRuntime {
    #[cfg(not(target_arch = "wasm32"))]
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

    #[cfg(target_arch = "wasm32")]
    pub(crate) fn new_web(
        username: impl Into<String>,
        profile_id: Uuid,
        download_proxy: Option<ServerPackDownloadProxy>,
    ) -> Self {
        Self::with_optional_runtime(
            true,
            PathBuf::from("/__bbb_mem__/server-packs"),
            username.into(),
            profile_id,
            download_proxy,
        )
    }

    fn with_optional_runtime(
        #[cfg(not(target_arch = "wasm32"))] runtime: Option<Handle>,
        #[cfg(target_arch = "wasm32")] async_enabled: bool,
        cache_dir: PathBuf,
        username: String,
        profile_id: Uuid,
        download_proxy: Option<ServerPackDownloadProxy>,
    ) -> Self {
        let session_serial = SESSION_SERIAL.fetch_add(1, Ordering::Relaxed);
        let session_nonce = bbb_client_clock::system_now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        #[cfg(not(target_arch = "wasm32"))]
        let materialized_session_dir = std::env::temp_dir()
            .join("bbb-client-server-pack-cache")
            .join(format!(
                "session-{}-{session_serial}-{session_nonce}",
                std::process::id()
            ));
        #[cfg(target_arch = "wasm32")]
        let materialized_session_dir = PathBuf::from(format!(
            "/__bbb_mem__/server-pack-cache/session-{session_serial}-{session_nonce}"
        ));
        let (task_tx, task_rx) = std_mpsc::channel();
        Self {
            #[cfg(not(target_arch = "wasm32"))]
            runtime,
            #[cfg(target_arch = "wasm32")]
            async_enabled,
            download_cache_dir: cache_dir,
            materialized_session_dir,
            username,
            profile_id,
            download_proxy,
            task_tx,
            task_rx,
            packs: Vec::new(),
            pack_indices: bbb_client_world::FxHashMap::default(),
            responses: VecDeque::new(),
            last_reported_ready_roots: Vec::new(),
            pack_id_serial_number: 0,
            next_reload_generation: 1,
            reload: None,
            force_reload_requested: false,
            recovery_reload_requested: false,
            #[cfg(any(target_arch = "wasm32", test))]
            local_pack_reload_requested: false,
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
        self.responses
            .extend(world.take_resource_pack_choices().into_iter().map(
                |(id, push_sequence, action)| DeferredResponse {
                    key: PackKey { id, push_sequence },
                    action,
                },
            ));
        // Flush player prompt choices (e.g. Accepted) before reconciling so
        // accepted packs begin downloading in the same step.
        self.flush_responses(world, commands);
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
        let forced = self.force_reload_requested;
        #[cfg(any(target_arch = "wasm32", test))]
        let local_pack_change_requested = self.local_pack_reload_requested;
        #[cfg(not(any(target_arch = "wasm32", test)))]
        let local_pack_change_requested = false;
        if !forced
            && !local_pack_change_requested
            && self
                .packs
                .iter()
                .filter(|pack| pack.active)
                .map(|pack| pack.key)
                .eq(self
                    .packs
                    .iter()
                    .filter(|pack| {
                        !pack.removed && matches!(pack.state, PackDownloadState::Ready(_))
                    })
                    .map(|pack| pack.key))
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
        let is_recovery = self.recovery_reload_requested;
        let blocked = self
            .blocked_failed_stack
            .as_ref()
            .is_some_and(|stack| stack.previous == previous && stack.desired == desired);
        let server_reload = forced || (stack_changed && !blocked);
        let local_pack_change = !server_reload && local_pack_change_requested;
        if !server_reload && !local_pack_change {
            return None;
        }
        let desired = if local_pack_change {
            previous.clone()
        } else {
            desired
        };
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
        let generation = self.next_reload_generation;
        self.next_reload_generation = self.next_reload_generation.saturating_add(1);
        if !local_pack_change {
            self.force_reload_requested = false;
            self.recovery_reload_requested = false;
            self.blocked_failed_stack = None;
            for pack in &mut self.packs {
                if desired.contains(&pack.key) {
                    pack.runtime_state = ResourcePackRuntimeState {
                        phase: ResourcePackRuntimePhase::Applying,
                        generation: Some(generation),
                        failure: None,
                    };
                }
            }
        }
        #[cfg(any(target_arch = "wasm32", test))]
        if local_pack_change {
            self.local_pack_reload_requested = false;
        }
        self.reload = Some(ReloadTransaction {
            generation,
            previous,
            desired,
            report_successful_loads: stack_changed,
            is_recovery,
            #[cfg(any(target_arch = "wasm32", test))]
            local_pack_change,
        });
        Some(ServerResourcePackReload {
            generation,
            server_packs,
            #[cfg(any(target_arch = "wasm32", test))]
            local_pack_change,
        })
    }

    pub(crate) fn request_reload(&mut self) {
        self.force_reload_requested = true;
    }

    #[cfg(any(target_arch = "wasm32", test))]
    pub(crate) fn request_local_pack_reload(&mut self) {
        self.local_pack_reload_requested = true;
    }

    pub(crate) fn finish_reload<T>(
        &mut self,
        generation: u64,
        apply: impl FnOnce() -> Result<T, String>,
    ) -> Option<(ResourcePackReloadFinish, Result<T, String>)> {
        let reload = self.reload.take()?;
        if reload.generation != generation {
            self.reload = Some(reload);
            return None;
        }
        let result = apply();
        #[cfg(any(target_arch = "wasm32", test))]
        if reload.local_pack_change {
            return Some((ResourcePackReloadFinish::Accepted, result));
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
            let error = result
                .as_ref()
                .err()
                .expect("failed reload result has an error");
            let mut removed_archives = Vec::new();
            let mut removed_roots = Vec::new();
            for pack in &mut self.packs {
                if reload.desired.contains(&pack.key) && !reload.previous.contains(&pack.key) {
                    if let PackDownloadState::Ready(materialization) = &pack.state {
                        removed_roots.push(materialization.cache_root.clone());
                        removed_archives.push(materialization.archive_path.clone());
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
            for root in removed_roots {
                self.remove_materialized_root_deferred(root);
            }
            self.cleanup_unreferenced_virtual_archives(&removed_archives);
            if reload.is_recovery {
                self.blocked_failed_stack = Some(ReloadStack {
                    previous: reload.previous.clone(),
                    desired: reload.desired.clone(),
                });
            } else {
                // Retry the previous stack once; a failed recovery waits for
                // an explicit reload or a different stack.
                self.force_reload_requested = true;
                self.recovery_reload_requested = true;
            }
        }
        self.cleanup_removed_packs();
        Some((finish, result))
    }

    fn reconcile_world(&mut self, world: &WorldStore, connected: bool) {
        let world_packs = world.resource_packs();
        if world_packs.is_empty() && self.packs.is_empty() {
            if !connected {
                self.download_presentations.clear();
            }
            return;
        }
        if connected {
            let any_removed = self.packs.iter().any(|tracked| {
                !tracked.removed
                    && !world_packs
                        .get(&tracked.key.id)
                        .is_some_and(|pack| pack.push_sequence == tracked.key.push_sequence)
            });
            let any_added = world_packs.values().any(|pack| {
                pack.last_response
                    .is_some_and(|response| response.action == ResourcePackResponseAction::Accepted)
                    && !self.pack_indices.contains_key(&PackKey::from_world(pack))
            });
            if !any_removed && !any_added {
                return;
            }
        }
        let mut canonical = world_packs.values().collect::<Vec<_>>();
        canonical.sort_by_key(|pack| pack.push_sequence);
        let canonical_keys = if connected {
            canonical
                .iter()
                .map(|pack| PackKey::from_world(pack))
                .collect::<HashSet<_>>()
        } else {
            Default::default()
        };

        for tracked in &mut self.packs {
            if !canonical_keys.contains(&tracked.key) {
                tracked.removed = true;
            }
        }
        self.prune_removed_unstarted_batches();

        if !connected {
            self.download_presentations.clear();
            return;
        }
        let additions = canonical
            .into_iter()
            .filter_map(|pack| {
                let key = PackKey::from_world(pack);
                let accepted = pack.last_response.is_some_and(|response| {
                    response.action == ResourcePackResponseAction::Accepted
                });
                if !accepted || self.pack_indices.contains_key(&key) {
                    return None;
                }
                Some((key, pack.url.clone(), pack.hash.clone()))
            })
            .collect::<Vec<_>>();
        if additions.is_empty() {
            return;
        }
        // Vanilla `ServerPackManager.updateDownloads` queues all accepted packs
        // discovered in one update pass as a single shared download batch.
        let download_batch = self.next_download_batch;
        self.next_download_batch = self.next_download_batch.saturating_add(1);
        let mut members = Vec::with_capacity(additions.len());
        let mut outcomes = HashMap::with_capacity(additions.len());
        let mut requests = Vec::with_capacity(additions.len());
        for (key, url, hash) in additions {
            members.push(key);
            outcomes.insert(key, DownloadOutcome::Pending);
            requests.push(PackDownloadRequest { key, url, hash });
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
        self.download_batches.push(DownloadBatch {
            id: download_batch,
            outcomes,
            members,
            requests,
            started: false,
        });
        self.packs.sort_by_key(|pack| pack.key.push_sequence);
        self.rebuild_pack_indices();
    }

    fn rebuild_pack_indices(&mut self) {
        self.pack_indices.clear();
        self.pack_indices.reserve(self.packs.len());
        for (index, pack) in self.packs.iter().enumerate() {
            self.pack_indices.insert(pack.key, index);
        }
    }

    fn prune_removed_unstarted_batches(&mut self) {
        self.download_batches.retain(|batch| {
            if batch.started {
                return true;
            }
            batch.members.iter().any(|member| {
                self.pack_indices
                    .get(member)
                    .and_then(|&index| self.packs.get(index))
                    .is_some_and(|pack| !pack.removed)
            })
        });
    }

    fn prune_stale_download_presentations(&mut self) {
        const MAX_RETAINED_COMPLETED_PRESENTATIONS: usize = 8;
        let active_batch_ids = self
            .download_batches
            .iter()
            .map(|batch| batch.id)
            .collect::<HashSet<_>>();
        let completed_count = self
            .download_presentations
            .iter()
            .filter(|state| !active_batch_ids.contains(&state.batch_sequence))
            .count();
        if completed_count <= MAX_RETAINED_COMPLETED_PRESENTATIONS {
            return;
        }
        let mut to_drop = completed_count - MAX_RETAINED_COMPLETED_PRESENTATIONS;
        self.download_presentations.retain(|state| {
            if active_batch_ids.contains(&state.batch_sequence) {
                return true;
            }
            if to_drop > 0 {
                to_drop -= 1;
                return false;
            }
            true
        });
    }

    fn start_next_download_batch(&mut self) {
        self.prune_removed_unstarted_batches();
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
        #[cfg(not(target_arch = "wasm32"))]
        let Some(runtime) = &self.runtime
        else {
            return;
        };
        #[cfg(target_arch = "wasm32")]
        if !self.async_enabled {
            return;
        }
        let download_cache_dir = self.download_cache_dir.clone();
        let materialized_session_dir = self.materialized_session_dir.clone();
        let username = self.username.clone();
        let profile_id = self.profile_id;
        let download_proxy = self.download_proxy.clone();
        let task_tx = self.task_tx.clone();
        let session_alive = Arc::clone(&self.session_alive);
        let batch_fut = async move {
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
                #[cfg(target_arch = "wasm32")]
                let request = {
                    let cached_bytes = expected_sha1
                        .as_deref()
                        .and_then(bbb_client_net::parse_expected_server_pack_sha1)
                        .and_then(|sha1| {
                            bbb_client_assets::get_virtual_file(&download_cache_dir.join(sha1))
                        })
                        .map(|existing| existing.len() as u64);
                    request.with_cached_bytes(cached_bytes)
                };
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
                #[cfg(target_arch = "wasm32")]
                let result = result.map(|mut download| {
                    if let Some(bytes) = download.archive_bytes.take() {
                        download.cache_hit =
                            bbb_client_assets::has_virtual_file(&download.archive_path);
                        bbb_client_assets::insert_virtual_file(&download.archive_path, bytes);
                    }
                    download
                });
                #[cfg(not(target_arch = "wasm32"))]
                {
                    let log_cache_dir = download_cache_dir.clone();
                    let log_pack_id = key.id;
                    let log_url = download_url.clone();
                    let log_hash = expected_sha1.clone();
                    let log_download = result.as_ref().ok().cloned();
                    match tokio::task::spawn_blocking(move || {
                        append_server_pack_download_log(
                            &log_cache_dir,
                            log_pack_id,
                            &log_url,
                            log_hash.as_deref(),
                            log_download.as_ref(),
                        )
                    })
                    .await
                    {
                        Ok(Ok(())) => {}
                        Ok(Err(error)) => {
                            tracing::warn!(%error, "server pack download event could not be logged");
                        }
                        Err(error) => {
                            tracing::warn!(%error, "server pack download log worker failed");
                        }
                    }
                }
                #[cfg(target_arch = "wasm32")]
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
                for (_, download) in downloaded {
                    if !download.cache_hit {
                        remove_virtual_archive(&download.archive_path);
                    }
                }
                if !session_alive.load(Ordering::Acquire) {
                    remove_virtual_archive(&download_cache_dir);
                }
                return;
            }

            let mut prepared = downloaded.into_iter().peekable();
            while let Some((key, download)) = prepared.next() {
                let output_dir = materialized_session_dir.join(format!(
                    "{:020}-{}-{}",
                    key.push_sequence, key.id, download.sha1
                ));
                let archive_path = download.archive_path.clone();
                let archive_was_cached = download.cache_hit;
                #[cfg(not(target_arch = "wasm32"))]
                let materialized = {
                    let task_session_dir = materialized_session_dir.clone();
                    tokio::task::spawn_blocking(move || {
                        std::fs::create_dir_all(&task_session_dir).map_err(anyhow::Error::from)?;
                        prepare_pack_archive(&download.archive_path, &output_dir)
                    })
                    .await
                    .map_err(|error| format!("join server pack preparation worker: {error}"))
                };
                #[cfg(target_arch = "wasm32")]
                let materialized: Result<
                    anyhow::Result<PreparedPackArchive>,
                    String,
                > = Ok(prepare_pack_archive(&download.archive_path, &output_dir));
                match materialized {
                    Ok(Ok(materialization)) => {
                        if !session_alive.load(Ordering::Acquire) {
                            remove_materialized_root_in_async(materialization.cache_root).await;
                            if !archive_was_cached {
                                remove_virtual_archive(&archive_path);
                            }
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
                                remove_materialized_root_in_async(materialization.cache_root).await;
                            }
                            if !archive_was_cached {
                                remove_virtual_archive(&archive_path);
                            }
                            break;
                        }
                    }
                    Ok(Err(error)) => {
                        if !archive_was_cached {
                            remove_virtual_archive(&archive_path);
                        }
                        let message = format!("{error:#}");
                        let _ = task_tx.send(PackTaskEvent::Failed {
                            key,
                            action: ResourcePackResponseAction::FailedReload,
                            error: message.clone(),
                        });
                        for (discarded_key, discarded_download) in prepared {
                            if !discarded_download.cache_hit {
                                remove_virtual_archive(&discarded_download.archive_path);
                            }
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
                    Err(message) => {
                        if !archive_was_cached {
                            remove_virtual_archive(&archive_path);
                        }
                        let _ = task_tx.send(PackTaskEvent::Failed {
                            key,
                            action: ResourcePackResponseAction::FailedReload,
                            error: message.clone(),
                        });
                        for (discarded_key, discarded_download) in prepared {
                            if !discarded_download.cache_hit {
                                remove_virtual_archive(&discarded_download.archive_path);
                            }
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
                remove_materialized_root_in_async(materialized_session_dir).await;
                remove_virtual_archive(&download_cache_dir);
            }
        };
        #[cfg(not(target_arch = "wasm32"))]
        runtime.spawn(batch_fut);
        #[cfg(target_arch = "wasm32")]
        wasm_bindgen_futures::spawn_local(batch_fut);
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

        let Some(pack_index) = self.pack_indices.get(&key).copied() else {
            if let PackTaskEvent::Ready {
                materialization, ..
            } = event
            {
                self.remove_materialized_root_deferred(materialization.cache_root);
                self.cleanup_unreferenced_virtual_archives(&[materialization.archive_path]);
            }
            return;
        };
        if self.packs[pack_index].removed {
            let mut removed_archive = None;
            let mut removed_root = None;
            let pack = &mut self.packs[pack_index];
            match event {
                PackTaskEvent::DownloadProgress { .. } => unreachable!(),
                PackTaskEvent::Downloaded { .. } => {}
                PackTaskEvent::Ready {
                    materialization, ..
                } => {
                    removed_root = Some(materialization.cache_root);
                    removed_archive = Some(materialization.archive_path);
                    pack.state = PackDownloadState::Failed;
                }
                PackTaskEvent::Failed { .. } => pack.state = PackDownloadState::Failed,
            }
            if let Some(root) = removed_root {
                self.remove_materialized_root_deferred(root);
            }
            if let Some(archive_path) = removed_archive {
                self.cleanup_unreferenced_virtual_archives(&[archive_path]);
            }
            return;
        }
        let pack = &mut self.packs[pack_index];

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
            toast_action: bbb_client_world::ResourcePackDownloadToastAction::None,
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
                    bbb_client_world::ResourcePackDownloadToastAction::AddOrUpdateTitlePreserveMessage;
            }
            ServerPackDownloadProgress::DownloadStarted { total_bytes } => {
                state.downloaded_bytes = 0;
                state.total_bytes = total_bytes;
                state.toast_action =
                    bbb_client_world::ResourcePackDownloadToastAction::AddOrUpdateProgress;
            }
            ServerPackDownloadProgress::DownloadedBytes { bytes } => {
                state.downloaded_bytes = bytes;
                state.toast_action =
                    bbb_client_world::ResourcePackDownloadToastAction::AddOrUpdateProgress;
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
                        bbb_client_world::ResourcePackDownloadToastAction::ForceHide
                    } else {
                        bbb_client_world::ResourcePackDownloadToastAction::AddOrUpdateTitle
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
                    let Some(pack) = self
                        .pack_indices
                        .get(&key)
                        .and_then(|&index| self.packs.get_mut(index))
                    else {
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
                let mut removed_archives = Vec::new();
                let mut removed_roots = Vec::new();
                for pack in &mut self.packs {
                    if pack.removed || pack.active {
                        continue;
                    }
                    let key = pack.key;
                    if let PackDownloadState::Ready(materialization) = &pack.state {
                        removed_roots.push(materialization.cache_root.clone());
                        removed_archives.push(materialization.archive_path.clone());
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
                for root in removed_roots {
                    self.remove_materialized_root_deferred(root);
                }
                self.cleanup_unreferenced_virtual_archives(&removed_archives);
            }
        }
        self.download_batches
            .retain(|batch| !resolved_ids.contains(&batch.id));
        self.prune_removed_unstarted_batches();
        self.prune_stale_download_presentations();
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
        let mut removed_archives = Vec::new();
        let mut removed_roots = Vec::new();
        for pack in &mut self.packs {
            if pack.removed || pack.active {
                continue;
            }
            let key = pack.key;
            if let PackDownloadState::Ready(materialization) = &pack.state {
                removed_roots.push(materialization.cache_root.clone());
                removed_archives.push(materialization.archive_path.clone());
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
        for root in removed_roots {
            self.remove_materialized_root_deferred(root);
        }
        self.cleanup_unreferenced_virtual_archives(&removed_archives);
    }

    fn cleanup_removed_packs(&mut self) {
        if !self.packs.iter().any(|pack| pack.removed && !pack.active) {
            return;
        }
        let pending_responses = self
            .responses
            .iter()
            .map(|response| response.key)
            .collect::<HashSet<_>>();
        let mut removed_archives = Vec::new();
        let mut removed_roots = Vec::new();
        let previous_len = self.packs.len();
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
                    removed_roots.push(materialization.cache_root.clone());
                    removed_archives.push(materialization.archive_path.clone());
                    false
                }
                PackDownloadState::Failed => false,
            }
        });
        if self.packs.len() != previous_len {
            self.rebuild_pack_indices();
        }
        for root in removed_roots {
            self.remove_materialized_root_deferred(root);
        }
        self.cleanup_unreferenced_virtual_archives(&removed_archives);
    }

    fn cleanup_unreferenced_virtual_archives(&self, candidates: &[PathBuf]) {
        for archive_path in candidates {
            if !bbb_client_assets::virtual_store::is_virtual_path(archive_path) {
                continue;
            }
            let still_referenced = self.packs.iter().any(|pack| {
                matches!(
                    &pack.state,
                    PackDownloadState::Ready(materialization)
                        if materialization.archive_path == *archive_path
                )
            });
            if !still_referenced {
                remove_virtual_archive(archive_path);
            }
        }
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
        let reason = required_pack_disconnect_reason(world.resource_pack(key.id).unwrap());
        match commands.try_send(NetCommand::DisconnectWithReason(reason)) {
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
        let unchanged = self
            .packs
            .iter()
            .filter(|pack| !pack.removed)
            .filter_map(|pack| match &pack.state {
                PackDownloadState::Ready(materialization) => Some(&materialization.cache_root),
                PackDownloadState::Pending | PackDownloadState::Failed => None,
            })
            .eq(self.last_reported_ready_roots.iter());
        if unchanged {
            return;
        }
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
        tracing::info!(
            ready_server_packs = ready_roots.len(),
            "server resource pack preparation stack changed"
        );
        self.last_reported_ready_roots = ready_roots;
    }

    fn remove_materialized_root_deferred(&self, root: PathBuf) {
        if bbb_client_assets::virtual_store::is_virtual_path(&root) {
            bbb_client_assets::virtual_store::remove_virtual_prefix(&root);
            return;
        }
        #[cfg(not(target_arch = "wasm32"))]
        if let Some(runtime) = &self.runtime {
            runtime.spawn_blocking(move || remove_materialized_root(&root));
            return;
        }
        remove_materialized_root(&root);
    }
}

async fn remove_materialized_root_in_async(root: PathBuf) {
    if bbb_client_assets::virtual_store::is_virtual_path(&root) {
        bbb_client_assets::virtual_store::remove_virtual_prefix(&root);
        return;
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = tokio::task::spawn_blocking(move || remove_materialized_root(&root)).await;
    }
}

impl Drop for ServerResourcePackRuntime {
    fn drop(&mut self) {
        self.session_alive.store(false, Ordering::Release);
        remove_materialized_root(&self.materialized_session_dir);
        remove_virtual_archive(&self.download_cache_dir);
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

fn remove_virtual_archive(path: &Path) {
    if bbb_client_assets::virtual_store::is_virtual_path(path) {
        bbb_client_assets::virtual_store::remove_virtual_prefix(path);
    }
}

fn remove_materialized_root(root: &Path) {
    if bbb_client_assets::virtual_store::is_virtual_path(root) {
        bbb_client_assets::virtual_store::remove_virtual_prefix(root);
        return;
    }
    #[cfg(not(target_arch = "wasm32"))]
    if let Err(error) = std::fs::remove_dir_all(root) {
        if error.kind() != std::io::ErrorKind::NotFound {
            tracing::warn!(?error, path = %root.display(), "failed to remove server pack entry cache");
        }
    }
}

#[cfg(test)]
mod tests;
