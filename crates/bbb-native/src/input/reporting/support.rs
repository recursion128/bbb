use super::*;

impl PlayerReportingRuntime {
    pub(super) fn focusables(&self, world: &WorldStore) -> Vec<HudPlayerReportingFocus> {
        match &self.screen {
            PlayerReportingRuntimeScreen::TypeChooser => {
                let mut values = Vec::with_capacity(4);
                if self.target.chat_enabled {
                    values.push(HudPlayerReportingFocus::ReportType(0));
                }
                values.extend([
                    HudPlayerReportingFocus::ReportType(1),
                    HudPlayerReportingFocus::ReportType(2),
                    HudPlayerReportingFocus::Cancel,
                ]);
                values
            }
            PlayerReportingRuntimeScreen::Editor(report_type) => {
                let mut values = Vec::with_capacity(6);
                if *report_type == PlayerReportType::Chat {
                    values.push(HudPlayerReportingFocus::SelectMessages);
                }
                if *report_type != PlayerReportType::Username {
                    values.push(HudPlayerReportingFocus::SelectReason);
                }
                values.extend([
                    HudPlayerReportingFocus::Comments,
                    HudPlayerReportingFocus::Attestation,
                    HudPlayerReportingFocus::Back,
                ]);
                if world.build_player_report().is_ok() {
                    values.push(HudPlayerReportingFocus::Send);
                }
                values
            }
            PlayerReportingRuntimeScreen::ReasonSelection {
                editor_type,
                pending: _,
            } => player_report_reasons(*editor_type)
                .into_iter()
                .enumerate()
                .filter_map(|(index, _)| {
                    u8::try_from(index)
                        .ok()
                        .map(HudPlayerReportingFocus::Reason)
                })
                .chain([
                    HudPlayerReportingFocus::ReadInfo,
                    HudPlayerReportingFocus::Done,
                    HudPlayerReportingFocus::Back,
                ])
                .collect(),
            PlayerReportingRuntimeScreen::ChatSelection { pending } => {
                let mut values = reportable_message_ids(world, self.target.profile_id)
                    .into_iter()
                    .filter(|message_id| {
                        pending.contains(message_id) || pending.len() < MAX_SELECTED_REPORT_MESSAGES
                    })
                    .map(HudPlayerReportingFocus::Message)
                    .collect::<Vec<_>>();
                if !pending.is_empty() {
                    values.push(HudPlayerReportingFocus::Done);
                }
                values.push(HudPlayerReportingFocus::Back);
                values
            }
            PlayerReportingRuntimeScreen::DraftChoice => vec![
                HudPlayerReportingFocus::DraftEdit,
                HudPlayerReportingFocus::DraftDiscard,
                HudPlayerReportingFocus::Cancel,
            ],
            PlayerReportingRuntimeScreen::DiscardWarning(_) => vec![
                HudPlayerReportingFocus::WarningReturn,
                HudPlayerReportingFocus::WarningSaveDraft,
                HudPlayerReportingFocus::WarningDiscard,
            ],
            PlayerReportingRuntimeScreen::Sending { .. } => {
                vec![HudPlayerReportingFocus::Cancel]
            }
            PlayerReportingRuntimeScreen::Success => vec![HudPlayerReportingFocus::Done],
            PlayerReportingRuntimeScreen::Failure => vec![
                HudPlayerReportingFocus::Retry,
                HudPlayerReportingFocus::Back,
            ],
        }
    }

    pub(super) fn move_focus(&mut self, world: &WorldStore, forward: bool) {
        let values = self.focusables(world);
        if values.is_empty() {
            return;
        }
        let current = values
            .iter()
            .position(|focus| *focus == self.focus)
            .unwrap_or(0);
        let next = if forward {
            (current + 1) % values.len()
        } else if current == 0 {
            values.len() - 1
        } else {
            current - 1
        };
        self.focus = values[next];
    }
}

pub(super) fn editor_initial_focus(report_type: PlayerReportType) -> HudPlayerReportingFocus {
    match report_type {
        PlayerReportType::Chat => HudPlayerReportingFocus::SelectMessages,
        PlayerReportType::Skin => HudPlayerReportingFocus::SelectReason,
        PlayerReportType::Username => HudPlayerReportingFocus::Comments,
    }
}

pub(super) fn selected_reason(world: &WorldStore) -> Option<PlayerReportReason> {
    match world.player_reporting().draft.as_ref()? {
        PlayerReportDraft::Chat { reason, .. } | PlayerReportDraft::Skin { reason, .. } => *reason,
        PlayerReportDraft::Username { .. } => None,
    }
}

pub(super) fn selected_chat_messages(world: &WorldStore) -> BTreeSet<u64> {
    match world.player_reporting().draft.as_ref() {
        Some(PlayerReportDraft::Chat {
            selected_message_ids,
            ..
        }) => selected_message_ids.clone(),
        _ => BTreeSet::new(),
    }
}

pub(super) fn reportable_message_ids(world: &WorldStore, target: Uuid) -> Vec<u64> {
    world
        .client_chat()
        .report_log
        .events
        .iter()
        .filter(|event| {
            event
                .signed_message
                .as_ref()
                .is_some_and(|signed| signed.sender == target && signed.signature.is_some())
        })
        .map(|event| event.id)
        .collect()
}

