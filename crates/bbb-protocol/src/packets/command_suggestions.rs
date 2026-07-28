use serde::{Deserialize, Serialize};

use crate::{
    codec::{Decoder, Encoder, ProtocolError, Result},
    component::{
        decode_styled_component_summary_with_nbt_from_decoder, styled_runs_summary_text,
        StyledTextRun,
    },
    ids, NbtSummaryValue,
};

const MAX_COMMAND_SUGGESTIONS: usize = 8192;
const MAX_COMMAND_SUGGESTION_INPUT_LENGTH: usize = 32_500;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommandSuggestions {
    pub id: i32,
    pub start: i32,
    pub length: i32,
    pub suggestions: Vec<CommandSuggestion>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommandSuggestion {
    pub text: String,
    pub tooltip: Option<String>,
    /// Styled-run projection of `tooltip`; `tooltip` remains its compatible
    /// plain-text concatenation.
    #[serde(default)]
    pub tooltip_styled: Option<Vec<StyledTextRun>>,
    /// Lossless bounded NBT tree for the trusted tooltip Component. This keeps
    /// semantic fields beyond the renderer's current styled-run projection.
    #[serde(default)]
    pub tooltip_component: Option<NbtSummaryValue>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommandSuggestionRequest {
    pub id: i32,
    pub command: String,
}

pub fn encode_play_command_suggestion_request(
    request: CommandSuggestionRequest,
) -> Result<(i32, Vec<u8>)> {
    let command_len = request.command.encode_utf16().count();
    if command_len > MAX_COMMAND_SUGGESTION_INPUT_LENGTH {
        return Err(ProtocolError::StringTooLong {
            actual: command_len,
            max: MAX_COMMAND_SUGGESTION_INPUT_LENGTH,
        });
    }
    let mut out = Encoder::new();
    out.write_var_i32(request.id);
    out.write_string(&request.command);
    Ok((ids::play::SERVERBOUND_COMMAND_SUGGESTION, out.into_inner()))
}

pub(super) fn decode_command_suggestions(decoder: &mut Decoder<'_>) -> Result<CommandSuggestions> {
    let id = decoder.read_var_i32()?;
    let start = decoder.read_var_i32()?;
    let length = decoder.read_var_i32()?;
    let count = decoder.read_len()?;
    if count > MAX_COMMAND_SUGGESTIONS {
        return Err(ProtocolError::PacketTooLarge(
            count,
            MAX_COMMAND_SUGGESTIONS,
        ));
    }

    let mut suggestions =
        Vec::with_capacity(crate::codec::bounded_collection_initial_capacity(count));
    for _ in 0..count {
        let text = decoder.read_string(32767)?;
        let (tooltip_styled, tooltip_component) = if decoder.read_bool()? {
            let (runs, component) = decode_styled_component_summary_with_nbt_from_decoder(decoder)?;
            (Some(runs), component)
        } else {
            (None, None)
        };
        let tooltip = tooltip_styled.as_deref().map(styled_runs_summary_text);
        suggestions.push(CommandSuggestion {
            text,
            tooltip,
            tooltip_styled,
            tooltip_component,
        });
    }

    if !decoder.is_empty() {
        return Err(ProtocolError::InvalidData(
            "trailing bytes after command suggestions packet".to_string(),
        ));
    }

    Ok(CommandSuggestions {
        id,
        start,
        length,
        suggestions,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        codec::{Decoder, Encoder},
        component::decode_component_summary_from_decoder,
        ids,
        packets::{decode_play_clientbound, PlayClientbound},
    };

    #[test]
    fn decodes_command_suggestions_packet_wire_order() {
        let mut payload = Encoder::new();
        payload.write_var_i32(17);
        payload.write_var_i32(1);
        payload.write_var_i32(5);
        payload.write_var_i32(2);
        payload.write_string("give");
        payload.write_bool(true);
        payload.write_bytes(&nbt_styled_text_root("Run give", "red", true));
        payload.write_string("gamemode");
        payload.write_bool(false);
        let payload = payload.into_inner();

        let packet =
            decode_play_clientbound(ids::play::CLIENTBOUND_COMMAND_SUGGESTIONS, &payload).unwrap();
        assert_eq!(
            packet,
            PlayClientbound::CommandSuggestions(CommandSuggestions {
                id: 17,
                start: 1,
                length: 5,
                suggestions: vec![
                    CommandSuggestion {
                        text: "give".to_string(),
                        tooltip: Some("Run give".to_string()),
                        tooltip_styled: Some(vec![StyledTextRun {
                            text: "Run give".to_string(),
                            style: crate::ComponentStyle {
                                bold: Some(true),
                                color: Some(0xff5555),
                                ..crate::ComponentStyle::default()
                            },
                        }]),
                        tooltip_component: Some(crate::NbtSummaryValue::Compound(vec![
                            crate::NbtSummaryEntry {
                                name: "text".to_string(),
                                value: crate::NbtSummaryValue::String("Run give".to_string()),
                            },
                            crate::NbtSummaryEntry {
                                name: "color".to_string(),
                                value: crate::NbtSummaryValue::String("red".to_string()),
                            },
                            crate::NbtSummaryEntry {
                                name: "bold".to_string(),
                                value: crate::NbtSummaryValue::Byte(1),
                            },
                        ])),
                    },
                    CommandSuggestion {
                        text: "gamemode".to_string(),
                        tooltip: None,
                        tooltip_styled: None,
                        tooltip_component: None,
                    },
                ],
            })
        );

        let mut decoder = Decoder::new(&payload);
        assert_eq!(decoder.read_var_i32().unwrap(), 17);
        assert_eq!(decoder.read_var_i32().unwrap(), 1);
        assert_eq!(decoder.read_var_i32().unwrap(), 5);
        assert_eq!(decoder.read_var_i32().unwrap(), 2);
        assert_eq!(decoder.read_string(32767).unwrap(), "give");
        assert!(decoder.read_bool().unwrap());
        assert_eq!(
            decode_component_summary_from_decoder(&mut decoder).unwrap(),
            "Run give"
        );
        assert_eq!(decoder.read_string(32767).unwrap(), "gamemode");
        assert!(!decoder.read_bool().unwrap());
        assert!(decoder.is_empty());

        let mut trailing = payload.clone();
        trailing.push(0);
        assert!(
            decode_play_clientbound(ids::play::CLIENTBOUND_COMMAND_SUGGESTIONS, &trailing)
                .unwrap_err()
                .to_string()
                .contains("trailing bytes after command suggestions packet")
        );
    }

    #[test]
    fn encodes_command_suggestion_request_packet() {
        let (id, payload) = encode_play_command_suggestion_request(CommandSuggestionRequest {
            id: 33,
            command: "/give @p minecraft:stone".to_string(),
        })
        .unwrap();

        assert_eq!(id, ids::play::SERVERBOUND_COMMAND_SUGGESTION);
        assert_eq!(id, 15);
        let mut decoder = Decoder::new(&payload);
        assert_eq!(decoder.read_var_i32().unwrap(), 33);
        assert_eq!(
            decoder.read_string(32500).unwrap(),
            "/give @p minecraft:stone"
        );
        assert!(decoder.is_empty());
    }

    #[test]
    fn command_suggestion_request_rejects_more_than_32500_utf16_code_units() {
        let command = format!("{}a", "😀".repeat(MAX_COMMAND_SUGGESTION_INPUT_LENGTH / 2));
        let error =
            encode_play_command_suggestion_request(CommandSuggestionRequest { id: 33, command })
                .unwrap_err();

        assert!(matches!(
            error,
            ProtocolError::StringTooLong {
                actual: 32_501,
                max: 32_500
            }
        ));
    }

    fn nbt_styled_text_root(text: &str, color: &str, bold: bool) -> Vec<u8> {
        let mut payload = vec![10];
        write_nbt_string_entry(&mut payload, "text", text);
        write_nbt_string_entry(&mut payload, "color", color);
        payload.push(1);
        write_nbt_string(&mut payload, "bold");
        payload.push(u8::from(bold));
        payload.push(0);
        payload
    }

    fn write_nbt_string_entry(payload: &mut Vec<u8>, name: &str, value: &str) {
        payload.push(8);
        write_nbt_string(payload, name);
        write_nbt_string(payload, value);
    }

    fn write_nbt_string(payload: &mut Vec<u8>, value: &str) {
        payload.extend_from_slice(&(value.len() as u16).to_be_bytes());
        payload.extend_from_slice(value.as_bytes());
    }
}
