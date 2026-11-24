//! MCP sessions and their resource subscriptions.
//!
//! Vanilla owner: `Connection` and `JsonRpcNotificationService`, where one
//! authenticated WebSocket is the session and every connected client receives
//! every notification. MCP separates the two: a session outlives any single
//! HTTP request, and a client receives only the resources it subscribed to.
//!
//! Everything here is bounded. A session count, a per-session subscription
//! count, a per-session event queue and an idle lifetime all have ceilings,
//! because a management endpoint an agent can reach must not be able to grow
//! or permanently wedge the server by being talked to and abandoned.
//!
//! Capacity & eviction policy:
//! - Any idle session (`!stream_open && inflight == 0`) untouched for at least
//!   [`SessionLimits::idle_ttl`] (default [`DEFAULT_SESSION_IDLE_TTL`], 10 min)
//!   is expired and reaped on lookup or before admitting a new `initialize`.
//! - When `sessions.len() >= max_sessions` after reaping expired sessions,
//!   `create` evicts the single oldest idle session (`!stream_open && inflight == 0`
//!   ordered by `(last_activity, activity_seq)`).
//! - Sessions with an open SSE event stream (`stream_open`) or an in-flight
//!   JSON-RPC request (`inflight > 0`) are never expired or evicted; `create`
//!   returns [`SessionError::TooManySessions`] (`HTTP 503`) only when
//!   `max_sessions == 0` or all `max_sessions` slots are actively busy.

use std::{
    collections::{BTreeSet, HashMap, VecDeque},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex, PoisonError,
    },
    time::{Duration, Instant},
};

use serde_json::Value;
use tokio::sync::Notify;

use crate::{auth::Principal, host::ScopeLadder};

/// Default idle lifetime before an inactive session expires (10 minutes).
pub const DEFAULT_SESSION_IDLE_TTL: Duration = Duration::from_secs(10 * 60);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SessionLimits {
    pub max_sessions: usize,
    pub max_subscriptions: usize,
    /// How many undelivered notifications one session may hold.
    pub event_queue: usize,
    /// How long an idle session (no open event stream and no in-flight request)
    /// may sit untouched before expiring.
    pub idle_ttl: Duration,
}

impl Default for SessionLimits {
    fn default() -> Self {
        Self {
            max_sessions: 32,
            max_subscriptions: 32,
            event_queue: 64,
            idle_ttl: DEFAULT_SESSION_IDLE_TTL,
        }
    }
}

/// The queue one session's event stream drains.
#[derive(Debug, Default)]
pub struct SessionEvents {
    queue: Mutex<VecDeque<Value>>,
    notify: Notify,
    closed: AtomicBool,
}

impl SessionEvents {
    /// Queues one message, dropping the oldest when the queue is full.
    ///
    /// Dropping the oldest is safe for what this queue carries: a resource
    /// update notification says only *that* a resource changed, and a client
    /// that missed one and receives a later one re-reads the same current
    /// value. Blocking the tick's fan-out on a slow reader would not be safe.
    pub fn push(&self, message: Value, capacity: usize) -> bool {
        let mut queue = self.queue.lock().unwrap_or_else(PoisonError::into_inner);
        let dropped = if queue.len() >= capacity {
            queue.pop_front();
            true
        } else {
            false
        };
        queue.push_back(message);
        drop(queue);
        self.notify.notify_one();
        dropped
    }

    pub fn take(&self) -> Vec<Value> {
        let mut queue = self.queue.lock().unwrap_or_else(PoisonError::into_inner);
        queue.drain(..).collect()
    }

    pub fn notified(&self) -> tokio::sync::futures::Notified<'_> {
        self.notify.notified()
    }

    pub fn is_closed(&self) -> bool {
        self.closed.load(Ordering::Acquire)
    }

    pub async fn wait(&self) {
        self.notify.notified().await;
    }

    /// Marks the stream closed and wakes any waiter so it can exit immediately.
    pub fn wake(&self) {
        self.closed.store(true, Ordering::Release);
        self.notify.notify_waiters();
    }
}

#[derive(Debug)]
struct SessionEntry<L: ScopeLadder> {
    principal: Principal<L>,
    protocol_version: &'static str,
    initialized: bool,
    subscriptions: BTreeSet<&'static str>,
    events: Arc<SessionEvents>,
    stream_open: bool,
    inflight: usize,
    last_activity: Instant,
    activity_seq: u64,
}

