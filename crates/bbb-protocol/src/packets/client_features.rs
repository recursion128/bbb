use serde::{Deserialize, Serialize};

use crate::codec::{Decoder, ProtocolError, Result};

mod recipes;

pub use recipes::*;
pub(crate) use recipes::{
    decode_place_ghost_recipe, decode_recipe_book_add, decode_recipe_book_remove,
    decode_recipe_book_settings, decode_update_recipes,
};

const MAX_CUSTOM_CHAT_COMPLETIONS: usize = 8192;
const MAX_TAG_QUERY_NBT_DEPTH: usize = 64;
const MAX_TAG_QUERY_NBT_LIST_ITEMS: usize = 1_000_000;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CustomChatCompletions {
    pub action: CustomChatCompletionsAction,
    pub entries: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CustomChatCompletionsAction {
    Add,
    Remove,
    Set,
}

impl CustomChatCompletionsAction {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Add => "add",
            Self::Remove => "remove",
            Self::Set => "set",
        }
    }

    fn from_ordinal(ordinal: i32) -> Result<Self> {
        Ok(match ordinal {
            0 => Self::Add,
            1 => Self::Remove,
            2 => Self::Set,
            other => {
                return Err(ProtocolError::InvalidData(format!(
                    "invalid custom chat completions action ordinal {other}"
                )))
            }
        })
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

pub(crate) fn decode_custom_chat_completions(
    decoder: &mut Decoder<'_>,
) -> Result<CustomChatCompletions> {
    let action = CustomChatCompletionsAction::from_ordinal(decoder.read_var_i32()?)?;
    let count = decoder.read_len()?;
    if count > MAX_CUSTOM_CHAT_COMPLETIONS {
        return Err(ProtocolError::PacketTooLarge(
            count,
            MAX_CUSTOM_CHAT_COMPLETIONS,
        ));
    }

    let mut entries = Vec::with_capacity(count);
    for _ in 0..count {
        entries.push(decoder.read_string(32767)?);
    }

    Ok(CustomChatCompletions { action, entries })
}

pub(crate) fn decode_select_advancements_tab(
    decoder: &mut Decoder<'_>,
) -> Result<SelectAdvancementsTab> {
    let tab = if decoder.read_bool()? {
        Some(decoder.read_string(32767)?)
    } else {
        None
    };

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

fn validate_tag_query_nbt(raw_nbt: &[u8]) -> Result<bool> {
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

    skip_nbt_payload(&mut decoder, tag_id, 0)?;
    if !decoder.is_empty() {
        return Err(ProtocolError::InvalidData(
            "trailing bytes after tag query nbt".to_string(),
        ));
    }
    Ok(true)
}

fn skip_nbt_payload(decoder: &mut Decoder<'_>, tag_id: u8, depth: usize) -> Result<()> {
    if depth > MAX_TAG_QUERY_NBT_DEPTH {
        return Err(ProtocolError::InvalidData(
            "tag query nbt exceeded max depth".to_string(),
        ));
    }

    match tag_id {
        1 => {
            decoder.read_exact(1, "nbt byte")?;
        }
        2 => {
            decoder.read_exact(2, "nbt short")?;
        }
        3 | 5 => {
            decoder.read_exact(4, "nbt int/float")?;
        }
        4 | 6 => {
            decoder.read_exact(8, "nbt long/double")?;
        }
        7 => {
            let len = read_nbt_len(decoder)?;
            decoder.read_exact(len, "nbt byte array")?;
        }
        8 => {
            skip_modified_utf8(decoder)?;
        }
        9 => {
            let element_type = decoder.read_u8()?;
            let len = read_nbt_len(decoder)?;
            if len > MAX_TAG_QUERY_NBT_LIST_ITEMS {
                return Err(ProtocolError::PacketTooLarge(
                    len,
                    MAX_TAG_QUERY_NBT_LIST_ITEMS,
                ));
            }
            if element_type == 0 && len > 0 {
                return Err(ProtocolError::InvalidData(
                    "non-empty tag query nbt list has end tag element type".to_string(),
                ));
            }
            for _ in 0..len {
                skip_nbt_payload(decoder, element_type, depth + 1)?;
            }
        }
        10 => loop {
            let nested_type = decoder.read_u8()?;
            if nested_type == 0 {
                break;
            }
            skip_modified_utf8(decoder)?;
            skip_nbt_payload(decoder, nested_type, depth + 1)?;
        },
        11 => {
            let len = read_nbt_len(decoder)?;
            let byte_len = len.checked_mul(4).ok_or_else(|| {
                ProtocolError::InvalidData("nbt int array length overflow".to_string())
            })?;
            decoder.read_exact(byte_len, "nbt int array")?;
        }
        12 => {
            let len = read_nbt_len(decoder)?;
            let byte_len = len.checked_mul(8).ok_or_else(|| {
                ProtocolError::InvalidData("nbt long array length overflow".to_string())
            })?;
            decoder.read_exact(byte_len, "nbt long array")?;
        }
        other => {
            return Err(ProtocolError::InvalidData(format!(
                "invalid tag query nbt tag id {other}"
            )))
        }
    }
    Ok(())
}

fn read_nbt_len(decoder: &mut Decoder<'_>) -> Result<usize> {
    let len = decoder.read_i32()?;
    if len < 0 {
        return Err(ProtocolError::NegativeLength(len));
    }
    Ok(len as usize)
}

fn skip_modified_utf8(decoder: &mut Decoder<'_>) -> Result<()> {
    let len = decoder.read_u16()? as usize;
    decoder.read_exact(len, "nbt string")?;
    Ok(())
}

#[cfg(test)]
mod tests;
