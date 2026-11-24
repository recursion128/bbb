//! The server thread: the one owner of `ServerWorld`, ticking on its own OS
//! thread the way `MinecraftServer.runServer` does, so no tick waits for the
//! network and no accept waits for a tick.
//!
//! Everything reaches the world through its bounded mailbox; what leaves it is
//! handed to each connection's bounded command queue on the tokio runtime,
//! and the status the listener answers with is published after every tick
//! that changed it.

use std::{
    any::Any,
    panic::{catch_unwind, AssertUnwindSafe},
    sync::{
        mpsc::{Receiver, RecvTimeoutError},
        Arc,
    },
    time::{Duration, Instant},
};

use bbb_server_core::admin::{AdmissionSnapshot, PersistedSettings};
use bbb_server_core::mailbox::{CommandEnvelope, WorldCommandReceiver, WorldCommandSender};
use bbb_server_core::world::{ReloadedDatapacks, SaveTrigger, ServerWorld, WorldCommand};
use bbb_server_datapack::startup::DatapackStartup;
use bbb_server_net::ServerConnectionConfig;
use tokio::{runtime::Handle, sync::watch};
use uuid::Uuid;

use super::{
    connection_config, datapack_startup_options, save_io::SaveIo, write_admin_lists,
    write_persisted_settings, AsyncRouteDispatcher, ConnectionStartup, Routes, ServerRuntimeError,
    StatusView, TickRequests, OUTBOUND_ROUTE_DRAIN_TIMEOUT,
};
use crate::{management::RuntimeGateway, panic_report, ServerOptions};

pub(super) struct WorldThread {
    pub(super) world: ServerWorld,
    pub(super) receiver: WorldCommandReceiver,
    /// Joins held for a later tick, retaining their mailbox budget.
    pub(super) deferred: Vec<CommandEnvelope>,
    pub(super) commands: WorldCommandSender,
    pub(super) routes: Arc<Routes>,
    pub(super) route_dispatcher: AsyncRouteDispatcher,
    pub(super) gateway: Arc<RuntimeGateway>,
    pub(super) persistence: Option<SaveIo>,
    pub(super) options: ServerOptions,
    pub(super) status: watch::Sender<Arc<ServerConnectionConfig>>,
    /// What `status` was last built from.
    pub(super) published: StatusView,
    pub(super) startup: ConnectionStartup,
    /// The login check the connections read, republished when what it reads
    /// changes: the lists, the allowlist switch, the capacity or the count.
    pub(super) admission: watch::Sender<Arc<AdmissionSnapshot>>,
    pub(super) admission_key: Option<(u64, bool, i32, usize)>,
    /// The runtime's stop: a value or a dropped sender both end the loop.
    pub(super) stop: Receiver<()>,
    pub(super) network: Handle,
    #[cfg(test)]
    pub(super) test_panic_hook: Option<Arc<super::TestPanicHook>>,
}

struct TickLoopState {
    loop_epoch: Instant,
    next_tick_time_nanos: i64,
    last_overload_warning_nanos: i64,
    tick_health: TickHealth,
    list_revision: u64,
    settings_revision: (u64, i32),
    persisted: PersistedSettings,
    settings_changed: bool,
    last_sample_end: Instant,
    tick_samples: bool,
}

enum TickOutcome {
    Continue,
    Stop(Option<String>),
}

