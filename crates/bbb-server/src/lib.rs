//! Headless process runtime for the repo-native Minecraft Java 26.1 server.
//!
//! The runtime owns listener acceptance, connection-task lifetimes, the 20 Hz
//! authoritative world tick integration, and graceful process shutdown.

mod management;
pub mod mods;
mod options;
mod panic_report;
mod runtime;
mod signal;

/// Matches `.cargo/config.toml`'s `RUST_MIN_STACK`: worldgen, commands, and
/// 512-level NBT recursion exceed Windows MSVC's 1 MiB default main-thread
/// stack and 2 MiB OS worker stacks when launched outside Cargo.
pub const SERVER_THREAD_STACK_BYTES: usize = 16 * 1024 * 1024;

pub use management::RuntimeGateway;
pub use options::{LoopbackGrant, ServerOptions};
pub use panic_report::install_panic_report;
pub use runtime::{ServerReport, ServerRuntime, ServerRuntimeError};
pub use signal::{ShutdownSignal, ShutdownSignals};

#[cfg(test)]
mod tests;
