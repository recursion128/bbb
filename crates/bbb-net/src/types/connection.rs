use std::{net::ToSocketAddrs, sync::Arc, time::Duration};

use anyhow::{anyhow, Result};
use bbb_pack::KnownPacksManager;
use bbb_protocol::{
    codec::offline_player_uuid,
    packets::{validate_handshake_host, validate_login_username, ClientInformation},
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// The reachable projection of Vanilla's `User` for the declared offline-only
/// external-client scope. Online access tokens and telemetry-only XUID/client
/// ids deliberately do not enter this runtime boundary.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClientIdentity {
    pub username: String,
    pub profile_id: Uuid,
}

impl ClientIdentity {
    pub fn offline(username: impl Into<String>) -> Self {
        let username = username.into();
        let profile_id = offline_player_uuid(&username);
        Self {
            username,
            profile_id,
        }
    }

    pub fn with_profile_id(username: impl Into<String>, profile_id: Uuid) -> Self {
        Self {
            username: username.into(),
            profile_id,
        }
    }
}

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
        let identity = ClientIdentity::offline(username);
        Self::offline_with_identity(address, &identity)
    }

    pub fn offline_with_identity(
        address: impl Into<String>,
        identity: &ClientIdentity,
    ) -> Result<Self> {
        let address = address.into();
        validate_login_username(&identity.username)?;
        let (host, port) = split_host_port(&address)?;
        Ok(Self {
            address,
            host,
            port,
            username: identity.username.clone(),
            profile_id: identity.profile_id,
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
    use super::{ClientIdentity, ConnectionOptions};
    use uuid::Uuid;

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

    #[test]
    fn explicit_launcher_identity_is_preserved_for_login_instead_of_rederived() {
        let profile_id = Uuid::from_u128(0x00112233_4455_6677_8899_aabbccddeeff);
        let identity = ClientIdentity::with_profile_id("bbb-client", profile_id);
        let options =
            ConnectionOptions::offline_with_identity("localhost:25565", &identity).unwrap();

        assert_eq!(options.username, "bbb-client");
        assert_eq!(options.profile_id, profile_id);
    }
}