impl WorldThread {
    pub(super) fn run(mut self) -> Result<(), ServerRuntimeError> {
        // `MinecraftServer.runServer` (MinecraftServer.java:722-752) does not
        // pace on a fixed period: each iteration takes the current
        // `nanosecondsPerTick` from the tick-rate manager, so `/tick rate`
        // moves the wall clock, and a running sprint collapses the wait to
        // zero. `nextTickTimeNanos` is measured from the moment the loop
        // starts, which is the same origin `Util.getNanos` gives vanilla.
        let loop_epoch = Instant::now();
        let mut state = TickLoopState {
            loop_epoch,
            next_tick_time_nanos: 0,
            last_overload_warning_nanos: 0,
            tick_health: TickHealth::default(),
            list_revision: self.world.admin_list_revision(),
            settings_revision: self.world.admin_settings_revision(),
            persisted: self.world.persisted_settings(),
            settings_changed: false,
            last_sample_end: loop_epoch,
            tick_samples: std::env::var("BBB_SERVER_TICK_SAMPLES").as_deref() == Ok("1"),
        };
        let mut refusal = None;
        loop {
            let scheduled = loop_epoch + nanos_duration(state.next_tick_time_nanos);
            let wait_started = Instant::now();
            match self
                .stop
                .recv_timeout(scheduled.saturating_duration_since(Instant::now()))
            {
                Err(RecvTimeoutError::Timeout) => {}
                Ok(()) | Err(RecvTimeoutError::Disconnected) => break,
            }
            match catch_unwind(AssertUnwindSafe(|| {
                self.step_tick(&mut state, scheduled, wait_started)
            })) {
                Ok(Ok(TickOutcome::Continue)) => {}
                Ok(Ok(TickOutcome::Stop(reason))) => {
                    refusal = reason;
                    break;
                }
                Ok(Err(error)) => return Err(error),
                Err(payload) => {
                    return Err(self.handle_tick_panic(
                        payload,
                        &mut state.persisted,
                        &mut state.settings_changed,
                    ));
                }
            }
        }

        if let Err(error) = self
            .commands
            .try_send(WorldCommand::Shutdown)
            .map_err(|error| ServerRuntimeError::WorldCommand(error.to_string()))
        {
            self.emergency_cleanup(&mut state.persisted, &mut state.settings_changed);
            return Err(error);
        }
        match catch_unwind(AssertUnwindSafe(|| self.tick())) {
            Ok(Ok(_)) => {}
            Ok(Err(error)) => {
                self.emergency_cleanup(&mut state.persisted, &mut state.settings_changed);
                return Err(error);
            }
            Err(payload) => {
                return Err(self.handle_tick_panic(
                    payload,
                    &mut state.persisted,
                    &mut state.settings_changed,
                ));
            }
        }
        let final_settings = self.world.persisted_settings();
        if final_settings != state.persisted {
            state.persisted = final_settings;
            state.settings_changed = true;
        }
        // Flush the final shutdown save and admin files before waiting on slow
        // client sockets so a stalled connection cannot block or delay persistence.
        if let Some(mut io) = self.persistence.take() {
            if io.has_world_storage() {
                io.save(&mut self.world, SaveTrigger::Shutdown, true);
            }
            io.close(&mut self.world);
        }
        if let Some(directory) = &self.options.admin_dir {
            write_admin_lists(directory, &self.world.admin_documents());
            if state.settings_changed {
                write_persisted_settings(directory, &state.persisted);
            }
        }
        self.route_dispatcher.finish_shutdown(
            &self.routes,
            &self.network,
            self.options.shutdown_timeout(),
        );
        // A caller still waiting is told the answer is not coming rather than
        // being left to time out against a server that has stopped ticking.
        self.gateway.abandon_pending();
        self.receiver.close();
        // The shutdown save can read a record of its own, and that one is
        // refused just the same.
        let refusal = refusal.or_else(|| {
            let message = self.world.storage_refusal()?;
            tracing::error!(event = "world.stopping_on_unsupported_data", %message);
            Some(message)
        });
        match refusal {
            Some(message) => Err(ServerRuntimeError::UnsupportedData(message)),
            None => Ok(()),
        }
    }

