use serde::{Deserialize, Serialize};

use crate::{
    codec::{Decoder, Encoder, ProtocolError, Result},
    component::{
        decode_styled_component_summary_with_nbt_from_decoder_with_registry,
        styled_runs_summary_text, StyledTextRun,
    },
    content_registry::ContentRegistry,
    ids,
    nbt_summary::{encode_nbt_summary_bounded, MAX_NBT_DEPTH},
    NbtSummaryValue,
};

const MAX_COMMAND_SUGGESTIONS: usize = 8192;
const MAX_COMMAND_SUGGESTION_INPUT_LENGTH: usize = 32_500;
/// Same trusted-component budget `COMPONENT_NBT_BYTES` in `server_play.rs` uses.
const TOOLTIP_NBT_BYTES: usize = crate::nbt_summary::VANILLA_DEFAULT_NBT_QUOTA as usize;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommandSuggestions {
    pub id: i32,
    pub start: i32,
    pub length: i32,
    pub suggestions: Vec<CommandSuggestion>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommandSuggestion {
    pub text: crate::JavaString,
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
pub struct CommandSuggestionRequest<S = String> {
    pub id: i32,
    pub command: S,
}

pub fn encode_play_command_suggestion_request<S: Into<crate::JavaString>>(
    request: CommandSuggestionRequest<S>,
) -> Result<(i32, Vec<u8>)> {
    let command: crate::JavaString = request.command.into();
    let command_len = command.units().len();
    if command_len > MAX_COMMAND_SUGGESTION_INPUT_LENGTH {
        return Err(ProtocolError::StringTooLong {
            actual: command_len,
            max: MAX_COMMAND_SUGGESTION_INPUT_LENGTH,
        });
    }
    let mut out = Encoder::new();
    out.write_var_i32(request.id);
    out.write_utf16_string(command.units());
    Ok((ids::play::SERVERBOUND_COMMAND_SUGGESTION, out.into_inner()))
}

/// The decoding constructor `ServerboundCommandSuggestionPacket.STREAM_CODEC`
/// names (ServerboundCommandSuggestionPacket.java:20-23): the transaction id
/// the answer echoes, and the partial command the client wants completed.
pub(super) fn decode_command_suggestion_request(
    decoder: &mut Decoder<'_>,
) -> Result<CommandSuggestionRequest> {
    Ok(CommandSuggestionRequest {
        id: decoder.read_var_i32()?,
        command: decoder.read_string(MAX_COMMAND_SUGGESTION_INPUT_LENGTH)?,
    })
}

/// `ClientboundCommandSuggestionsPacket.STREAM_CODEC`
/// (ClientboundCommandSuggestionsPacket.java:19-29), whose per-entry codec
/// (`Entry.STREAM_CODEC`, :62-68) is the text followed by the tooltip through
/// `ComponentSerialization.TRUSTED_OPTIONAL_STREAM_CODEC` — a present flag and,
/// when set, the Component NBT.
///
/// Most suggestions have no tooltip, because `SuggestionsBuilder.suggest(String)`
/// (SuggestionsBuilder.java:54-61) leaves it null and that is the overload
/// `LiteralCommandNode.listSuggestions` (LiteralCommandNode.java:67-69) and the
/// `SharedSuggestionProvider.suggest` name filters all use. The tooltipped
/// overload (SuggestionsBuilder.java:63-70) is reached by
/// `EntitySelectorParser.fillSelectorSuggestions`
/// (EntitySelectorParser.java:475-482), which is why this writes the payload
/// rather than refusing it.
///
/// [`CommandSuggestion::tooltip_component`] is the value on the wire, because
/// `Entry` carries exactly one tooltip and it is a Component
/// (ClientboundCommandSuggestionsPacket.java:61-68). The other two tooltip
/// fields are projections the decoder derives from that Component, so a
/// suggestion carrying one of them without the Component describes a tooltip
/// this encoder has no field to put anywhere. Writing it as a tooltipless entry
/// would drop it in silence, so it is refused instead — the same answer the
/// caller would get for any other unrepresentable value.
pub fn encode_play_clientbound_command_suggestions(
    update: &CommandSuggestions,
) -> Result<(i32, Vec<u8>)> {
    if update.suggestions.len() > MAX_COMMAND_SUGGESTIONS {
        return Err(ProtocolError::PacketTooLarge(
            update.suggestions.len(),
            MAX_COMMAND_SUGGESTIONS,
        ));
    }
    let mut out = Encoder::new();
    out.write_var_i32(update.id);
    out.write_var_i32(update.start);
    out.write_var_i32(update.length);
    out.write_var_i32(update.suggestions.len() as i32);
    for suggestion in &update.suggestions {
        out.write_java_string(&suggestion.text, 32767)?;
        match &suggestion.tooltip_component {
            Some(tooltip) => {
                out.write_bool(true);
                out.write_bytes(&encode_nbt_summary_bounded(
                    Some(tooltip),
                    TOOLTIP_NBT_BYTES,
                    MAX_NBT_DEPTH,
                )?);
            }
            None if suggestion.tooltip.is_some() || suggestion.tooltip_styled.is_some() => {
                return Err(ProtocolError::InvalidData(format!(
                    "command suggestion {:?} carries a tooltip projection without the component it \
                     projects",
                    suggestion.text
                )));
            }
            None => out.write_bool(false),
        }
    }
    Ok((ids::play::CLIENTBOUND_COMMAND_SUGGESTIONS, out.into_inner()))
}

pub(super) fn decode_command_suggestions_with_registry(
    decoder: &mut Decoder<'_>,
    registry: &ContentRegistry,
) -> Result<CommandSuggestions> {
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

    let mut suggestions = Vec::with_capacity(decoder.bounded_capacity(count, 2));
    for _ in 0..count {
        let text = decoder.read_java_string(32767)?;
        let (tooltip_styled, tooltip_component) = if decoder.read_bool()? {
            let (runs, component) =
                decode_styled_component_summary_with_nbt_from_decoder_with_registry(
                    decoder, registry,
                )?;
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
    #[test]
    fn chat_wire_suggestion_text_uses_java_replacement() {
        let mut out = Encoder::new();
        for value in [7, 0, 1, 1, 3] {
            out.write_var_i32(value);
        }
        out.write_bytes(&[0xed, 0xa0, 0x80]);
        out.write_bool(false);
        let payload = out.into_inner();
        let packet = crate::packets::decode_play_clientbound(
            ids::play::CLIENTBOUND_COMMAND_SUGGESTIONS,
            &payload,
        )
        .unwrap();
        let crate::packets::PlayClientbound::CommandSuggestions(mut packet) = packet else {
            panic!("expected suggestions")
        };
        assert_eq!(packet.suggestions[0].text, "�");
        assert_eq!((packet.id, packet.start, packet.length), (7, 0, 1));
        packet.suggestions[0].text = vec![0xd800, 0x20ac].into();
        let (_, encoded) = encode_play_clientbound_command_suggestions(&packet).unwrap();
        let mut decoder = Decoder::new(&encoded);
        for expected in [7, 0, 1, 1, 2] {
            assert_eq!(decoder.read_var_i32().unwrap(), expected);
        }
        assert_eq!(
            decoder.read_exact(2, "suggestion text").unwrap(),
            &[0x3f, 0xac]
        );
        assert!(!decoder.read_bool().unwrap());
        assert!(decoder.is_empty());
    }
    #[test]
    fn chat_wire_suggestion_text_encoder_checks_java_length() {
        let mut packet = CommandSuggestions {
            id: 1,
            start: 0,
            length: 0,
            suggestions: vec![CommandSuggestion {
                text: ("😀".repeat(16383) + "x").into(),
                tooltip: None,
                tooltip_styled: None,
                tooltip_component: None,
            }],
        };
        let (id, bytes) = encode_play_clientbound_command_suggestions(&packet).unwrap();
        assert!(crate::packets::decode_play_clientbound(id, &bytes).is_ok());
        packet.suggestions[0].text.push_str("x");
        assert!(encode_play_clientbound_command_suggestions(&packet).is_err());
    }
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
                        text: "give".into(),
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
                                name: "text".into(),
                                value: crate::NbtSummaryValue::String("Run give".into()),
                            },
                            crate::NbtSummaryEntry {
                                name: "color".into(),
                                value: crate::NbtSummaryValue::String("red".into()),
                            },
                            crate::NbtSummaryEntry {
                                name: "bold".into(),
                                value: crate::NbtSummaryValue::Byte(1),
                            },
                        ])),
                    },
                    CommandSuggestion {
                        text: "gamemode".into(),
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

    /// `Entry` carries one tooltip and it is the Component
    /// (ClientboundCommandSuggestionsPacket.java:61-68), so the two projections
    /// have no field of their own on the wire. An entry holding a projection
    /// without the Component it projects must not leave as a tooltipless entry:
    /// that is the one way a tooltip could disappear between this encoder and
    /// the client.
    #[test]
    fn a_tooltip_projection_without_its_component_is_refused_rather_than_dropped() {
        let sent = |suggestion: CommandSuggestion| {
            encode_play_clientbound_command_suggestions(&CommandSuggestions {
                id: 7,
                start: 1,
                length: 0,
                suggestions: vec![suggestion],
            })
        };
        let decoded = |payload: &[u8]| match decode_play_clientbound(
            ids::play::CLIENTBOUND_COMMAND_SUGGESTIONS,
            payload,
        )
        .unwrap()
        {
            PlayClientbound::CommandSuggestions(update) => update.suggestions,
            other => panic!("{other:?}"),
        };
        let bare = CommandSuggestion {
            text: "@e".into(),
            tooltip: None,
            tooltip_styled: None,
            tooltip_component: None,
        };
        let runs = vec![StyledTextRun {
            text: "All entities".to_string(),
            style: crate::ComponentStyle::default(),
        }];
        let component = NbtSummaryValue::Compound(vec![crate::NbtSummaryEntry {
            name: "translate".into(),
            value: NbtSummaryValue::String("argument.entity.selector.allEntities".into()),
        }]);

        // A suggestion with no tooltip at all is the common case and still goes
        // out, present flag clear.
        let (id, payload) = sent(bare.clone()).unwrap();
        assert_eq!(id, ids::play::CLIENTBOUND_COMMAND_SUGGESTIONS);
        assert_eq!(decoded(&payload), vec![bare.clone()]);

        for broken in [
            CommandSuggestion {
                tooltip: Some("All entities".to_string()),
                ..bare.clone()
            },
            CommandSuggestion {
                tooltip_styled: Some(runs.clone()),
                ..bare.clone()
            },
        ] {
            let error = sent(broken.clone()).unwrap_err();
            assert!(
                matches!(&error, ProtocolError::InvalidData(message)
                    if message.contains("\"@e\"") && message.contains("tooltip projection")),
                "{broken:?} was answered with {error:?}"
            );
        }

        // With the Component present the tooltip does reach the wire, which is
        // what makes the branch above a loss rather than a no-op.
        let (_, payload) = sent(CommandSuggestion {
            tooltip: Some("All entities".to_string()),
            tooltip_styled: Some(runs),
            tooltip_component: Some(component.clone()),
            ..bare
        })
        .unwrap();
        assert_eq!(
            decoded(&payload)[0].tooltip_component.as_ref(),
            Some(&component)
        );
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
