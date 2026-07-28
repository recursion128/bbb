use serde::{Deserialize, Serialize};

use crate::codec::{Decoder, ProtocolError, Result};
use crate::number_format_types::{
    VANILLA_NUMBER_FORMAT_TYPE_BLANK_ID, VANILLA_NUMBER_FORMAT_TYPE_FIXED_ID,
    VANILLA_NUMBER_FORMAT_TYPE_STYLED_ID,
};
pub use crate::objective_render_types::ObjectiveRenderType;
pub use crate::scoreboard_display_slots::ScoreboardDisplaySlot;
pub use crate::scoreboard_methods::{PlayerTeamMethod, SetObjectiveMethod};
pub use crate::scoreboard_team_options::{TeamCollisionRule, TeamVisibility};
use crate::{RichTextComponent, RichTextStyle};

use super::{decode_nullable_string, ChatFormatting};
use crate::component::{
    decode_rich_text_component_from_decoder, decode_rich_text_style_from_decoder,
};

const MAX_PLAYER_TEAM_PLAYERS: usize = 8192;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResetScore {
    pub owner: String,
    pub objective_name: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SetDisplayObjective {
    pub slot: ScoreboardDisplaySlot,
    pub objective_name: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SetObjective {
    pub objective_name: String,
    pub method: SetObjectiveOperation,
    pub parameters: Option<SetObjectiveParameters>,
}

/// Vanilla preserves the signed method byte even when it is outside the three
/// known objective operations. Unknown values carry no parameter payload and
/// become handler no-ops.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct SetObjectiveOperation(i8);

impl SetObjectiveOperation {
    pub const fn from_raw(raw: i8) -> Self {
        Self(raw)
    }

    pub const fn raw(self) -> i8 {
        self.0
    }

    pub fn known(self) -> Option<SetObjectiveMethod> {
        SetObjectiveMethod::from_id(self.0)
    }

    fn has_parameters(self) -> bool {
        matches!(
            self.known(),
            Some(SetObjectiveMethod::Add | SetObjectiveMethod::Change)
        )
    }
}

impl From<SetObjectiveMethod> for SetObjectiveOperation {
    fn from(method: SetObjectiveMethod) -> Self {
        Self(method.id())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SetObjectiveParameters {
    pub display_name: RichTextComponent,
    pub render_type: ObjectiveRenderType,
    pub number_format: Option<NumberFormatSummary>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SetScore {
    pub owner: String,
    pub objective_name: String,
    pub score: i32,
    pub display: Option<RichTextComponent>,
    pub number_format: Option<NumberFormatSummary>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SetPlayerTeam {
    pub name: String,
    pub method: PlayerTeamOperation,
    pub parameters: Option<PlayerTeamParameters>,
    pub players: Vec<String>,
}

/// Vanilla preserves the signed method byte even when it is outside the five
/// known team operations. Unknown values carry neither parameters nor players
/// and become handler no-ops.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct PlayerTeamOperation(i8);

impl PlayerTeamOperation {
    pub const fn from_raw(raw: i8) -> Self {
        Self(raw)
    }

    pub const fn raw(self) -> i8 {
        self.0
    }

    pub fn known(self) -> Option<PlayerTeamMethod> {
        PlayerTeamMethod::from_id(self.0)
    }

    fn has_parameters(self) -> bool {
        matches!(
            self.known(),
            Some(PlayerTeamMethod::Add | PlayerTeamMethod::Change)
        )
    }

    fn has_player_list(self) -> bool {
        matches!(
            self.known(),
            Some(PlayerTeamMethod::Add | PlayerTeamMethod::Join | PlayerTeamMethod::Leave)
        )
    }
}

impl From<PlayerTeamMethod> for PlayerTeamOperation {
    fn from(method: PlayerTeamMethod) -> Self {
        Self(method.id())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlayerTeamParameters {
    pub display_name: RichTextComponent,
    pub options: u8,
    pub nametag_visibility: TeamVisibility,
    pub collision_rule: TeamCollisionRule,
    pub color: ChatFormatting,
    pub player_prefix: RichTextComponent,
    pub player_suffix: RichTextComponent,
}

/// The complete client-visible payload of vanilla's three built-in
/// `NumberFormatTypes`, whose wire discriminants follow the generated bootstrap
/// registry order.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", content = "value", rename_all = "snake_case")]
pub enum NumberFormatSummary {
    Blank,
    Styled(RichTextStyle),
    Fixed(RichTextComponent),
}

pub(super) fn decode_reset_score(decoder: &mut Decoder<'_>) -> Result<ResetScore> {
    let packet = ResetScore {
        owner: decoder.read_string(32767)?,
        objective_name: decode_nullable_string(decoder)?,
    };
    if !decoder.is_empty() {
        return Err(ProtocolError::InvalidData(
            "trailing bytes after reset score packet".to_string(),
        ));
    }
    Ok(packet)
}

pub(super) fn decode_set_display_objective(
    decoder: &mut Decoder<'_>,
) -> Result<SetDisplayObjective> {
    let slot = ScoreboardDisplaySlot::from_id(decoder.read_var_i32()?);
    let objective_name = decoder.read_string(32767)?;
    let packet = SetDisplayObjective {
        slot,
        objective_name: (!objective_name.is_empty()).then_some(objective_name),
    };
    if !decoder.is_empty() {
        return Err(ProtocolError::InvalidData(
            "trailing bytes after set display objective packet".to_string(),
        ));
    }
    Ok(packet)
}

pub(super) fn decode_set_objective(decoder: &mut Decoder<'_>) -> Result<SetObjective> {
    let objective_name = decoder.read_string(32767)?;
    let method_id = decoder.read_i8()?;
    let method = SetObjectiveOperation::from_raw(method_id);
    let parameters = if method.has_parameters() {
        Some(SetObjectiveParameters {
            display_name: decode_rich_text_component_from_decoder(decoder)?,
            render_type: {
                let ordinal = decoder.read_var_i32()?;
                ObjectiveRenderType::from_ordinal(ordinal).ok_or_else(|| {
                    ProtocolError::InvalidData(format!("invalid objective render type {ordinal}"))
                })?
            },
            number_format: decode_optional_number_format(decoder)?,
        })
    } else {
        None
    };

    let packet = SetObjective {
        objective_name,
        method,
        parameters,
    };
    if !decoder.is_empty() {
        return Err(ProtocolError::InvalidData(
            "trailing bytes after set objective packet".to_string(),
        ));
    }
    Ok(packet)
}

pub(super) fn decode_set_score(decoder: &mut Decoder<'_>) -> Result<SetScore> {
    let packet = SetScore {
        owner: decoder.read_string(32767)?,
        objective_name: decoder.read_string(32767)?,
        score: decoder.read_var_i32()?,
        display: decode_optional_rich_text_component(decoder)?,
        number_format: decode_optional_number_format(decoder)?,
    };
    if !decoder.is_empty() {
        return Err(ProtocolError::InvalidData(
            "trailing bytes after set score packet".to_string(),
        ));
    }
    Ok(packet)
}

pub(super) fn decode_set_player_team(decoder: &mut Decoder<'_>) -> Result<SetPlayerTeam> {
    let name = decoder.read_string(32767)?;
    let method_id = decoder.read_i8()?;
    let method = PlayerTeamOperation::from_raw(method_id);
    let parameters = if method.has_parameters() {
        Some(decode_player_team_parameters(decoder)?)
    } else {
        None
    };
    let players = if method.has_player_list() {
        decode_player_team_player_list(decoder)?
    } else {
        Vec::new()
    };

    let packet = SetPlayerTeam {
        name,
        method,
        parameters,
        players,
    };
    if !decoder.is_empty() {
        return Err(ProtocolError::InvalidData(
            "trailing bytes after set player team packet".to_string(),
        ));
    }
    Ok(packet)
}

pub(super) fn decode_player_team_parameters(
    decoder: &mut Decoder<'_>,
) -> Result<PlayerTeamParameters> {
    Ok(PlayerTeamParameters {
        display_name: decode_rich_text_component_from_decoder(decoder)?,
        options: decoder.read_u8()?,
        nametag_visibility: TeamVisibility::from_id(decoder.read_var_i32()?),
        collision_rule: TeamCollisionRule::from_id(decoder.read_var_i32()?),
        color: {
            let ordinal = decoder.read_var_i32()?;
            ChatFormatting::from_ordinal(ordinal).ok_or_else(|| {
                ProtocolError::InvalidData(format!("invalid chat formatting ordinal {ordinal}"))
            })?
        },
        player_prefix: decode_rich_text_component_from_decoder(decoder)?,
        player_suffix: decode_rich_text_component_from_decoder(decoder)?,
    })
}

fn decode_optional_rich_text_component(
    decoder: &mut Decoder<'_>,
) -> Result<Option<RichTextComponent>> {
    decoder
        .read_bool()?
        .then(|| decode_rich_text_component_from_decoder(decoder))
        .transpose()
}

fn decode_optional_number_format(decoder: &mut Decoder<'_>) -> Result<Option<NumberFormatSummary>> {
    decoder
        .read_bool()?
        .then(|| decode_number_format(decoder))
        .transpose()
}

fn decode_number_format(decoder: &mut Decoder<'_>) -> Result<NumberFormatSummary> {
    let type_id = decoder.read_var_i32()?;
    match type_id {
        VANILLA_NUMBER_FORMAT_TYPE_BLANK_ID => Ok(NumberFormatSummary::Blank),
        VANILLA_NUMBER_FORMAT_TYPE_STYLED_ID => Ok(NumberFormatSummary::Styled(
            decode_rich_text_style_from_decoder(decoder)?,
        )),
        VANILLA_NUMBER_FORMAT_TYPE_FIXED_ID => Ok(NumberFormatSummary::Fixed(
            decode_rich_text_component_from_decoder(decoder)?,
        )),
        _ => Err(ProtocolError::InvalidData(format!(
            "unknown number format type id {type_id}"
        ))),
    }
}

fn decode_player_team_player_list(decoder: &mut Decoder<'_>) -> Result<Vec<String>> {
    let count = decoder.read_len()?;
    if count > MAX_PLAYER_TEAM_PLAYERS {
        return Err(ProtocolError::PacketTooLarge(
            count,
            MAX_PLAYER_TEAM_PLAYERS,
        ));
    }

    let mut players = Vec::with_capacity(crate::codec::bounded_collection_initial_capacity(count));
    for _ in 0..count {
        players.push(decoder.read_string(32767)?);
    }
    Ok(players)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        codec::Encoder,
        ids,
        packets::{decode_play_clientbound, ChatFormatting, PlayClientbound},
    };

    #[test]
    fn decodes_scoreboard_display_objective_packet() {
        let mut payload = Encoder::new();
        payload.write_var_i32(1);
        payload.write_string("");
        let mut bytes = payload.into_inner();

        let packet =
            decode_play_clientbound(ids::play::CLIENTBOUND_SET_DISPLAY_OBJECTIVE, &bytes).unwrap();
        assert_eq!(
            packet,
            PlayClientbound::SetDisplayObjective(SetDisplayObjective {
                slot: ScoreboardDisplaySlot::Sidebar,
                objective_name: None,
            })
        );

        bytes.push(0);
        let error = decode_play_clientbound(ids::play::CLIENTBOUND_SET_DISPLAY_OBJECTIVE, &bytes)
            .unwrap_err();
        assert!(error
            .to_string()
            .contains("trailing bytes after set display objective packet"));
    }

    #[test]
    fn decodes_scoreboard_set_objective_add_and_remove_packets() {
        let mut payload = Encoder::new();
        payload.write_string("kills");
        payload.write_i8(0);
        payload.write_bytes(&nbt_string_root("Kills"));
        payload.write_var_i32(1);
        payload.write_bool(true);
        payload.write_var_i32(0);

        let packet =
            decode_play_clientbound(ids::play::CLIENTBOUND_SET_OBJECTIVE, &payload.into_inner())
                .unwrap();
        assert_eq!(
            packet,
            PlayClientbound::SetObjective(SetObjective {
                objective_name: "kills".to_string(),
                method: SetObjectiveMethod::Add.into(),
                parameters: Some(SetObjectiveParameters {
                    display_name: RichTextComponent::plain("Kills"),
                    render_type: ObjectiveRenderType::Hearts,
                    number_format: Some(NumberFormatSummary::Blank),
                }),
            })
        );

        let mut payload = Encoder::new();
        payload.write_string("kills");
        payload.write_i8(1);
        let mut bytes = payload.into_inner();

        let packet = decode_play_clientbound(ids::play::CLIENTBOUND_SET_OBJECTIVE, &bytes).unwrap();
        assert_eq!(
            packet,
            PlayClientbound::SetObjective(SetObjective {
                objective_name: "kills".to_string(),
                method: SetObjectiveMethod::Remove.into(),
                parameters: None,
            })
        );
        bytes.push(0);
        let error =
            decode_play_clientbound(ids::play::CLIENTBOUND_SET_OBJECTIVE, &bytes).unwrap_err();
        assert!(error
            .to_string()
            .contains("trailing bytes after set objective packet"));
    }

    #[test]
    fn rejects_invalid_scoreboard_objective_render_type() {
        let mut payload = Encoder::new();
        payload.write_string("kills");
        payload.write_i8(0);
        payload.write_bytes(&nbt_string_root("Kills"));
        payload.write_var_i32(2);

        let error =
            decode_play_clientbound(ids::play::CLIENTBOUND_SET_OBJECTIVE, &payload.into_inner())
                .unwrap_err();
        assert!(error
            .to_string()
            .contains("invalid objective render type 2"));
    }

    #[test]
    fn preserves_unknown_scoreboard_method_ids_without_conditional_payloads() {
        let mut objective = Encoder::new();
        objective.write_string("kills");
        objective.write_i8(3);
        let packet = decode_play_clientbound(
            ids::play::CLIENTBOUND_SET_OBJECTIVE,
            &objective.into_inner(),
        )
        .unwrap();
        assert_eq!(
            packet,
            PlayClientbound::SetObjective(SetObjective {
                objective_name: "kills".to_string(),
                method: SetObjectiveOperation::from_raw(3),
                parameters: None,
            })
        );

        let mut team = Encoder::new();
        team.write_string("red");
        team.write_i8(-1);
        let packet =
            decode_play_clientbound(ids::play::CLIENTBOUND_SET_PLAYER_TEAM, &team.into_inner())
                .unwrap();
        assert_eq!(
            packet,
            PlayClientbound::SetPlayerTeam(SetPlayerTeam {
                name: "red".to_string(),
                method: PlayerTeamOperation::from_raw(-1),
                parameters: None,
                players: Vec::new(),
            })
        );
    }

    #[test]
    fn unknown_scoreboard_methods_still_reject_trailing_payloads() {
        let mut objective = Encoder::new();
        objective.write_string("kills");
        objective.write_i8(3);
        objective.write_u8(0);
        let error = decode_play_clientbound(
            ids::play::CLIENTBOUND_SET_OBJECTIVE,
            &objective.into_inner(),
        )
        .unwrap_err();
        assert!(error
            .to_string()
            .contains("trailing bytes after set objective packet"));

        let mut team = Encoder::new();
        team.write_string("red");
        team.write_i8(5);
        team.write_u8(0);
        let error =
            decode_play_clientbound(ids::play::CLIENTBOUND_SET_PLAYER_TEAM, &team.into_inner())
                .unwrap_err();
        assert!(error
            .to_string()
            .contains("trailing bytes after set player team packet"));
    }

    #[test]
    fn decodes_scoreboard_set_score_with_optional_display_and_number_format() {
        let mut payload = Encoder::new();
        payload.write_string("Steve");
        payload.write_string("kills");
        payload.write_var_i32(42);
        payload.write_bool(true);
        payload.write_bytes(&nbt_string_root("Forty two"));
        payload.write_bool(true);
        payload.write_var_i32(0);

        let packet =
            decode_play_clientbound(ids::play::CLIENTBOUND_SET_SCORE, &payload.into_inner())
                .unwrap();
        assert_eq!(
            packet,
            PlayClientbound::SetScore(SetScore {
                owner: "Steve".to_string(),
                objective_name: "kills".to_string(),
                score: 42,
                display: Some(RichTextComponent::plain("Forty two")),
                number_format: Some(NumberFormatSummary::Blank),
            })
        );

        let mut trailing = Encoder::new();
        trailing.write_string("Steve");
        trailing.write_string("kills");
        trailing.write_var_i32(42);
        trailing.write_bool(false);
        trailing.write_bool(false);
        trailing.write_u8(0);
        let error =
            decode_play_clientbound(ids::play::CLIENTBOUND_SET_SCORE, &trailing.into_inner())
                .unwrap_err();
        assert!(error
            .to_string()
            .contains("trailing bytes after set score packet"));
    }

    #[test]
    fn decodes_all_scoreboard_number_format_payloads() {
        let mut styled = Encoder::new();
        styled.write_string("Steve");
        styled.write_string("kills");
        styled.write_var_i32(42);
        styled.write_bool(false);
        styled.write_bool(true);
        styled.write_var_i32(VANILLA_NUMBER_FORMAT_TYPE_STYLED_ID);
        styled.write_bytes(&nbt_style_root("red", true));
        let packet =
            decode_play_clientbound(ids::play::CLIENTBOUND_SET_SCORE, &styled.into_inner())
                .unwrap();
        let PlayClientbound::SetScore(packet) = packet else {
            panic!("expected set score packet");
        };
        let Some(NumberFormatSummary::Styled(style)) = packet.number_format else {
            panic!("expected styled number format");
        };
        assert_eq!(
            style.style(),
            &crate::ComponentStyle {
                bold: Some(true),
                color: Some(0xFF_55_55),
                ..crate::ComponentStyle::default()
            }
        );
        assert!(matches!(
            style.component(),
            crate::NbtSummaryValue::Compound(entries)
                if entries.iter().any(|entry| {
                    entry.name == "color"
                        && entry.value == crate::NbtSummaryValue::String("red".to_string())
                })
        ));

        let mut fixed = Encoder::new();
        fixed.write_string("Steve");
        fixed.write_string("kills");
        fixed.write_var_i32(42);
        fixed.write_bool(false);
        fixed.write_bool(true);
        fixed.write_var_i32(VANILLA_NUMBER_FORMAT_TYPE_FIXED_ID);
        fixed.write_bytes(&nbt_string_root("forty-two"));
        let packet =
            decode_play_clientbound(ids::play::CLIENTBOUND_SET_SCORE, &fixed.into_inner()).unwrap();
        let PlayClientbound::SetScore(packet) = packet else {
            panic!("expected set score packet");
        };
        assert_eq!(
            packet.number_format,
            Some(NumberFormatSummary::Fixed(RichTextComponent::plain(
                "forty-two"
            )))
        );
    }

    #[test]
    fn rejects_unknown_scoreboard_number_format_type() {
        let mut payload = Encoder::new();
        payload.write_string("Steve");
        payload.write_string("kills");
        payload.write_var_i32(42);
        payload.write_bool(false);
        payload.write_bool(true);
        payload.write_var_i32(3);

        let error =
            decode_play_clientbound(ids::play::CLIENTBOUND_SET_SCORE, &payload.into_inner())
                .unwrap_err();
        assert!(error
            .to_string()
            .contains("unknown number format type id 3"));
    }

    #[test]
    fn decodes_scoreboard_reset_score_null_and_objective_packets() {
        let mut payload = Encoder::new();
        payload.write_string("Steve");
        payload.write_bool(false);

        let packet =
            decode_play_clientbound(ids::play::CLIENTBOUND_RESET_SCORE, &payload.into_inner())
                .unwrap();
        assert_eq!(
            packet,
            PlayClientbound::ResetScore(ResetScore {
                owner: "Steve".to_string(),
                objective_name: None,
            })
        );

        let mut payload = Encoder::new();
        payload.write_string("Alex");
        payload.write_bool(true);
        payload.write_string("kills");
        let mut bytes = payload.into_inner();

        let packet = decode_play_clientbound(ids::play::CLIENTBOUND_RESET_SCORE, &bytes).unwrap();
        assert_eq!(
            packet,
            PlayClientbound::ResetScore(ResetScore {
                owner: "Alex".to_string(),
                objective_name: Some("kills".to_string()),
            })
        );
        bytes.push(0);
        let error =
            decode_play_clientbound(ids::play::CLIENTBOUND_RESET_SCORE, &bytes).unwrap_err();
        assert!(error
            .to_string()
            .contains("trailing bytes after reset score packet"));
    }

    #[test]
    fn decodes_scoreboard_player_team_add_join_and_leave_packets() {
        let mut payload = Encoder::new();
        payload.write_string("red");
        payload.write_i8(0);
        payload.write_bytes(&nbt_string_root("Red Team"));
        payload.write_u8(0b11);
        payload.write_var_i32(2);
        payload.write_var_i32(3);
        payload.write_var_i32(12);
        payload.write_bytes(&nbt_string_root("[R]"));
        payload.write_bytes(&nbt_string_root("!"));
        payload.write_var_i32(2);
        payload.write_string("Steve");
        payload.write_string("Alex");

        let mut bytes = payload.into_inner();
        let packet =
            decode_play_clientbound(ids::play::CLIENTBOUND_SET_PLAYER_TEAM, &bytes).unwrap();
        assert_eq!(
            packet,
            PlayClientbound::SetPlayerTeam(SetPlayerTeam {
                name: "red".to_string(),
                method: PlayerTeamMethod::Add.into(),
                parameters: Some(PlayerTeamParameters {
                    display_name: RichTextComponent::plain("Red Team"),
                    options: 0b11,
                    nametag_visibility: TeamVisibility::HideForOtherTeams,
                    collision_rule: TeamCollisionRule::PushOwnTeam,
                    color: ChatFormatting::Red,
                    player_prefix: RichTextComponent::plain("[R]"),
                    player_suffix: RichTextComponent::plain("!"),
                }),
                players: vec!["Steve".to_string(), "Alex".to_string()],
            })
        );

        let mut payload = Encoder::new();
        payload.write_string("red");
        payload.write_i8(3);
        payload.write_var_i32(1);
        payload.write_string("Sam");

        let packet = decode_play_clientbound(
            ids::play::CLIENTBOUND_SET_PLAYER_TEAM,
            &payload.into_inner(),
        )
        .unwrap();
        assert_eq!(
            packet,
            PlayClientbound::SetPlayerTeam(SetPlayerTeam {
                name: "red".to_string(),
                method: PlayerTeamMethod::Join.into(),
                parameters: None,
                players: vec!["Sam".to_string()],
            })
        );

        let mut payload = Encoder::new();
        payload.write_string("red");
        payload.write_i8(4);
        payload.write_var_i32(1);
        payload.write_string("Sam");

        let packet = decode_play_clientbound(
            ids::play::CLIENTBOUND_SET_PLAYER_TEAM,
            &payload.into_inner(),
        )
        .unwrap();
        assert_eq!(
            packet,
            PlayClientbound::SetPlayerTeam(SetPlayerTeam {
                name: "red".to_string(),
                method: PlayerTeamMethod::Leave.into(),
                parameters: None,
                players: vec!["Sam".to_string()],
            })
        );

        bytes.push(0);
        let error =
            decode_play_clientbound(ids::play::CLIENTBOUND_SET_PLAYER_TEAM, &bytes).unwrap_err();
        assert!(error
            .to_string()
            .contains("trailing bytes after set player team packet"));
    }

    fn nbt_string_root(text: &str) -> Vec<u8> {
        let mut payload = vec![8];
        payload.extend_from_slice(&(text.len() as u16).to_be_bytes());
        payload.extend_from_slice(text.as_bytes());
        payload
    }

    fn nbt_style_root(color: &str, bold: bool) -> Vec<u8> {
        let mut payload = vec![10];
        payload.push(8);
        write_nbt_string(&mut payload, "color");
        write_nbt_string(&mut payload, color);
        payload.push(1);
        write_nbt_string(&mut payload, "bold");
        payload.push(u8::from(bold));
        payload.push(0);
        payload
    }

    fn write_nbt_string(payload: &mut Vec<u8>, value: &str) {
        payload.extend_from_slice(&(value.len() as u16).to_be_bytes());
        payload.extend_from_slice(value.as_bytes());
    }
}
