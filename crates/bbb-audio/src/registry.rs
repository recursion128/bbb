use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SoundEventRegistry {
    by_protocol_id: Vec<String>,
}

impl SoundEventRegistry {
    pub fn from_ids(ids: impl IntoIterator<Item = impl Into<String>>) -> Self {
        Self {
            by_protocol_id: ids.into_iter().map(Into::into).collect(),
        }
    }

    pub fn event_id(&self, registry_id: i32) -> Option<&str> {
        let index = usize::try_from(registry_id).ok()?;
        self.by_protocol_id.get(index).map(String::as_str)
    }

    pub fn len(&self) -> usize {
        self.by_protocol_id.len()
    }

    pub fn is_empty(&self) -> bool {
        self.by_protocol_id.is_empty()
    }
}
