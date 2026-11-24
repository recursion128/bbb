//! Dedicated off-main-thread Ogg Vorbis decode worker for static sound cache
//! misses and streaming PCM chunk lookahead refills.

use std::{
    collections::VecDeque,
    io,
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, AtomicU64, Ordering},
        Arc, Condvar, Mutex, MutexGuard,
    },
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

use crate::residency::{
    decode_static_pack_until, AudioDecodeError, DecodedStaticAudio, OpenAlPcmChunk,
    OpenAlStreamDecoder,
};

pub(crate) const STREAM_PREFETCH_CHUNKS: usize = 4;
const MAX_QUEUED_STATIC_DECODES: usize = 256;

pub(crate) struct StaticDecodeCompletion {
    pub(crate) epoch: u64,
    pub(crate) generation: u64,
    pub(crate) path: PathBuf,
    pub(crate) result: Result<DecodedStaticAudio, AudioDecodeError>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum StreamChunkStatus {
    Waiting,
    Ready,
    FinishedEmpty,
    Error(String),
}

pub(crate) enum StreamChunkPop {
    Chunk(OpenAlPcmChunk),
    Empty,
    Eof,
    Error(String),
}

struct StreamQueueInner {
    chunks: VecDeque<OpenAlPcmChunk>,
    eof: bool,
    error: Option<String>,
}

struct StreamChunkQueue {
    state: Mutex<StreamQueueInner>,
    cancelled: AtomicBool,
}

impl StreamChunkQueue {
    fn new() -> Self {
        Self {
            state: Mutex::new(StreamQueueInner {
                chunks: VecDeque::with_capacity(STREAM_PREFETCH_CHUNKS),
                eof: false,
                error: None,
            }),
            cancelled: AtomicBool::new(false),
        }
    }

    fn lock(&self) -> MutexGuard<'_, StreamQueueInner> {
        self.state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::Acquire)
    }

    fn buffered_chunks(&self) -> usize {
        self.lock().chunks.len()
    }
}

pub(crate) struct AsyncOpenAlStreamHandle {
    shared: Arc<DecodeWorkerShared>,
    queue: Arc<StreamChunkQueue>,
    path: PathBuf,
    looping: bool,
    finished: bool,
}

impl AsyncOpenAlStreamHandle {
    pub(crate) fn path(&self) -> &Path {
        &self.path
    }

    pub(crate) fn looping(&self) -> bool {
        self.looping
    }

    #[cfg(test)]
    pub(crate) fn is_finished(&self) -> bool {
        self.finished
    }

    pub(crate) fn peek_status(&self) -> StreamChunkStatus {
        let state = self.queue.lock();
        if !state.chunks.is_empty() {
            StreamChunkStatus::Ready
        } else if let Some(error) = &state.error {
            StreamChunkStatus::Error(error.clone())
        } else if state.eof {
            StreamChunkStatus::FinishedEmpty
        } else {
            StreamChunkStatus::Waiting
        }
    }

    pub(crate) fn try_pop_chunk(&mut self) -> StreamChunkPop {
        if self.finished {
            return StreamChunkPop::Eof;
        }
        let popped = {
            let mut state = self.queue.lock();
            if let Some(chunk) = state.chunks.pop_front() {
                StreamChunkPop::Chunk(chunk)
            } else if let Some(error) = state.error.take() {
                self.finished = true;
                return StreamChunkPop::Error(error);
            } else if state.eof {
                self.finished = true;
                return StreamChunkPop::Eof;
            } else {
                return StreamChunkPop::Empty;
            }
        };
        let mut worker_state = lock_worker_state(&self.shared);
        worker_state.refill_requested = true;
        drop(worker_state);
        self.shared.wake.notify_one();
        popped
    }
}

impl Drop for AsyncOpenAlStreamHandle {
    fn drop(&mut self) {
        self.queue.cancelled.store(true, Ordering::Release);
        let mut worker_state = lock_worker_state(&self.shared);
        worker_state.refill_requested = true;
        drop(worker_state);
        self.shared.wake.notify_one();
    }
}

