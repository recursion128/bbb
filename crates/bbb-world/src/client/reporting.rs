use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::WorldStore;

mod evidence;

use evidence::{build_chat_evidence, chat_event_is_reportable};

pub const MAX_REPORT_COMMENT_UTF16_UNITS: usize = 1_000;
pub const MAX_SELECTED_REPORT_MESSAGES: usize = 4;
pub const MAX_REPORT_EVIDENCE_MESSAGES: usize = 40;
pub const LEADING_REPORT_CONTEXT_MESSAGES: usize = 9;
pub const TRAILING_REPORT_CONTEXT_MESSAGES: usize = 0;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PlayerReportType {
    Chat,
    Skin,
    Username,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PlayerReportReason {
    IWantToReportThem,
    HateSpeech,
    HarassmentOrBullying,
    SelfHarmOrSuicide,
    ImminentHarm,
    DefamationImpersonationFalseInformation,
    AlcoholTobaccoDrugs,
    ChildSexualExploitationOrAbuse,
    TerrorismOrViolentExtremism,
    NonConsensualIntimateImagery,
    SexuallyInappropriate,
}

impl PlayerReportReason {
    pub const ALL: [Self; 11] = [
        Self::IWantToReportThem,
        Self::HateSpeech,
        Self::HarassmentOrBullying,
        Self::SelfHarmOrSuicide,
        Self::ImminentHarm,
        Self::DefamationImpersonationFalseInformation,
        Self::AlcoholTobaccoDrugs,
        Self::ChildSexualExploitationOrAbuse,
        Self::TerrorismOrViolentExtremism,
        Self::NonConsensualIntimateImagery,
        Self::SexuallyInappropriate,
    ];

    pub fn backend_name(self) -> &'static str {
        match self {
            Self::IWantToReportThem => "I_WANT_TO_REPORT_THEM",
            Self::HateSpeech => "HATE_SPEECH",
            Self::HarassmentOrBullying => "HARASSMENT_OR_BULLYING",
            Self::SelfHarmOrSuicide => "SELF_HARM_OR_SUICIDE",
            Self::ImminentHarm => "IMMINENT_HARM",
            Self::DefamationImpersonationFalseInformation => {
                "DEFAMATION_IMPERSONATION_FALSE_INFORMATION"
            }
            Self::AlcoholTobaccoDrugs => "ALCOHOL_TOBACCO_DRUGS",
            Self::ChildSexualExploitationOrAbuse => "CHILD_SEXUAL_EXPLOITATION_OR_ABUSE",
            Self::TerrorismOrViolentExtremism => "TERRORISM_OR_VIOLENT_EXTREMISM",
            Self::NonConsensualIntimateImagery => "NON_CONSENSUAL_INTIMATE_IMAGERY",
            Self::SexuallyInappropriate => "SEXUALLY_INAPPROPRIATE",
        }
    }

    pub fn is_compatible_with(self, report_type: PlayerReportType) -> bool {
        !matches!(
            (report_type, self),
            (PlayerReportType::Chat, Self::SexuallyInappropriate)
                | (PlayerReportType::Skin, Self::ImminentHarm)
                | (
                    PlayerReportType::Skin,
                    Self::DefamationImpersonationFalseInformation
                )
        )
    }
}

