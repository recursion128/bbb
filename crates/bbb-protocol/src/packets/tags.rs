use serde::{Deserialize, Serialize};

use crate::codec::{Decoder, Encoder, ProtocolError, Result};

use super::{normalize_resource_location, read_resource_location};

const MAX_TAG_REGISTRIES: usize = 1024;
const MAX_TAGS_PER_REGISTRY: usize = 65_536;
const MAX_TAG_ENTRIES: usize = 1_000_000;
const MAX_RESOURCE_LOCATION_CHARS: usize = 32_767;

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

pub(crate) fn encode_update_tags(update_tags: &UpdateTags) -> Result<Vec<u8>> {
    check_collection_len(update_tags.registries.len(), MAX_TAG_REGISTRIES)?;

    let mut encoder = Encoder::new();
    encoder.write_var_i32(update_tags.registries.len() as i32);
    for registry in &update_tags.registries {
        write_resource_location(&mut encoder, &registry.registry)?;
        check_collection_len(registry.tags.len(), MAX_TAGS_PER_REGISTRY)?;
        encoder.write_var_i32(registry.tags.len() as i32);
        for tag in &registry.tags {
            write_resource_location(&mut encoder, &tag.tag)?;
            check_collection_len(tag.entries.len(), MAX_TAG_ENTRIES)?;
            encoder.write_var_i32(tag.entries.len() as i32);
            for entry in &tag.entries {
                if *entry < 0 {
                    return Err(ProtocolError::InvalidData(format!(
                        "tag {} in registry {} contains negative holder id {entry}",
                        tag.tag, registry.registry
                    )));
                }
                encoder.write_var_i32(*entry);
            }
        }
    }
    Ok(encoder.into_inner())
}

fn check_collection_len(actual: usize, max: usize) -> Result<()> {
    if actual > max {
        Err(ProtocolError::PacketTooLarge(actual, max))
    } else {
        Ok(())
    }
}

fn write_resource_location(encoder: &mut Encoder, raw: &str) -> Result<()> {
    let resource = normalize_resource_location(raw.to_string())?;
    let actual = resource.chars().count();
    if actual > MAX_RESOURCE_LOCATION_CHARS {
        return Err(ProtocolError::StringTooLong {
            actual,
            max: MAX_RESOURCE_LOCATION_CHARS,
        });
    }
    encoder.write_string(&resource);
    Ok(())
}

pub(crate) fn decode_update_tags(decoder: &mut Decoder<'_>) -> Result<UpdateTags> {
    let registry_count = decoder.read_len()?;
    if registry_count > MAX_TAG_REGISTRIES {
        return Err(ProtocolError::PacketTooLarge(
            registry_count,
            MAX_TAG_REGISTRIES,
        ));
    }

    let mut registries = Vec::with_capacity(decoder.bounded_capacity(registry_count, 3));
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

    let mut tags = Vec::with_capacity(decoder.bounded_capacity(tag_count, 3));
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

    let mut entries = Vec::with_capacity(decoder.bounded_capacity(entry_count, 1));
    for _ in 0..entry_count {
        entries.push(decoder.read_var_i32()?);
    }
    Ok(TagNetworkPayload { tag, entries })
}

#[cfg(test)]
mod tests;
