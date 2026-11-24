//! Split save reader and writer threads and the world-thread half that feeds them.
//!
//! Reads run on a dedicated `bbb-save-read` thread so chunk loads never queue
//! behind autosaves or admin JSON writes on `bbb-save-write`. Only one save
//! write request is in flight on `bbb-save-write` at a time, preserving write
//! ordering; both `Autosave` and `ChunkUnload` run detached with epoch-tracked
//! completion acknowledgements, while explicit `Management` and `Shutdown`
//! saves settle in-flight reads and wait for completion.

use std::collections::BTreeSet;
use std::path::PathBuf;
use std::sync::{
    mpsc::{self, Receiver, Sender},
    Arc,
};
use std::thread::JoinHandle;

use bbb_server_core::admin::{PersistedSettings, StoredUserListDocuments};
use bbb_server_core::world::{
    materialize_deferred_chunks, DeferredChunkWrite, DetachedUnloadBatch, SaveTrigger, ServerWorld,
};
use bbb_server_persistence::world_storage::Java26RuntimeColumn;
use bbb_server_persistence::{
    Java26LevelKey, Java26LoadedEntityChunk, Java26SaveRequest, Java26WorldCandidate,
    Java26WorldStorage, RegionCompression,
};
use bbb_server_types::save::{SaveError, SaveResult};

use super::{epoch_millis, region_timestamp, write_admin_lists, write_persisted_settings};

/// Maximum dirty columns serialized into a detached autosave per server tick.
const AUTOSAVE_COLUMNS_PER_TICK: usize = 16;

struct SaveJob {
    epoch: u64,
    request: Java26SaveRequest,
    deferred_chunks: Vec<DeferredChunkWrite>,
    trigger: SaveTrigger,
    flush: bool,
    last_played: i64,
    region_timestamp: u32,
    unload_batch: Option<DetachedUnloadBatch>,
    result: SaveResult<()>,
}

type ChunkKey = (i32, i32);

