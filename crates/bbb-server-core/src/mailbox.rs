use std::{
    collections::VecDeque,
    fmt,
    mem::size_of,
    sync::{Arc, Mutex, Weak},
};

use tokio::sync::Notify;

use crate::world::WorldCommand;
use bbb_server_types::ServerWorldError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorldProducerClass {
    Network,
    Admin,
    Lifecycle,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MailboxClassLimits {
    pub entries: usize,
    pub bytes: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WorldMailboxLimits {
    pub network: MailboxClassLimits,
    pub admin: MailboxClassLimits,
    pub lifecycle: MailboxClassLimits,
}

impl Default for WorldMailboxLimits {
    fn default() -> Self {
        Self {
            network: MailboxClassLimits {
                entries: 1024,
                bytes: 4 * 1024 * 1024,
            },
            admin: MailboxClassLimits {
                entries: 128,
                bytes: 512 * 1024,
            },
            lifecycle: MailboxClassLimits {
                entries: 16,
                bytes: 64 * 1024,
            },
        }
    }
}

impl WorldMailboxLimits {
    fn validate(self) -> std::result::Result<Self, ServerWorldError> {
        for (name, limits) in [
            ("network", self.network),
            ("admin", self.admin),
            ("lifecycle", self.lifecycle),
        ] {
            if limits.entries == 0 || limits.bytes == 0 {
                return Err(ServerWorldError::InvalidConfiguration(format!(
                    "{name} world mailbox capacities must be non-zero"
                )));
            }
        }
        Ok(self)
    }

    fn for_class(self, class: WorldProducerClass) -> MailboxClassLimits {
        match class {
            WorldProducerClass::Network => self.network,
            WorldProducerClass::Admin => self.admin,
            WorldProducerClass::Lifecycle => self.lifecycle,
        }
    }
}

#[derive(Debug)]
pub struct CommandEnvelope {
    pub(crate) sequence: u64,
    pub(crate) class: WorldProducerClass,
    pub(crate) command: WorldCommand,
    _budget: CommandBudget,
}

#[derive(Debug)]
struct CommandBudget {
    mailbox: Weak<MailboxInner>,
    class: WorldProducerClass,
    bytes: usize,
}

impl Drop for CommandBudget {
    fn drop(&mut self) {
        let Some(mailbox) = self.mailbox.upgrade() else {
            return;
        };
        let mut state = mailbox.state.lock().expect("world mailbox poisoned");
        let usage = &mut state.usage[class_index(self.class)];
        usage.entries -= 1;
        usage.bytes -= self.bytes;
        drop(state);
        mailbox.space_ready.notify_waiters();
    }
}

impl CommandEnvelope {
    #[cfg(test)]
    pub(crate) fn for_test(
        sequence: u64,
        class: WorldProducerClass,
        command: WorldCommand,
    ) -> Self {
        Self {
            sequence,
            class,
            command,
            _budget: CommandBudget {
                mailbox: Weak::new(),
                class,
                bytes: 0,
            },
        }
    }

    pub fn sequence(&self) -> u64 {
        self.sequence
    }

    pub fn producer_class(&self) -> WorldProducerClass {
        self.class
    }

    /// The command itself, for a producer that has to assert what it queued.
    pub fn command(&self) -> &WorldCommand {
        &self.command
    }
}

#[derive(Debug)]
pub enum WorldCommandSendError {
    Closed(Box<WorldCommand>),
    Full(Box<WorldCommand>),
    Oversized {
        command: Box<WorldCommand>,
        actual: usize,
        max: usize,
    },
}

impl fmt::Display for WorldCommandSendError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Closed(_) => formatter.write_str("world command mailbox is closed"),
            Self::Full(_) => formatter.write_str("world command mailbox class is full"),
            Self::Oversized { actual, max, .. } => write!(
                formatter,
                "world command retains {actual} bytes, class maximum is {max}"
            ),
        }
    }
}

#[derive(Debug, Clone, Copy, Default)]
struct ClassUsage {
    entries: usize,
    bytes: usize,
}

struct MailboxState {
    queue: VecDeque<CommandEnvelope>,
    usage: [ClassUsage; 3],
    next_sequence: u64,
    closed: bool,
}

struct MailboxInner {
    limits: WorldMailboxLimits,
    state: Mutex<MailboxState>,
    space_ready: Notify,
}

#[derive(Clone)]
pub struct WorldCommandSender {
    inner: Arc<MailboxInner>,
}

pub struct WorldCommandReceiver {
    inner: Arc<MailboxInner>,
}

pub fn world_command_mailbox(
    limits: WorldMailboxLimits,
) -> bbb_server_types::Result<(WorldCommandSender, WorldCommandReceiver)> {
    let limits = limits.validate()?;
    let inner = Arc::new(MailboxInner {
        limits,
        state: Mutex::new(MailboxState {
            queue: VecDeque::new(),
            usage: [ClassUsage::default(); 3],
            next_sequence: 1,
            closed: false,
        }),
        space_ready: Notify::new(),
    });
    Ok((
        WorldCommandSender {
            inner: Arc::clone(&inner),
        },
        WorldCommandReceiver { inner },
    ))
}

