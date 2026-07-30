use std::fmt;

use bbb_protocol::{
    chat_formattings::vanilla_chat_formatting_color_by_name,
    component_style_from_nbt_value,
    packets::{ChatTypeDecorationSummary, ChatTypeHolder},
    registry_data_types::VanillaChatTypeIdentity,
    ComponentStyle, NbtSummaryEntry, NbtSummaryValue,
};
use serde::{Deserialize, Serialize};

use super::{FrozenRegistryAccess, RegistryEntrySource, RegistryResolutionError};

pub const CHAT_TYPE_REGISTRY: &str = "minecraft:chat_type";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChatTypeDecorationParameter {
    Sender,
    Target,
    Content,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChatTypeDecoration {
    pub translation_key: String,
    pub parameters: Vec<ChatTypeDecorationParameter>,
    pub style: ComponentStyle,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ResolvedChatType {
    pub holder_id: Option<i32>,
    pub resource_id: Option<String>,
    pub source: Option<RegistryEntrySource>,
    pub chat: ChatTypeDecoration,
    pub narration: ChatTypeDecoration,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChatTypeProjectionError {
    Registry(RegistryResolutionError),
    MissingPayload { resource_id: String },
    MalformedPayload { resource_id: String },
}

pub(crate) fn resolve_chat_type(
    access: FrozenRegistryAccess<'_>,
    holder: &ChatTypeHolder,
) -> Result<ResolvedChatType, ChatTypeProjectionError> {
    match holder {
        ChatTypeHolder::Direct { chat, narration } => Ok(ResolvedChatType {
            holder_id: None,
            resource_id: None,
            source: None,
            chat: direct_decoration(chat),
            narration: direct_decoration(narration),
        }),
        ChatTypeHolder::Registry { id } => {
            let entry = access
                .require_entry_by_holder_id(CHAT_TYPE_REGISTRY, *id)
                .map_err(ChatTypeProjectionError::Registry)?;
            let (chat, narration) = match entry.source {
                RegistryEntrySource::Server => {
                    if let Some((chat, narration)) = access.typed_chat_type(entry.resource_id) {
                        return Ok(ResolvedChatType {
                            holder_id: Some(*id),
                            resource_id: Some(entry.resource_id.to_string()),
                            source: Some(entry.source),
                            chat: chat.clone(),
                            narration: narration.clone(),
                        });
                    }
                    let data = entry.element_data.ok_or_else(|| {
                        ChatTypeProjectionError::MissingPayload {
                            resource_id: entry.resource_id.to_string(),
                        }
                    })?;
                    decode_chat_type(&data.value).ok_or_else(|| {
                        ChatTypeProjectionError::MalformedPayload {
                            resource_id: entry.resource_id.to_string(),
                        }
                    })?
                }
                RegistryEntrySource::VanillaFallback => {
                    vanilla_chat_type(*id).ok_or_else(|| {
                        ChatTypeProjectionError::Registry(
                            RegistryResolutionError::MissingHolderId {
                                registry: CHAT_TYPE_REGISTRY.to_string(),
                                holder_id: *id,
                            },
                        )
                    })?
                }
            };
            Ok(ResolvedChatType {
                holder_id: Some(*id),
                resource_id: Some(entry.resource_id.to_string()),
                source: Some(entry.source),
                chat,
                narration,
            })
        }
    }
}

fn direct_decoration(decoration: &ChatTypeDecorationSummary) -> ChatTypeDecoration {
    ChatTypeDecoration {
        translation_key: decoration.translation_key.clone(),
        parameters: decoration
            .parameters
            .iter()
            .map(|parameter| network_parameter(*parameter))
            .collect(),
        style: decoration.style.style().clone(),
    }
}

fn network_parameter(parameter: i32) -> ChatTypeDecorationParameter {
    match parameter {
        1 => ChatTypeDecorationParameter::Target,
        2 => ChatTypeDecorationParameter::Content,
        // ChatTypeDecoration.Parameter uses ByIdMap.OutOfBoundsStrategy.ZERO.
        _ => ChatTypeDecorationParameter::Sender,
    }
}

pub(super) fn decode_chat_type(
    value: &NbtSummaryValue,
) -> Option<(ChatTypeDecoration, ChatTypeDecoration)> {
    let NbtSummaryValue::Compound(entries) = value else {
        return None;
    };
    Some((
        decode_decoration(compound_field(&entries, "chat")?)?,
        decode_decoration(compound_field(&entries, "narration")?)?,
    ))
}

fn decode_decoration(entries: &[NbtSummaryEntry]) -> Option<ChatTypeDecoration> {
    let translation_key = string_field(entries, "translation_key")?.to_string();
    let NbtSummaryValue::List(parameters) = field(entries, "parameters")? else {
        return None;
    };
    let parameters = parameters
        .iter()
        .map(|parameter| {
            let NbtSummaryValue::String(parameter) = parameter else {
                return None;
            };
            match parameter.as_str() {
                "sender" => Some(ChatTypeDecorationParameter::Sender),
                "target" => Some(ChatTypeDecorationParameter::Target),
                "content" => Some(ChatTypeDecorationParameter::Content),
                _ => None,
            }
        })
        .collect::<Option<Vec<_>>>()?;
    let style = match field(entries, "style") {
        Some(style) => component_style_from_nbt_value(style)?,
        None => ComponentStyle::default(),
    };
    Some(ChatTypeDecoration {
        translation_key,
        parameters,
        style,
    })
}

fn vanilla_chat_type(holder_id: i32) -> Option<(ChatTypeDecoration, ChatTypeDecoration)> {
    use ChatTypeDecorationParameter::{Content, Sender, Target};
    use VanillaChatTypeIdentity::{
        Chat, EmoteCommand, MsgCommandIncoming, MsgCommandOutgoing, SayCommand,
        TeamMsgCommandIncoming, TeamMsgCommandOutgoing,
    };

    let plain = ComponentStyle::default();
    let direct_message = ComponentStyle {
        italic: Some(true),
        color: vanilla_chat_formatting_color_by_name("gray"),
        ..ComponentStyle::default()
    };
    let resolved = match VanillaChatTypeIdentity::for_vanilla_registry_id(holder_id)? {
        Chat => (
            decoration("chat.type.text", &[Sender, Content], plain.clone()),
            decoration("chat.type.text.narrate", &[Sender, Content], plain),
        ),
        SayCommand => (
            decoration("chat.type.announcement", &[Sender, Content], plain.clone()),
            decoration("chat.type.text.narrate", &[Sender, Content], plain),
        ),
        MsgCommandIncoming => (
            decoration(
                "commands.message.display.incoming",
                &[Sender, Content],
                direct_message,
            ),
            decoration("chat.type.text.narrate", &[Sender, Content], plain),
        ),
        MsgCommandOutgoing => (
            decoration(
                "commands.message.display.outgoing",
                &[Target, Content],
                direct_message,
            ),
            decoration("chat.type.text.narrate", &[Sender, Content], plain),
        ),
        TeamMsgCommandIncoming => (
            decoration(
                "chat.type.team.text",
                &[Target, Sender, Content],
                plain.clone(),
            ),
            decoration("chat.type.text.narrate", &[Sender, Content], plain),
        ),
        TeamMsgCommandOutgoing => (
            decoration(
                "chat.type.team.sent",
                &[Target, Sender, Content],
                plain.clone(),
            ),
            decoration("chat.type.text.narrate", &[Sender, Content], plain),
        ),
        EmoteCommand => (
            decoration("chat.type.emote", &[Sender, Content], plain.clone()),
            decoration("chat.type.emote", &[Sender, Content], plain),
        ),
    };
    Some(resolved)
}

fn decoration(
    translation_key: &str,
    parameters: &[ChatTypeDecorationParameter],
    style: ComponentStyle,
) -> ChatTypeDecoration {
    ChatTypeDecoration {
        translation_key: translation_key.to_string(),
        parameters: parameters.to_vec(),
        style,
    }
}

fn field<'a>(entries: &'a [NbtSummaryEntry], name: &str) -> Option<&'a NbtSummaryValue> {
    entries
        .iter()
        .find(|entry| entry.name == name)
        .map(|entry| &entry.value)
}

fn compound_field<'a>(entries: &'a [NbtSummaryEntry], name: &str) -> Option<&'a [NbtSummaryEntry]> {
    let NbtSummaryValue::Compound(entries) = field(entries, name)? else {
        return None;
    };
    Some(entries)
}

fn string_field<'a>(entries: &'a [NbtSummaryEntry], name: &str) -> Option<&'a str> {
    let NbtSummaryValue::String(value) = field(entries, name)? else {
        return None;
    };
    Some(value)
}

impl fmt::Display for ChatTypeProjectionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Registry(error) => error.fmt(formatter),
            Self::MissingPayload { resource_id } => {
                write!(formatter, "missing {resource_id} chat type payload")
            }
            Self::MalformedPayload { resource_id } => {
                write!(formatter, "malformed {resource_id} chat type payload")
            }
        }
    }
}

impl std::error::Error for ChatTypeProjectionError {}
