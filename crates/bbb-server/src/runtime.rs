use std::{
    collections::{hash_map::RandomState, BTreeMap, HashMap, VecDeque},
    future::Future,
    hash::BuildHasher,
    net::{IpAddr, SocketAddr},
    path::Path,
    sync::{Arc, Mutex, PoisonError},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use bbb_protocol::packets::GameProfile;
use bbb_server_admin::{
    check_bind, tls_acceptor, AdminGateway, AuthConfig, McpServer, McpServerConfig, OAuthIssuer,
    OAuthResource, OAuthScopeNames, RemoteAuthority, TlsMaterial, TokenGrant,
    DEFAULT_LEEWAY_SECONDS,
};
use bbb_server_core::admin::{
    AdminScope, AdminSettings, NameAndId, PersistedSettings, StoredUserListDocuments,
    StoredUserLists, ALLOWLIST_FILE, IP_BANS_FILE, OPERATORS_FILE, PERSISTED_SETTINGS_FILE,
    PLAYER_BANS_FILE,
};
use bbb_server_core::connection::ConnectionId;
use bbb_server_core::mailbox::{world_command_mailbox, WorldCommandSender, WorldMailboxLimits};
use bbb_server_core::world::{
    JoinContext, SaveTrigger, ServerWorld, ServerWorldConfig, WorldCommand,
};
use bbb_server_core::world_creation::created_preset_save;
use bbb_server_datapack::registry_data_loader::DatapackLoadLimits;
use bbb_server_datapack::startup::{DatapackStartup, DatapackStartupError, DatapackStartupOptions};
use bbb_server_net::{
    ConnectionCommand, ConnectionEvent, ConnectionHandle, ConnectionOutcome, LoginAdmission,
    ServerConnectionConfig, ServerConnectionDriver, ServerListener, ServerNetError,
    ServerResourcePack,
};
use bbb_server_persistence::atomic_file::write_atomic;
use bbb_server_persistence::Java26WorldBootstrap;
use bbb_server_persistence::{
    Java26CreatedSave, Java26WorldCreation, Java26WorldOptions, Java26WorldStorage,
    RegionCompression,
};
use bbb_server_types::save::{SaveError, WorldSaveLimits, WorldSavePaths};
use serde_json::json;
use thiserror::Error;
use tokio::{
    sync::{oneshot, watch, OwnedSemaphorePermit, Semaphore},
    task::{JoinError, JoinSet},
    time::timeout,
};

use crate::{management::RuntimeGateway, ServerOptions};

mod packet_encoder;
mod route_dispatch;
mod save_io;
mod world_thread;

#[cfg(test)]
use route_dispatch::{dispatch_world_events, PendingRouteEvent, RouteDispatch};
use route_dispatch::{
    AsyncRouteDispatcher, RouteDispatchEnv, Routes, OUTBOUND_ROUTE_DRAIN_TIMEOUT,
};
use save_io::SaveIo;
use world_thread::WorldThread;

#[derive(Debug, Error)]
pub enum ServerRuntimeError {
    #[error("invalid server configuration: {0}")]
    InvalidConfiguration(String),
    #[error(transparent)]
    Network(#[from] ServerNetError),
    #[error(transparent)]
    World(#[from] bbb_server_types::ServerWorldError),
    #[error(transparent)]
    Persistence(#[from] SaveError),
    #[error("datapack load failed: {0}")]
    Datapacks(#[from] DatapackStartupError),
    #[error("world command handoff failed: {0}")]
    WorldCommand(String),
    #[error("world thread failed: {0}")]
    WorldThread(String),
    /// A stored record this 26.1-only server will not read.
    #[error("{0}")]
    UnsupportedData(String),
    #[error("connection id space is exhausted")]
    ConnectionIdExhausted,
    #[error("connection task failed: {0}")]
    ConnectionTask(#[from] JoinError),
    #[error("connection tasks exceeded the {millis} ms shutdown budget")]
    ShutdownTimeout { millis: u128 },
    #[error("management endpoint refused to start: {0}")]
    Management(String),
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ServerReport {
    pub accepted_connections: u64,
    pub rejected_connections: u64,
    pub completed_connections: u64,
}

const DEFAULT_MAX_CONNECTIONS_PER_IP_WINDOW: usize = 60;
const DEFAULT_CONNECTION_RATE_WINDOW: Duration = Duration::from_secs(10);
const MAX_TRACKED_CONNECTION_IPS: usize = 4096;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum IpLimitRejection {
    RateLimit,
    ActiveLimit,
}

impl IpLimitRejection {
    fn as_str(self) -> &'static str {
        match self {
            Self::RateLimit => "per_ip_rate_limit",
            Self::ActiveLimit => "per_ip_connection_limit",
        }
    }
}

struct IpConnectionEntry {
    active: usize,
    attempts: VecDeque<Instant>,
    last_seen: Instant,
}

struct IpConnectionLimiter {
    max_active_per_ip: usize,
    max_attempts_per_window: usize,
    window: Duration,
    max_tracked_ips: usize,
    state: Mutex<HashMap<IpAddr, IpConnectionEntry>>,
}

struct IpConnectionPermit {
    limiter: Arc<IpConnectionLimiter>,
    ip: IpAddr,
}

impl Drop for IpConnectionPermit {
    fn drop(&mut self) {
        let mut state = self
            .limiter
            .state
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        if let Some(entry) = state.get_mut(&self.ip) {
            entry.active = entry.active.saturating_sub(1);
            if entry.active == 0 && entry.attempts.is_empty() {
                state.remove(&self.ip);
            }
        }
    }
}

impl IpConnectionLimiter {
    fn new(
        max_active_per_ip: usize,
        max_attempts_per_window: usize,
        window: Duration,
        max_tracked_ips: usize,
    ) -> Arc<Self> {
        Arc::new(Self {
            max_active_per_ip: max_active_per_ip.max(1),
            max_attempts_per_window: max_attempts_per_window.max(1),
            window,
            max_tracked_ips: max_tracked_ips.max(1),
            state: Mutex::new(HashMap::new()),
        })
    }

    fn try_acquire(
        self: &Arc<Self>,
        ip: IpAddr,
        now: Instant,
    ) -> Result<IpConnectionPermit, IpLimitRejection> {
        let mut state = self.state.lock().unwrap_or_else(PoisonError::into_inner);
        if !state.contains_key(&ip) && state.len() >= self.max_tracked_ips {
            state.retain(|_, entry| {
                while entry
                    .attempts
                    .front()
                    .is_some_and(|&ts| now.saturating_duration_since(ts) >= self.window)
                {
                    entry.attempts.pop_front();
                }
                entry.active > 0 || !entry.attempts.is_empty()
            });
            if state.len() >= self.max_tracked_ips {
                if let Some(oldest_ip) = state
                    .iter()
                    .filter(|(_, entry)| entry.active == 0)
                    .min_by_key(|(_, entry)| entry.last_seen)
                    .map(|(&addr, _)| addr)
                {
                    state.remove(&oldest_ip);
                }
            }
        }
        let entry = state.entry(ip).or_insert_with(|| IpConnectionEntry {
            active: 0,
            attempts: VecDeque::new(),
            last_seen: now,
        });
        entry.last_seen = now;
        while entry
            .attempts
            .front()
            .is_some_and(|&ts| now.saturating_duration_since(ts) >= self.window)
        {
            entry.attempts.pop_front();
        }
        if entry.attempts.len() >= self.max_attempts_per_window {
            return Err(IpLimitRejection::RateLimit);
        }
        if entry.active >= self.max_active_per_ip {
            return Err(IpLimitRejection::ActiveLimit);
        }
        entry.attempts.push_back(now);
        entry.active += 1;
        Ok(IpConnectionPermit {
            limiter: Arc::clone(self),
            ip,
        })
    }
}

pub struct ServerRuntime {
    listener: ServerListener,
    options: ServerOptions,
    connection_slots: Arc<Semaphore>,
    ip_limiter: Arc<IpConnectionLimiter>,
    management: Option<ManagementEndpoint>,
    startup: ConnectionStartup,
    #[cfg(test)]
    test_panic_hook: Option<Arc<TestPanicHook>>,
}

#[cfg(test)]
#[derive(Default)]
pub(crate) struct TestPanicHook {
    pub(crate) panic_on_tick: std::sync::atomic::AtomicBool,
    pub(crate) panic_on_emergency_save: std::sync::atomic::AtomicBool,
}

/// What every connection's login and configuration phases carry beyond the
/// status, fixed at startup.
#[derive(Debug, Clone)]
struct ConnectionStartup {
    core_registries: Arc<[bbb_protocol::packets::PublishedRegistry]>,
    bug_report_link: Option<String>,
    code_of_conduct: Arc<BTreeMap<String, String>>,
    resource_pack: Option<ServerResourcePack>,
    admission: Option<LoginAdmission>,
    plugins: Arc<bbb_server_core::plugin::ServerPluginRegistry>,
}

impl Default for ConnectionStartup {
    fn default() -> Self {
        Self {
            core_registries: Arc::from(Vec::new()),
            bug_report_link: None,
            code_of_conduct: Arc::default(),
            resource_pack: None,
            admission: None,
            plugins: Arc::clone(bbb_server_core::plugin::ServerPluginRegistry::vanilla()),
        }
    }
}

impl ConnectionStartup {
    fn from_options(options: &ServerOptions) -> Result<Self, ServerRuntimeError> {
        let plugins = crate::mods::build_server_plugins(options)
            .map_err(ServerRuntimeError::InvalidConfiguration)?;
        let core_registries = bbb_server_core::registry_sync::SynchronizedRegistries::load(None)
            .map_err(|error| ServerRuntimeError::InvalidConfiguration(error.to_string()))?
            .published_shared();
        let code_of_conduct = match &options.code_of_conduct {
            Some(directory) => {
                read_code_of_conduct(directory).map_err(ServerRuntimeError::InvalidConfiguration)?
            }
            None => BTreeMap::new(),
        };
        Ok(Self {
            core_registries,
            bug_report_link: options.bug_report_link.clone(),
            code_of_conduct: Arc::new(code_of_conduct),
            resource_pack: options
                .server_resource_pack()
                .map_err(ServerRuntimeError::InvalidConfiguration)?,
            admission: None,
            plugins,
        })
    }
}

/// `DedicatedServer.readCodeOfConducts` (`DedicatedServer.java:133-167`): every
/// `<language>.txt` directly in the directory, its lines joined by `\n` and
/// its formatting codes stripped (`StringUtil.stripColor`).
fn read_code_of_conduct(directory: &Path) -> Result<BTreeMap<String, String>, String> {
    let root = std::fs::canonicalize(directory)
        .ok()
        .filter(|root| root.is_dir())
        .ok_or_else(|| {
            format!(
                "Code of Conduct folder does not exist: {}",
                directory.display()
            )
        })?;
    let entries = std::fs::read_dir(&root)
        .map_err(|error| format!("Failed to read Code of Conduct folder: {error}"))?;
    let mut texts = BTreeMap::new();
    for entry in entries {
        let path = entry
            .map_err(|error| format!("Failed to read Code of Conduct folder: {error}"))?
            .path();
        let Some(language) = path
            .file_name()
            .and_then(|name| name.to_str())
            .and_then(|name| name.strip_suffix(".txt"))
        else {
            continue;
        };
        let file = path.display();
        if std::fs::canonicalize(&path)
            .ok()
            .and_then(|real| real.parent().map(|parent| parent == root))
            != Some(true)
        {
            return Err(format!(
                "Failed to read Code of Conduct file \"{file}\" because it links to a file outside the allowed directory"
            ));
        }
        let text = std::fs::read_to_string(&path)
            .map_err(|error| format!("Failed to read Code of Conduct file {file}: {error}"))?;
        let joined = text.lines().collect::<Vec<_>>().join("\n");
        texts.insert(language.to_lowercase(), strip_formatting_codes(&joined));
    }
    Ok(texts)
}

/// `StringUtil.STRIP_COLOR_PATTERN` (`StringUtil.java:10`).
fn strip_formatting_codes(text: &str) -> String {
    let mut stripped = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\u{a7}'
            && chars.peek().is_some_and(
                |next| matches!(next.to_ascii_lowercase(), '0'..='9' | 'a'..='f' | 'k'..='o' | 'r'),
            )
        {
            chars.next();
            continue;
        }
        stripped.push(c);
    }
    stripped
}

/// The management endpoint, bound but not yet serving.
///
/// It is bound in `bind` rather than when the tick loop starts so that a
/// management port that cannot be taken, or an address that is not protected
/// well enough to be taken, stops the process before the game listener has told
/// anyone it is ready.
struct ManagementEndpoint {
    listener: tokio::net::TcpListener,
    config: McpServerConfig,
    /// Present exactly when the operator declared TLS, and already built, so a
    /// chain the TLS stack refuses is a startup failure rather than an endpoint
    /// that quietly serves nothing.
    acceptor: Option<tokio_rustls::TlsAcceptor>,
}

impl ServerRuntime {
    pub async fn bind(options: ServerOptions) -> Result<Self, ServerRuntimeError> {
        options
            .validate()
            .map_err(ServerRuntimeError::InvalidConfiguration)?;
        let listener = ServerListener::bind(options.bind).await?;
        let management = match options.no_mcp {
            true => {
                tracing::info!(event = "management.disabled");
                None
            }
            false => {
                let (config, authority, tls_material) = management_config(&options)?;
                check_bind(options.mcp_bind, &authority, &config.auth)
                    .map_err(|refusal| ServerRuntimeError::Management(refusal.to_string()))?;
                let acceptor = match &tls_material {
                    None => None,
                    Some(material) => Some(
                        tls_acceptor(material)
                            .map_err(|error| ServerRuntimeError::Management(error.to_string()))?,
                    ),
                };
                let listener = tokio::net::TcpListener::bind(options.mcp_bind)
                    .await
                    .map_err(|error| {
                        ServerRuntimeError::Management(format!(
                            "cannot bind {}: {error}",
                            options.mcp_bind
                        ))
                    })?;
                Some(ManagementEndpoint {
                    listener,
                    config,
                    acceptor,
                })
            }
        };
        let startup = ConnectionStartup::from_options(&options)?;
        let runtime = Self {
            listener,
            connection_slots: Arc::new(Semaphore::new(options.max_connections)),
            ip_limiter: IpConnectionLimiter::new(
                options
                    .max_connections_per_ip
                    .min(options.max_connections.max(1)),
                DEFAULT_MAX_CONNECTIONS_PER_IP_WINDOW.max(options.max_connections_per_ip),
                DEFAULT_CONNECTION_RATE_WINDOW,
                MAX_TRACKED_CONNECTION_IPS,
            ),
            options,
            management,
            startup,
            #[cfg(test)]
            test_panic_hook: None,
        };
        runtime
            .connection_config(&StatusView::from_options(&runtime.options), None)
            .validate()?;
        Ok(runtime)
    }

    pub fn local_addr(&self) -> Result<SocketAddr, ServerRuntimeError> {
        Ok(self.listener.local_addr()?)
    }

    /// Where the management endpoint is listening, or `None` when it was
    /// switched off.
    pub fn management_addr(&self) -> Result<Option<SocketAddr>, ServerRuntimeError> {
        self.management
            .as_ref()
            .map(|endpoint| {
                endpoint
                    .listener
                    .local_addr()
                    .map_err(|error| ServerRuntimeError::Management(error.to_string()))
            })
            .transpose()
    }

    #[cfg(test)]
    pub(crate) fn set_test_panic_hook(&mut self, hook: Arc<TestPanicHook>) {
        self.test_panic_hook = Some(hook);
    }

    pub async fn run_until<F>(mut self, shutdown: F) -> Result<ServerReport, ServerRuntimeError>
    where
        F: Future<Output = ()>,
    {
        let mut report = ServerReport::default();
        let mut tasks = JoinSet::new();
        let routes = Arc::new(Routes::default());
        let mut next_connection_id = 1_u64;
        let (world_commands, world_receiver) =
            world_command_mailbox(WorldMailboxLimits::default())?;
        if let Some(directory) = &self.options.admin_dir {
            let persisted = load_persisted_settings(directory)?;
            self.options.apply_persisted(&persisted);
        }
        let world_bootstrap = match &self.options.world {
            Some(path) => Some(open_world_storage(
                path,
                &self.options,
                Arc::clone(self.startup.plugins.content_registry()),
            )?),
            None => None,
        };
        // `WorldStem.load` equivalent: the datapack pipeline runs before the
        // world is created; any registry failure aborts startup (`Main.java:185`).
        let datapacks = load_datapack_startup(
            &self.options,
            world_bootstrap
                .as_ref()
                .map(|(_, bootstrap)| bootstrap.level()),
            self.startup.plugins.content_registry(),
        )?;
        let persistence = world_bootstrap
            .map(|(storage, bootstrap)| {
                let candidate =
                    bbb_server_core::world_creation::load_candidate_from_bootstrap_with_content_registry(
                        &storage,
                        bootstrap,
                        &datapacks.registries,
                        datapacks.content_registry(),
                    )
                        .map_err(|error| {
                            tracing::error!(
                                event = "world.load_failed",
                                path = %storage.paths().root().display(),
                                %error,
                            );
                            ServerRuntimeError::Persistence(error)
                        })?;
                tracing::info!(
                    event = "world.loaded",
                    path = %storage.paths().root().display(),
                    game_time = candidate.level.game_time(),
                );
                Ok::<_, ServerRuntimeError>((storage, candidate))
            })
            .transpose()?;
        // `PlayerList` reads its four stored user lists from disk before the
        // first player can log in, so they are loaded before the world exists.
        let admin_lists = match &self.options.admin_dir {
            Some(directory) => load_admin_lists(directory)?,
            None => StoredUserLists::default(),
        };
        let options = &self.options;
        let world_config = ServerWorldConfig {
            max_players: options.max_players() as usize,
            chunk_radius: options.view_distance(),
            simulation_distance: options.simulation_distance(),
            difficulty: options.difficulty(),
            game_type: options.gamemode(),
            force_game_mode: options.force_gamemode,
            spawn_protection_radius: options.spawn_protection(),
            motd: options.motd().to_string(),
            use_allowlist: options.white_list.unwrap_or(false),
            enforce_allowlist: options.enforce_whitelist.unwrap_or(false),
            player_idle_timeout: options.player_idle_timeout.unwrap_or(0),
            hide_online_players: options.hide_online_players.unwrap_or(false),
            status_replies: options.enable_status.unwrap_or(true),
            accepts_transfers: options.accepts_transfers.unwrap_or(false),
            operator_user_permission_level: options.op_permission_level(),
            function_permission_level: options.function_permission_level(),
            allow_flight: options.allow_flight.unwrap_or(false),
            pause_when_empty_seconds: options.pause_when_empty_seconds.unwrap_or(60),
            entity_broadcast_range_percentage: options
                .entity_broadcast_range_percentage
                .unwrap_or(bbb_server_core::admin::DEFAULT_ENTITY_BROADCAST_RANGE_PERCENTAGE),
            resource_pack_required: options.resource_pack.is_some()
                && options.require_resource_pack,
            admin_lists,
            datapacks: Some(datapacks),
            generated_structure_directory: persistence
                .as_ref()
                .map(|(storage, _)| storage.paths().root().join("generated")),
            background_generation: true,
            plugins: Arc::clone(&self.startup.plugins),
            ..ServerWorldConfig::default()
        };
        let mut world = match &persistence {
            Some((_, candidate)) => {
                ServerWorld::restore(world_config, candidate).map_err(|error| {
                    tracing::error!(event = "world.restore_failed", %error);
                    ServerRuntimeError::Persistence(error)
                })?
            }
            None => ServerWorld::new(world_config)?,
        };
        if let Err(error) = world.prepare_default_spawn() {
            tracing::warn!(event = "world.spawn_preparation_failed", %error);
        }
        if let Some(message) = world.storage_refusal() {
            tracing::error!(event = "world.load_failed", %message);
            return Err(ServerRuntimeError::UnsupportedData(message));
        }
        let persistence = if persistence.is_some() || self.options.admin_dir.is_some() {
            Some(
                SaveIo::start(persistence, self.options.admin_dir.clone())
                    .map_err(|error| ServerRuntimeError::WorldThread(error.to_string()))?,
            )
        } else {
            None
        };
        let (shutdown_tx, shutdown_rx) = watch::channel(false);
        // One deadline for every management request: the caller's own timeout
        // expressed in the only clock the world has.
        let deadline_ticks = self.options.mcp_request_timeout().as_millis() / 50;
        let gateway = RuntimeGateway::new(
            world_commands.clone(),
            u64::try_from(deadline_ticks).unwrap_or(u64::MAX),
        );
        let connection_limits = self.options.connection_limits();
        let mut management = self
            .management
            .take()
            .map(|endpoint| serve_management(endpoint, &gateway, shutdown_rx.clone()));
        let published = StatusView::from_settings(world.admin_settings(), Vec::new());
        let (admission_tx, admission_rx) = watch::channel(Arc::new(world.admission_snapshot()));
        self.startup.admission = Some(LoginAdmission(Arc::new(
            move |profile: &GameProfile, address: Option<SocketAddr>| {
                let user = NameAndId {
                    id: profile.uuid,
                    name: (&profile.name).into(),
                };
                let address = address.map(|address| address.ip().to_string());
                admission_rx
                    .borrow()
                    .refusal(&user, address.as_deref(), epoch_millis())
            },
        )));
        let (status_tx, mut status) = watch::channel(
            self.connection_config(&published, Some(world.synchronized_registries())),
        );
        let (stop_world, world_stop) = std::sync::mpsc::sync_channel(1);
        let (world_done_tx, mut world_done) = oneshot::channel();
        let world_thread = WorldThread {
            world,
            receiver: world_receiver,
            deferred: Vec::new(),
            commands: world_commands.clone(),
            routes: Arc::clone(&routes),
            route_dispatcher: AsyncRouteDispatcher::default(),
            gateway: Arc::clone(&gateway),
            persistence,
            options: self.options.clone(),
            status: status_tx,
            published,
            startup: self.startup.clone(),
            admission: admission_tx,
            admission_key: None,
            stop: world_stop,
            network: tokio::runtime::Handle::current(),
            #[cfg(test)]
            test_panic_hook: self.test_panic_hook.clone(),
        };
        std::thread::Builder::new()
            .name("bbb-world".to_string())
            .stack_size(crate::SERVER_THREAD_STACK_BYTES)
            .spawn(move || {
                let outcome =
                    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(move || {
                        world_thread.run()
                    })) {
                        Ok(result) => result,
                        Err(payload) => {
                            let message = crate::panic_report::panic_message(payload.as_ref());
                            if !crate::panic_report::panic_report_installed() {
                                tracing::error!(
                                    event = "server.panicked",
                                    message,
                                    thread = std::thread::current().name(),
                                );
                            }
                            Err(ServerRuntimeError::WorldThread(format!(
                                "the world thread panicked: {message}"
                            )))
                        }
                    };
                let _ = world_done_tx.send(outcome);
            })
            .map_err(|error| ServerRuntimeError::WorldThread(error.to_string()))?;
        tokio::pin!(shutdown);
        // `DedicatedServer.initServer` prints its readiness line after
        // `loadLevel` returns (`DedicatedServer.java:282`, `:285`), not when the
        // socket is bound -- and this is the same place: the world above is
        // open, the datapacks are loaded, the stored user lists are read and
        // the management endpoint is serving. Announcing it at `bind` instead
        // named a process that had no world to serve and, because nothing had
        // yet polled the shutdown future, no handler to stop it with either.
        tracing::info!(
            event = "server.ready",
            address = %self.listener.local_addr()?,
            minecraft_version = bbb_protocol::MC_VERSION,
            protocol_version = bbb_protocol::PROTOCOL_VERSION,
        );

        // Vanilla accepts on `ServerConnectionListener`'s own event-loop
        // threads (ServerConnectionListener.java:51-91), which never wait for
        // `runServer`; the world thread above is that `runServer`.
        let mut world_result = None;
        let served = async {
            loop {
                // Read before the select: a borrowed publication held across
                // the accept would block the world thread's next one.
                let config = status.borrow_and_update().clone();
                tokio::select! {
                    biased;
                    () = &mut shutdown => return Ok::<(), ServerRuntimeError>(()),
                    result = &mut world_done => {
                        world_result = Some(result);
                        return Ok(());
                    }
                    // A newer status restarts the accept, which is cancel-safe,
                    // so a connection never starts from a superseded one.
                    Ok(()) = status.changed() => {}
                    joined = tasks.join_next(), if !tasks.is_empty() => {
                        record_completed(joined, &mut report, &routes)?;
                    }
                    accepted = self.listener.accept(config, connection_limits) => {
                        let (remote_address, handle, driver) = match accepted {
                            Ok(connection) => connection,
                            Err(error) => {
                                tracing::warn!(event = "listener.accept_failed", %error);
                                let transient = matches!(
                                    &error,
                                    bbb_server_net::ServerNetError::Io(io_error)
                                        if matches!(
                                            io_error.kind(),
                                            std::io::ErrorKind::ConnectionReset
                                                | std::io::ErrorKind::ConnectionAborted
                                                | std::io::ErrorKind::BrokenPipe
                                                | std::io::ErrorKind::Interrupted
                                                | std::io::ErrorKind::WouldBlock
                                        )
                                );
                                if !transient {
                                    tokio::select! {
                                        biased;
                                        () = &mut shutdown => return Ok(()),
                                        () = tokio::time::sleep(Duration::from_millis(50)) => {}
                                    }
                                }
                                continue;
                            }
                        };
                        let ip_permit = match self.ip_limiter.try_acquire(remote_address.ip(), Instant::now()) {
                            Ok(permit) => permit,
                            Err(rejection) => {
                                report.rejected_connections += 1;
                                tracing::warn!(
                                    event = "connection.rejected",
                                    %remote_address,
                                    reason = rejection.as_str(),
                                );
                                drop(handle);
                                drop(driver);
                                continue;
                            }
                        };
                        let Ok(permit) = Arc::clone(&self.connection_slots).try_acquire_owned() else {
                            report.rejected_connections += 1;
                            tracing::warn!(
                                event = "connection.rejected",
                                %remote_address,
                                reason = "connection_capacity",
                            );
                            drop(ip_permit);
                            drop(handle);
                            drop(driver);
                            continue;
                        };
                        let connection_id = ConnectionId(next_connection_id);
                        next_connection_id = next_connection_id
                            .checked_add(1)
                            .ok_or(ServerRuntimeError::ConnectionIdExhausted)?;
                        routes
                            .lock()
                            .unwrap_or_else(PoisonError::into_inner)
                            .insert(connection_id, handle.command_sender());
                        report.accepted_connections += 1;
                        tracing::info!(
                            event = "connection.accepted",
                            connection_id = connection_id.0,
                            %remote_address,
                        );
                        tasks.spawn(run_connection(
                            connection_id,
                            remote_address,
                            handle,
                            driver.with_live_config(status.clone()),
                            world_commands.clone(),
                            shutdown_rx.clone(),
                            ConnectionPermits {
                                _connection: permit,
                                _ip: ip_permit,
                            },
                        ));
                    }
                }
            }
        }
        .await;
        // The world thread runs the shutdown tick and the final save whether
        // the stop came from a signal, from `server/stop`, from a listener
        // failure, or from a world-thread panic cleanup; connections, the
        // listener, and the management endpoint are always torn down before
        // returning any error.
        let _ = stop_world.try_send(());
        let world_result = match world_result {
            Some(result) => result,
            None => world_done.await,
        };
        drop(self.listener);
        let _ = shutdown_tx.send(true);
        let shutdown_timeout = self.options.shutdown_timeout();
        let joined = timeout(shutdown_timeout, async {
            while let Some(completed) = tasks.join_next().await {
                record_completed(Some(completed), &mut report, &routes)?;
            }
            Ok::<(), ServerRuntimeError>(())
        })
        .await;
        let tasks_result = match joined {
            Ok(result) => result,
            Err(_) => {
                tasks.shutdown().await;
                if let Some(management) = management.take() {
                    management.abort();
                }
                Err(ServerRuntimeError::ShutdownTimeout {
                    millis: shutdown_timeout.as_millis(),
                })
            }
        };
        if let Some(management) = management {
            match timeout(shutdown_timeout, management).await {
                Ok(_) => tracing::info!(event = "management.stopped"),
                Err(_) => tracing::warn!(event = "management.stop_timeout"),
            }
        }
        served?;
        world_result
            .map_err(|_| ServerRuntimeError::WorldThread("the world thread panicked".into()))??;
        tasks_result?;
        Ok(report)
    }

    fn connection_config(
        &self,
        view: &StatusView,
        publication: Option<&bbb_server_core::registry_sync::SynchronizedRegistries>,
    ) -> Arc<ServerConnectionConfig> {
        connection_config(
            view,
            self.options.compression_threshold(),
            publication,
            &self.startup,
        )
    }
}

/// `MinecraftServer.buildServerStatus` over the *live* management settings:
/// motd, capacity, the player sample and whether a status is answered at all
/// are all writable through `serversettings/*`, so a startup snapshot would
/// report values the server no longer holds.
fn connection_config(
    view: &StatusView,
    compression_threshold: Option<i32>,
    publication: Option<&bbb_server_core::registry_sync::SynchronizedRegistries>,
    startup: &ConnectionStartup,
) -> Arc<ServerConnectionConfig> {
    let status = json!({
        "description": {"text": view.motd},
        "enforcesSecureChat": false,
        "players": {
            "max": view.max_players,
            "online": view.players.len(),
            "sample": view.sample(),
        },
        "version": {
            "name": bbb_protocol::MC_VERSION,
            "protocol": bbb_protocol::PROTOCOL_VERSION,
        },
    });
    let mut config = ServerConnectionConfig::new(status);
    config.status_replies = view.status_replies;
    config.accepts_transfers = view.accepts_transfers;
    config.compression_threshold = compression_threshold;
    config.known_packs = vec![bbb_server_net::vanilla_core_known_pack()];
    if let Some(publication) = publication {
        config.registries = publication.published_shared();
        if !startup.core_registries.is_empty() {
            config.core_registries = Some(Arc::clone(&startup.core_registries));
        }
        config.tags = publication.published_tags_shared();
    }
    config.bug_report_link = startup.bug_report_link.clone();
    config.code_of_conduct = Arc::clone(&startup.code_of_conduct);
    config.resource_pack = startup.resource_pack.clone();
    config.admission = startup.admission.clone();
    config.content_registry = Arc::clone(startup.plugins.content_registry());
    Arc::new(config)
}

/// Starts serving an already bound management endpoint.
fn serve_management(
    endpoint: ManagementEndpoint,
    gateway: &Arc<RuntimeGateway>,
    mut shutdown: watch::Receiver<bool>,
) -> tokio::task::JoinHandle<()> {
    let ManagementEndpoint {
        listener,
        config,
        acceptor,
    } = endpoint;
    let address = listener.local_addr().ok();
    let path = config.endpoint_path.clone();
    let server = McpServer::new(config, Arc::clone(gateway) as Arc<dyn AdminGateway>);
    let stop = async move {
        let _ = shutdown.changed().await;
    };
    match acceptor {
        None => {
            tracing::info!(event = "management.listening", ?address, path, tls = false);
            tokio::spawn(server.serve(listener, stop))
        }
        Some(acceptor) => {
            tracing::info!(event = "management.listening", ?address, path, tls = true);
            tokio::spawn(server.serve_tls(listener, acceptor, stop))
        }
    }
}

/// The status values one accept reads, refreshed from the world each tick.
#[derive(Debug, Clone, PartialEq, Eq)]
struct StatusView {
    motd: String,
    max_players: i32,
    hide_online_players: bool,
    status_replies: bool,
    accepts_transfers: bool,
    players: Vec<(String, String)>,
}

impl StatusView {
    /// `MinecraftServer.buildPlayerStatus` caps the sample at 12, chooses a
    /// random window offset, and shuffles the selected entries
    /// (MinecraftServer.java:1063-1081).
    const MAX_SAMPLE: usize = 12;

    /// The startup view, used only to prove the advertised status encodes
    /// before the listener says it is ready. Every later view comes from the
    /// world, which owns the live values.
    fn from_options(options: &ServerOptions) -> Self {
        Self {
            motd: options.motd().to_string(),
            max_players: options.max_players(),
            hide_online_players: false,
            status_replies: true,
            accepts_transfers: options.accepts_transfers.unwrap_or(false),
            players: Vec::new(),
        }
    }

    fn from_settings(settings: &AdminSettings, players: Vec<(String, String)>) -> Self {
        Self {
            motd: settings.motd.clone(),
            max_players: settings.max_players,
            hide_online_players: settings.hide_online_players,
            status_replies: settings.status_replies,
            accepts_transfers: settings.accepts_transfers,
            players,
        }
    }

    fn matches_settings(&self, settings: &AdminSettings) -> bool {
        self.motd == settings.motd
            && self.max_players == settings.max_players
            && self.hide_online_players == settings.hide_online_players
            && self.status_replies == settings.status_replies
            && self.accepts_transfers == settings.accepts_transfers
    }

    fn apply_settings(&mut self, settings: &AdminSettings) {
        if self.motd != settings.motd {
            self.motd.clone_from(&settings.motd);
        }
        self.max_players = settings.max_players;
        self.hide_online_players = settings.hide_online_players;
        self.status_replies = settings.status_replies;
        self.accepts_transfers = settings.accepts_transfers;
    }

    fn sample(&self) -> Vec<serde_json::Value> {
        let seed = RandomState::new().hash_one(self.players.len());
        self.sample_with_seed(seed)
    }

    fn sample_with_seed(&self, mut state: u64) -> Vec<serde_json::Value> {
        if self.hide_online_players || self.players.is_empty() {
            return Vec::new();
        }
        let mut next_u64 = || {
            state = state.wrapping_add(0x9e37_79b9_7f4a_7c15);
            let mut z = state;
            z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
            z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
            z ^ (z >> 31)
        };
        let sample_size = self.players.len().min(Self::MAX_SAMPLE);
        let max_offset = self.players.len() - sample_size;
        let offset = if max_offset == 0 {
            0
        } else {
            (next_u64() as usize) % (max_offset + 1)
        };
        let mut picked: Vec<&(String, String)> =
            self.players[offset..offset + sample_size].iter().collect();
        for i in (1..picked.len()).rev() {
            let j = (next_u64() as usize) % (i + 1);
            picked.swap(i, j);
        }
        picked
            .into_iter()
            .map(|(id, name)| json!({ "id": id, "name": name }))
            .collect()
    }
}

/// Reads the four stored user lists. A malformed document aborts startup: a
/// server that silently dropped an operator list would be running with an
/// authority nobody granted it.
fn load_admin_lists(directory: &Path) -> Result<StoredUserLists, ServerRuntimeError> {
    let read = |file: &str| -> Result<Vec<u8>, ServerRuntimeError> {
        match std::fs::read(directory.join(file)) {
            Ok(bytes) => Ok(bytes),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(Vec::new()),
            Err(error) => Err(ServerRuntimeError::InvalidConfiguration(format!(
                "cannot read {file}: {error}"
            ))),
        }
    };
    let documents = StoredUserListDocuments {
        allowlist: read(ALLOWLIST_FILE)?,
        operators: read(OPERATORS_FILE)?,
        player_bans: read(PLAYER_BANS_FILE)?,
        ip_bans: read(IP_BANS_FILE)?,
    };
    let lists = StoredUserLists::parse(&documents).map_err(|error| {
        tracing::error!(event = "admin.lists_failed", file = error.file, detail = %error.detail);
        ServerRuntimeError::InvalidConfiguration(error.to_string())
    })?;
    tracing::info!(
        event = "admin.lists_loaded",
        directory = %directory.display(),
        allowlist = lists.allowlist.len(),
        operators = lists.operators.len(),
        player_bans = lists.player_bans.len(),
        ip_bans = lists.ip_bans.len(),
    );
    Ok(lists)
}

/// Reads `server-settings.json`; a malformed one aborts startup like a list.
fn load_persisted_settings(directory: &Path) -> Result<PersistedSettings, ServerRuntimeError> {
    let bytes = match std::fs::read(directory.join(PERSISTED_SETTINGS_FILE)) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Default::default()),
        Err(error) => {
            return Err(ServerRuntimeError::InvalidConfiguration(format!(
                "cannot read {PERSISTED_SETTINGS_FILE}: {error}"
            )))
        }
    };
    PersistedSettings::parse(&bytes).map_err(|detail| {
        ServerRuntimeError::InvalidConfiguration(format!("{PERSISTED_SETTINGS_FILE}: {detail}"))
    })
}

/// Writes `server-settings.json` the way [`write_admin_lists`] writes a list.
fn write_persisted_settings(directory: &Path, settings: &PersistedSettings) {
    let target = directory.join(PERSISTED_SETTINGS_FILE);
    if let Err(error) = write_atomic(&target, None, &settings.to_json()) {
        tracing::error!(event = "admin.settings_write_failed", %error);
    }
}

/// Writes the four documents, each through a temporary file and a rename, so a
/// reader never sees a half-written list. A failure is logged and the server
/// keeps running: the lists in memory are still authoritative for this process.
fn write_admin_lists(directory: &Path, documents: &StoredUserListDocuments) {
    for (file, bytes) in documents.files() {
        let target = directory.join(file);
        if let Err(error) = write_atomic(&target, None, bytes) {
            tracing::error!(event = "admin.lists_write_failed", file, %error);
        }
    }
}

/// Builds the management endpoint's configuration from the command line, and
/// reads every file it names now rather than at first use.
fn management_config(
    options: &ServerOptions,
) -> Result<(McpServerConfig, RemoteAuthority, Option<TlsMaterial>), ServerRuntimeError> {
    let refuse = |detail: String| ServerRuntimeError::Management(detail);
    let mut tokens = Vec::new();
    if let Some(path) = &options.mcp_token_file {
        let document = std::fs::read_to_string(path)
            .map_err(|error| refuse(format!("cannot read {}: {error}", path.display())))?;
        for (index, line) in document.lines().enumerate() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let mut fields = line.splitn(3, ':');
            let (Some(principal), Some(scope), Some(secret)) =
                (fields.next(), fields.next(), fields.next())
            else {
                return Err(refuse(format!(
                    "{}:{} is not principal:scope:secret",
                    path.display(),
                    index + 1
                )));
            };
            let scope = AdminScope::by_name(scope).ok_or_else(|| {
                refuse(format!("{}:{} names no scope", path.display(), index + 1))
            })?;
            if secret.is_empty() {
                return Err(refuse(format!(
                    "{}:{} carries an empty secret",
                    path.display(),
                    index + 1
                )));
            }
            tokens.push(TokenGrant {
                principal: principal.to_string(),
                scope,
                secret: secret.to_string(),
            });
        }
        tracing::info!(event = "management.tokens_loaded", grants = tokens.len());
    }

    let oauth = match &options.mcp_oauth_resource {
        None => None,
        Some(resource) => {
            let mut issuers = Vec::new();
            for (issuer, jwks) in options
                .mcp_oauth_issuer
                .iter()
                .zip(options.mcp_oauth_jwks.iter())
            {
                let document = std::fs::read(jwks)
                    .map_err(|error| refuse(format!("cannot read {}: {error}", jwks.display())))?;
                let keys = bbb_server_admin::parse_jwks(&document)
                    .map_err(|error| refuse(format!("{}: {error}", jwks.display())))?;
                issuers.push(OAuthIssuer {
                    issuer: issuer.clone(),
                    keys,
                });
            }
            Some(OAuthResource {
                resource: resource.clone(),
                issuers,
                scopes: OAuthScopeNames::default(),
                leeway_seconds: DEFAULT_LEEWAY_SECONDS,
            })
        }
    };

    let tls_material = match (&options.mcp_tls_cert, &options.mcp_tls_key) {
        (Some(certificate), Some(key)) => Some(TlsMaterial {
            certificate_chain_pem: std::fs::read(certificate).map_err(|error| {
                refuse(format!("cannot read {}: {error}", certificate.display()))
            })?,
            private_key_pem: std::fs::read(key)
                .map_err(|error| refuse(format!("cannot read {}: {error}", key.display())))?,
        }),
        _ => None,
    };

    let authority = match (&tls_material, &oauth) {
        (Some(material), Some(resource)) => RemoteAuthority::Tls {
            material: material.clone(),
            oauth: resource.clone(),
        },
        _ => RemoteAuthority::LoopbackOnly,
    };

    let config = McpServerConfig {
        endpoint_path: options.mcp_path.clone(),
        server_version: env!("CARGO_PKG_VERSION").to_string(),
        request_timeout: options.mcp_request_timeout(),
        auth: AuthConfig {
            loopback_scope: options.mcp_loopback_scope.scope(),
            tokens,
            allowed_origins: options.mcp_allowed_origin.clone(),
            oauth,
        },
        ..McpServerConfig::default()
    };
    Ok((config, authority, tls_material))
}

/// Runs the datapack startup pipeline (`WorldStem.load` equivalent) before the
/// world exists. With `--world`, the selection comes from the save's
/// `Java26DataPacks` and `<world>/datapacks`; ephemeral mode loads the built-in
/// packs only. `--safe-mode` forces the vanilla-only selection
/// (`Main.java:153-158`); any failure aborts startup (`Main.java:185`).
/// What one datapack load reads, whether it is the startup load or a reload.
///
/// `ReloadCommand.discoverNewPacks` re-reads the repository and keeps the
/// saved selection's disabled set, which is exactly what these options express:
/// the same directory and the same saved selection, re-scanned.
fn datapack_startup_options(
    options: &ServerOptions,
    level: Option<&bbb_server_persistence::Java26LevelData>,
) -> DatapackStartupOptions {
    DatapackStartupOptions {
        datapacks_dir: options.world.as_ref().map(|path| path.join("datapacks")),
        saved_data_packs: level.and_then(|level| level.data_packs().cloned()),
        safe_mode: options.safe_mode,
        limits: DatapackLoadLimits::default(),
    }
}

fn load_datapack_startup(
    options: &ServerOptions,
    level: Option<&bbb_server_persistence::Java26LevelData>,
    content_registry: &bbb_protocol::content_registry::ContentRegistry,
) -> Result<std::sync::Arc<DatapackStartup>, ServerRuntimeError> {
    let startup_options = datapack_startup_options(options, level);
    let startup = DatapackStartup::load_with_content_registry(&startup_options, content_registry)
        .map_err(|error| {
        tracing::error!(event = "world.datapacks_failed", %error);
        ServerRuntimeError::Datapacks(error)
    })?;
    for warning in startup.warnings() {
        tracing::warn!(event = "world.datapack_warning", message = %warning);
    }
    tracing::info!(
        event = "world.datapacks_loaded",
        packs = ?startup.selection.ordered_ids,
        registries = startup.registries.registries.len(),
        recipes = startup.recipes.recipes.len(),
        advancements = startup.advancements.advancements.len(),
        functions = startup.functions.functions.len(),
        structures = startup.structures.templates.len(),
    );
    Ok(std::sync::Arc::new(startup))
}

fn open_world_storage(
    path: &Path,
    options: &ServerOptions,
    content_registry: Arc<bbb_protocol::content_registry::ContentRegistry>,
) -> Result<(Java26WorldStorage, Java26WorldBootstrap), ServerRuntimeError> {
    let mut created = false;
    let loaded = (|| {
        let paths = WorldSavePaths::new(path)?;
        let storage = Java26WorldStorage::open_with_content_registry(
            paths,
            WorldSaveLimits::default(),
            content_registry,
        )?;
        // A directory with no level is one this server makes a save in, the way
        // `Main` creates a world when `getDataTag` finds none. A level that is
        // there and unreadable is still an error: `load_candidate` below is
        // what says so, and nothing here replaces it.
        if !storage.holds_level_data() {
            storage.create_world(
                &created_world(path, options)?,
                RegionCompression::Zlib,
                region_timestamp(),
            )?;
            created = true;
        }
        let bootstrap = storage.load_bootstrap()?;
        Ok::<_, SaveError>((storage, bootstrap))
    })();
    match loaded {
        Ok(pair) => {
            if created {
                tracing::info!(event = "world.created", path = %path.display());
            }
            Ok(pair)
        }
        Err(error) => {
            tracing::error!(event = "world.load_failed", path = %path.display(), %error);
            Err(ServerRuntimeError::Persistence(error))
        }
    }
}

/// The save this server would publish into an empty directory, from the
/// `--level-type`, `--level-seed` and `--generate-structures` options.
///
/// The save carries no chunks and is uninitialized, so loading it finds the
/// spawn (`MinecraftServer.setInitialSpawn`) and generates the terrain around it.
pub(crate) fn created_world(
    path: &Path,
    options: &ServerOptions,
) -> Result<Java26CreatedSave, SaveError> {
    let request = Java26WorldCreation {
        level_name: path
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|| DEFAULT_CREATED_LEVEL_NAME.to_string()),
        // `Main.java:242-248` builds the created level's `LevelSettings` from
        // `properties.gameMode.get()`, the same property `DedicatedServer.java:237`
        // then re-applies over the loaded data -- so the document this writes and
        // the world's live default are one setting.
        game_type: options.gamemode().id(),
        difficulty: options.difficulty().serialized_name().to_string(),
        spawn_position: ServerWorld::fixture_spawn_block(),
        last_played: epoch_millis(),
    };
    let world_options = Java26WorldOptions {
        seed: options.parsed_level_seed().unwrap_or_else(random_seed),
        generate_structures: options.generate_structures,
        preset: options.level_type,
        hardcore: options.hardcore,
    };
    created_preset_save(&request, &world_options)
}