    fn step_tick(
        &mut self,
        state: &mut TickLoopState,
        scheduled: Instant,
        wait_started: Instant,
    ) -> Result<TickOutcome, ServerRuntimeError> {
        let started = Instant::now();
        let started_nanos = elapsed_nanos(state.loop_epoch, started);
        // `MinecraftServer.java:728-733`: a sprinting server does not wait
        // at all, and the deadline restarts from now so the ticks it
        // skipped are not owed back afterwards.
        let sprinting = self.world.check_should_sprint_this_tick(started_nanos);
        let this_tick_nanos = if sprinting {
            state.next_tick_time_nanos = started_nanos;
            state.last_overload_warning_nanos = state.next_tick_time_nanos;
            0
        } else {
            let this_tick_nanos = self.world.tick_rate().nanoseconds_per_tick();
            // `:734-743`: falling behind is caught up by running ticks
            // back to back, and only a run that is both far behind and has
            // not warned recently gives up the backlog it accumulated.
            let behind_time_nanos = started_nanos - state.next_tick_time_nanos;
            if behind_time_nanos > OVERLOADED_THRESHOLD_NANOS + 20 * this_tick_nanos
                && state.next_tick_time_nanos - state.last_overload_warning_nanos
                    >= OVERLOADED_WARNING_INTERVAL_NANOS + 100 * this_tick_nanos
            {
                let ticks = behind_time_nanos / this_tick_nanos;
                tracing::warn!(
                    event = "world.overloaded",
                    behind_ms = behind_time_nanos / NANOSECONDS_PER_MILLISECOND,
                    behind_ticks = ticks,
                );
                state.next_tick_time_nanos += ticks * this_tick_nanos;
                state.last_overload_warning_nanos = state.next_tick_time_nanos;
            }
            this_tick_nanos
        };
        state.next_tick_time_nanos += this_tick_nanos;
        if let Some(io) = &mut self.persistence {
            io.complete_reads(&mut self.world);
        }
        let requests = self.tick()?;
        if let Some(io) = &mut self.persistence {
            io.pump_reads(&mut self.world);
            if io.has_world_storage() {
                io.pump_autosave(&mut self.world);
                // An unload save waits for the next tick while an autosave
                // snapshot or earlier detached save is still active.
                if !self.world.has_pending_autosave_snapshot()
                    && io.idle(&mut self.world)
                    && self.world.needs_runtime_unload_save()
                    && (started.elapsed() < UNLOAD_TICK_BUDGET
                        || self.world.runtime_unload_backlog() > UNLOAD_BACKLOG)
                {
                    io.save_unload_batch(&mut self.world, UNLOAD_BATCH);
                }
            }
        }
        // `MinecraftServer.java:988-994`: the world owns the ring `/tick
        // query` reports from, and this loop owns the clock that fills it.
        let tick_time = started.elapsed();
        let tick_time_nanos = i64::try_from(tick_time.as_nanos()).unwrap_or(i64::MAX);
        self.world.record_tick_time(tick_time_nanos);
        // `TpsDebugDimensions`: full, tick method, scheduled tasks, idle.
        // Every task of this loop runs inside the tick.
        let sample_end = Instant::now();
        let nanos = |duration: Duration| i64::try_from(duration.as_nanos()).unwrap_or(i64::MAX);
        self.world.log_tick_time_sample([
            nanos(sample_end.saturating_duration_since(state.last_sample_end)),
            tick_time_nanos,
            0,
            nanos(started.saturating_duration_since(wait_started)),
        ]);
        state.last_sample_end = sample_end;
        // `:761-765`: the sprint's own accounting closes after the wait,
        // which for a sprinting tick is not a wait at all.
        if sprinting {
            self.world
                .end_tick_work(elapsed_nanos(state.loop_epoch, Instant::now()));
        }
        state.tick_health.observe(
            started.saturating_duration_since(scheduled),
            tick_time,
            requests.processed_commands,
        );
        let routes = self
            .routes
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .len();
        if state.tick_samples {
            tracing::info!(
                event = "world.tick_sample",
                tick = requests.tick,
                tick_micros = tick_time.as_micros() as u64,
                connections = routes,
            );
        }
        state.tick_health.report(requests.tick, routes);
        self.gateway.observe_tick(requests.tick);
        // The stored user lists are persisted off the world thread via
        // `bbb-save-io` so admin list changes never stall a tick.
        if self.world.admin_list_revision() != state.list_revision {
            state.list_revision = self.world.admin_list_revision();
            if self.options.admin_dir.is_some() {
                if let Some(io) = &self.persistence {
                    io.write_admin_lists_async(self.world.admin_documents());
                }
            }
        }
        // `DedicatedServerSettings.update` (`DedicatedServerSettings.java:23-26`):
        // a runtime change of a property is saved asynchronously for the next start.
        let settings_revision = self.world.admin_settings_revision();
        if settings_revision != state.settings_revision {
            state.settings_revision = settings_revision;
            let settings = self.world.persisted_settings();
            if settings != state.persisted {
                if self.options.admin_dir.is_some() {
                    if let Some(io) = &self.persistence {
                        io.write_persisted_settings_async(settings.clone());
                    }
                }
                state.persisted = settings;
                state.settings_changed = true;
            }
        }
        // `ServerStateService.save` runs on the server thread between
        // ticks; only the unattended autosave amortizes snapshot building
        // across ticks and finishes writing off-thread.
        if let Some((flush, trigger)) = requests.save {
            match &mut self.persistence {
                Some(io) if io.has_world_storage() => {
                    if matches!(trigger, SaveTrigger::Autosave) && !flush {
                        io.start_autosave(&mut self.world);
                    } else {
                        io.save(&mut self.world, trigger, flush);
                    }
                }
                _ => tracing::warn!(
                    event = "admin.save_skipped",
                    trigger = trigger.as_str(),
                    reason = "ephemeral_world",
                ),
            }
        }
        if let Some(selected) = requests.reload {
            self.reload_datapacks(selected);
        }
        if requests.stop {
            tracing::info!(event = "admin.stop_requested");
            return Ok(TickOutcome::Stop(None));
        }
        if let Some(message) = self.world.storage_refusal() {
            tracing::error!(event = "world.stopping_on_unsupported_data", %message);
            return Ok(TickOutcome::Stop(Some(message)));
        }
        Ok(TickOutcome::Continue)
    }

