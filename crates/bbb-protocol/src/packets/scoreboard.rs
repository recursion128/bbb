use serde::{Deserialize, Serialize};

use crate::codec::{Decoder, ProtocolError, Result};
pub use crate::objective_render_types::ObjectiveRenderType;
pub use crate::scoreboard_display_slots::ScoreboardDisplaySlot;
pub use crate::scoreboard_methods::{PlayerTeamMethod, SetObjectiveMethod};
pub use crate::scoreboard_team_options::{TeamCollisionRule, TeamVisibility};

use super::{
    decode_component_summary_from_decoder, decode_nullable_string,
    decode_optional_component_summary_from_decoder, decode_optional_trailing_number_format,
    ChatFormatting,
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
    pub method: SetObjectiveMethod,
    pub parameters: Option<SetObjectiveParameters>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SetObjectiveParameters {
    pub display_name: String,
    pub render_type: ObjectiveRenderType,
    pub number_format: Option<Vec<u8>>,
}

impl SetObjectiveMethod {
    fn has_parameters(self) -> bool {
        matches!(self, Self::Add | Self::Change)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SetScore {
    pub owner: String,
    pub objective_name: String,
    pub score: i32,
    pub display: Option<String>,
    pub number_format: Option<Vec<u8>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SetPlayerTeam {
    pub name: String,
    pub method: PlayerTeamMethod,
    pub parameters: Option<PlayerTeamParameters>,
    pub players: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlayerTeamParameters {
    pub display_name: String,
    pub options: u8,
    pub nametag_visibility: TeamVisibility,
    pub collision_rule: TeamCollisionRule,
    pub color: ChatFormatting,
    pub player_prefix: String,
    pub player_suffix: String,
}

impl PlayerTeamMethod {
    fn has_parameters(self) -> bool {
        matches!(self, Self::Add | Self::Change)
    }

    fn has_player_list(self) -> bool {
        matches!(self, Self::Add | Self::Join | Self::Leave)
    }
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
    let method = SetObjectiveMethod::from_id(method_id).ok_or_else(|| {
        ProtocolError::InvalidData(format!("invalid objective method {method_id}"))
    })?;
    let parameters = if method.has_parameters() {
        Some(SetObjectiveParameters {
            display_name: decode_component_summary_from_decoder(decoder)?,
            render_type: {
                let ordinal = decoder.read_var_i32()?;
                ObjectiveRenderType::from_ordinal(ordinal).ok_or_else(|| {
                    ProtocolError::InvalidData(format!("invalid objective render type {ordinal}"))
                })?
            },
            number_format: decode_optional_trailing_number_format(decoder)?,
        })
    } else {
        None
    };

    Ok(SetObjective {
        objective_name,
        method,
        parameters,
    })
}

pub(super) fn decode_set_score(decoder: &mut Decoder<'_>) -> Result<SetScore> {
    Ok(SetScore {
        owner: decoder.read_string(32767)?,
        objective_name: decoder.read_string(32767)?,
        score: decoder.read_var_i32()?,
        display: decode_optional_component_summary_from_decoder(decoder)?,
        number_format: decode_optional_trailing_number_format(decoder)?,
    })
}

pub(super) fn decode_set_player_team(decoder: &mut Decoder<'_>) -> Result<SetPlayerTeam> {
    let name = decoder.read_string(32767)?;
    let method_id = decoder.read_i8()?;
    let method = PlayerTeamMethod::from_id(method_id).ok_or_else(|| {
        ProtocolError::InvalidData(format!("invalid player team method {method_id}"))
    })?;
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

    Ok(SetPlayerTeam {
        name,
        method,
        parameters,
        players,
    })
}

pub(super) fn decode_player_team_parameters(
    decoder: &mut Decoder<'_>,
) -> Result<PlayerTeamParameters> {
    Ok(PlayerTeamParameters {
        display_name: decode_component_summary_from_decoder(decoder)?,
        options: decoder.read_u8()?,
        nametag_visibility: TeamVisibility::from_id(decoder.read_var_i32()?),
        collision_rule: TeamCollisionRule::from_id(decoder.read_var_i32()?),
        color: {
            let ordinal = decoder.read_var_i32()?;
            ChatFormatting::from_ordinal(ordinal).ok_or_else(|| {
                ProtocolError::InvalidData(format!("invalid chat formatting ordinal {ordinal}"))
            })?
        },
        player_prefix: decode_component_summary_from_decoder(decoder)?,
        player_suffix: decode_component_summary_from_decoder(decoder)?,
    })
}

fn decode_player_team_player_list(decoder: &mut Decoder<'_>) -> Result<Vec<String>> {
    let count = decoder.read_len()?;
    if count > MAX_PLAYER_TEAM_PLAYERS {
        return Err(ProtocolError::PacketTooLarge(
            count,
            MAX_PLAYER_TEAM_PLAYERS,
        ));
    }

    let mut players = Vec::with_capacity(count);
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
                method: SetObjectiveMethod::Add,
                parameters: Some(SetObjectiveParameters {
                    display_name: "Kills".to_string(),
                    render_type: ObjectiveRenderType::Hearts,
                    number_format: Some(vec![0]),
                }),
            })
        );

        let mut payload = Encoder::new();
        payload.write_string("kills");
        payload.write_i8(1);

        let packet =
            decode_play_clientbound(ids::play::CLIENTBOUND_SET_OBJECTIVE, &payload.into_inner())
                .unwrap();
        assert_eq!(
            packet,
            PlayClientbound::SetObjective(SetObjective {
                objective_name: "kills".to_string(),
                method: SetObjectiveMethod::Remove,
                parameters: None,
            })
        );
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
    fn rejects_invalid_scoreboard_method_ids() {
        let mut objective = Encoder::new();
        objective.write_string("kills");
        objective.write_i8(3);
        let error = decode_play_clientbound(
            ids::play::CLIENTBOUND_SET_OBJECTIVE,
            &objective.into_inner(),
        )
        .unwrap_err();
        assert!(error.to_string().contains("invalid objective method 3"));

        let mut team = Encoder::new();
        team.write_string("red");
        team.write_i8(5);
        let error =
            decode_play_clientbound(ids::play::CLIENTBOUND_SET_PLAYER_TEAM, &team.into_inner())
                .unwrap_err();
        assert!(error.to_string().contains("invalid player team method 5"));
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
                display: Some("Forty two".to_string()),
                number_format: Some(vec![0]),
            })
        );
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

        let packet = decode_play_clientbound(
            ids::play::CLIENTBOUND_SET_PLAYER_TEAM,
            &payload.into_inner(),
        )
        .unwrap();
        assert_eq!(
            packet,
            PlayClientbound::SetPlayerTeam(SetPlayerTeam {
                name: "red".to_string(),
                method: PlayerTeamMethod::Add,
                parameters: Some(PlayerTeamParameters {
                    display_name: "Red Team".to_string(),
                    options: 0b11,
                    nametag_visibility: TeamVisibility::HideForOtherTeams,
                    collision_rule: TeamCollisionRule::PushOwnTeam,
                    color: ChatFormatting::Red,
                    player_prefix: "[R]".to_string(),
                    player_suffix: "!".to_string(),
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
                method: PlayerTeamMethod::Join,
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
                method: PlayerTeamMethod::Leave,
                parameters: None,
                players: vec!["Sam".to_string()],
            })
        );
    }

    fn nbt_string_root(text: &str) -> Vec<u8> {
        let mut payload = vec![8];
        payload.extend_from_slice(&(text.len() as u16).to_be_bytes());
        payload.extend_from_slice(text.as_bytes());
        payload
    }
}
