use bbb_protocol::{
    packets::{
        ChatAcknowledgement as ProtocolChatAcknowledgement,
        ChatTypeHolder as ProtocolChatTypeHolder, DeleteChat as ProtocolDeleteChat,
        DisguisedChat as ProtocolDisguisedChat, FilterMask as ProtocolFilterMask,
        FilterMaskKind as ProtocolFilterMaskKind, LastSeenMessagesUpdate,
        MessageSignature as ProtocolMessageSignature, PackedMessageSignature, PlayerChat,
    },
    ComponentStyle, NbtSummaryEntry, NbtSummaryValue, RichTextComponent, StyledTextRun,
};
use rsa::{pkcs8::DecodePublicKey, Pkcs1v15Sign, RsaPublicKey};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::VecDeque;
use uuid::Uuid;

mod message_resolution;
use message_resolution::*;

mod presentation;

pub use presentation::{
    ChatMessageSourceState, ChatMessageTagState, ChatNarrationKind, ChatNarrationRequest,
    ChatPresentationConfig, ChatTrustLevelState, ChatVisibilityState, DelayedChatMessageState,
    RecentChatIdentity, SocialChatHistory,
};

use crate::{
    registries::{
        resolve_chat_type, ChatTypeDecoration, ChatTypeDecorationParameter, ResolvedChatType,
    },
    RegistryEntrySource, WorldStore,
};

const SIGNATURE_CACHE_CAPACITY: usize = 128;
const LAST_SEEN_MESSAGES_TRACKER_CAPACITY: usize = 20;
const CHAT_ACKNOWLEDGEMENT_OFFSET_THRESHOLD: i32 = 64;
const PROFILE_PUBLIC_KEY_EXPIRY_GRACE_MILLIS: i64 = 8 * 60 * 60 * 1_000;
const CHAT_DELETION_DELAY_TICKS: u64 = 60;

