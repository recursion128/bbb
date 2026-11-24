//! Audited non-blocking synchronization primitives for shared-memory WebAssembly.
//!
//! Browsers forbid `memory.atomic.wait32` (`Atomics.wait`) on the `Window` main
//! thread and throw a `TypeError` whenever `std::sync::{Mutex, RwLock}` blocks
//! there. Every Web-reachable spin lock, wait-free oneshot, and intrusive
//! lock-free stack across client crates lives in this single module so all
//! memory orderings, aliasing proofs, and drop invariants are audited and
//! stress-tested in one place.

use std::{
    cell::UnsafeCell,
    fmt,
    future::Future,
    marker::PhantomData,
    ops::{Deref, DerefMut},
    pin::Pin,
    ptr::null_mut,
    sync::{
        atomic::{AtomicBool, AtomicPtr, AtomicU8, AtomicUsize, Ordering},
        Arc,
    },
    task::{Context, Poll, Waker},
};

const RW_WRITER_LOCKED: usize = usize::MAX;
const RW_MAX_READERS: usize = usize::MAX / 2;

/// Non-blocking reader-writer spin lock that never calls `memory.atomic.wait32`.
///
/// Intended only for short, bounded critical sections on shared-memory WASM
/// where `std::sync::RwLock` would trap on the browser `Window` thread.
pub struct SpinRwLock<T> {
    state: AtomicUsize,
    value: UnsafeCell<T>,
}

// SAFETY: `SpinRwLock<T>` transfers ownership of `T` when moved across threads.
unsafe impl<T: Send> Send for SpinRwLock<T> {}
// SAFETY: Concurrent readers observe `&T` (requiring `T: Sync`) and writers
// hold exclusive `&mut T` (requiring `T: Send`), enforced by `state`.
unsafe impl<T: Send + Sync> Sync for SpinRwLock<T> {}

impl<T> SpinRwLock<T> {
    pub const fn new(value: T) -> Self {
        Self {
            state: AtomicUsize::new(0),
            value: UnsafeCell::new(value),
        }
    }

    pub fn try_read(&self) -> Option<SpinReadGuard<'_, T>> {
        let mut cur = self.state.load(Ordering::Relaxed);
        while cur < RW_MAX_READERS {
            match self.state.compare_exchange_weak(
                cur,
                cur + 1,
                Ordering::Acquire,
                Ordering::Relaxed,
            ) {
                Ok(_) => {
                    return Some(SpinReadGuard {
                        lock: self,
                        _marker: PhantomData,
                    });
                }
                Err(actual) => cur = actual,
            }
        }
        None
    }

    pub fn read(&self) -> SpinReadGuard<'_, T> {
        loop {
            if let Some(guard) = self.try_read() {
                return guard;
            }
            std::hint::spin_loop();
        }
    }

    pub fn try_write(&self) -> Option<SpinWriteGuard<'_, T>> {
        self.state
            .compare_exchange(0, RW_WRITER_LOCKED, Ordering::Acquire, Ordering::Relaxed)
            .ok()
            .map(|_| SpinWriteGuard {
                lock: self,
                _marker: PhantomData,
            })
    }

    pub fn write(&self) -> SpinWriteGuard<'_, T> {
        loop {
            while self.state.load(Ordering::Relaxed) != 0 {
                std::hint::spin_loop();
            }
            if self
                .state
                .compare_exchange_weak(0, RW_WRITER_LOCKED, Ordering::Acquire, Ordering::Relaxed)
                .is_ok()
            {
                return SpinWriteGuard {
                    lock: self,
                    _marker: PhantomData,
                };
            }
        }
    }
}

impl<T: Default> Default for SpinRwLock<T> {
    fn default() -> Self {
        Self::new(T::default())
    }
}

impl<T> fmt::Debug for SpinRwLock<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SpinRwLock").finish_non_exhaustive()
    }
}

pub struct SpinReadGuard<'a, T> {
    lock: &'a SpinRwLock<T>,
    _marker: PhantomData<&'a T>,
}