    fn handle_tick_panic(
        &mut self,
        payload: Box<dyn Any + Send>,
        persisted: &mut PersistedSettings,
        settings_changed: &mut bool,
    ) -> ServerRuntimeError {
        let message = panic_report::panic_message(payload.as_ref()).to_string();
        if !panic_report::panic_report_installed() {
            emit_panicked_log(&message);
        }
        self.emergency_cleanup(persisted, settings_changed);
        ServerRuntimeError::WorldThread(format!("the world thread panicked: {message}"))
    }

    fn emergency_cleanup(
        &mut self,
        persisted: &mut PersistedSettings,
        settings_changed: &mut bool,
    ) {
        if let Err(secondary) = catch_unwind(AssertUnwindSafe(|| {
            self.gateway.abandon_pending();
        })) {
            tracing::error!(
                event = "world.emergency_cleanup_panicked",
                step = "abandon_gateway",
                message = panic_report::panic_message(secondary.as_ref()),
            );
        }
        if let Err(secondary) = catch_unwind(AssertUnwindSafe(|| {
            self.route_dispatcher.finish_shutdown(
                &self.routes,
                &self.network,
                self.options.shutdown_timeout(),
            );
        })) {
            tracing::error!(
                event = "world.emergency_cleanup_panicked",
                step = "finish_routes",
                message = panic_report::panic_message(secondary.as_ref()),
            );
        }
        if let Some(mut io) = self.persistence.take() {
            if let Err(secondary) = catch_unwind(AssertUnwindSafe(|| {
                #[cfg(test)]
                if let Some(hook) = self.test_panic_hook.as_ref() {
                    if hook
                        .panic_on_emergency_save
                        .swap(false, std::sync::atomic::Ordering::AcqRel)
                    {
                        panic!("simulated emergency save panic");
                    }
                }
                if io.has_world_storage() {
                    tracing::warn!(event = "world.emergency_save_started");
                    io.save(&mut self.world, SaveTrigger::Shutdown, true);
                }
                io.close(&mut self.world);
            })) {
                tracing::error!(
                    event = "world.emergency_save_panicked",
                    message = panic_report::panic_message(secondary.as_ref()),
                );
            }
        }
        if let Err(secondary) = catch_unwind(AssertUnwindSafe(|| {
            let final_settings = self.world.persisted_settings();
            if final_settings != *persisted {
                *persisted = final_settings;
                *settings_changed = true;
            }
            if let Some(directory) = &self.options.admin_dir {
                write_admin_lists(directory, &self.world.admin_documents());
                if *settings_changed {
                    write_persisted_settings(directory, persisted);
                }
            }
        })) {
            tracing::error!(
                event = "world.emergency_cleanup_panicked",
                step = "admin_flush",
                message = panic_report::panic_message(secondary.as_ref()),
            );
        }
        self.receiver.close();
    }

    fn tick(&mut self) -> Result<TickRequests, ServerRuntimeError> {
        let mut commands = std::mem::take(&mut self.deferred);
        commands.extend(self.receiver.drain());
        self.deferred = defer_join_burst(&mut commands);
        let result = self.world.tick(commands);
        #[cfg(test)]
        if let Some(hook) = self.test_panic_hook.as_ref() {
            if hook
                .panic_on_tick
                .swap(false, std::sync::atomic::Ordering::AcqRel)
            {
                panic!("simulated world tick panic");
            }
        }
        let processed_commands = result.processed_commands;
        let outgoing_events = result.events.len();
        let online_players = result.online_player_count;
        let mut requests = TickRequests {
            tick: result.tick,
            processed_commands,
            ..TickRequests::default()
        };
        self.publish_status(result.online_players, online_players);
        self.route_dispatcher.dispatch(
            result.events,
            super::RouteDispatchEnv {
                routes: &self.routes,
                compression_threshold: self.options.compression_threshold(),
                content_registry: self.startup.plugins.content_registry(),
                gateway: &self.gateway,
                network: &self.network,
                drain_timeout: OUTBOUND_ROUTE_DRAIN_TIMEOUT,
            },
            &mut requests,
        );
        tracing::trace!(
            event = "world.tick",
            tick = result.tick,
            processed_commands,
            outgoing_events,
            online_players,
        );
        Ok(requests)
    }