enum WriteJob {
    Save(Box<SaveJob>),
    AdminLists(StoredUserListDocuments),
    AdminSettings(PersistedSettings),
    #[cfg(test)]
    PauseForTest(Receiver<()>),
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
enum RecordRead {
    Column(Java26LevelKey, ChunkKey),
    Entities(Java26LevelKey, ChunkKey),
}

enum RecordReadResult {
    Column(
        Java26LevelKey,
        ChunkKey,
        SaveResult<Box<Java26RuntimeColumn>>,
    ),
    Entities(
        Java26LevelKey,
        ChunkKey,
        SaveResult<Option<Java26LoadedEntityChunk>>,
    ),
}

pub(super) struct SaveIo {
    candidate: Option<Arc<Java26WorldCandidate>>,
    write_jobs: Option<Sender<WriteJob>>,
    read_jobs: Option<Sender<Vec<RecordRead>>>,
    written: Receiver<SaveJob>,
    read: Receiver<RecordReadResult>,
    /// Reads sent and not answered yet.
    reading: BTreeSet<RecordRead>,
    /// Epoch of an in-flight detached save (`Autosave` or `ChunkUnload`).
    detached_epoch: Option<u64>,
    next_save_epoch: u64,
    reader: Option<JoinHandle<()>>,
    writer: Option<JoinHandle<()>>,
}

impl SaveIo {
    pub(super) fn start(
        world: Option<(Java26WorldStorage, Java26WorldCandidate)>,
        admin_dir: Option<PathBuf>,
    ) -> std::io::Result<Self> {
        let (storage, candidate) = match world {
            Some((storage, candidate)) => (Some(Arc::new(storage)), Some(Arc::new(candidate))),
            None => (None, None),
        };
        let (write_jobs, write_queued) = mpsc::channel::<WriteJob>();
        let (done, written) = mpsc::sync_channel(1);
        let (read_done, read) = mpsc::channel();

        let (read_jobs, reader) = if let (Some(read_storage), Some(read_candidate)) =
            (storage.clone(), candidate.clone())
        {
            let (read_tx, read_queued) = mpsc::channel::<Vec<RecordRead>>();
            let handle = std::thread::Builder::new()
                .name("bbb-save-read".to_string())
                .stack_size(crate::SERVER_THREAD_STACK_BYTES)
                .spawn(move || {
                    for reads in read_queued {
                        if !read_records(&read_storage, &read_candidate, reads, &read_done) {
                            break;
                        }
                    }
                })?;
            (Some(read_tx), Some(handle))
        } else {
            (None, None)
        };

        let writer = {
            let candidate = candidate.clone();
            std::thread::Builder::new()
                .name("bbb-save-write".to_string())
                .stack_size(crate::SERVER_THREAD_STACK_BYTES)
                .spawn(move || {
                    for job in write_queued {
                        let mut job = match job {
                            WriteJob::Save(job) => *job,
                            WriteJob::AdminLists(documents) => {
                                if let Some(directory) = admin_dir.as_deref() {
                                    write_admin_lists(directory, &documents);
                                }
                                continue;
                            }
                            WriteJob::AdminSettings(settings) => {
                                if let Some(directory) = admin_dir.as_deref() {
                                    write_persisted_settings(directory, &settings);
                                }
                                continue;
                            }
                            #[cfg(test)]
                            WriteJob::PauseForTest(gate) => {
                                let _ = gate.recv();
                                continue;
                            }
                        };
                        let (Some(storage), Some(candidate)) =
                            (storage.as_ref(), candidate.as_ref())
                        else {
                            continue;
                        };
                        let deferred = std::mem::take(&mut job.deferred_chunks);
                        job.result = if deferred.is_empty() {
                            Ok(())
                        } else {
                            materialize_deferred_chunks(deferred).map(|chunks| {
                                job.request.chunk_writes = chunks;
                            })
                        }
                        .and_then(|()| match job.trigger {
                            SaveTrigger::ChunkUnload => {
                                storage.save_runtime_unloads(candidate, &job.request)
                            }
                            SaveTrigger::Autosave => storage.save_autosave(candidate, &job.request),
                            SaveTrigger::Management | SaveTrigger::Shutdown => {
                                storage.save_snapshot(candidate, &job.request)
                            }
                        })
                        .and_then(|()| {
                            if job.flush {
                                storage.flush()
                            } else {
                                Ok(())
                            }
                        });
                        if done.send(job).is_err() {
                            break;
                        }
                    }
                })?
        };
        Ok(Self {
            candidate,
            write_jobs: Some(write_jobs),
            read_jobs,
            written,
            read,
            reading: BTreeSet::new(),
            detached_epoch: None,
            next_save_epoch: 1,
            reader,
            writer: Some(writer),
        })
    }

    pub(super) const fn has_world_storage(&self) -> bool {
        self.candidate.is_some()
    }

    pub(super) fn candidate(&self) -> Option<&Java26WorldCandidate> {
        self.candidate.as_deref()
    }

    /// Offloads `whitelist.json`, `ops.json`, and `banned-*.json` writes to
    /// `bbb-save-write` so `bbb-world` never blocks on filesystem sync.
    pub(super) fn write_admin_lists_async(&self, documents: StoredUserListDocuments) {
        if !self
            .write_jobs
            .as_ref()
            .is_some_and(|jobs| jobs.send(WriteJob::AdminLists(documents)).is_ok())
        {
            tracing::error!(
                event = "admin.lists_write_failed",
                error = "the save writer thread is gone",
            );
        }
    }

    /// Offloads `server-settings.json` writes to `bbb-save-write`.
    pub(super) fn write_persisted_settings_async(&self, settings: PersistedSettings) {
        if !self
            .write_jobs
            .as_ref()
            .is_some_and(|jobs| jobs.send(WriteJob::AdminSettings(settings)).is_ok())
        {
            tracing::error!(
                event = "admin.settings_write_failed",
                error = "the save writer thread is gone",
            );
        }
    }

