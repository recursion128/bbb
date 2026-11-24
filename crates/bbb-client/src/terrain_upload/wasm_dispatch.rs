//! Lock-free Web main-thread -> Rayon Web Worker task relay and MPSC result queue.
//!
//! Calling `rayon::spawn` directly from the browser `Window` thread can contend on
//! `rayon_core::sleep::Sleep`'s `std::sync::Mutex<bool>`, which compiles to
//! `memory.atomic.wait32` (`Atomics.wait`) and immediately throws a `TypeError` on
//! the main browser thread. Instead, the `Window` thread pushes tasks onto the
//! audited `bbb_render_types::wasm_sync::AtomicWorkStack` and wakes a dedicated
//! Web Worker via `memory.atomic.notify`, and that Web Worker (where `Atomics.wait`
//! is legal) fans tasks out to the global Rayon worker pool.

use std::sync::atomic::{AtomicBool, AtomicI32, Ordering};

use bbb_render_types::wasm_sync::AtomicWorkStack;
pub use bbb_render_types::wasm_sync::{lock_free_channel, LockFreeReceiver, LockFreeSender};

type RelayTask = Box<dyn FnOnce() + Send + 'static>;

static TASK_STACK: AtomicWorkStack<RelayTask> = AtomicWorkStack::new();
static RELAY_SIGNAL: AtomicI32 = AtomicI32::new(0);
static RELAY_ACTIVE: AtomicBool = AtomicBool::new(false);

/// Returns `true` once the dedicated browser relay Web Worker is running.
#[cfg(target_arch = "wasm32")]
pub fn is_relay_active() -> bool {
    RELAY_ACTIVE.load(Ordering::Acquire)
}

/// Enqueues `task` for the Rayon Web Worker pool without touching any lock or
/// `memory.atomic.wait32` on the calling thread.
pub fn spawn_from_main_thread<F: FnOnce() + Send + 'static>(task: F) {
    if !RELAY_ACTIVE.load(Ordering::Acquire) {
        // Standalone wasm32 unit tests run without the browser relay Worker.
        task();
        return;
    }
    TASK_STACK.push(Box::new(task));
    RELAY_SIGNAL.store(1, Ordering::SeqCst);
    #[cfg(target_feature = "atomics")]
    // SAFETY: `RELAY_SIGNAL` is a valid aligned `AtomicI32` in shared linear
    // memory; waking at most one waiter has no aliasing preconditions.
    unsafe {
        core::arch::wasm32::memory_atomic_notify(RELAY_SIGNAL.as_ptr(), 1);
    }
}

/// Entry point run inside a dedicated browser Web Worker. Drains lock-free tasks
/// submitted by the `Window` thread and dispatches them across the Rayon worker pool.
pub fn run_wasm_worker_relay_loop() {
    RELAY_ACTIVE.store(true, Ordering::Release);
    loop {
        RELAY_SIGNAL.store(0, Ordering::SeqCst);
        let tasks = TASK_STACK.drain_fifo();
        if !tasks.is_empty() {
            for task in tasks {
                rayon::spawn(task);
            }
            continue;
        }
        #[cfg(target_feature = "atomics")]
        // SAFETY: called only on the dedicated relay Web Worker where
        // `Atomics.wait` is permitted; `RELAY_SIGNAL` is a valid aligned `AtomicI32`.
        unsafe {
            let _ = core::arch::wasm32::memory_atomic_wait32(RELAY_SIGNAL.as_ptr(), 0, -1);
        }
        #[cfg(not(target_feature = "atomics"))]
        break;
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{
        atomic::{AtomicUsize, Ordering},
        Arc, Mutex,
    };

    use super::*;

    #[test]
    fn relay_task_stack_preserves_fifo_and_drops_unrun_tasks() {
        let stack = AtomicWorkStack::<RelayTask>::new();
        let order = Arc::new(Mutex::new(Vec::new()));
        let dropped = Arc::new(AtomicUsize::new(0));
        struct DropCounter(Arc<AtomicUsize>);
        impl Drop for DropCounter {
            fn drop(&mut self) {
                self.0.fetch_add(1, Ordering::Relaxed);
            }
        }

        for id in [10u32, 20, 30] {
            let order = Arc::clone(&order);
            let guard = DropCounter(Arc::clone(&dropped));
            stack.push(Box::new(move || {
                let _keep = guard;
                order.lock().unwrap().push(id);
            }));
        }

        let mut tasks = stack.drain_fifo().into_iter();
        tasks.next().unwrap()();
        drop(tasks.next().unwrap());
        tasks.next().unwrap()();

        assert_eq!(*order.lock().unwrap(), vec![10, 30]);
        assert_eq!(dropped.load(Ordering::Relaxed), 3);
    }
}
