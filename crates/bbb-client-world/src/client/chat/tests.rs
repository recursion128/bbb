use std::sync::OnceLock;

use super::*;
use crate::CHAT_TYPE_REGISTRY;
use bbb_protocol::packets::{
    ChatTypeBound, ChatTypeDecorationSummary, ChatTypeHolder, CommonPlayerSpawnInfo, FilterMask,
    FilterMaskKind, GameProfile, GameType, MessageSignature, PackedMessageSignature, PlayLogin,
    PlayerInfoAction, PlayerInfoEntry, PlayerInfoRemove, PlayerInfoUpdate, RegistryData,
    RegistryDataEntry, SignedMessageBody,
};
use rsa::{pkcs8::EncodePublicKey, RsaPrivateKey, RsaPublicKey};

mod messages;

#[test]
fn player_chat_tracks_message_and_signature_cache() {
    let mut store = WorldStore::new();
    let sender = Uuid::from_u128(1);
    add_player_without_session(&mut store, sender);

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
        vec![
            StyledTextRun {
                text: "#".to_string(),
                style: ComponentStyle {
                    color: Some(0x555555),
                    ..ComponentStyle::default()
                },
            },
            StyledTextRun {
                text: "ello".to_string(),
                style: ComponentStyle::default(),
            },
        ]
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
    assert_eq!(decoration.arguments[1].summary, "#ello");
    assert_eq!(message.sender, Some(sender));
    assert_eq!(message.sender_name, "Alice");
    assert_eq!(message.validation_state, ChatValidationState::Unsigned);
    assert!(message.signature.is_none());
    assert_eq!(store.counters().player_chat_packets, 1);
    assert_eq!(store.counters().player_chat_unsigned_content_packets, 1);
    assert_eq!(store.counters().player_chat_filtered_packets, 1);
    assert_eq!(store.counters().chat_signature_cache_entries, 1);
    assert_eq!(
        store.counters().player_chat_acknowledgement_pending_offset,
        0
    );
}

