use bbb_world::{
    BuiltPlayerReport, BuiltPlayerReportMessage, PlayerReportReason, PlayerReportType,
};
use chrono::{DateTime, SecondsFormat, Utc};

use super::{
    AbuseReport, AbuseReportClientInfo, AbuseReportEvidence, AbuseReportMessage,
    AbuseReportRequest, AbuseReportType, MinecraftUserApiError, ReportByteBuffer, ReportedEntity,
    ThirdPartyServerInfo,
};

impl TryFrom<BuiltPlayerReport> for AbuseReportRequest {
    type Error = MinecraftUserApiError;

    fn try_from(value: BuiltPlayerReport) -> Result<Self, Self::Error> {
        let BuiltPlayerReport {
            id,
            report_type,
            comments,
            reason,
            evidence,
            skin_url,
            reported_profile_id,
            created_at_epoch_millis,
            environment,
        } = value;
        validate_built_report_shape(
            report_type,
            reason,
            evidence.as_deref(),
            skin_url.as_deref(),
        )?;
        Ok(Self {
            version: 1,
            id,
            report: AbuseReport {
                opinion_comments: comments,
                reason: reason
                    .map(PlayerReportReason::backend_name)
                    .map(str::to_string),
                evidence: evidence
                    .map(|messages| {
                        messages
                            .into_iter()
                            .map(abuse_report_message_from_built)
                            .collect::<Result<Vec<_>, _>>()
                    })
                    .transpose()?
                    .map(|messages| AbuseReportEvidence { messages }),
                skin_url,
                reported_entity: ReportedEntity {
                    profile_id: reported_profile_id,
                },
                created_time: format_report_instant(created_at_epoch_millis)?,
            },
            client_info: AbuseReportClientInfo {
                client_version: environment.client_version,
                locale: environment.locale,
            },
            third_party_server_info: environment
                .third_party_server_address
                .map(|address| ThirdPartyServerInfo { address }),
            realm_info: None,
            report_type: match report_type {
                PlayerReportType::Chat => AbuseReportType::Chat,
                PlayerReportType::Skin => AbuseReportType::Skin,
                PlayerReportType::Username => AbuseReportType::Username,
            },
        })
    }
}

fn validate_built_report_shape(
    report_type: PlayerReportType,
    reason: Option<PlayerReportReason>,
    evidence: Option<&[BuiltPlayerReportMessage]>,
    skin_url: Option<&str>,
) -> Result<(), MinecraftUserApiError> {
    let valid = match report_type {
        PlayerReportType::Chat => {
            reason.is_some_and(|reason| reason.is_compatible_with(report_type))
                && evidence.is_some()
                && skin_url.is_none()
        }
        PlayerReportType::Skin => {
            reason.is_some_and(|reason| reason.is_compatible_with(report_type))
                && evidence.is_none()
        }
        PlayerReportType::Username => reason.is_none() && evidence.is_none() && skin_url.is_none(),
    };
    if !valid {
        return Err(MinecraftUserApiError::invalid_request(
            "built abuse-report fields do not match the report type",
        ));
    }
    Ok(())
}

fn abuse_report_message_from_built(
    message: BuiltPlayerReportMessage,
) -> Result<AbuseReportMessage, MinecraftUserApiError> {
    Ok(AbuseReportMessage {
        index: message.index,
        profile_id: message.profile_id,
        session_id: message.session_id,
        timestamp: format_report_instant(message.timestamp_millis)?,
        salt: message.salt,
        last_seen: message
            .last_seen_signatures
            .into_iter()
            .map(ReportByteBuffer)
            .collect(),
        message: message.message,
        signature: message.signature.map(ReportByteBuffer),
        message_reported: message.message_reported,
    })
}

fn format_report_instant(epoch_millis: i64) -> Result<String, MinecraftUserApiError> {
    DateTime::<Utc>::from_timestamp_millis(epoch_millis)
        .map(|timestamp| timestamp.to_rfc3339_opts(SecondsFormat::AutoSi, true))
        .ok_or_else(|| {
            MinecraftUserApiError::invalid_request("abuse-report timestamp is out of range")
        })
}
