use std::{
    io,
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, AtomicU64, Ordering},
        Arc, Condvar, Mutex, MutexGuard,
    },
    thread::{self, JoinHandle},
};

use serde::{Deserialize, Serialize};

use crate::residency::{AudioResidency, PreloadDecodeOutcome};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AudioPreloadRequest {
    pub sound_name: String,
    pub ogg_path: PathBuf,
    pub resource_generation: u64,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AudioPreloadObservation {
    pub resource_generation: u64,
    pub requested: usize,
    pub completed: usize,
    pub failures: usize,
    pub pending: usize,
    pub worker_jobs: u64,
    pub cancelled_jobs: u64,
    pub stale_results: u64,
    pub last_error: Option<String>,
}

pub(crate) struct AudioPreloadBatch {
    pub(crate) token: u64,
    pub(crate) resource_generation: u64,
    pub(crate) requested: usize,
    pub(crate) decoded: usize,
    pub(crate) failures: usize,
    pub(crate) last_error: Option<String>,
    pub(crate) residency: AudioResidency,
}

struct AudioPreloadJob {
    token: u64,
    resource_generation: u64,
    requests: Vec<AudioPreloadRequest>,
}

#[derive(Default)]
struct AudioPreloadState {
    pending: Option<AudioPreloadJob>,
    result: Option<AudioPreloadBatch>,
}

struct AudioPreloadShared {
    state: Mutex<AudioPreloadState>,
    wake: Condvar,
    latest_token: AtomicU64,
    result_ready: AtomicBool,
    shutdown: AtomicBool,
}

pub(crate) struct AudioPreloadWorker {
    shared: Arc<AudioPreloadShared>,
    join: Option<JoinHandle<()>>,
}

impl AudioPreloadWorker {
    pub(crate) fn spawn() -> io::Result<Self> {
        let shared = Arc::new(AudioPreloadShared {
            state: Mutex::new(AudioPreloadState::default()),
            wake: Condvar::new(),
            latest_token: AtomicU64::new(0),
            result_ready: AtomicBool::new(false),
            shutdown: AtomicBool::new(false),
        });
        let worker_shared = Arc::clone(&shared);
        let join = thread::Builder::new()
            .name("bbb-audio-preload".to_string())
            .spawn(move || preload_worker_loop(&worker_shared))?;
        Ok(Self {
            shared,
            join: Some(join),
        })
    }

    pub(crate) fn submit(
        &self,
        resource_generation: u64,
        requests: Vec<AudioPreloadRequest>,
    ) -> u64 {
        let token = self
            .shared
            .latest_token
            .fetch_add(1, Ordering::AcqRel)
            .wrapping_add(1);
        let mut state = lock_state(&self.shared);
        state.pending = Some(AudioPreloadJob {
            token,
            resource_generation,
            requests,
        });
        state.result = None;
        self.shared.result_ready.store(false, Ordering::Release);
        drop(state);
        self.shared.wake.notify_one();
        token
    }

    pub(crate) fn cancel(&self) {
        self.shared.latest_token.fetch_add(1, Ordering::AcqRel);
        let mut state = lock_state(&self.shared);
        state.pending = None;
        state.result = None;
        self.shared.result_ready.store(false, Ordering::Release);
    }

    pub(crate) fn take_result(&self) -> Option<AudioPreloadBatch> {
        if !self.shared.result_ready.load(Ordering::Acquire) {
            return None;
        }
        let mut state = lock_state(&self.shared);
        let result = state.result.take();
        self.shared
            .result_ready
            .store(state.result.is_some(), Ordering::Release);
        result
    }
}

impl Drop for AudioPreloadWorker {
    fn drop(&mut self) {
        self.shared.shutdown.store(true, Ordering::Release);
        self.shared.latest_token.fetch_add(1, Ordering::AcqRel);
        self.shared.wake.notify_one();
        if let Some(join) = self.join.take() {
            let _ = join.join();
        }
    }
}

fn lock_state(shared: &AudioPreloadShared) -> MutexGuard<'_, AudioPreloadState> {
    shared
        .state
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

fn preload_worker_loop(shared: &AudioPreloadShared) {
    loop {
        let job = {
            let mut state = lock_state(shared);
            while state.pending.is_none() && !shared.shutdown.load(Ordering::Acquire) {
                state = shared
                    .wake
                    .wait(state)
                    .unwrap_or_else(std::sync::PoisonError::into_inner);
            }
            if shared.shutdown.load(Ordering::Acquire) {
                return;
            }
            state.pending.take().expect("preload wake has a job")
        };
        let token = job.token;
        let Some(batch) = decode_preload_job(job, || {
            shared.shutdown.load(Ordering::Acquire)
                || shared.latest_token.load(Ordering::Acquire) != token
        }) else {
            continue;
        };
        let mut state = lock_state(shared);
        if !shared.shutdown.load(Ordering::Acquire)
            && shared.latest_token.load(Ordering::Acquire) == token
        {
            state.result = Some(batch);
            shared.result_ready.store(true, Ordering::Release);
        }
    }
}

fn decode_preload_job(
    job: AudioPreloadJob,
    cancelled: impl Fn() -> bool,
) -> Option<AudioPreloadBatch> {
    let requested = job.requests.len();
    let mut residency = AudioResidency::new(job.resource_generation);
    let mut decoded = 0_usize;
    let mut failures = 0_usize;
    let mut last_error = None;
    for request in job.requests {
        if cancelled() {
            return None;
        }
        match residency.preload_static_sound(&request, &cancelled) {
            Ok(PreloadDecodeOutcome::Decoded) => decoded = decoded.saturating_add(1),
            Ok(PreloadDecodeOutcome::Cancelled) => return None,
            Err(error) => {
                failures = failures.saturating_add(1);
                last_error = Some(format!("preload {}: {error:#}", request.sound_name));
            }
        }
    }
    Some(AudioPreloadBatch {
        token: job.token,
        resource_generation: job.resource_generation,
        requested,
        decoded,
        failures,
        last_error,
        residency,
    })
}

#[cfg(test)]
pub(crate) fn write_test_ogg(path: &std::path::Path) {
    use base64::{engine::general_purpose::STANDARD, Engine as _};

    // Self-generated 80-frame mono silence encoded with libvorbis; keeping
    // the text fixture checked in makes decode tests independent of host tools.
    let bytes = STANDARD
        .decode(include_str!("test-silence.ogg.b64").trim())
        .expect("checked-in silence fixture is valid base64");
    std::fs::write(path, bytes).unwrap();
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

    #[test]
    fn worker_decodes_complete_buffers_off_caller_thread_and_returns_one_batch() {
        let path = unique_temp_path("complete-buffer");
        write_test_ogg(&path);
        let worker = AudioPreloadWorker::spawn().unwrap();

        let token = worker.submit(
            7,
            vec![AudioPreloadRequest {
                sound_name: "minecraft:test/preloaded".to_string(),
                ogg_path: path.clone(),
                resource_generation: 7,
            }],
        );
        let batch = wait_for_result(&worker);

        assert_eq!(batch.token, token);
        assert_eq!(batch.resource_generation, 7);
        assert_eq!(batch.requested, 1);
        assert_eq!(batch.decoded, 1);
        assert_eq!(batch.failures, 0);
        assert_eq!(batch.residency.observation().static_buffers, 1);
        assert!(worker.take_result().is_none());
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn latest_job_cancels_or_supersedes_older_generation_without_publishing_it() {
        let old_path = unique_temp_path("old-generation");
        let current_path = unique_temp_path("current-generation");
        write_test_ogg(&old_path);
        write_test_ogg(&current_path);
        let worker = AudioPreloadWorker::spawn().unwrap();

        worker.submit(
            3,
            vec![AudioPreloadRequest {
                sound_name: "minecraft:test/old".to_string(),
                ogg_path: old_path.clone(),
                resource_generation: 3,
            }],
        );
        let current_token = worker.submit(
            4,
            vec![AudioPreloadRequest {
                sound_name: "minecraft:test/current".to_string(),
                ogg_path: current_path.clone(),
                resource_generation: 4,
            }],
        );
        let batch = wait_for_result(&worker);

        assert_eq!(batch.token, current_token);
        assert_eq!(batch.resource_generation, 4);
        assert_eq!(batch.decoded, 1);
        std::fs::remove_file(old_path).unwrap();
        std::fs::remove_file(current_path).unwrap();
    }

    #[test]
    fn malformed_preloads_are_attributable_and_do_not_publish_cache_owners() {
        let path = unique_temp_path("malformed");
        std::fs::write(&path, b"not an audio stream").unwrap();
        let worker = AudioPreloadWorker::spawn().unwrap();
        worker.submit(
            9,
            vec![AudioPreloadRequest {
                sound_name: "minecraft:test/bad".to_string(),
                ogg_path: path.clone(),
                resource_generation: 9,
            }],
        );

        let batch = wait_for_result(&worker);

        assert_eq!(batch.decoded, 0);
        assert_eq!(batch.failures, 1);
        assert!(batch.last_error.unwrap().contains("minecraft:test/bad"));
        assert_eq!(batch.residency.observation().static_buffers, 0);
        std::fs::remove_file(path).unwrap();
    }

    fn wait_for_result(worker: &AudioPreloadWorker) -> AudioPreloadBatch {
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            if let Some(result) = worker.take_result() {
                return result;
            }
            assert!(Instant::now() < deadline, "audio preload worker timed out");
            thread::yield_now();
        }
    }

    fn unique_temp_path(label: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("bbb-audio-preload-{label}-{nanos}.ogg"))
    }
}