pub fn player_report_reasons(report_type: PlayerReportType) -> Vec<PlayerReportReason> {
    PlayerReportReason::ALL
        .into_iter()
        .filter(|reason| reason.is_compatible_with(report_type))
        .collect()
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlayerReportEnvironment {
    pub client_version: String,
    pub locale: String,
    pub third_party_server_address: Option<String>,
}

impl PlayerReportEnvironment {
    pub fn local(client_version: impl Into<String>, locale: impl Into<String>) -> Self {
        Self {
            client_version: client_version.into(),
            locale: locale.into(),
            third_party_server_address: None,
        }
    }

    pub fn third_party(
        client_version: impl Into<String>,
        locale: impl Into<String>,
        address: impl Into<String>,
    ) -> Self {
        Self {
            client_version: client_version.into(),
            locale: locale.into(),
            third_party_server_address: Some(address.into()),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlayerReportDraftCommon {
    pub report_id: Uuid,
    pub created_at_epoch_millis: i64,
    pub reported_profile_id: Uuid,
    pub comments: String,
    pub attested: bool,
}

impl PlayerReportDraftCommon {
    fn new(report_id: Uuid, created_at_epoch_millis: i64, reported_profile_id: Uuid) -> Self {
        Self {
            report_id,
            created_at_epoch_millis,
            reported_profile_id,
            comments: String::new(),
            attested: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum PlayerReportDraft {
    Chat {
        common: PlayerReportDraftCommon,
        reason: Option<PlayerReportReason>,
        selected_message_ids: BTreeSet<u64>,
    },
    Skin {
        common: PlayerReportDraftCommon,
        reason: Option<PlayerReportReason>,
        skin_url: Option<String>,
    },
    Username {
        common: PlayerReportDraftCommon,
        reported_name: String,
    },
}

impl PlayerReportDraft {
    pub fn report_type(&self) -> PlayerReportType {
        match self {
            Self::Chat { .. } => PlayerReportType::Chat,
            Self::Skin { .. } => PlayerReportType::Skin,
            Self::Username { .. } => PlayerReportType::Username,
        }
    }

    pub fn common(&self) -> &PlayerReportDraftCommon {
        match self {
            Self::Chat { common, .. }
            | Self::Skin { common, .. }
            | Self::Username { common, .. } => common,
        }
    }

    fn common_mut(&mut self) -> &mut PlayerReportDraftCommon {
        match self {
            Self::Chat { common, .. }
            | Self::Skin { common, .. }
            | Self::Username { common, .. } => common,
        }
    }

    pub fn has_content(&self) -> bool {
        match self {
            Self::Chat {
                common,
                reason,
                selected_message_ids,
            } => {
                !common.comments.is_empty() || reason.is_some() || !selected_message_ids.is_empty()
            }
            Self::Skin { common, reason, .. } => !common.comments.is_empty() || reason.is_some(),
            Self::Username { common, .. } => !common.comments.is_empty(),
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlayerReportingState {
    pub generation: u64,
    pub environment: Option<PlayerReportEnvironment>,
    pub draft: Option<PlayerReportDraft>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum PlayerReportMutationError {
    #[error("player reporting has no active environment")]
    MissingEnvironment,
    #[error("the local or zero profile cannot be reported")]
    InvalidTarget,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum PlayerReportBuildError {
    #[error("no player-report draft exists")]
    NoDraft,
    #[error("player reporting has no active environment")]
    MissingEnvironment,
    #[error("the report id is invalid")]
    InvalidReportId,
    #[error("the reported profile is invalid")]
    InvalidTarget,
    #[error("no chat messages were selected")]
    NoReportedMessages,
    #[error("too many chat messages were selected")]
    TooManyReportedMessages,
    #[error("the selected report reason is missing")]
    NoReason,
    #[error("the selected report reason is incompatible with this report type")]
    IncompatibleReason,
    #[error("the report comment is too long")]
    CommentTooLong,
    #[error("the report was not attested")]
    NotAttested,
    #[error("a selected chat message is no longer reportable")]
    SelectedMessageUnavailable,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BuiltPlayerReport {
    pub id: Uuid,
    pub report_type: PlayerReportType,
    pub comments: String,
    pub reason: Option<PlayerReportReason>,
    pub evidence: Option<Vec<BuiltPlayerReportMessage>>,
    pub skin_url: Option<String>,
    pub reported_profile_id: Uuid,
    pub created_at_epoch_millis: i64,
    pub environment: PlayerReportEnvironment,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BuiltPlayerReportMessage {
    pub index: i32,
    pub profile_id: Uuid,
    pub session_id: Uuid,
    pub timestamp_millis: i64,
    pub salt: i64,
    pub last_seen_signatures: Vec<Vec<u8>>,
    pub message: String,
    pub signature: Option<Vec<u8>>,
    pub message_reported: bool,
}

impl WorldStore {
    pub fn player_reporting(&self) -> &PlayerReportingState {
        &self.player_reporting
    }

    pub fn set_player_report_environment(
        &mut self,
        environment: Option<PlayerReportEnvironment>,
    ) -> bool {
        if self.player_reporting.environment == environment {
            return false;
        }
        let discarded_draft = self.player_reporting.draft.take().is_some();
        self.player_reporting.environment = environment;
        self.advance_player_reporting_generation();
        discarded_draft
    }

    pub fn set_player_report_server_address(&mut self, address: impl Into<String>) -> bool {
        let Some(mut environment) = self.player_reporting.environment.clone() else {
            return false;
        };
        environment.third_party_server_address = Some(address.into());
        self.set_player_report_environment(Some(environment))
    }

    pub fn start_chat_report_draft(
        &mut self,
        report_id: Uuid,
        created_at_epoch_millis: i64,
        reported_profile_id: Uuid,
    ) -> Result<u64, PlayerReportMutationError> {
        self.start_player_report_draft(PlayerReportDraft::Chat {
            common: PlayerReportDraftCommon::new(
                report_id,
                created_at_epoch_millis,
                reported_profile_id,
            ),
            reason: None,
            selected_message_ids: BTreeSet::new(),
        })
    }

    pub fn start_skin_report_draft(
        &mut self,
        report_id: Uuid,
        created_at_epoch_millis: i64,
        reported_profile_id: Uuid,
        skin_url: Option<String>,
    ) -> Result<u64, PlayerReportMutationError> {
        self.start_player_report_draft(PlayerReportDraft::Skin {
            common: PlayerReportDraftCommon::new(
                report_id,
                created_at_epoch_millis,
                reported_profile_id,
            ),
            reason: None,
            skin_url,
        })
    }

    pub fn start_username_report_draft(
        &mut self,
        report_id: Uuid,
        created_at_epoch_millis: i64,
        reported_profile_id: Uuid,
        reported_name: impl Into<String>,
    ) -> Result<u64, PlayerReportMutationError> {
        self.start_player_report_draft(PlayerReportDraft::Username {
            common: PlayerReportDraftCommon::new(
                report_id,
                created_at_epoch_millis,
                reported_profile_id,
            ),
            reported_name: reported_name.into(),
        })
    }

    pub fn set_player_report_comments(&mut self, report_id: Uuid, comments: String) -> bool {
        let Some(draft) = self.player_report_draft_mut(report_id) else {
            return false;
        };
        draft.common_mut().comments = comments;
        true
    }

    pub fn set_player_report_reason(
        &mut self,
        report_id: Uuid,
        reason: PlayerReportReason,
    ) -> bool {
        let Some(draft) = self.player_report_draft_mut(report_id) else {
            return false;
        };
        if !reason.is_compatible_with(draft.report_type()) {
            return false;
        }
        match draft {
            PlayerReportDraft::Chat {
                reason: selected, ..
            }
            | PlayerReportDraft::Skin {
                reason: selected, ..
            } => {
                *selected = Some(reason);
                true
            }
            PlayerReportDraft::Username { .. } => false,
        }
    }

    pub fn set_player_report_attested(&mut self, report_id: Uuid, attested: bool) -> bool {
        let Some(draft) = self.player_report_draft_mut(report_id) else {
            return false;
        };
        draft.common_mut().attested = attested;
        true
    }

    pub fn toggle_chat_report_message(&mut self, report_id: Uuid, message_id: u64) -> bool {
        let Some(PlayerReportDraft::Chat {
            common,
            selected_message_ids,
            ..
        }) = self
            .player_reporting
            .draft
            .as_ref()
            .filter(|draft| draft.common().report_id == report_id)
        else {
            return false;
        };
        if selected_message_ids.contains(&message_id) {
            let Some(PlayerReportDraft::Chat {
                selected_message_ids,
                ..
            }) = self.player_reporting.draft.as_mut()
            else {
                unreachable!("the checked chat draft still exists");
            };
            selected_message_ids.remove(&message_id);
            return true;
        }
        if selected_message_ids.len() >= MAX_SELECTED_REPORT_MESSAGES
            || !chat_event_is_reportable(
                &self.client_chat.report_log,
                message_id,
                common.reported_profile_id,
            )
        {
            return false;
        }
        let Some(PlayerReportDraft::Chat {
            selected_message_ids,
            ..
        }) = self.player_reporting.draft.as_mut()
        else {
            unreachable!("the checked chat draft still exists");
        };
        selected_message_ids.insert(message_id)
    }

    pub fn discard_player_report_draft(&mut self, report_id: Uuid) -> bool {
        if self
            .player_reporting
            .draft
            .as_ref()
            .is_none_or(|draft| draft.common().report_id != report_id)
        {
            return false;
        }
        self.player_reporting.draft = None;
        self.advance_player_reporting_generation();
        true
    }

    pub fn copy_player_report_draft(&self) -> Option<PlayerReportDraft> {
        self.player_reporting.draft.clone()
    }

    pub fn build_player_report(&self) -> Result<BuiltPlayerReport, PlayerReportBuildError> {
        let draft = self
            .player_reporting
            .draft
            .as_ref()
            .ok_or(PlayerReportBuildError::NoDraft)?;
        let environment = self
            .player_reporting
            .environment
            .clone()
            .ok_or(PlayerReportBuildError::MissingEnvironment)?;
        validate_common(draft.common(), self.local_player_profile_id())?;

        let (reason, evidence, skin_url) = match draft {
            PlayerReportDraft::Chat {
                common,
                reason,
                selected_message_ids,
            } => {
                if selected_message_ids.is_empty() {
                    return Err(PlayerReportBuildError::NoReportedMessages);
                }
                if selected_message_ids.len() > MAX_SELECTED_REPORT_MESSAGES {
                    return Err(PlayerReportBuildError::TooManyReportedMessages);
                }
                let reason = validate_reason(*reason, PlayerReportType::Chat)?;
                validate_comment_and_attestation(common)?;
                let evidence = build_chat_evidence(
                    &self.client_chat.report_log,
                    common.reported_profile_id,
                    selected_message_ids,
                )?;
                (Some(reason), Some(evidence), None)
            }
            PlayerReportDraft::Skin {
                common,
                reason,
                skin_url,
            } => {
                let reason = validate_reason(*reason, PlayerReportType::Skin)?;
                validate_comment_and_attestation(common)?;
                (Some(reason), None, skin_url.clone())
            }
            PlayerReportDraft::Username { common, .. } => {
                validate_comment_and_attestation(common)?;
                (None, None, None)
            }
        };
        Ok(BuiltPlayerReport {
            id: draft.common().report_id,
            report_type: draft.report_type(),
            comments: draft.common().comments.clone(),
            reason,
            evidence,
            skin_url,
            reported_profile_id: draft.common().reported_profile_id,
            created_at_epoch_millis: draft.common().created_at_epoch_millis,
            environment,
        })
    }

    fn start_player_report_draft(
        &mut self,
        draft: PlayerReportDraft,
    ) -> Result<u64, PlayerReportMutationError> {
        if self.player_reporting.environment.is_none() {
            return Err(PlayerReportMutationError::MissingEnvironment);
        }
        let target = draft.common().reported_profile_id;
        if target.is_nil() || Some(target) == self.local_player_profile_id() {
            return Err(PlayerReportMutationError::InvalidTarget);
        }
        self.player_reporting.draft = Some(draft);
        self.advance_player_reporting_generation();
        Ok(self.player_reporting.generation)
    }

    fn player_report_draft_mut(&mut self, report_id: Uuid) -> Option<&mut PlayerReportDraft> {
        self.player_reporting
            .draft
            .as_mut()
            .filter(|draft| draft.common().report_id == report_id)
    }

    fn advance_player_reporting_generation(&mut self) {
        self.player_reporting.generation = self.player_reporting.generation.saturating_add(1);
    }
}

fn validate_common(
    common: &PlayerReportDraftCommon,
    local_profile_id: Option<Uuid>,
) -> Result<(), PlayerReportBuildError> {
    if common.report_id.is_nil() {
        return Err(PlayerReportBuildError::InvalidReportId);
    }
    if common.reported_profile_id.is_nil() || Some(common.reported_profile_id) == local_profile_id {
        return Err(PlayerReportBuildError::InvalidTarget);
    }
    Ok(())
}

fn validate_reason(
    reason: Option<PlayerReportReason>,
    report_type: PlayerReportType,
) -> Result<PlayerReportReason, PlayerReportBuildError> {
    let reason = reason.ok_or(PlayerReportBuildError::NoReason)?;
    if !reason.is_compatible_with(report_type) {
        return Err(PlayerReportBuildError::IncompatibleReason);
    }
    Ok(reason)
}

fn validate_comment_and_attestation(
    common: &PlayerReportDraftCommon,
) -> Result<(), PlayerReportBuildError> {
    if common.comments.encode_utf16().count() > MAX_REPORT_COMMENT_UTF16_UNITS {
        return Err(PlayerReportBuildError::CommentTooLong);
    }
    if !common.attested {
        return Err(PlayerReportBuildError::NotAttested);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        ChatReportEventKind, ChatReportEventState, ChatReportSignedMessageState,
        ChatTrustLevelState,
    };

    fn environment(address: &str) -> PlayerReportEnvironment {
        PlayerReportEnvironment::third_party("26.1 (modded)", "en-US", address)
    }

    fn signed_event(
        id: u64,
        sender: Uuid,
        session_id: Uuid,
        index: i32,
        last_seen_signatures: Vec<Vec<u8>>,
    ) -> ChatReportEventState {
        let signature = vec![u8::try_from(id + 1).unwrap(); 4];
        ChatReportEventState {
            id,
            kind: ChatReportEventKind::Player,
            sender: Some(sender),
            sender_name: Some("Reported".to_string()),
            signed_content: Some(format!("message {id}")),
            unsigned_content: None,
            displayed_content: format!("message {id}"),
            received_epoch_millis: i64::try_from(id).unwrap(),
            message_timestamp_millis: Some(i64::try_from(id).unwrap()),
            signed_message: Some(ChatReportSignedMessageState {
                index,
                sender,
                session_id,
                timestamp_millis: i64::try_from(id).unwrap(),
                salt: i64::try_from(id + 10).unwrap(),
                content: format!("message {id}"),
                last_seen_signatures,
                signature: Some(signature.clone()),
            }),
            trust_level: Some(ChatTrustLevelState::Secure),
            signature: None,
            filter_mask: None,
        }
    }

    fn start_chat_draft(store: &mut WorldStore, target: Uuid) -> Uuid {
        let report_id = Uuid::from_u128(100);
        store.set_player_report_environment(Some(environment("example.org:25565")));
        store
            .start_chat_report_draft(report_id, 1_000, target)
            .unwrap();
        report_id
    }

    #[test]
    fn player_reporting_reasons_match_vanilla_type_exclusions() {
        assert!(!player_report_reasons(PlayerReportType::Chat)
            .contains(&PlayerReportReason::SexuallyInappropriate));
        let skin = player_report_reasons(PlayerReportType::Skin);
        assert!(!skin.contains(&PlayerReportReason::ImminentHarm));
        assert!(!skin.contains(&PlayerReportReason::DefamationImpersonationFalseInformation));
        assert_eq!(
            player_report_reasons(PlayerReportType::Username),
            PlayerReportReason::ALL
        );
    }

    #[test]
    fn player_reporting_chat_builder_collects_chain_and_last_seen_oldest_first() {
        let target = Uuid::from_u128(1);
        let other = Uuid::from_u128(2);
        let session = Uuid::from_u128(3);
        let other_session = Uuid::from_u128(4);
        let mut store = WorldStore::new();
        store.client_chat.report_log.events = vec![
            signed_event(0, target, session, 0, vec![]),
            signed_event(1, other, other_session, 0, vec![]),
            signed_event(2, target, session, 1, vec![vec![2; 4]]),
            signed_event(3, target, session, 2, vec![vec![2; 4]]),
        ];
        store.client_chat.report_log.next_id = 4;
        let report_id = start_chat_draft(&mut store, target);
        assert!(store.toggle_chat_report_message(report_id, 3));
        assert!(store.set_player_report_reason(report_id, PlayerReportReason::HarassmentOrBullying));
        assert!(store.set_player_report_attested(report_id, true));

        let built = store.build_player_report().unwrap();
        let evidence = built.evidence.unwrap();
        assert_eq!(
            evidence
                .iter()
                .map(|message| (message.message.as_str(), message.message_reported))
                .collect::<Vec<_>>(),
            vec![
                ("message 0", false),
                ("message 1", false),
                ("message 2", false),
                ("message 3", true),
            ]
        );
    }

    #[test]
    fn player_reporting_builder_validates_every_common_and_chat_failure() {
        let target = Uuid::from_u128(5);
        let session = Uuid::from_u128(6);
        let mut store = WorldStore::new();
        store.client_chat.report_log.events = vec![signed_event(0, target, session, 0, vec![])];
        let report_id = start_chat_draft(&mut store, target);

        assert_eq!(
            store.build_player_report(),
            Err(PlayerReportBuildError::NoReportedMessages)
        );
        assert!(store.toggle_chat_report_message(report_id, 0));
        assert_eq!(
            store.build_player_report(),
            Err(PlayerReportBuildError::NoReason)
        );
        assert!(store.set_player_report_reason(report_id, PlayerReportReason::HateSpeech));
        assert_eq!(
            store.build_player_report(),
            Err(PlayerReportBuildError::NotAttested)
        );
        assert!(store.set_player_report_attested(report_id, true));
        assert!(store.set_player_report_comments(
            report_id,
            "😀".repeat(MAX_REPORT_COMMENT_UTF16_UNITS / 2)
        ));
        assert!(store.build_player_report().is_ok());
        assert!(store.set_player_report_comments(
            report_id,
            format!("{}x", "😀".repeat(MAX_REPORT_COMMENT_UTF16_UNITS / 2))
        ));
        assert_eq!(
            store.build_player_report(),
            Err(PlayerReportBuildError::CommentTooLong)
        );

        let PlayerReportDraft::Chat {
            selected_message_ids,
            ..
        } = store.player_reporting.draft.as_mut().unwrap()
        else {
            unreachable!()
        };
        selected_message_ids.extend(1..=MAX_SELECTED_REPORT_MESSAGES as u64);
        assert_eq!(
            store.build_player_report(),
            Err(PlayerReportBuildError::TooManyReportedMessages)
        );
    }

    #[test]
    fn player_reporting_builder_rejects_restored_invalid_common_and_reason_state() {
        let target = Uuid::from_u128(17);
        let session = Uuid::from_u128(18);
        let mut store = WorldStore::new();
        assert_eq!(
            store.build_player_report(),
            Err(PlayerReportBuildError::NoDraft)
        );

        store.player_reporting.draft = Some(PlayerReportDraft::Username {
            common: PlayerReportDraftCommon {
                report_id: Uuid::from_u128(1),
                created_at_epoch_millis: 0,
                reported_profile_id: target,
                comments: String::new(),
                attested: true,
            },
            reported_name: "Reported".to_string(),
        });
        assert_eq!(
            store.build_player_report(),
            Err(PlayerReportBuildError::MissingEnvironment)
        );

        store.player_reporting.environment = Some(environment("example.org:25565"));
        store
            .player_reporting
            .draft
            .as_mut()
            .unwrap()
            .common_mut()
            .report_id = Uuid::nil();
        assert_eq!(
            store.build_player_report(),
            Err(PlayerReportBuildError::InvalidReportId)
        );
        let common = store.player_reporting.draft.as_mut().unwrap().common_mut();
        common.report_id = Uuid::from_u128(1);
        common.reported_profile_id = Uuid::nil();
        assert_eq!(
            store.build_player_report(),
            Err(PlayerReportBuildError::InvalidTarget)
        );

        store.client_chat.report_log.events = vec![signed_event(0, target, session, 0, vec![])];
        store.player_reporting.draft = Some(PlayerReportDraft::Chat {
            common: PlayerReportDraftCommon {
                report_id: Uuid::from_u128(2),
                created_at_epoch_millis: 0,
                reported_profile_id: target,
                comments: String::new(),
                attested: true,
            },
            reason: Some(PlayerReportReason::SexuallyInappropriate),
            selected_message_ids: BTreeSet::from([0]),
        });
        assert_eq!(
            store.build_player_report(),
            Err(PlayerReportBuildError::IncompatibleReason)
        );
    }

    #[test]
    fn player_reporting_accepts_exact_evidence_limit_and_rejects_stale_selection() {
        let target = Uuid::from_u128(7);
        let mut store = WorldStore::new();
        for group in 0..MAX_SELECTED_REPORT_MESSAGES {
            let session = Uuid::from_u128(8 + group as u128);
            for index in 0..=LEADING_REPORT_CONTEXT_MESSAGES {
                let id =
                    u64::try_from(group * (LEADING_REPORT_CONTEXT_MESSAGES + 1) + index).unwrap();
                store.client_chat.report_log.events.push(signed_event(
                    id,
                    target,
                    session,
                    i32::try_from(index).unwrap(),
                    vec![],
                ));
            }
        }
        let report_id = start_chat_draft(&mut store, target);
        for group in 0..MAX_SELECTED_REPORT_MESSAGES {
            let root =
                u64::try_from((group + 1) * (LEADING_REPORT_CONTEXT_MESSAGES + 1) - 1).unwrap();
            assert!(store.toggle_chat_report_message(report_id, root));
        }
        assert!(store.set_player_report_reason(report_id, PlayerReportReason::HateSpeech));
        assert!(store.set_player_report_attested(report_id, true));
        assert_eq!(
            store.build_player_report().unwrap().evidence.unwrap().len(),
            MAX_REPORT_EVIDENCE_MESSAGES
        );

        store.client_chat.report_log.events.clear();
        assert_eq!(
            store.build_player_report(),
            Err(PlayerReportBuildError::SelectedMessageUnavailable)
        );
    }

    #[test]
    fn player_reporting_sparse_restored_log_ids_remain_bounded() {
        let target = Uuid::from_u128(70);
        let session = Uuid::from_u128(71);
        let mut store = WorldStore::new();
        let mut event = signed_event(0, target, session, 0, vec![]);
        event.id = u64::MAX;
        store.client_chat.report_log.events = vec![event];
        let report_id = start_chat_draft(&mut store, target);
        assert!(store.toggle_chat_report_message(report_id, u64::MAX));
        assert!(store.set_player_report_reason(report_id, PlayerReportReason::HateSpeech));
        assert!(store.set_player_report_attested(report_id, true));

        let evidence = store.build_player_report().unwrap().evidence.unwrap();
        assert_eq!(evidence.len(), 1);
        assert!(evidence[0].message_reported);
    }

    #[test]
    fn player_reporting_skin_username_copy_discard_reset_and_publication_isolate() {
        let target = Uuid::from_u128(9);
        let skin_report_id = Uuid::from_u128(101);
        let mut store = WorldStore::new();
        store.set_player_report_environment(Some(environment("first.example:25565")));
        store
            .start_skin_report_draft(
                skin_report_id,
                2_000,
                target,
                Some("https://textures.example/skin.png".to_string()),
            )
            .unwrap();
        assert!(!store.set_player_report_reason(skin_report_id, PlayerReportReason::ImminentHarm));
        assert!(store.set_player_report_reason(skin_report_id, PlayerReportReason::HateSpeech));
        assert!(store.set_player_report_attested(skin_report_id, true));
        let copy = store.copy_player_report_draft().unwrap();
        let publication = store.publication_snapshot();
        assert_eq!(
            store.build_player_report().unwrap().skin_url.as_deref(),
            Some("https://textures.example/skin.png")
        );
        assert!(store.discard_player_report_draft(skin_report_id));
        assert_eq!(publication.player_reporting().draft.as_ref(), Some(&copy));
        assert!(store.player_reporting().draft.is_none());

        let username_report_id = Uuid::from_u128(102);
        store
            .start_username_report_draft(username_report_id, 3_000, target, "Reported")
            .unwrap();
        assert!(store.set_player_report_attested(username_report_id, true));
        assert!(store.build_player_report().is_ok());
        assert!(store.set_player_report_server_address("second.example:25565"));
        assert_eq!(
            store
                .player_reporting()
                .environment
                .as_ref()
                .and_then(|environment| environment.third_party_server_address.as_deref()),
            Some("second.example:25565")
        );
        assert!(store.player_reporting().draft.is_none());
    }

    #[test]
    fn player_reporting_rejects_offline_local_zero_and_wrong_report_mutations() {
        let mut store = WorldStore::new();
        let local = Uuid::from_u128(10);
        store.set_local_player_profile_id(local);
        assert_eq!(
            store.start_chat_report_draft(Uuid::from_u128(1), 0, Uuid::from_u128(11)),
            Err(PlayerReportMutationError::MissingEnvironment)
        );
        store.set_player_report_environment(Some(environment("example.org:25565")));
        assert_eq!(
            store.start_chat_report_draft(Uuid::from_u128(1), 0, local),
            Err(PlayerReportMutationError::InvalidTarget)
        );
        assert_eq!(
            store.start_chat_report_draft(Uuid::from_u128(1), 0, Uuid::nil()),
            Err(PlayerReportMutationError::InvalidTarget)
        );
        store
            .start_username_report_draft(Uuid::from_u128(2), 0, Uuid::from_u128(12), "Reported")
            .unwrap();
        assert!(!store.set_player_report_comments(Uuid::from_u128(3), "wrong".to_string()));
        assert!(!store.toggle_chat_report_message(Uuid::from_u128(2), 0));
    }
}