impl<L: ScopeLadder> SessionEntry<L> {
    fn is_active(&self) -> bool {
        self.stream_open || self.inflight > 0
    }

    fn is_expired(&self, now: Instant, idle_ttl: Duration) -> bool {
        !self.is_active() && now.saturating_duration_since(self.last_activity) >= idle_ttl
    }

    fn touch(&mut self, now: Instant, seq: u64) {
        self.last_activity = now;
        self.activity_seq = seq;
    }
}

#[derive(Debug)]
struct SessionState<L: ScopeLadder> {
    entries: HashMap<String, SessionEntry<L>>,
    next_seq: u64,
}

impl<L: ScopeLadder> Default for SessionState<L> {
    fn default() -> Self {
        Self {
            entries: HashMap::new(),
            next_seq: 0,
        }
    }
}

impl<L: ScopeLadder> SessionState<L> {
    fn next_seq(&mut self) -> u64 {
        let seq = self.next_seq;
        self.next_seq = self.next_seq.wrapping_add(1);
        seq
    }

    fn reap_expired(&mut self, now: Instant, idle_ttl: Duration) -> Vec<Arc<SessionEvents>> {
        let mut woken = Vec::new();
        self.entries.retain(|_, entry| {
            if entry.is_expired(now, idle_ttl) {
                woken.push(Arc::clone(&entry.events));
                false
            } else {
                true
            }
        });
        woken
    }

    fn evict_oldest_idle(&mut self) -> Option<Arc<SessionEvents>> {
        let oldest_id = self
            .entries
            .iter()
            .filter(|(_, entry)| !entry.is_active())
            .min_by_key(|(_, entry)| (entry.last_activity, entry.activity_seq))
            .map(|(id, _)| id.clone())?;
        self.entries.remove(&oldest_id).map(|entry| entry.events)
    }
}

fn wake_all(events: Vec<Arc<SessionEvents>>) {
    for stream in events {
        stream.wake();
    }
}

/// A read-only view of one session, taken while the registry lock is held.
#[derive(Debug, Clone)]
pub struct SessionView<L: ScopeLadder> {
    pub id: String,
    pub principal: Principal<L>,
    pub protocol_version: &'static str,
    pub initialized: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionError {
    /// The server already holds as many active sessions as it will.
    TooManySessions,
    /// The named session is unknown or was terminated. MCP answers this with
    /// `404`, which tells the client to start a new session.
    Unknown,
    /// The session has not yet received `notifications/initialized`.
    NotInitialized,
    /// This session already has an event stream open.
    StreamAlreadyOpen,
    TooManySubscriptions,
}

/// RAII guard marking an in-flight JSON-RPC request on a session so concurrent
/// `initialize` calls cannot evict a session whose call is actively running.
pub struct SessionRequestGuard<'registry, L: ScopeLadder> {
    registry: &'registry SessionRegistry<L>,
    id: String,
}

impl<L: ScopeLadder> Drop for SessionRequestGuard<'_, L> {
    fn drop(&mut self) {
        self.registry.finish_request(&self.id);
    }
}

pub struct SessionRegistry<L: ScopeLadder> {
    limits: SessionLimits,
    state: Mutex<SessionState<L>>,
}

impl<L: ScopeLadder> SessionRegistry<L> {
    pub fn new(limits: SessionLimits) -> Self {
        Self {
            limits,
            state: Mutex::new(SessionState::default()),
        }
    }

    pub fn with_idle_ttl(mut self, idle_ttl: Duration) -> Self {
        self.limits.idle_ttl = idle_ttl;
        self
    }

    pub fn limits(&self) -> SessionLimits {
        self.limits
    }

    /// Reaps idle sessions whose inactivity has reached `idle_ttl`.
    pub fn reap_expired(&self) -> usize {
        let now = Instant::now();
        let woken = {
            let mut state = self.state.lock().unwrap_or_else(PoisonError::into_inner);
            state.reap_expired(now, self.limits.idle_ttl)
        };
        let reaped = woken.len();
        wake_all(woken);
        reaped
    }

