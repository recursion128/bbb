use serde::{Deserialize, Serialize};

use crate::codec::{Decoder, ProtocolError, Result};

use super::read_resource_key;

const MAX_CUSTOM_PAYLOAD: usize = 1024 * 1024;
const MAX_DIALOG_PAYLOAD: usize = 2 * 1024 * 1024;

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
    Direct { raw_dialog_payload: Vec<u8> },
}

pub(crate) fn decode_custom_payload(decoder: &mut Decoder<'_>) -> Result<CustomPayload> {
    let id = read_resource_key(decoder)?;
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
            raw_dialog_payload: decode_remaining_payload(
                decoder,
                MAX_DIALOG_PAYLOAD,
                "direct dialog payload",
            )?,
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
            raw_dialog_payload: decode_remaining_payload(
                decoder,
                MAX_DIALOG_PAYLOAD,
                "context-free dialog payload",
            )?,
        },
    })
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