struct StaticDecodeJob {
    epoch: u64,
    generation: u64,
    path: PathBuf,
}

struct StreamOpenJob {
    epoch: u64,
    path: PathBuf,
    looping: bool,
    start_frame: u64,
    queue: Arc<StreamChunkQueue>,
}

#[derive(Default)]
struct DecodeWorkerState {
    static_jobs: VecDeque<StaticDecodeJob>,
    stream_opens: VecDeque<StreamOpenJob>,
    completed_static: Vec<StaticDecodeCompletion>,
    refill_requested: bool,
    progress_seq: u64,
}

struct DecodeWorkerShared {
    state: Mutex<DecodeWorkerState>,
    wake: Condvar,
    progress_wake: Condvar,
    latest_epoch: AtomicU64,
    completed_ready: AtomicBool,
    shutdown: AtomicBool,
}

struct WorkerStream {
    epoch: u64,
    decoder: OpenAlStreamDecoder,
    queue: Arc<StreamChunkQueue>,
}

pub(crate) struct AudioDecodeWorker {
    shared: Arc<DecodeWorkerShared>,
    join: Option<JoinHandle<()>>,
}

impl AudioDecodeWorker {
    pub(crate) fn spawn() -> io::Result<Self> {
        let shared = Arc::new(DecodeWorkerShared {
            state: Mutex::new(DecodeWorkerState::default()),
            wake: Condvar::new(),
            progress_wake: Condvar::new(),
            latest_epoch: AtomicU64::new(1),
            completed_ready: AtomicBool::new(false),
            shutdown: AtomicBool::new(false),
        });
        let worker_shared = Arc::clone(&shared);
        let join = thread::Builder::new()
            .name("bbb-client-audio-decode".to_owned())
            .spawn(move || decode_worker_loop(&worker_shared))?;
        Ok(Self {
            shared,
            join: Some(join),
        })
    }

    pub(crate) fn epoch(&self) -> u64 {
        self.shared.latest_epoch.load(Ordering::Acquire)
    }

    pub(crate) fn cancel_all(&self) -> u64 {
        let next_epoch = self
            .shared
            .latest_epoch
            .fetch_add(1, Ordering::AcqRel)
            .wrapping_add(1);
        let mut state = lock_worker_state(&self.shared);
        state.static_jobs.clear();
        state.stream_opens.clear();
        state.completed_static.clear();
        state.refill_requested = true;
        self.shared.completed_ready.store(false, Ordering::Release);
        drop(state);
        self.shared.wake.notify_one();
        next_epoch
    }

    pub(crate) fn submit_static(&self, generation: u64, path: PathBuf) -> bool {
        let epoch = self.epoch();
        let mut state = lock_worker_state(&self.shared);
        if state.static_jobs.len() >= MAX_QUEUED_STATIC_DECODES {
            return false;
        }
        state.static_jobs.push_back(StaticDecodeJob {
            epoch,
            generation,
            path,
        });
        drop(state);
        self.shared.wake.notify_one();
        true
    }

    pub(crate) fn open_stream(&self, path: PathBuf, looping: bool) -> AsyncOpenAlStreamHandle {
        self.open_stream_at(path, looping, 0)
    }

    pub(crate) fn open_stream_at(
        &self,
        path: PathBuf,
        looping: bool,
        start_frame: u64,
    ) -> AsyncOpenAlStreamHandle {
        let epoch = self.epoch();
        let queue = Arc::new(StreamChunkQueue::new());
        let mut state = lock_worker_state(&self.shared);
        state.stream_opens.push_back(StreamOpenJob {
            epoch,
            path: path.clone(),
            looping,
            start_frame,
            queue: Arc::clone(&queue),
        });
        drop(state);
        self.shared.wake.notify_one();
        AsyncOpenAlStreamHandle {
            shared: Arc::clone(&self.shared),
            queue,
            path,
            looping,
            finished: false,
        }
    }

