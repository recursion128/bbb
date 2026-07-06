use serde::{Deserialize, Serialize};

use crate::codec::{Decoder, ProtocolError, Result};

const MAX_NBT_DEPTH: usize = 64;
const MAX_NBT_LIST_ITEMS: usize = 4096;

#[derive(Debug, Clone, PartialEq, Eq)]
enum NbtValue {
    Byte(i8),
    String(String),
    List(Vec<NbtValue>),
    Compound(Vec<(String, NbtValue)>),
    Other,
}

/// The render-relevant subset of vanilla `net.minecraft.network.chat.Style`
/// (codec keys `bold` / `italic` / `underlined` / `strikethrough` /
/// `obfuscated` / `color`, `Style.Serializer.MAP_CODEC`). Each key is `None`
/// when the component chain never set it, mirroring vanilla's nullable style
/// fields so downstream default-style merges (`ComponentUtils.mergeStyles`,
/// e.g. the lore style) can still fill unset keys.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct ComponentStyle {
    #[serde(default)]
    pub bold: Option<bool>,
    #[serde(default)]
    pub italic: Option<bool>,
    #[serde(default)]
    pub underlined: Option<bool>,
    #[serde(default)]
    pub strikethrough: Option<bool>,
    #[serde(default)]
    pub obfuscated: Option<bool>,
    /// Resolved text colour as `0xRRGGBB` (vanilla `TextColor.getValue()`),
    /// parsed from either a named `ChatFormatting` colour or a `#RRGGBB`
    /// literal (`TextColor.parseColor`).
    #[serde(default)]
    pub color: Option<u32>,
}

impl ComponentStyle {
    /// Vanilla `Style.applyTo(other)`: keys set on `self` win, unset keys
    /// inherit from `parent`.
    pub fn apply_to(&self, parent: &ComponentStyle) -> ComponentStyle {
        ComponentStyle {
            bold: self.bold.or(parent.bold),
            italic: self.italic.or(parent.italic),
            underlined: self.underlined.or(parent.underlined),
            strikethrough: self.strikethrough.or(parent.strikethrough),
            obfuscated: self.obfuscated.or(parent.obfuscated),
            color: self.color.or(parent.color),
        }
    }
}

/// One flattened chat-component run: contiguous text with its fully inherited
/// style (parent styles already merged via [`ComponentStyle::apply_to`]).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StyledTextRun {
    pub text: String,
    #[serde(default)]
    pub style: ComponentStyle,
}

/// Concatenated plain text of a run list; the empty-component fallback matches
/// the legacy plain-text decoder (`"component nbt"`), keeping the old API a
/// pure delegation of the styled one.
pub fn styled_runs_summary_text(runs: &[StyledTextRun]) -> String {
    let mut out = String::new();
    for run in runs {
        out.push_str(&run.text);
    }
    if out.is_empty() {
        "component nbt".to_string()
    } else {
        out
    }
}

pub(crate) fn decode_component_summary_from_decoder(decoder: &mut Decoder<'_>) -> Result<String> {
    let runs = decode_styled_component_summary_from_decoder(decoder)?;
    Ok(styled_runs_summary_text(&runs))
}

pub(crate) fn decode_styled_component_summary_from_decoder(
    decoder: &mut Decoder<'_>,
) -> Result<Vec<StyledTextRun>> {
    let root = read_nbt_any(decoder)?;
    let mut runs = RunCollector::default();
    append_component_runs(&root, &ComponentStyle::default(), &mut runs);
    Ok(runs.runs)
}

pub(crate) fn skip_nbt_tag_from_decoder(decoder: &mut Decoder<'_>) -> Result<()> {
    read_nbt_any(decoder).map(|_| ())
}

pub(crate) fn decode_component_summary(payload: &[u8]) -> Result<String> {
    let mut decoder = Decoder::new(payload);
    let summary = decode_component_summary_from_decoder(&mut decoder)?;
    if !decoder.is_empty() {
        return Err(ProtocolError::InvalidData(
            "trailing bytes after component nbt".to_string(),
        ));
    }
    if summary == "component nbt" {
        Ok(format!("component nbt ({} bytes)", payload.len()))
    } else {
        Ok(summary)
    }
}