#[test]
fn player_chat_generates_acknowledgement_after_vanilla_offset_threshold() {
    let mut store = WorldStore::new();
    let mut acknowledgement = None;

    for index in 0..65 {
        acknowledgement =
            store.record_processed_player_chat_signature(&signature(index as u8), true);
    }

    assert_eq!(
        acknowledgement,
        Some(ProtocolChatAcknowledgement { offset: 65 })
    );
    assert_eq!(store.counters().player_chat_packets, 0);
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
        store.record_processed_player_chat_signature(&duplicate, true),
        None
    );
    assert_eq!(
        store.record_processed_player_chat_signature(&duplicate, true),
        None
    );

    assert_eq!(store.counters().player_chat_packets, 0);
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
fn outbound_chat_last_seen_update_offsets_fully_filtered_messages_without_ack_bit() {
    let mut store = WorldStore::new();
    assert_eq!(
        store.record_processed_player_chat_signature(&signature(13), false),
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
        store.record_processed_player_chat_signature(&deleted, true),
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
    store.begin_registry_configuration();
    store.finish_registry_configuration().unwrap();
    let mut bound = chat_type("Server");
    bound.target_name = Some("Blue".to_string());
    bound.target_name_styled = Some(plain_chat_styled_content("Blue"));
    bound.target_name_component = Some(NbtSummaryValue::string("Blue"));
    store.apply_disguised_chat(ProtocolDisguisedChat {
        message: "custom notice".to_string(),
        message_styled: plain_chat_styled_content("custom notice"),
        message_component: Some(NbtSummaryValue::string("custom notice")),
        chat_type: bound,
    });
    let mut vanilla_bound = chat_type("Alex");
    vanilla_bound.chat_type = ChatTypeHolder::Registry { id: 1 };
    store.apply_disguised_chat(ProtocolDisguisedChat {
        message: "vanilla moved".to_string(),
        message_styled: plain_chat_styled_content("vanilla moved"),
        message_component: Some(NbtSummaryValue::string("vanilla moved")),
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
fn synchronized_chat_type_stub_malformed_and_unknown_stay_diagnostic() {
    let mut store = WorldStore::new();
    store.record_registry_data(RegistryData {
        registry: CHAT_TYPE_REGISTRY.to_string(),
        entries: vec![RegistryDataEntry {
            id: "minecraft:chat".to_string(),
            raw_data: None,
        }],
        raw_payload_len: 16,
    });

    store.begin_registry_configuration();
    store.finish_registry_configuration().unwrap();

    for id in 0..2 {
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
        .contains("missing minecraft:chat chat type payload"));
    assert!(messages[1]
        .chat_type
        .resolution_error
        .as_deref()
        .unwrap()
        .contains("missing minecraft:chat_type holder 1"));
    assert!(messages.iter().all(|message| message.decoration.is_none()));
    store.record_registry_data(RegistryData {
        registry: CHAT_TYPE_REGISTRY.to_string(),
        entries: vec![RegistryDataEntry {
            id: "example:broken".to_string(),
            raw_data: Some(vec![10, 0]),
        }],
        raw_payload_len: 2,
    });
    store.begin_registry_configuration();
    assert!(store.finish_registry_configuration().is_err());
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

#[test]
fn remote_session_validates_signature_link_order_and_vanilla_timestamp_behavior() {
    let mut store = WorldStore::new();
    let sender = Uuid::from_u128(41);
    let session_id = Uuid::from_u128(42);
    let private_key = install_remote_sender(&mut store, sender, session_id, 10_000);

    let first = signed_player_chat(&private_key, sender, session_id, 0, 0, 2_000, "first");
    let first_outcome = store.apply_player_chat_at(first, 1);
    assert_eq!(first_outcome.disconnect, None);
    assert_eq!(
        store.client_chat().messages[0].validation_state,
        ChatValidationState::Validated
    );

    // 26.1 `SignedMessageValidator.KeyBased` requires a descendant link,
    // but it permits skipped indices and does not reject an older body
    // timestamp. Timestamp age is diagnostic-only in this client path.
    let second = signed_player_chat(&private_key, sender, session_id, 1, 2, 1_000, "second");
    let second_outcome = store.apply_player_chat_at(second.clone(), 1);
    assert_eq!(second_outcome.disconnect, None);
    assert_eq!(
        store.client_chat().messages[1].validation_state,
        ChatValidationState::Validated
    );

    let mut duplicate = second;
    duplicate.global_index = 2;
    let duplicate_outcome = store.apply_player_chat_at(duplicate, 1);
    assert_eq!(duplicate_outcome.disconnect, None);
    assert_eq!(
        store.client_chat().messages[2].validation_state,
        ChatValidationState::Validated
    );
    assert_eq!(store.client_chat().acknowledgement.pending_offset, 2);
}

#[test]
fn invalid_remote_signature_breaks_chain_until_session_replacement() {
    let mut store = WorldStore::new();
    let sender = Uuid::from_u128(43);
    let session_id = Uuid::from_u128(44);
    let private_key = install_remote_sender(&mut store, sender, session_id, 10_000);
    let mut invalid = signed_player_chat(&private_key, sender, session_id, 0, 0, 1_000, "invalid");
    invalid.signature.as_mut().unwrap().bytes[0] ^= 0xff;

    let invalid_outcome = store.apply_player_chat_at(invalid, 1);
    assert_eq!(invalid_outcome.disconnect, None);
    assert_eq!(
        store.client_chat().messages[0].validation_state,
        ChatValidationState::InvalidSignature
    );
    assert_eq!(
        store.client_chat().messages[0].content,
        "chat.validation_error"
    );

    let valid = signed_player_chat(
        &private_key,
        sender,
        session_id,
        1,
        1,
        2_000,
        "cannot resume",
    );
    let chained_outcome = store.apply_player_chat_at(valid, 1);
    assert_eq!(chained_outcome.disconnect, None);
    assert_eq!(
        store.client_chat().messages[1].validation_state,
        ChatValidationState::ChainBroken
    );
}

#[test]
fn remote_key_expiry_uses_vanilla_eight_hour_grace() {
    let mut store = WorldStore::new();
    let sender = Uuid::from_u128(45);
    let session_id = Uuid::from_u128(46);
    let private_key = install_remote_sender(&mut store, sender, session_id, 100);
    let packet = signed_player_chat(&private_key, sender, session_id, 0, 0, 1, "grace");

    let within_grace =
        store.apply_player_chat_at(packet, 100 + PROFILE_PUBLIC_KEY_EXPIRY_GRACE_MILLIS);
    assert_eq!(within_grace.disconnect, None);
    assert_eq!(
        store.client_chat().messages[0].validation_state,
        ChatValidationState::Validated
    );

    let replacement_key = install_remote_sender(&mut store, sender, session_id, 100);
    let expired = signed_player_chat(&replacement_key, sender, session_id, 1, 0, 2, "expired");
    let expired_outcome =
        store.apply_player_chat_at(expired, 101 + PROFILE_PUBLIC_KEY_EXPIRY_GRACE_MILLIS);
    assert_eq!(expired_outcome.disconnect, None);
    assert_eq!(
        store.client_chat().messages[1].validation_state,
        ChatValidationState::ExpiredProfileKey
    );
}

#[test]
fn secure_chat_without_remote_session_presents_error_and_acknowledges_hidden() {
    let mut store = WorldStore::new();
    let sender = Uuid::from_u128(47);
    add_player_without_session(&mut store, sender);
    store.services_player_certificate_keys = vec![vec![1]].into();
    store.server_enforces_secure_chat = Some(true);
    let mut packet = player_chat_with_signature(0, signature(19));
    packet.sender = sender;

    let outcome = store.apply_player_chat_at(packet, 1);
    assert_eq!(outcome.disconnect, None);
    assert_eq!(
        store.client_chat().messages[0].validation_state,
        ChatValidationState::MissingProfileKey
    );
    assert_eq!(
        store.client_chat().messages[0].content,
        "chat.validation_error"
    );
    assert_eq!(store.client_chat().acknowledgement.pending_offset, 1);
    assert!(store
        .client_chat()
        .last_seen_messages
        .entries
        .iter()
        .all(Option::is_none));
}

#[test]
fn unknown_packed_signature_is_fatal_before_cache_or_presentation_mutation() {
    let mut store = WorldStore::new();
    let mut packet = player_chat_with_signature(0, signature(20));
    packet.body.last_seen.push(PackedMessageSignature {
        cache_id: Some(0),
        full_signature: None,
    });

    let outcome = store.apply_player_chat_at(packet, 1);
    assert_eq!(
        outcome.disconnect,
        Some(ChatDisconnectReason::InvalidPackedSignature)
    );
    assert!(store.client_chat().messages.is_empty());
    assert!(store
        .client_chat()
        .signature_cache
        .iter()
        .all(Option::is_none));
}

#[test]
fn delete_chat_replaces_visible_message_on_sixtieth_gui_tick() {
    let mut store = WorldStore::new();
    let sender = Uuid::from_u128(48);
    let session_id = Uuid::from_u128(49);
    let private_key = install_remote_sender(&mut store, sender, session_id, 10_000);
    let packet = signed_player_chat(&private_key, sender, session_id, 0, 0, 1, "delete me");
    let signature = packet.signature.clone().unwrap();
    let _ = store.apply_player_chat_at(packet, 1);
    let display_sequence = store.client_chat().messages[0].display_sequence;
    let display_revision = store.client_chat().delivery_revision;

    assert_eq!(
        store.apply_delete_chat(ProtocolDeleteChat {
            message_signature: PackedMessageSignature {
                cache_id: None,
                full_signature: Some(signature),
            },
        }),
        None
    );
    assert_eq!(
        store.client_chat().deleted_messages[0].deletable_after_tick,
        Some(60)
    );
    store.advance_chat_gui_ticks(59);
    assert_eq!(store.client_chat().messages[0].content, "delete me");
    assert_eq!(store.client_chat().delivery_revision, display_revision);
    assert_eq!(store.client_chat().display_refresh_revision, 0);
    store.advance_chat_gui_ticks(1);
    assert_eq!(store.client_chat().display_refresh_revision, 1);
    assert_eq!(
        store.client_chat().messages[0]
            .content_translation_key
            .as_deref(),
        Some("chat.deleted_marker")
    );
    assert!(store.client_chat().deleted_messages[0].visible_replaced);
    assert_eq!(
        store.client_chat().messages[0].display_sequence,
        display_sequence
    );
    assert_eq!(
        store.client_chat().delivery_revision,
        display_revision.saturating_add(1)
    );
    assert_eq!(
        store.client_chat().deleted_messages[0].deletable_after_tick,
        None
    );
    assert_eq!(
        store
            .take_last_seen_messages_for_outbound_chat()
            .update
            .acknowledged,
        0
    );
}

#[test]
fn delete_chat_removes_delayed_message_and_cache_miss_is_fatal() {
    let mut store = WorldStore::new();
    let sender = Uuid::from_u128(50);
    let session_id = Uuid::from_u128(51);
    let private_key = install_remote_sender(&mut store, sender, session_id, 10_000);
    let packet = signed_player_chat(&private_key, sender, session_id, 0, 0, 1, "delayed");
    let signature = packet.signature.clone().unwrap();
    let _ = store.apply_player_chat_at(packet, 1);
    let message = store.client_chat.messages.pop().unwrap();
    store
        .client_chat
        .delayed_messages
        .push_back(DelayedChatMessageState {
            acknowledge_signature: message.full_signature.as_ref().map(|bytes| {
                ProcessedChatSignatureState {
                    checksum: message.signature.as_ref().unwrap().checksum,
                    bytes: bytes.clone(),
                }
            }),
            message,
            message_timestamp_millis: 1,
            only_show_secure: false,
            validation_error: false,
        });

    assert_eq!(
        store.apply_delete_chat(ProtocolDeleteChat {
            message_signature: PackedMessageSignature {
                cache_id: None,
                full_signature: Some(signature),
            },
        }),
        None
    );
    assert!(store.client_chat().delayed_messages.is_empty());
    assert_eq!(
        store.client_chat().deleted_messages[0].deletable_after_tick,
        None
    );

    let deleted_count = store.client_chat().deleted_messages.len();
    assert_eq!(
        store.apply_delete_chat(ProtocolDeleteChat {
            message_signature: PackedMessageSignature {
                cache_id: Some(127),
                full_signature: None,
            },
        }),
        Some(ChatDisconnectReason::InvalidPackedSignature)
    );
    assert_eq!(store.client_chat().deleted_messages.len(), deleted_count);
}

#[test]
fn delayed_player_chat_defers_display_social_history_and_last_seen_until_acceptance() {
    let mut store = WorldStore::new();
    let sender = Uuid::from_u128(60);
    let session_id = Uuid::from_u128(61);
    let private_key = install_remote_sender(&mut store, sender, session_id, 20_000);
    store.set_chat_presentation_config(ChatPresentationConfig {
        message_delay_millis: 1_000,
        ..ChatPresentationConfig::default()
    });

    let first = signed_player_chat(&private_key, sender, session_id, 0, 0, 1_000, "first");
    let second = signed_player_chat(&private_key, sender, session_id, 1, 1, 1_001, "second");
    let first_outcome = store.apply_player_chat_at_times(first, 1_000, 100);
    let second_outcome = store.apply_player_chat_at_times(second, 1_001, 200);

    assert_eq!(first_outcome.disconnect, None);
    assert_eq!(second_outcome.disconnect, None);
    assert_eq!(store.client_chat().messages.len(), 1);
    assert_eq!(store.client_chat().messages[0].content, "first");
    assert_eq!(store.client_chat().delayed_messages.len(), 1);
    assert_eq!(store.client_chat().social_history.events.len(), 1);
    assert_eq!(store.client_chat().acknowledgement.pending_offset, 1);

    assert!(store.advance_chat_listener(1_099, false).is_empty());
    assert_eq!(store.client_chat().messages.len(), 1);
    assert!(store.advance_chat_listener(1_100, false).is_empty());
    assert_eq!(store.client_chat().messages.len(), 2);
    assert_eq!(store.client_chat().messages[1].content, "second");
    assert!(store.client_chat().delayed_messages.is_empty());
    assert_eq!(store.client_chat().social_history.events.len(), 2);
    assert_eq!(store.client_chat().acknowledgement.pending_offset, 2);
}

#[test]
fn secure_only_captures_receive_option_and_suppresses_unsigned_messages() {
    let mut store = WorldStore::new();
    let sender = Uuid::from_u128(62);
    add_player_without_session(&mut store, sender);
    store.set_chat_presentation_config(ChatPresentationConfig {
        only_show_secure: true,
        ..ChatPresentationConfig::default()
    });
    let mut packet = player_chat_with_signature(0, signature(22));
    packet.sender = sender;
    packet.unsigned_content = Some("decorated".to_string());
    packet.unsigned_content_styled = Some(plain_chat_styled_content("decorated"));

    let outcome = store.apply_player_chat_at_times(packet, 1, 100);

    assert_eq!(outcome.disconnect, None);
    assert!(store.client_chat().messages.is_empty());
    assert!(store.client_chat().social_history.events.is_empty());
    assert!(store.client_chat().pending_narrations.is_empty());
}

#[test]
fn player_visibility_and_blocking_suppress_display_but_process_valid_signature() {
    let mut store = WorldStore::new();
    let sender = Uuid::from_u128(63);
    let session_id = Uuid::from_u128(64);
    let private_key = install_remote_sender(&mut store, sender, session_id, 20_000);
    store.set_chat_presentation_config(ChatPresentationConfig {
        blocked_players: vec![sender],
        ..ChatPresentationConfig::default()
    });
    let packet = signed_player_chat(&private_key, sender, session_id, 0, 0, 1, "blocked");

    let outcome = store.apply_player_chat_at_times(packet, 1, 100);

    assert_eq!(outcome.disconnect, None);
    assert!(store.client_chat().messages.is_empty());
    assert_eq!(store.client_chat().acknowledgement.pending_offset, 1);
    assert!(store
        .client_chat()
        .last_seen_messages
        .entries
        .iter()
        .all(Option::is_none));
}

#[test]
fn system_visibility_and_matched_hidden_name_follow_chat_options() {
    let mut store = WorldStore::new();
    let sender = Uuid::from_u128(67);
    add_player_without_session(&mut store, sender);
    store.set_chat_presentation_config(ChatPresentationConfig {
        visibility: ChatVisibilityState::System,
        hidden_players: vec![sender],
        ..ChatPresentationConfig::default()
    });

    store.push_rich_server_system_chat_message(
        "<Alice> hidden".to_string(),
        plain_chat_styled_content("<Alice> hidden"),
        None,
        1,
    );
    assert!(store.client_chat().messages.is_empty());

    // Vanilla resolves the mention through the discovered-names map, so a
    // hidden player keeps matching after leaving the server.
    store.apply_player_info_remove(PlayerInfoRemove {
        profile_ids: vec![sender],
    });
    store.push_rich_server_system_chat_message(
        "<Alice> still hidden".to_string(),
        plain_chat_styled_content("<Alice> still hidden"),
        None,
        2,
    );
    assert!(store.client_chat().messages.is_empty());

    store.push_rich_server_system_chat_message(
        "server notice".to_string(),
        plain_chat_styled_content("server notice"),
        None,
        2,
    );
    assert_eq!(store.client_chat().messages.len(), 1);
    assert_eq!(store.client_chat().social_history.events.len(), 1);

    let mut config = store.chat_presentation_config().clone();
    config.visibility = ChatVisibilityState::Hidden;
    store.set_chat_presentation_config(config);
    store.push_server_system_chat_message("another notice");
    store.push_client_system_chat_message("local feedback");
    assert_eq!(store.client_chat().messages.len(), 2);
    assert_eq!(
        store.client_chat().messages[1].kind,
        ChatMessageKind::ClientSystem
    );
}

#[test]
fn reset_chat_preserves_startup_presentation_options() {
    let mut store = WorldStore::new();
    store.set_chat_presentation_config(ChatPresentationConfig {
        visibility: ChatVisibilityState::Hidden,
        only_show_secure: true,
        hide_matched_names: false,
        message_delay_millis: 600,
        hidden_players: vec![Uuid::from_u128(68)],
        blocked_players: vec![Uuid::from_u128(69)],
    });
    let expected = store.chat_presentation_config().clone();

    store.apply_reset_chat();

    assert_eq!(store.chat_presentation_config(), &expected);
}

#[test]
fn publication_snapshot_isolates_chat_delivery_queue_and_social_history() {
    let mut store = WorldStore::new();
    store.push_server_system_chat_message("generation a");
    let generation_a = store.publication_snapshot();

    store.push_server_system_chat_message("generation b");
    let _ = store.take_pending_chat_narrations();
    let generation_b = store.publication_snapshot();

    assert_eq!(generation_a.client_chat().messages.len(), 1);
    assert_eq!(
        generation_a.client_chat().messages[0].content,
        "generation a"
    );
    assert_eq!(generation_a.client_chat().social_history.events.len(), 1);
    assert_eq!(generation_a.client_chat().pending_narrations.len(), 1);
    assert_eq!(generation_b.client_chat().messages.len(), 2);
    assert_eq!(generation_b.client_chat().social_history.events.len(), 2);
    assert!(generation_b.client_chat().pending_narrations.is_empty());
}

fn signature(byte: u8) -> MessageSignature {
    MessageSignature {
        bytes: vec![byte; 256],
    }
}

fn add_player_without_session(store: &mut WorldStore, sender: Uuid) {
    let _ = store.apply_player_info_update(PlayerInfoUpdate {
        actions: vec![PlayerInfoAction::AddPlayer],
        entries: vec![PlayerInfoEntry {
            profile_id: sender,
            profile: Some(GameProfile {
                uuid: sender,
                name: "Alice".to_string(),
                properties: Vec::new(),
            }),
            listed: false,
            latency: 0,
            game_mode: GameType::Survival,
            display_name: None,
            show_hat: true,
            list_order: 0,
            chat_session: None,
        }],
    });
}

fn fixture_private_key() -> RsaPrivateKey {
    static KEY: OnceLock<RsaPrivateKey> = OnceLock::new();
    KEY.get_or_init(|| {
        let mut random = crate::RsaTestRng::seeded(401);
        RsaPrivateKey::new(&mut random, 2_048).unwrap()
    })
    .clone()
}

fn install_remote_sender(
    store: &mut WorldStore,
    sender: Uuid,
    session_id: Uuid,
    expires_at_epoch_millis: i64,
) -> RsaPrivateKey {
    add_player_without_session(store, sender);
    let private_key = fixture_private_key();
    let public_key_der = RsaPublicKey::from(&private_key)
        .to_public_key_der()
        .unwrap()
        .as_bytes()
        .to_vec();
    let info = store.player_info.entries.get_mut(&sender).unwrap();
    info.chat_session_present = true;
    info.chat_session = Some(RemoteChatSessionState::new(
        session_id,
        expires_at_epoch_millis,
        public_key_der,
    ));
    private_key
}

fn signed_player_chat(
    private_key: &RsaPrivateKey,
    sender: Uuid,
    session_id: Uuid,
    global_index: i32,
    message_index: i32,
    timestamp_millis: i64,
    content: &str,
) -> PlayerChat {
    let mut payload = Vec::new();
    payload.extend_from_slice(&1_i32.to_be_bytes());
    payload.extend_from_slice(sender.as_bytes());
    payload.extend_from_slice(session_id.as_bytes());
    payload.extend_from_slice(&message_index.to_be_bytes());
    payload.extend_from_slice(&(message_index as i64 + 17).to_be_bytes());
    payload.extend_from_slice(&timestamp_millis.div_euclid(1_000).to_be_bytes());
    payload.extend_from_slice(&(content.len() as i32).to_be_bytes());
    payload.extend_from_slice(content.as_bytes());
    payload.extend_from_slice(&0_i32.to_be_bytes());
    let signature = private_key
        .sign(Pkcs1v15Sign::new::<Sha256>(), &Sha256::digest(payload))
        .unwrap();
    PlayerChat {
        global_index,
        sender,
        index: message_index,
        signature: Some(MessageSignature { bytes: signature }),
        body: SignedMessageBody {
            content: content.to_string(),
            timestamp_millis,
            salt: message_index as i64 + 17,
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
