use std::fmt;

use bbb_protocol::{
    chat_formattings::vanilla_chat_formatting_color_by_name,
    component_style_from_validated_nbt_value,
    packets::{ChatTypeDecorationSummary, ChatTypeHolder},
    registry_data_types::VanillaChatTypeIdentity,
    ComponentStyle, NbtSummaryValue,
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
    pub translation_key: bbb_protocol::JavaString,
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
            let (chat, narration) =
                match entry.source {
                    RegistryEntrySource::Server => access
                        .typed_chat_type(entry.resource_id)
                        .cloned()
                        .ok_or_else(|| ChatTypeProjectionError::MissingPayload {
                            resource_id: entry.resource_id.to_string(),
                        })?,
                    RegistryEntrySource::VanillaFallback => {
                        // `minecraft:chat_type` is data-loaded, so the fallback
                        // holder id is a sorted-resource-id index. Key the built-in
                        // decorations off the resolved resource id rather than off
                        // the bootstrap declaration order.
                        vanilla_chat_type(entry.resource_id).ok_or_else(|| {
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
        translation_key: decoration.translation_key.clone().into(),
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

pub(super) fn decode_validated_chat_type(
    value: NbtSummaryValue,
) -> (ChatTypeDecoration, ChatTypeDecoration) {
    let NbtSummaryValue::Compound(entries) = value else {
        unreachable!("canonical chat type compound");
    };
    let mut values = entries.into_iter().map(|entry| entry.value);
    (
        decode_decoration(values.next().expect("canonical chat decoration")),
        decode_decoration(values.next().expect("canonical narration decoration")),
    )
}

fn decode_decoration(value: NbtSummaryValue) -> ChatTypeDecoration {
    let NbtSummaryValue::Compound(entries) = value else {
        unreachable!("canonical chat decoration compound");
    };
    let mut values = entries.into_iter().map(|entry| entry.value);
    let Some(NbtSummaryValue::String(translation_key)) = values.next() else {
        unreachable!("canonical chat translation key string");
    };
    let Some(NbtSummaryValue::List(raw_parameters)) = values.next() else {
        unreachable!("canonical chat parameters list");
    };
    let mut parameters = Vec::with_capacity(raw_parameters.len());
    for parameter in raw_parameters {
        let NbtSummaryValue::String(parameter) = parameter else {
            unreachable!("canonical chat parameter string");
        };
        parameters.push(if parameter == "sender" {
            ChatTypeDecorationParameter::Sender
        } else if parameter == "target" {
            ChatTypeDecorationParameter::Target
        } else if parameter == "content" {
            ChatTypeDecorationParameter::Content
        } else {
            unreachable!("canonical chat parameter");
        });
    }
    let style = match values.next() {
        Some(style) => component_style_from_validated_nbt_value(&style)
            .expect("canonical chat decoration style"),
        None => ComponentStyle::default(),
    };
    ChatTypeDecoration {
        translation_key,
        parameters,
        style,
    }
}

fn vanilla_chat_type(resource_id: &str) -> Option<(ChatTypeDecoration, ChatTypeDecoration)> {
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
    let resolved = match VanillaChatTypeIdentity::for_resource_id(resource_id)? {
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
        translation_key: translation_key.into(),
        parameters: parameters.to_vec(),
        style,
    }
}

impl fmt::Display for ChatTypeProjectionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Registry(error) => error.fmt(formatter),
            Self::MissingPayload { resource_id } => {
                write!(formatter, "missing {resource_id} chat type payload")
            }
        }
    }
}

impl std::error::Error for ChatTypeProjectionError {}
