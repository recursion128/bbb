use bbb_protocol::{
    packets::{
        ChatAcknowledgement as ProtocolChatAcknowledgement,
        ChatTypeHolder as ProtocolChatTypeHolder, DeleteChat as ProtocolDeleteChat,
        DisguisedChat as ProtocolDisguisedChat, FilterMaskKind as ProtocolFilterMaskKind,
        LastSeenMessagesUpdate, MessageSignature as ProtocolMessageSignature,
        PackedMessageSignature, PlayerChat,
    },
    ComponentStyle, StyledTextRun,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::WorldStore;

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
    pub styled_content: Vec<StyledTextRun>,
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
    ClientSystem,
}

impl ChatMessageKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Player => "player",
            Self::Disguised => "disguised",
            Self::ClientSystem => "client_system",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChatTypeState {
    pub registry_id: Option<i32>,
    pub direct_translation_key: Option<String>,
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

    pub fn push_client_system_chat_message(&mut self, content: impl Into<String>) {
        let content = content.into();
        let styled_content = plain_chat_styled_content(&content);
        self.push_client_system_chat_message_state(content, styled_content);
    }

    pub fn push_styled_client_system_chat_message(&mut self, styled_content: Vec<StyledTextRun>) {
        let content = styled_chat_summary_text(&styled_content);
        self.push_client_system_chat_message_state(content, styled_content);
    }

    fn push_client_system_chat_message_state(
        &mut self,
        content: String,
        styled_content: Vec<StyledTextRun>,
    ) {
        self.client_chat.messages.push(ChatMessageState {
            kind: ChatMessageKind::ClientSystem,
            content,
            styled_content,
            sender: None,
            sender_name: String::new(),
            target_name: None,
            global_index: None,
            message_index: None,
            chat_type: ChatTypeState {
                registry_id: None,
                direct_translation_key: None,
            },
            signature: None,
            unsigned_content: None,
            filter_mask: ProtocolFilterMaskKind::PassThrough.as_str().to_string(),
            validation_state: ChatValidationState::Unsigned,
        });
        refresh_chat_counters(self);
    }

    pub fn apply_player_chat(&mut self, packet: PlayerChat) -> Option<ProtocolChatAcknowledgement> {
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

        let message = ChatMessageState {
            kind: ChatMessageKind::Player,
            styled_content: plain_chat_styled_content(&packet.body.content),
            content: packet.body.content,
            sender: Some(packet.sender),
            sender_name: packet.chat_type.name,
            target_name: packet.chat_type.target_name,
            global_index: Some(packet.global_index),
            message_index: Some(packet.index),
            chat_type: chat_type_state(packet.chat_type.chat_type),
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
        self.counters.disguised_chat_packets += 1;
        self.client_chat.messages.push(ChatMessageState {
            kind: ChatMessageKind::Disguised,
            styled_content: plain_chat_styled_content(&packet.message),
            content: packet.message,
            sender: None,
            sender_name: packet.chat_type.name,
            target_name: packet.chat_type.target_name,
            global_index: None,
            message_index: None,
            chat_type: chat_type_state(packet.chat_type.chat_type),
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

    pub fn take_last_seen_messages_update_for_outbound_chat(&mut self) -> LastSeenMessagesUpdate {
        let offset = self.client_chat.acknowledgement.pending_offset;
        self.client_chat.acknowledgement.pending_offset = 0;

        let mut acknowledged = 0_u32;
        let mut checksum = 1_i32;
        let len = self.client_chat.last_seen_messages.entries.len();
        if len == 0 {
            refresh_chat_counters(self);
            return LastSeenMessagesUpdate {
                offset,
                ..LastSeenMessagesUpdate::default()
            };
        }
        for i in 0..len {
            let index = (self.client_chat.last_seen_messages.tail + i) % len;
            if let Some(entry) = self.client_chat.last_seen_messages.entries[index].as_mut() {
                acknowledged |= 1 << i;
                checksum = checksum
                    .wrapping_mul(31)
                    .wrapping_add(entry.signature.checksum);
                entry.pending = false;
            }
        }

        refresh_chat_counters(self);
        LastSeenMessagesUpdate {
            offset,
            acknowledged,
            checksum: last_seen_checksum_byte(checksum),
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

fn chat_type_state(chat_type: ProtocolChatTypeHolder) -> ChatTypeState {
    match chat_type {
        ProtocolChatTypeHolder::Registry { id } => ChatTypeState {
            registry_id: Some(id),
            direct_translation_key: None,
        },
        ProtocolChatTypeHolder::Direct { chat, .. } => ChatTypeState {
            registry_id: None,
            direct_translation_key: Some(chat.translation_key),
        },
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
    use bbb_protocol::packets::{
        ChatTypeBound, ChatTypeHolder, FilterMask, FilterMaskKind, MessageSignature,
        PackedMessageSignature, SignedMessageBody,
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
                text: "hello".to_string(),
                style: ComponentStyle::default(),
            }]
        );
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
            store.apply_player_chat(player_chat_with_signature(1, duplicate)),
            None
        );

        assert_eq!(store.counters().player_chat_packets, 2);
        assert_eq!(
            store.counters().player_chat_acknowledgement_pending_offset,
            1
        );
        assert_eq!(store.client_chat().acknowledgement.pending_offset, 1);

        let update = store.take_last_seen_messages_update_for_outbound_chat();
        assert_eq!(update.offset, 1);
        assert_eq!(
            update.acknowledged,
            1 << (LAST_SEEN_MESSAGES_TRACKER_CAPACITY - 1)
        );
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

        let update = store.take_last_seen_messages_update_for_outbound_chat();

        assert_eq!(update.offset, 1);
        assert_eq!(
            update.acknowledged,
            1 << (LAST_SEEN_MESSAGES_TRACKER_CAPACITY - 1)
        );
        assert_eq!(
            update.checksum,
            last_seen_checksum_byte(1_i32.wrapping_mul(31).wrapping_add(seen.checksum()))
        );
        assert_eq!(
            store.counters().player_chat_acknowledgement_pending_offset,
            0
        );

        let second = store.take_last_seen_messages_update_for_outbound_chat();
        assert_eq!(second.offset, 0);
        assert_eq!(
            second.acknowledged,
            1 << (LAST_SEEN_MESSAGES_TRACKER_CAPACITY - 1)
        );
        assert_eq!(second.checksum, update.checksum);
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

        let update = store.take_last_seen_messages_update_for_outbound_chat();

        assert_eq!(update.offset, 1);
        assert_eq!(update.acknowledged, 0);
        assert_eq!(update.checksum, LastSeenMessagesUpdate::default().checksum);
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
        let update = store.take_last_seen_messages_update_for_outbound_chat();

        assert_eq!(update.offset, 1);
        assert_eq!(update.acknowledged, 0);
        assert_eq!(update.checksum, LastSeenMessagesUpdate::default().checksum);
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

        let update = store.take_last_seen_messages_update_for_outbound_chat();

        assert_eq!(update.offset, 21);
        assert_eq!(
            update.acknowledged,
            (1 << LAST_SEEN_MESSAGES_TRACKER_CAPACITY) - 1
        );
        assert_eq!(update.checksum, last_seen_checksum_byte(expected_checksum));
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
            target_name: None,
        }
    }
}
