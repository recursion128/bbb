use serde::{Deserialize, Serialize};

use crate::{
    codec::{Decoder, Encoder, ProtocolError, Result},
    ids,
    nbt_summary::{
        decode_nbt_summary_from_decoder, decode_nbt_summary_from_decoder_with_limits,
        encode_nbt_summary_bounded, NbtSummaryDecodeLimits, MAX_NBT_DEPTH,
    },
    NbtSummaryValue,
};

use super::{normalize_resource_location, read_resource_location};

/// `ClientboundCustomPayloadPacket.MAX_PAYLOAD_SIZE` (`ClientboundCustomPayloadPacket.java:16`).
pub(crate) const MAX_CLIENTBOUND_CUSTOM_PAYLOAD: usize = 1024 * 1024;
/// `ServerboundCustomPayloadPacket.MAX_PAYLOAD_SIZE` (`ServerboundCustomPayloadPacket.java:14`).
pub(crate) const MAX_SERVERBOUND_CUSTOM_PAYLOAD: usize = 32_767;
const MAX_BRAND_UTF16_UNITS: usize = 32_767;
const MAX_DIALOG_PAYLOAD: usize = 2 * 1024 * 1024;
const MAX_CUSTOM_CLICK_ACTION_ID_CHARS: usize = 32_767;
/// `ServerboundCustomClickActionPacket.UNTRUSTED_TAG_CODEC` (`ServerboundCustomClickActionPacket.java:14-15`).
pub(crate) const MAX_CUSTOM_CLICK_ACTION_TAG_BYTES: usize = 65_536;
pub(crate) const MAX_CUSTOM_CLICK_ACTION_TAG_DEPTH: usize = 16;
pub(crate) const CUSTOM_CLICK_ACTION_NBT_QUOTA: u64 = 32_768;

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
    ContextFree { dialog: NbtSummaryValue },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CustomClickAction {
    pub id: String,
    #[serde(default)]
    pub payload: Option<NbtSummaryValue>,
}

pub(crate) fn decode_custom_payload(
    decoder: &mut Decoder<'_>,
    max_payload_len: usize,
) -> Result<CustomPayload> {
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
        let raw_payload = decode_remaining_payload(decoder, max_payload_len, "custom payload")?;
        CustomPayloadBody::Unknown { raw_payload }
    };

    Ok(CustomPayload { id, payload })
}

pub(super) fn encode_custom_payload_bytes(
    channel: &str,
    raw_payload: &[u8],
    max_len: usize,
) -> Result<Vec<u8>> {
    let channel = normalize_resource_location(channel)?;
    if raw_payload.len() > max_len {
        return Err(ProtocolError::PacketTooLarge(raw_payload.len(), max_len));
    }
    let mut encoder = Encoder::new();
    encoder.write_string(&channel);
    encoder.write_bytes(raw_payload);
    Ok(encoder.into_inner())
}

pub fn encode_play_clientbound_custom_payload(payload: &CustomPayload) -> Result<(i32, Vec<u8>)> {
    let bytes = match &payload.payload {
        CustomPayloadBody::Brand { brand } => {
            let units = brand.encode_utf16().count();
            if units > MAX_BRAND_UTF16_UNITS {
                return Err(ProtocolError::InvalidData(format!(
                    "server brand has {units} UTF-16 code units, maximum is {MAX_BRAND_UTF16_UNITS}"
                )));
            }
            let mut encoder = Encoder::new();
            encoder.write_string("minecraft:brand");
            encoder.write_string(brand);
            encoder.into_inner()
        }
        CustomPayloadBody::Unknown { raw_payload } => {
            encode_custom_payload_bytes(&payload.id, raw_payload, MAX_CLIENTBOUND_CUSTOM_PAYLOAD)?
        }
    };
    Ok((ids::play::CLIENTBOUND_CUSTOM_PAYLOAD, bytes))
}

pub(crate) fn decode_clear_dialog(decoder: &Decoder<'_>) -> Result<()> {
    if !decoder.is_empty() {
        return Err(ProtocolError::InvalidData(
            "trailing bytes after clear dialog packet".to_string(),
        ));
    }
    Ok(())
}