impl<T> Deref for SpinReadGuard<'_, T> {
    type Target = T;

    fn deref(&self) -> &T {
        // SAFETY: `self` accounts for one reader increment in `lock.state`,
        // preventing any writer from acquiring `RW_WRITER_LOCKED`, and
        // `Acquire` synchronized with the previous writer's `Release`.
        unsafe { &*self.lock.value.get() }
    }
}

impl<T> Drop for SpinReadGuard<'_, T> {
    fn drop(&mut self) {
        self.lock.state.fetch_sub(1, Ordering::Release);
    }
}

pub struct SpinWriteGuard<'a, T> {
    lock: &'a SpinRwLock<T>,
    _marker: PhantomData<&'a mut T>,
}

impl<T> Deref for SpinWriteGuard<'_, T> {
    type Target = T;

    fn deref(&self) -> &T {
        // SAFETY: `lock.state == RW_WRITER_LOCKED` guarantees exclusive access
        // and `Acquire` synchronized with all prior readers/writers.
        unsafe { &*self.lock.value.get() }
    }
}

impl<T> DerefMut for SpinWriteGuard<'_, T> {
    fn deref_mut(&mut self) -> &mut T {
        // SAFETY: `lock.state == RW_WRITER_LOCKED` guarantees exclusive access
        // and `Acquire` synchronized with all prior readers/writers.
        unsafe { &mut *self.lock.value.get() }
    }
}

impl<T> Drop for SpinWriteGuard<'_, T> {
    fn drop(&mut self) {
        self.lock.state.store(0, Ordering::Release);
    }
}

/// Non-blocking exclusive spin mutex that never calls `memory.atomic.wait32`.
pub struct SpinMutex<T> {
    locked: AtomicBool,
    value: UnsafeCell<T>,
}

// SAFETY: `SpinMutex<T>` transfers ownership of `T` when moved across threads.
unsafe impl<T: Send> Send for SpinMutex<T> {}
// SAFETY: Only one thread at a time can hold `SpinMutexGuard`, providing
// exclusive `&mut T` access serialized by `Acquire`/`Release` on `locked`.
unsafe impl<T: Send> Sync for SpinMutex<T> {}

impl<T> SpinMutex<T> {
    pub const fn new(value: T) -> Self {
        Self {
            locked: AtomicBool::new(false),
            value: UnsafeCell::new(value),
        }
    }

    pub fn try_lock(&self) -> Option<SpinMutexGuard<'_, T>> {
        self.locked
            .compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
            .ok()
            .map(|_| SpinMutexGuard {
                lock: self,
                _marker: PhantomData,
            })
    }

    pub fn lock(&self) -> SpinMutexGuard<'_, T> {
        loop {
            while self.locked.load(Ordering::Relaxed) {
                std::hint::spin_loop();
            }
            if self
                .locked
                .compare_exchange_weak(false, true, Ordering::Acquire, Ordering::Relaxed)
                .is_ok()
            {
                return SpinMutexGuard {
                    lock: self,
                    _marker: PhantomData,
                };
            }
        }
    }
}

impl<T: Default> Default for SpinMutex<T> {
    fn default() -> Self {
        Self::new(T::default())
    }
}

pub struct SpinMutexGuard<'a, T> {
    lock: &'a SpinMutex<T>,
    _marker: PhantomData<&'a mut T>,
}

impl<T> Deref for SpinMutexGuard<'_, T> {
    type Target = T;

    fn deref(&self) -> &T {
        // SAFETY: `lock.locked == true` grants exclusive access synchronized
        // with the previous holder's `Release`.
        unsafe { &*self.lock.value.get() }
    }
}

impl<T> DerefMut for SpinMutexGuard<'_, T> {
    fn deref_mut(&mut self) -> &mut T {
        // SAFETY: `lock.locked == true` grants exclusive access synchronized
        // with the previous holder's `Release`.
        unsafe { &mut *self.lock.value.get() }
    }
}

impl<T> Drop for SpinMutexGuard<'_, T> {
    fn drop(&mut self) {
        self.lock.locked.store(false, Ordering::Release);
    }
}

const VALUE_EMPTY: u8 = 0;
const VALUE_WRITING: u8 = 1;
const VALUE_READY: u8 = 2;
const VALUE_TAKEN: u8 = 3;