/// Decodes a network chat component into flattened styled runs. The companion
/// of [`decode_component_summary`]: same NBT traversal and text extraction,
/// but each contiguous piece of text keeps its inherited vanilla `Style`
/// subset instead of being flattened to a bare string.
pub fn decode_styled_component_summary(payload: &[u8]) -> Result<Vec<StyledTextRun>> {
    let mut decoder = Decoder::new(payload);
    let runs = decode_styled_component_summary_from_decoder(&mut decoder)?;
    if !decoder.is_empty() {
        return Err(ProtocolError::InvalidData(
            "trailing bytes after component nbt".to_string(),
        ));
    }
    Ok(runs)
}

/// One sign face decoded from the sign block entity's NBT — vanilla
/// `SignText.DIRECT_CODEC` (`SignText.java`): `messages` is a list of exactly
/// four chat components (`LINES_CODEC` enforces `Util.fixedSize(_, 4)`), each
/// flattened here into styled runs by the shared component traversal;
/// `color` is the raw `DyeColor.CODEC` name string (`"black"` when absent —
/// `fieldOf("color").orElse(DyeColor.BLACK)`); `has_glowing_text` is the
/// `Codec.BOOL` byte (`orElse(false)`). The `filtered_messages` list only
/// matters when text filtering is enabled and is not decoded.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SignTextNbt {
    pub messages: [Vec<StyledTextRun>; 4],
    pub color: Option<String>,
    pub has_glowing_text: bool,
}

/// The sign block entity's render-relevant NBT — vanilla
/// `SignBlockEntity.loadAdditional`: the `front_text` / `back_text`
/// `SignText.DIRECT_CODEC` compounds plus the root `is_waxed` boolean.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SignBlockEntityNbt {
    pub front_text: Option<SignTextNbt>,
    pub back_text: Option<SignTextNbt>,
    pub is_waxed: bool,
}

/// Decodes a sign block entity's NBT (`SignBlockEntity.loadAdditional`
/// fields). Returns `Ok(None)` when the payload carries neither `front_text`
/// nor `back_text` — every non-sign block entity's NBT flows through here, so
/// absence of the sign keys is the (cheap) sign gate, mirroring how the codec
/// only ever sees sign-shaped data in vanilla. Message components reuse the
/// single styled-component traversal ([`decode_styled_component_summary`]'s
/// `append_component_runs`), so sign lines carry the same inherited-style
/// runs as every other decoded component in the workspace.
pub fn decode_sign_block_entity_nbt(raw_nbt: &[u8]) -> Result<Option<SignBlockEntityNbt>> {
    let mut decoder = Decoder::new(raw_nbt);
    let root = read_nbt_any(&mut decoder)?;
    if !decoder.is_empty() {
        return Err(ProtocolError::InvalidData(
            "trailing bytes after block entity nbt".to_string(),
        ));
    }
    let NbtValue::Compound(entries) = root else {
        return Ok(None);
    };
    let front_text = decode_sign_text_nbt(&entries, "front_text");
    let back_text = decode_sign_text_nbt(&entries, "back_text");
    if front_text.is_none() && back_text.is_none() {
        return Ok(None);
    }
    Ok(Some(SignBlockEntityNbt {
        front_text,
        back_text,
        is_waxed: matches!(find_entry(&entries, "is_waxed"), Some(NbtValue::Byte(value)) if *value != 0),
    }))
}

fn decode_sign_text_nbt(entries: &[(String, NbtValue)], key: &str) -> Option<SignTextNbt> {
    let NbtValue::Compound(text_entries) = find_entry(entries, key)? else {
        return None;
    };
    let NbtValue::List(messages) = find_entry(text_entries, "messages")? else {
        return None;
    };
    // Vanilla `SignText.LINES_CODEC` rejects any list that is not exactly four
    // components (`Util.fixedSize(_, 4)`), falling back to an empty SignText.
    if messages.len() != 4 {
        return None;
    }
    let messages = std::array::from_fn(|index| {
        let mut runs = RunCollector::default();
        append_component_runs(&messages[index], &ComponentStyle::default(), &mut runs);
        runs.runs
    });
    let color = match find_entry(text_entries, "color") {
        Some(NbtValue::String(color)) => Some(color.clone()),
        _ => None,
    };
    Some(SignTextNbt {
        messages,
        color,
        has_glowing_text: matches!(
            find_entry(text_entries, "has_glowing_text"),
            Some(NbtValue::Byte(value)) if *value != 0
        ),
    })
}

