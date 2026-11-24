use super::*;

#[test]
fn take_pending_player_chat_acknowledgement_drains_below_threshold_offset() {
    let mut store = WorldStore::new();
    assert_eq!(
        store.record_processed_player_chat_signature(&signature(11), true),
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
        store.record_processed_player_chat_signature(&seen, true),
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
            store.record_processed_player_chat_signature(&seen, true),
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
fn player_chat_bad_global_index_requests_disconnect_without_presentation() {
    let mut store = WorldStore::new();
    let outcome = store.apply_player_chat_at(
        PlayerChat {
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
        },
        1,
    );

    assert_eq!(store.counters().player_chat_index_mismatches, 1);
    assert_eq!(
        outcome.disconnect,
        Some(ChatDisconnectReason::BadGlobalIndex)
    );
    assert!(store.client_chat().messages.is_empty());
    assert!(store
        .client_chat()
        .signature_cache
        .iter()
        .all(Option::is_none));
}

#[test]
fn direct_chat_type_preserves_style_and_vanilla_zero_parameter_fallback() {
    let mut store = WorldStore::new();
    store.apply_disguised_chat(ProtocolDisguisedChat {
        message: "direct notice".to_string(),
        message_styled: plain_chat_styled_content("direct notice"),
        message_component: Some(NbtSummaryValue::string("direct notice")),
        chat_type: ChatTypeBound {
            chat_type: ChatTypeHolder::Direct {
                chat: ChatTypeDecorationSummary {
                    translation_key: "example.direct".to_string(),
                    parameters: vec![99, 2],
                    style: bbb_protocol::RichTextStyle::try_from_style(ComponentStyle {
                        italic: Some(true),
                        ..ComponentStyle::default()
                    })
                    .unwrap(),
                },
                narration: ChatTypeDecorationSummary {
                    translation_key: "example.direct.narrate".to_string(),
                    parameters: vec![0, 2],
                    style: bbb_protocol::RichTextStyle::default(),
                },
            },
            name: "Server".to_string(),
            name_styled: plain_chat_styled_content("Server"),
            name_component: Some(NbtSummaryValue::string("Server")),
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
fn clear_client_chat_display_messages_preserves_signature_state() {
    let mut store = WorldStore::new();
    let seen = signature(9);
    store.push_client_system_chat_message("hello");
    push_signature_cache(
        &mut store.client_chat.signature_cache,
        Vec::new(),
        Some(chat_signature_state(&seen)),
    );
    let _ = store.record_processed_player_chat_signature(&seen, true);
    assert_eq!(store.client_chat().messages.len(), 1);
    assert!(store.client_chat().deleted_messages.is_empty());
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
    let display_revision = store.client_chat().delivery_revision;

    store.clear_client_chat_display_messages();

    assert!(store.client_chat().messages.is_empty());
    assert!(store.client_chat().deleted_messages.is_empty());
    assert_eq!(store.client_chat().expected_player_chat_global_index, 0);
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
    assert_eq!(store.counters().player_chat_packets, 0);
    assert_eq!(store.counters().delete_chat_packets, 0);
    assert_eq!(store.counters().chat_messages_tracked, 0);
    assert_eq!(store.counters().deleted_chat_messages_tracked, 0);
    assert_eq!(store.counters().chat_signature_cache_entries, 1);
    assert_eq!(
        store.counters().player_chat_acknowledgement_pending_offset,
        1
    );
    assert_eq!(
        store.client_chat().delivery_revision,
        display_revision.saturating_add(1)
    );
}

#[test]
fn trust_marks_expired_and_custom_font_unsigned_content() {
    let mut store = WorldStore::new();
    let sender = Uuid::from_u128(65);
    let session_id = Uuid::from_u128(66);
    let private_key = install_remote_sender(&mut store, sender, session_id, 1_000_000);
    let expired = signed_player_chat(&private_key, sender, session_id, 0, 0, 1, "expired");
    let _ =
        store.apply_player_chat_at_times(expired, presentation::CLIENT_CHAT_EXPIRY_MILLIS + 2, 100);
    assert_eq!(
        store.client_chat().messages[0].trust_level,
        ChatTrustLevelState::NotSecure
    );
    assert_eq!(
        store.client_chat().messages[0].tag,
        Some(ChatMessageTagState::NotSecure)
    );

    let mut modified = signed_player_chat(&private_key, sender, session_id, 1, 1, 10, "modified");
    modified.unsigned_content = Some("modified".to_string());
    modified.unsigned_content_styled = Some(plain_chat_styled_content("modified"));
    modified.unsigned_content_component = Some(NbtSummaryValue::Compound(vec![
        bbb_protocol::NbtSummaryEntry {
            name: "text".into(),
            value: NbtSummaryValue::string("modified"),
        },
        bbb_protocol::NbtSummaryEntry {
            name: "font".into(),
            value: NbtSummaryValue::string("minecraft:uniform"),
        },
    ]));
    let _ = store.apply_player_chat_at_times(modified, 10, 200);
    assert_eq!(
        store.client_chat().messages[1].trust_level,
        ChatTrustLevelState::Modified
    );
    assert!(matches!(
        store.client_chat().messages[1].tag,
        Some(ChatMessageTagState::Modified { .. })
    ));
}