const WAKER_NONE: u8 = 0;
const WAKER_REGISTERING: u8 = 1;
const WAKER_REGISTERED: u8 = 2;
const WAKER_CLOSED: u8 = 3;

/// Error returned by [`WaitSlotReceiver`] when the paired [`WaitSlotSender`]
/// is dropped before completing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WaitSlotCanceled;

impl fmt::Display for WaitSlotCanceled {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("wait slot sender dropped before completion")
    }
}

impl std::error::Error for WaitSlotCanceled {}

struct WaitSlotInner<T> {
    value_state: AtomicU8,
    waker_state: AtomicU8,
    value: UnsafeCell<Option<T>>,
    waker: UnsafeCell<Option<Waker>>,
}

// SAFETY: `value` is written at most once by the sole `WaitSlotSender` while
// `value_state == VALUE_WRITING`, published via `SeqCst` (`VALUE_READY`), and
// read at most once by the sole `WaitSlotReceiver` via `AcqRel` transition to
// `VALUE_TAKEN`. `waker` is written only by the sole `WaitSlotReceiver` while
// `waker_state == WAKER_REGISTERING` and taken by the sole `WaitSlotSender`
// only if `waker_state.swap(WAKER_CLOSED, SeqCst)` observes `WAKER_REGISTERED`.
unsafe impl<T: Send> Send for WaitSlotInner<T> {}
unsafe impl<T: Send> Sync for WaitSlotInner<T> {}

impl<T> WaitSlotInner<T> {
    fn finish(&self, value: Option<T>) {
        if self
            .value_state
            .compare_exchange(
                VALUE_EMPTY,
                VALUE_WRITING,
                Ordering::AcqRel,
                Ordering::Acquire,
            )
            .is_err()
        {
            return;
        }
        // SAFETY: `value_state == VALUE_WRITING` grants the sole sender
        // exclusive write access to `self.value`.
        unsafe {
            *self.value.get() = value;
        }
        // Publish `value` before observing `waker_state` (Dekker pair with `poll`).
        self.value_state.store(VALUE_READY, Ordering::SeqCst);
        if self.waker_state.swap(WAKER_CLOSED, Ordering::SeqCst) == WAKER_REGISTERED {
            // SAFETY: observing `WAKER_REGISTERED` transfers exclusive
            // ownership of `self.waker` from the receiver to the sender.
            if let Some(waker) = unsafe { (*self.waker.get()).take() } {
                waker.wake();
            }
        }
    }

    fn take_ready(&self) -> Result<T, WaitSlotCanceled> {
        let prev = self
            .value_state
            .compare_exchange(
                VALUE_READY,
                VALUE_TAKEN,
                Ordering::AcqRel,
                Ordering::Acquire,
            )
            .expect("WaitSlotReceiver polled after already completing");
        debug_assert_eq!(prev, VALUE_READY);
        // SAFETY: `AcqRel` transition from `VALUE_READY` to `VALUE_TAKEN`
        // synchronizes with the sender's `SeqCst` publish and grants sole read
        // ownership to the receiver.
        unsafe { (*self.value.get()).take() }.ok_or(WaitSlotCanceled)
    }
}

/// Producer half of a wait-free single-producer / single-consumer async oneshot.
pub struct WaitSlotSender<T> {
    inner: Arc<WaitSlotInner<T>>,
    completed: bool,
}

impl<T> WaitSlotSender<T> {
    pub fn complete(mut self, value: T) {
        self.completed = true;
        self.inner.finish(Some(value));
    }
}

impl<T> Drop for WaitSlotSender<T> {
    fn drop(&mut self) {
        if !self.completed {
            self.inner.finish(None);
        }
    }
}

/// Consumer future half of a wait-free single-producer / single-consumer async oneshot.
pub struct WaitSlotReceiver<T> {
    inner: Arc<WaitSlotInner<T>>,
}

impl<T> Future for WaitSlotReceiver<T> {
    type Output = Result<T, WaitSlotCanceled>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let inner = &self.inner;
        if inner.value_state.load(Ordering::SeqCst) == VALUE_READY {
            return Poll::Ready(inner.take_ready());
        }