fn read_nbt_any(decoder: &mut Decoder<'_>) -> Result<NbtValue> {
    let tag_id = decoder.read_u8()?;
    if tag_id == 0 {
        return Ok(NbtValue::Other);
    }
    read_nbt_payload(decoder, tag_id, 0)
}

fn read_nbt_payload(decoder: &mut Decoder<'_>, tag_id: u8, depth: usize) -> Result<NbtValue> {
    if depth > MAX_NBT_DEPTH {
        return Err(ProtocolError::InvalidData(
            "component nbt exceeded max depth".to_string(),
        ));
    }

    match tag_id {
        1 => Ok(NbtValue::Byte(decoder.read_u8()? as i8)),
        2 => {
            decoder.read_exact(2, "nbt short")?;
            Ok(NbtValue::Other)
        }
        3 | 5 => {
            decoder.read_exact(4, "nbt int/float")?;
            Ok(NbtValue::Other)
        }
        4 | 6 => {
            decoder.read_exact(8, "nbt long/double")?;
            Ok(NbtValue::Other)
        }
        7 => {
            let len = read_nbt_len(decoder)?;
            decoder.read_exact(len, "nbt byte array")?;
            Ok(NbtValue::Other)
        }
        8 => Ok(NbtValue::String(read_modified_utf8(decoder)?)),
        9 => {
            let element_type = decoder.read_u8()?;
            let len = read_nbt_len(decoder)?;
            if len > MAX_NBT_LIST_ITEMS {
                return Err(ProtocolError::PacketTooLarge(len, MAX_NBT_LIST_ITEMS));
            }
            if element_type == 0 && len > 0 {
                return Err(ProtocolError::InvalidData(
                    "non-empty nbt list has end tag element type".to_string(),
                ));
            }
            let mut items = Vec::with_capacity(len);
            for _ in 0..len {
                items.push(read_nbt_payload(decoder, element_type, depth + 1)?);
            }
            Ok(NbtValue::List(items))
        }
        10 => {
            let mut entries = Vec::new();
            loop {
                let nested_type = decoder.read_u8()?;
                if nested_type == 0 {
                    break;
                }
                let name = read_modified_utf8(decoder)?;
                let value = read_nbt_payload(decoder, nested_type, depth + 1)?;
                entries.push((name, value));
            }
            Ok(NbtValue::Compound(entries))
        }
        11 => {
            let len = read_nbt_len(decoder)?;
            let byte_len = len.checked_mul(4).ok_or_else(|| {
                ProtocolError::InvalidData("nbt int array length overflow".to_string())
            })?;
            decoder.read_exact(byte_len, "nbt int array")?;
            Ok(NbtValue::Other)
        }
        12 => {
            let len = read_nbt_len(decoder)?;
            let byte_len = len.checked_mul(8).ok_or_else(|| {
                ProtocolError::InvalidData("nbt long array length overflow".to_string())
            })?;
            decoder.read_exact(byte_len, "nbt long array")?;
            Ok(NbtValue::Other)
        }
        other => Err(ProtocolError::InvalidData(format!(
            "invalid component nbt tag id {other}"
        ))),
    }
}

fn read_nbt_len(decoder: &mut Decoder<'_>) -> Result<usize> {
    let len = decoder.read_i32()?;
    if len < 0 {
        return Err(ProtocolError::NegativeLength(len));
    }
    Ok(len as usize)
}