    /// Hands every record that came back to the world, then asks the dedicated
    /// reader thread for the ones its tickets now need.
    pub(super) fn pump_reads(&mut self, world: &mut ServerWorld) {
        if !self.has_world_storage() {
            return;
        }
        self.complete_reads(world);
        let mut reads = world
            .pending_runtime_columns()
            .into_iter()
            .map(|(dimension, chunk)| RecordRead::Column(dimension, chunk))
            .chain(
                world
                    .pending_runtime_entity_chunks()
                    .into_iter()
                    .map(|(dimension, chunk)| RecordRead::Entities(dimension, chunk)),
            )
            .filter(|read| !self.reading.contains(read))
            .collect::<Vec<_>>();
        reads.sort();
        reads.dedup();
        if reads.is_empty() {
            return;
        }
        self.reading.extend(reads.iter().cloned());
        if !self
            .read_jobs
            .as_ref()
            .is_some_and(|jobs| jobs.send(reads).is_ok())
        {
            tracing::error!(
                event = "world.chunk_read_failed",
                error = "the save reader thread is gone",
            );
        }
    }

    /// Hands every record that came back to the world.
    pub(super) fn complete_reads(&mut self, world: &mut ServerWorld) {
        while let Ok(result) = self.read.try_recv() {
            self.complete_read(world, result);
        }
    }

    fn in_flight_read_chunks(&self) -> BTreeSet<(Java26LevelKey, ChunkKey)> {
        self.reading
            .iter()
            .map(|read| match read {
                RecordRead::Column(dimension, chunk) | RecordRead::Entities(dimension, chunk) => {
                    (dimension.clone(), *chunk)
                }
            })
            .collect()
    }

    /// Every read the world waits on lands before an explicit save freezes, so
    /// an entity that moved into a chunk still being read is saved with it.
    fn settle_reads(&mut self, world: &mut ServerWorld) {
        self.pump_reads(world);
        while !self.reading.is_empty() {
            let Ok(result) = self.read.recv() else {
                break;
            };
            self.complete_read(world, result);
        }
    }

    fn complete_read(&mut self, world: &mut ServerWorld, result: RecordReadResult) {
        match result {
            RecordReadResult::Column(dimension, chunk, record) => {
                self.reading
                    .remove(&RecordRead::Column(dimension.clone(), chunk));
                world.complete_runtime_columns(
                    &dimension,
                    vec![(chunk, record.map(|column| *column))],
                );
            }
            RecordReadResult::Entities(dimension, chunk, record) => {
                self.reading
                    .remove(&RecordRead::Entities(dimension.clone(), chunk));
                match record {
                    Ok(record) => {
                        if let Err(error) =
                            world.complete_runtime_entity_chunk(&dimension, chunk, record)
                        {
                            tracing::error!(
                                event = "world.runtime_entity_publication_failed",
                                dimension = %dimension.resource_id(),
                                x = chunk.0,
                                z = chunk.1,
                                %error
                            );
                        }
                    }
                    Err(error) => world.fail_runtime_entity_chunk(&dimension, chunk, &error),
                }
            }
        }
    }

    /// Acknowledges a detached save (`Autosave` or `ChunkUnload`) that finished
    /// and reports whether the writer is free.
    pub(super) fn idle(&mut self, world: &mut ServerWorld) -> bool {
        if let Some(expected_epoch) = self.detached_epoch {
            match self.written.try_recv() {
                Ok(job) => {
                    if job.epoch == expected_epoch {
                        self.detached_epoch = None;
                    }
                    self.finish(world, job, true);
                }
                Err(mpsc::TryRecvError::Empty) => {}
                Err(mpsc::TryRecvError::Disconnected) => {
                    self.detached_epoch = None;
                }
            }
        }
        self.detached_epoch.is_none()
    }

    fn wait(&mut self, world: &mut ServerWorld) {
        while let Some(expected_epoch) = self.detached_epoch {
            let Ok(job) = self.written.recv() else {
                self.detached_epoch = None;
                break;
            };
            if job.epoch == expected_epoch {
                self.detached_epoch = None;
            }
            self.finish(world, job, true);
        }
    }

    /// Starts an asynchronous detached unload batch up to `batch_limit` columns
    /// and entity sections, skipping any column with an in-flight read instead
    /// of blocking on `settle_reads`.
    pub(super) fn save_unload_batch(&mut self, world: &mut ServerWorld, batch_limit: usize) {
        self.save_unload_scoped(world, batch_limit, false);
    }