        let acquired = inner
            .waker_state
            .compare_exchange(
                WAKER_NONE,
                WAKER_REGISTERING,
                Ordering::SeqCst,
                Ordering::SeqCst,
            )
            .is_ok()
            || inner
                .waker_state
                .compare_exchange(
                    WAKER_REGISTERED,
                    WAKER_REGISTERING,
                    Ordering::SeqCst,
                    Ordering::SeqCst,
                )
                .is_ok();

        if !acquired {
            // Producer already closed the waker slot after publishing `VALUE_READY`.
            debug_assert_eq!(inner.value_state.load(Ordering::SeqCst), VALUE_READY);
            return Poll::Ready(inner.take_ready());
        }

        // SAFETY: `WAKER_REGISTERING` grants the sole receiver exclusive write
        // access to `inner.waker`.
        unsafe {
            *inner.waker.get() = Some(cx.waker().clone());
        }

        // Publish the waker before re-checking `value_state` (Dekker pair with `finish`).
        if inner
            .waker_state
            .compare_exchange(
                WAKER_REGISTERING,
                WAKER_REGISTERED,
                Ordering::SeqCst,
                Ordering::SeqCst,
            )
            .is_err()
        {
            // Producer transitioned `WAKER_REGISTERING -> WAKER_CLOSED` and will
            // never touch `inner.waker`, so clear it now and take the ready value.
            // SAFETY: sole receiver still owns `inner.waker` after observing the
            // sender's `WAKER_CLOSED` transition from `WAKER_REGISTERING`.
            unsafe {
                let _ = (*inner.waker.get()).take();
            }
            return Poll::Ready(inner.take_ready());
        }

        if inner.value_state.load(Ordering::SeqCst) == VALUE_READY {
            // Producer published `VALUE_READY`; do not touch `inner.waker` again
            // because the producer may concurrently wake it via `WAKER_CLOSED`.
            return Poll::Ready(inner.take_ready());
        }

        Poll::Pending
    }
}

/// Creates a wait-free single-producer / single-consumer async oneshot channel
/// that never invokes `memory.atomic.wait32`.
pub fn wait_slot<T>() -> (WaitSlotSender<T>, WaitSlotReceiver<T>) {
    let inner = Arc::new(WaitSlotInner {
        value_state: AtomicU8::new(VALUE_EMPTY),
        waker_state: AtomicU8::new(WAKER_NONE),
        value: UnsafeCell::new(None),
        waker: UnsafeCell::new(None),
    });
    (
        WaitSlotSender {
            inner: Arc::clone(&inner),
            completed: false,
        },
        WaitSlotReceiver { inner },
    )
}

struct WorkNode<T> {
    value: T,
    next: *mut WorkNode<T>,
}

/// Lock-free multi-producer intrusive atomic work stack with whole-batch FIFO drain.
///
/// Consumers always detach the entire chain with `swap(null, AcqRel)` rather
/// than popping individual nodes, eliminating the classic Treiber-stack ABA
/// hazard while keeping producer `push` completely wait-free aside from CAS retry.
pub struct AtomicWorkStack<T> {
    head: AtomicPtr<WorkNode<T>>,
}

// SAFETY: Every pushed `T` is uniquely owned by the stack; `push` publishes
// ownership with `Ordering::Release` and draining or dropping reclaims
// ownership with `Ordering::AcqRel` or `&mut self`.
unsafe impl<T: Send> Send for AtomicWorkStack<T> {}
unsafe impl<T: Send> Sync for AtomicWorkStack<T> {}

