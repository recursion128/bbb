use serde::{Deserialize, Serialize};
use uuid::Uuid;

use bbb_protocol::packets::ClientChatVisibility;

use super::{ChatMessageState, ChatSignatureState, ProcessedChatSignatureState};

pub(super) const MAX_VISIBLE_CHAT_MESSAGES: usize = 100;
pub(super) const MAX_DELAYED_CHAT_MESSAGES: usize = 1_024;
pub(super) const MAX_CHAT_REPORT_EVENTS: usize = 1_024;
pub(super) const MAX_PENDING_CHAT_NARRATIONS: usize = 1_024;
pub(super) const CLIENT_CHAT_EXPIRY_MILLIS: i64 = 7 * 60 * 1_000;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum ChatVisibilityState {
    #[default]
    Full,
    System,
    Hidden,
}

impl From<ClientChatVisibility> for ChatVisibilityState {
    fn from(value: ClientChatVisibility) -> Self {
        match value {
            ClientChatVisibility::Full => Self::Full,
            ClientChatVisibility::System => Self::System,
            ClientChatVisibility::Hidden => Self::Hidden,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChatPresentationConfig {
    pub visibility: ChatVisibilityState,
    pub only_show_secure: bool,
    pub hide_matched_names: bool,
    pub message_delay_millis: u64,
    #[serde(default)]
    pub hidden_players: Vec<Uuid>,
    /// Explicit startup-configured blocked profiles. Minecraft Services
    /// profiles are owned separately by `PlayerSocialState`.
    #[serde(default)]
    pub blocked_players: Vec<Uuid>,
}

impl Default for ChatPresentationConfig {
    fn default() -> Self {
        Self {
            visibility: ChatVisibilityState::Full,
            only_show_secure: false,
            hide_matched_names: true,
            message_delay_millis: 0,
            hidden_players: Vec::new(),
            blocked_players: Vec::new(),
        }
    }
}

impl ChatPresentationConfig {
    pub(super) fn can_receive_player_messages(&self) -> bool {
        self.visibility == ChatVisibilityState::Full
    }

    pub(super) fn can_receive_system_messages(&self) -> bool {
        self.visibility != ChatVisibilityState::Hidden
    }

    pub(super) fn normalize_player_sets(&mut self) {
        self.hidden_players.sort_unstable();
        self.hidden_players.dedup();
        self.blocked_players.sort_unstable();
        self.blocked_players.dedup();
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum ChatMessageSourceState {
    Player,
    SystemServer,
    #[default]
    SystemClient,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum ChatTrustLevelState {
    Secure,
    Modified,
    #[default]
    NotSecure,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ChatMessageTagState {
    System,
    SystemSinglePlayer,
    NotSecure,
    Modified { original_content: String },
    Error,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChatNarrationKind {
    Chat,
    SystemChat,
    System,
    SystemNow,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChatNarrationRequest {
    pub sequence: u64,
    pub kind: ChatNarrationKind,
    pub translation_key: Option<String>,
    pub arguments: Vec<super::ChatDecorationArgumentState>,
    #[serde(default)]
    pub content_component: Option<bbb_protocol::NbtSummaryValue>,
    pub styled_content: Vec<bbb_protocol::StyledTextRun>,
    pub content: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChatReportEventKind {
    Player,
    System,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChatReportEventState {
    pub id: u64,
    pub kind: ChatReportEventKind,
    pub sender: Option<Uuid>,
    pub sender_name: Option<String>,
    pub signed_content: Option<String>,
    pub unsigned_content: Option<String>,
    pub displayed_content: String,
    pub received_epoch_millis: i64,
    pub message_timestamp_millis: Option<i64>,
    pub trust_level: Option<ChatTrustLevelState>,
    pub signature: Option<ChatSignatureState>,
    pub filter_mask: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChatReportLogState {
    pub next_id: u64,
    pub events: Vec<ChatReportEventState>,
}

impl Default for ChatReportLogState {
    fn default() -> Self {
        Self {
            next_id: 0,
            events: Vec::new(),
        }
    }
}

impl ChatReportLogState {
    pub(super) fn push(&mut self, mut event: ChatReportEventState) {
        event.id = self.next_id;
        self.next_id = self.next_id.saturating_add(1);
        self.events.push(event);
        if self.events.len() > MAX_CHAT_REPORT_EVENTS {
            let excess = self.events.len() - MAX_CHAT_REPORT_EVENTS;
            self.events.drain(..excess);
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DelayedChatMessageState {
    pub message: ChatMessageState,
    pub only_show_secure: bool,
    pub validation_error: bool,
    pub acknowledge_signature: Option<ProcessedChatSignatureState>,
    pub message_timestamp_millis: i64,
}

impl DelayedChatMessageState {
    pub(super) fn signature_bytes(&self) -> Option<&[u8]> {
        self.message.full_signature.as_deref()
    }
}
