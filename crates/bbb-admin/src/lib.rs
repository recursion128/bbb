//! The administration client for a bbb dedicated server.
//!
//! bbb's dedicated server exposes exactly one management protocol — MCP over
//! Streamable HTTP, served by `bbb-server-admin` — and this crate is a client
//! of it, not a second surface beside it. A person running `bbb-admin` and an
//! agent connected to the same endpoint reach the same tools, under the same
//! authority, with the same refusals, because there is only one implementation
//! of any of that.
//!
//! Vanilla 26.1 ships no first-party management client; its `ManagementServer`
//! expects one an operator supplies. Shipping one in the repository is what
//! makes the endpoint testable end to end without trusting an external client
//! to be correct.

pub mod cli;
pub mod client;

pub use cli::{run, Cli, CliError, Command, Outcome};
pub use client::{endpoint_url, AdminClient, ClientConfiguration, ClientError, MAX_ANSWER_BYTES};

#[cfg(test)]
mod tests;
