use bbb_protocol::{
    packets::{
        ChatAcknowledgement as ProtocolChatAcknowledgement,
        ChatTypeHolder as ProtocolChatTypeHolder, DeleteChat as ProtocolDeleteChat,
        DisguisedChat as ProtocolDisguisedChat, FilterMaskKind as ProtocolFilterMaskKind,
        LastSeenMessagesUpdate, MessageSignature as ProtocolMessageSignature,
        PackedMessageSignature, PlayerChat,
    },
    ComponentStyle, NbtSummaryValue, StyledTextRun,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    registries::{
        resolve_chat_type, ChatTypeDecoration, ChatTypeDecorationParameter, ResolvedChatType,
    },
    RegistryEntrySource, WorldStore,
};

const SIGNATURE_CACHE_CAPACITY: usize = 128;
const LAST_SEEN_MESSAGES_TRACKER_CAPACITY: usize = 20;
const CHAT_ACKNOWLEDGEMENT_OFFSET_THRESHOLD: i32 = 64;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientChatState {
    pub messages: Vec<ChatMessageState>,
    pub deleted_messages: Vec<DeletedChatState>,
    pub expected_player_chat_global_index: i32,
    pub signature_cache: Vec<Option<ChatSignatureState>>,
    #[serde(default)]
    pub acknowledgement: ChatAcknowledgementState,
    #[serde(default)]
    pub last_seen_messages: LastSeenMessagesTrackerState,
}

