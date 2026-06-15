use bbb_protocol::packets::TagQuery as ProtocolTagQuery;
use serde::{Deserialize, Serialize};

use crate::WorldStore;

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClientDebugQueryState {
    #[serde(default)]
    pub last_tag_query: Option<TagQueryResponseState>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TagQueryResponseState {
    pub transaction_id: i32,
    pub tag_present: bool,
    pub raw_nbt: Vec<u8>,
}

impl TagQueryResponseState {
    pub fn raw_nbt_len(&self) -> usize {
        self.raw_nbt.len()
    }
}

impl WorldStore {
    pub fn apply_tag_query(&mut self, packet: ProtocolTagQuery) {
        self.counters.tag_query_packets += 1;
        self.debug_query.last_tag_query = Some(TagQueryResponseState {
            transaction_id: packet.transaction_id,
            tag_present: packet.tag_present,
            raw_nbt: packet.raw_nbt,
        });
    }

    pub fn debug_query(&self) -> &ClientDebugQueryState {
        &self.debug_query
    }

    pub fn client_debug_query(&self) -> &ClientDebugQueryState {
        self.debug_query()
    }

    pub fn last_tag_query(&self) -> Option<&TagQueryResponseState> {
        self.debug_query.last_tag_query.as_ref()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bbb_protocol::packets::TagQuery;

    #[test]
    fn tag_query_stores_latest_response_and_counter() {
        let mut store = WorldStore::new();

        store.apply_tag_query(TagQuery {
            transaction_id: 7,
            tag_present: true,
            raw_nbt: vec![10, 0, 0],
        });

        assert_eq!(
            store.last_tag_query(),
            Some(&TagQueryResponseState {
                transaction_id: 7,
                tag_present: true,
                raw_nbt: vec![10, 0, 0],
            })
        );
        assert_eq!(store.last_tag_query().unwrap().raw_nbt_len(), 3);
        assert_eq!(store.counters().tag_query_packets, 1);

        store.apply_tag_query(TagQuery {
            transaction_id: 8,
            tag_present: false,
            raw_nbt: vec![0],
        });

        assert_eq!(
            store.client_debug_query().last_tag_query,
            Some(TagQueryResponseState {
                transaction_id: 8,
                tag_present: false,
                raw_nbt: vec![0],
            })
        );
        assert_eq!(store.counters().tag_query_packets, 2);
    }
}