    pub fn create(
        &self,
        id: String,
        principal: Principal<L>,
        protocol_version: &'static str,
    ) -> Result<SessionView<L>, SessionError> {
        if self.limits.max_sessions == 0 {
            return Err(SessionError::TooManySessions);
        }
        let now = Instant::now();
        let mut woken = {
            let mut state = self.state.lock().unwrap_or_else(PoisonError::into_inner);
            let mut woken = state.reap_expired(now, self.limits.idle_ttl);
            while state.entries.len() >= self.limits.max_sessions {
                let Some(evicted) = state.evict_oldest_idle() else {
                    drop(state);
                    wake_all(woken);
                    return Err(SessionError::TooManySessions);
                };
                woken.push(evicted);
            }
            let seq = state.next_seq();
            state.entries.insert(
                id.clone(),
                SessionEntry {
                    principal: principal.clone(),
                    protocol_version,
                    initialized: false,
                    subscriptions: BTreeSet::new(),
                    events: Arc::new(SessionEvents::default()),
                    stream_open: false,
                    inflight: 0,
                    last_activity: now,
                    activity_seq: seq,
                },
            );
            woken
        };
        wake_all(std::mem::take(&mut woken));
        Ok(SessionView {
            id,
            principal,
            protocol_version,
            initialized: false,
        })
    }

    pub fn view(&self, id: &str) -> Option<SessionView<L>> {
        let now = Instant::now();
        let (view, woken) = {
            let mut state = self.state.lock().unwrap_or_else(PoisonError::into_inner);
            let woken = state.reap_expired(now, self.limits.idle_ttl);
            let view = state.entries.get(id).map(|entry| SessionView {
                id: id.to_string(),
                principal: entry.principal.clone(),
                protocol_version: entry.protocol_version,
                initialized: entry.initialized,
            });
            (view, woken)
        };
        wake_all(woken);
        view
    }

    pub fn touch(&self, id: &str) -> bool {
        let now = Instant::now();
        let (found, woken) = {
            let mut state = self.state.lock().unwrap_or_else(PoisonError::into_inner);
            let woken = state.reap_expired(now, self.limits.idle_ttl);
            let seq = state.next_seq();
            let found = if let Some(entry) = state.entries.get_mut(id) {
                entry.touch(now, seq);
                true
            } else {
                false
            };
            (found, woken)
        };
        wake_all(woken);
        found
    }

    pub fn mark_initialized(&self, id: &str) -> bool {
        let now = Instant::now();
        let (marked, woken) = {
            let mut state = self.state.lock().unwrap_or_else(PoisonError::into_inner);
            let woken = state.reap_expired(now, self.limits.idle_ttl);
            let seq = state.next_seq();
            let marked = if let Some(entry) = state.entries.get_mut(id) {
                entry.initialized = true;
                entry.touch(now, seq);
                true
            } else {
                false
            };
            (marked, woken)
        };
        wake_all(woken);
        marked
    }