    /// Starts an amortized unattended autosave cycle, stepping up to
    /// [`AUTOSAVE_COLUMNS_PER_TICK`] dirty columns on this tick and continuing
    /// on subsequent ticks via [`Self::pump_autosave`].
    pub(super) fn start_autosave(&mut self, world: &mut ServerWorld) {
        if !self.has_world_storage() {
            return;
        }
        if let Err(error) = world.begin_autosave_snapshot() {
            tracing::error!(
                event = "world.save_request_failed",
                trigger = SaveTrigger::Autosave.as_str(),
                %error,
            );
            return;
        }
        self.pump_autosave(world);
    }

    pub(super) fn pump_autosave(&mut self, world: &mut ServerWorld) {
        self.pump_autosave_with_budget(world, AUTOSAVE_COLUMNS_PER_TICK);
    }

    fn pump_autosave_with_budget(&mut self, world: &mut ServerWorld, column_budget: usize) {
        if !self.has_world_storage() || !world.has_pending_autosave_snapshot() {
            return;
        }
        let can_submit = self.idle(world);
        let last_played = if can_submit { epoch_millis() } else { 0 };
        let region_ts = if can_submit { region_timestamp() } else { 0 };
        match world.step_autosave_snapshot_deferred(
            last_played,
            region_ts,
            RegionCompression::Zlib,
            column_budget,
            can_submit,
        ) {
            Ok(Some((request, deferred_chunks))) => {
                self.submit_save_job(
                    world,
                    request,
                    deferred_chunks,
                    SaveTrigger::Autosave,
                    false,
                    last_played,
                    region_ts,
                    None,
                );
            }
            Ok(None) => {}
            Err(error) => {
                tracing::error!(
                    event = "world.save_request_failed",
                    trigger = SaveTrigger::Autosave.as_str(),
                    %error,
                );
            }
        }
    }

    pub(super) fn save(&mut self, world: &mut ServerWorld, trigger: SaveTrigger, flush: bool) {
        if matches!(trigger, SaveTrigger::ChunkUnload) {
            self.save_unload_scoped(world, usize::MAX, flush);
            return;
        }
        if !self.has_world_storage() {
            return;
        }
        self.wait(world);
        if matches!(trigger, SaveTrigger::Management | SaveTrigger::Shutdown) {
            self.settle_reads(world);
        } else {
            self.complete_reads(world);
        }
        let last_played = epoch_millis();
        let region_timestamp = region_timestamp();
        let (request, deferred_chunks) = match world.save_request_deferred(
            last_played,
            region_timestamp,
            RegionCompression::Zlib,
        ) {
            Ok(pair) => pair,
            Err(error) => {
                tracing::error!(
                    event = "world.save_request_failed",
                    trigger = trigger.as_str(),
                    %error,
                );
                return;
            }
        };
        self.submit_save_job(
            world,
            request,
            deferred_chunks,
            trigger,
            flush,
            last_played,
            region_timestamp,
            None,
        );
    }

    fn save_unload_scoped(&mut self, world: &mut ServerWorld, batch_limit: usize, flush: bool) {
        if !self.has_world_storage() {
            return;
        }
        self.wait(world);
        self.complete_reads(world);
        let in_flight = self.in_flight_read_chunks();
        let region_timestamp = region_timestamp();
        let (request, deferred_chunks, batch) = match world
            .runtime_unload_save_request_batch_deferred(
                region_timestamp,
                RegionCompression::Zlib,
                batch_limit,
                &in_flight,
            ) {
            Ok(Some(triple)) => triple,
            Ok(None) => return,
            Err(error) => {
                tracing::error!(
                    event = "world.save_request_failed",
                    trigger = SaveTrigger::ChunkUnload.as_str(),
                    %error,
                );
                return;
            }
        };
        self.submit_save_job(
            world,
            request,
            deferred_chunks,
            SaveTrigger::ChunkUnload,
            flush,
            0,
            region_timestamp,
            Some(batch),
        );
    }