impl<T> AtomicWorkStack<T> {
    pub const fn new() -> Self {
        Self {
            head: AtomicPtr::new(null_mut()),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.head.load(Ordering::Acquire).is_null()
    }

    pub fn push(&self, value: T) {
        let node = Box::into_raw(Box::new(WorkNode {
            value,
            next: null_mut(),
        }));
        let mut head = self.head.load(Ordering::Relaxed);
        loop {
            // SAFETY: `node` is exclusively owned by this thread until the CAS
            // below publishes it into `self.head` with `Ordering::Release`.
            unsafe {
                (*node).next = head;
            }
            match self
                .head
                .compare_exchange_weak(head, node, Ordering::Release, Ordering::Relaxed)
            {
                Ok(_) => return,
                Err(actual) => head = actual,
            }
        }
    }

    /// Detaches all currently queued items and returns them in FIFO push order.
    pub fn drain_fifo(&self) -> Vec<T> {
        let mut ptr = self.head.swap(null_mut(), Ordering::AcqRel);
        let mut items = Vec::new();
        while !ptr.is_null() {
            // SAFETY: `swap(null, AcqRel)` transferred exclusive ownership of
            // the detached chain; every node was allocated by `Box::into_raw`
            // in `push` and published with `Ordering::Release`.
            let node = unsafe { Box::from_raw(ptr) };
            ptr = node.next;
            items.push(node.value);
        }
        items.reverse();
        items
    }

    /// Pops one item in FIFO order, reusing `ready` as a LIFO buffer for any
    /// additional items detached in the same batch. When only one item is
    /// queued, it is returned directly without allocating `ready`.
    pub fn pop_fifo_into(&self, ready: &mut Vec<T>) -> Option<T> {
        if let Some(value) = ready.pop() {
            return Some(value);
        }
        let mut ptr = self.head.swap(null_mut(), Ordering::AcqRel);
        if ptr.is_null() {
            return None;
        }
        loop {
            // SAFETY: `swap(null, AcqRel)` transferred exclusive ownership of
            // the detached chain; every node was allocated by `Box::into_raw`
            // in `push` and published with `Ordering::Release`.
            let node = unsafe { Box::from_raw(ptr) };
            let next = node.next;
            if next.is_null() {
                return Some(node.value);
            }
            ready.push(node.value);
            ptr = next;
        }
    }
}

impl<T> Default for AtomicWorkStack<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> Drop for AtomicWorkStack<T> {
    fn drop(&mut self) {
        let head = self.head.get_mut();
        while !head.is_null() {
            let ptr = *head;
            // SAFETY: `&mut self` guarantees exclusive access to the stack;
            // every remaining node came from `Box::into_raw` in `push`.
            let node = unsafe { Box::from_raw(ptr) };
            *head = node.next;
            drop(node);
        }
    }
}

/// Lock-free multi-producer sender backed by [`AtomicWorkStack`].
pub struct LockFreeSender<T> {
    stack: Arc<AtomicWorkStack<T>>,
}

impl<T> Clone for LockFreeSender<T> {
    fn clone(&self) -> Self {
        Self {
            stack: Arc::clone(&self.stack),
        }
    }
}

impl<T> fmt::Debug for LockFreeSender<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("LockFreeSender").finish_non_exhaustive()
    }
}

impl<T: Send> LockFreeSender<T> {
    pub fn send(&self, value: T) -> Result<(), T> {
        self.stack.push(value);
        Ok(())
    }
}

/// Lock-free single-consumer FIFO receiver backed by [`AtomicWorkStack`].
pub struct LockFreeReceiver<T> {
    stack: Arc<AtomicWorkStack<T>>,
    ready: Vec<T>,
}

impl<T> fmt::Debug for LockFreeReceiver<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("LockFreeReceiver")
            .field("ready_len", &self.ready.len())
            .finish_non_exhaustive()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TryRecvEmpty;

impl<T: Send> LockFreeReceiver<T> {
    pub fn try_recv(&mut self) -> Result<T, TryRecvEmpty> {
        self.stack
            .pop_fifo_into(&mut self.ready)
            .ok_or(TryRecvEmpty)
    }
}

/// Creates a lock-free MPSC FIFO channel suitable for `Window` <-> Web Worker
/// communication without `memory.atomic.wait32`.
pub fn lock_free_channel<T>() -> (LockFreeSender<T>, LockFreeReceiver<T>) {
    let stack = Arc::new(AtomicWorkStack::new());
    (
        LockFreeSender {
            stack: Arc::clone(&stack),
        },
        LockFreeReceiver {
            stack,
            ready: Vec::new(),
        },
    )
}

