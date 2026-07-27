use serde::{de, Deserialize, Deserializer, Serialize};

use crate::{
    chat_formattings::vanilla_chat_formatting_color_by_name,
    codec::{Decoder, ProtocolError, Result},
    nbt_summary::{decode_nbt_summary_from_decoder, NbtSummaryEntry, NbtSummaryValue},
};

mod canonical_json;
mod codec;
mod command;

pub(crate) use canonical_json::java_double_to_string;
pub use canonical_json::{
    canonical_component_codec_value, canonical_component_json_utf16_len,
    canonical_component_json_utf16_len_with_owner, canonical_f32_json_utf16_len,
    canonical_f64_json_utf16_len, canonical_json_string_utf16_len, canonical_nbt_json_utf16_len,
    canonical_resolvable_profile_json_utf16_len, java_float_to_string,
    CanonicalComponentCodecValue, CanonicalComponentOwnerValue,
};
pub use codec::{validate_component_nbt, visit_component_payloads, ComponentPayload};
pub use command::{
    parse_command_coordinates_prefix, parse_command_entity_selector_prefix,
    parse_command_nbt_path_prefix, parse_command_snbt_prefix, parse_command_snbt_value_prefix,
    parse_command_uuid_prefix, CommandCoordinatesKind, CommandEntitySelectorOption,
    CommandEntitySelectorOptionKind, CommandEntitySelectorPrefix, CommandSnbtKind,
    VANILLA_COMMAND_ENTITY_SELECTOR_OPTIONS,
};

/// The client-visible subset of vanilla `net.minecraft.network.chat.Style`
/// (`Style.Serializer.MAP_CODEC`). Each key is `None` when the component chain
/// never set it, mirroring vanilla's nullable style fields so downstream
/// default-style merges (`ComponentUtils.mergeStyles`, e.g. the lore style)
/// can still fill unset keys.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
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
    /// Click event carried by the run (`click_event` in the 26.1 Style codec).
    /// Native Dialog text hit testing consumes it at glyph granularity; other
    /// renderer projections may retain it without making the text interactive.
    #[serde(default)]
    pub click_event: Option<ComponentClickEvent>,
    /// Hover payload carried by the resolved run. The enum retains all three
    /// server-valid Vanilla actions; presentation owners may render the
    /// lightweight show-text case while keeping item/entity payloads exact.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hover_event: Option<ComponentHoverEvent>,
    /// Exact resolved font identifier. Unsupported font providers may fall
    /// back visually, but trust evaluation and cache identity must not lose it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub font: Option<String>,
}