/// `WorldOptions.randomSeed` (`WorldOptions.java:87-89`).
fn random_seed() -> i64 {
    use std::hash::{BuildHasher, Hasher};
    let mut hasher = std::collections::hash_map::RandomState::new().build_hasher();
    hasher.write_i64(epoch_millis());
    hasher.finish() as i64
}

/// What a save is called when its directory has no name to take one from.
/// Vanilla's `level-name` default (`DedicatedServerProperties`).
const DEFAULT_CREATED_LEVEL_NAME: &str = "world";

/// The one wall-clock reading in the server: every value the world needs is
/// stamped here and travels in with the command that needs it.
pub(crate) fn region_timestamp() -> u32 {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    u32::try_from(now.as_secs()).unwrap_or(u32::MAX)
}

pub(crate) fn epoch_millis() -> i64 {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    i64::try_from(now.as_millis()).unwrap_or(i64::MAX)
}

/// What one tick asked the runtime to do beyond sending packets.
#[derive(Debug, Default)]
struct TickRequests {
    /// The tick that just ran, which is where the next deadline is measured
    /// from.
    tick: u64,
    /// What the tick drained from the world mailbox, which is the only queue
    /// depth there is to sample: the receiver hands over everything waiting.
    processed_commands: usize,
    /// `Some((flush, trigger))` when a save was asked for this tick, by the
    /// management surface or by the unattended interval.
    save: Option<(bool, SaveTrigger)>,
    /// True when `server/stop` ran this tick.
    stop: bool,
    /// `Some` when a reload ran this tick, carrying the pack ids
    /// `/datapack enable` or `disable` computed and `None` for `/reload`,
    /// which re-discovers instead (`ReloadCommand.java:26-38`).
    reload: Option<Option<Vec<String>>>,
}

