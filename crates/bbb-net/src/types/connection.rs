use std::{net::ToSocketAddrs, sync::Arc, time::Duration};

use anyhow::{anyhow, Result};
use bbb_pack::KnownPacksManager;
use bbb_protocol::{
    codec::offline_player_uuid,
    packets::{validate_handshake_host, validate_login_username, ClientInformation},
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionOptions {
    pub address: String,
    pub host: String,
    pub port: u16,
    pub username: String,
    pub profile_id: Uuid,
    pub timeout: Duration,
    #[serde(default)]
    pub accepted_code_of_conduct_hash: Option<i32>,
    #[serde(default)]
    pub probe_after_first_chunk_packets: usize,
    #[serde(default)]
    pub client_information: ClientInformation,
    #[serde(default)]
    pub server_resource_pack_policy: ServerResourcePackPolicy,
    #[serde(skip)]
    pub known_packs_manager: Option<Arc<KnownPacksManager>>,
}

impl ConnectionOptions {
    pub fn offline(address: impl Into<String>, username: impl Into<String>) -> Result<Self> {
        let address = address.into();
        let username = username.into();
        validate_login_username(&username)?;
        let (host, port) = split_host_port(&address)?;
        let profile_id = offline_player_uuid(&username);
        Ok(Self {
            address,
            host,
            port,
            username,
            profile_id,
            timeout: Duration::from_secs(20),
            accepted_code_of_conduct_hash: None,
            probe_after_first_chunk_packets: 0,
            client_information: ClientInformation::default(),
            server_resource_pack_policy: ServerResourcePackPolicy::Decline,
            known_packs_manager: None,
        })
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ServerResourcePackPolicy {
    Accept,
    #[default]
    Decline,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConnectionState {
    Handshake,
    Status,
    Login,
    Configuration,
    Play,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatusPing {
    pub json: String,
    pub latency_ms: u128,
}

pub(crate) fn split_host_port(address: &str) -> Result<(String, u16)> {
    if let Some((host, port)) = address.rsplit_once(':') {
        validate_handshake_host(host)?;
        let port = port.parse::<u16>()?;
        return Ok((host.to_string(), port));
    }

    validate_handshake_host(address)?;
    let mut addrs = (address, 25565).to_socket_addrs()?;
    let first = addrs
        .next()
        .ok_or_else(|| anyhow!("could not resolve {address}:25565"))?;
    Ok((address.to_string(), first.port()))
}

#[cfg(test)]
mod tests {
    use super::ConnectionOptions;

    #[test]
    fn offline_connection_options_delegate_login_username_validation() {
        assert!(ConnectionOptions::offline("localhost:25565", "valid_name").is_ok());
        assert!(ConnectionOptions::offline("localhost:25565", "invalid name").is_err());
        assert!(ConnectionOptions::offline("localhost:25565", "abcdefghijklmnopq").is_err());
    }

    #[test]
    fn connection_addresses_delegate_handshake_host_validation() {
        let maximum_host = "a".repeat(255);
        assert!(ConnectionOptions::offline(format!("{maximum_host}:25565"), "valid_name").is_ok());

        let oversized_host = "a".repeat(256);
        assert!(
            ConnectionOptions::offline(format!("{oversized_host}:25565"), "valid_name").is_err()
        );
    }
}