    /// Publishes the status the listener answers with before any of the
    /// tick's events leave, so a caller told its settings change applied
    /// already connects to the changed status.
    fn publish_status(&mut self, online_players: Option<Vec<(Uuid, String)>>, online_count: usize) {
        let settings = self.world.admin_settings();
        let admission_key = Some((
            self.world.admin_list_revision(),
            settings.use_allowlist,
            settings.max_players,
            online_count,
        ));
        if admission_key != self.admission_key {
            self.admission_key = admission_key;
            self.admission
                .send_replace(Arc::new(self.world.admission_snapshot()));
        }
        let settings_changed = !self.published.matches_settings(settings);
        let players_changed = if let Some(players) = online_players {
            let formatted: Vec<(String, String)> = players
                .into_iter()
                .map(|(uuid, name)| (uuid.to_string(), name))
                .collect();
            if formatted != self.published.players {
                self.published.players = formatted;
                self.gateway.publish_player_list_change();
                true
            } else {
                false
            }
        } else {
            false
        };
        if settings_changed {
            self.published.apply_settings(settings);
        }
        let registries = self.world.synchronized_registries();
        let current_status_changed = {
            let current = self.status.borrow();
            !Arc::ptr_eq(&current.registries, &registries.published_shared())
                || !Arc::ptr_eq(&current.tags, &registries.published_tags_shared())
        };
        if settings_changed || players_changed || current_status_changed {
            self.status.send_replace(connection_config(
                &self.published,
                self.options.compression_threshold(),
                Some(registries),
                &self.startup,
            ));
        }
    }

    /// `MinecraftServer.reloadResources` builds the new generation off the
    /// server thread and installs it on it. The build reads the filesystem,
    /// so it runs on a blocking worker and comes back through the same mailbox
    /// every other world mutation uses, landing at a tick boundary.
    fn reload_datapacks(&self, selected: Option<Vec<String>>) {
        tracing::info!(event = "world.datapack_reload_requested");
        let startup_options = datapack_startup_options(
            &self.options,
            self.persistence
                .as_ref()
                .and_then(SaveIo::candidate)
                .map(|candidate| &candidate.level),
        );
        let previous = self
            .world
            .datapacks_shared()
            .expect("runtime worlds always own the startup datapack generation");
        let commands = self.commands.clone();
        self.network.spawn_blocking(move || {
            // `MinecraftServer.reloadResources` opens exactly the ids it was
            // handed (`MinecraftServer.java:1505-1508`); only `/reload` builds
            // its list by re-discovering.
            let built = match &selected {
                Some(ids) => {
                    DatapackStartup::reload_with_selected_ids(&startup_options, ids, &previous)
                }
                None => DatapackStartup::reload_with_data_packs(
                    &startup_options,
                    previous.selection.data_packs(),
                    &previous,
                ),
            };
            let startup = match built {
                Ok(startup) => {
                    for warning in startup.warnings() {
                        tracing::warn!(event = "world.datapack_warning", message = %warning);
                    }
                    tracing::info!(
                        event = "world.datapacks_reloaded",
                        packs = ?startup.selection.ordered_ids,
                    );
                    ReloadedDatapacks::Built(Arc::new(startup))
                }
                Err(error) => {
                    tracing::error!(event = "world.datapack_reload_failed", %error);
                    ReloadedDatapacks::Failed(error.to_string())
                }
            };
            if let Err(error) = commands.try_send(WorldCommand::Datapacks { startup }) {
                tracing::error!(event = "world.datapack_reload_undelivered", %error);
            }
        });
    }
}

/// The joins one tick admits; a login burst spreads over the following
/// ticks instead of stalling one.
const JOINS_PER_TICK: usize = 3;