struct ConnectionTaskResult {
    connection_id: ConnectionId,
    outcome: Result<ConnectionOutcome, ServerNetError>,
}

fn record_completed(
    joined: Option<Result<ConnectionTaskResult, JoinError>>,
    report: &mut ServerReport,
    routes: &Routes,
) -> Result<(), ServerRuntimeError> {
    let Some(joined) = joined else {
        return Ok(());
    };
    let completed = joined?;
    routes
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .remove(&completed.connection_id);
    report.completed_connections += 1;
    match completed.outcome {
        Ok(outcome) => tracing::info!(
            event = "connection.closed",
            connection_id = completed.connection_id.0,
            phase = ?outcome.final_phase,
            reason = ?outcome.reason,
        ),
        Err(error) => tracing::warn!(
            event = "connection.failed",
            connection_id = completed.connection_id.0,
            %error,
        ),
    }
    Ok(())
}

struct ConnectionPermits {
    _connection: OwnedSemaphorePermit,
    _ip: IpConnectionPermit,
}

async fn run_connection(
    connection_id: ConnectionId,
    remote_address: SocketAddr,
    handle: ConnectionHandle,
    driver: ServerConnectionDriver,
    world_commands: WorldCommandSender,
    mut shutdown: watch::Receiver<bool>,
    _permits: ConnectionPermits,
) -> ConnectionTaskResult {
    let forward = async {
        let mut in_play = false;
        while let Some(event) = handle.next_event().await {
            match event {
                ConnectionEvent::CustomClickAction(action) => {
                    tracing::debug!(
                        event = "connection.custom_click_action",
                        connection_id = connection_id.0,
                        %remote_address,
                        action_id = %action.id,
                        has_payload = action.payload.is_some(),
                    );
                }
                ConnectionEvent::PlayReady {
                    transferred,
                    profile,
                    client_information,
                    ..
                } => {
                    in_play = true;
                    tracing::info!(
                        event = "connection.play_ready",
                        connection_id = connection_id.0,
                        %remote_address,
                        player_name = %profile.name,
                        player_uuid = %profile.uuid,
                    );
                    if let Err(error) = world_commands
                        .send(WorldCommand::Join {
                            connection_id,
                            profile,
                            client_information,
                            // `PlayerList.canPlayerLogin` needs the address
                            // the ban list is keyed on and the instant an
                            // expiry is compared against; the world owns
                            // neither, so the runtime stamps both here.
                            context: JoinContext {
                                transferred,
                                remote_address: Some(remote_address.ip().to_string()),
                                now: epoch_millis(),
                            },
                        })
                        .await
                    {
                        tracing::warn!(
                            event = "world.command_rejected",
                            connection_id = connection_id.0,
                            command = "join",
                            %error,
                        );
                        request_connection_close(&handle);
                    }
                }
                ConnectionEvent::PhaseChanged { phase } => {
                    tracing::debug!(
                        event = "connection.phase",
                        connection_id = connection_id.0,
                        %remote_address,
                        ?phase,
                    );
                    if in_play && phase == bbb_server_net::ConnectionPhase::Configuration {
                        in_play = false;
                        if let Err(error) = world_commands
                            .send(WorldCommand::Disconnect { connection_id })
                            .await
                        {
                            tracing::warn!(
                                event = "world.command_rejected",
                                connection_id = connection_id.0,
                                command = "reconfigure_disconnect",
                                %error,
                            );
                            request_connection_close(&handle);
                        }
                    }
                }
                ConnectionEvent::PlayPacket { packet_id, payload } => {
                    if let Err(error) = world_commands
                        .send(WorldCommand::PlayPacket {
                            connection_id,
                            packet_id,
                            payload,
                        })
                        .await
                    {
                        tracing::warn!(
                            event = "world.command_rejected",
                            connection_id = connection_id.0,
                            packet_id,
                            command = "play",
                            %error,
                        );
                        request_connection_close(&handle);
                    }
                }
            }
        }
    };
    let drive = async {
        let driver = driver.run();
        tokio::pin!(driver);
        let mut shutdown_requested = false;
        loop {
            tokio::select! {
                biased;
                changed = shutdown.changed(), if !shutdown_requested => {
                    shutdown_requested = true;
                    if changed.is_err() || *shutdown.borrow_and_update() {
                        request_connection_close(&handle);
                        // Give healthy sockets a brief window to flush Close, then force-close
                        // the command channel so a stalled write_frame aborts immediately.
                        match timeout(Duration::from_millis(250), &mut driver).await {
                            Ok(outcome) => break outcome,
                            Err(_) => {
                                handle.close();
                                break driver.await;
                            }
                        }
                    }
                }
                outcome = &mut driver => break outcome,
            }
        }
    };
    // EOF or a route failure ends the driver, but accepted events can still
    // be waiting for world quota. Drain them before submitting Disconnect.
    let (outcome, ()) = tokio::join!(drive, forward);
    if let Err(error) = world_commands
        .send(WorldCommand::Disconnect { connection_id })
        .await
    {
        tracing::debug!(
            event = "world.disconnect_not_enqueued",
            connection_id = connection_id.0,
            %error,
        );
    }
    ConnectionTaskResult {
        connection_id,
        outcome,
    }
}

fn request_connection_close(handle: &ConnectionHandle) {
    if handle.try_send(ConnectionCommand::Close).is_err() {
        handle.close();
    }
}

#[cfg(test)]
mod tests;
