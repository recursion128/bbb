use std::{
    collections::VecDeque,
    fmt,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc, Mutex,
    },
};

use tokio::sync::Notify;

pub(crate) trait RetainedBytes {
    fn retained_bytes(&self) -> usize;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct MailboxLimits {
    pub entries: usize,
    pub bytes: usize,
}

impl MailboxLimits {
    pub(crate) fn validate(self, name: &str) -> std::result::Result<Self, String> {
        if self.entries == 0 {
            return Err(format!("{name} entry capacity must be non-zero"));
        }
        if self.bytes == 0 {
            return Err(format!("{name} byte capacity must be non-zero"));
        }
        Ok(self)
    }
}

#[derive(Debug)]
pub enum MailboxTrySendError<T> {
    Closed(T),
    Full(T),
    Oversized { value: T, actual: usize, max: usize },
}

impl<T> fmt::Display for MailboxTrySendError<T> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Closed(_) => formatter.write_str("mailbox is closed"),
            Self::Full(_) => formatter.write_str("mailbox is full"),
            Self::Oversized { actual, max, .. } => {
                write!(
                    formatter,
                    "mailbox item retains {actual} bytes, maximum is {max}"
                )
            }
        }
    }
}

#[derive(Debug)]
pub enum MailboxSendError<T> {
    Closed(T),
    Oversized { value: T, actual: usize, max: usize },
}

impl<T> fmt::Display for MailboxSendError<T> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Closed(_) => formatter.write_str("mailbox is closed"),
            Self::Oversized { actual, max, .. } => {
                write!(
                    formatter,
                    "mailbox item retains {actual} bytes, maximum is {max}"
                )
            }
        }
    }
}

struct MailboxState<T> {
    queue: VecDeque<(T, usize)>,
    retained_bytes: usize,
    closed: bool,
}

struct MailboxInner<T> {
    limits: MailboxLimits,
    state: Mutex<MailboxState<T>>,
    sender_count: AtomicUsize,
    item_ready: Notify,
    space_ready: Notify,
    closed: Notify,
}

pub(crate) struct MailboxSender<T> {
    inner: Arc<MailboxInner<T>>,
}

pub(crate) struct MailboxReceiver<T> {
    inner: Arc<MailboxInner<T>>,
}

pub(crate) fn bounded_mailbox<T>(limits: MailboxLimits) -> (MailboxSender<T>, MailboxReceiver<T>) {
    let inner = Arc::new(MailboxInner {
        limits,
        state: Mutex::new(MailboxState {
            queue: VecDeque::with_capacity(limits.entries),
            retained_bytes: 0,
            closed: false,
        }),
        sender_count: AtomicUsize::new(1),
        item_ready: Notify::new(),
        space_ready: Notify::new(),
        closed: Notify::new(),
    });
    (
        MailboxSender {
            inner: Arc::clone(&inner),
        },
        MailboxReceiver { inner },
    )
}

impl<T> Clone for MailboxSender<T> {
    fn clone(&self) -> Self {
        self.inner
            .sender_count
            .try_update(Ordering::Relaxed, Ordering::Relaxed, |count| {
                count.checked_add(1)
            })
            .expect("mailbox sender count overflow");
        Self {
            inner: Arc::clone(&self.inner),
        }
    }
}

impl<T: RetainedBytes> MailboxSender<T> {
    pub(crate) fn try_send(&self, value: T) -> std::result::Result<(), MailboxTrySendError<T>> {
        let actual = value.retained_bytes();
        if actual > self.inner.limits.bytes {
            return Err(MailboxTrySendError::Oversized {
                value,
                actual,
                max: self.inner.limits.bytes,
            });
        }

        let mut state = self
            .inner
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if state.closed {
            return Err(MailboxTrySendError::Closed(value));
        }
        let bytes_fit = state
            .retained_bytes
            .checked_add(actual)
            .is_some_and(|next| next <= self.inner.limits.bytes);
        if state.queue.len() >= self.inner.limits.entries || !bytes_fit {
            return Err(MailboxTrySendError::Full(value));
        }
        state.queue.push_back((value, actual));
        state.retained_bytes += actual;
        drop(state);
        self.inner.item_ready.notify_one();
        Ok(())
    }

    pub(crate) async fn send(&self, value: T) -> std::result::Result<(), MailboxSendError<T>> {
        let mut value = Some(value);
        loop {
            let notified = self.inner.space_ready.notified();
            tokio::pin!(notified);
            notified.as_mut().enable();
            match self.try_send(value.take().expect("mailbox value retained while waiting")) {
                Ok(()) => return Ok(()),
                Err(MailboxTrySendError::Closed(value)) => {
                    return Err(MailboxSendError::Closed(value));
                }
                Err(MailboxTrySendError::Oversized { value, actual, max }) => {
                    return Err(MailboxSendError::Oversized { value, actual, max });
                }
                Err(MailboxTrySendError::Full(returned)) => {
                    value = Some(returned);
                }
            }
            notified.await;
        }
    }