/// Splits off everything from the first join past [`JOINS_PER_TICK`], in
/// arrival order, unless the tail carries the shutdown.
fn defer_join_burst(commands: &mut Vec<CommandEnvelope>) -> Vec<CommandEnvelope> {
    let Some(cut) = commands
        .iter()
        .enumerate()
        .filter(|(_, envelope)| matches!(envelope.command(), WorldCommand::Join { .. }))
        .nth(JOINS_PER_TICK)
        .map(|(index, _)| index)
    else {
        return Vec::new();
    };
    if commands[cut..]
        .iter()
        .any(|envelope| matches!(envelope.command(), WorldCommand::Shutdown))
    {
        return Vec::new();
    }
    commands.split_off(cut)
}

/// The columns one tick's unload save writes; a mass logout unloads its
/// area over the following ticks instead of in one.
const UNLOAD_BATCH: usize = 8;

/// How much of the tick may be gone before an unload save starts; vanilla
/// drains its unload queue only while the tick has time
/// (`ChunkMap.processUnloads`, `ChunkMap.java:485-491`).
const UNLOAD_TICK_BUDGET: Duration = Duration::from_millis(20);

/// The backlog past which unloads proceed without spare time, so a server
/// that never has any still drains them (`ChunkMap.java:485`).
const UNLOAD_BACKLOG: usize = 2000;

/// How many ticks the rolling window keeps, which is vanilla's
/// (`MinecraftServer.tickTimesNanos` :249). The window is what makes a sample
/// mean something: one slow tick is a scheduler, a hundred of them is a server.
const TICK_TIME_SAMPLES: usize = 100;

/// The tick loop's own health, sampled the way vanilla samples it and written
/// to the log once per window.
///
/// There is no management method for this and there must not be: the projection
/// is one-to-one with vanilla's RPC surface, which has none. It is a diagnostic
/// the process publishes about itself, and the release soak run is what reads
/// it back.
#[derive(Debug)]
struct TickHealth {
    /// How long each of the last `TICK_TIME_SAMPLES` ticks took to run.
    times_nanos: [u64; TICK_TIME_SAMPLES],
    /// How late each of them was against the deadline it was scheduled for.
    /// This is vanilla's `behindTimeNanos` (`MinecraftServer` :734): the loop
    /// falling behind shows up here before it shows up in a tick's own cost.
    behind_nanos: [u64; TICK_TIME_SAMPLES],
    /// What each of them drained from the world mailbox.
    processed: [usize; TICK_TIME_SAMPLES],
    observed: usize,
}

impl Default for TickHealth {
    fn default() -> Self {
        Self {
            times_nanos: [0; TICK_TIME_SAMPLES],
            behind_nanos: [0; TICK_TIME_SAMPLES],
            processed: [0; TICK_TIME_SAMPLES],
            observed: 0,
        }
    }
}

impl TickHealth {
    fn observe(&mut self, behind: Duration, elapsed: Duration, processed: usize) {
        let index = self.observed % TICK_TIME_SAMPLES;
        self.times_nanos[index] = u64::try_from(elapsed.as_nanos()).unwrap_or(u64::MAX);
        self.behind_nanos[index] = u64::try_from(behind.as_nanos()).unwrap_or(u64::MAX);
        self.processed[index] = processed;
        self.observed += 1;
    }

    /// Writes one sample per full window, so a soak run reads a series rather
    /// than a number and a log stays readable at twenty ticks a second.
    fn report(&self, tick: u64, connections: usize) {
        if self.observed == 0 || !self.observed.is_multiple_of(TICK_TIME_SAMPLES) {
            return;
        }
        let mut window = self.times_nanos;
        window.sort_unstable();
        let total: u128 = window.iter().map(|nanos| u128::from(*nanos)).sum();
        let mut histogram = [0_u32; TICK_HISTOGRAM_MILLIS.len() + 1];
        for &nanos in &window {
            let bucket = TICK_HISTOGRAM_MILLIS.partition_point(|&edge| nanos >= edge * 1_000_000);
            histogram[bucket] += 1;
        }
        let histogram = histogram
            .iter()
            .map(u32::to_string)
            .collect::<Vec<_>>()
            .join(",");
        tracing::info!(
            event = "world.tick_health",
            tick,
            samples = TICK_TIME_SAMPLES,
            mean_tick_micros = (total / TICK_TIME_SAMPLES as u128) as u64 / 1_000,
            max_tick_micros = window[TICK_TIME_SAMPLES - 1] / 1_000,
            p99_tick_micros = window[TICK_TIME_SAMPLES * 99 / 100 - 1] / 1_000,
            tick_histogram_millis = %histogram,
            max_behind_micros = self.behind_nanos.iter().copied().max().unwrap_or_default() / 1_000,
            max_queue_depth = self.processed.iter().copied().max().unwrap_or_default(),
            connections,
        );
    }
}