#[cfg(test)]
mod tests {
    use std::{
        sync::{
            atomic::{AtomicBool, AtomicUsize, Ordering},
            Arc,
        },
        task::{Context, Poll, Wake, Waker},
        thread,
    };

    use super::*;

    struct FlagWaker {
        woken: AtomicBool,
        count: AtomicUsize,
    }

    impl FlagWaker {
        fn new() -> Arc<Self> {
            Arc::new(Self {
                woken: AtomicBool::new(false),
                count: AtomicUsize::new(0),
            })
        }
    }

    impl Wake for FlagWaker {
        fn wake(self: Arc<Self>) {
            self.woken.store(true, Ordering::SeqCst);
            self.count.fetch_add(1, Ordering::SeqCst);
        }

        fn wake_by_ref(self: &Arc<Self>) {
            self.woken.store(true, Ordering::SeqCst);
            self.count.fetch_add(1, Ordering::SeqCst);
        }
    }

    #[test]
    fn spin_rwlock_enforces_reader_and_writer_exclusion() {
        let lock = Arc::new(SpinRwLock::new(0_usize));
        {
            let r1 = lock.read();
            let r2 = lock.try_read().expect("multiple readers allowed");
            assert_eq!(*r1, 0);
            assert_eq!(*r2, 0);
            assert!(lock.try_write().is_none());
        }
        {
            let mut w = lock.try_write().expect("writer allowed once readers drop");
            *w = 42;
            assert!(lock.try_read().is_none());
            assert!(lock.try_write().is_none());
        }
        assert_eq!(*lock.read(), 42);

        let mut handles = Vec::new();
        for _ in 0..4 {
            let lock = Arc::clone(&lock);
            handles.push(thread::spawn(move || {
                for _ in 0..500 {
                    *lock.write() += 1;
                }
            }));
        }
        for _ in 0..4 {
            let lock = Arc::clone(&lock);
            handles.push(thread::spawn(move || {
                let mut prev = 0;
                for _ in 0..500 {
                    let v = *lock.read();
                    assert!(v >= prev);
                    prev = v;
                }
            }));
        }
        for handle in handles {
            handle.join().unwrap();
        }
        assert_eq!(*lock.read(), 42 + 4 * 500);
    }

    #[test]
    fn spin_mutex_serializes_concurrent_updates() {
        let mutex = Arc::new(SpinMutex::new(0_usize));
        {
            let guard = mutex.try_lock().expect("unlocked mutex");
            assert!(mutex.try_lock().is_none());
            drop(guard);
        }
        let mut handles = Vec::new();
        for _ in 0..4 {
            let mutex = Arc::clone(&mutex);
            handles.push(thread::spawn(move || {
                for _ in 0..1_000 {
                    *mutex.lock() += 1;
                }
            }));
        }
        for handle in handles {
            handle.join().unwrap();
        }
        assert_eq!(*mutex.lock(), 4_000);
    }

    #[test]
    fn wait_slot_delivers_before_poll_after_poll_and_on_cancel() {
        let flag = FlagWaker::new();
        let waker = Waker::from(Arc::clone(&flag));
        let mut cx = Context::from_waker(&waker);

        // Complete before poll.
        let (tx, mut rx) = wait_slot::<u32>();
        tx.complete(99);
        assert_eq!(Pin::new(&mut rx).poll(&mut cx), Poll::Ready(Ok(99)));
        assert!(!flag.woken.load(Ordering::SeqCst));

        // Poll before complete, then update waker and complete.
        let (tx, mut rx) = wait_slot::<u32>();
        assert_eq!(Pin::new(&mut rx).poll(&mut cx), Poll::Pending);
        let flag2 = FlagWaker::new();
        let waker2 = Waker::from(Arc::clone(&flag2));
        let mut cx2 = Context::from_waker(&waker2);
        assert_eq!(Pin::new(&mut rx).poll(&mut cx2), Poll::Pending);
        tx.complete(123);
        assert!(flag2.woken.load(Ordering::SeqCst));
        assert_eq!(Pin::new(&mut rx).poll(&mut cx2), Poll::Ready(Ok(123)));

        // Sender dropped without completing wakes receiver with WaitSlotCanceled.
        let flag3 = FlagWaker::new();
        let waker3 = Waker::from(Arc::clone(&flag3));
        let mut cx3 = Context::from_waker(&waker3);
        let (tx, mut rx) = wait_slot::<u32>();
        assert_eq!(Pin::new(&mut rx).poll(&mut cx3), Poll::Pending);
        drop(tx);
        assert!(flag3.woken.load(Ordering::SeqCst));
        assert_eq!(
            Pin::new(&mut rx).poll(&mut cx3),
            Poll::Ready(Err(WaitSlotCanceled))
        );
    }

