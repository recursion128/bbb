use std::collections::BTreeMap;

use bbb_protocol::packets::{
    ResourcePackPop as ProtocolResourcePackPop, ResourcePackPush as ProtocolResourcePackPush,
    ServerData as ProtocolServerData,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::WorldStore;

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ServerPresentationState {
    pub server_data: Option<ServerDataState>,
    pub resource_packs: BTreeMap<Uuid, ResourcePackState>,
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResourcePackState {
    pub id: Uuid,
    pub url: String,
    pub hash: String,
    pub required: bool,
    pub prompt: Option<String>,
}

impl WorldStore {
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
        };
        self.presentation.resource_packs.insert(pack.id, pack);
        self.update_resource_pack_count();
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
        self.update_resource_pack_count();
        removed
    }

    pub fn presentation(&self) -> &ServerPresentationState {
        &self.presentation
    }

    pub fn server_data(&self) -> Option<&ServerDataState> {
        self.presentation.server_data.as_ref()
    }

    pub fn resource_packs(&self) -> &BTreeMap<Uuid, ResourcePackState> {
        &self.presentation.resource_packs
    }

    pub fn resource_pack(&self, id: Uuid) -> Option<&ResourcePackState> {
        self.presentation.resource_packs.get(&id)
    }

    fn update_resource_pack_count(&mut self) {
        self.counters.resource_packs_tracked = self.presentation.resource_packs.len();
    }
}

fn non_empty_component_string(component: Option<String>) -> Option<String> {
    component.filter(|value| !value.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;

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
        assert_eq!(counters.resource_packs_tracked, 0);
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