    #[allow(clippy::too_many_arguments)]
    fn submit_save_job(
        &mut self,
        world: &mut ServerWorld,
        request: Java26SaveRequest,
        deferred_chunks: Vec<DeferredChunkWrite>,
        trigger: SaveTrigger,
        flush: bool,
        last_played: i64,
        region_timestamp: u32,
        unload_batch: Option<DetachedUnloadBatch>,
    ) {
        let epoch = self.next_save_epoch;
        self.next_save_epoch = self.next_save_epoch.wrapping_add(1).max(1);
        let job = SaveJob {
            epoch,
            request,
            deferred_chunks,
            trigger,
            flush,
            last_played,
            region_timestamp,
            unload_batch,
            result: Ok(()),
        };
        if !self
            .write_jobs
            .as_ref()
            .is_some_and(|jobs| jobs.send(WriteJob::Save(Box::new(job))).is_ok())
        {
            tracing::error!(
                event = "world.save_failed",
                trigger = trigger.as_str(),
                error = "the save writer thread is gone",
            );
            return;
        }
        let detach = !flush && matches!(trigger, SaveTrigger::Autosave | SaveTrigger::ChunkUnload);
        if detach {
            self.detached_epoch = Some(epoch);
            return;
        }
        if let Ok(job) = self.written.recv() {
            self.finish(world, job, false);
        }
    }

    fn finish(&mut self, world: &mut ServerWorld, job: SaveJob, detached: bool) {
        self.complete_reads(world);
        let SaveJob {
            epoch: _,
            request,
            trigger,
            last_played,
            region_timestamp,
            unload_batch,
            result,
            ..
        } = job;
        if let Err(error) = result {
            tracing::error!(
                event = "world.save_failed",
                trigger = trigger.as_str(),
                %error,
            );
            return;
        }
        if detached {
            if matches!(trigger, SaveTrigger::ChunkUnload) {
                if let Some(batch) = unload_batch.as_ref() {
                    let in_flight = self.in_flight_read_chunks();
                    world.acknowledge_detached_unload(&request, batch, &in_flight);
                }
            } else {
                world.acknowledge_detached_save(&request);
            }
        } else if let Some(batch) = unload_batch.as_ref() {
            let in_flight = self.in_flight_read_chunks();
            world.acknowledge_detached_unload(&request, batch, &in_flight);
        } else {
            world.acknowledge_save_success(&request);
        }
        // The receipt names what asked for the save, so an unattended one is
        // distinguishable from an operator's and from the shutdown flush.
        tracing::info!(
            event = "world.saved",
            trigger = trigger.as_str(),
            last_played,
            region_timestamp,
        );
    }

    /// Acknowledges what the writer still owes, drains queued admin JSON jobs,
    /// and joins both the reader and writer threads.
    pub(super) fn close(mut self, world: &mut ServerWorld) {
        self.wait(world);
        self.read_jobs = None;
        self.write_jobs = None;
        if let Some(reader) = self.reader.take() {
            let _ = reader.join();
        }
        if let Some(writer) = self.writer.take() {
            let _ = writer.join();
        }
    }
}

/// Returns `false` when the world thread's read receiver has disconnected.
fn read_records(
    storage: &Java26WorldStorage,
    candidate: &Java26WorldCandidate,
    reads: Vec<RecordRead>,
    done: &Sender<RecordReadResult>,
) -> bool {
    let reader = storage.runtime_reader(candidate);
    let unavailable = |error: &SaveError| SaveError::InvalidNbt {
        path: None,
        detail: error.to_string(),
    };
    for read in reads {
        let sent = done.send(match read {
            RecordRead::Column(dimension, (x, z)) => {
                let record = match &reader {
                    Ok(reader) => reader.read_column(&dimension, x, z),
                    Err(error) => Err(unavailable(error)),
                };
                RecordReadResult::Column(dimension, (x, z), record.map(Box::new))
            }
            RecordRead::Entities(dimension, (x, z)) => {
                let record = match &reader {
                    Ok(reader) => reader.read_entity_chunk(&dimension, x, z),
                    Err(error) => Err(unavailable(error)),
                };
                RecordReadResult::Entities(dimension, (x, z), record)
            }
        });
        if sent.is_err() {
            return false;
        }
    }
    true
}

impl Drop for SaveIo {
    /// Even a world thread that failed lets a write in progress finish, so
    /// the process never exits halfway through a region file.
    fn drop(&mut self) {
        self.read_jobs = None;
        self.write_jobs = None;
        if let Some(reader) = self.reader.take() {
            let _ = reader.join();
        }
        if let Some(writer) = self.writer.take() {
            let _ = writer.join();
        }
    }
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, Instant};

