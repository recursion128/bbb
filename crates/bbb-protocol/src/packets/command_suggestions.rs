use serde::{Deserialize, Serialize};

use crate::{
    codec::{Decoder, Encoder, ProtocolError, Result},
    ids,
};

use super::decode_optional_component_summary_from_decoder;

const MAX_COMMAND_SUGGESTIONS: usize = 8192;

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
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommandSuggestionRequest {
    pub id: i32,
    pub command: String,
}

pub fn encode_play_command_suggestion_request(request: CommandSuggestionRequest) -> (i32, Vec<u8>) {
    let mut out = Encoder::new();
    out.write_var_i32(request.id);
    out.write_string(&request.command);
    (ids::play::SERVERBOUND_COMMAND_SUGGESTION, out.into_inner())
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

    let mut suggestions = Vec::with_capacity(count);
    for _ in 0..count {
        suggestions.push(CommandSuggestion {
            text: decoder.read_string(32767)?,
            tooltip: decode_optional_component_summary_from_decoder(decoder)?,
        });
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
        payload.write_bytes(&nbt_string_root("Run give"));
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
                    },
                    CommandSuggestion {
                        text: "gamemode".to_string(),
                        tooltip: None,
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
    }

    #[test]
    fn encodes_command_suggestion_request_packet() {
        let (id, payload) = encode_play_command_suggestion_request(CommandSuggestionRequest {
            id: 33,
            command: "/give @p minecraft:stone".to_string(),
        });

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

    fn nbt_string_root(text: &str) -> Vec<u8> {
        let mut payload = vec![8];
        payload.extend_from_slice(&(text.len() as u16).to_be_bytes());
        payload.extend_from_slice(text.as_bytes());
        payload
    }
}
