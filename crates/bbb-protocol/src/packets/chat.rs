use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    codec::{Decoder, Encoder, ProtocolError, Result},
    component::{
        decode_rich_text_style_from_decoder_with_registry,
        decode_styled_component_summary_with_nbt_from_decoder_with_registry,
    },
    content_registry::ContentRegistry,
    ids, styled_runs_summary_text, NbtSummaryValue, RichTextStyle, StyledTextRun,
};
pub use bbb_facts::filter_mask_types::FilterMaskKind;

const MESSAGE_SIGNATURE_BYTES: usize = 256;
const MAX_LAST_SEEN_MESSAGES: usize = 20;
const MAX_FILTER_MASK_LONGS: usize = 4096;
const MAX_CHAT_TYPE_DECORATION_PARAMETERS: usize = 16;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeleteChat {
    pub message_signature: PackedMessageSignature,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DisguisedChat {
    pub message: String,
    #[serde(default)]
    pub message_styled: Vec<StyledTextRun>,
    #[serde(default)]
    pub message_component: Option<NbtSummaryValue>,
    pub chat_type: ChatTypeBound,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlayerChat {
    pub global_index: i32,
    pub sender: Uuid,
    pub index: i32,
    pub signature: Option<MessageSignature>,
    pub body: SignedMessageBody,
    pub unsigned_content: Option<String>,
    #[serde(default)]
    pub unsigned_content_styled: Option<Vec<StyledTextRun>>,
    #[serde(default)]
    pub unsigned_content_component: Option<NbtSummaryValue>,
    pub filter_mask: FilterMask,
    pub chat_type: ChatTypeBound,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MessageSignature {
    pub bytes: Vec<u8>,
}

impl MessageSignature {
    pub fn checksum(&self) -> i32 {
        java_arrays_hash_code(&self.bytes)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PackedMessageSignature {
    pub cache_id: Option<i32>,
    pub full_signature: Option<MessageSignature>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SignedMessageBody {
    pub content: String,
    pub timestamp_millis: i64,
    pub salt: i64,
    pub last_seen: Vec<PackedMessageSignature>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FilterMask {
    pub kind: FilterMaskKind,
    pub mask_words: Vec<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChatTypeBound {
    pub chat_type: ChatTypeHolder,
    pub name: String,
    #[serde(default)]
    pub name_styled: Vec<StyledTextRun>,
    #[serde(default)]
    pub name_component: Option<NbtSummaryValue>,
    pub target_name: Option<String>,
    #[serde(default)]
    pub target_name_styled: Option<Vec<StyledTextRun>>,
    #[serde(default)]
    pub target_name_component: Option<NbtSummaryValue>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ChatTypeHolder {
    Registry {
        id: i32,
    },
    Direct {
        chat: ChatTypeDecorationSummary,
        narration: ChatTypeDecorationSummary,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChatTypeDecorationSummary {
    pub translation_key: String,
    pub parameters: Vec<i32>,
    #[serde(default)]
    pub style: RichTextStyle,
}

pub fn encode_play_clientbound_delete_chat(update: &DeleteChat) -> Result<(i32, Vec<u8>)> {
    let mut encoder = Encoder::new();
    match (
        update.message_signature.cache_id,
        &update.message_signature.full_signature,
    ) {
        (Some(cache_id), None) if cache_id >= 0 => {
            let packed = cache_id.checked_add(1).ok_or_else(|| {
                ProtocolError::InvalidData("packed message signature cache id overflow".to_string())
            })?;
            encoder.write_var_i32(packed);
        }
        (None, Some(signature)) if signature.bytes.len() == MESSAGE_SIGNATURE_BYTES => {
            encoder.write_var_i32(0);
            encoder.write_bytes(&signature.bytes);
        }
        _ => {
            return Err(ProtocolError::InvalidData(
                "delete chat requires either a non-negative cache id or a 256-byte message signature"
                    .to_string(),
            ));
        }
    }
    Ok((ids::play::CLIENTBOUND_DELETE_CHAT, encoder.into_inner()))
}

pub(crate) fn decode_delete_chat(decoder: &mut Decoder<'_>) -> Result<DeleteChat> {
    let packet = DeleteChat {
        message_signature: decode_packed_message_signature(decoder)?,
    };
    reject_trailing(decoder, "delete chat packet")?;
    Ok(packet)
}

pub(crate) fn decode_disguised_chat_with_registry(
    decoder: &mut Decoder<'_>,
    registry: &ContentRegistry,
) -> Result<DisguisedChat> {
    let (message, message_styled, message_component) =
        decode_component_projection(decoder, registry)?;
    let packet = DisguisedChat {
        message,
        message_styled,
        message_component,
        chat_type: decode_chat_type_bound(decoder, registry)?,
    };
    reject_trailing(decoder, "disguised chat packet")?;
    Ok(packet)
}

pub(crate) fn decode_player_chat_with_registry(
    decoder: &mut Decoder<'_>,
    registry: &ContentRegistry,
) -> Result<PlayerChat> {
    let global_index = decoder.read_var_i32()?;
    let sender = decoder.read_uuid()?;
    let index = decoder.read_var_i32()?;
    let signature = decode_nullable_message_signature(decoder)?;
    let body = decode_signed_message_body(decoder)?;
    let unsigned = decode_nullable_component_projection(decoder, registry)?;
    let packet = PlayerChat {
        global_index,
        sender,
        index,
        signature,
        body,
        unsigned_content: unsigned.summary,
        unsigned_content_styled: unsigned.styled,
        unsigned_content_component: unsigned.component,
        filter_mask: decode_filter_mask(decoder)?,
        chat_type: decode_chat_type_bound(decoder, registry)?,
    };
    reject_trailing(decoder, "player chat packet")?;
    Ok(packet)
}

fn decode_nullable_message_signature(
    decoder: &mut Decoder<'_>,
) -> Result<Option<MessageSignature>> {
    if decoder.read_bool()? {
        Ok(Some(decode_message_signature(decoder)?))
    } else {
        Ok(None)
    }
}

fn decode_component_projection(
    decoder: &mut Decoder<'_>,
    registry: &ContentRegistry,
) -> Result<(String, Vec<StyledTextRun>, Option<NbtSummaryValue>)> {
    let (styled, component) =
        decode_styled_component_summary_with_nbt_from_decoder_with_registry(decoder, registry)?;
    let summary = styled_runs_summary_text(&styled);
    Ok((summary, styled, component))
}

struct NullableComponentProjection {
    summary: Option<String>,
    styled: Option<Vec<StyledTextRun>>,
    component: Option<NbtSummaryValue>,
}

fn decode_nullable_component_projection(
    decoder: &mut Decoder<'_>,
    registry: &ContentRegistry,
) -> Result<NullableComponentProjection> {
    if decoder.read_bool()? {
        let (summary, styled, component) = decode_component_projection(decoder, registry)?;
        Ok(NullableComponentProjection {
            summary: Some(summary),
            styled: Some(styled),
            component,
        })
    } else {
        Ok(NullableComponentProjection {
            summary: None,
            styled: None,
            component: None,
        })
    }
}

fn decode_message_signature(decoder: &mut Decoder<'_>) -> Result<MessageSignature> {
    Ok(MessageSignature {
        bytes: decoder
            .read_exact(MESSAGE_SIGNATURE_BYTES, "message signature")?
            .to_vec(),
    })
}

fn decode_packed_message_signature(decoder: &mut Decoder<'_>) -> Result<PackedMessageSignature> {
    let raw = decoder.read_var_i32()?;
    let id = raw.checked_sub(1).ok_or_else(|| {
        ProtocolError::InvalidData(format!("invalid packed message signature id {raw}"))
    })?;
    if id < -1 {
        return Err(ProtocolError::InvalidData(format!(
            "invalid packed message signature id {id}"
        )));
    }
    if id == -1 {
        Ok(PackedMessageSignature {
            cache_id: None,
            full_signature: Some(decode_message_signature(decoder)?),
        })
    } else {
        Ok(PackedMessageSignature {
            cache_id: Some(id),
            full_signature: None,
        })
    }
}

fn decode_signed_message_body(decoder: &mut Decoder<'_>) -> Result<SignedMessageBody> {
    let content = decoder.read_string(256)?;
    let timestamp_millis = decoder.read_i64()?;
    let salt = decoder.read_i64()?;
    let last_seen_count = decoder.read_len()?;
    if last_seen_count > MAX_LAST_SEEN_MESSAGES {
        return Err(ProtocolError::PacketTooLarge(
            last_seen_count,
            MAX_LAST_SEEN_MESSAGES,
        ));
    }

    let mut last_seen = Vec::with_capacity(decoder.bounded_capacity(last_seen_count, 1));
    for _ in 0..last_seen_count {
        last_seen.push(decode_packed_message_signature(decoder)?);
    }

    Ok(SignedMessageBody {
        content,
        timestamp_millis,
        salt,
        last_seen,
    })
}

fn decode_filter_mask(decoder: &mut Decoder<'_>) -> Result<FilterMask> {
    let ordinal = decoder.read_var_i32()?;
    let kind = FilterMaskKind::from_ordinal(ordinal).ok_or_else(|| {
        ProtocolError::InvalidData(format!("invalid filter mask type ordinal {ordinal}"))
    })?;

    let mask_words = if kind == FilterMaskKind::PartiallyFiltered {
        let len = decoder.read_len()?;
        if len > MAX_FILTER_MASK_LONGS {
            return Err(ProtocolError::PacketTooLarge(len, MAX_FILTER_MASK_LONGS));
        }
        let mut words = Vec::with_capacity(decoder.bounded_capacity(len, 8));
        for _ in 0..len {
            words.push(decoder.read_i64()?);
        }
        words
    } else {
        Vec::new()
    };

    Ok(FilterMask { kind, mask_words })
}

fn decode_chat_type_bound(
    decoder: &mut Decoder<'_>,
    registry: &ContentRegistry,
) -> Result<ChatTypeBound> {
    let chat_type = decode_chat_type_holder(decoder, registry)?;
    let (name, name_styled, name_component) = decode_component_projection(decoder, registry)?;
    let target = decode_nullable_component_projection(decoder, registry)?;
    Ok(ChatTypeBound {
        chat_type,
        name,
        name_styled,
        name_component,
        target_name: target.summary,
        target_name_styled: target.styled,
        target_name_component: target.component,
    })
}

fn decode_chat_type_holder(
    decoder: &mut Decoder<'_>,
    registry: &ContentRegistry,
) -> Result<ChatTypeHolder> {
    let holder_id = decoder.read_var_i32()?;
    if holder_id > 0 {
        return Ok(ChatTypeHolder::Registry { id: holder_id - 1 });
    }
    if holder_id < 0 {
        return Err(ProtocolError::InvalidData(format!(
            "invalid chat type holder id {holder_id}"
        )));
    }

    Ok(ChatTypeHolder::Direct {
        chat: decode_chat_type_decoration(decoder, registry)?,
        narration: decode_chat_type_decoration(decoder, registry)?,
    })
}

fn decode_chat_type_decoration(
    decoder: &mut Decoder<'_>,
    registry: &ContentRegistry,
) -> Result<ChatTypeDecorationSummary> {
    let translation_key = decoder.read_string(32767)?;
    let parameter_count = decoder.read_len()?;
    if parameter_count > MAX_CHAT_TYPE_DECORATION_PARAMETERS {
        return Err(ProtocolError::PacketTooLarge(
            parameter_count,
            MAX_CHAT_TYPE_DECORATION_PARAMETERS,
        ));
    }
    let mut parameters = Vec::with_capacity(decoder.bounded_capacity(parameter_count, 1));
    for _ in 0..parameter_count {
        parameters.push(decoder.read_var_i32()?);
    }

    // Style.Serializer.TRUSTED_STREAM_CODEC is an NBT-backed codec distinct
    // from ComponentSerialization.CODEC.
    let style = decode_rich_text_style_from_decoder_with_registry(decoder, registry)?;

    Ok(ChatTypeDecorationSummary {
        translation_key,
        parameters,
        style,
    })
}

fn reject_trailing(decoder: &Decoder<'_>, packet: &'static str) -> Result<()> {
    if decoder.is_empty() {
        Ok(())
    } else {
        Err(ProtocolError::InvalidData(format!(
            "trailing bytes after {packet}"
        )))
    }
}

fn java_arrays_hash_code(bytes: &[u8]) -> i32 {
    bytes.iter().fold(1i32, |hash, byte| {
        hash.wrapping_mul(31).wrapping_add(i32::from(*byte as i8))
    })
}

#[cfg(test)]
mod tests;