/// `ClientboundClearDialogPacket.STREAM_CODEC` (`ClientboundClearDialogPacket.java:9-10`)
/// is `StreamCodec.unit`, so the play payload is empty.
pub fn encode_play_clientbound_clear_dialog() -> (i32, Vec<u8>) {
    (ids::play::CLIENTBOUND_CLEAR_DIALOG, Vec::new())
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

/// `ClientboundShowDialogPacket.STREAM_CODEC` (`ClientboundShowDialogPacket.java:12-14`)
/// carries `Dialog.STREAM_CODEC` (`Dialog.java:24-26`), which is
/// `ByteBufCodecs.holder` (`ByteBufCodecs.java:588-614`): VarInt 0 then the
/// direct dialog NBT, or VarInt `registry_id + 1` for a reference.
pub fn encode_play_clientbound_show_dialog(update: &ShowDialog) -> Result<(i32, Vec<u8>)> {
    let mut encoder = Encoder::new();
    match &update.dialog {
        DialogHolder::Reference { registry_id } => {
            if *registry_id < 0 {
                return Err(ProtocolError::InvalidData(format!(
                    "invalid dialog holder registry id {registry_id}"
                )));
            }
            let holder_id = registry_id.checked_add(1).ok_or_else(|| {
                ProtocolError::InvalidData("dialog holder id overflow".to_string())
            })?;
            encoder.write_var_i32(holder_id);
        }
        DialogHolder::Direct { dialog } => {
            encoder.write_var_i32(0);
            encoder.write_bytes(&encode_nbt_summary_bounded(
                Some(dialog),
                MAX_DIALOG_PAYLOAD,
                MAX_NBT_DEPTH,
            )?);
        }
        DialogHolder::ContextFree { .. } => {
            return Err(ProtocolError::InvalidData(
                "play show-dialog encode refuses a context-free holder".to_string(),
            ));
        }
    }
    Ok((ids::play::CLIENTBOUND_SHOW_DIALOG, encoder.into_inner()))
}

pub(crate) fn decode_context_free_show_dialog(decoder: &mut Decoder<'_>) -> Result<ShowDialog> {
    Ok(ShowDialog {
        dialog: DialogHolder::ContextFree {
            dialog: decode_direct_dialog(decoder, "context-free dialog payload")?,
        },
    })
}

pub(super) fn encode_context_free_show_dialog_payload(update: &ShowDialog) -> Result<Vec<u8>> {
    let dialog = match &update.dialog {
        DialogHolder::ContextFree { dialog } | DialogHolder::Direct { dialog } => dialog,
        DialogHolder::Reference { .. } => {
            return Err(ProtocolError::InvalidData(
                "configuration show-dialog encode refuses a registry reference holder".to_string(),
            ));
        }
    };
    if !matches!(dialog, NbtSummaryValue::Compound(_)) {
        return Err(ProtocolError::InvalidData(
            "context-free dialog payload root must be a compound tag".to_string(),
        ));
    }
    encode_nbt_summary_bounded(Some(dialog), MAX_DIALOG_PAYLOAD, MAX_NBT_DEPTH)
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

/// `ServerboundCustomClickActionPacket`: the action id, then the optional NBT
/// payload under the same tag budget the client encoder writes.
pub(crate) fn decode_custom_click_action_payload(
    decoder: &mut Decoder<'_>,
) -> Result<CustomClickAction> {
    let id = read_resource_location(decoder)?;
    if id.len() > MAX_CUSTOM_CLICK_ACTION_ID_CHARS {
        return Err(ProtocolError::StringTooLong {
            actual: id.len(),
            max: MAX_CUSTOM_CLICK_ACTION_ID_CHARS,
        });
    }
    let len = decoder.read_len()?;
    if len > MAX_CUSTOM_CLICK_ACTION_TAG_BYTES {
        return Err(ProtocolError::PacketTooLarge(
            len,
            MAX_CUSTOM_CLICK_ACTION_TAG_BYTES,
        ));
    }
    let tag = decoder.read_exact(len, "custom click action tag")?;
    let mut tag_decoder = Decoder::new(tag);
    // The encoder writes a bare `TAG_End` for an absent payload, so the blob is
    // never empty and the decoder reports absence through the tag id.
    let limits = NbtSummaryDecodeLimits {
        max_depth: MAX_CUSTOM_CLICK_ACTION_TAG_DEPTH,
        ..NbtSummaryDecodeLimits::NETWORK_QUOTA
    };
    let payload = decode_nbt_summary_from_decoder_with_limits(
        &mut tag_decoder,
        &limits,
        CUSTOM_CLICK_ACTION_NBT_QUOTA,
    )?;
    if !tag_decoder.is_empty() {
        return Err(ProtocolError::InvalidData(
            "trailing bytes after custom click action tag".to_string(),
        ));
    }
    Ok(CustomClickAction { id, payload })
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
