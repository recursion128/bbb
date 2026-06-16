use std::collections::BTreeMap;

use bbb_protocol::packets::{
    CustomPayload as ProtocolCustomPayload, CustomPayloadBody as ProtocolCustomPayloadBody,
    CustomReportDetails as ProtocolCustomReportDetails, ResourcePackPop as ProtocolResourcePackPop,
    ResourcePackPush as ProtocolResourcePackPush,
    ResourcePackResponseAction as ProtocolResourcePackResponseAction,
    ServerData as ProtocolServerData, ServerLinkEntry as ProtocolServerLinkEntry,
    ServerLinkType as ProtocolServerLinkType, ServerLinks as ProtocolServerLinks,
    Transfer as ProtocolTransfer,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::WorldStore;

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ServerPresentationState {
    #[serde(default)]
    pub server_data: Option<ServerDataState>,
    #[serde(default)]
    pub server_brand: Option<String>,
    #[serde(default)]
    pub server_cookies: ServerCookieState,
    #[serde(default)]
    pub last_custom_payload: Option<CustomPayloadState>,
    #[serde(default)]
    pub last_transfer: Option<TransferTargetState>,
    #[serde(default)]
    pub resource_packs: BTreeMap<Uuid, ResourcePackState>,
    #[serde(default)]
    pub server_links: Vec<ServerLinkState>,
    #[serde(default)]
    pub custom_report_details: BTreeMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ServerDataState {
    pub motd: String,
    pub icon_bytes: Option<Vec<u8>>,
}

impl ServerDataState {
    pub fn icon_byte_len(&self) -> Option<usize> {
        self.icon_bytes.as_ref().map(Vec::len)
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CustomPayloadState {
    pub id: String,
    pub kind: String,
    pub brand: Option<String>,
    pub raw_payload_len: usize,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ServerCookieState {
    pub last_key: Option<String>,
    pub stored_count: usize,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TransferTargetState {
    pub host: String,
    pub port: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResourcePackState {
    pub id: Uuid,
    pub url: String,
    pub hash: String,
    pub required: bool,
    pub prompt: Option<String>,
    #[serde(default)]
    pub last_response: Option<ResourcePackResponseState>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResourcePackResponseState {
    pub action: ProtocolResourcePackResponseAction,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ServerLinkState {
    pub label: String,
    pub url: String,
    pub known_type: Option<String>,
}

impl WorldStore {
    pub fn apply_custom_payload(&mut self, packet: ProtocolCustomPayload) -> &CustomPayloadState {
        self.counters.custom_payload_packets += 1;
        let state = CustomPayloadState::from_packet(packet);
        if let Some(brand) = &state.brand {
            self.counters.custom_payload_brand_packets += 1;
            self.presentation.server_brand = Some(brand.clone());
        } else {
            self.counters.custom_payload_unknown_packets += 1;
        }
        self.presentation.last_custom_payload = Some(state);
        self.presentation
            .last_custom_payload
            .as_ref()
            .expect("last custom payload was just stored")
    }

    pub fn apply_transfer(&mut self, packet: ProtocolTransfer) -> &TransferTargetState {
        self.counters.transfer_packets += 1;
        self.presentation.last_transfer = Some(TransferTargetState {
            host: packet.host,
            port: packet.port,
        });
        self.presentation
            .last_transfer
            .as_ref()
            .expect("last transfer was just stored")
    }

    pub fn apply_server_data(&mut self, packet: ProtocolServerData) {
        self.counters.server_data_packets += 1;
        let icon_bytes = packet.icon_bytes.or_else(|| {
            self.presentation
                .server_data
                .as_ref()
                .and_then(|server_data| server_data.icon_bytes.clone())
        });
        self.presentation.server_data = Some(ServerDataState {
            motd: packet.motd,
            icon_bytes,
        });
    }

    pub fn apply_resource_pack_push(&mut self, packet: ProtocolResourcePackPush) {
        self.counters.resource_pack_push_packets += 1;
        let pack = ResourcePackState {
            id: packet.id,
            url: packet.url,
            hash: packet.hash,
            required: packet.required,
            prompt: non_empty_component_string(packet.prompt),
            last_response: None,
        };
        self.presentation.resource_packs.insert(pack.id, pack);
        self.update_resource_pack_count();
    }

    pub fn apply_resource_pack_response(
        &mut self,
        id: Uuid,
        action: ProtocolResourcePackResponseAction,
    ) -> bool {
        self.counters.resource_pack_response_packets += 1;
        let Some(pack) = self.presentation.resource_packs.get_mut(&id) else {
            self.counters.resource_pack_response_updates_ignored += 1;
            return false;
        };

        if pack.required && action == ProtocolResourcePackResponseAction::Declined {
            self.counters.resource_pack_required_declines += 1;
        }
        pack.last_response = Some(ResourcePackResponseState { action });
        self.counters.resource_pack_response_updates_applied += 1;
        true
    }

    pub fn apply_resource_pack_pop(&mut self, packet: ProtocolResourcePackPop) -> usize {
        self.counters.resource_pack_pop_packets += 1;
        let removed = match packet.id {
            Some(id) => {
                if self.presentation.resource_packs.remove(&id).is_some() {
                    1
                } else {
                    0
                }
            }
            None => {
                let removed = self.presentation.resource_packs.len();
                self.presentation.resource_packs.clear();
                removed
            }
        };
        if removed > 0 {
            self.counters.resource_pack_pop_updates_applied += 1;
        } else {
            self.counters.resource_pack_pop_updates_ignored += 1;
        }
        self.update_resource_pack_count();
        removed
    }

    pub fn apply_server_links(&mut self, packet: ProtocolServerLinks) -> usize {
        self.counters.server_link_packets += 1;
        let mut invalid_entries = 0usize;
        let server_links = packet
            .links
            .into_iter()
            .filter_map(|entry| {
                if is_allowed_untrusted_uri(&entry.url) {
                    Some(ServerLinkState::from_entry(entry))
                } else {
                    invalid_entries += 1;
                    None
                }
            })
            .collect();
        self.presentation.server_links = server_links;
        self.counters.server_link_invalid_entries = self
            .counters
            .server_link_invalid_entries
            .saturating_add(invalid_entries);
        self.update_server_link_count();
        invalid_entries
    }

    pub fn apply_custom_report_details(&mut self, packet: ProtocolCustomReportDetails) {
        self.counters.custom_report_detail_packets += 1;
        self.presentation.custom_report_details = packet.details;
        self.update_custom_report_detail_count();
    }

    pub fn apply_cookie_request(&mut self, key: impl Into<String>, response_payload_present: bool) {
        self.counters.cookie_request_packets += 1;
        if response_payload_present {
            self.counters.cookie_response_hits += 1;
        } else {
            self.counters.cookie_response_misses += 1;
        }
        self.presentation.server_cookies.last_key = Some(key.into());
    }

    pub fn apply_store_cookie(
        &mut self,
        key: impl Into<String>,
        payload_len: usize,
        stored_cookie_count: usize,
    ) {
        self.counters.store_cookie_packets += 1;
        self.counters.stored_cookie_bytes = self
            .counters
            .stored_cookie_bytes
            .saturating_add(payload_len);
        self.presentation.server_cookies.last_key = Some(key.into());
        self.presentation.server_cookies.stored_count = stored_cookie_count;
    }

    pub fn presentation(&self) -> &ServerPresentationState {
        &self.presentation
    }

    pub fn server_data(&self) -> Option<&ServerDataState> {
        self.presentation.server_data.as_ref()
    }

    pub fn server_brand(&self) -> Option<&str> {
        self.presentation.server_brand.as_deref()
    }

    pub fn server_cookies(&self) -> &ServerCookieState {
        &self.presentation.server_cookies
    }

    pub fn last_cookie_key(&self) -> Option<&str> {
        self.presentation.server_cookies.last_key.as_deref()
    }

    pub fn stored_cookie_count(&self) -> usize {
        self.presentation.server_cookies.stored_count
    }

    pub fn last_custom_payload(&self) -> Option<&CustomPayloadState> {
        self.presentation.last_custom_payload.as_ref()
    }

    pub fn last_transfer(&self) -> Option<&TransferTargetState> {
        self.presentation.last_transfer.as_ref()
    }

    pub fn resource_packs(&self) -> &BTreeMap<Uuid, ResourcePackState> {
        &self.presentation.resource_packs
    }

    pub fn resource_pack(&self, id: Uuid) -> Option<&ResourcePackState> {
        self.presentation.resource_packs.get(&id)
    }

    pub fn server_links(&self) -> &[ServerLinkState] {
        &self.presentation.server_links
    }

    pub fn custom_report_details(&self) -> &BTreeMap<String, String> {
        &self.presentation.custom_report_details
    }

    fn update_resource_pack_count(&mut self) {
        self.counters.resource_packs_tracked = self.presentation.resource_packs.len();
    }

    fn update_server_link_count(&mut self) {
        self.counters.server_links_tracked = self.presentation.server_links.len();
    }

    fn update_custom_report_detail_count(&mut self) {
        self.counters.custom_report_details_tracked = self.presentation.custom_report_details.len();
    }
}

impl CustomPayloadState {
    fn from_packet(packet: ProtocolCustomPayload) -> Self {
        match packet.payload {
            ProtocolCustomPayloadBody::Brand { brand } => Self {
                id: packet.id,
                kind: "brand".to_string(),
                brand: Some(brand),
                raw_payload_len: 0,
            },
            ProtocolCustomPayloadBody::Unknown { raw_payload } => Self {
                id: packet.id,
                kind: "unknown".to_string(),
                brand: None,
                raw_payload_len: raw_payload.len(),
            },
        }
    }
}

impl ServerLinkState {
    fn from_entry(entry: ProtocolServerLinkEntry) -> Self {
        match entry.link_type {
            ProtocolServerLinkType::Known(kind) => {
                let known_type = kind.vanilla_name();
                Self {
                    label: format!("known_server_link.{known_type}"),
                    url: entry.url,
                    known_type: Some(known_type.to_string()),
                }
            }
            ProtocolServerLinkType::Custom { label } => Self {
                label,
                url: entry.url,
                known_type: None,
            },
        }
    }
}

fn non_empty_component_string(component: Option<String>) -> Option<String> {
    component.filter(|value| !value.is_empty())
}

fn is_allowed_untrusted_uri(uri: &str) -> bool {
    if uri
        .chars()
        .any(|ch| ch.is_ascii_control() || ch.is_whitespace())
    {
        return false;
    }
    let Some((scheme, _)) = uri.split_once(':') else {
        return false;
    };
    if scheme.is_empty() {
        return false;
    }
    let mut chars = scheme.chars();
    if !chars.next().is_some_and(|ch| ch.is_ascii_alphabetic()) {
        return false;
    }
    if !chars.all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '+' | '-' | '.')) {
        return false;
    }
    matches!(scheme.to_ascii_lowercase().as_str(), "http" | "https")
}

#[cfg(test)]
mod tests {
    use super::*;
    use bbb_protocol::packets::{
        CustomPayloadBody, CustomReportDetails, ServerLinkKnownType, ServerLinkType,
    };

    #[test]
    fn custom_payload_tracks_server_brand_and_unknown_payloads() {
        let mut store = WorldStore::new();

        let brand = store
            .apply_custom_payload(ProtocolCustomPayload {
                id: "minecraft:brand".to_string(),
                payload: CustomPayloadBody::Brand {
                    brand: "vanilla".to_string(),
                },
            })
            .clone();

        assert_eq!(
            brand,
            CustomPayloadState {
                id: "minecraft:brand".to_string(),
                kind: "brand".to_string(),
                brand: Some("vanilla".to_string()),
                raw_payload_len: 0,
            }
        );
        assert_eq!(store.server_brand(), Some("vanilla"));

        store.apply_custom_payload(ProtocolCustomPayload {
            id: "example:diagnostic".to_string(),
            payload: CustomPayloadBody::Unknown {
                raw_payload: vec![1, 2, 3, 4],
            },
        });

        assert_eq!(store.server_brand(), Some("vanilla"));
        assert_eq!(
            store.last_custom_payload(),
            Some(&CustomPayloadState {
                id: "example:diagnostic".to_string(),
                kind: "unknown".to_string(),
                brand: None,
                raw_payload_len: 4,
            })
        );
        let counters = store.counters();
        assert_eq!(counters.custom_payload_packets, 2);
        assert_eq!(counters.custom_payload_brand_packets, 1);
        assert_eq!(counters.custom_payload_unknown_packets, 1);
    }

    #[test]
    fn cookie_metadata_tracks_requests_and_stores() {
        let mut store = WorldStore::new();

        store.apply_store_cookie("bbb:session", 3, 1);
        store.apply_cookie_request("bbb:session", true);
        store.apply_cookie_request("bbb:missing", false);

        assert_eq!(store.last_cookie_key(), Some("bbb:missing"));
        assert_eq!(store.server_cookies().stored_count, 1);
        assert_eq!(store.stored_cookie_count(), 1);
        let counters = store.counters();
        assert_eq!(counters.store_cookie_packets, 1);
        assert_eq!(counters.stored_cookie_bytes, 3);
        assert_eq!(counters.cookie_request_packets, 2);
        assert_eq!(counters.cookie_response_hits, 1);
        assert_eq!(counters.cookie_response_misses, 1);
    }

    #[test]
    fn transfer_stores_latest_server_target() {
        let mut store = WorldStore::new();

        store.apply_transfer(ProtocolTransfer {
            host: "first.example.com".to_string(),
            port: 25565,
        });
        let latest = store.apply_transfer(ProtocolTransfer {
            host: "next.example.com".to_string(),
            port: 25566,
        });

        assert_eq!(
            latest,
            &TransferTargetState {
                host: "next.example.com".to_string(),
                port: 25566,
            }
        );
        assert_eq!(
            store.last_transfer(),
            Some(&TransferTargetState {
                host: "next.example.com".to_string(),
                port: 25566,
            })
        );
        assert_eq!(store.counters().transfer_packets, 2);
    }

    #[test]
    fn server_data_stores_motd_icon_and_counter() {
        let mut store = WorldStore::new();

        store.apply_server_data(ProtocolServerData {
            motd: "Welcome to BBB".to_string(),
            icon_bytes: Some(vec![137, 80, 78, 71]),
        });

        let server_data = store.server_data().expect("server data is stored");
        assert_eq!(server_data.motd, "Welcome to BBB");
        assert_eq!(server_data.icon_byte_len(), Some(4));
        assert_eq!(
            server_data.icon_bytes.as_deref(),
            Some(&[137, 80, 78, 71][..])
        );
        assert_eq!(store.counters().server_data_packets, 1);
    }

    #[test]
    fn resource_pack_push_stores_and_upserts_by_id() {
        let mut store = WorldStore::new();
        let id = Uuid::from_u128(0x11111111111111111111111111111111);

        store.apply_resource_pack_push(protocol_resource_pack_push(
            id,
            "https://example.test/first.zip",
            "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            false,
            Some("Use server pack?"),
        ));
        store.apply_resource_pack_push(protocol_resource_pack_push(
            id,
            "https://example.test/second.zip",
            "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
            true,
            None,
        ));

        let pack = store.resource_pack(id).expect("pack is tracked");
        assert_eq!(pack.url, "https://example.test/second.zip");
        assert_eq!(pack.hash, "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb");
        assert!(pack.required);
        assert_eq!(pack.prompt, None);
        assert_eq!(store.resource_packs().len(), 1);
        let counters = store.counters();
        assert_eq!(counters.resource_pack_push_packets, 2);
        assert_eq!(counters.resource_packs_tracked, 1);
    }

    #[test]
    fn resource_pack_response_updates_existing_pack_only() {
        let mut store = WorldStore::new();
        let id = Uuid::from_u128(0x11111111111111111111111111111111);
        let missing = Uuid::from_u128(0x22222222222222222222222222222222);

        store.apply_resource_pack_push(protocol_resource_pack_push(
            id,
            "https://example.test/required.zip",
            "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            true,
            Some("Required pack"),
        ));

        assert!(
            store.apply_resource_pack_response(id, ProtocolResourcePackResponseAction::Declined,)
        );
        let pack = store.resource_pack(id).expect("pack is tracked");
        assert_eq!(
            pack.last_response,
            Some(ResourcePackResponseState {
                action: ProtocolResourcePackResponseAction::Declined,
            })
        );

        assert!(!store
            .apply_resource_pack_response(missing, ProtocolResourcePackResponseAction::Accepted,));
        let counters = store.counters();
        assert_eq!(counters.resource_pack_response_packets, 2);
        assert_eq!(counters.resource_pack_response_updates_applied, 1);
        assert_eq!(counters.resource_pack_response_updates_ignored, 1);
        assert_eq!(counters.resource_pack_required_declines, 1);
        assert_eq!(counters.resource_packs_tracked, 1);
    }

    #[test]
    fn resource_pack_pop_removes_one_pack_by_id() {
        let mut store = WorldStore::new();
        let first = Uuid::from_u128(0x11111111111111111111111111111111);
        let second = Uuid::from_u128(0x22222222222222222222222222222222);

        store.apply_resource_pack_push(protocol_resource_pack_push(
            first,
            "https://example.test/first.zip",
            "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            false,
            None,
        ));
        store.apply_resource_pack_push(protocol_resource_pack_push(
            second,
            "https://example.test/second.zip",
            "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
            false,
            None,
        ));

        assert_eq!(
            store.apply_resource_pack_pop(ProtocolResourcePackPop { id: Some(first) }),
            1
        );
        assert!(store.resource_pack(first).is_none());
        assert!(store.resource_pack(second).is_some());
        let counters = store.counters();
        assert_eq!(counters.resource_pack_pop_packets, 1);
        assert_eq!(counters.resource_pack_pop_updates_applied, 1);
        assert_eq!(counters.resource_pack_pop_updates_ignored, 0);
        assert_eq!(counters.resource_packs_tracked, 1);

        assert_eq!(
            store.apply_resource_pack_pop(ProtocolResourcePackPop { id: Some(first) }),
            0
        );
        let counters = store.counters();
        assert_eq!(counters.resource_pack_pop_packets, 2);
        assert_eq!(counters.resource_pack_pop_updates_applied, 1);
        assert_eq!(counters.resource_pack_pop_updates_ignored, 1);
        assert_eq!(counters.resource_packs_tracked, 1);
    }

    #[test]
    fn resource_pack_pop_without_id_clears_all_packs() {
        let mut store = WorldStore::new();
        let first = Uuid::from_u128(0x11111111111111111111111111111111);
        let second = Uuid::from_u128(0x22222222222222222222222222222222);

        store.apply_resource_pack_push(protocol_resource_pack_push(
            first,
            "https://example.test/first.zip",
            "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            false,
            None,
        ));
        store.apply_resource_pack_push(protocol_resource_pack_push(
            second,
            "https://example.test/second.zip",
            "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
            true,
            Some("Required pack"),
        ));

        assert_eq!(
            store.apply_resource_pack_pop(ProtocolResourcePackPop { id: None }),
            2
        );
        assert!(store.resource_packs().is_empty());
        let counters = store.counters();
        assert_eq!(counters.resource_pack_pop_packets, 1);
        assert_eq!(counters.resource_pack_pop_updates_applied, 1);
        assert_eq!(counters.resource_pack_pop_updates_ignored, 0);
        assert_eq!(counters.resource_packs_tracked, 0);

        assert_eq!(
            store.apply_resource_pack_pop(ProtocolResourcePackPop { id: None }),
            0
        );
        let counters = store.counters();
        assert_eq!(counters.resource_pack_pop_packets, 2);
        assert_eq!(counters.resource_pack_pop_updates_applied, 1);
        assert_eq!(counters.resource_pack_pop_updates_ignored, 1);
        assert_eq!(counters.resource_packs_tracked, 0);
    }

    #[test]
    fn server_links_replace_trusted_links_and_count_invalid_entries() {
        let mut store = WorldStore::new();

        let invalid = store.apply_server_links(ProtocolServerLinks {
            links: vec![
                ProtocolServerLinkEntry {
                    link_type: ServerLinkType::Known(ServerLinkKnownType::Support),
                    url: "https://example.invalid/support".to_string(),
                },
                ProtocolServerLinkEntry {
                    link_type: ServerLinkType::Custom {
                        label: "Rules".to_string(),
                    },
                    url: "http://example.invalid/rules".to_string(),
                },
                ProtocolServerLinkEntry {
                    link_type: ServerLinkType::Known(ServerLinkKnownType::Website),
                    url: "ftp://example.invalid/file".to_string(),
                },
            ],
        });

        assert_eq!(invalid, 1);
        assert_eq!(
            store.server_links(),
            &[
                ServerLinkState {
                    label: "known_server_link.support".to_string(),
                    url: "https://example.invalid/support".to_string(),
                    known_type: Some("support".to_string()),
                },
                ServerLinkState {
                    label: "Rules".to_string(),
                    url: "http://example.invalid/rules".to_string(),
                    known_type: None,
                },
            ]
        );
        let counters = store.counters();
        assert_eq!(counters.server_link_packets, 1);
        assert_eq!(counters.server_links_tracked, 2);
        assert_eq!(counters.server_link_invalid_entries, 1);

        store.apply_server_links(ProtocolServerLinks {
            links: vec![ProtocolServerLinkEntry {
                link_type: ServerLinkType::Known(ServerLinkKnownType::BugReport),
                url: "https://example.invalid/bug".to_string(),
            }],
        });

        assert_eq!(store.server_links().len(), 1);
        assert_eq!(
            store.server_links()[0].known_type.as_deref(),
            Some("report_bug")
        );
        let counters = store.counters();
        assert_eq!(counters.server_link_packets, 2);
        assert_eq!(counters.server_links_tracked, 1);
        assert_eq!(counters.server_link_invalid_entries, 1);
    }

    #[test]
    fn custom_report_details_replace_current_details() {
        let mut store = WorldStore::new();
        let details = BTreeMap::from([
            ("Region".to_string(), "local".to_string()),
            ("Server".to_string(), "bbb test shard".to_string()),
        ]);

        store.apply_custom_report_details(CustomReportDetails {
            details: details.clone(),
        });

        assert_eq!(store.custom_report_details(), &details);
        let counters = store.counters();
        assert_eq!(counters.custom_report_detail_packets, 1);
        assert_eq!(counters.custom_report_details_tracked, 2);

        store.apply_custom_report_details(CustomReportDetails {
            details: BTreeMap::new(),
        });

        assert!(store.custom_report_details().is_empty());
        let counters = store.counters();
        assert_eq!(counters.custom_report_detail_packets, 2);
        assert_eq!(counters.custom_report_details_tracked, 0);
    }

    fn protocol_resource_pack_push(
        id: Uuid,
        url: &str,
        hash: &str,
        required: bool,
        prompt: Option<&str>,
    ) -> ProtocolResourcePackPush {
        ProtocolResourcePackPush {
            id,
            url: url.to_string(),
            hash: hash.to_string(),
            required,
            prompt: prompt.map(str::to_string),
        }
    }
}
