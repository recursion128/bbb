use serde::{Deserialize, Serialize};

use crate::codec::{Decoder, Encoder, ProtocolError, Result};
use crate::ids;
use crate::nbt_summary::{encode_nbt_summary_bounded, MAX_NBT_DEPTH};
use crate::NbtSummaryValue;
pub use bbb_facts::custom_chat_completions_actions::CustomChatCompletionsAction;

use super::read_resource_location;

mod advancements;
mod recipes;

pub use advancements::*;
pub use recipes::*;
pub(crate) use recipes::{decode_recipe_book_remove, decode_recipe_book_settings};

const MAX_CUSTOM_CHAT_COMPLETIONS: usize = 8192;
const MAX_TAG_QUERY_NBT_DEPTH: usize = 64;
const MAX_TAG_QUERY_NBT_LIST_ITEMS: usize = 1_000_000;
/// `writeNbt` itself is unbounded (`FriendlyByteBuf.java:526-535`); this only
/// stops a runaway encode short of the logical packet bound.
const MAX_TAG_QUERY_NBT_BYTES: usize = 8 * 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CustomChatCompletions {
    pub action: CustomChatCompletionsAction,
    pub entries: Vec<String>,
}

pub fn custom_chat_completions_action_name(action: CustomChatCompletionsAction) -> &'static str {
    match action {
        CustomChatCompletionsAction::Add => "add",
        CustomChatCompletionsAction::Remove => "remove",
        CustomChatCompletionsAction::Set => "set",
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SelectAdvancementsTab {
    pub tab: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TagQuery {
    pub transaction_id: i32,
    pub tag_present: bool,
    pub raw_nbt: Vec<u8>,
}

impl TagQuery {
    /// `new ClientboundTagQueryPacket(transactionId, tag)`
    /// (`ClientboundTagQueryPacket.java:17-20`) in the form `write` puts on the
    /// wire: `None` is the null tag, which `writeNbt` sends as a bare `EndTag`
    /// (`FriendlyByteBuf.java:526-529`).
    pub fn from_tag(transaction_id: i32, tag: Option<&NbtSummaryValue>) -> Result<Self> {
        if tag.is_some_and(|tag| !matches!(tag, NbtSummaryValue::Compound(_))) {
            return Err(ProtocolError::InvalidData(
                "tag query nbt root must be a compound".to_string(),
            ));
        }
        Ok(Self {
            transaction_id,
            tag_present: tag.is_some(),
            raw_nbt: encode_nbt_summary_bounded(tag, MAX_TAG_QUERY_NBT_BYTES, MAX_NBT_DEPTH)?,
        })
    }
}

pub fn encode_play_clientbound_custom_chat_completions(
    update: &CustomChatCompletions,
) -> Result<(i32, Vec<u8>)> {
    if update.entries.len() > MAX_CUSTOM_CHAT_COMPLETIONS {
        return Err(ProtocolError::PacketTooLarge(
            update.entries.len(),
            MAX_CUSTOM_CHAT_COMPLETIONS,
        ));
    }
    let mut encoder = Encoder::new();
    encoder.write_var_i32(update.action.ordinal());
    encoder.write_var_i32(update.entries.len() as i32);
    for entry in &update.entries {
        let utf16_len = entry.encode_utf16().count();
        if utf16_len > 32767 {
            return Err(ProtocolError::StringTooLong {
                actual: utf16_len,
                max: 32767,
            });
        }
        encoder.write_string(entry);
    }
    Ok((
        ids::play::CLIENTBOUND_CUSTOM_CHAT_COMPLETIONS,
        encoder.into_inner(),
    ))
}

pub(crate) fn decode_custom_chat_completions(
    decoder: &mut Decoder<'_>,
) -> Result<CustomChatCompletions> {
    let ordinal = decoder.read_var_i32()?;
    let action = CustomChatCompletionsAction::from_ordinal(ordinal).ok_or_else(|| {
        ProtocolError::InvalidData(format!(
            "invalid custom chat completions action ordinal {ordinal}"
        ))
    })?;
    let count = decoder.read_len()?;
    if count > MAX_CUSTOM_CHAT_COMPLETIONS {
        return Err(ProtocolError::PacketTooLarge(
            count,
            MAX_CUSTOM_CHAT_COMPLETIONS,
        ));
    }

    let mut entries = Vec::with_capacity(decoder.bounded_capacity(count, 1));
    for _ in 0..count {
        entries.push(decoder.read_string(32767)?);
    }

    if !decoder.is_empty() {
        return Err(ProtocolError::InvalidData(
            "trailing bytes after custom chat completions packet".to_string(),
        ));
    }
    Ok(CustomChatCompletions { action, entries })
}

pub(crate) fn decode_select_advancements_tab(
    decoder: &mut Decoder<'_>,
) -> Result<SelectAdvancementsTab> {
    let tab = if decoder.read_bool()? {
        Some(read_resource_location(decoder)?)
    } else {
        None
    };

    if !decoder.is_empty() {
        return Err(ProtocolError::InvalidData(
            "trailing bytes after select advancements tab packet".to_string(),
        ));
    }
    Ok(SelectAdvancementsTab { tab })
}

pub(crate) fn decode_tag_query(decoder: &mut Decoder<'_>) -> Result<TagQuery> {
    let transaction_id = decoder.read_var_i32()?;
    let nbt_len = decoder.remaining_len();
    if nbt_len == 0 {
        return Err(ProtocolError::UnexpectedEof {
            what: "tag query nbt",
        });
    }
    let raw_nbt = decoder.read_exact(nbt_len, "tag query nbt")?.to_vec();
    let tag_present = validate_tag_query_nbt(&raw_nbt)?;

    Ok(TagQuery {
        transaction_id,
        tag_present,
        raw_nbt,
    })
}

pub(super) fn validate_tag_query_nbt(raw_nbt: &[u8]) -> Result<bool> {
    let mut decoder = Decoder::new(raw_nbt);
    let tag_id = decoder.read_u8()?;
    if tag_id == 0 {
        if !decoder.is_empty() {
            return Err(ProtocolError::InvalidData(
                "trailing bytes after null tag query nbt".to_string(),
            ));
        }
        return Ok(false);
    }
    if tag_id != 10 {
        return Err(ProtocolError::InvalidData(format!(
            "tag query nbt root must be compound or end tag, got {tag_id}"
        )));
    }

    crate::nbt_summary::skip_network_nbt_payload(
        &mut decoder,
        tag_id,
        crate::nbt_summary::SkipNbtLimits {
            max_depth: MAX_TAG_QUERY_NBT_DEPTH,
            max_list_items: MAX_TAG_QUERY_NBT_LIST_ITEMS,
            max_array_items: usize::MAX,
            max_compound_entries: usize::MAX,
            max_total_bytes: None,
            validate_mutf8: true,
            label: "tag query nbt",
        },
    )?;
    if !decoder.is_empty() {
        return Err(ProtocolError::InvalidData(
            "trailing bytes after tag query nbt".to_string(),
        ));
    }
    Ok(true)
}

#[cfg(test)]
mod tests;