impl Default for ClientChatState {
    fn default() -> Self {
        Self {
            messages: Vec::new(),
            deleted_messages: Vec::new(),
            expected_player_chat_global_index: 0,
            signature_cache: vec![None; SIGNATURE_CACHE_CAPACITY],
            acknowledgement: ChatAcknowledgementState::default(),
            last_seen_messages: LastSeenMessagesTrackerState::default(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChatMessageState {
    pub kind: ChatMessageKind,
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
    pub translation_key: String,
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

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct OutboundLastSeenMessages {
    pub update: LastSeenMessagesUpdate,
    pub signatures: Vec<ProtocolMessageSignature>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeletedChatState {
    pub signature: Option<ChatSignatureState>,
    pub cache_id: Option<i32>,
    pub resolved: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ChatValidationState {
    Unchecked,
    Unsigned,
    UnknownSender,
    UnknownCachedSignature,
    OutOfOrder,
}

impl ChatValidationState {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Unchecked => "unchecked",
            Self::Unsigned => "unsigned",
            Self::UnknownSender => "unknown_sender",
            Self::UnknownCachedSignature => "unknown_cached_signature",
            Self::OutOfOrder => "out_of_order",
        }
    }
}

impl WorldStore {
    pub fn apply_reset_chat(&mut self) {
        self.counters.reset_chat_packets += 1;
        self.client_chat = ClientChatState::default();
        refresh_chat_counters(self);
    }

    pub fn clear_client_chat_display_messages(&mut self) {
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

    pub fn push_client_system_chat_message(&mut self, content: impl Into<String>) {
        let content = content.into();
        let styled_content = plain_chat_styled_content(&content);
        self.push_system_chat_message_state(
            ChatMessageKind::ClientSystem,
            content,
            None,
            styled_content,
        );
    }

    pub fn push_styled_client_system_chat_message(&mut self, styled_content: Vec<StyledTextRun>) {
        let content = styled_chat_summary_text(&styled_content);
        self.push_system_chat_message_state(
            ChatMessageKind::ClientSystem,
            content,
            None,
            styled_content,
        );
    }

    pub fn push_translatable_client_system_chat_message(&mut self, translation_key: &str) {
        self.push_system_chat_message_state(
            ChatMessageKind::ClientSystem,
            translation_key.to_string(),
            Some(translation_key.to_string()),
            Vec::new(),
        );
    }

    pub fn push_server_system_chat_message(&mut self, content: impl Into<String>) {
        let content = content.into();
        let styled_content = plain_chat_styled_content(&content);
        self.push_system_chat_message_state(ChatMessageKind::System, content, None, styled_content);
    }

    fn push_system_chat_message_state(
        &mut self,
        kind: ChatMessageKind,
        content: String,
        content_translation_key: Option<String>,
        styled_content: Vec<StyledTextRun>,
    ) {
        self.client_chat.messages.push(ChatMessageState {
            kind,
            content,
            content_translation_key,
            styled_content,
            content_component: None,
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
        });
        refresh_chat_counters(self);
    }

    pub fn apply_player_chat(&mut self, packet: PlayerChat) -> Option<ProtocolChatAcknowledgement> {
        let resolved_chat_type =
            resolve_chat_type(self.frozen_registry_access(), &packet.chat_type.chat_type);
        self.counters.player_chat_packets += 1;
        let mut validation_state = if packet.signature.is_some() {
            ChatValidationState::Unchecked
        } else {
            ChatValidationState::Unsigned
        };

        let expected_global_index = self.client_chat.expected_player_chat_global_index;
        self.client_chat.expected_player_chat_global_index = self
            .client_chat
            .expected_player_chat_global_index
            .saturating_add(1);
        let index_matches = packet.global_index == expected_global_index;
        if !index_matches {
            self.counters.player_chat_index_mismatches += 1;
            validation_state = ChatValidationState::OutOfOrder;
        }

        if index_matches {
            let mut resolved_last_seen = Vec::with_capacity(packet.body.last_seen.len());
            let mut unknown_cached_signature = false;
            let cache_snapshot = self.client_chat.signature_cache.clone();
            for packed in &packet.body.last_seen {
                match resolve_packed_signature(&cache_snapshot, packed) {
                    Some(signature) => resolved_last_seen.push(signature),
                    None => {
                        unknown_cached_signature = true;
                        self.counters.chat_unknown_packed_signatures += 1;
                    }
                }
            }
            if unknown_cached_signature {
                validation_state = ChatValidationState::UnknownCachedSignature;
            } else {
                let own_signature = packet.signature.as_ref().map(chat_signature_state);
                push_signature_cache(
                    &mut self.client_chat.signature_cache,
                    resolved_last_seen,
                    own_signature.clone(),
                );
            }
        }

        if packet.unsigned_content.is_some() {
            self.counters.player_chat_unsigned_content_packets += 1;
        }
        let was_shown = packet.filter_mask.kind != ProtocolFilterMaskKind::FullyFiltered;
        let acknowledgement = packet.signature.as_ref().and_then(|signature| {
            self.record_processed_player_chat_signature(signature, was_shown)
        });
        match packet.filter_mask.kind {
            ProtocolFilterMaskKind::FullyFiltered => {
                self.counters.player_chat_fully_filtered_packets += 1;
            }
            ProtocolFilterMaskKind::PartiallyFiltered => {
                self.counters.player_chat_filtered_packets += 1;
            }
            ProtocolFilterMaskKind::PassThrough => {}
        }

        let (styled_content, content_component) = packet
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
                        .or_else(|| Some(NbtSummaryValue::String(unsigned.clone()))),
                )
            })
            .unwrap_or_else(|| {
                (
                    plain_chat_styled_content(&packet.body.content),
                    Some(NbtSummaryValue::String(packet.body.content.clone())),
                )
            });
        let (chat_type, decoration, narration_decoration) = resolved_chat_state(
            resolved_chat_type,
            &packet.chat_type,
            packet
                .unsigned_content
                .as_deref()
                .unwrap_or(&packet.body.content),
            &styled_content,
            content_component.as_ref(),
        );
        let content_translation_key = decoration
            .as_ref()
            .map(|decoration| decoration.translation_key.clone());
        let message = ChatMessageState {
            kind: ChatMessageKind::Player,
            styled_content,
            content_component,
            content: packet.body.content,
            content_translation_key,
            decoration,
            narration_decoration,
            sender: Some(packet.sender),
            sender_name: packet.chat_type.name,
            target_name: packet.chat_type.target_name,
            global_index: Some(packet.global_index),
            message_index: Some(packet.index),
            chat_type,
            signature: packet.signature.as_ref().map(chat_signature_state),
            unsigned_content: packet.unsigned_content,
            filter_mask: packet.filter_mask.kind.as_str().to_string(),
            validation_state,
        };
        self.client_chat.messages.push(message);
        refresh_chat_counters(self);
        acknowledgement
    }

    pub fn apply_disguised_chat(&mut self, packet: ProtocolDisguisedChat) {
        let resolved_chat_type =
            resolve_chat_type(self.frozen_registry_access(), &packet.chat_type.chat_type);
        self.counters.disguised_chat_packets += 1;
        let styled_content = nonempty_styled_or_plain(&packet.message_styled, &packet.message);
        let content_component = packet
            .message_component
            .clone()
            .or_else(|| Some(NbtSummaryValue::String(packet.message.clone())));
        let (chat_type, decoration, narration_decoration) = resolved_chat_state(
            resolved_chat_type,
            &packet.chat_type,
            &packet.message,
            &styled_content,
            content_component.as_ref(),
        );
        let content_translation_key = decoration
            .as_ref()
            .map(|decoration| decoration.translation_key.clone());
        self.client_chat.messages.push(ChatMessageState {
            kind: ChatMessageKind::Disguised,
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
        });
        refresh_chat_counters(self);
    }

    pub fn apply_delete_chat(&mut self, packet: ProtocolDeleteChat) {
        self.counters.delete_chat_packets += 1;
        if let Some(signature) = packet.message_signature.full_signature.as_ref() {
            self.ignore_pending_last_seen_message(&processed_chat_signature_state(signature));
        }
        let signature =
            resolve_packed_signature(&self.client_chat.signature_cache, &packet.message_signature);
        if signature.is_none() {
            self.counters.chat_unknown_packed_signatures += 1;
        }
        let resolved = signature.is_some();
        self.client_chat.deleted_messages.push(DeletedChatState {
            signature,
            cache_id: packet.message_signature.cache_id,
            resolved,
        });
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

    fn record_processed_player_chat_signature(
        &mut self,
        signature: &ProtocolMessageSignature,
        was_shown: bool,
    ) -> Option<ProtocolChatAcknowledgement> {
        let processed_signature = processed_chat_signature_state(signature);
        if self
            .client_chat
            .acknowledgement
            .last_processed_signature
            .as_ref()
            == Some(&processed_signature)
        {
            return None;
        }

        self.add_last_seen_tracked_message(processed_signature.clone(), was_shown);
        self.client_chat.acknowledgement.last_processed_signature = Some(processed_signature);
        self.client_chat.acknowledgement.pending_offset = self
            .client_chat
            .acknowledgement
            .pending_offset
            .saturating_add(1);

        if self.client_chat.acknowledgement.pending_offset > CHAT_ACKNOWLEDGEMENT_OFFSET_THRESHOLD {
            let offset = self.client_chat.acknowledgement.pending_offset;
            self.client_chat.acknowledgement.pending_offset = 0;
            self.counters.player_chat_acknowledgement_packets += 1;
            Some(ProtocolChatAcknowledgement { offset })
        } else {
            None
        }
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

fn plain_chat_styled_content(content: &str) -> Vec<StyledTextRun> {
    vec![StyledTextRun {
        text: content.to_string(),
        style: ComponentStyle::default(),
    }]
}

fn styled_chat_summary_text(runs: &[StyledTextRun]) -> String {
    runs.iter().map(|run| run.text.as_str()).collect()
}

fn resolve_packed_signature(
    cache: &[Option<ChatSignatureState>],
    packed: &PackedMessageSignature,
) -> Option<ChatSignatureState> {
    if let Some(signature) = &packed.full_signature {
        return Some(chat_signature_state(signature));
    }

    let id = usize::try_from(packed.cache_id?).ok()?;
    cache.get(id)?.clone()
}

fn push_signature_cache(
    cache: &mut [Option<ChatSignatureState>],
    last_seen: Vec<ChatSignatureState>,
    signature: Option<ChatSignatureState>,
) {
    let mut queue = last_seen;
    if let Some(signature) = signature {
        queue.push(signature);
    }

    for slot in cache.iter_mut() {
        let Some(next) = queue.pop() else {
            break;
        };
        let previous = slot.replace(next);
        if let Some(previous) = previous {
            if !queue.contains(&previous) {
                queue.insert(0, previous);
            }
        }
    }
}

fn chat_signature_state(signature: &ProtocolMessageSignature) -> ChatSignatureState {
    ChatSignatureState {
        checksum: signature.checksum(),
        bytes_len: signature.bytes.len(),
    }
}

fn processed_chat_signature_state(
    signature: &ProtocolMessageSignature,
) -> ProcessedChatSignatureState {
    ProcessedChatSignatureState {
        checksum: signature.checksum(),
        bytes: signature.bytes.clone(),
    }
}

fn last_seen_checksum_byte(checksum: i32) -> u8 {
    let checksum = checksum as u8;
    if checksum == 0 {
        1
    } else {
        checksum
    }
}

fn nonempty_styled_or_plain(styled: &[StyledTextRun], plain: &str) -> Vec<StyledTextRun> {
    if styled.is_empty() {
        plain_chat_styled_content(plain)
    } else {
        styled.to_vec()
    }
}

fn resolved_chat_state(
    resolved: Result<ResolvedChatType, crate::ChatTypeProjectionError>,
    bound: &bbb_protocol::packets::ChatTypeBound,
    content: &str,
    content_styled: &[StyledTextRun],
    content_component: Option<&NbtSummaryValue>,
) -> (
    ChatTypeState,
    Option<ChatDecorationState>,
    Option<ChatDecorationState>,
) {
    match resolved {
        Ok(resolved) => {
            let chat = chat_decoration_state(
                &resolved.chat,
                bound,
                content,
                content_styled,
                content_component,
            );
            let narration = chat_decoration_state(
                &resolved.narration,
                bound,
                content,
                content_styled,
                content_component,
            );
            let direct_translation_key = resolved
                .holder_id
                .is_none()
                .then(|| resolved.chat.translation_key.clone());
            (
                ChatTypeState {
                    registry_id: resolved.holder_id,
                    resource_id: resolved.resource_id,
                    source: resolved.source,
                    direct_translation_key,
                    resolution_error: None,
                },
                Some(chat),
                Some(narration),
            )
        }
        Err(error) => {
            let (registry_id, direct_translation_key) = match &bound.chat_type {
                ProtocolChatTypeHolder::Registry { id } => (Some(*id), None),
                ProtocolChatTypeHolder::Direct { chat, .. } => {
                    (None, Some(chat.translation_key.clone()))
                }
            };
            (
                ChatTypeState {
                    registry_id,
                    resource_id: None,
                    source: None,
                    direct_translation_key,
                    resolution_error: Some(error.to_string()),
                },
                None,
                None,
            )
        }
    }
}

fn chat_decoration_state(
    decoration: &ChatTypeDecoration,
    bound: &bbb_protocol::packets::ChatTypeBound,
    content: &str,
    content_styled: &[StyledTextRun],
    content_component: Option<&NbtSummaryValue>,
) -> ChatDecorationState {
    let arguments = decoration
        .parameters
        .iter()
        .map(|parameter| {
            let (summary, styled, component) = match parameter {
                ChatTypeDecorationParameter::Sender => (
                    bound.name.clone(),
                    nonempty_styled_or_plain(&bound.name_styled, &bound.name),
                    bound
                        .name_component
                        .clone()
                        .or_else(|| Some(NbtSummaryValue::String(bound.name.clone()))),
                ),
                ChatTypeDecorationParameter::Target => {
                    let summary = bound.target_name.clone().unwrap_or_default();
                    let styled = bound
                        .target_name_styled
                        .as_deref()
                        .map(|styled| nonempty_styled_or_plain(styled, &summary))
                        .unwrap_or_default();
                    let component = bound.target_name_component.clone().or_else(|| {
                        bound
                            .target_name
                            .as_ref()
                            .map(|target| NbtSummaryValue::String(target.clone()))
                    });
                    (summary, styled, component)
                }
                ChatTypeDecorationParameter::Content => (
                    content.to_string(),
                    content_styled.to_vec(),
                    content_component.cloned(),
                ),
            };
            ChatDecorationArgumentState {
                parameter: *parameter,
                summary,
                styled,
                component,
            }
        })
        .collect();
    ChatDecorationState {
        translation_key: decoration.translation_key.clone(),
        arguments,
        style: decoration.style.clone(),
    }
}

fn refresh_chat_counters(store: &mut WorldStore) {
    store.counters.chat_messages_tracked = store.client_chat.messages.len();
    store.counters.deleted_chat_messages_tracked = store.client_chat.deleted_messages.len();
    store.counters.chat_signature_cache_entries = store
        .client_chat
        .signature_cache
        .iter()
        .filter(|entry| entry.is_some())
        .count();
    store.counters.player_chat_acknowledgement_pending_offset =
        usize::try_from(store.client_chat.acknowledgement.pending_offset.max(0)).unwrap_or(0);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::CHAT_TYPE_REGISTRY;
    use bbb_protocol::packets::{
        ChatTypeBound, ChatTypeDecorationSummary, ChatTypeHolder, CommonPlayerSpawnInfo,
        FilterMask, FilterMaskKind, MessageSignature, PackedMessageSignature, PlayLogin,
        RegistryData, RegistryDataEntry, SignedMessageBody,
    };

    #[test]
    fn player_chat_tracks_message_and_signature_cache() {
        let mut store = WorldStore::new();
        let sender = Uuid::from_u128(1);

        let acknowledgement = store.apply_player_chat(PlayerChat {
            global_index: 0,
            sender,
            index: 5,
            signature: Some(signature(3)),
            body: SignedMessageBody {
                content: "hello".to_string(),
                timestamp_millis: 1,
                salt: 2,
                last_seen: Vec::new(),
            },
            unsigned_content: Some("unsigned".to_string()),
            unsigned_content_styled: Some(plain_chat_styled_content("unsigned")),
            unsigned_content_component: None,
            filter_mask: FilterMask {
                kind: FilterMaskKind::PartiallyFiltered,
                mask_words: vec![1],
            },
            chat_type: chat_type("Alice"),
        });

        assert_eq!(acknowledgement, None);
        let message = store.client_chat().messages.last().unwrap();
        assert_eq!(message.kind, ChatMessageKind::Player);
        assert_eq!(message.content, "hello");
        assert_eq!(
            message.styled_content,
            vec![StyledTextRun {
                text: "unsigned".to_string(),
                style: ComponentStyle::default(),
            }]
        );
        assert_eq!(
            message.content_translation_key.as_deref(),
            Some("chat.type.text")
        );
        assert_eq!(message.chat_type.registry_id, Some(0));
        assert_eq!(
            message.chat_type.resource_id.as_deref(),
            Some("minecraft:chat")
        );
        assert_eq!(
            message.chat_type.source,
            Some(RegistryEntrySource::VanillaFallback)
        );
        let decoration = message.decoration.as_ref().unwrap();
        assert_eq!(decoration.arguments.len(), 2);
        assert_eq!(decoration.arguments[0].summary, "Alice");
        assert_eq!(decoration.arguments[1].summary, "unsigned");
        assert_eq!(message.sender, Some(sender));
        assert_eq!(message.sender_name, "Alice");
        assert_eq!(message.validation_state, ChatValidationState::Unchecked);
        assert!(message.signature.is_some());
        assert_eq!(store.counters().player_chat_packets, 1);
        assert_eq!(store.counters().player_chat_unsigned_content_packets, 1);
        assert_eq!(store.counters().player_chat_filtered_packets, 1);
        assert_eq!(store.counters().chat_signature_cache_entries, 1);
        assert_eq!(
            store.counters().player_chat_acknowledgement_pending_offset,
            1
        );
    }

    #[test]
    fn player_chat_generates_acknowledgement_after_vanilla_offset_threshold() {
        let mut store = WorldStore::new();
        let mut acknowledgement = None;

        for index in 0..65 {
            acknowledgement =
                store.apply_player_chat(player_chat_with_signature(index, signature(index as u8)));
        }

        assert_eq!(
            acknowledgement,
            Some(ProtocolChatAcknowledgement { offset: 65 })
        );
        assert_eq!(store.counters().player_chat_packets, 65);
        assert_eq!(store.counters().player_chat_acknowledgement_packets, 1);
        assert_eq!(
            store.counters().player_chat_acknowledgement_pending_offset,
            0
        );
        assert_eq!(store.client_chat().acknowledgement.pending_offset, 0);
    }

    #[test]
    fn player_chat_acknowledgement_ignores_duplicate_last_signature() {
        let mut store = WorldStore::new();
        let duplicate = signature(7);

        assert_eq!(
            store.apply_player_chat(player_chat_with_signature(0, duplicate.clone())),
            None
        );
        assert_eq!(
            store.apply_player_chat(player_chat_with_signature(1, duplicate.clone())),
            None
        );

        assert_eq!(store.counters().player_chat_packets, 2);
        assert_eq!(
            store.counters().player_chat_acknowledgement_pending_offset,
            1
        );
        assert_eq!(store.client_chat().acknowledgement.pending_offset, 1);

        let outbound = store.take_last_seen_messages_for_outbound_chat();
        assert_eq!(outbound.update.offset, 1);
        assert_eq!(
            outbound.update.acknowledged,
            1 << (LAST_SEEN_MESSAGES_TRACKER_CAPACITY - 1)
        );
        assert_eq!(outbound.signatures, vec![duplicate]);
    }

    #[test]
    fn take_pending_player_chat_acknowledgement_drains_below_threshold_offset() {
        let mut store = WorldStore::new();
        assert_eq!(
            store.apply_player_chat(player_chat_with_signature(0, signature(11))),
            None
        );

        assert_eq!(
            store.take_pending_player_chat_acknowledgement(),
            Some(ProtocolChatAcknowledgement { offset: 1 })
        );
        assert_eq!(
            store.counters().player_chat_acknowledgement_pending_offset,
            0
        );
        assert_eq!(store.counters().player_chat_acknowledgement_packets, 1);
        assert_eq!(store.take_pending_player_chat_acknowledgement(), None);
        assert_eq!(store.counters().player_chat_acknowledgement_packets, 1);
    }

    #[test]
    fn outbound_chat_last_seen_update_uses_tail_order_before_ring_is_full() {
        let mut store = WorldStore::new();
        let seen = signature(12);
        assert_eq!(
            store.apply_player_chat(player_chat_with_signature(0, seen.clone())),
            None
        );

        let outbound = store.take_last_seen_messages_for_outbound_chat();

        assert_eq!(outbound.update.offset, 1);
        assert_eq!(
            outbound.update.acknowledged,
            1 << (LAST_SEEN_MESSAGES_TRACKER_CAPACITY - 1)
        );
        assert_eq!(
            outbound.update.checksum,
            last_seen_checksum_byte(1_i32.wrapping_mul(31).wrapping_add(seen.checksum()))
        );
        assert_eq!(outbound.signatures, vec![seen]);
        assert_eq!(
            store.counters().player_chat_acknowledgement_pending_offset,
            0
        );

        let second = store.take_last_seen_messages_for_outbound_chat();
        assert_eq!(second.update.offset, 0);
        assert_eq!(
            second.update.acknowledged,
            1 << (LAST_SEEN_MESSAGES_TRACKER_CAPACITY - 1)
        );
        assert_eq!(second.update.checksum, outbound.update.checksum);
        assert_eq!(second.signatures, outbound.signatures);
    }

    #[test]
    fn outbound_chat_last_seen_update_offsets_fully_filtered_messages_without_ack_bit() {
        let mut store = WorldStore::new();
        assert_eq!(
            store.apply_player_chat(player_chat_with_signature_and_filter(
                0,
                signature(13),
                FilterMaskKind::FullyFiltered,
            )),
            None
        );

        let outbound = store.take_last_seen_messages_for_outbound_chat();

        assert_eq!(outbound.update.offset, 1);
        assert_eq!(outbound.update.acknowledged, 0);
        assert_eq!(
            outbound.update.checksum,
            LastSeenMessagesUpdate::default().checksum
        );
        assert!(outbound.signatures.is_empty());
        assert_eq!(
            store.counters().player_chat_acknowledgement_pending_offset,
            0
        );
    }

    #[test]
    fn delete_chat_ignores_pending_full_signature_for_last_seen_updates() {
        let mut store = WorldStore::new();
        let deleted = signature(14);
        assert_eq!(
            store.apply_player_chat(player_chat_with_signature(0, deleted.clone())),
            None
        );

        store.apply_delete_chat(ProtocolDeleteChat {
            message_signature: PackedMessageSignature {
                cache_id: None,
                full_signature: Some(deleted),
            },
        });
        let outbound = store.take_last_seen_messages_for_outbound_chat();

        assert_eq!(outbound.update.offset, 1);
        assert_eq!(outbound.update.acknowledged, 0);
        assert_eq!(
            outbound.update.checksum,
            LastSeenMessagesUpdate::default().checksum
        );
        assert!(outbound.signatures.is_empty());
    }

    #[test]
    fn outbound_chat_last_seen_update_uses_vanilla_ring_order() {
        let mut store = WorldStore::new();
        let mut expected_checksum = 1_i32;

        for index in 0..21 {
            let seen = signature(index as u8);
            if index > 0 {
                expected_checksum = expected_checksum
                    .wrapping_mul(31)
                    .wrapping_add(seen.checksum());
            }
            assert_eq!(
                store.apply_player_chat(player_chat_with_signature(index, seen)),
                None
            );
        }

        let outbound = store.take_last_seen_messages_for_outbound_chat();

        assert_eq!(outbound.update.offset, 21);
        assert_eq!(
            outbound.update.acknowledged,
            (1 << LAST_SEEN_MESSAGES_TRACKER_CAPACITY) - 1
        );
        assert_eq!(
            outbound.update.checksum,
            last_seen_checksum_byte(expected_checksum)
        );
        assert_eq!(
            outbound.signatures.len(),
            LAST_SEEN_MESSAGES_TRACKER_CAPACITY
        );
        assert_eq!(outbound.signatures[0], signature(1));
        assert_eq!(outbound.signatures[19], signature(20));
    }

    #[test]
    fn last_seen_checksum_byte_never_returns_zero() {
        assert_eq!(last_seen_checksum_byte(0), 1);
        assert_eq!(last_seen_checksum_byte(256), 1);
    }

    #[test]
    fn delete_chat_resolves_cached_signature() {
        let mut store = WorldStore::new();
        let _ = store.apply_player_chat(PlayerChat {
            global_index: 0,
            sender: Uuid::from_u128(1),
            index: 0,
            signature: Some(signature(9)),
            body: SignedMessageBody {
                content: "hello".to_string(),
                timestamp_millis: 1,
                salt: 2,
                last_seen: Vec::new(),
            },
            unsigned_content: None,
            unsigned_content_styled: None,
            unsigned_content_component: None,
            filter_mask: FilterMask {
                kind: FilterMaskKind::PassThrough,
                mask_words: Vec::new(),
            },
            chat_type: chat_type("Alice"),
        });

        store.apply_delete_chat(ProtocolDeleteChat {
            message_signature: PackedMessageSignature {
                cache_id: Some(0),
                full_signature: None,
            },
        });

        let deleted = store.client_chat().deleted_messages.last().unwrap();
        assert!(deleted.resolved);
        assert_eq!(deleted.cache_id, Some(0));
        assert_eq!(store.counters().delete_chat_packets, 1);
        assert_eq!(store.counters().deleted_chat_messages_tracked, 1);
    }

    #[test]
    fn player_chat_out_of_order_is_counted() {
        let mut store = WorldStore::new();
        let _ = store.apply_player_chat(PlayerChat {
            global_index: 3,
            sender: Uuid::from_u128(1),
            index: 0,
            signature: None,
            body: SignedMessageBody {
                content: "late".to_string(),
                timestamp_millis: 1,
                salt: 2,
                last_seen: Vec::new(),
            },
            unsigned_content: None,
            unsigned_content_styled: None,
            unsigned_content_component: None,
            filter_mask: FilterMask {
                kind: FilterMaskKind::PassThrough,
                mask_words: Vec::new(),
            },
            chat_type: chat_type("Alice"),
        });

        assert_eq!(store.counters().player_chat_index_mismatches, 1);
        assert_eq!(
            store
                .client_chat()
                .messages
                .last()
                .unwrap()
                .validation_state,
            ChatValidationState::OutOfOrder
        );
    }

    #[test]
    fn disguised_chat_tracks_system_like_message() {
        let mut store = WorldStore::new();
        store.apply_disguised_chat(ProtocolDisguisedChat {
            message: "notice".to_string(),
            message_styled: plain_chat_styled_content("notice"),
            message_component: None,
            chat_type: chat_type("Server"),
        });

        let message = store.client_chat().messages.last().unwrap();
        assert_eq!(message.kind, ChatMessageKind::Disguised);
        assert_eq!(message.content, "notice");
        assert_eq!(
            message.styled_content,
            vec![StyledTextRun {
                text: "notice".to_string(),
                style: ComponentStyle::default(),
            }]
        );
        assert_eq!(message.sender_name, "Server");
        assert_eq!(store.counters().disguised_chat_packets, 1);
    }

    #[test]
    fn synchronized_chat_type_uses_server_order_and_typed_decoration() {
        let mut store = WorldStore::new();
        store.record_registry_data(RegistryData {
            registry: CHAT_TYPE_REGISTRY.to_string(),
            entries: vec![
                RegistryDataEntry {
                    id: "example:team_notice".to_string(),
                    raw_data: Some(chat_type_payload(
                        "example.chat.team_notice",
                        &["target", "content"],
                        "example.chat.team_notice.narrate",
                        &["sender", "content"],
                        true,
                    )),
                },
                RegistryDataEntry {
                    id: "minecraft:chat".to_string(),
                    raw_data: Some(chat_type_payload(
                        "chat.type.text",
                        &["sender", "content"],
                        "chat.type.text.narrate",
                        &["sender", "content"],
                        false,
                    )),
                },
            ],
            raw_payload_len: 128,
        });
        let mut bound = chat_type("Server");
        bound.target_name = Some("Blue".to_string());
        bound.target_name_styled = Some(plain_chat_styled_content("Blue"));
        bound.target_name_component = Some(NbtSummaryValue::String("Blue".to_string()));
        store.apply_disguised_chat(ProtocolDisguisedChat {
            message: "custom notice".to_string(),
            message_styled: plain_chat_styled_content("custom notice"),
            message_component: Some(NbtSummaryValue::String("custom notice".to_string())),
            chat_type: bound,
        });
        let mut vanilla_bound = chat_type("Alex");
        vanilla_bound.chat_type = ChatTypeHolder::Registry { id: 1 };
        store.apply_disguised_chat(ProtocolDisguisedChat {
            message: "vanilla moved".to_string(),
            message_styled: plain_chat_styled_content("vanilla moved"),
            message_component: Some(NbtSummaryValue::String("vanilla moved".to_string())),
            chat_type: vanilla_bound,
        });

        let message = &store.client_chat().messages[0];
        assert_eq!(message.chat_type.registry_id, Some(0));
        assert_eq!(
            message.chat_type.resource_id.as_deref(),
            Some("example:team_notice")
        );
        assert_eq!(message.chat_type.source, Some(RegistryEntrySource::Server));
        assert_eq!(message.chat_type.resolution_error, None);
        let decoration = message.decoration.as_ref().unwrap();
        assert_eq!(decoration.translation_key, "example.chat.team_notice");
        assert_eq!(decoration.style.bold, Some(true));
        assert_eq!(
            decoration
                .arguments
                .iter()
                .map(|argument| (argument.parameter, argument.summary.as_str()))
                .collect::<Vec<_>>(),
            vec![
                (ChatTypeDecorationParameter::Target, "Blue"),
                (ChatTypeDecorationParameter::Content, "custom notice"),
            ]
        );
        let vanilla = &store.client_chat().messages[1];
        assert_eq!(vanilla.chat_type.registry_id, Some(1));
        assert_eq!(
            vanilla.chat_type.resource_id.as_deref(),
            Some("minecraft:chat")
        );
        assert_eq!(
            vanilla.content_translation_key.as_deref(),
            Some("chat.type.text")
        );
    }

    #[test]
    fn direct_chat_type_preserves_style_and_vanilla_zero_parameter_fallback() {
        let mut store = WorldStore::new();
        store.apply_disguised_chat(ProtocolDisguisedChat {
            message: "direct notice".to_string(),
            message_styled: plain_chat_styled_content("direct notice"),
            message_component: Some(NbtSummaryValue::String("direct notice".to_string())),
            chat_type: ChatTypeBound {
                chat_type: ChatTypeHolder::Direct {
                    chat: ChatTypeDecorationSummary {
                        translation_key: "example.direct".to_string(),
                        parameters: vec![99, 2],
                        style: ComponentStyle {
                            italic: Some(true),
                            ..ComponentStyle::default()
                        },
                    },
                    narration: ChatTypeDecorationSummary {
                        translation_key: "example.direct.narrate".to_string(),
                        parameters: vec![0, 2],
                        style: ComponentStyle::default(),
                    },
                },
                name: "Server".to_string(),
                name_styled: plain_chat_styled_content("Server"),
                name_component: Some(NbtSummaryValue::String("Server".to_string())),
                target_name: None,
                target_name_styled: None,
                target_name_component: None,
            },
        });

        let message = store.client_chat().messages.last().unwrap();
        assert_eq!(message.chat_type.registry_id, None);
        assert_eq!(message.chat_type.source, None);
        assert_eq!(
            message.chat_type.direct_translation_key.as_deref(),
            Some("example.direct")
        );
        let decoration = message.decoration.as_ref().unwrap();
        assert_eq!(decoration.style.italic, Some(true));
        assert_eq!(
            decoration.arguments[0].parameter,
            ChatTypeDecorationParameter::Sender
        );
        assert_eq!(
            decoration.arguments[1].parameter,
            ChatTypeDecorationParameter::Content
        );
    }

    #[test]
    fn synchronized_chat_type_stub_malformed_and_unknown_stay_diagnostic() {
        let mut store = WorldStore::new();
        store.record_registry_data(RegistryData {
            registry: CHAT_TYPE_REGISTRY.to_string(),
            entries: vec![
                RegistryDataEntry {
                    id: "example:stub".to_string(),
                    raw_data: None,
                },
                RegistryDataEntry {
                    id: "example:broken".to_string(),
                    raw_data: Some(vec![10, 0]),
                },
            ],
            raw_payload_len: 16,
        });

        for id in 0..3 {
            let mut bound = chat_type("Server");
            bound.chat_type = ChatTypeHolder::Registry { id };
            store.apply_disguised_chat(ProtocolDisguisedChat {
                message: format!("notice {id}"),
                message_styled: Vec::new(),
                message_component: None,
                chat_type: bound,
            });
        }

        let messages = &store.client_chat().messages;
        assert!(messages[0]
            .chat_type
            .resolution_error
            .as_deref()
            .unwrap()
            .contains("missing example:stub chat type payload"));
        assert!(messages[1]
            .chat_type
            .resolution_error
            .as_deref()
            .unwrap()
            .contains("malformed example:broken chat type payload"));
        assert!(messages[2]
            .chat_type
            .resolution_error
            .as_deref()
            .unwrap()
            .contains("missing minecraft:chat_type holder 2"));
        assert!(messages.iter().all(|message| message.decoration.is_none()));
    }

    #[test]
    fn client_system_chat_tracks_local_display_message_without_protocol_counters() {
        let mut store = WorldStore::new();

        store.push_client_system_chat_message("[Debug]: Client version info:");

        let message = store.client_chat().messages.last().unwrap();
        assert_eq!(message.kind, ChatMessageKind::ClientSystem);
        assert_eq!(message.content, "[Debug]: Client version info:");
        assert_eq!(
            message.styled_content,
            vec![StyledTextRun {
                text: "[Debug]: Client version info:".to_string(),
                style: ComponentStyle::default(),
            }]
        );
        assert_eq!(message.sender, None);
        assert!(message.sender_name.is_empty());
        assert_eq!(message.chat_type.registry_id, None);
        assert_eq!(message.chat_type.direct_translation_key, None);
        assert_eq!(message.validation_state, ChatValidationState::Unsigned);
        assert_eq!(store.counters().chat_messages_tracked, 1);
        assert_eq!(store.counters().player_chat_packets, 0);
        assert_eq!(store.counters().disguised_chat_packets, 0);
        assert_eq!(store.counters().system_chat_packets, 0);
    }

    #[test]
    fn reset_chat_clears_messages_and_signature_state() {
        let mut store = WorldStore::new();
        let _ = store.apply_player_chat(PlayerChat {
            global_index: 0,
            sender: Uuid::from_u128(1),
            index: 0,
            signature: Some(signature(9)),
            body: SignedMessageBody {
                content: "hello".to_string(),
                timestamp_millis: 1,
                salt: 2,
                last_seen: Vec::new(),
            },
            unsigned_content: None,
            unsigned_content_styled: None,
            unsigned_content_component: None,
            filter_mask: FilterMask {
                kind: FilterMaskKind::PassThrough,
                mask_words: Vec::new(),
            },
            chat_type: chat_type("Alice"),
        });
        store.apply_delete_chat(ProtocolDeleteChat {
            message_signature: PackedMessageSignature {
                cache_id: Some(0),
                full_signature: None,
            },
        });

        store.apply_reset_chat();

        assert!(store.client_chat().messages.is_empty());
        assert!(store.client_chat().deleted_messages.is_empty());
        assert_eq!(store.client_chat().expected_player_chat_global_index, 0);
        assert!(store
            .client_chat()
            .signature_cache
            .iter()
            .all(Option::is_none));
        assert!(store
            .client_chat()
            .last_seen_messages
            .entries
            .iter()
            .all(Option::is_none));
        assert_eq!(store.client_chat().acknowledgement.pending_offset, 0);
        assert!(store
            .client_chat()
            .acknowledgement
            .last_processed_signature
            .is_none());
        assert_eq!(store.counters().reset_chat_packets, 1);
        assert_eq!(store.counters().player_chat_packets, 1);
        assert_eq!(store.counters().delete_chat_packets, 1);
        assert_eq!(store.counters().chat_messages_tracked, 0);
        assert_eq!(store.counters().deleted_chat_messages_tracked, 0);
        assert_eq!(store.counters().chat_signature_cache_entries, 0);
    }

    #[test]
    fn clear_client_chat_display_messages_preserves_signature_state() {
        let mut store = WorldStore::new();
        let _ = store.apply_player_chat(PlayerChat {
            global_index: 0,
            sender: Uuid::from_u128(1),
            index: 0,
            signature: Some(signature(9)),
            body: SignedMessageBody {
                content: "hello".to_string(),
                timestamp_millis: 1,
                salt: 2,
                last_seen: Vec::new(),
            },
            unsigned_content: None,
            unsigned_content_styled: None,
            unsigned_content_component: None,
            filter_mask: FilterMask {
                kind: FilterMaskKind::PassThrough,
                mask_words: Vec::new(),
            },
            chat_type: chat_type("Alice"),
        });
        store.apply_delete_chat(ProtocolDeleteChat {
            message_signature: PackedMessageSignature {
                cache_id: Some(0),
                full_signature: None,
            },
        });
        assert_eq!(store.client_chat().messages.len(), 1);
        assert_eq!(store.client_chat().deleted_messages.len(), 1);
        assert!(store
            .client_chat()
            .signature_cache
            .iter()
            .any(Option::is_some));
        assert!(store
            .client_chat()
            .last_seen_messages
            .entries
            .iter()
            .any(Option::is_some));
        assert_eq!(store.client_chat().acknowledgement.pending_offset, 1);

        store.clear_client_chat_display_messages();

        assert!(store.client_chat().messages.is_empty());
        assert!(store.client_chat().deleted_messages.is_empty());
        assert_eq!(store.client_chat().expected_player_chat_global_index, 1);
        assert!(store
            .client_chat()
            .signature_cache
            .iter()
            .any(Option::is_some));
        assert!(store
            .client_chat()
            .last_seen_messages
            .entries
            .iter()
            .any(Option::is_some));
        assert_eq!(store.client_chat().acknowledgement.pending_offset, 1);
        assert_eq!(store.counters().reset_chat_packets, 0);
        assert_eq!(store.counters().player_chat_packets, 1);
        assert_eq!(store.counters().delete_chat_packets, 1);
        assert_eq!(store.counters().chat_messages_tracked, 0);
        assert_eq!(store.counters().deleted_chat_messages_tracked, 0);
        assert_eq!(store.counters().chat_signature_cache_entries, 1);
        assert_eq!(
            store.counters().player_chat_acknowledgement_pending_offset,
            1
        );
    }

    #[test]
    fn login_resets_chat_session_trackers_and_preserves_display_state() {
        let mut store = WorldStore::new();
        let _ = store.apply_player_chat(player_chat_with_signature(0, signature(9)));
        store.apply_delete_chat(ProtocolDeleteChat {
            message_signature: PackedMessageSignature {
                cache_id: Some(0),
                full_signature: None,
            },
        });
        assert_eq!(store.client_chat().messages.len(), 1);
        assert_eq!(store.client_chat().deleted_messages.len(), 1);
        assert_eq!(store.client_chat().expected_player_chat_global_index, 1);
        assert_eq!(store.client_chat().acknowledgement.pending_offset, 1);

        store.apply_login(&login_packet());

        assert_eq!(store.client_chat().messages.len(), 1);
        assert_eq!(store.client_chat().deleted_messages.len(), 1);
        assert_eq!(store.client_chat().expected_player_chat_global_index, 0);
        assert!(store
            .client_chat()
            .signature_cache
            .iter()
            .all(Option::is_none));
        assert!(store
            .client_chat()
            .last_seen_messages
            .entries
            .iter()
            .all(Option::is_none));
        assert_eq!(store.client_chat().acknowledgement.pending_offset, 0);
        assert!(store
            .client_chat()
            .acknowledgement
            .last_processed_signature
            .is_none());
        assert_eq!(store.counters().reset_chat_packets, 0);
        assert_eq!(store.counters().player_chat_packets, 1);
        assert_eq!(store.counters().delete_chat_packets, 1);
        assert_eq!(store.counters().chat_messages_tracked, 1);
        assert_eq!(store.counters().deleted_chat_messages_tracked, 1);
        assert_eq!(store.counters().chat_signature_cache_entries, 0);
        assert_eq!(
            store.counters().player_chat_acknowledgement_pending_offset,
            0
        );
    }

    fn signature(byte: u8) -> MessageSignature {
        MessageSignature {
            bytes: vec![byte; 256],
        }
    }

    fn player_chat_with_signature(global_index: i32, signature: MessageSignature) -> PlayerChat {
        player_chat_with_signature_and_filter(global_index, signature, FilterMaskKind::PassThrough)
    }

    fn player_chat_with_signature_and_filter(
        global_index: i32,
        signature: MessageSignature,
        filter_kind: FilterMaskKind,
    ) -> PlayerChat {
        PlayerChat {
            global_index,
            sender: Uuid::from_u128(1),
            index: global_index,
            signature: Some(signature),
            body: SignedMessageBody {
                content: format!("message {global_index}"),
                timestamp_millis: i64::from(global_index),
                salt: i64::from(global_index) + 1,
                last_seen: Vec::new(),
            },
            unsigned_content: None,
            unsigned_content_styled: None,
            unsigned_content_component: None,
            filter_mask: FilterMask {
                kind: filter_kind,
                mask_words: Vec::new(),
            },
            chat_type: chat_type("Alice"),
        }
    }

    fn chat_type(name: &str) -> ChatTypeBound {
        ChatTypeBound {
            chat_type: ChatTypeHolder::Registry { id: 0 },
            name: name.to_string(),
            name_styled: plain_chat_styled_content(name),
            name_component: None,
            target_name: None,
            target_name_styled: None,
            target_name_component: None,
        }
    }

    fn chat_type_payload(
        chat_key: &str,
        chat_parameters: &[&str],
        narration_key: &str,
        narration_parameters: &[&str],
        bold: bool,
    ) -> Vec<u8> {
        let mut out = vec![10];
        write_chat_type_decoration_nbt(&mut out, "chat", chat_key, chat_parameters, bold);
        write_chat_type_decoration_nbt(
            &mut out,
            "narration",
            narration_key,
            narration_parameters,
            false,
        );
        out.push(0);
        out
    }

    fn write_chat_type_decoration_nbt(
        out: &mut Vec<u8>,
        name: &str,
        translation_key: &str,
        parameters: &[&str],
        bold: bool,
    ) {
        out.push(10);
        write_nbt_string(out, name);
        out.push(8);
        write_nbt_string(out, "translation_key");
        write_nbt_string(out, translation_key);
        out.push(9);
        write_nbt_string(out, "parameters");
        out.push(8);
        out.extend_from_slice(&(parameters.len() as i32).to_be_bytes());
        for parameter in parameters {
            write_nbt_string(out, parameter);
        }
        if bold {
            out.push(10);
            write_nbt_string(out, "style");
            out.push(1);
            write_nbt_string(out, "bold");
            out.push(1);
            out.push(0);
        }
        out.push(0);
    }

    fn write_nbt_string(out: &mut Vec<u8>, value: &str) {
        let bytes = value.as_bytes();
        out.extend_from_slice(&(bytes.len() as u16).to_be_bytes());
        out.extend_from_slice(bytes);
    }

    fn login_packet() -> PlayLogin {
        PlayLogin {
            player_id: 42,
            hardcore: false,
            levels: vec!["minecraft:overworld".to_string()],
            max_players: 20,
            chunk_radius: 8,
            simulation_distance: 6,
            reduced_debug_info: false,
            show_death_screen: true,
            do_limited_crafting: false,
            common_spawn_info: CommonPlayerSpawnInfo {
                dimension_type_id: 0,
                dimension: "minecraft:overworld".to_string(),
                seed: 12345,
                game_type: 0,
                previous_game_type: -1,
                is_debug: false,
                is_flat: false,
                last_death_location: None,
                portal_cooldown: 0,
                sea_level: 63,
            },
            enforces_secure_chat: true,
        }
    }
}