pub(super) fn epoch_millis() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .ok()
        .and_then(|duration| i64::try_from(duration.as_millis()).ok())
        .unwrap_or(0)
}

pub(super) fn next_report_id() -> Uuid {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.as_nanos());
    let nonce = u128::from(NEXT_REPORT_NONCE.fetch_add(1, Ordering::Relaxed));
    let mut bytes = (now ^ nonce).to_be_bytes();
    bytes[6] = (bytes[6] & 0x0f) | 0x40;
    bytes[8] = (bytes[8] & 0x3f) | 0x80;
    Uuid::from_bytes(bytes)
}

pub(super) fn edit_comments_key(
    world: &mut WorldStore,
    active: &mut PlayerReportingRuntime,
    code: KeyCode,
    shift_down: bool,
    control_down: bool,
) -> bool {
    let Some(draft) = world.copy_player_report_draft() else {
        return false;
    };
    let mut comments = draft.common().comments.clone();
    let mut changed = false;
    match code {
        KeyCode::KeyA if control_down => {
            active.comments_cursor = text_edit::char_len(&comments);
            active.comments_selection = 0;
        }
        KeyCode::ArrowLeft => {
            let next = if control_down {
                text_edit::word_position(&comments, active.comments_cursor, -1)
            } else {
                active.comments_cursor.saturating_sub(1)
            };
            set_cursor(active, next, shift_down, &comments);
        }
        KeyCode::ArrowRight => {
            let next = if control_down {
                text_edit::word_position(&comments, active.comments_cursor, 1)
            } else {
                active.comments_cursor.saturating_add(1)
            };
            set_cursor(active, next, shift_down, &comments);
        }
        KeyCode::Home => set_cursor(active, 0, shift_down, &comments),
        KeyCode::End => {
            let len = text_edit::char_len(&comments);
            set_cursor(active, len, shift_down, &comments);
        }
        KeyCode::Backspace => {
            if !delete_selection(
                &mut comments,
                &mut active.comments_cursor,
                &mut active.comments_selection,
            ) {
                if control_down {
                    text_edit::remove_word_before_cursor(
                        &mut comments,
                        &mut active.comments_cursor,
                    );
                } else if active.comments_cursor > 0 {
                    let start = text_edit::byte_index(&comments, active.comments_cursor - 1);
                    let end = text_edit::byte_index(&comments, active.comments_cursor);
                    comments.replace_range(start..end, "");
                    active.comments_cursor -= 1;
                }
                active.comments_selection = active.comments_cursor;
            }
            changed = true;
        }
        KeyCode::Delete => {
            if !delete_selection(
                &mut comments,
                &mut active.comments_cursor,
                &mut active.comments_selection,
            ) {
                if control_down {
                    text_edit::remove_word_at_cursor(&mut comments, active.comments_cursor);
                } else if active.comments_cursor < text_edit::char_len(&comments) {
                    let start = text_edit::byte_index(&comments, active.comments_cursor);
                    let end = text_edit::byte_index(&comments, active.comments_cursor + 1);
                    comments.replace_range(start..end, "");
                }
                active.comments_selection = active.comments_cursor;
            }
            changed = true;
        }
        KeyCode::Enter | KeyCode::NumpadEnter => {
            replace_selection(
                &mut comments,
                &mut active.comments_cursor,
                &mut active.comments_selection,
                "\n",
            );
            changed = true;
        }
        _ => return false,
    }
    if changed {
        comments = truncate_utf16(comments, MAX_REPORT_COMMENT_UTF16_UNITS);
        let len = text_edit::char_len(&comments);
        active.comments_cursor = active.comments_cursor.min(len);
        active.comments_selection = active.comments_selection.min(len);
        world.set_player_report_comments(draft.common().report_id, comments);
    }
    true
}

fn set_cursor(active: &mut PlayerReportingRuntime, cursor: usize, selecting: bool, text: &str) {
    active.comments_cursor = cursor.min(text_edit::char_len(text));
    if !selecting {
        active.comments_selection = active.comments_cursor;
    }
}

pub(super) fn replace_selection(
    text: &mut String,
    cursor: &mut usize,
    selection: &mut usize,
    replacement: &str,
) {
    let start = (*cursor).min(*selection);
    let end = (*cursor).max(*selection);
    let start_byte = text_edit::byte_index(text, start);
    let end_byte = text_edit::byte_index(text, end);
    text.replace_range(start_byte..end_byte, replacement);
    *cursor = start + replacement.chars().count();
    *selection = *cursor;
}

fn delete_selection(text: &mut String, cursor: &mut usize, selection: &mut usize) -> bool {
    if *cursor == *selection {
        return false;
    }
    replace_selection(text, cursor, selection, "");
    true
}

pub(super) fn truncate_utf16(value: String, max_units: usize) -> String {
    if value.encode_utf16().count() <= max_units {
        return value;
    }
    let mut units = 0;
    value
        .chars()
        .take_while(|character| {
            let next = units + character.len_utf16();
            if next > max_units {
                false
            } else {
                units = next;
                true
            }
        })
        .collect()
}
