//! Chat messages bound to a chat type: unsigned `ClientboundPlayerChatPacket`s
//! for player-sent messages and `ClientboundDisguisedChatPacket`s for the rest.

use bbb_protocol::chat_formattings::ChatFormatting;
use bbb_protocol::packets::{BoundChatType, DisguisedChatMessage, UnsignedPlayerChat};
use bbb_protocol::{
    component_literal_nbt, component_translatable_nbt, ComponentStyle, NbtSummaryValue,
    RichTextComponent,
};
use uuid::Uuid;

use super::{send, ClientboundPlayPacket, ConnectionId, ServerWorld, WorldEvent};

const CHAT_TYPE_REGISTRY: &str = "minecraft:chat_type";

/// The built-in chat types (ChatType.java:31-37, bootstrap :42-64).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChatKind {
    Chat,
    SayCommand,
    EmoteCommand,
    MsgCommandIncoming,
    MsgCommandOutgoing,
    TeamMsgCommandIncoming,
    TeamMsgCommandOutgoing,
}

impl ChatKind {
    #[cfg(test)]
    pub(crate) const ALL: [Self; 7] = [
        Self::Chat,
        Self::SayCommand,
        Self::EmoteCommand,
        Self::MsgCommandIncoming,
        Self::MsgCommandOutgoing,
        Self::TeamMsgCommandIncoming,
        Self::TeamMsgCommandOutgoing,
    ];

    pub(crate) fn resource_id(self) -> &'static str {
        match self {
            Self::Chat => "minecraft:chat",
            Self::SayCommand => "minecraft:say_command",
            Self::EmoteCommand => "minecraft:emote_command",
            Self::MsgCommandIncoming => "minecraft:msg_command_incoming",
            Self::MsgCommandOutgoing => "minecraft:msg_command_outgoing",
            Self::TeamMsgCommandIncoming => "minecraft:team_msg_command_incoming",
            Self::TeamMsgCommandOutgoing => "minecraft:team_msg_command_outgoing",
        }
    }
}

/// A `PlayerChatMessage` with the `ChatType.Bound` it is sent under.
#[derive(Debug, Clone, PartialEq)]
pub struct OutgoingChat {
    /// `SignedMessageLink.sender`. `None` is `PlayerChatMessage.system`, which
    /// `OutgoingChatMessage.create` (OutgoingChatMessage.java:10-11) disguises.
    pub sender: Option<Uuid>,
    /// `SignedMessageBody.content`.
    pub text: String,
    /// `unsignedContent`, set only when it differs from the literal text
    /// (`withUnsignedContent`, PlayerChatMessage.java:52-55).
    pub formatted: Option<RichTextComponent>,
    pub timestamp_millis: i64,
    pub kind: ChatKind,
    pub name: RichTextComponent,
    pub target_name: Option<RichTextComponent>,
    content_registry: std::sync::Arc<bbb_protocol::content_registry::ContentRegistry>,
}

impl OutgoingChat {
    /// `formatted` is the resolved message component; it is kept only when it
    /// is not the bare literal of `text`.
    pub fn new(
        sender: Option<Uuid>,
        text: impl Into<String>,
        formatted: NbtSummaryValue,
        kind: ChatKind,
        name: NbtSummaryValue,
    ) -> Self {
        Self::new_with_registry(
            sender,
            text,
            formatted,
            kind,
            name,
            std::sync::Arc::clone(bbb_protocol::content_registry::ContentRegistry::vanilla()),
        )
    }

    pub fn new_with_registry(
        sender: Option<Uuid>,
        text: impl Into<String>,
        formatted: NbtSummaryValue,
        kind: ChatKind,
        name: NbtSummaryValue,
        content_registry: std::sync::Arc<bbb_protocol::content_registry::ContentRegistry>,
    ) -> Self {
        let text = text.into();
        let formatted = (formatted != component_literal_nbt(text.as_str())).then(|| {
            RichTextComponent::try_from_component_nbt_with_registry(formatted, &content_registry)
                .expect("a resolved message is a component")
        });
        Self {
            sender,
            text,
            formatted,
            // `SignedMessageBody.unsigned` stamps `Instant.now()`.
            timestamp_millis: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_or(0, |elapsed| elapsed.as_millis() as i64),
            kind,
            name: RichTextComponent::try_from_component_nbt_with_registry(name, &content_registry)
                .expect("a display name is a component"),
            target_name: None,
            content_registry,
        }
    }

    /// `ChatType.Bound.withTargetName` (ChatType.java:103-105).
    #[must_use]
    pub fn with_target(self, kind: ChatKind, target_name: NbtSummaryValue) -> Self {
        let target_name = RichTextComponent::try_from_component_nbt_with_registry(
            target_name,
            &self.content_registry,
        )
        .expect("a display name is a component");
        Self {
            kind,
            target_name: Some(target_name),
            ..self
        }
    }