fn read_modified_utf8(decoder: &mut Decoder<'_>) -> Result<String> {
    let len = decoder.read_u16()? as usize;
    let bytes = decoder.read_exact(len, "nbt string")?;
    let mut units = Vec::with_capacity(len);
    let mut cursor = 0;

    while cursor < bytes.len() {
        let b0 = bytes[cursor];
        if b0 & 0x80 == 0 {
            units.push(u16::from(b0));
            cursor += 1;
        } else if b0 & 0xe0 == 0xc0 {
            if cursor + 1 >= bytes.len() {
                return Err(ProtocolError::InvalidData(
                    "truncated modified utf-8 sequence".to_string(),
                ));
            }
            let b1 = bytes[cursor + 1];
            if b1 & 0xc0 != 0x80 {
                return Err(ProtocolError::InvalidData(
                    "invalid modified utf-8 continuation".to_string(),
                ));
            }
            units.push((u16::from(b0 & 0x1f) << 6) | u16::from(b1 & 0x3f));
            cursor += 2;
        } else if b0 & 0xf0 == 0xe0 {
            if cursor + 2 >= bytes.len() {
                return Err(ProtocolError::InvalidData(
                    "truncated modified utf-8 sequence".to_string(),
                ));
            }
            let b1 = bytes[cursor + 1];
            let b2 = bytes[cursor + 2];
            if b1 & 0xc0 != 0x80 || b2 & 0xc0 != 0x80 {
                return Err(ProtocolError::InvalidData(
                    "invalid modified utf-8 continuation".to_string(),
                ));
            }
            units.push(
                (u16::from(b0 & 0x0f) << 12) | (u16::from(b1 & 0x3f) << 6) | u16::from(b2 & 0x3f),
            );
            cursor += 3;
        } else {
            return Err(ProtocolError::InvalidData(
                "invalid modified utf-8 leading byte".to_string(),
            ));
        }
    }

    String::from_utf16(&units)
        .map_err(|_| ProtocolError::InvalidData("invalid modified utf-8 string".to_string()))
}

/// Accumulates flattened runs, merging adjacent same-style text so the run
/// list is canonical and its concatenation reproduces the legacy plain-text
/// traversal byte for byte.
#[derive(Default)]
struct RunCollector {
    runs: Vec<StyledTextRun>,
}

impl RunCollector {
    fn push(&mut self, text: &str, style: ComponentStyle) {
        if text.is_empty() {
            return;
        }
        if let Some(last) = self.runs.last_mut() {
            if last.style == style {
                last.text.push_str(text);
                return;
            }
        }
        self.runs.push(StyledTextRun {
            text: text.to_string(),
            style,
        });
    }

    fn is_empty(&self) -> bool {
        self.runs.is_empty()
    }
}

fn append_component_runs(value: &NbtValue, inherited: &ComponentStyle, out: &mut RunCollector) {
    match value {
        NbtValue::String(text) => out.push(text, *inherited),
        NbtValue::List(items) => {
            for item in items {
                append_component_runs(item, inherited, out);
            }
        }
        NbtValue::Compound(entries) => {
            // Vanilla component inheritance: the node's own style keys win,
            // unset keys inherit the parent chain (`Style.applyTo` inside
            // `Component.visit`); siblings in `extra` inherit this node's
            // effective style.
            let style = component_style_from_entries(entries).apply_to(inherited);
            append_primary_component_runs(entries, &style, out);
            if let Some(extra) = find_entry(entries, "extra") {
                append_component_runs(extra, &style, out);
            }
        }
        NbtValue::Byte(_) | NbtValue::Other => {}
    }
}

fn append_primary_component_runs(
    entries: &[(String, NbtValue)],
    style: &ComponentStyle,
    out: &mut RunCollector,
) {
    if let Some(text) = find_entry(entries, "text") {
        append_component_runs(text, style, out);
        return;
    }

    if let Some(fallback) = find_entry(entries, "fallback") {
        append_component_runs(fallback, style, out);
    } else if let Some(translate) = find_entry(entries, "translate") {
        append_component_runs(translate, style, out);
    } else if let Some(keybind) = find_entry(entries, "keybind") {
        append_component_runs(keybind, style, out);
    } else if let Some(selector) = find_entry(entries, "selector") {
        append_component_runs(selector, style, out);
    } else if let Some(nbt) = find_entry(entries, "nbt") {
        append_component_runs(nbt, style, out);
    }

    if let Some(with) = find_entry(entries, "with") {
        if !out.is_empty() {
            out.push(" ", *style);
        }
        append_component_runs(with, style, out);
    }
}

