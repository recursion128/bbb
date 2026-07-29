use std::collections::{BTreeSet, HashSet};

use uuid::Uuid;

use crate::{ChatReportEventKind, ChatReportLogState, ChatReportSignedMessageState};

use super::{
    BuiltPlayerReportMessage, PlayerReportBuildError, LEADING_REPORT_CONTEXT_MESSAGES,
    MAX_REPORT_EVIDENCE_MESSAGES,
};

pub(super) fn chat_event_is_reportable(
    log: &ChatReportLogState,
    message_id: u64,
    reported_profile_id: Uuid,
) -> bool {
    log.events.iter().any(|event| {
        event.id == message_id
            && event.kind == ChatReportEventKind::Player
            && event.signed_message.as_ref().is_some_and(|message| {
                message.sender == reported_profile_id && message.signature.is_some()
            })
    })
}

pub(super) fn build_chat_evidence(
    log: &ChatReportLogState,
    reported_profile_id: Uuid,
    selected_message_ids: &BTreeSet<u64>,
) -> Result<Vec<BuiltPlayerReportMessage>, PlayerReportBuildError> {
    if selected_message_ids
        .iter()
        .any(|id| !chat_event_is_reportable(log, *id, reported_profile_id))
    {
        return Err(PlayerReportBuildError::SelectedMessageUnavailable);
    }

    let mut uncollected_roots = selected_message_ids.clone();
    let mut collectors = Vec::<ChatContextCollector>::new();
    let mut collected = Vec::new();

    for event in log.events.iter().rev() {
        if collectors.is_empty() && uncollected_roots.is_empty() {
            break;
        }
        if let Some(message) = event.signed_message.as_ref() {
            let context = accept_context(&mut collectors, message);
            let reported = uncollected_roots.remove(&event.id);
            if reported {
                collectors.push(ChatContextCollector::new(message));
            }
            if reported || context {
                collected.push(built_report_message(message, reported));
            }
        }
    }
    if !uncollected_roots.is_empty() {
        return Err(PlayerReportBuildError::SelectedMessageUnavailable);
    }
    debug_assert!(collected.len() <= MAX_REPORT_EVIDENCE_MESSAGES);
    collected.reverse();
    Ok(collected)
}

fn built_report_message(
    message: &ChatReportSignedMessageState,
    message_reported: bool,
) -> BuiltPlayerReportMessage {
    BuiltPlayerReportMessage {
        index: message.index,
        profile_id: message.sender,
        session_id: message.session_id,
        timestamp_millis: message.timestamp_millis,
        salt: message.salt,
        last_seen_signatures: message.last_seen_signatures.clone(),
        message: message.content.clone(),
        signature: message.signature.clone(),
        message_reported,
    }
}

fn accept_context(
    collectors: &mut Vec<ChatContextCollector>,
    message: &ChatReportSignedMessageState,
) -> bool {
    let mut selected = false;
    collectors.retain_mut(|collector| {
        selected |= collector.accept(message);
        !collector.is_complete()
    });
    selected
}

struct ChatContextCollector {
    last_seen_signatures: HashSet<Vec<u8>>,
    last_chain_message: ChatReportSignedMessageState,
    collecting_chain: bool,
    count: usize,
}

impl ChatContextCollector {
    fn new(message: &ChatReportSignedMessageState) -> Self {
        Self {
            last_seen_signatures: message.last_seen_signatures.iter().cloned().collect(),
            last_chain_message: message.clone(),
            collecting_chain: true,
            count: 0,
        }
    }

    fn accept(&mut self, message: &ChatReportSignedMessageState) -> bool {
        if message == &self.last_chain_message {
            return false;
        }
        let mut selected = message
            .signature
            .as_ref()
            .is_some_and(|signature| self.last_seen_signatures.remove(signature));
        if self.collecting_chain && self.last_chain_message.sender == message.sender {
            if self.last_chain_message.index > message.index
                && self.last_chain_message.session_id == message.session_id
            {
                selected = true;
                self.last_chain_message = message.clone();
            } else {
                self.collecting_chain = false;
            }
        }
        if selected {
            self.count += 1;
        }
        selected
    }

    fn is_complete(&self) -> bool {
        self.count >= LEADING_REPORT_CONTEXT_MESSAGES
            || (!self.collecting_chain && self.last_seen_signatures.is_empty())
    }
}