    use bbb_server_core::world::{ServerWorld, ServerWorldConfig};
    use bbb_server_persistence::{Java26WorldCreation, Java26WorldStorage, RegionCompression};
    use bbb_server_types::save::{WorldSaveLimits, WorldSavePaths};

    use super::*;

    fn test_world_setup(name: &str) -> (PathBuf, SaveIo, ServerWorld) {
        let root = std::env::temp_dir().join(format!(
            "bbb-save-io-{name}-{}",
            uuid::Uuid::new_v4().simple()
        ));
        let fixture = bbb_server_types::fixture::FlatFixture::new().unwrap();
        let source = bbb_server_core::world_creation::default_overworld_level_source().unwrap();
        let c0 = source
            .generate_chunk(0, 0, fixture.biome_id())
            .into_column();
        let c40 = source
            .generate_chunk(40, 0, fixture.biome_id())
            .into_column();
        let created = bbb_server_core::world_creation::created_save(
            &Java26WorldCreation {
                level_name: name.into(),
                game_type: 0,
                difficulty: "normal".into(),
                spawn_position: [0, -60, 0],
                last_played: 1,
            },
            &fixture,
            &[(0, 0, &c0), (40, 0, &c40)],
        )
        .unwrap();
        let storage = Java26WorldStorage::open(
            WorldSavePaths::new(&root).unwrap(),
            WorldSaveLimits::default(),
        )
        .unwrap();
        storage
            .create_world(&created, RegionCompression::Zlib, 1)
            .unwrap();
        let candidate = storage.load_candidate().unwrap();
        let config = ServerWorldConfig {
            background_generation: true,
            ..ServerWorldConfig::default()
        };
        let world = ServerWorld::restore(config, &candidate).unwrap();
        let io = SaveIo::start(Some((storage, candidate)), None).unwrap();
        (root, io, world)
    }

    fn run_admin_command(
        sender: &bbb_server_core::mailbox::WorldCommandSender,
        receiver: &bbb_server_core::mailbox::WorldCommandReceiver,
        world: &mut ServerWorld,
        op_id: u64,
        command: &str,
    ) -> Vec<bbb_server_core::world::WorldEvent> {
        sender
            .try_send(bbb_server_core::world::WorldCommand::Admin {
                envelope: bbb_server_core::admin::AdminEnvelope {
                    operation_id: op_id,
                    request_id: format!("req-{op_id}"),
                    principal: "test".into(),
                    scope: bbb_server_core::admin::AdminScope::Command,
                    deadline_tick: u64::MAX,
                    now: 1,
                },
                request: bbb_server_core::admin::AdminRequest::ExecuteCommand {
                    command: command.into(),
                },
            })
            .unwrap();
        world.tick(receiver.drain()).events
    }