    pub(crate) fn drain_completed_static(&self) -> Vec<StaticDecodeCompletion> {
        if !self.shared.completed_ready.load(Ordering::Acquire) {
            return Vec::new();
        }
        let mut state = lock_worker_state(&self.shared);
        let completed = std::mem::take(&mut state.completed_static);
        self.shared.completed_ready.store(false, Ordering::Release);
        completed
    }

    /// Blocks briefly until the decode worker publishes any static completion or
    /// stream chunk update, or `timeout` elapses. Used only for recording handoff
    /// and deterministic tests.
    pub(crate) fn wait_for_progress(&self, timeout: Duration) {
        let deadline = Instant::now() + timeout;
        let mut state = lock_worker_state(&self.shared);
        let start_seq = state.progress_seq;
        while state.progress_seq == start_seq && !self.shared.shutdown.load(Ordering::Acquire) {
            let Some(remaining) = deadline.checked_duration_since(Instant::now()) else {
                break;
            };
            let (next, result) = self
                .shared
                .progress_wake
                .wait_timeout(state, remaining)
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            state = next;
            if result.timed_out() {
                break;
            }
        }
    }
}

impl Drop for AudioDecodeWorker {
    fn drop(&mut self) {
        self.shared.shutdown.store(true, Ordering::Release);
        self.shared.latest_epoch.fetch_add(1, Ordering::AcqRel);
        let mut state = lock_worker_state(&self.shared);
        state.static_jobs.clear();
        state.stream_opens.clear();
        state.completed_static.clear();
        drop(state);
        self.shared.wake.notify_all();
        self.shared.progress_wake.notify_all();
        if let Some(join) = self.join.take() {
            let _ = join.join();
        }
    }
}

fn lock_worker_state(shared: &DecodeWorkerShared) -> MutexGuard<'_, DecodeWorkerState> {
    shared
        .state
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

fn notify_progress(shared: &DecodeWorkerShared) {
    let mut state = lock_worker_state(shared);
    state.progress_seq = state.progress_seq.wrapping_add(1);
    drop(state);
    shared.progress_wake.notify_all();
}

fn decode_worker_loop(shared: &DecodeWorkerShared) {
    let mut streams: Vec<WorkerStream> = Vec::new();
    loop {
        if shared.shutdown.load(Ordering::Acquire) {
            return;
        }
        let new_opens = {
            let mut state = lock_worker_state(shared);
            state.refill_requested = false;
            std::mem::take(&mut state.stream_opens)
        };
        for open_job in new_opens {
            if shared.shutdown.load(Ordering::Acquire) {
                return;
            }
            if open_job.epoch != shared.latest_epoch.load(Ordering::Acquire)
                || open_job.queue.is_cancelled()
            {
                continue;
            }
            let opened = if open_job.start_frame == 0 {
                OpenAlStreamDecoder::open(&open_job.path, open_job.looping)
            } else {
                OpenAlStreamDecoder::open_at(&open_job.path, open_job.looping, open_job.start_frame)
            };
            match opened {
                Ok(decoder) => streams.push(WorkerStream {
                    epoch: open_job.epoch,
                    decoder,
                    queue: open_job.queue,
                }),
                Err(error) => {
                    let mut q = open_job.queue.lock();
                    q.error = Some(error.to_string());
                    q.eof = true;
                    drop(q);
                    notify_progress(shared);
                }
            }
        }

        let current_epoch = shared.latest_epoch.load(Ordering::Acquire);
        streams.retain(|stream| stream.epoch == current_epoch && !stream.queue.is_cancelled());

        // Prioritize any stream with zero buffered chunks for low playback start latency.
        if let Some(index) = streams
            .iter()
            .position(|stream| stream.queue.buffered_chunks() == 0)
        {
            pump_one_worker_stream(&mut streams, index, shared);
            continue;
        }

        // Service one static decode job before topping up non-starved streams.
        let static_job = {
            let mut state = lock_worker_state(shared);
            state.static_jobs.pop_front()
        };
        if let Some(job) = static_job {
            let epoch = job.epoch;
            if epoch == shared.latest_epoch.load(Ordering::Acquire) {
                let decoded = decode_static_pack_until(&job.path, || {
                    shared.shutdown.load(Ordering::Acquire)
                        || shared.latest_epoch.load(Ordering::Acquire) != epoch
                });
                if let Some(result) = decoded {
                    let mut state = lock_worker_state(shared);
                    if !shared.shutdown.load(Ordering::Acquire)
                        && shared.latest_epoch.load(Ordering::Acquire) == epoch
                    {
                        state.completed_static.push(StaticDecodeCompletion {
                            epoch,
                            generation: job.generation,
                            path: job.path,
                            result,
                        });
                        state.progress_seq = state.progress_seq.wrapping_add(1);
                        shared.completed_ready.store(true, Ordering::Release);
                    }
                    drop(state);
                    shared.progress_wake.notify_all();
                }
            }
            continue;
        }

        // Top up streaming lookahead buffers round-robin up to STREAM_PREFETCH_CHUNKS.
        if let Some(index) = streams
            .iter()
            .position(|stream| stream.queue.buffered_chunks() < STREAM_PREFETCH_CHUNKS)
        {
            let still_alive = pump_one_worker_stream(&mut streams, index, shared);
            if still_alive && streams.len() > 1 {
                let stream = streams.remove(index);
                streams.push(stream);
            }
            continue;
        }

        // No ready work: sleep until new static work, a new stream, refill request, or shutdown.
        let mut state = lock_worker_state(shared);
        while state.static_jobs.is_empty()
            && state.stream_opens.is_empty()
            && !state.refill_requested
            && !shared.shutdown.load(Ordering::Acquire)
            && shared.latest_epoch.load(Ordering::Acquire) == current_epoch
        {
            state = shared
                .wake
                .wait(state)
                .unwrap_or_else(std::sync::PoisonError::into_inner);
        }
    }
}

