//! Spinlock-guarded cell for tiny internal caches that must never invoke
//! `std::sync::Mutex` (`Atomics.wait`) on `wasm32` main-thread callers.

use core::cell::UnsafeCell;
use core::hint::spin_loop;
use core::sync::atomic::{AtomicBool, Ordering};

pub(crate) struct SpinLock<T> {
    locked: AtomicBool,
    value: UnsafeCell<T>,
}

// SAFETY: Exclusive access is enforced by the `Acquire` / `Release` CAS lock in
// `with`, without blocking via `Atomics.wait` on wasm32 main threads.
unsafe impl<T: Send> Sync for SpinLock<T> {}

struct SpinGuard<'a>(&'a AtomicBool);

impl Drop for SpinGuard<'_> {
    fn drop(&mut self) {
        self.0.store(false, Ordering::Release);
    }
}

impl<T> SpinLock<T> {
    pub(crate) const fn new(value: T) -> Self {
        Self {
            locked: AtomicBool::new(false),
            value: UnsafeCell::new(value),
        }
    }

    pub(crate) fn with<R>(&self, f: impl FnOnce(&mut T) -> R) -> R {
        while self
            .locked
            .compare_exchange_weak(false, true, Ordering::Acquire, Ordering::Relaxed)
            .is_err()
        {
            spin_loop();
        }
        let _guard = SpinGuard(&self.locked);
        // SAFETY: Exclusive access is guaranteed by `_guard` on `self.locked`.
        let value = unsafe { &mut *self.value.get() };
        f(value)
    }
}
