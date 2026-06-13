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