/// Decodes one chunk for `streams[index]`. Returns `true` if the stream remains
/// active in `streams`, or `false` if it reached EOF, errored, or was cancelled.
fn pump_one_worker_stream(
    streams: &mut Vec<WorkerStream>,
    index: usize,
    shared: &DecodeWorkerShared,
) -> bool {
    let stream = &mut streams[index];
    if stream.queue.is_cancelled() {
        streams.remove(index);
        return false;
    }
    match stream.decoder.read_one_second() {
        Ok(chunk) => {
            if chunk.samples.is_empty() {
                let mut q = stream.queue.lock();
                q.eof = true;
                drop(q);
                streams.remove(index);
                notify_progress(shared);
                false
            } else {
                let mut q = stream.queue.lock();
                q.chunks.push_back(chunk);
                drop(q);
                notify_progress(shared);
                true
            }
        }
        Err(error) => {
            let mut q = stream.queue.lock();
            q.error = Some(error.to_string());
            q.eof = true;
            drop(q);
            streams.remove(index);
            notify_progress(shared);
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn static_decode_completes_off_caller_thread_and_produces_pcm16() {
        let path = unique_temp_path("static-decode");
        crate::preload::write_test_ogg(&path);
        let worker = AudioDecodeWorker::spawn().unwrap();
        assert!(worker.submit_static(5, path.clone()));

        let deadline = Instant::now() + Duration::from_secs(5);
        let completed = loop {
            let batch = worker.drain_completed_static();
            if !batch.is_empty() {
                break batch;
            }
            assert!(Instant::now() < deadline, "static decode timed out");
            worker.wait_for_progress(Duration::from_millis(50));
        };
        assert_eq!(completed.len(), 1);
        let item = &completed[0];
        assert_eq!(item.epoch, worker.epoch());
        assert_eq!(item.generation, 5);
        assert_eq!(item.path, path);
        let decoded = item.result.as_ref().unwrap();
        assert!(decoded.sample_rate > 0);
        assert!((1..=2).contains(&decoded.channels));
        assert!(!decoded.samples.is_empty());

        let next_epoch = worker.cancel_all();
        assert!(next_epoch > item.epoch);
        assert_eq!(worker.epoch(), next_epoch);
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn streaming_worker_prefetches_and_refills_looping_and_oneshot_streams() {
        let path = unique_temp_path("stream-decode");
        crate::preload::write_test_ogg(&path);
        let worker = AudioDecodeWorker::spawn().unwrap();

        // One-shot short silence fixture yields one chunk then EOF.
        let mut oneshot = worker.open_stream(path.clone(), false);
        assert_eq!(oneshot.path(), path.as_path());
        assert!(!oneshot.looping());
        let deadline = Instant::now() + Duration::from_secs(5);
        while oneshot.peek_status() == StreamChunkStatus::Waiting {
            assert!(Instant::now() < deadline, "oneshot stream timed out");
            worker.wait_for_progress(Duration::from_millis(50));
        }
        assert_eq!(oneshot.peek_status(), StreamChunkStatus::Ready);
        let chunk = match oneshot.try_pop_chunk() {
            StreamChunkPop::Chunk(chunk) => chunk,
            _ => panic!("expected first PCM chunk"),
        };
        assert!(chunk.sample_rate > 0);
        assert!((1..=2).contains(&chunk.channels));
        assert!(!chunk.samples.is_empty());
        loop {
            match oneshot.try_pop_chunk() {
                StreamChunkPop::Eof => break,
                StreamChunkPop::Empty => {
                    assert!(Instant::now() < deadline, "oneshot EOF timed out");
                    worker.wait_for_progress(Duration::from_millis(50));
                }
                StreamChunkPop::Chunk(_) => {}
                StreamChunkPop::Error(err) => panic!("unexpected stream error: {err}"),
            }
        }
        assert!(oneshot.is_finished());

        // Looping stream keeps refilling past the 4-chunk lookahead window.
        let mut looping = worker.open_stream(path.clone(), true);
        let mut popped = 0_usize;
        while popped < STREAM_PREFETCH_CHUNKS + 2 {
            match looping.try_pop_chunk() {
                StreamChunkPop::Chunk(chunk) => {
                    assert!(!chunk.samples.is_empty());
                    popped += 1;
                }
                StreamChunkPop::Empty => {
                    assert!(Instant::now() < deadline, "looping refill timed out");
                    worker.wait_for_progress(Duration::from_millis(50));
                }
                StreamChunkPop::Eof => panic!("looping stream should not hit EOF"),
                StreamChunkPop::Error(err) => panic!("unexpected looping error: {err}"),
            }
        }
        drop(looping);
        drop(worker);
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn malformed_stream_reports_error_without_blocking_worker() {
        let bad = unique_temp_path("bad-stream");
        let good = unique_temp_path("good-static");
        std::fs::write(&bad, b"not a valid ogg").unwrap();
        crate::preload::write_test_ogg(&good);
        let worker = AudioDecodeWorker::spawn().unwrap();

        let mut stream = worker.open_stream(bad.clone(), false);
        assert!(worker.submit_static(2, good.clone()));

        let deadline = Instant::now() + Duration::from_secs(5);
        while stream.peek_status() == StreamChunkStatus::Waiting {
            assert!(Instant::now() < deadline, "bad stream status timed out");
            worker.wait_for_progress(Duration::from_millis(50));
        }
        assert!(matches!(stream.peek_status(), StreamChunkStatus::Error(_)));
        assert!(matches!(stream.try_pop_chunk(), StreamChunkPop::Error(_)));
        assert!(stream.is_finished());

        let completed = loop {
            let batch = worker.drain_completed_static();
            if !batch.is_empty() {
                break batch;
            }
            assert!(Instant::now() < deadline, "good static decode timed out");
            worker.wait_for_progress(Duration::from_millis(50));
        };
        assert_eq!(completed.len(), 1);
        assert!(completed[0].result.is_ok());

        drop(worker);
        std::fs::remove_file(bad).unwrap();
        std::fs::remove_file(good).unwrap();
    }

    fn unique_temp_path(label: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("bbb-client-audio-decode-{label}-{nanos}.ogg"))
    }
}