/// Extracts the render-relevant vanilla `Style` keys from a component
/// compound (`Style.Serializer.MAP_CODEC` field names). Booleans arrive as
/// NBT bytes (`Codec.BOOL` via `NbtOps`), the colour as a string
/// (`TextColor.CODEC`); malformed values are treated as unset, matching the
/// summary decoder's lenient posture.
fn component_style_from_entries(entries: &[(String, NbtValue)]) -> ComponentStyle {
    ComponentStyle {
        bold: style_flag(entries, "bold"),
        italic: style_flag(entries, "italic"),
        underlined: style_flag(entries, "underlined"),
        strikethrough: style_flag(entries, "strikethrough"),
        obfuscated: style_flag(entries, "obfuscated"),
        color: match find_entry(entries, "color") {
            Some(NbtValue::String(color)) => parse_text_color(color),
            _ => None,
        },
    }
}

fn style_flag(entries: &[(String, NbtValue)], key: &str) -> Option<bool> {
    match find_entry(entries, key) {
        Some(NbtValue::Byte(value)) => Some(*value != 0),
        _ => None,
    }
}

/// Vanilla `TextColor.parseColor`: `#`-prefixed hex literal in `0..=0xFFFFFF`,
/// otherwise an exact named `ChatFormatting` colour (`getName()` =
/// lowercased enum name, underscores kept — the `NAMED_COLORS` map).
fn parse_text_color(color: &str) -> Option<u32> {
    if let Some(hex) = color.strip_prefix('#') {
        let value = u32::from_str_radix(hex, 16).ok()?;
        return (value <= 0xFF_FF_FF).then_some(value);
    }
    // ChatFormatting colour table (enum name -> colour int).
    Some(match color {
        "black" => 0x00_00_00,
        "dark_blue" => 0x00_00_AA,
        "dark_green" => 0x00_AA_00,
        "dark_aqua" => 0x00_AA_AA,
        "dark_red" => 0xAA_00_00,
        "dark_purple" => 0xAA_00_AA,
        "gold" => 0xFF_AA_00,
        "gray" => 0xAA_AA_AA,
        "dark_gray" => 0x55_55_55,
        "blue" => 0x55_55_FF,
        "green" => 0x55_FF_55,
        "aqua" => 0x55_FF_FF,
        "red" => 0xFF_55_55,
        "light_purple" => 0xFF_55_FF,
        "yellow" => 0xFF_FF_55,
        "white" => 0xFF_FF_FF,
        _ => return None,
    })
}

