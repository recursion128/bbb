use serde::{Deserialize, Serialize};

use crate::codec::{Decoder, ProtocolError, Result};

use super::read_resource_location;

const MAX_TAG_REGISTRIES: usize = 1024;
const MAX_TAGS_PER_REGISTRY: usize = 65_536;
const MAX_TAG_ENTRIES: usize = 1_000_000;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UpdateTags {
    pub registries: Vec<RegistryTags>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RegistryTags {
    pub registry: String,
    pub tags: Vec<TagNetworkPayload>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TagNetworkPayload {
    pub tag: String,
    pub entries: Vec<i32>,
}

pub(crate) fn decode_update_tags(decoder: &mut Decoder<'_>) -> Result<UpdateTags> {
    let registry_count = decoder.read_len()?;
    if registry_count > MAX_TAG_REGISTRIES {
        return Err(ProtocolError::PacketTooLarge(
            registry_count,
            MAX_TAG_REGISTRIES,
        ));
    }

    let mut registries = Vec::with_capacity(registry_count);
    for _ in 0..registry_count {
        registries.push(decode_registry_tags(decoder)?);
    }

    if !decoder.is_empty() {
        return Err(ProtocolError::InvalidData(
            "trailing bytes after update tags packet".to_string(),
        ));
    }
    Ok(UpdateTags { registries })
}

fn decode_registry_tags(decoder: &mut Decoder<'_>) -> Result<RegistryTags> {
    let registry = read_resource_location(decoder)?;
    let tag_count = decoder.read_len()?;
    if tag_count > MAX_TAGS_PER_REGISTRY {
        return Err(ProtocolError::PacketTooLarge(
            tag_count,
            MAX_TAGS_PER_REGISTRY,
        ));
    }

    let mut tags = Vec::with_capacity(tag_count);
    for _ in 0..tag_count {
        tags.push(decode_tag_payload(decoder)?);
    }
    Ok(RegistryTags { registry, tags })
}

fn decode_tag_payload(decoder: &mut Decoder<'_>) -> Result<TagNetworkPayload> {
    let tag = read_resource_location(decoder)?;
    let entry_count = decoder.read_len()?;
    if entry_count > MAX_TAG_ENTRIES {
        return Err(ProtocolError::PacketTooLarge(entry_count, MAX_TAG_ENTRIES));
    }

    let mut entries = Vec::with_capacity(entry_count);
    for _ in 0..entry_count {
        entries.push(decoder.read_var_i32()?);
    }
    Ok(TagNetworkPayload { tag, entries })
}

#[cfg(test)]
mod tests;
