use serde::{Deserialize, Serialize};

use crate::{
    codec::{Decoder, Encoder, ProtocolError, Result},
    nbt_summary::{decode_nbt_summary_from_decoder, encode_nbt_summary_bounded},
    NbtSummaryValue,
};

use super::{normalize_resource_location, read_resource_location};

const MAX_CUSTOM_PAYLOAD: usize = 1024 * 1024;
const MAX_DIALOG_PAYLOAD: usize = 2 * 1024 * 1024;
const MAX_CUSTOM_CLICK_ACTION_ID_CHARS: usize = 32_767;
pub(crate) const MAX_CUSTOM_CLICK_ACTION_TAG_BYTES: usize = 65_536;
pub(crate) const MAX_CUSTOM_CLICK_ACTION_TAG_DEPTH: usize = 16;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CustomPayload {
    pub id: String,
    pub payload: CustomPayloadBody,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum CustomPayloadBody {
    Brand { brand: String },
    Unknown { raw_payload: Vec<u8> },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ShowDialog {
    pub dialog: DialogHolder,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DialogHolder {
    Reference { registry_id: i32 },
    Direct { dialog: NbtSummaryValue },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CustomClickAction {
    pub id: String,
    #[serde(default)]
    pub payload: Option<NbtSummaryValue>,
}

pub(crate) fn decode_custom_payload(decoder: &mut Decoder<'_>) -> Result<CustomPayload> {
    let id = read_resource_location(decoder)?;
    let payload = if id == "minecraft:brand" {
        let brand = decoder.read_string(32767)?;
        if !decoder.is_empty() {
            return Err(ProtocolError::InvalidData(
                "trailing bytes after brand custom payload".to_string(),
            ));
        }
        CustomPayloadBody::Brand { brand }
    } else {
        let raw_payload = decode_remaining_payload(decoder, MAX_CUSTOM_PAYLOAD, "custom payload")?;
        CustomPayloadBody::Unknown { raw_payload }
    };

    Ok(CustomPayload { id, payload })
}

pub(crate) fn decode_clear_dialog(decoder: &Decoder<'_>) -> Result<()> {
    if !decoder.is_empty() {
        return Err(ProtocolError::InvalidData(
            "trailing bytes after clear dialog packet".to_string(),
        ));
    }
    Ok(())
}

pub(crate) fn decode_show_dialog(decoder: &mut Decoder<'_>) -> Result<ShowDialog> {
    let holder_id = decoder.read_var_i32()?;
    let dialog = if holder_id == 0 {
        DialogHolder::Direct {
            dialog: decode_direct_dialog(decoder, "direct dialog payload")?,
        }
    } else if holder_id > 0 {
        if !decoder.is_empty() {
            return Err(ProtocolError::InvalidData(
                "trailing bytes after referenced dialog holder".to_string(),
            ));
        }
        DialogHolder::Reference {
            registry_id: holder_id - 1,
        }
    } else {
        return Err(ProtocolError::InvalidData(format!(
            "invalid dialog holder id {holder_id}"
        )));
    };

    Ok(ShowDialog { dialog })
}

pub(crate) fn decode_context_free_show_dialog(decoder: &mut Decoder<'_>) -> Result<ShowDialog> {
    Ok(ShowDialog {
        dialog: DialogHolder::Direct {
            dialog: decode_direct_dialog(decoder, "context-free dialog payload")?,
        },
    })
}

fn decode_direct_dialog(decoder: &mut Decoder<'_>, what: &'static str) -> Result<NbtSummaryValue> {
    let len = decoder.remaining_len();
    if len > MAX_DIALOG_PAYLOAD {
        return Err(ProtocolError::PacketTooLarge(len, MAX_DIALOG_PAYLOAD));
    }
    let dialog = decode_nbt_summary_from_decoder(decoder)?
        .ok_or_else(|| ProtocolError::InvalidData(format!("{what} cannot be an end tag")))?;
    if !matches!(dialog, NbtSummaryValue::Compound(_)) {
        return Err(ProtocolError::InvalidData(format!(
            "{what} root must be a compound tag"
        )));
    }
    if !decoder.is_empty() {
        return Err(ProtocolError::InvalidData(format!(
            "trailing bytes after {what}"
        )));
    }
    Ok(dialog)
}

pub(super) fn encode_custom_click_action_payload(packet: &CustomClickAction) -> Result<Vec<u8>> {
    let id = normalize_resource_location(packet.id.clone())?;
    if id.len() > MAX_CUSTOM_CLICK_ACTION_ID_CHARS {
        return Err(ProtocolError::StringTooLong {
            actual: id.len(),
            max: MAX_CUSTOM_CLICK_ACTION_ID_CHARS,
        });
    }
    let tag = encode_nbt_summary_bounded(
        packet.payload.as_ref(),
        MAX_CUSTOM_CLICK_ACTION_TAG_BYTES,
        MAX_CUSTOM_CLICK_ACTION_TAG_DEPTH,
    )?;
    let mut out = Encoder::new();
    out.write_string(&id);
    out.write_var_i32(tag.len() as i32);
    out.write_bytes(&tag);
    Ok(out.into_inner())
}

fn decode_remaining_payload(
    decoder: &mut Decoder<'_>,
    max_len: usize,
    what: &'static str,
) -> Result<Vec<u8>> {
    let len = decoder.remaining_len();
    if len > max_len {
        return Err(ProtocolError::PacketTooLarge(len, max_len));
    }

    Ok(decoder.read_exact(len, what)?.to_vec())
}

#[cfg(test)]
mod tests;