    /// Enters an initialized session for one in-flight request, keeping it
    /// protected from idle expiry and LRU eviction until the guard drops.
    pub fn enter_request(&self, id: &str) -> Result<SessionRequestGuard<'_, L>, SessionError> {
        let now = Instant::now();
        let (result, woken) = {
            let mut state = self.state.lock().unwrap_or_else(PoisonError::into_inner);
            let woken = state.reap_expired(now, self.limits.idle_ttl);
            let seq = state.next_seq();
            let result = match state.entries.get_mut(id) {
                None => Err(SessionError::Unknown),
                Some(entry) if !entry.initialized => Err(SessionError::NotInitialized),
                Some(entry) => {
                    entry.inflight += 1;
                    entry.touch(now, seq);
                    Ok(())
                }
            };
            (result, woken)
        };
        wake_all(woken);
        result.map(|()| SessionRequestGuard {
            registry: self,
            id: id.to_string(),
        })
    }

    fn finish_request(&self, id: &str) {
        let now = Instant::now();
        let mut state = self.state.lock().unwrap_or_else(PoisonError::into_inner);
        let seq = state.next_seq();
        if let Some(entry) = state.entries.get_mut(id) {
            entry.inflight = entry.inflight.saturating_sub(1);
            entry.touch(now, seq);
        }
    }

    pub fn remove(&self, id: &str) -> bool {
        let removed = {
            let mut state = self.state.lock().unwrap_or_else(PoisonError::into_inner);
            state.entries.remove(id)
        };
        match removed {
            Some(entry) => {
                entry.events.wake();
                true
            }
            None => false,
        }
    }

    pub fn len(&self) -> usize {
        let now = Instant::now();
        let (len, woken) = {
            let mut state = self.state.lock().unwrap_or_else(PoisonError::into_inner);
            let woken = state.reap_expired(now, self.limits.idle_ttl);
            (state.entries.len(), woken)
        };
        wake_all(woken);
        len
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn subscribe(&self, id: &str, uri: &'static str) -> Result<(), SessionError> {
        let now = Instant::now();
        let (result, woken) = {
            let mut state = self.state.lock().unwrap_or_else(PoisonError::into_inner);
            let woken = state.reap_expired(now, self.limits.idle_ttl);
            let seq = state.next_seq();
            let result = match state.entries.get_mut(id) {
                None => Err(SessionError::Unknown),
                Some(entry) if !entry.initialized => Err(SessionError::NotInitialized),
                Some(entry) => {
                    if !entry.subscriptions.contains(uri)
                        && entry.subscriptions.len() >= self.limits.max_subscriptions
                    {
                        Err(SessionError::TooManySubscriptions)
                    } else {
                        entry.subscriptions.insert(uri);
                        entry.touch(now, seq);
                        Ok(())
                    }
                }
            };
            (result, woken)
        };
        wake_all(woken);
        result
    }

    pub fn unsubscribe(&self, id: &str, uri: &str) -> Result<bool, SessionError> {
        let now = Instant::now();
        let (result, woken) = {
            let mut state = self.state.lock().unwrap_or_else(PoisonError::into_inner);
            let woken = state.reap_expired(now, self.limits.idle_ttl);
            let seq = state.next_seq();
            let result = match state.entries.get_mut(id) {
                None => Err(SessionError::Unknown),
                Some(entry) if !entry.initialized => Err(SessionError::NotInitialized),
                Some(entry) => {
                    let removed = entry.subscriptions.remove(uri);
                    entry.touch(now, seq);
                    Ok(removed)
                }
            };
            (result, woken)
        };
        wake_all(woken);
        result
    }

    pub fn subscriptions(&self, id: &str) -> Vec<&'static str> {
        let now = Instant::now();
        let (subs, woken) = {
            let mut state = self.state.lock().unwrap_or_else(PoisonError::into_inner);
            let woken = state.reap_expired(now, self.limits.idle_ttl);
            let subs = state
                .entries
                .get(id)
                .map(|entry| entry.subscriptions.iter().copied().collect())
                .unwrap_or_default();
            (subs, woken)
        };
        wake_all(woken);
        subs
    }

    /// Claims the one event stream an initialized session is allowed to have open.
    pub fn open_stream(&self, id: &str) -> Result<Arc<SessionEvents>, SessionError> {
        let now = Instant::now();
        let (result, woken) = {
            let mut state = self.state.lock().unwrap_or_else(PoisonError::into_inner);
            let woken = state.reap_expired(now, self.limits.idle_ttl);
            let seq = state.next_seq();
            let result = match state.entries.get_mut(id) {
                None => Err(SessionError::Unknown),
                Some(entry) if !entry.initialized => Err(SessionError::NotInitialized),
                Some(entry) if entry.stream_open => Err(SessionError::StreamAlreadyOpen),
                Some(entry) => {
                    entry.stream_open = true;
                    entry.touch(now, seq);
                    Ok(Arc::clone(&entry.events))
                }
            };
            (result, woken)
        };
        wake_all(woken);
        result
    }

    pub fn close_stream(&self, id: &str) {
        let now = Instant::now();
        let mut state = self.state.lock().unwrap_or_else(PoisonError::into_inner);
        let seq = state.next_seq();
        if let Some(entry) = state.entries.get_mut(id) {
            entry.stream_open = false;
            entry.touch(now, seq);
        }
    }

    pub fn is_live(&self, id: &str) -> bool {
        let now = Instant::now();
        let (live, woken) = {
            let mut state = self.state.lock().unwrap_or_else(PoisonError::into_inner);
            let woken = state.reap_expired(now, self.limits.idle_ttl);
            (state.entries.contains_key(id), woken)
        };
        wake_all(woken);
        live
    }

    /// Queues a message for every session subscribed to a resource. Returns how
    /// many sessions were told.
    pub fn publish(&self, uri: &str, message: &Value) -> usize {
        let now = Instant::now();
        let (told, woken) = {
            let mut state = self.state.lock().unwrap_or_else(PoisonError::into_inner);
            let woken = state.reap_expired(now, self.limits.idle_ttl);
            let mut told = 0;
            for entry in state.entries.values() {
                if entry.subscriptions.contains(uri) {
                    entry.events.push(message.clone(), self.limits.event_queue);
                    told += 1;
                }
            }
            (told, woken)
        };
        wake_all(woken);
        told
    }
}