    #[test]
    fn chunk_reads_complete_while_writer_is_blocked() {
        let (root, mut io, mut world) = test_world_setup("reads-while-writer-paused");
        let (sender, receiver) = bbb_server_core::mailbox::world_command_mailbox(
            bbb_server_core::mailbox::WorldMailboxLimits::default(),
        )
        .unwrap();
        let (tx, rx) = mpsc::channel();
        io.write_jobs
            .as_ref()
            .unwrap()
            .send(WriteJob::PauseForTest(rx))
            .unwrap();

        let events = run_admin_command(&sender, &receiver, &mut world, 1, "forceload add 640 0");
        for _ in 0..4 {
            if !world.pending_runtime_columns().is_empty() {
                break;
            }
            world.tick(Vec::new());
        }
        assert!(
            !world.pending_runtime_columns().is_empty(),
            "forceload add should queue a runtime column read; events={events:?}"
        );
        io.pump_reads(&mut world);

        let deadline = Instant::now() + Duration::from_secs(5);
        while Instant::now() < deadline {
            io.complete_reads(&mut world);
            if io.reading.is_empty() && world.pending_runtime_columns().is_empty() {
                break;
            }
            std::thread::sleep(Duration::from_millis(5));
        }
        assert!(io.reading.is_empty());
        assert!(world.pending_runtime_columns().is_empty());

        let _ = tx.send(());
        io.close(&mut world);
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn save_unload_batch_is_detached_and_waits_for_writer() {
        let (root, mut io, mut world) = test_world_setup("detached-unload");
        let (sender, receiver) = bbb_server_core::mailbox::world_command_mailbox(
            bbb_server_core::mailbox::WorldMailboxLimits::default(),
        )
        .unwrap();
        run_admin_command(&sender, &receiver, &mut world, 1, "forceload add 640 0");
        for _ in 0..8 {
            world.tick(Vec::new());
            io.pump_reads(&mut world);
            io.settle_reads(&mut world);
        }
        run_admin_command(&sender, &receiver, &mut world, 2, "forceload remove 640 0");
        for _ in 0..20 {
            world.tick(Vec::new());
            if world.needs_runtime_unload_save() {
                break;
            }
        }
        assert!(world.needs_runtime_unload_save());

        let (tx, rx) = mpsc::channel();
        io.write_jobs
            .as_ref()
            .unwrap()
            .send(WriteJob::PauseForTest(rx))
            .unwrap();

        // Returns immediately without blocking the world thread on written.recv().
        io.save_unload_batch(&mut world, 8);
        assert!(io.detached_epoch.is_some());
        assert!(!io.idle(&mut world));

        let _ = tx.send(());
        let deadline = Instant::now() + Duration::from_secs(5);
        while Instant::now() < deadline {
            if io.idle(&mut world) {
                break;
            }
            std::thread::sleep(Duration::from_millis(5));
        }
        assert!(io.idle(&mut world));

        io.close(&mut world);
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn amortized_autosave_spreads_columns_across_ticks_and_persists_all() {
        let (root, mut io, mut world) = test_world_setup("amortized-autosave");
        let (sender, receiver) = bbb_server_core::mailbox::world_command_mailbox(
            bbb_server_core::mailbox::WorldMailboxLimits::default(),
        )
        .unwrap();
        run_admin_command(&sender, &receiver, &mut world, 1, "forceload add 0 0");
        run_admin_command(&sender, &receiver, &mut world, 2, "forceload add 16 0");
        run_admin_command(&sender, &receiver, &mut world, 3, "forceload add 640 0");
        for _ in 0..8 {
            world.tick(Vec::new());
            io.pump_reads(&mut world);
            io.settle_reads(&mut world);
        }
        run_admin_command(
            &sender,
            &receiver,
            &mut world,
            4,
            "setblock 1 -60 1 minecraft:diamond_block",
        );
        run_admin_command(
            &sender,
            &receiver,
            &mut world,
            5,
            "setblock 17 -60 1 minecraft:emerald_block",
        );
        run_admin_command(
            &sender,
            &receiver,
            &mut world,
            6,
            "setblock 641 -60 1 minecraft:gold_block",
        );

        world.begin_autosave_snapshot().unwrap();
        assert!(world.has_pending_autosave_snapshot());
        io.pump_autosave_with_budget(&mut world, 16);
        assert!(world.has_pending_autosave_snapshot());
        assert!(io.detached_epoch.is_none());

        let mut steps = 1usize;
        while world.has_pending_autosave_snapshot() && steps < 2048 {
            io.pump_autosave_with_budget(&mut world, 16);
            steps += 1;
        }
        assert!(steps > 1);
        assert!(!world.has_pending_autosave_snapshot());
        assert!(io.detached_epoch.is_some());

        let deadline = Instant::now() + Duration::from_secs(5);
        while Instant::now() < deadline {
            if io.idle(&mut world) {
                break;
            }
            std::thread::sleep(Duration::from_millis(5));
        }
        assert!(io.idle(&mut world));

        io.close(&mut world);
        let reopen = Java26WorldStorage::open(
            WorldSavePaths::new(&root).unwrap(),
            WorldSaveLimits::default(),
        )
        .unwrap();
        let candidate = reopen.load_candidate().unwrap();
        let overworld = bbb_server_types::save::Java26BuiltinDimension::Overworld.into();
        let c40 = reopen
            .read_runtime_column(&candidate, &overworld, 40, 0)
            .unwrap();
        assert!(c40.chunk.is_some());
        reopen.close().unwrap();
        let _ = std::fs::remove_dir_all(root);
    }
}