impl WorldCommandSender {
    pub fn try_send(
        &self,
        command: WorldCommand,
    ) -> std::result::Result<u64, WorldCommandSendError> {
        let class = command.producer_class();
        let retained = retained_bytes(&command);
        let limits = self.inner.limits.for_class(class);
        if retained > limits.bytes {
            return Err(WorldCommandSendError::Oversized {
                command: Box::new(command),
                actual: retained,
                max: limits.bytes,
            });
        }
        let mut state = self.inner.state.lock().expect("world mailbox poisoned");
        if state.closed {
            return Err(WorldCommandSendError::Closed(Box::new(command)));
        }
        let index = class_index(class);
        let usage = state.usage[index];
        let bytes_fit = usage
            .bytes
            .checked_add(retained)
            .is_some_and(|next| next <= limits.bytes);
        if usage.entries >= limits.entries || !bytes_fit {
            return Err(WorldCommandSendError::Full(Box::new(command)));
        }
        let sequence = state.next_sequence;
        state.next_sequence = state.next_sequence.wrapping_add(1);
        state.usage[index].entries += 1;
        state.usage[index].bytes += retained;
        state.queue.push_back(CommandEnvelope {
            sequence,
            class,
            command,
            _budget: CommandBudget {
                mailbox: Arc::downgrade(&self.inner),
                class,
                bytes: retained,
            },
        });
        Ok(sequence)
    }

    pub async fn send(
        &self,
        command: WorldCommand,
    ) -> std::result::Result<u64, WorldCommandSendError> {
        let mut command = Some(command);
        loop {
            let notified = self.inner.space_ready.notified();
            tokio::pin!(notified);
            notified.as_mut().enable();
            match self.try_send(command.take().expect("command retained while waiting")) {
                Ok(sequence) => return Ok(sequence),
                Err(WorldCommandSendError::Full(returned)) => command = Some(*returned),
                Err(error) => return Err(error),
            }
            notified.await;
        }
    }

    pub fn close(&self) {
        self.inner.close();
    }
}

impl WorldCommandReceiver {
    pub fn drain(&self) -> Vec<CommandEnvelope> {
        let mut state = self.inner.state.lock().expect("world mailbox poisoned");
        // A drained command still retains its payload until the tick applies
        // or discards it, including a join deferred to a later tick.
        state.queue.drain(..).collect()
    }

    pub fn close(&self) {
        self.inner.close();
    }
}

impl Drop for WorldCommandReceiver {
    fn drop(&mut self) {
        self.inner.close();
    }
}

impl MailboxInner {
    fn close(&self) {
        let mut state = self.state.lock().expect("world mailbox poisoned");
        state.closed = true;
        drop(state);
        self.space_ready.notify_waiters();
    }
}

fn class_index(class: WorldProducerClass) -> usize {
    match class {
        WorldProducerClass::Network => 0,
        WorldProducerClass::Admin => 1,
        WorldProducerClass::Lifecycle => 2,
    }
}

fn retained_bytes(command: &WorldCommand) -> usize {
    let nested = match command {
        WorldCommand::Join {
            profile,
            client_information,
            context,
            ..
        } => {
            profile.name.capacity()
                + client_information.language.capacity()
                + profile.properties.capacity()
                    * size_of::<bbb_protocol::packets::GameProfileProperty>()
                + profile
                    .properties
                    .iter()
                    .map(|property| {
                        property.name.capacity()
                            + property.value.capacity()
                            + property.signature.as_ref().map_or(0, String::capacity)
                    })
                    .sum::<usize>()
                + context.remote_address.as_ref().map_or(0, String::capacity)
        }
        // The admin class quota has to count what the request itself retains,
        // or a bounded mailbox would hold an unbounded management payload.
        WorldCommand::Admin { envelope, request } => {
            envelope.request_id.capacity()
                + envelope.principal.capacity()
                + request.retained_bytes()
        }
        // A rebuilt datapack generation is shared, not owned by the command:
        // the runtime built it and the world takes a second handle. A failure
        // detail is the message the reload reports.
        WorldCommand::Datapacks { startup } => match startup {
            crate::world::ReloadedDatapacks::Built(_) => 0,
            crate::world::ReloadedDatapacks::Failed(detail) => detail.capacity(),
        },
        WorldCommand::PlayPacket { payload, .. } => payload.capacity(),
        #[cfg(test)]
        WorldCommand::Play { .. } => 0,
        WorldCommand::Disconnect { .. } | WorldCommand::Shutdown => 0,
    };
    size_of::<WorldCommand>() + nested
}