    pub(crate) fn close(&self) {
        self.inner.close();
    }

    pub(crate) fn is_closed(&self) -> bool {
        self.inner
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .closed
    }
}

impl<T> Drop for MailboxSender<T> {
    fn drop(&mut self) {
        if self.inner.sender_count.fetch_sub(1, Ordering::AcqRel) == 1 {
            self.inner.close();
        }
    }
}

impl<T> MailboxReceiver<T> {
    pub(crate) fn closed(&self) -> impl std::future::Future<Output = ()> + Send + 'static
    where
        T: Send + 'static,
    {
        let inner = Arc::clone(&self.inner);
        async move {
            let notified = inner.closed.notified();
            tokio::pin!(notified);
            notified.as_mut().enable();
            let closed = inner
                .state
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .closed;
            if !closed {
                notified.await;
            }
        }
    }

    pub(crate) async fn recv(&self) -> Option<T> {
        loop {
            let notified = self.inner.item_ready.notified();
            tokio::pin!(notified);
            notified.as_mut().enable();
            match self.try_recv() {
                MailboxReceive::Item(value) => return Some(value),
                MailboxReceive::Closed => return None,
                MailboxReceive::Empty => {}
            }
            notified.await;
        }
    }

    pub(crate) fn try_recv(&self) -> MailboxReceive<T> {
        let mut state = self
            .inner
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if let Some((value, retained_bytes)) = state.queue.pop_front() {
            state.retained_bytes = state.retained_bytes.saturating_sub(retained_bytes);
            drop(state);
            self.inner.space_ready.notify_one();
            return MailboxReceive::Item(value);
        }
        if state.closed {
            MailboxReceive::Closed
        } else {
            MailboxReceive::Empty
        }
    }

    pub(crate) fn close(&self) {
        self.inner.close();
    }
}

pub(crate) enum MailboxReceive<T> {
    Item(T),
    Empty,
    Closed,
}

impl<T> Drop for MailboxReceiver<T> {
    fn drop(&mut self) {
        self.inner.close();
    }
}

impl<T> MailboxInner<T> {
    fn close(&self) {
        let mut state = self
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        state.closed = true;
        drop(state);
        self.item_ready.notify_waiters();
        self.space_ready.notify_waiters();
        self.closed.notify_waiters();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, PartialEq, Eq)]
    struct Item(usize);

    impl RetainedBytes for Item {
        fn retained_bytes(&self) -> usize {
            self.0
        }
    }

    #[tokio::test]
    async fn mailbox_enforces_entry_and_byte_limits_and_releases_budget_on_receive() {
        let (sender, receiver) = bounded_mailbox(MailboxLimits {
            entries: 2,
            bytes: 5,
        });
        sender.try_send(Item(3)).unwrap();
        sender.try_send(Item(2)).unwrap();
        assert!(matches!(
            sender.try_send(Item(0)),
            Err(MailboxTrySendError::Full(Item(0)))
        ));
        assert!(matches!(
            sender.try_send(Item(6)),
            Err(MailboxTrySendError::Oversized {
                value: Item(6),
                actual: 6,
                max: 5
            })
        ));

        assert_eq!(receiver.recv().await, Some(Item(3)));
        sender.try_send(Item(3)).unwrap();
        assert_eq!(receiver.recv().await, Some(Item(2)));
        assert_eq!(receiver.recv().await, Some(Item(3)));
    }

    #[tokio::test]
    async fn close_wakes_a_waiting_receiver_and_sender() {
        let (sender, receiver) = bounded_mailbox(MailboxLimits {
            entries: 1,
            bytes: 1,
        });
        sender.try_send(Item(1)).unwrap();
        let blocked = sender.send(Item(1));
        tokio::pin!(blocked);
        assert!(
            tokio::time::timeout(std::time::Duration::from_millis(10), &mut blocked)
                .await
                .is_err()
        );
        receiver.close();
        assert!(matches!(
            blocked.await,
            Err(MailboxSendError::Closed(Item(1)))
        ));

        let (sender, receiver) = bounded_mailbox::<Item>(MailboxLimits {
            entries: 1,
            bytes: 1,
        });
        let waiting = receiver.recv();
        tokio::pin!(waiting);
        assert!(
            tokio::time::timeout(std::time::Duration::from_millis(10), &mut waiting)
                .await
                .is_err()
        );
        sender.close();
        assert_eq!(waiting.await, None);
    }

    #[tokio::test]
    async fn dropping_a_sender_clone_keeps_the_mailbox_open_until_the_last_producer() {
        let (sender, receiver) = bounded_mailbox(MailboxLimits {
            entries: 1,
            bytes: 1,
        });
        let clone = sender.clone();
        drop(sender);
        clone.send(Item(1)).await.unwrap();
        assert_eq!(receiver.recv().await, Some(Item(1)));
        drop(clone);
        assert_eq!(receiver.recv().await, None);
    }
}
