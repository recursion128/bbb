use super::*;

pub(super) fn plain_chat_styled_content(content: &str) -> Vec<StyledTextRun> {
    vec![StyledTextRun {
        text: content.to_string(),
        style: ComponentStyle::default(),
    }]
}

pub(super) fn styled_chat_summary_text(runs: &[StyledTextRun]) -> String {
    runs.iter().map(|run| run.text.as_str()).collect()
}

pub(super) fn resolve_packed_signature(
    cache: &[Option<ChatSignatureState>],
    packed: &PackedMessageSignature,
) -> Option<ChatSignatureState> {
    if let Some(signature) = &packed.full_signature {
        return Some(chat_signature_state(signature));
    }

    let id = usize::try_from(packed.cache_id?).ok()?;
    cache.get(id)?.clone()
}

pub(super) fn remote_message_signature_is_valid(
    session: &RemoteChatSessionState,
    message: &ValidatedRemoteChatMessageState,
) -> bool {
    let Ok(key) = RsaPublicKey::from_public_key_der(&session.public_key_der) else {
        return false;
    };
    let mut payload = Vec::new();
    payload.extend_from_slice(&1_i32.to_be_bytes());
    payload.extend_from_slice(message.sender.as_bytes());
    payload.extend_from_slice(message.session_id.as_bytes());
    payload.extend_from_slice(&message.index.to_be_bytes());
    payload.extend_from_slice(&message.salt.to_be_bytes());
    payload.extend_from_slice(&message.timestamp_millis.div_euclid(1_000).to_be_bytes());
    let Ok(content_len) = i32::try_from(message.content.len()) else {
        return false;
    };
    payload.extend_from_slice(&content_len.to_be_bytes());
    payload.extend_from_slice(message.content.as_bytes());
    let Ok(last_seen_len) = i32::try_from(message.last_seen.len()) else {
        return false;
    };
    payload.extend_from_slice(&last_seen_len.to_be_bytes());
    for signature in &message.last_seen {
        payload.extend_from_slice(signature);
    }
    let digest = Sha256::digest(payload);
    key.verify(Pkcs1v15Sign::new::<Sha256>(), &digest, &message.signature)
        .is_ok()
}

pub(super) fn replace_visible_chat_message_or_delay(
    messages: &mut [ChatMessageState],
    signature: &[u8],
    now: u64,
) -> (bool, Option<u64>) {
    let Some(message) = messages
        .iter_mut()
        .rev()
        .find(|message| message.full_signature.as_deref() == Some(signature))
    else {
        return (false, None);
    };
    let deletable_after = message.added_tick.saturating_add(CHAT_DELETION_DELAY_TICKS);
    if now < deletable_after {
        return (false, Some(deletable_after));
    }
    let added_tick = message.added_tick;
    let display_sequence = message.display_sequence;
    *message = deleted_chat_marker(added_tick, display_sequence);
    (true, None)
}

pub(super) fn deleted_chat_marker(added_tick: u64, display_sequence: u64) -> ChatMessageState {
    ChatMessageState {
        kind: ChatMessageKind::System,
        source: ChatMessageSourceState::SystemServer,
        content: "chat.deleted_marker".to_string(),
        content_translation_key: Some("chat.deleted_marker".to_string()),
        styled_content: vec![StyledTextRun {
            text: "chat.deleted_marker".to_string(),
            style: ComponentStyle {
                italic: Some(true),
                color: Some(0xaaaaaa),
                ..ComponentStyle::default()
            },
        }],
        content_component: Some(NbtSummaryValue::string("chat.deleted_marker")),
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
        tag: Some(ChatMessageTagState::System),
        received_epoch_millis: 0,
        display_sequence,
        filter_mask_words: Vec::new(),
        unsigned_content_has_custom_font: false,
        added_tick,
        full_signature: None,
    }
}

pub(super) fn push_signature_cache(
    cache: &mut [Option<ChatSignatureState>],
    last_seen: Vec<ChatSignatureState>,
    signature: Option<ChatSignatureState>,
) {
    let mut queue = last_seen;
    if let Some(signature) = signature {
        queue.push(signature);
    }
    // Vanilla `MessageSignatureCache.push`: snapshot the initial batch before
    // popping so evicted entries are deduplicated against the incoming batch,
    // not the shrinking tail of `queue`.
    let new_entries = queue.clone();

    for slot in cache.iter_mut() {
        let Some(next) = queue.pop() else {
            break;
        };
        let previous = slot.replace(next);
        if let Some(previous) = previous {
            if !new_entries.contains(&previous) {
                queue.insert(0, previous);
            }
        }
    }
}

pub(super) fn chat_signature_state(signature: &ProtocolMessageSignature) -> ChatSignatureState {
    ChatSignatureState {
        checksum: signature.checksum(),
        bytes_len: signature.bytes.len(),
        bytes: signature.bytes.clone(),
    }
}

pub(super) fn processed_chat_signature_state(
    signature: &ProtocolMessageSignature,
) -> ProcessedChatSignatureState {
    ProcessedChatSignatureState {
        checksum: signature.checksum(),
        bytes: signature.bytes.clone(),
    }
}