/// Supported vanilla `ClickEvent` payloads in the same externally tagged shape
/// as the 26.1 codec (`action` plus the action-specific value key).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "action")]
pub enum ComponentClickEvent {
    /// Vanilla permits this for locally-created Components but rejects it from
    /// the server-facing serialized `ClickEvent.Action.CODEC`.
    #[serde(rename = "open_file")]
    OpenFile { path: String },
    #[serde(rename = "open_url")]
    OpenUrl { url: String },
    #[serde(rename = "run_command")]
    RunCommand { command: String },
    #[serde(rename = "suggest_command")]
    SuggestCommand { command: String },
    #[serde(rename = "show_dialog")]
    ShowDialog { dialog: NbtSummaryValue },
    #[serde(rename = "change_page")]
    ChangePage { page: i32 },
    #[serde(rename = "copy_to_clipboard")]
    CopyToClipboard { value: String },
    #[serde(rename = "custom")]
    Custom {
        id: String,
        #[serde(default)]
        payload: Option<NbtSummaryValue>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum ComponentHoverEvent {
    ShowText {
        value: Vec<StyledTextRun>,
    },
    ShowItem {
        id: String,
        count: i32,
        #[serde(default)]
        components: Option<NbtSummaryValue>,
    },
    ShowEntity {
        id: String,
        uuid: NbtSummaryValue,
        #[serde(default)]
        name: Option<Vec<StyledTextRun>>,
    },
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
            click_event: self
                .click_event
                .clone()
                .or_else(|| parent.click_event.clone()),
            hover_event: self
                .hover_event
                .clone()
                .or_else(|| parent.hover_event.clone()),
            font: self.font.clone().or_else(|| parent.font.clone()),
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

/// One trusted network `Component`, preserving the exact validated NBT value
/// together with the two projections used by native presentation code.
///
/// Keeping these fields together prevents metadata consumers from treating a
/// lossy plain string as the canonical value while independently attempting to
/// reconstruct style or owner-sensitive payloads later.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RichTextComponent {
    text: String,
    styled: Vec<StyledTextRun>,
    component: NbtSummaryValue,
}

impl RichTextComponent {
    pub fn try_from_component_nbt(component: NbtSummaryValue) -> std::result::Result<Self, String> {
        validate_component_nbt(&component, "rich text component")?;
        Ok(Self::from_decoded_component(component))
    }

    fn from_decoded_component(component: NbtSummaryValue) -> Self {
        let styled = styled_component_runs_from_nbt_value(&component);
        let text = styled_runs_summary_text(&styled);
        Self {
            text,
            styled,
            component,
        }
    }

    pub fn plain(text: impl Into<String>) -> Self {
        Self::from_decoded_component(NbtSummaryValue::String(text.into()))
    }

    pub fn text(&self) -> &str {
        &self.text
    }

    pub fn styled(&self) -> &[StyledTextRun] {
        &self.styled
    }

    /// Mirrors `Component.getString().isEmpty()` for packet consumers that
    /// turn an empty component into an absent presentation value.
    ///
    /// The legacy text summary intentionally substitutes `"component nbt"`
    /// when no display run exists, so emptiness must come from the canonical
    /// styled projection instead of [`Self::text`].
    pub fn is_empty(&self) -> bool {
        self.styled.is_empty()
    }

    pub fn component(&self) -> &NbtSummaryValue {
        &self.component
    }
}

impl<'de> Deserialize<'de> for RichTextComponent {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct RichTextComponentFields {
            text: String,
            styled: Vec<StyledTextRun>,
            component: NbtSummaryValue,
        }

        let fields = RichTextComponentFields::deserialize(deserializer)?;
        let component =
            Self::try_from_component_nbt(fields.component).map_err(de::Error::custom)?;
        if component.text != fields.text || component.styled != fields.styled {
            return Err(de::Error::custom(
                "rich text component projections do not match exact component NBT",
            ));
        }
        Ok(component)
    }
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

/// Flattens an already-decoded network-NBT Component value into inherited
/// styled runs. Registry codecs decode their enclosing compound separately,
/// so this is the value-level companion to [`decode_styled_component_summary`].
pub fn styled_component_runs_from_nbt_value(value: &NbtSummaryValue) -> Vec<StyledTextRun> {
    let mut runs = RunCollector::default();
    append_component_runs(value, &ComponentStyle::default(), &mut runs);
    runs.runs
}

pub(crate) fn decode_component_summary_from_decoder(decoder: &mut Decoder<'_>) -> Result<String> {
    let runs = decode_styled_component_summary_from_decoder(decoder)?;
    Ok(styled_runs_summary_text(&runs))
}

pub(crate) fn decode_rich_text_component_from_decoder(
    decoder: &mut Decoder<'_>,
) -> Result<RichTextComponent> {
    let (styled, component) = decode_styled_component_summary_with_nbt_from_decoder(decoder)?;
    let component = component.expect("validated component network NBT root is present");
    Ok(RichTextComponent {
        text: styled_runs_summary_text(&styled),
        styled,
        component,
    })
}

pub(crate) fn decode_boxed_rich_text_component_from_decoder(
    decoder: &mut Decoder<'_>,
) -> Result<Box<RichTextComponent>> {
    let (styled, component) = decode_styled_component_summary_with_nbt_from_decoder(decoder)?;
    let component = component.expect("validated component network NBT root is present");
    Ok(Box::new(RichTextComponent {
        text: styled_runs_summary_text(&styled),
        styled,
        component,
    }))
}

pub(crate) fn decode_component_style_from_decoder(
    decoder: &mut Decoder<'_>,
) -> Result<ComponentStyle> {
    let style = decode_nbt_summary_from_decoder(decoder)?.ok_or_else(|| {
        ProtocolError::InvalidData("component style network nbt root is absent".to_string())
    })?;
    codec::validate_component_style_nbt(&style, "component style network nbt")
        .map_err(ProtocolError::InvalidData)?;
    let NbtSummaryValue::Compound(entries) = style else {
        unreachable!("validated component style must be a compound")
    };
    Ok(component_style_from_entries(&entries))
}

/// Decodes a validated standalone vanilla `Style.Serializer.CODEC` value.
///
/// Chat-type registry entries use the persistent style codec while direct
/// chat-type holders use its trusted network form. Keeping both projections
/// here prevents semantic consumers from reconstructing style fields.
pub fn component_style_from_nbt_value(value: &NbtSummaryValue) -> Option<ComponentStyle> {
    codec::validate_component_style_nbt(value, "component style").ok()?;
    let NbtSummaryValue::Compound(entries) = value else {
        return None;
    };
    Some(component_style_from_entries(entries))
}

/// Extracts only the style declared directly by one already-validated Component
/// node. Presentation owners that resolve translatable content can use this to
/// preserve the same parent-chain semantics as the protocol's flattened run
/// projection without duplicating the style codec.
pub fn component_node_style_from_nbt_value(value: &NbtSummaryValue) -> ComponentStyle {
    let NbtSummaryValue::Compound(entries) = value else {
        return ComponentStyle::default();
    };
    component_style_from_entries(entries)
}

pub(crate) fn decode_styled_component_summary_from_decoder(
    decoder: &mut Decoder<'_>,
) -> Result<Vec<StyledTextRun>> {
    decode_styled_component_summary_with_nbt_from_decoder(decoder).map(|(runs, _)| runs)
}

pub(crate) fn decode_styled_component_summary_with_nbt_from_decoder(
    decoder: &mut Decoder<'_>,
) -> Result<(Vec<StyledTextRun>, Option<NbtSummaryValue>)> {
    let root = decode_nbt_summary_from_decoder(decoder)?;
    let component = root.as_ref().ok_or_else(|| {
        ProtocolError::InvalidData("component network nbt root is absent".to_string())
    })?;
    validate_component_nbt(component, "component network nbt")
        .map_err(ProtocolError::InvalidData)?;
    let mut runs = RunCollector::default();
    append_component_runs(component, &ComponentStyle::default(), &mut runs);
    Ok((runs.runs, root))
}

pub(crate) fn skip_nbt_tag_from_decoder(decoder: &mut Decoder<'_>) -> Result<()> {
    decode_nbt_summary_from_decoder(decoder).map(|_| ())
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

/// Decodes the lenient-JSON Component representation used by the login
/// disconnect packet, then delegates its text/style traversal to the same
/// semantic projection as network-NBT Components.
pub(crate) fn decode_json_component_summary(raw_json: &str) -> Result<String> {
    let value: serde_json::Value = serde_json::from_str(raw_json)
        .map_err(|error| ProtocolError::InvalidData(format!("invalid component json: {error}")))?;
    if !matches!(
        value,
        serde_json::Value::String(_) | serde_json::Value::Array(_) | serde_json::Value::Object(_)
    ) {
        return Err(ProtocolError::InvalidData(
            "component json root must be a string, array, or object".to_string(),
        ));
    }
    let value = component_json_value_to_nbt(value)?;
    validate_component_nbt(&value, "component json").map_err(ProtocolError::InvalidData)?;
    let mut runs = RunCollector::default();
    append_component_runs(&value, &ComponentStyle::default(), &mut runs);
    Ok(styled_runs_summary_text(&runs.runs))
}

fn component_json_value_to_nbt(value: serde_json::Value) -> Result<NbtSummaryValue> {
    Ok(match value {
        serde_json::Value::Null => {
            return Err(ProtocolError::InvalidData(
                "component json cannot contain null".to_string(),
            ));
        }
        serde_json::Value::Bool(value) => NbtSummaryValue::Byte(i8::from(value)),
        serde_json::Value::Number(value) => {
            if let Some(value) = value.as_i64() {
                if let Ok(value) = i32::try_from(value) {
                    NbtSummaryValue::Int(value)
                } else {
                    NbtSummaryValue::Long(value)
                }
            } else if let Some(value) = value.as_u64().and_then(|value| i64::try_from(value).ok()) {
                NbtSummaryValue::Long(value)
            } else {
                NbtSummaryValue::Double(value.as_f64().unwrap_or_default().to_bits())
            }
        }
        serde_json::Value::String(value) => NbtSummaryValue::String(value),
        serde_json::Value::Array(values) => NbtSummaryValue::List(
            values
                .into_iter()
                .map(component_json_value_to_nbt)
                .collect::<Result<_>>()?,
        ),
        serde_json::Value::Object(entries) => NbtSummaryValue::Compound(
            entries
                .into_iter()
                .map(|(name, value)| {
                    Ok(NbtSummaryEntry {
                        name,
                        value: component_json_value_to_nbt(value)?,
                    })
                })
                .collect::<Result<_>>()?,
        ),
    })
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
/// `filtered_messages` is a lenient optional list of exactly four components
/// and falls back to `messages` when absent or malformed;
/// `color` is the raw `DyeColor.CODEC` name string (`"black"` when absent —
/// `fieldOf("color").orElse(DyeColor.BLACK)`); `has_glowing_text` is the
/// `Codec.BOOL` byte (`orElse(false)`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SignTextNbt {
    pub messages: [Vec<StyledTextRun>; 4],
    pub filtered_messages: Option<[Vec<StyledTextRun>; 4]>,
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
    let root = decode_nbt_summary_from_decoder(&mut decoder)?;
    if !decoder.is_empty() {
        return Err(ProtocolError::InvalidData(
            "trailing bytes after block entity nbt".to_string(),
        ));
    }
    let Some(NbtSummaryValue::Compound(entries)) = root else {
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
        is_waxed: matches!(find_entry(&entries, "is_waxed"), Some(NbtSummaryValue::Byte(value)) if *value != 0),
    }))
}

fn decode_sign_text_nbt(entries: &[NbtSummaryEntry], key: &str) -> Option<SignTextNbt> {
    let NbtSummaryValue::Compound(text_entries) = find_entry(entries, key)? else {
        return None;
    };
    let messages = decode_sign_lines(find_entry(text_entries, "messages")?)?;
    // Vanilla wraps `filtered_messages` in `ExtraCodecs.lenientOptionalFieldOf`:
    // absence or any codec failure is the same as no filtered value, after
    // which `SignText` copies the raw messages for filtered reads.
    let filtered_messages =
        find_entry(text_entries, "filtered_messages").and_then(decode_sign_lines);
    let color = match find_entry(text_entries, "color") {
        Some(NbtSummaryValue::String(color)) => Some(color.clone()),
        _ => None,
    };
    Some(SignTextNbt {
        messages,
        filtered_messages,
        color,
        has_glowing_text: matches!(
            find_entry(text_entries, "has_glowing_text"),
            Some(NbtSummaryValue::Byte(value)) if *value != 0
        ),
    })
}

fn decode_sign_lines(value: &NbtSummaryValue) -> Option<[Vec<StyledTextRun>; 4]> {
    let NbtSummaryValue::List(lines) = value else {
        return None;
    };
    // Vanilla `SignText.LINES_CODEC` rejects any list that is not exactly four
    // components (`Util.fixedSize(_, 4)`).
    if lines.len() != 4 {
        return None;
    }
    Some(std::array::from_fn(|index| {
        let mut runs = RunCollector::default();
        append_component_runs(&lines[index], &ComponentStyle::default(), &mut runs);
        runs.runs
    }))
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

fn append_component_runs(
    value: &NbtSummaryValue,
    inherited: &ComponentStyle,
    out: &mut RunCollector,
) {
    match value {
        NbtSummaryValue::String(text) => out.push(text, inherited.clone()),
        NbtSummaryValue::List(items) => {
            for item in items {
                append_component_runs(item, inherited, out);
            }
        }
        NbtSummaryValue::Compound(entries) => {
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
        _ => {}
    }
}

fn append_primary_component_runs(
    entries: &[NbtSummaryEntry],
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
            out.push(" ", style.clone());
        }
        append_component_runs(with, style, out);
    }
}

/// Extracts the render-relevant vanilla `Style` keys from a component
/// compound (`Style.Serializer.MAP_CODEC` field names). `DynamicOps` may expose
/// booleans through any numeric tag width after registry NBT/JSON normalization;
/// the colour remains a string (`TextColor.CODEC`). Wire callers validate first;
/// direct projection keeps a defensive unset fallback for malformed values.
fn component_style_from_entries(entries: &[NbtSummaryEntry]) -> ComponentStyle {
    ComponentStyle {
        bold: style_flag(entries, "bold"),
        italic: style_flag(entries, "italic"),
        underlined: style_flag(entries, "underlined"),
        strikethrough: style_flag(entries, "strikethrough"),
        obfuscated: style_flag(entries, "obfuscated"),
        color: match find_entry(entries, "color") {
            Some(NbtSummaryValue::String(color)) => parse_text_color(color),
            _ => None,
        },
        click_event: style_click_event(entries),
        hover_event: style_hover_event(entries),
        font: match find_entry(entries, "font") {
            Some(NbtSummaryValue::String(font)) => Some(font.clone()),
            _ => None,
        },
    }
}

fn style_click_event(entries: &[NbtSummaryEntry]) -> Option<ComponentClickEvent> {
    codec::decode_component_click_event(find_entry(entries, "click_event")?, "click event").ok()
}

fn style_hover_event(entries: &[NbtSummaryEntry]) -> Option<ComponentHoverEvent> {
    let NbtSummaryValue::Compound(hover) = find_entry(entries, "hover_event")? else {
        return None;
    };
    let action = match find_entry(hover, "action")? {
        NbtSummaryValue::String(action) => action.as_str(),
        _ => return None,
    };
    match action {
        "show_text" => {
            let value = find_entry(hover, "value")?;
            let mut runs = RunCollector::default();
            append_component_runs(value, &ComponentStyle::default(), &mut runs);
            Some(ComponentHoverEvent::ShowText { value: runs.runs })
        }
        "show_item" => Some(ComponentHoverEvent::ShowItem {
            id: match find_entry(hover, "id")? {
                NbtSummaryValue::String(id) => id.clone(),
                _ => return None,
            },
            count: find_entry(hover, "count")
                .and_then(codec::component_i32)
                .unwrap_or(1),
            components: find_entry(hover, "components").cloned(),
        }),
        "show_entity" => {
            let name = find_entry(hover, "name").map(|name| {
                let mut runs = RunCollector::default();
                append_component_runs(name, &ComponentStyle::default(), &mut runs);
                runs.runs
            });
            Some(ComponentHoverEvent::ShowEntity {
                id: match find_entry(hover, "id")? {
                    NbtSummaryValue::String(id) => id.clone(),
                    _ => return None,
                },
                uuid: find_entry(hover, "uuid")?.clone(),
                name,
            })
        }
        _ => None,
    }
}

fn style_flag(entries: &[NbtSummaryEntry], key: &str) -> Option<bool> {
    codec::component_bool(find_entry(entries, key)?)
}

/// Vanilla `TextColor.parseColor`: `#`-prefixed hex literal in `0..=0xFFFFFF`,
/// otherwise an exact named `ChatFormatting` colour (`getName()` =
/// lowercased enum name, underscores kept — the `NAMED_COLORS` map).
fn parse_text_color(color: &str) -> Option<u32> {
    if let Some(hex) = color.strip_prefix('#') {
        let value = u32::from_str_radix(hex, 16).ok()?;
        return (value <= 0xFF_FF_FF).then_some(value);
    }
    vanilla_chat_formatting_color_by_name(color)
}

fn find_entry<'a>(entries: &'a [NbtSummaryEntry], key: &str) -> Option<&'a NbtSummaryValue> {
    entries
        .iter()
        .rev()
        .find(|entry| entry.name == key)
        .map(|entry| &entry.value)
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
    fn direct_component_projection_reads_duplicate_fields_last_wins() {
        let component = NbtSummaryValue::Compound(vec![
            NbtSummaryEntry {
                name: "text".to_string(),
                value: NbtSummaryValue::String("first".to_string()),
            },
            NbtSummaryEntry {
                name: "text".to_string(),
                value: NbtSummaryValue::String("last".to_string()),
            },
        ]);
        assert_eq!(
            styled_component_runs_from_nbt_value(&component),
            vec![StyledTextRun {
                text: "last".to_string(),
                style: ComponentStyle::default(),
            }]
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
                    click_event: None,
                    hover_event: None,
                    font: None,
                },
            }]
        );
    }

    #[test]
    fn component_style_projection_accepts_normalized_numeric_booleans() {
        let style = NbtSummaryValue::Compound(vec![
            NbtSummaryEntry {
                name: "bold".to_string(),
                value: NbtSummaryValue::Long(1),
            },
            NbtSummaryEntry {
                name: "italic".to_string(),
                value: NbtSummaryValue::Double(0.0f64.to_bits()),
            },
        ]);

        assert_eq!(
            component_style_from_nbt_value(&style),
            Some(ComponentStyle {
                bold: Some(true),
                italic: Some(false),
                ..ComponentStyle::default()
            })
        );
    }

    #[test]
    fn styled_runs_decode_click_event_copy_to_clipboard() {
        // {text:"debug", underlined:1b,
        //  click_event:{action:"copy_to_clipboard", value:"debug value"}}
        let mut payload = vec![10];
        write_named_string(&mut payload, "text", "debug");
        write_named_byte(&mut payload, "underlined", 1);
        write_named_compound_start(&mut payload, "click_event");
        write_named_string(&mut payload, "action", "copy_to_clipboard");
        write_named_string(&mut payload, "value", "debug value");
        payload.push(0);
        payload.push(0);

        assert_eq!(
            decode_styled_component_summary(&payload).unwrap(),
            vec![StyledTextRun {
                text: "debug".to_string(),
                style: ComponentStyle {
                    underlined: Some(true),
                    click_event: Some(ComponentClickEvent::CopyToClipboard {
                        value: "debug value".to_string(),
                    }),
                    ..ComponentStyle::default()
                },
            }]
        );
    }

    #[test]
    fn styled_runs_retain_hover_text_and_custom_font() {
        let mut payload = vec![10];
        write_named_string(&mut payload, "text", "hover me");
        write_named_string(&mut payload, "font", "example:compact");
        write_named_compound_start(&mut payload, "hover_event");
        write_named_string(&mut payload, "action", "show_text");
        write_named_compound_start(&mut payload, "value");
        write_named_string(&mut payload, "text", "details");
        write_named_byte(&mut payload, "italic", 1);
        payload.push(0);
        payload.push(0);
        payload.push(0);

        let runs = decode_styled_component_summary(&payload).unwrap();
        assert_eq!(runs.len(), 1);
        assert_eq!(runs[0].style.font.as_deref(), Some("example:compact"));
        assert_eq!(
            runs[0].style.hover_event,
            Some(ComponentHoverEvent::ShowText {
                value: vec![StyledTextRun {
                    text: "details".to_string(),
                    style: ComponentStyle {
                        italic: Some(true),
                        ..ComponentStyle::default()
                    },
                }],
            })
        );
    }

    #[test]
    fn component_wire_rejects_absent_root_and_server_disallowed_click_event() {
        assert!(decode_styled_component_summary(&[0]).is_err());

        let mut payload = vec![10];
        write_named_string(&mut payload, "text", "debug");
        write_named_compound_start(&mut payload, "click_event");
        write_named_string(&mut payload, "action", "open_file");
        write_named_string(&mut payload, "path", "screenshots/debug");
        payload.push(0);
        payload.push(0);
        let error = decode_styled_component_summary(&payload).unwrap_err();
        assert!(error.to_string().contains("not allowed from a server"));
    }

    #[test]
    fn component_style_wire_is_distinct_from_component_contents() {
        let empty_style = [10, 0];
        let mut decoder = Decoder::new(&empty_style);
        decode_component_style_from_decoder(&mut decoder).unwrap();
        assert!(decoder.is_empty());

        let mut malformed_style = vec![10];
        write_named_string(&mut malformed_style, "bold", "true");
        malformed_style.push(0);
        let mut decoder = Decoder::new(&malformed_style);
        let error = decode_component_style_from_decoder(&mut decoder).unwrap_err();
        assert!(error.to_string().contains("bold is not a boolean"));
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

        // Out-of-range hex and unknown names fail vanilla's Style codec.
        for bad in ["#1FFFFFF", "#nothex", "purple", "DARK_PURPLE"] {
            let mut payload = vec![10];
            write_named_string(&mut payload, "text", "Bad");
            write_named_string(&mut payload, "color", bad);
            payload.push(0);
            assert!(
                decode_styled_component_summary(&payload).is_err(),
                "color {bad:?} should fail component decoding"
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
            font: Some("example:child".to_string()),
            ..ComponentStyle::default()
        };
        let parent = ComponentStyle {
            bold: Some(true),
            italic: Some(true),
            color: Some(0xFF_FF_FF),
            hover_event: Some(ComponentHoverEvent::ShowText {
                value: vec![StyledTextRun {
                    text: "parent hover".to_string(),
                    style: ComponentStyle::default(),
                }],
            }),
            font: Some("example:parent".to_string()),
            ..ComponentStyle::default()
        };
        assert_eq!(
            child.apply_to(&parent),
            ComponentStyle {
                bold: Some(false),
                italic: Some(true),
                color: Some(0x12_34_56),
                hover_event: parent.hover_event,
                font: Some("example:child".to_string()),
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
        write_sign_line_list(out, "messages", lines);
        if let Some(color) = color {
            write_named_string(out, "color", color);
        }
        if glowing {
            write_named_byte(out, "has_glowing_text", 1);
        }
        out.push(0);
    }

    fn write_sign_line_list(out: &mut Vec<u8>, key: &str, lines: &[&str]) {
        out.push(9);
        write_mutf8(out, key);
        out.push(8);
        out.extend_from_slice(&(lines.len() as i32).to_be_bytes());
        for line in lines {
            write_mutf8(out, line);
        }
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
    fn sign_filtered_messages_decode_leniently_and_fall_back_when_malformed() {
        let mut payload = vec![10, 10];
        write_mutf8(&mut payload, "front_text");
        write_sign_line_list(&mut payload, "messages", &["raw", "b", "c", "d"]);
        write_sign_line_list(
            &mut payload,
            "filtered_messages",
            &["filtered", "B", "C", "D"],
        );
        payload.push(0);
        payload.push(0);

        let sign = decode_sign_block_entity_nbt(&payload).unwrap().unwrap();
        let front = sign.front_text.unwrap();
        assert_eq!(front.messages[0][0].text, "raw");
        assert_eq!(
            front.filtered_messages.as_ref().unwrap()[0][0].text,
            "filtered"
        );

        let mut malformed = vec![10, 10];
        write_mutf8(&mut malformed, "front_text");
        write_sign_line_list(&mut malformed, "messages", &["raw", "b", "c", "d"]);
        write_sign_line_list(
            &mut malformed,
            "filtered_messages",
            &["only", "three", "lines"],
        );
        malformed.push(0);
        malformed.push(0);

        let sign = decode_sign_block_entity_nbt(&malformed).unwrap().unwrap();
        let front = sign.front_text.unwrap();
        assert_eq!(front.messages[0][0].text, "raw");
        assert_eq!(front.filtered_messages, None);
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

    fn write_named_compound_start(out: &mut Vec<u8>, name: &str) {
        out.push(10);
        write_mutf8(out, name);
    }

    fn write_mutf8(out: &mut Vec<u8>, value: &str) {
        let bytes = value.as_bytes();
        out.extend_from_slice(&(bytes.len() as u16).to_be_bytes());
        out.extend_from_slice(bytes);
    }
}