    #[test]
    fn wait_slot_concurrent_poll_complete_race_never_loses_wakeup() {
        for iter in 0..500_u32 {
            let (tx, mut rx) = wait_slot::<u32>();
            let flag = FlagWaker::new();
            let waker = Waker::from(Arc::clone(&flag));
            let producer = thread::spawn(move || {
                tx.complete(iter);
            });
            let mut cx = Context::from_waker(&waker);
            let value = loop {
                match Pin::new(&mut rx).poll(&mut cx) {
                    Poll::Ready(res) => break res.unwrap(),
                    Poll::Pending => {
                        while !flag.woken.swap(false, Ordering::SeqCst) {
                            thread::yield_now();
                        }
                    }
                }
            };
            producer.join().unwrap();
            assert_eq!(value, iter);
        }
    }

    #[test]
    fn atomic_work_stack_preserves_fifo_concurrent_pushes_and_drops_remaining() {
        let dropped = Arc::new(AtomicUsize::new(0));
        struct DropTracker(u32, Arc<AtomicUsize>);
        impl Drop for DropTracker {
            fn drop(&mut self) {
                self.1.fetch_add(1, Ordering::Relaxed);
            }
        }

        let stack = AtomicWorkStack::new();
        assert!(stack.is_empty());
        stack.push(DropTracker(1, Arc::clone(&dropped)));
        stack.push(DropTracker(2, Arc::clone(&dropped)));
        stack.push(DropTracker(3, Arc::clone(&dropped)));
        assert!(!stack.is_empty());

        let drained: Vec<u32> = stack.drain_fifo().into_iter().map(|t| t.0).collect();
        assert_eq!(drained, vec![1, 2, 3]);
        assert_eq!(dropped.load(Ordering::Relaxed), 3);

        // Un-drained items must run Drop when the stack is dropped.
        stack.push(DropTracker(4, Arc::clone(&dropped)));
        stack.push(DropTracker(5, Arc::clone(&dropped)));
        drop(stack);
        assert_eq!(dropped.load(Ordering::Relaxed), 5);

        // Multi-producer concurrent stress test.
        let shared = Arc::new(AtomicWorkStack::<usize>::new());
        let mut producers = Vec::new();
        for t in 0..4_usize {
            let shared = Arc::clone(&shared);
            producers.push(thread::spawn(move || {
                for i in 0..500_usize {
                    shared.push(t * 1_000 + i);
                }
            }));
        }
        let mut received = Vec::new();
        for producer in producers {
            producer.join().unwrap();
            received.extend(shared.drain_fifo());
        }
        received.extend(shared.drain_fifo());
        received.sort_unstable();
        assert_eq!(received.len(), 2_000);
        received.dedup();
        assert_eq!(received.len(), 2_000);
    }

    #[test]
    fn lock_free_channel_delivers_fifo_and_skips_ready_buffer_for_single_items() {
        let (tx, mut rx) = lock_free_channel::<u32>();
        tx.send(7).unwrap();
        assert_eq!(rx.try_recv(), Ok(7));
        assert_eq!(rx.ready.capacity(), 0);
        assert_eq!(rx.try_recv(), Err(TryRecvEmpty));

        tx.send(1).unwrap();
        tx.send(2).unwrap();
        tx.send(3).unwrap();
        assert_eq!(rx.try_recv(), Ok(1));
        assert_eq!(rx.try_recv(), Ok(2));
        assert_eq!(rx.try_recv(), Ok(3));
        assert_eq!(rx.try_recv(), Err(TryRecvEmpty));
    }
}
