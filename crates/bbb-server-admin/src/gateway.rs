//! The one way a management request reaches canonical state.
//!
//! Vanilla owner: `MinecraftExecutorServiceImpl.submit`
//! (MinecraftExecutorServiceImpl.java:15-22), which hands every management
//! operation to the server thread and gives the caller a future for the answer.
//! `DedicatedServer.tickServer` is where those submissions run.
//!
//! bbb's equivalent is the bounded world mailbox: the transport never borrows
//! the world, it submits an owned request and awaits the answer the tick
//! produces. This trait is the seam — the runtime implements it, and everything
//! in this crate is written against it, so the transport can be exercised
//! without a world and the world never learns that MCP exists.

pub use bbb_mcp::ResourceChange;
use bbb_server_core::admin::{AdminOutcome, AdminRequest, AdminScope};
use thiserror::Error;
use tokio::sync::{broadcast, oneshot};

/// One management call, with the authority the transport resolved for it.
///
/// The operation id and the deadline tick are deliberately absent: both belong
/// to the runtime, which owns the id counter and the only clock that can name a
/// tick.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdminCall {
    pub principal: String,
    pub scope: AdminScope,
    /// The caller's own id. Two calls from one principal sharing it are the
    /// same operation, and the second is answered from the first's record.
    pub request_id: String,
    pub request: AdminRequest,
}

/// A submitted call: its server-assigned id, and the answer the tick will
/// produce.
#[derive(Debug)]
pub struct PendingOperation {
    pub operation_id: u64,
    pub answer: oneshot::Receiver<AdminOutcome>,
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum GatewayError {
    #[error("the management mailbox is full")]
    Busy,
    #[error("the server is shutting down")]
    Closed,
    #[error("the request retains {actual} bytes and the management budget is {max}")]
    Oversized { actual: usize, max: usize },
}

pub trait AdminGateway: Send + Sync + 'static {
    /// Submits a call into the bounded mailbox. Never blocks: a full mailbox is
    /// attributable backpressure, not a wait.
    fn submit(&self, call: AdminCall) -> Result<PendingOperation, GatewayError>;

    /// Epoch milliseconds. Neither the world nor this crate owns a clock, so
    /// the one instant both need travels in from the runtime.
    fn now(&self) -> i64;

    /// The change stream the subscription fan-out reads.
    fn subscribe(&self) -> broadcast::Receiver<ResourceChange>;
}