    /// `PlayerChatMessage.decoratedContent` (PlayerChatMessage.java:93-95).
    pub(crate) fn content(&self) -> RichTextComponent {
        self.formatted
            .clone()
            .unwrap_or_else(|| RichTextComponent::plain(self.text.clone()))
    }

    /// `ChatType.Bound.decorate`: what a source without a player session sees.
    pub fn decorated(&self) -> RichTextComponent {
        decorate_with_registry(
            self.kind,
            &self.content(),
            &self.name,
            self.target_name.as_ref(),
            &self.content_registry,
        )
    }
}

/// `ChatTypeDecoration.decorate` with the bootstrap decorations
/// (ChatTypeDecoration.java:34-57).
#[cfg(test)]
pub(crate) fn decorate(
    kind: ChatKind,
    content: &RichTextComponent,
    name: &RichTextComponent,
    target_name: Option<&RichTextComponent>,
) -> RichTextComponent {
    decorate_with_registry(
        kind,
        content,
        name,
        target_name,
        bbb_protocol::content_registry::ContentRegistry::vanilla(),
    )
}

pub(crate) fn decorate_with_registry(
    kind: ChatKind,
    content: &RichTextComponent,
    name: &RichTextComponent,
    target_name: Option<&RichTextComponent>,
    content_registry: &bbb_protocol::content_registry::ContentRegistry,
) -> RichTextComponent {
    let content = content.component().clone();
    let sender = name.component().clone();
    let target = target_name.map_or_else(
        || component_literal_nbt(""),
        |target| target.component().clone(),
    );
    let direct = || ComponentStyle {
        italic: Some(true),
        ..ComponentStyle::with_chat_formatting_color(ChatFormatting::Gray).expect("gray is a color")
    };
    let (key, arguments, style) = match kind {
        ChatKind::Chat => ("chat.type.text", vec![sender, content], None),
        ChatKind::SayCommand => ("chat.type.announcement", vec![sender, content], None),
        ChatKind::EmoteCommand => ("chat.type.emote", vec![sender, content], None),
        ChatKind::MsgCommandIncoming => (
            "commands.message.display.incoming",
            vec![sender, content],
            Some(direct()),
        ),
        ChatKind::MsgCommandOutgoing => (
            "commands.message.display.outgoing",
            vec![target, content],
            Some(direct()),
        ),
        ChatKind::TeamMsgCommandIncoming => {
            ("chat.type.team.text", vec![target, sender, content], None)
        }
        ChatKind::TeamMsgCommandOutgoing => {
            ("chat.type.team.sent", vec![target, sender, content], None)
        }
    };
    RichTextComponent::try_from_component_nbt_with_registry(
        component_translatable_nbt(key, arguments, &style.unwrap_or_default())
            .expect("chat decorations are representable"),
        content_registry,
    )
    .expect("chat decorations are components")
}

impl ServerWorld {
    /// `PlayerList.broadcastChatMessage` (PlayerList.java:734-751).
    pub(super) fn broadcast_chat(&mut self, chat: &OutgoingChat, events: &mut Vec<WorldEvent>) {
        let recipients = self
            .levels
            .player_entries()
            .map(|(&connection_id, _)| connection_id)
            .collect::<Vec<_>>();
        for connection_id in recipients {
            self.send_chat(connection_id, chat, events);
        }
    }

    /// `ServerPlayer.sendChatMessage` (ServerPlayer.java:1822-1826) through
    /// `OutgoingChatMessage.sendToPlayer` and
    /// `ServerGamePacketListenerImpl.sendPlayerChatMessage` (:1735-1761): an
    /// unsigned message touches neither the signature cache nor last-seen.
    pub(super) fn send_chat(
        &mut self,
        connection_id: ConnectionId,
        chat: &OutgoingChat,
        events: &mut Vec<WorldEvent>,
    ) {
        let chat_type_id = self
            .registries
            .registry_element_index(CHAT_TYPE_REGISTRY, chat.kind.resource_id())
            .expect("the built-in chat types are always published");
        let Some(player) = self.levels.player_mut(connection_id) else {
            return;
        };
        if !super::player_packets::accepts_player_message(
            player.cold.client_information.chat_visibility,
            false,
            true,
        ) {
            return;
        }
        let chat_type = BoundChatType {
            chat_type_id,
            name: chat.name.clone(),
            target_name: chat.target_name.clone(),
        };
        let packet = match chat.sender {
            Some(sender) => {
                let global_index = player.cold.next_chat_index;
                player.cold.next_chat_index = player.cold.next_chat_index.wrapping_add(1);
                ClientboundPlayPacket::PlayerChat(UnsignedPlayerChat {
                    global_index,
                    sender,
                    content: chat.text.clone(),
                    timestamp_millis: chat.timestamp_millis,
                    unsigned_content: chat.formatted.clone(),
                    chat_type,
                })
            }
            None => ClientboundPlayPacket::DisguisedChat(DisguisedChatMessage {
                message: chat.content(),
                chat_type,
            }),
        };
        events.push(send(connection_id, packet));
    }
}
