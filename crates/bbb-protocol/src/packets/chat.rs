use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub use crate::filter_mask_types::FilterMaskKind;
use crate::{
    codec::{Decoder, ProtocolError, Result},
    component::{
        decode_rich_text_style_from_decoder, decode_styled_component_summary_with_nbt_from_decoder,
    },
    styled_runs_summary_text, NbtSummaryValue, RichTextStyle, StyledTextRun,
};

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

pub(crate) fn decode_delete_chat(decoder: &mut Decoder<'_>) -> Result<DeleteChat> {
    let packet = DeleteChat {
        message_signature: decode_packed_message_signature(decoder)?,
    };
    reject_trailing(decoder, "delete chat packet")?;
    Ok(packet)
}

pub(crate) fn decode_disguised_chat(decoder: &mut Decoder<'_>) -> Result<DisguisedChat> {
    let (message, message_styled, message_component) = decode_component_projection(decoder)?;
    let packet = DisguisedChat {
        message,
        message_styled,
        message_component,
        chat_type: decode_chat_type_bound(decoder)?,
    };
    reject_trailing(decoder, "disguised chat packet")?;
    Ok(packet)
}

pub(crate) fn decode_player_chat(decoder: &mut Decoder<'_>) -> Result<PlayerChat> {
    let global_index = decoder.read_var_i32()?;
    let sender = decoder.read_uuid()?;
    let index = decoder.read_var_i32()?;
    let signature = decode_nullable_message_signature(decoder)?;
    let body = decode_signed_message_body(decoder)?;
    let (unsigned_content, unsigned_content_styled, unsigned_content_component) =
        decode_nullable_component_projection(decoder)?;
    let packet = PlayerChat {
        global_index,
        sender,
        index,
        signature,
        body,
        unsigned_content,
        unsigned_content_styled,
        unsigned_content_component,
        filter_mask: decode_filter_mask(decoder)?,
        chat_type: decode_chat_type_bound(decoder)?,
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
) -> Result<(String, Vec<StyledTextRun>, Option<NbtSummaryValue>)> {
    let (styled, component) = decode_styled_component_summary_with_nbt_from_decoder(decoder)?;
    let summary = styled_runs_summary_text(&styled);
    Ok((summary, styled, component))
}

fn decode_nullable_component_projection(
    decoder: &mut Decoder<'_>,
) -> Result<(
    Option<String>,
    Option<Vec<StyledTextRun>>,
    Option<NbtSummaryValue>,
)> {
    if decoder.read_bool()? {
        let (summary, styled, component) = decode_component_projection(decoder)?;
        Ok((Some(summary), Some(styled), component))
    } else {
        Ok((None, None, None))
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
    let id = decoder.read_var_i32()? - 1;
    if id == -1 {
        Ok(PackedMessageSignature {
            cache_id: None,
            full_signature: Some(decode_message_signature(decoder)?),
        })
    } else if id < -1 {
        Err(ProtocolError::InvalidData(format!(
            "invalid packed message signature id {id}"
        )))
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

    let mut last_seen = Vec::with_capacity(last_seen_count);
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
        let mut words = Vec::with_capacity(len);
        for _ in 0..len {
            words.push(decoder.read_i64()?);
        }
        words
    } else {
        Vec::new()
    };

    Ok(FilterMask { kind, mask_words })
}

fn decode_chat_type_bound(decoder: &mut Decoder<'_>) -> Result<ChatTypeBound> {
    let chat_type = decode_chat_type_holder(decoder)?;
    let (name, name_styled, name_component) = decode_component_projection(decoder)?;
    let (target_name, target_name_styled, target_name_component) =
        decode_nullable_component_projection(decoder)?;
    Ok(ChatTypeBound {
        chat_type,
        name,
        name_styled,
        name_component,
        target_name,
        target_name_styled,
        target_name_component,
    })
}

fn decode_chat_type_holder(decoder: &mut Decoder<'_>) -> Result<ChatTypeHolder> {
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
        chat: decode_chat_type_decoration(decoder)?,
        narration: decode_chat_type_decoration(decoder)?,
    })
}

fn decode_chat_type_decoration(decoder: &mut Decoder<'_>) -> Result<ChatTypeDecorationSummary> {
    let translation_key = decoder.read_string(32767)?;
    let parameter_count = decoder.read_len()?;
    if parameter_count > MAX_CHAT_TYPE_DECORATION_PARAMETERS {
        return Err(ProtocolError::PacketTooLarge(
            parameter_count,
            MAX_CHAT_TYPE_DECORATION_PARAMETERS,
        ));
    }
    let mut parameters = Vec::with_capacity(parameter_count);
    for _ in 0..parameter_count {
        parameters.push(decoder.read_var_i32()?);
    }

    // Style.Serializer.TRUSTED_STREAM_CODEC is an NBT-backed codec distinct
    // from ComponentSerialization.CODEC.
    let style = decode_rich_text_style_from_decoder(decoder)?;

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