fn translatable_component_nbt(translation_key: &str, arguments: &[&str]) -> NbtSummaryValue {
    let mut entries = vec![NbtSummaryEntry {
        name: "translate".into(),
        value: NbtSummaryValue::string(translation_key),
    }];
    if !arguments.is_empty() {
        entries.push(NbtSummaryEntry {
            name: "with".into(),
            value: NbtSummaryValue::List(
                arguments
                    .iter()
                    .map(|argument| NbtSummaryValue::string(*argument))
                    .collect(),
            ),
        });
    }
    NbtSummaryValue::Compound(entries)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientChatState {
    pub messages: Vec<ChatMessageState>,
    #[serde(default)]
    pub delayed_messages: VecDeque<DelayedChatMessageState>,
    pub deleted_messages: Vec<DeletedChatState>,
    #[serde(default)]
    pub gui_ticks: u64,
    pub expected_player_chat_global_index: i32,
    pub signature_cache: Vec<Option<ChatSignatureState>>,
    #[serde(default)]
    pub acknowledgement: ChatAcknowledgementState,
    #[serde(default)]
    pub last_seen_messages: LastSeenMessagesTrackerState,
    #[serde(default)]
    pub presentation: ChatPresentationConfig,
    #[serde(default)]
    pub social_history: SocialChatHistory,
    #[serde(default)]
    pub pending_narrations: Vec<ChatNarrationRequest>,
    #[serde(default)]
    pub last_delivery_monotonic_millis: u64,
    #[serde(default)]
    pub delivery_revision: u64,
    /// Bumped where Vanilla re-displays every message after replacing one in
    /// place (ChatComponent.java:329-332, 351-359).
    #[serde(default)]
    pub display_refresh_revision: u64,
}

impl Default for ClientChatState {
    fn default() -> Self {
        Self {
            messages: Vec::new(),
            delayed_messages: VecDeque::new(),
            deleted_messages: Vec::new(),
            gui_ticks: 0,
            expected_player_chat_global_index: 0,
            signature_cache: vec![None; SIGNATURE_CACHE_CAPACITY],
            acknowledgement: ChatAcknowledgementState::default(),
            last_seen_messages: LastSeenMessagesTrackerState::default(),
            presentation: ChatPresentationConfig::default(),
            social_history: SocialChatHistory::default(),
            pending_narrations: Vec::new(),
            last_delivery_monotonic_millis: 0,
            delivery_revision: 0,
            display_refresh_revision: 0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChatMessageState {
    pub kind: ChatMessageKind,
    #[serde(default)]
    pub source: ChatMessageSourceState,
    pub content: String,
    #[serde(default)]
    pub content_translation_key: Option<String>,
    #[serde(default)]
    pub styled_content: Vec<StyledTextRun>,
    #[serde(default)]
    pub content_component: Option<NbtSummaryValue>,
    #[serde(default)]
    pub decoration: Option<ChatDecorationState>,
    #[serde(default)]
    pub narration_decoration: Option<ChatDecorationState>,
    pub sender: Option<Uuid>,
    pub sender_name: String,
    pub target_name: Option<String>,
    pub global_index: Option<i32>,
    pub message_index: Option<i32>,
    pub chat_type: ChatTypeState,
    pub signature: Option<ChatSignatureState>,
    pub unsigned_content: Option<String>,
    pub filter_mask: String,
    pub validation_state: ChatValidationState,
    #[serde(default)]
    pub trust_level: ChatTrustLevelState,
    #[serde(default)]
    pub tag: Option<ChatMessageTagState>,
    #[serde(default)]
    pub received_epoch_millis: i64,
    #[serde(default)]
    pub display_sequence: u64,
    #[serde(default)]
    pub filter_mask_words: Vec<i64>,
    #[serde(default)]
    pub unsigned_content_has_custom_font: bool,
    #[serde(default)]
    pub added_tick: u64,
    #[serde(skip)]
    pub(crate) full_signature: Option<Vec<u8>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ChatMessageKind {
    Player,
    Disguised,
    System,
    ClientSystem,
}

impl ChatMessageKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Player => "player",
            Self::Disguised => "disguised",
            Self::System => "system",
            Self::ClientSystem => "client_system",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChatTypeState {
    pub registry_id: Option<i32>,
    #[serde(default)]
    pub resource_id: Option<String>,
    #[serde(default)]
    pub source: Option<RegistryEntrySource>,
    pub direct_translation_key: Option<String>,
    #[serde(default)]
    pub resolution_error: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChatDecorationState {
    pub translation_key: bbb_protocol::JavaString,
    pub arguments: Vec<ChatDecorationArgumentState>,
    pub style: ComponentStyle,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChatDecorationArgumentState {
    pub parameter: ChatTypeDecorationParameter,
    pub summary: String,
    #[serde(default)]
    pub styled: Vec<StyledTextRun>,
    #[serde(default)]
    pub component: Option<NbtSummaryValue>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChatSignatureState {
    pub checksum: i32,
    pub bytes_len: usize,
    #[serde(skip)]
    pub(crate) bytes: Vec<u8>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChatAcknowledgementState {
    pub pending_offset: i32,
    pub last_processed_signature: Option<ProcessedChatSignatureState>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LastSeenMessagesTrackerState {
    pub entries: Vec<Option<LastSeenTrackedMessageState>>,
    pub tail: usize,
}

impl Default for LastSeenMessagesTrackerState {
    fn default() -> Self {
        Self {
            entries: vec![None; LAST_SEEN_MESSAGES_TRACKER_CAPACITY],
            tail: 0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LastSeenTrackedMessageState {
    pub signature: ProcessedChatSignatureState,
    pub pending: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProcessedChatSignatureState {
    pub checksum: i32,
    pub bytes: Vec<u8>,
}

pub use bbb_protocol::packets::OutboundLastSeenMessages;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeletedChatState {
    pub signature: Option<ChatSignatureState>,
    pub cache_id: Option<i32>,
    pub resolved: bool,
    #[serde(default)]
    pub requested_tick: u64,
    #[serde(default)]
    pub deletable_after_tick: Option<u64>,
    #[serde(default)]
    pub visible_replaced: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ChatValidationState {
    Unchecked,
    Validated,
    Unsigned,
    UnknownSender,
    MissingProfileKey,
    UnknownCachedSignature,
    ExpiredProfileKey,
    InvalidSignature,
    OutOfOrder,
    ChainBroken,
}

impl ChatValidationState {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Unchecked => "unchecked",
            Self::Validated => "validated",
            Self::Unsigned => "unsigned",
            Self::UnknownSender => "unknown_sender",
            Self::MissingProfileKey => "missing_profile_key",
            Self::UnknownCachedSignature => "unknown_cached_signature",
            Self::ExpiredProfileKey => "expired_profile_key",
            Self::InvalidSignature => "invalid_signature",
            Self::OutOfOrder => "out_of_order",
            Self::ChainBroken => "chain_broken",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChatDisconnectReason {
    BadGlobalIndex,
    InvalidPackedSignature,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RemoteChatSessionState {
    session_id: Uuid,
    expires_at_epoch_millis: i64,
    public_key_der: Vec<u8>,
    chain_valid: bool,
    last_message: Option<ValidatedRemoteChatMessageState>,
}

impl RemoteChatSessionState {
    pub(crate) fn new(
        session_id: Uuid,
        expires_at_epoch_millis: i64,
        public_key_der: Vec<u8>,
    ) -> Self {
        Self {
            session_id,
            expires_at_epoch_millis,
            public_key_der,
            chain_valid: true,
            last_message: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ValidatedRemoteChatMessageState {
    index: i32,
    sender: Uuid,
    session_id: Uuid,
    signature: Vec<u8>,
    content: String,
    timestamp_millis: i64,
    salt: i64,
    last_seen: Vec<Vec<u8>>,
    unsigned_content: Option<String>,
    unsigned_content_styled: Option<Vec<StyledTextRun>>,
    unsigned_content_component: Option<NbtSummaryValue>,
    filter_mask: ProtocolFilterMask,
}

#[derive(Debug, Default)]
pub(crate) struct PlayerChatApplyOutcome {
    pub(crate) acknowledgement: Option<ProtocolChatAcknowledgement>,
    pub(crate) disconnect: Option<ChatDisconnectReason>,
}

#[derive(Debug, Default)]
struct ChatDeliveryOutcome {
    acknowledgement: Option<ProtocolChatAcknowledgement>,
    shown: bool,
}

impl WorldStore {
    pub fn apply_reset_chat(&mut self) {
        self.counters.reset_chat_packets += 1;
        let presentation = self.client_chat.presentation.clone();
        self.client_chat = ClientChatState {
            presentation,
            ..ClientChatState::default()
        }
        .into();
        refresh_chat_counters(self);
    }

    pub fn clear_client_chat_display_messages(&mut self) {
        if !self.client_chat.messages.is_empty() || !self.client_chat.deleted_messages.is_empty() {
            self.client_chat.delivery_revision =
                self.client_chat.delivery_revision.saturating_add(1);
        }
        self.client_chat.messages.clear();
        self.client_chat.deleted_messages.clear();
        refresh_chat_counters(self);
    }

    pub(crate) fn reset_chat_session_state(&mut self) {
        self.client_chat.expected_player_chat_global_index = 0;
        self.client_chat.signature_cache = vec![None; SIGNATURE_CACHE_CAPACITY];
        self.client_chat.acknowledgement = ChatAcknowledgementState::default();
        self.client_chat.last_seen_messages = LastSeenMessagesTrackerState::default();
        refresh_chat_counters(self);
    }

    pub fn set_chat_presentation_config(&mut self, mut config: ChatPresentationConfig) {
        config.normalize_player_sets();
        self.client_chat.presentation = config;
        refresh_chat_counters(self);
    }

    pub fn chat_presentation_config(&self) -> &ChatPresentationConfig {
        &self.client_chat.presentation
    }

    pub fn take_pending_chat_narrations(&mut self) -> Vec<ChatNarrationRequest> {
        std::mem::take(&mut self.client_chat.pending_narrations)
    }

    pub(crate) fn push_overlay_chat_narration(
        &mut self,
        content: String,
        styled_content: Vec<StyledTextRun>,
    ) {
        self.push_pending_chat_narration(ChatNarrationKind::System, content, styled_content, None);
    }

    pub(crate) fn push_interrupting_system_narration(
        &mut self,
        content: String,
        styled_content: Vec<StyledTextRun>,
    ) {
        self.push_pending_chat_narration(
            ChatNarrationKind::SystemNow,
            content,
            styled_content,
            None,
        );
    }

    pub(crate) fn push_interrupting_system_component_narration(
        &mut self,
        component: &RichTextComponent,
    ) {
        self.push_pending_chat_narration(
            ChatNarrationKind::SystemNow,
            component.text().to_string(),
            component.styled().to_vec(),
            Some(component.component().clone()),
        );
    }

    fn push_pending_chat_narration(
        &mut self,
        kind: ChatNarrationKind,
        content: String,
        styled_content: Vec<StyledTextRun>,
        content_component: Option<NbtSummaryValue>,
    ) {
        let sequence = self.client_chat.delivery_revision.saturating_add(1);
        self.client_chat.delivery_revision = sequence;
        self.client_chat
            .pending_narrations
            .push(ChatNarrationRequest {
                sequence,
                kind,
                translation_key: None,
                arguments: Vec::new(),
                content_component,
                styled_content,
                content,
            });
        if self.client_chat.pending_narrations.len() > presentation::MAX_PENDING_CHAT_NARRATIONS {
            let excess = self.client_chat.pending_narrations.len()
                - presentation::MAX_PENDING_CHAT_NARRATIONS;
            self.client_chat.pending_narrations.drain(..excess);
        }
    }

    pub fn push_client_system_chat_message(&mut self, content: impl Into<String>) {
        let content = content.into();
        let styled_content = plain_chat_styled_content(&content);
        self.push_system_chat_message_state(
            ChatMessageKind::ClientSystem,
            content,
            None,
            styled_content,
            None,
            0,
            ChatNarrationKind::SystemChat,
        );
    }

    /// `KeyboardHandler.showDebugChat`: `addClientSystemMessage` plus
    /// `GameNarrator.saySystemQueued`, so the narration is gated on the
    /// system-only statuses rather than the chat-or-system pair.
    pub fn push_styled_client_system_chat_message(&mut self, styled_content: Vec<StyledTextRun>) {
        let content = styled_chat_summary_text(&styled_content);
        self.push_system_chat_message_state(
            ChatMessageKind::ClientSystem,
            content,
            None,
            styled_content,
            None,
            0,
            ChatNarrationKind::System,
        );
    }

    pub fn push_translatable_client_system_chat_message(&mut self, translation_key: &str) {
        self.push_translatable_client_system_chat_message_with_arguments(
            translation_key,
            &[],
            ChatNarrationKind::SystemChat,
        );
    }

    pub fn push_translatable_client_system_chat_message_with_arguments(
        &mut self,
        translation_key: &str,
        arguments: &[&str],
        narration_kind: ChatNarrationKind,
    ) {
        let content_component = translatable_component_nbt(translation_key, arguments);
        self.push_system_chat_message_state(
            ChatMessageKind::ClientSystem,
            translation_key.to_string(),
            Some(translation_key.to_string()),
            Vec::new(),
            Some(content_component),
            0,
            narration_kind,
        );
    }

    pub fn push_server_system_chat_message(&mut self, content: impl Into<String>) {
        let content = content.into();
        let styled_content = plain_chat_styled_content(&content);
        self.push_rich_server_system_chat_message(content, styled_content, None, 0);
    }

    pub(crate) fn push_rich_server_system_chat_message(
        &mut self,
        content: String,
        styled_content: Vec<StyledTextRun>,
        content_component: Option<NbtSummaryValue>,
        received_epoch_millis: i64,
    ) {
        if !self.client_chat.presentation.can_receive_system_messages()
            || self.system_chat_matches_hidden_player(&content)
        {
            return;
        }
        self.push_system_chat_message_state(
            ChatMessageKind::System,
            content,
            None,
            styled_content,
            content_component,
            received_epoch_millis,
            ChatNarrationKind::SystemChat,
        );
    }

    fn system_chat_matches_hidden_player(&self, content: &str) -> bool {
        if !self.client_chat.presentation.hide_matched_names {
            return false;
        }
        let Some(open) = content.find('<') else {
            return false;
        };
        let Some(close_offset) = content[open + 1..].find('>') else {
            return false;
        };
        let name = &content[open + 1..open + 1 + close_offset];
        // Vanilla `ChatListener.guessChatUUID` resolves the mention through
        // `PlayerSocialManager.getDiscoveredUUID`, which keeps every name ever
        // seen in a player-info add — so a hidden player stays matched after
        // leaving the server, unlike a live tab-list scan.
        self.social_discovered_uuid(name)
            .is_some_and(|uuid| self.social_should_hide_message_from(uuid))
    }

    #[allow(clippy::too_many_arguments)]
    fn push_system_chat_message_state(
        &mut self,
        kind: ChatMessageKind,
        content: String,
        content_translation_key: Option<String>,
        styled_content: Vec<StyledTextRun>,
        content_component: Option<NbtSummaryValue>,
        received_epoch_millis: i64,
        narration_kind: ChatNarrationKind,
    ) {
        let source = match kind {
            ChatMessageKind::System => ChatMessageSourceState::SystemServer,
            ChatMessageKind::ClientSystem => ChatMessageSourceState::SystemClient,
            ChatMessageKind::Player | ChatMessageKind::Disguised => ChatMessageSourceState::Player,
        };
        let tag = Some(match kind {
            ChatMessageKind::ClientSystem => ChatMessageTagState::SystemSinglePlayer,
            ChatMessageKind::System | ChatMessageKind::Player | ChatMessageKind::Disguised => {
                ChatMessageTagState::System
            }
        });
        let message = ChatMessageState {
            kind,
            source,
            content,
            content_translation_key,
            styled_content,
            content_component,
            decoration: None,
            narration_decoration: None,
            sender: None,
            sender_name: String::new(),
            target_name: None,
            global_index: None,
            message_index: None,
            chat_type: ChatTypeState {
                registry_id: None,
                resource_id: None,
                source: None,
                direct_translation_key: None,
                resolution_error: None,
            },
            signature: None,
            unsigned_content: None,
            filter_mask: ProtocolFilterMaskKind::PassThrough.as_str().to_string(),
            validation_state: ChatValidationState::Unsigned,
            trust_level: ChatTrustLevelState::NotSecure,
            tag,
            received_epoch_millis,
            display_sequence: 0,
            filter_mask_words: Vec::new(),
            unsigned_content_has_custom_font: false,
            added_tick: self.client_chat.gui_ticks,
            full_signature: None,
        };
        if kind == ChatMessageKind::System {
            self.push_social_chat_identity(&message);
        }
        self.push_visible_chat_message(message, narration_kind);
        refresh_chat_counters(self);
    }

    pub fn apply_player_chat(&mut self, packet: PlayerChat) -> Option<ProtocolChatAcknowledgement> {
        self.apply_player_chat_at(packet, i64::MIN).acknowledgement
    }

    pub(crate) fn apply_player_chat_at(
        &mut self,
        packet: PlayerChat,
        now_epoch_millis: i64,
    ) -> PlayerChatApplyOutcome {
        self.apply_player_chat_at_times(packet, now_epoch_millis, u64::MAX)
    }

    pub(crate) fn apply_player_chat_at_times(
        &mut self,
        packet: PlayerChat,
        now_epoch_millis: i64,
        now_monotonic_millis: u64,
    ) -> PlayerChatApplyOutcome {
        self.counters.player_chat_packets += 1;
        if !self.preflight_packet_components(&packet, "player chat") {
            return PlayerChatApplyOutcome::default();
        }
        let resolved_chat_type =
            resolve_chat_type(self.frozen_registry_access(), &packet.chat_type.chat_type);

        let expected_global_index = self.client_chat.expected_player_chat_global_index;
        self.client_chat.expected_player_chat_global_index = self
            .client_chat
            .expected_player_chat_global_index
            .wrapping_add(1);
        if packet.global_index != expected_global_index {
            self.counters.player_chat_index_mismatches += 1;
            refresh_chat_counters(self);
            return PlayerChatApplyOutcome {
                disconnect: Some(ChatDisconnectReason::BadGlobalIndex),
                ..PlayerChatApplyOutcome::default()
            };
        }

        let mut resolved_last_seen = Vec::with_capacity(packet.body.last_seen.len());
        let cache_snapshot = self.client_chat.signature_cache.clone();
        for packed in &packet.body.last_seen {
            let Some(signature) = resolve_packed_signature(&cache_snapshot, packed) else {
                self.counters.chat_unknown_packed_signatures += 1;
                refresh_chat_counters(self);
                return PlayerChatApplyOutcome {
                    disconnect: Some(ChatDisconnectReason::InvalidPackedSignature),
                    ..PlayerChatApplyOutcome::default()
                };
            };
            resolved_last_seen.push(signature);
        }
        let own_signature = packet.signature.as_ref().map(chat_signature_state);
        push_signature_cache(
            &mut self.client_chat.signature_cache,
            resolved_last_seen.clone(),
            own_signature,
        );

        let enforces_secure_chat = self.effectively_enforces_secure_chat();
        let validation_state = match self.player_info.entries.get_mut(&packet.sender) {
            None => ChatValidationState::UnknownSender,
            Some(sender) => match sender.chat_session.as_mut() {
                Some(session) => validate_remote_player_chat(
                    session,
                    &packet,
                    &resolved_last_seen,
                    now_epoch_millis,
                ),
                None if enforces_secure_chat => ChatValidationState::MissingProfileKey,
                None => ChatValidationState::Unsigned,
            },
        };
        let validation_error = matches!(
            validation_state,
            ChatValidationState::UnknownSender
                | ChatValidationState::MissingProfileKey
                | ChatValidationState::ExpiredProfileKey
                | ChatValidationState::InvalidSignature
                | ChatValidationState::OutOfOrder
                | ChatValidationState::ChainBroken
        );
        let acknowledge_signature = (validation_state != ChatValidationState::Unsigned)
            .then(|| {
                packet
                    .signature
                    .as_ref()
                    .map(processed_chat_signature_state)
            })
            .flatten();
        if packet.unsigned_content.is_some() {
            self.counters.player_chat_unsigned_content_packets += 1;
        }
        match packet.filter_mask.kind {
            ProtocolFilterMaskKind::FullyFiltered if !validation_error => {
                self.counters.player_chat_fully_filtered_packets += 1;
            }
            ProtocolFilterMaskKind::PartiallyFiltered if !validation_error => {
                self.counters.player_chat_filtered_packets += 1;
            }
            ProtocolFilterMaskKind::PassThrough
            | ProtocolFilterMaskKind::PartiallyFiltered
            | ProtocolFilterMaskKind::FullyFiltered => {}
        }

        let only_show_secure = self.client_chat.presentation.only_show_secure;
        let unsigned_content_has_custom_font = packet
            .unsigned_content_component
            .as_ref()
            .is_some_and(component_has_custom_font);
        let displayed_content = if validation_error {
            "chat.validation_error"
        } else if only_show_secure {
            &packet.body.content
        } else {
            packet
                .unsigned_content
                .as_deref()
                .unwrap_or(&packet.body.content)
        };
        let (styled_content, content_component) = if validation_error {
            (
                vec![StyledTextRun {
                    text: displayed_content.to_string(),
                    style: ComponentStyle {
                        italic: Some(true),
                        color: Some(0xff5555),
                        ..ComponentStyle::default()
                    },
                }],
                Some(NbtSummaryValue::string(displayed_content)),
            )
        } else if only_show_secure {
            (
                plain_chat_styled_content(&packet.body.content),
                Some(NbtSummaryValue::string(packet.body.content.clone())),
            )
        } else {
            packet
                .unsigned_content
                .as_ref()
                .map(|unsigned| {
                    (
                        nonempty_styled_or_plain(
                            packet
                                .unsigned_content_styled
                                .as_deref()
                                .unwrap_or_default(),
                            unsigned,
                        ),
                        packet
                            .unsigned_content_component
                            .clone()
                            .or_else(|| Some(NbtSummaryValue::string(unsigned.clone()))),
                    )
                })
                .unwrap_or_else(|| {
                    (
                        plain_chat_styled_content(&packet.body.content),
                        Some(NbtSummaryValue::string(packet.body.content.clone())),
                    )
                })
        };
        let (chat_type, decoration, narration_decoration) = resolved_chat_state(
            resolved_chat_type,
            &packet.chat_type,
            displayed_content,
            &styled_content,
            content_component.as_ref(),
        );
        let content_translation_key = decoration
            .as_ref()
            .map(|decoration| decoration.translation_key.to_display_string());
        let message = ChatMessageState {
            kind: ChatMessageKind::Player,
            source: ChatMessageSourceState::Player,
            styled_content,
            content_component,
            content: if validation_error {
                displayed_content.to_string()
            } else {
                packet.body.content.clone()
            },
            content_translation_key,
            decoration,
            narration_decoration,
            sender: Some(packet.sender),
            sender_name: packet.chat_type.name,
            target_name: packet.chat_type.target_name,
            global_index: Some(packet.global_index),
            message_index: Some(packet.index),
            chat_type,
            signature: (!validation_error && validation_state == ChatValidationState::Validated)
                .then(|| packet.signature.as_ref().map(chat_signature_state))
                .flatten(),
            unsigned_content: (!validation_error)
                .then_some(packet.unsigned_content)
                .flatten(),
            filter_mask: if validation_error {
                ProtocolFilterMaskKind::PassThrough.as_str().to_string()
            } else {
                packet.filter_mask.kind.as_str().to_string()
            },
            validation_state,
            trust_level: ChatTrustLevelState::NotSecure,
            tag: validation_error.then_some(ChatMessageTagState::Error),
            received_epoch_millis: now_epoch_millis,
            display_sequence: 0,
            filter_mask_words: packet.filter_mask.mask_words,
            unsigned_content_has_custom_font,
            added_tick: self.client_chat.gui_ticks,
            full_signature: (!validation_error
                && validation_state == ChatValidationState::Validated)
                .then(|| packet.signature.map(|signature| signature.bytes))
                .flatten(),
        };
        let delayed = DelayedChatMessageState {
            message,
            only_show_secure,
            validation_error,
            acknowledge_signature,
            message_timestamp_millis: packet.body.timestamp_millis,
        };
        let acknowledgement = self.handle_or_delay_chat_message(delayed, now_monotonic_millis);
        refresh_chat_counters(self);
        PlayerChatApplyOutcome {
            acknowledgement,
            disconnect: None,
        }
    }

    pub fn apply_disguised_chat(&mut self, packet: ProtocolDisguisedChat) {
        self.apply_disguised_chat_at(packet, 0, u64::MAX);
    }

    pub(crate) fn apply_disguised_chat_at(
        &mut self,
        packet: ProtocolDisguisedChat,
        now_epoch_millis: i64,
        now_monotonic_millis: u64,
    ) {
        self.counters.disguised_chat_packets += 1;
        if !self.preflight_packet_components(&packet, "disguised chat") {
            return;
        }
        let resolved_chat_type =
            resolve_chat_type(self.frozen_registry_access(), &packet.chat_type.chat_type);
        let styled_content = nonempty_styled_or_plain(&packet.message_styled, &packet.message);
        let content_component = packet
            .message_component
            .clone()
            .or_else(|| Some(NbtSummaryValue::string(packet.message.clone())));
        let (chat_type, decoration, narration_decoration) = resolved_chat_state(
            resolved_chat_type,
            &packet.chat_type,
            &packet.message,
            &styled_content,
            content_component.as_ref(),
        );
        let content_translation_key = decoration
            .as_ref()
            .map(|decoration| decoration.translation_key.to_display_string());
        let message = ChatMessageState {
            kind: ChatMessageKind::Disguised,
            source: ChatMessageSourceState::SystemServer,
            styled_content,
            content_component,
            content: packet.message,
            content_translation_key,
            decoration,
            narration_decoration,
            sender: None,
            sender_name: packet.chat_type.name,
            target_name: packet.chat_type.target_name,
            global_index: None,
            message_index: None,
            chat_type,
            signature: None,
            unsigned_content: None,
            filter_mask: ProtocolFilterMaskKind::PassThrough.as_str().to_string(),
            validation_state: ChatValidationState::Unsigned,
            trust_level: ChatTrustLevelState::NotSecure,
            tag: Some(ChatMessageTagState::System),
            received_epoch_millis: now_epoch_millis,
            display_sequence: 0,
            filter_mask_words: Vec::new(),
            unsigned_content_has_custom_font: false,
            added_tick: self.client_chat.gui_ticks,
            full_signature: None,
        };
        let delayed = DelayedChatMessageState {
            message,
            only_show_secure: false,
            validation_error: false,
            acknowledge_signature: None,
            message_timestamp_millis: now_epoch_millis,
        };
        let _ = self.handle_or_delay_chat_message(delayed, now_monotonic_millis);
        refresh_chat_counters(self);
    }

    fn handle_or_delay_chat_message(
        &mut self,
        delayed: DelayedChatMessageState,
        now_monotonic_millis: u64,
    ) -> Option<ProtocolChatAcknowledgement> {
        let delay = self.client_chat.presentation.message_delay_millis;
        let will_delay = delay > 0
            && self.client_chat.last_delivery_monotonic_millis != 0
            && now_monotonic_millis
                < self
                    .client_chat
                    .last_delivery_monotonic_millis
                    .saturating_add(delay);
        if !will_delay {
            return self
                .accept_delayed_chat_message(delayed, now_monotonic_millis)
                .acknowledgement;
        }

        let mut acknowledgement = None;
        if self.client_chat.delayed_messages.len() >= presentation::MAX_DELAYED_CHAT_MESSAGES {
            let oldest = self
                .client_chat
                .delayed_messages
                .pop_front()
                .expect("bounded delayed chat queue was non-empty");
            acknowledgement = self
                .accept_delayed_chat_message(oldest, now_monotonic_millis)
                .acknowledgement;
        }
        self.client_chat.delayed_messages.push_back(delayed);
        acknowledgement
    }

    fn accept_delayed_chat_message(
        &mut self,
        mut delayed: DelayedChatMessageState,
        now_monotonic_millis: u64,
    ) -> ChatDeliveryOutcome {
        if delayed.validation_error {
            let acknowledgement = delayed
                .acknowledge_signature
                .as_ref()
                .and_then(|signature| {
                    self.record_processed_player_chat_signature_state(signature.clone(), false)
                });
            let sender = delayed.message.sender;
            if sender.is_some_and(|sender| self.social_should_hide_message_from(sender))
                || !self.client_chat.presentation.can_receive_player_messages()
            {
                return ChatDeliveryOutcome {
                    acknowledgement,
                    shown: false,
                };
            }
            delayed.message.tag = Some(ChatMessageTagState::Error);
            self.push_visible_chat_message(delayed.message, ChatNarrationKind::SystemChat);
            self.client_chat.last_delivery_monotonic_millis = now_monotonic_millis;
            return ChatDeliveryOutcome {
                acknowledgement,
                shown: true,
            };
        }

        if delayed.message.kind == ChatMessageKind::Disguised {
            if !self.client_chat.presentation.can_receive_player_messages() {
                return ChatDeliveryOutcome::default();
            }
            delayed.message.tag = Some(ChatMessageTagState::System);
            self.push_social_chat_identity(&delayed.message);
            self.push_visible_chat_message(delayed.message, ChatNarrationKind::Chat);
            self.client_chat.last_delivery_monotonic_millis = now_monotonic_millis;
            return ChatDeliveryOutcome {
                acknowledgement: None,
                shown: true,
            };
        }

        let trust_level = evaluate_chat_trust_level(&delayed);
        delayed.message.trust_level = trust_level;
        let sender_hidden = delayed
            .message
            .sender
            .is_some_and(|sender| self.social_should_hide_message_from(sender));
        let can_show = !(delayed.only_show_secure && trust_level == ChatTrustLevelState::NotSecure)
            && !sender_hidden
            && delayed.message.filter_mask != ProtocolFilterMaskKind::FullyFiltered.as_str()
            && self.client_chat.presentation.can_receive_player_messages();
        if can_show
            && delayed.message.filter_mask == ProtocolFilterMaskKind::PartiallyFiltered.as_str()
        {
            let (styled, component) =
                filtered_chat_content(&delayed.message.content, &delayed.message.filter_mask_words);
            delayed.message.styled_content = styled;
            delayed.message.content_component = component;
            replace_chat_decoration_content(&mut delayed.message);
        }
        delayed.message.tag = match trust_level {
            ChatTrustLevelState::Secure => None,
            ChatTrustLevelState::Modified => Some(ChatMessageTagState::Modified {
                original_content: delayed.message.content.clone(),
            }),
            ChatTrustLevelState::NotSecure => Some(ChatMessageTagState::NotSecure),
        };

        if can_show {
            self.push_social_chat_identity(&delayed.message);
            self.push_visible_chat_message(delayed.message, ChatNarrationKind::Chat);
            self.client_chat.last_delivery_monotonic_millis = now_monotonic_millis;
        }
        let acknowledgement = delayed.acknowledge_signature.and_then(|signature| {
            self.record_processed_player_chat_signature_state(signature, can_show)
        });
        ChatDeliveryOutcome {
            acknowledgement,
            shown: can_show,
        }
    }

    pub fn advance_chat_listener(
        &mut self,
        now_monotonic_millis: u64,
        paused: bool,
    ) -> Vec<ProtocolChatAcknowledgement> {
        if paused {
            if self.client_chat.presentation.message_delay_millis > 0 {
                self.client_chat.last_delivery_monotonic_millis = self
                    .client_chat
                    .last_delivery_monotonic_millis
                    .saturating_add(50);
            }
            return Vec::new();
        }

        let delay = self.client_chat.presentation.message_delay_millis;
        if delay == 0 {
            return self.flush_delayed_chat_messages(now_monotonic_millis);
        }
        if self.client_chat.last_delivery_monotonic_millis != 0
            && now_monotonic_millis
                < self
                    .client_chat
                    .last_delivery_monotonic_millis
                    .saturating_add(delay)
        {
            return Vec::new();
        }

        let mut acknowledgements = Vec::new();
        while !self.client_chat.delayed_messages.is_empty() {
            let delayed = self
                .client_chat
                .delayed_messages
                .pop_front()
                .expect("delayed chat queue was non-empty");
            let outcome = self.accept_delayed_chat_message(delayed, now_monotonic_millis);
            if let Some(acknowledgement) = outcome.acknowledgement {
                acknowledgements.push(acknowledgement);
            }
            if outcome.shown {
                break;
            }
        }
        refresh_chat_counters(self);
        acknowledgements
    }

    pub fn accept_next_delayed_chat_message(
        &mut self,
        now_monotonic_millis: u64,
    ) -> Option<ProtocolChatAcknowledgement> {
        if self.client_chat.delayed_messages.is_empty() {
            return None;
        }
        let delayed = self
            .client_chat
            .delayed_messages
            .pop_front()
            .expect("delayed chat queue was non-empty");
        let acknowledgement = self
            .accept_delayed_chat_message(delayed, now_monotonic_millis)
            .acknowledgement;
        refresh_chat_counters(self);
        acknowledgement
    }

    pub fn flush_delayed_chat_messages(
        &mut self,
        now_monotonic_millis: u64,
    ) -> Vec<ProtocolChatAcknowledgement> {
        let delayed = std::mem::take(&mut self.client_chat.delayed_messages);
        let mut acknowledgements = Vec::new();
        for message in delayed {
            if let Some(acknowledgement) = self
                .accept_delayed_chat_message(message, now_monotonic_millis)
                .acknowledgement
            {
                acknowledgements.push(acknowledgement);
            }
        }
        self.client_chat.last_delivery_monotonic_millis = 0;
        refresh_chat_counters(self);
        acknowledgements
    }

    fn push_visible_chat_message(
        &mut self,
        mut message: ChatMessageState,
        narration_kind: ChatNarrationKind,
    ) {
        let sequence = self.client_chat.delivery_revision.saturating_add(1);
        self.client_chat.delivery_revision = sequence;
        message.display_sequence = sequence;
        message.added_tick = self.client_chat.gui_ticks;
        self.push_chat_narration(&message, narration_kind);
        self.client_chat.messages.push(message);
        if self.client_chat.messages.len() > presentation::MAX_VISIBLE_CHAT_MESSAGES {
            let excess = self.client_chat.messages.len() - presentation::MAX_VISIBLE_CHAT_MESSAGES;
            self.client_chat.messages.drain(..excess);
        }
    }

    fn push_chat_narration(&mut self, message: &ChatMessageState, kind: ChatNarrationKind) {
        let narration = message.narration_decoration.as_ref();
        self.client_chat
            .pending_narrations
            .push(ChatNarrationRequest {
                sequence: message.display_sequence,
                kind,
                translation_key: narration.map(|decoration| decoration.translation_key.clone()),
                arguments: narration
                    .map(|decoration| decoration.arguments.clone())
                    .unwrap_or_default(),
                content_component: message.content_component.clone(),
                styled_content: message.styled_content.clone(),
                content: styled_chat_summary_text(&message.styled_content),
            });
        if self.client_chat.pending_narrations.len() > presentation::MAX_PENDING_CHAT_NARRATIONS {
            let excess = self.client_chat.pending_narrations.len()
                - presentation::MAX_PENDING_CHAT_NARRATIONS;
            self.client_chat.pending_narrations.drain(..excess);
        }
    }

    fn push_social_chat_identity(&mut self, message: &ChatMessageState) {
        self.client_chat.social_history.push(RecentChatIdentity {
            sender: (message.kind == ChatMessageKind::Player)
                .then_some(message.sender)
                .flatten(),
            sender_name: (!message.sender_name.is_empty()).then(|| message.sender_name.clone()),
            signed: message.signature.is_some(),
        });
    }

    pub fn apply_delete_chat(
        &mut self,
        packet: ProtocolDeleteChat,
    ) -> Option<ChatDisconnectReason> {
        self.counters.delete_chat_packets += 1;
        let signature =
            resolve_packed_signature(&self.client_chat.signature_cache, &packet.message_signature);
        let Some(signature) = signature else {
            self.counters.chat_unknown_packed_signatures += 1;
            refresh_chat_counters(self);
            return Some(ChatDisconnectReason::InvalidPackedSignature);
        };
        self.ignore_pending_last_seen_message(&ProcessedChatSignatureState {
            checksum: signature.checksum,
            bytes: signature.bytes.clone(),
        });

        let chat = &mut *self.client_chat;
        let delayed_len = chat.delayed_messages.len();
        chat.delayed_messages
            .retain(|message| message.signature_bytes() != Some(&signature.bytes));
        let removed_delayed = chat.delayed_messages.len() != delayed_len;
        let gui_ticks = chat.gui_ticks;
        let (visible_replaced, deletable_after_tick) = if removed_delayed {
            (false, None)
        } else {
            replace_visible_chat_message_or_delay(&mut chat.messages, &signature.bytes, gui_ticks)
        };
        if visible_replaced {
            chat.delivery_revision = chat.delivery_revision.saturating_add(1);
            chat.display_refresh_revision = chat.display_refresh_revision.saturating_add(1);
        }
        chat.deleted_messages.push(DeletedChatState {
            signature: Some(signature),
            cache_id: packet.message_signature.cache_id,
            resolved: true,
            requested_tick: gui_ticks,
            deletable_after_tick,
            visible_replaced,
        });
        prune_deleted_chat_messages(&mut chat.deleted_messages);
        refresh_chat_counters(self);
        None
    }

    pub fn advance_chat_gui_ticks(&mut self, ticks: u32) {
        self.client_chat.gui_ticks = self.client_chat.gui_ticks.saturating_add(u64::from(ticks));
        let now = self.client_chat.gui_ticks;
        let due: Vec<_> = self
            .client_chat
            .deleted_messages
            .iter()
            .enumerate()
            .filter_map(|(index, deletion)| {
                deletion
                    .deletable_after_tick
                    .is_some_and(|deadline| now >= deadline)
                    .then(|| {
                        deletion
                            .signature
                            .as_ref()
                            .map(|signature| (index, signature.bytes.clone()))
                    })
                    .flatten()
            })
            .collect();
        for (index, signature) in due {
            let (replaced, next_deadline) = replace_visible_chat_message_or_delay(
                &mut self.client_chat.messages,
                &signature,
                now,
            );
            let deletion = &mut self.client_chat.deleted_messages[index];
            deletion.visible_replaced |= replaced;
            deletion.deletable_after_tick = next_deadline;
            if replaced {
                self.client_chat.delivery_revision =
                    self.client_chat.delivery_revision.saturating_add(1);
                self.client_chat.display_refresh_revision =
                    self.client_chat.display_refresh_revision.saturating_add(1);
            }
        }
        prune_deleted_chat_messages(&mut self.client_chat.deleted_messages);
        refresh_chat_counters(self);
    }

    pub fn client_chat(&self) -> &ClientChatState {
        &self.client_chat
    }

    pub fn take_pending_player_chat_acknowledgement(
        &mut self,
    ) -> Option<ProtocolChatAcknowledgement> {
        let offset = self.client_chat.acknowledgement.pending_offset;
        if offset <= 0 {
            return None;
        }

        self.client_chat.acknowledgement.pending_offset = 0;
        self.counters.player_chat_acknowledgement_packets += 1;
        refresh_chat_counters(self);
        Some(ProtocolChatAcknowledgement { offset })
    }

    pub fn take_last_seen_messages_for_outbound_chat(&mut self) -> OutboundLastSeenMessages {
        let offset = self.client_chat.acknowledgement.pending_offset;
        self.client_chat.acknowledgement.pending_offset = 0;

        let mut acknowledged = 0_u32;
        let mut checksum = 1_i32;
        let mut signatures = Vec::new();
        let len = self.client_chat.last_seen_messages.entries.len();
        if len == 0 {
            refresh_chat_counters(self);
            return OutboundLastSeenMessages {
                update: LastSeenMessagesUpdate {
                    offset,
                    ..LastSeenMessagesUpdate::default()
                },
                signatures,
            };
        }
        for i in 0..len {
            let index = (self.client_chat.last_seen_messages.tail + i) % len;
            if let Some(entry) = self.client_chat.last_seen_messages.entries[index].as_mut() {
                acknowledged |= 1 << i;
                checksum = checksum
                    .wrapping_mul(31)
                    .wrapping_add(entry.signature.checksum);
                signatures.push(ProtocolMessageSignature {
                    bytes: entry.signature.bytes.clone(),
                });
                entry.pending = false;
            }
        }

        refresh_chat_counters(self);
        OutboundLastSeenMessages {
            update: LastSeenMessagesUpdate {
                offset,
                acknowledged,
                checksum: last_seen_checksum_byte(checksum),
            },
            signatures,
        }
    }

    #[cfg(test)]
    fn record_processed_player_chat_signature(
        &mut self,
        signature: &ProtocolMessageSignature,
        was_shown: bool,
    ) -> Option<ProtocolChatAcknowledgement> {
        self.record_processed_player_chat_signature_state(
            processed_chat_signature_state(signature),
            was_shown,
        )
    }

    fn record_processed_player_chat_signature_state(
        &mut self,
        processed_signature: ProcessedChatSignatureState,
        was_shown: bool,
    ) -> Option<ProtocolChatAcknowledgement> {
        if self
            .client_chat
            .acknowledgement
            .last_processed_signature
            .as_ref()
            == Some(&processed_signature)
        {
            refresh_chat_counters(self);
            return None;
        }

        self.add_last_seen_tracked_message(processed_signature.clone(), was_shown);
        self.client_chat.acknowledgement.last_processed_signature = Some(processed_signature);
        self.client_chat.acknowledgement.pending_offset = self
            .client_chat
            .acknowledgement
            .pending_offset
            .saturating_add(1);

        let acknowledgement = if self.client_chat.acknowledgement.pending_offset
            > CHAT_ACKNOWLEDGEMENT_OFFSET_THRESHOLD
        {
            let offset = self.client_chat.acknowledgement.pending_offset;
            self.client_chat.acknowledgement.pending_offset = 0;
            self.counters.player_chat_acknowledgement_packets += 1;
            Some(ProtocolChatAcknowledgement { offset })
        } else {
            None
        };
        refresh_chat_counters(self);
        acknowledgement
    }

    fn add_last_seen_tracked_message(
        &mut self,
        signature: ProcessedChatSignatureState,
        was_shown: bool,
    ) {
        let len = self.client_chat.last_seen_messages.entries.len();
        if len == 0 {
            self.client_chat.last_seen_messages = LastSeenMessagesTrackerState::default();
        }
        let len = self.client_chat.last_seen_messages.entries.len();
        let index = self.client_chat.last_seen_messages.tail % len;
        self.client_chat.last_seen_messages.tail = (index + 1) % len;
        self.client_chat.last_seen_messages.entries[index] =
            was_shown.then_some(LastSeenTrackedMessageState {
                signature,
                pending: true,
            });
    }

    fn ignore_pending_last_seen_message(&mut self, signature: &ProcessedChatSignatureState) {
        for entry in &mut self.client_chat.last_seen_messages.entries.iter_mut() {
            if entry
                .as_ref()
                .is_some_and(|entry| entry.pending && &entry.signature == signature)
            {
                *entry = None;
                break;
            }
        }
    }
}

fn validate_remote_player_chat(
    session: &mut RemoteChatSessionState,
    packet: &PlayerChat,
    last_seen: &[ChatSignatureState],
    now_epoch_millis: i64,
) -> ChatValidationState {
    if !session.chain_valid {
        return ChatValidationState::ChainBroken;
    }
    if session
        .expires_at_epoch_millis
        .saturating_add(PROFILE_PUBLIC_KEY_EXPIRY_GRACE_MILLIS)
        < now_epoch_millis
    {
        session.chain_valid = false;
        return ChatValidationState::ExpiredProfileKey;
    }
    let Some(signature) = packet.signature.as_ref() else {
        session.chain_valid = false;
        return ChatValidationState::InvalidSignature;
    };
    let candidate = ValidatedRemoteChatMessageState {
        index: packet.index,
        sender: packet.sender,
        session_id: session.session_id,
        signature: signature.bytes.clone(),
        content: packet.body.content.clone(),
        timestamp_millis: packet.body.timestamp_millis,
        salt: packet.body.salt,
        last_seen: last_seen
            .iter()
            .map(|signature| signature.bytes.clone())
            .collect(),
        unsigned_content: packet.unsigned_content.clone(),
        unsigned_content_styled: packet.unsigned_content_styled.clone(),
        unsigned_content_component: packet.unsigned_content_component.clone(),
        filter_mask: packet.filter_mask.clone(),
    };
    if !remote_message_signature_is_valid(session, &candidate) {
        session.chain_valid = false;
        return ChatValidationState::InvalidSignature;
    }
    if session.last_message.as_ref() == Some(&candidate) {
        return ChatValidationState::Validated;
    }
    if session.last_message.as_ref().is_some_and(|last| {
        candidate.index <= last.index
            || candidate.sender != last.sender
            || candidate.session_id != last.session_id
    }) {
        session.chain_valid = false;
        return ChatValidationState::OutOfOrder;
    }
    session.last_message = Some(candidate);
    ChatValidationState::Validated
}

const MAX_DELETED_CHAT_MESSAGES: usize = 256;

fn prune_deleted_chat_messages(deleted: &mut Vec<DeletedChatState>) {
    if deleted.len() <= MAX_DELETED_CHAT_MESSAGES {
        return;
    }
    let mut excess = deleted.len() - MAX_DELETED_CHAT_MESSAGES;
    deleted.retain(|entry| {
        if excess > 0 && entry.deletable_after_tick.is_none() {
            excess -= 1;
            false
        } else {
            true
        }
    });
    if deleted.len() > MAX_DELETED_CHAT_MESSAGES {
        let overflow = deleted.len() - MAX_DELETED_CHAT_MESSAGES;
        deleted.drain(..overflow);
    }
}

#[cfg(test)]
mod tests;