fn find_entry<'a>(entries: &'a [(String, NbtValue)], key: &str) -> Option<&'a NbtValue> {
    entries.iter().find(|(name, _)| name == key).map(|(_, v)| v)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_string_component_nbt() {
        let payload = nbt_string_root("Disconnected");
        assert_eq!(
            decode_component_summary(&payload).unwrap(),
            "Disconnected".to_string()
        );
    }

    #[test]
    fn decodes_component_text_without_trimming_magic_names() {
        let payload = nbt_string_root(" jeb_ ");
        assert_eq!(decode_component_summary(&payload).unwrap(), " jeb_ ");
    }

    #[test]
    fn decodes_compound_text_and_extra_component_nbt() {
        let mut payload = vec![10];
        write_named_string(&mut payload, "text", "Kicked");
        payload.push(9);
        write_mutf8(&mut payload, "extra");
        payload.push(10);
        payload.extend_from_slice(&1i32.to_be_bytes());
        write_named_string(&mut payload, "text", " by server");
        payload.push(0);
        payload.push(0);

        assert_eq!(
            decode_component_summary(&payload).unwrap(),
            "Kicked by server".to_string()
        );
    }

    #[test]
    fn styled_runs_of_plain_string_root_carry_default_style() {
        let payload = nbt_string_root("Hello");
        assert_eq!(
            decode_styled_component_summary(&payload).unwrap(),
            vec![StyledTextRun {
                text: "Hello".to_string(),
                style: ComponentStyle::default(),
            }]
        );
    }

    #[test]
    fn styled_runs_decode_every_style_key_and_named_color() {
        // {text:"Styled", bold:1b, italic:0b, underlined:1b, strikethrough:1b,
        //  obfuscated:1b, color:"dark_purple"}
        let mut payload = vec![10];
        write_named_string(&mut payload, "text", "Styled");
        write_named_byte(&mut payload, "bold", 1);
        write_named_byte(&mut payload, "italic", 0);
        write_named_byte(&mut payload, "underlined", 1);
        write_named_byte(&mut payload, "strikethrough", 1);
        write_named_byte(&mut payload, "obfuscated", 1);
        write_named_string(&mut payload, "color", "dark_purple");
        payload.push(0);

        assert_eq!(
            decode_styled_component_summary(&payload).unwrap(),
            vec![StyledTextRun {
                text: "Styled".to_string(),
                style: ComponentStyle {
                    bold: Some(true),
                    italic: Some(false),
                    underlined: Some(true),
                    strikethrough: Some(true),
                    obfuscated: Some(true),
                    color: Some(0xAA_00_AA),
                },
            }]
        );
    }

    #[test]
    fn styled_runs_parse_hex_color_and_reject_invalid_colors() {
        let mut payload = vec![10];
        write_named_string(&mut payload, "text", "Hex");
        write_named_string(&mut payload, "color", "#1A2b3C");
        payload.push(0);
        assert_eq!(
            decode_styled_component_summary(&payload).unwrap()[0]
                .style
                .color,
            Some(0x1A_2B_3C)
        );

        // Out-of-range hex and unknown names are unset (vanilla parseColor errors).
        for bad in ["#1FFFFFF", "#nothex", "purple", "DARK_PURPLE"] {
            let mut payload = vec![10];
            write_named_string(&mut payload, "text", "Bad");
            write_named_string(&mut payload, "color", bad);
            payload.push(0);
            assert_eq!(
                decode_styled_component_summary(&payload).unwrap()[0]
                    .style
                    .color,
                None,
                "color {bad:?} should not parse"
            );
        }
    }

    #[test]
    fn styled_runs_inherit_parent_style_into_extra_children() {
        // {text:"Kicked", bold:1b, color:"red", extra:[
        //    {text:" by"},                       // inherits bold + red
        //    {text:" server", bold:0b, italic:1b} // overrides bold, keeps red
        // ]}
        let mut payload = vec![10];
        write_named_string(&mut payload, "text", "Kicked");
        write_named_byte(&mut payload, "bold", 1);
        write_named_string(&mut payload, "color", "red");
        payload.push(9);
        write_mutf8(&mut payload, "extra");
        payload.push(10);
        payload.extend_from_slice(&2i32.to_be_bytes());
        write_named_string(&mut payload, "text", " by");
        payload.push(0);
        write_named_string(&mut payload, "text", " server");
        write_named_byte(&mut payload, "bold", 0);
        write_named_byte(&mut payload, "italic", 1);
        payload.push(0);
        payload.push(0);

        let bold_red = ComponentStyle {
            bold: Some(true),
            color: Some(0xFF_55_55),
            ..ComponentStyle::default()
        };
        assert_eq!(
            decode_styled_component_summary(&payload).unwrap(),
            vec![
                StyledTextRun {
                    text: "Kicked by".to_string(),
                    style: bold_red,
                },
                StyledTextRun {
                    text: " server".to_string(),
                    style: ComponentStyle {
                        bold: Some(false),
                        italic: Some(true),
                        color: Some(0xFF_55_55),
                        ..ComponentStyle::default()
                    },
                },
            ]
        );

        // The plain-text API is a pure delegation: concatenated run text.
        assert_eq!(
            decode_component_summary(&payload).unwrap(),
            "Kicked by server"
        );
    }

    #[test]
    fn apply_to_matches_vanilla_style_apply_to() {
        let child = ComponentStyle {
            bold: Some(false),
            color: Some(0x12_34_56),
            ..ComponentStyle::default()
        };
        let parent = ComponentStyle {
            bold: Some(true),
            italic: Some(true),
            color: Some(0xFF_FF_FF),
            ..ComponentStyle::default()
        };
        assert_eq!(
            child.apply_to(&parent),
            ComponentStyle {
                bold: Some(false),
                italic: Some(true),
                color: Some(0x12_34_56),
                ..ComponentStyle::default()
            }
        );
    }

    #[test]
    fn styled_runs_summary_text_falls_back_like_the_legacy_decoder() {
        assert_eq!(styled_runs_summary_text(&[]), "component nbt");
        assert_eq!(
            styled_runs_summary_text(&[StyledTextRun {
                text: "ab".to_string(),
                style: ComponentStyle::default(),
            }]),
            "ab"
        );
    }

    fn sign_side_nbt(
        out: &mut Vec<u8>,
        key: &str,
        lines: &[&str],
        color: Option<&str>,
        glowing: bool,
    ) {
        out.push(10);
        write_mutf8(out, key);
        out.push(9);
        write_mutf8(out, "messages");
        out.push(8);
        out.extend_from_slice(&(lines.len() as i32).to_be_bytes());
        for line in lines {
            write_mutf8(out, line);
        }
        if let Some(color) = color {
            write_named_string(out, "color", color);
        }
        if glowing {
            write_named_byte(out, "has_glowing_text", 1);
        }
        out.push(0);
    }

    #[test]
    fn decodes_sign_block_entity_nbt_with_color_glowing_and_waxed() {
        let mut payload = vec![10];
        sign_side_nbt(
            &mut payload,
            "front_text",
            &["a", "b", "c", "d"],
            Some("red"),
            true,
        );
        sign_side_nbt(&mut payload, "back_text", &["", "", "", ""], None, false);
        write_named_byte(&mut payload, "is_waxed", 1);
        payload.push(0);

        let sign = decode_sign_block_entity_nbt(&payload).unwrap().unwrap();
        assert!(sign.is_waxed);
        let front = sign.front_text.unwrap();
        assert_eq!(front.color.as_deref(), Some("red"));
        assert!(front.has_glowing_text);
        assert_eq!(front.messages[0][0].text, "a");
        assert_eq!(front.messages[3][0].text, "d");
        let back = sign.back_text.unwrap();
        assert_eq!(back.color, None);
        assert!(!back.has_glowing_text);
        assert!(back.messages.iter().all(Vec::is_empty));
    }

    #[test]
    fn sign_message_components_keep_styled_runs() {
        // messages[0] = {text:"Hi", bold:1b, color:"#123456"} as a compound list element.
        let mut payload = vec![10];
        payload.push(10);
        write_mutf8(&mut payload, "front_text");
        payload.push(9);
        write_mutf8(&mut payload, "messages");
        payload.push(10);
        payload.extend_from_slice(&4i32.to_be_bytes());
        for index in 0..4 {
            if index == 0 {
                write_named_string(&mut payload, "text", "Hi");
                write_named_byte(&mut payload, "bold", 1);
                write_named_string(&mut payload, "color", "#123456");
            } else {
                write_named_string(&mut payload, "text", "");
            }
            payload.push(0);
        }
        payload.push(0);
        payload.push(0);

        let sign = decode_sign_block_entity_nbt(&payload).unwrap().unwrap();
        let front = sign.front_text.unwrap();
        assert_eq!(
            front.messages[0],
            vec![StyledTextRun {
                text: "Hi".to_string(),
                style: ComponentStyle {
                    bold: Some(true),
                    color: Some(0x12_34_56),
                    ..ComponentStyle::default()
                },
            }]
        );
        assert!(sign.back_text.is_none());
    }

    #[test]
    fn non_sign_block_entity_nbt_decodes_to_none() {
        // A vault-ish compound without front_text/back_text.
        let mut payload = vec![10];
        write_named_string(&mut payload, "id", "minecraft:vault");
        payload.push(0);
        assert_eq!(decode_sign_block_entity_nbt(&payload).unwrap(), None);
        // An empty (end-tag-only) network NBT.
        assert_eq!(decode_sign_block_entity_nbt(&[0]).unwrap(), None);
    }

    #[test]
    fn sign_messages_list_must_be_exactly_four_lines() {
        let mut payload = vec![10];
        sign_side_nbt(&mut payload, "front_text", &["a", "b", "c"], None, false);
        payload.push(0);
        assert_eq!(decode_sign_block_entity_nbt(&payload).unwrap(), None);
    }

    fn nbt_string_root(value: &str) -> Vec<u8> {
        let mut payload = vec![8];
        write_mutf8(&mut payload, value);
        payload
    }

    fn write_named_byte(out: &mut Vec<u8>, name: &str, value: u8) {
        out.push(1);
        write_mutf8(out, name);
        out.push(value);
    }

    fn write_named_string(out: &mut Vec<u8>, name: &str, value: &str) {
        out.push(8);
        write_mutf8(out, name);
        write_mutf8(out, value);
    }

    fn write_mutf8(out: &mut Vec<u8>, value: &str) {
        let bytes = value.as_bytes();
        out.extend_from_slice(&(bytes.len() as u16).to_be_bytes());
        out.extend_from_slice(bytes);
    }
}