pub(super) fn last_seen_checksum_byte(checksum: i32) -> u8 {
    let checksum = checksum as u8;
    if checksum == 0 {
        1
    } else {
        checksum
    }
}

pub(super) fn nonempty_styled_or_plain(
    styled: &[StyledTextRun],
    plain: &str,
) -> Vec<StyledTextRun> {
    if styled.is_empty() {
        plain_chat_styled_content(plain)
    } else {
        styled.to_vec()
    }
}

pub(super) fn evaluate_chat_trust_level(delayed: &DelayedChatMessageState) -> ChatTrustLevelState {
    let message = &delayed.message;
    if message.signature.is_none()
        || (message.received_epoch_millis != i64::MIN
            && delayed.message_timestamp_millis
                < message
                    .received_epoch_millis
                    .saturating_sub(presentation::CLIENT_CHAT_EXPIRY_MILLIS))
    {
        return ChatTrustLevelState::NotSecure;
    }
    if !styled_chat_summary_text(&message.styled_content).contains(&message.content)
        || message.unsigned_content_has_custom_font
    {
        ChatTrustLevelState::Modified
    } else {
        ChatTrustLevelState::Secure
    }
}

pub(super) fn component_has_custom_font(component: &NbtSummaryValue) -> bool {
    match component {
        NbtSummaryValue::Compound(entries) => entries.iter().any(|entry| {
            (entry.name == "font"
                && !matches!(
                    &entry.value,
                    NbtSummaryValue::String(font)
                        if font == "default" || font == "minecraft:default"
                ))
                || component_has_custom_font(&entry.value)
        }),
        NbtSummaryValue::List(values) => values.iter().any(component_has_custom_font),
        NbtSummaryValue::Byte(_)
        | NbtSummaryValue::Short(_)
        | NbtSummaryValue::Int(_)
        | NbtSummaryValue::Long(_)
        | NbtSummaryValue::Float(_)
        | NbtSummaryValue::Double(_)
        | NbtSummaryValue::ByteArray(_)
        | NbtSummaryValue::String(_)
        | NbtSummaryValue::IntArray(_)
        | NbtSummaryValue::LongArray(_) => false,
    }
}

pub(super) fn filtered_chat_content(
    content: &str,
    mask_words: &[i64],
) -> (Vec<StyledTextRun>, Option<NbtSummaryValue>) {
    let units: Vec<u16> = content.encode_utf16().collect();
    let mut runs = Vec::new();
    let mut start = 0;
    while start < units.len() {
        let filtered = filter_bit_is_set(mask_words, start);
        let mut end = start + 1;
        while end < units.len() && filter_bit_is_set(mask_words, end) == filtered {
            end += 1;
        }
        let text = if filtered {
            "#".repeat(end - start)
        } else {
            String::from_utf16_lossy(&units[start..end])
        };
        runs.push(StyledTextRun {
            text,
            style: if filtered {
                ComponentStyle {
                    color: Some(0x555555),
                    ..ComponentStyle::default()
                }
            } else {
                ComponentStyle::default()
            },
        });
        start = end;
    }
    let summary = styled_chat_summary_text(&runs);
    (runs, Some(NbtSummaryValue::string(summary)))
}

pub(super) fn filter_bit_is_set(mask_words: &[i64], index: usize) -> bool {
    let Some(word) = mask_words.get(index / i64::BITS as usize) else {
        return false;
    };
    ((*word as u64) & (1_u64 << (index % i64::BITS as usize))) != 0
}

pub(super) fn replace_chat_decoration_content(message: &mut ChatMessageState) {
    let content = styled_chat_summary_text(&message.styled_content);
    let styled = message.styled_content.clone();
    let component = message.content_component.clone();
    for decoration in [
        message.decoration.as_mut(),
        message.narration_decoration.as_mut(),
    ]
    .into_iter()
    .flatten()
    {
        for argument in &mut decoration.arguments {
            if argument.parameter == ChatTypeDecorationParameter::Content {
                argument.summary = content.clone();
                argument.styled = styled.clone();
                argument.component = component.clone();
            }
        }
    }
}

pub(super) fn resolved_chat_state(
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
                .then(|| resolved.chat.translation_key.to_display_string());
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

pub(super) fn chat_decoration_state(
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
                        .or_else(|| Some(NbtSummaryValue::string(bound.name.clone()))),
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
                            .map(|target| NbtSummaryValue::string(target.clone()))
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

pub(super) fn refresh_chat_counters(store: &mut WorldStore) {
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

    #[test]
    fn push_signature_cache_preserves_vanilla_slot_order_across_three_signed_messages() {
        let sig = |id: u8| ChatSignatureState {
            checksum: i32::from(id),
            bytes_len: 1,
            bytes: vec![id],
        };
        let mut cache = vec![None; 8];
        push_signature_cache(&mut cache, vec![], Some(sig(1)));
        push_signature_cache(&mut cache, vec![sig(1)], Some(sig(2)));
        push_signature_cache(&mut cache, vec![sig(1), sig(2)], Some(sig(3)));
        assert_eq!(
            &cache[..4],
            &[Some(sig(3)), Some(sig(2)), Some(sig(1)), None]
        );
    }
}
