//! Dedicated-server connection ownership for Minecraft Java 26.1.
//!
//! This crate owns accepted sockets, connection phases, compression, and the
//! bounded handoff to the future authoritative server tick. It deliberately
//! does not depend on the client-owned `bbb-client-net` or `bbb-client-world` crates.

mod driver;
mod listener;
mod mailbox;
mod session;
mod transport;

pub use driver::{
    ConnectionCommand, ConnectionCommandSender, ConnectionEvent, ConnectionHandle,
    ConnectionOutcome, PreparedPlayPacket, ServerConnectionDriver,
};
pub use listener::ServerListener;
pub use mailbox::{MailboxSendError, MailboxTrySendError};
pub use session::{
    vanilla_core_known_pack, ConnectionCloseReason, ConnectionLimits, ConnectionPhase,
    LoginAdmission, ServerConnectionConfig, ServerResourcePack,
};

use std::io;

use bbb_protocol::codec::ProtocolError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ServerNetError {
    #[error("network I/O failed: {0}")]
    Io(#[from] io::Error),
    #[error("protocol failed: {0}")]
    Protocol(#[from] ProtocolError),
    #[error("invalid server connection configuration: {0}")]
    InvalidConfiguration(String),
}

pub type Result<T> = std::result::Result<T, ServerNetError>;

#[cfg(test)]
mod tests;