/// Lower bucket edges of `tick_histogram_millis`: bucket `i` counts ticks in
/// `[edge[i-1], edge[i])`, the last one everything from 1000 ms up, so a reader
/// can add windows and take a percentile over any span of them.
const TICK_HISTOGRAM_MILLIS: [u64; 10] = [5, 10, 20, 30, 40, 50, 100, 250, 500, 1000];

/// `TimeUtil.NANOSECONDS_PER_MILLISECOND`.
const NANOSECONDS_PER_MILLISECOND: i64 = 1_000_000;

/// `MinecraftServer.OVERLOADED_THRESHOLD_NANOS` (MinecraftServer.java:199):
/// one second of arrears at the default rate.
const OVERLOADED_THRESHOLD_NANOS: i64 = 20 * 1_000_000_000 / 20;

/// `MinecraftServer.OVERLOADED_WARNING_INTERVAL_NANOS`
/// (MinecraftServer.java:201).
const OVERLOADED_WARNING_INTERVAL_NANOS: i64 = 10 * 1_000_000_000;

/// The loop's own reading of `Util.getNanos`, taken against the instant the
/// loop started so the difference is what vanilla's absolute nanoseconds mean.
fn elapsed_nanos(epoch: Instant, at: Instant) -> i64 {
    i64::try_from(at.saturating_duration_since(epoch).as_nanos()).unwrap_or(i64::MAX)
}

fn emit_panicked_log(message: &str) {
    tracing::error!(
        event = "server.panicked",
        message = %message,
        thread = std::thread::current().name(),
    );
}

/// The inverse, for turning a deadline back into something to sleep until. A
/// deadline can only be non-negative here: it starts at zero and every arm
/// that moves it adds either a positive period or the current elapsed time.
fn nanos_duration(nanos: i64) -> Duration {
    Duration::from_nanos(u64::try_from(nanos).unwrap_or(0))
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};
    use tracing::Subscriber;

    #[derive(Default)]
    struct CapturePanicked(Arc<Mutex<Vec<(String, String)>>>);

    struct FieldVisitor(Option<String>, Option<String>);

    impl tracing::field::Visit for FieldVisitor {
        fn record_str(&mut self, field: &tracing::field::Field, value: &str) {
            match field.name() {
                "event" => self.0 = Some(value.to_string()),
                "message" => self.1 = Some(value.to_string()),
                _ => {}
            }
        }

        fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
            if field.name() == "message" && self.1.is_none() {
                self.1 = Some(format!("{value:?}"));
            }
        }
    }

    impl Subscriber for CapturePanicked {
        fn enabled(&self, _metadata: &tracing::Metadata<'_>) -> bool {
            true
        }
        fn new_span(&self, _span: &tracing::span::Attributes<'_>) -> tracing::span::Id {
            tracing::span::Id::from_u64(1)
        }
        fn record(&self, _span: &tracing::span::Id, _values: &tracing::span::Record<'_>) {}
        fn record_follows_from(&self, _span: &tracing::span::Id, _follows: &tracing::span::Id) {}
        fn event(&self, event: &tracing::Event<'_>) {
            let mut visitor = FieldVisitor(None, None);
            event.record(&mut visitor);
            if let (Some(ev), Some(msg)) = (visitor.0, visitor.1) {
                self.0.lock().unwrap().push((ev, msg));
            }
        }
        fn enter(&self, _span: &tracing::span::Id) {}
        fn exit(&self, _span: &tracing::span::Id) {}
    }

    #[test]
    fn emits_structured_server_panicked_event() {
        let captured = Arc::new(Mutex::new(Vec::new()));
        let subscriber = CapturePanicked(Arc::clone(&captured));
        tracing::subscriber::with_default(subscriber, || {
            super::emit_panicked_log("boom in tick");
        });
        let events = captured.lock().unwrap();
        assert_eq!(
            events.as_slice(),
            &[("server.panicked".to_string(), "boom in tick".to_string())]
        );
    }
}
