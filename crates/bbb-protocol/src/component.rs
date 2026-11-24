use serde::{de, Deserialize, Deserializer, Serialize, Serializer};

use crate::{
    chat_formattings::{ChatFormatting, VANILLA_CHAT_FORMATTINGS},
    codec::{Decoder, ProtocolError, Result},
    content_registry::ContentRegistry,
    nbt_summary::{decode_nbt_summary_from_decoder, NbtSummaryEntry, NbtSummaryValue},
};

mod builders;
mod canonical_json;
mod codec;
mod command;
mod projection;
#[cfg(test)]
mod shadow_tests;
mod sign;
#[cfg(test)]
mod tests;

#[cfg(test)]
use builders::nbt_entry;
use builders::style_entries_from_projection;
pub use builders::{
    collapsed_component_nbt, component_appended_nbt, component_keybind_nbt, component_literal_nbt,
    component_styled_literal_nbt, component_translatable_nbt, component_with_siblings_nbt,
};
#[cfg(test)]
pub(crate) use projection::decode_component_summary;
use projection::{append_component_runs, component_style_from_entries, find_entry, RunCollector};
pub use projection::{
    component_codec_values_equal, component_node_style_from_nbt_value,
    component_style_from_nbt_value, component_style_from_nbt_value_with_registry,
    component_style_from_validated_nbt_value, decode_styled_component_summary,
    decode_styled_json_component_summary, parse_text_color, styled_component_runs_from_nbt_value,
    styled_runs_summary_text,
};
pub(crate) use projection::{
    decode_boxed_rich_text_component_from_decoder_with_registry,
    decode_component_summary_and_styled, decode_component_summary_and_styled_with_registry,
    decode_component_summary_from_decoder_with_registry, decode_json_component_summary_and_styled,
    decode_rich_text_component_from_decoder, decode_rich_text_component_from_decoder_with_registry,
    decode_rich_text_style_from_decoder_with_registry,
    decode_styled_component_summary_with_nbt_from_decoder_with_registry, skip_nbt_tag_from_decoder,
};
#[cfg(test)]
pub(crate) use projection::{
    decode_component_summary_from_decoder, decode_rich_text_style_from_decoder,
    decode_styled_component_summary_with_nbt_from_decoder,
};
pub use sign::{decode_sign_block_entity_nbt, SignBlockEntityNbt, SignTextNbt};

pub use canonical_json::{
    canonical_component_codec_value, canonical_component_codec_value_with_registry,
    java_double_to_string, java_float_to_string, CanonicalComponentCodecValue,
    CanonicalComponentOwnerValue,
};
pub use codec::{
    argb_from_channels, canonical_text_color_string, component_bool, component_record_type,
    normalize_built_in_entity_type_id, normalize_built_in_entity_type_id_with_registry,
    parse_untrusted_uri_units, resolve_component_record_type, validate_chat_string_units,
    validate_component_nbt, validate_component_nbt_with_registry, validate_untrusted_uri_units,
    validate_url, visit_component_payloads, visit_component_payloads_with_registry,
    BuiltInEntityIdError, ComponentHoverAction, ComponentPayload, ComponentRecordTypeError,
    ComponentStaticClickAction, UntrustedUriAuthority, UntrustedUriIdentity, UntrustedUriKind,
    COMPONENT_CONTENT_FIELDS, COMPONENT_DATA_SOURCE_FIELDS, COMPONENT_DEFAULT_ATLAS,
    COMPONENT_OBJECT_FIELDS, COMPONENT_STYLE_FIELDS, COMPONENT_STYLE_FLAG_FIELDS,
};
pub use command::{
    command_item_predicate_component, command_known_item_predicate, command_nbt_is_too_deep,
    command_persistent_data_component, command_resource_selector_matches,
    command_stat_key_for_resource_location, command_syntax_error, command_utf16_byte_cursor,
    is_valid_block_position, is_valid_block_position_utf16, is_valid_entity_selector,
    is_valid_entity_selector_utf16, is_valid_entity_selector_utf16_with_registry,
    is_valid_entity_selector_with_registry, is_valid_nbt_path, is_valid_nbt_path_utf16,
    merge_command_nbt_compound, parse_command_coordinates_prefix,
    parse_command_coordinates_prefix_utf16, parse_command_entity_selector_prefix,
    parse_command_entity_selector_prefix_utf16,
    parse_command_entity_selector_prefix_utf16_with_registry,
    parse_command_entity_selector_prefix_with_registry, parse_command_java_uuid,
    parse_command_nbt_path_prefix, parse_command_nbt_path_value_prefix,
    parse_command_nbt_path_value_prefix_utf16, parse_command_snbt_prefix,
    parse_command_snbt_value_prefix, parse_command_snbt_value_prefix_utf16,
    parse_command_uuid_prefix, parse_command_uuid_prefix_utf16, read_command_string_value,
    read_command_string_value_utf16, scan_command_angle_argument,
    scan_command_angle_argument_utf16, scan_command_block_body, scan_command_block_body_utf16,
    scan_command_block_body_with_validation, scan_command_block_identifier,
    scan_command_block_identifier_utf16, scan_command_catalog_argument_utf16,
    scan_command_color_argument, scan_command_context_free_argument,
    scan_command_context_free_argument_utf16, scan_command_coordinates_argument,
    scan_command_coordinates_argument_utf16, scan_command_degrees_range_argument_utf16,
    scan_command_entity_anchor_argument, scan_command_entity_argument,
    scan_command_float_range_argument, scan_command_game_mode_argument,
    scan_command_game_profile_argument, scan_command_game_profile_argument_utf16,
    scan_command_heightmap_argument, scan_command_hex_color_argument,
    scan_command_hex_color_argument_utf16, scan_command_int_range_argument,
    scan_command_item_predicate_head, scan_command_item_predicate_head_utf16,
    scan_command_item_slot_argument, scan_command_nbt_path_argument, scan_command_number_lexeme,
    scan_command_number_lexeme_utf16, scan_command_operation_argument,
    scan_command_operation_argument_utf16, scan_command_particle_options,
    scan_command_particle_options_utf16, scan_command_quotable_string,
    scan_command_quotable_string_utf16, scan_command_range_argument_utf16,
    scan_command_resource_location, scan_command_resource_location_argument,
    scan_command_resource_location_argument_utf16, scan_command_resource_location_utf16,
    scan_command_resource_or_id, scan_command_resource_or_id_utf16, scan_command_resource_selector,
    scan_command_resource_selector_utf16, scan_command_score_holder_argument,
    scan_command_scoreboard_slot_argument, scan_command_snbt_argument,
    scan_command_snbt_argument_utf16, scan_command_swizzle_argument,
    scan_command_swizzle_argument_utf16, scan_command_template_mirror_argument,
    scan_command_template_rotation_argument, scan_command_text_argument,
    scan_command_text_argument_utf16, scan_command_time_argument, scan_command_time_argument_utf16,
    scan_command_token, scan_command_token_utf16, scan_command_unquoted_string,
    scan_command_unquoted_string_utf16, scan_command_uuid_argument,
    scan_command_uuid_argument_utf16, scan_command_whitespace, scan_command_whitespace_utf16,
    CommandCoordinatesKind, CommandEntitySelectorOption, CommandEntitySelectorOptionKind,
    CommandEntitySelectorPrefix, CommandItemComponentPatchScanner,
    CommandItemComponentPatchScannerUtf16, CommandItemComponentSuggestions,
    CommandItemPredicateSuggestions, CommandItemPredicateTestScanner,
    CommandItemPredicateTestScannerUtf16, CommandNbtPath, CommandNbtPathErrorKind,
    CommandNbtPathNode, CommandNbtPathNodeKind, CommandNbtPathParseError, CommandSnbtKind,
    CommandSyntaxError, CommandSyntaxErrorKind, CommandSyntaxErrorUtf16, ScannedCommandBlockBody,
    ScannedCommandBlockBodyUtf16, ScannedCommandBlockIdentifier,
    ScannedCommandBlockIdentifierUtf16, ScannedCommandItemComponent,
    ScannedCommandItemComponentUtf16, ScannedCommandItemPredicateHead,
    ScannedCommandItemPredicateHeadUtf16, ScannedCommandItemPredicateTerm,
    ScannedCommandItemPredicateTermUtf16, ScannedCommandItemPredicateTest,
    ScannedCommandItemPredicateTestUtf16, ScannedCommandParticleOptions,
    ScannedCommandParticleOptionsUtf16, ScannedCommandResourceOrId,
    ScannedCommandResourceOrIdUtf16, VANILLA_COMMAND_ENTITY_SELECTOR_OPTIONS,
};

/// The client-visible subset of vanilla `net.minecraft.network.chat.Style`
/// (`Style.Serializer.MAP_CODEC`). Each key is `None` when the component chain
/// never set it, mirroring vanilla's nullable style fields so downstream
/// default-style merges (`ComponentUtils.mergeStyles`, e.g. the lore style)
/// can still fill unset keys.
#[derive(Debug, Clone, Eq, Default, Serialize, Deserialize)]
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shadow_color: Option<u32>,
    /// Vanilla `TextColor.name` (`TextColor.java:25`): the `ChatFormatting`
    /// this colour was spelled with, or `None` for an anonymous RGB colour that
    /// came from (and goes back out as) a `#RRGGBB` literal.
    ///
    /// The pair mirrors vanilla's own two `TextColor` fields rather than adding
    /// a second colour source: only `serialize` (`TextColor.java:41-46`) reads
    /// the name, every renderer keeps reading the resolved value. Set the two
    /// together — [`ComponentStyle::with_chat_formatting_color`] is the only
    /// way that cannot disagree, and encoding refuses a name whose
    /// `ChatFormatting` colour is not the resolved value.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color_name: Option<ChatFormatting>,
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
    /// Text inserted by Shift-click in `ChatScreen`. Like every other style
    /// field this inherits through `Style.applyTo`; it is not itself a click
    /// event and consumers must ignore it outside insertion-click mode.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        with = "insertion_serde"
    )]
    pub insertion: Option<crate::JavaString>,
    /// Exact resolved font identifier. Unsupported font providers may fall
    /// back visually, but trust evaluation and cache identity must not lose it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub font: Option<String>,
}

mod insertion_serde {
    use serde::{Deserialize, Serialize};

    #[derive(Serialize, Deserialize)]
    #[serde(untagged)]
    enum Value {
        Text(String),
        Units(Vec<u16>),
    }

    pub fn serialize<S: serde::Serializer>(
        value: &Option<crate::JavaString>,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        value
            .as_ref()
            .map(|value| match String::from_utf16(value.units()) {
                Ok(text) => Value::Text(text),
                Err(_) => Value::Units(value.units().to_vec()),
            })
            .serialize(serializer)
    }

    pub fn deserialize<'de, D: serde::Deserializer<'de>>(
        deserializer: D,
    ) -> Result<Option<crate::JavaString>, D::Error> {
        Ok(
            Option::<Value>::deserialize(deserializer)?.map(|value| match value {
                Value::Text(text) => text.into(),
                Value::Units(units) => units.into(),
            }),
        )
    }
}

/// Vanilla `Style.equals` compares colours through `TextColor.equals`
/// (`Style.java:576-593`), which looks at the resolved value alone and ignores
/// the name (`TextColor.java:51-58`). So a run coloured `red` and a run
/// coloured `#FF5555` are one style in vanilla — adjacent-run merging and the
/// `withColor` short circuit (`Style.java:137-141`) both rely on it — and
/// equality here has to skip `color_name` for the same reason.
///
/// The exhaustive destructuring (no `..`) makes a newly added style key a
/// compile error rather than a silently unread field.
impl PartialEq for ComponentStyle {
    fn eq(&self, other: &Self) -> bool {
        let Self {
            bold,
            italic,
            underlined,
            strikethrough,
            obfuscated,
            color,
            color_name: _,
            shadow_color,
            click_event,
            hover_event,
            insertion,
            font,
        } = self;
        *bold == other.bold
            && *italic == other.italic
            && *underlined == other.underlined
            && *strikethrough == other.strikethrough
            && *obfuscated == other.obfuscated
            && *color == other.color
            && *shadow_color == other.shadow_color
            && *click_event == other.click_event
            && *hover_event == other.hover_event
            && *insertion == other.insertion
            && *font == other.font
    }
}

/// Supported vanilla `ClickEvent` payloads in the same externally tagged shape
/// as the 26.1 codec (`action` plus the action-specific value key).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "action")]
pub enum ComponentClickEvent {
    /// Vanilla permits this for locally-created Components but rejects it from
    /// the server-facing serialized `ClickEvent.Action.CODEC`.
    #[serde(rename = "open_file")]
    OpenFile { path: crate::JavaString },
    #[serde(rename = "open_url")]
    OpenUrl { url: crate::JavaString },
    #[serde(rename = "run_command")]
    RunCommand { command: crate::JavaString },
    #[serde(rename = "suggest_command")]
    SuggestCommand { command: crate::JavaString },
    #[serde(rename = "show_dialog")]
    ShowDialog { dialog: NbtSummaryValue },
    #[serde(rename = "change_page")]
    ChangePage { page: i32 },
    #[serde(rename = "copy_to_clipboard")]
    CopyToClipboard { value: crate::JavaString },
    #[serde(rename = "custom")]
    Custom {
        id: String,
        #[serde(default)]
        payload: Option<NbtSummaryValue>,
    },
}

impl PartialEq for ComponentClickEvent {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::OpenUrl { url: left }, Self::OpenUrl { url: right }) => {
                match (
                    parse_untrusted_uri_units(left, "url"),
                    parse_untrusted_uri_units(right, "url"),
                ) {
                    (Ok(left), Ok(right)) => left == right,
                    // Local construction and serde can bypass the wire codec.
                    _ => left == right,
                }
            }
            (Self::OpenFile { path: left }, Self::OpenFile { path: right }) => left == right,
            (Self::RunCommand { command: left }, Self::RunCommand { command: right })
            | (Self::SuggestCommand { command: left }, Self::SuggestCommand { command: right }) => {
                left == right
            }
            (Self::ShowDialog { dialog: left }, Self::ShowDialog { dialog: right }) => {
                left == right
            }
            (Self::ChangePage { page: left }, Self::ChangePage { page: right }) => left == right,
            (Self::CopyToClipboard { value: left }, Self::CopyToClipboard { value: right }) => {
                left == right
            }
            (
                Self::Custom {
                    id: left,
                    payload: left_payload,
                },
                Self::Custom {
                    id: right,
                    payload: right_payload,
                },
            ) => left == right && left_payload == right_payload,
            _ => false,
        }
    }
}

impl Eq for ComponentClickEvent {}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum ComponentHoverEvent {
    ShowText {
        value: Vec<StyledTextRun>,
        /// Exact nested Component value retained beside the flattened runs so
        /// resource-language presentation can resolve translatable contents.
        #[serde(default)]
        component: Option<NbtSummaryValue>,
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
        /// Exact optional name Component retained for localized tooltip
        /// presentation. Older serialized state may carry only `name`.
        #[serde(default)]
        name_component: Option<NbtSummaryValue>,
    },
}

/// Formats the lenient UUID representation accepted by the Component
/// `show_entity` codec as Vanilla's canonical hyphenated UUID line.
pub fn component_hover_uuid(value: &NbtSummaryValue) -> Option<uuid::Uuid> {
    component_uuid_words(value).map(crate::nbt_summary::uuid_from_words)
}

fn component_uuid_words(value: &NbtSummaryValue) -> Option<[i32; 4]> {
    Some(match value {
        NbtSummaryValue::IntArray(values) => values.as_slice().try_into().ok()?,
        NbtSummaryValue::ByteArray(values) if values.len() == 4 => {
            std::array::from_fn(|index| i32::from(values[index]))
        }
        NbtSummaryValue::LongArray(values) if values.len() == 4 => {
            std::array::from_fn(|index| values[index] as i32)
        }
        NbtSummaryValue::List(values) if values.len() == 4 => {
            let mut words = [0; 4];
            for (word, value) in words.iter_mut().zip(values) {
                *word = codec::component_i32(value)?;
            }
            words
        }
        NbtSummaryValue::String(value) => value.with_utf8(parse_uuid_words).flatten()?,
        _ => return None,
    })
}

pub fn parse_uuid_words(value: &str) -> Option<[i32; 4]> {
    let packed = parse_command_java_uuid(value)?;
    Some([
        (packed >> 96) as i32,
        (packed >> 64) as i32,
        (packed >> 32) as i32,
        packed as i32,
    ])
}

/// The canonical form `UUIDUtil.CODEC` writes: the four big-endian 32-bit
/// words of the UUID as an int array. `UUIDUtil.LENIENT_CODEC` accepts the
/// string spelling too, but `Codec.withAlternative` encodes through the first
/// codec, so a server-built Component always carries the array.
pub use crate::nbt_summary::uuid_nbt as component_hover_uuid_nbt;

impl ComponentStyle {
    /// Vanilla `Style.withColor(ChatFormatting)` (`Style.java:159-161`), which
    /// resolves the constant through `TextColor.fromLegacyFormat`
    /// (`TextColor.java:69-71`) and so keeps the name beside the value.
    ///
    /// `None` for a formatting that is not a colour — `fromLegacyFormat` reads
    /// the `isColor`-filtered `LEGACY_FORMAT_TO_COLOR` map (`TextColor.java:18-20`)
    /// and answers null for `BOLD`, `RESET` and the other format codes.
    pub fn with_chat_formatting_color(formatting: ChatFormatting) -> Option<ComponentStyle> {
        Some(ComponentStyle {
            color: Some(formatting.color()?),
            color_name: Some(formatting),
            ..ComponentStyle::default()
        })
    }

    /// Vanilla `Style.applyTo(other)`: keys set on `self` win, unset keys
    /// inherit from `parent`.
    pub fn apply_to(&self, parent: &ComponentStyle) -> ComponentStyle {
        // `applyTo` inherits the whole `TextColor` (`Style.java:508`), so the
        // value and the name it was spelled with always travel together.
        let (color, color_name) = match self.color {
            Some(_) => (self.color, self.color_name),
            None => (parent.color, parent.color_name),
        };
        ComponentStyle {
            bold: self.bold.or(parent.bold),
            italic: self.italic.or(parent.italic),
            underlined: self.underlined.or(parent.underlined),
            strikethrough: self.strikethrough.or(parent.strikethrough),
            obfuscated: self.obfuscated.or(parent.obfuscated),
            color,
            color_name,
            shadow_color: self.shadow_color.or(parent.shadow_color),
            click_event: self
                .click_event
                .clone()
                .or_else(|| parent.click_event.clone()),
            hover_event: self
                .hover_event
                .clone()
                .or_else(|| parent.hover_event.clone()),
            insertion: self.insertion.clone().or_else(|| parent.insertion.clone()),
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
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RichTextComponent {
    multi_run_text: Option<Box<str>>,
    styled: Vec<StyledTextRun>,
    component: NbtSummaryValue,
}

impl RichTextComponent {
    pub fn try_from_component_nbt(component: NbtSummaryValue) -> std::result::Result<Self, String> {
        Self::try_from_component_nbt_with_registry(component, ContentRegistry::vanilla())
    }

    pub fn try_from_component_nbt_with_registry(
        component: NbtSummaryValue,
        registry: &ContentRegistry,
    ) -> std::result::Result<Self, String> {
        validate_component_nbt_with_registry(&component, "rich text component", registry)?;
        Ok(Self::from_decoded_component(component))
    }

    fn from_decoded_component(component: NbtSummaryValue) -> Self {
        let styled = styled_component_runs_from_nbt_value(&component);
        Self::from_styled_and_component(styled, component)
    }

    pub(crate) fn from_styled_and_component(
        styled: Vec<StyledTextRun>,
        component: NbtSummaryValue,
    ) -> Self {
        let multi_run_text = match styled.as_slice() {
            [] | [_] => None,
            runs => {
                let total_len: usize = runs.iter().map(|run| run.text.len()).sum();
                if total_len == 0 {
                    None
                } else {
                    let mut out = String::with_capacity(total_len);
                    for run in runs {
                        out.push_str(&run.text);
                    }
                    Some(out.into_boxed_str())
                }
            }
        };
        Self {
            multi_run_text,
            styled,
            component,
        }
    }

    pub fn plain(text: impl Into<String>) -> Self {
        Self::from_decoded_component(NbtSummaryValue::String(text.into().into()))
    }

    pub fn text(&self) -> &str {
        if let Some(text) = self.multi_run_text.as_deref() {
            return text;
        }
        match self.styled.first() {
            Some(run) if !run.text.is_empty() => &run.text,
            _ => "component nbt",
        }
    }

    pub fn styled(&self) -> &[StyledTextRun] {
        &self.styled
    }

    // Component.java:71-86: sibling text visits remain separate even with identical style.
    pub fn source_styled_parts(&self) -> Vec<StyledTextRun> {
        let mut parts = RunCollector {
            preserve_parts: true,
            ..Default::default()
        };
        append_component_runs(&self.component, &ComponentStyle::default(), &mut parts);
        parts.runs
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

impl Serialize for RichTextComponent {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut state = serializer.serialize_struct("RichTextComponent", 3)?;
        state.serialize_field("text", self.text())?;
        state.serialize_field("styled", &self.styled)?;
        state.serialize_field("component", &self.component)?;
        state.end()
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
        if component.text() != fields.text || component.styled != fields.styled {
            return Err(de::Error::custom(
                "rich text component projections do not match exact component NBT",
            ));
        }
        Ok(component)
    }
}

/// One trusted standalone `Style.Serializer.CODEC` value, preserving the exact
/// validated NBT beside its presentation projection.
///
/// Direct chat-type holders and styled scoreboard number formats carry a
/// `Style` without an enclosing Component. Keeping the source value here lets
/// the world/session owner validate nested click/hover Component payloads
/// before canonical mutation.
#[derive(Debug, Clone, PartialEq, Eq)]
struct RichTextStyleInner {
    style: ComponentStyle,
    component: NbtSummaryValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RichTextStyle(Box<RichTextStyleInner>);

impl RichTextStyle {
    pub fn try_from_style_nbt(component: NbtSummaryValue) -> std::result::Result<Self, String> {
        Self::try_from_style_nbt_with_registry(component, ContentRegistry::vanilla())
    }

    pub fn try_from_style_nbt_with_registry(
        component: NbtSummaryValue,
        registry: &ContentRegistry,
    ) -> std::result::Result<Self, String> {
        codec::validate_component_style_nbt_with_registry(&component, "rich text style", registry)?;
        let NbtSummaryValue::Compound(entries) = &component else {
            unreachable!("validated component style must be a compound")
        };
        Ok(Self(Box::new(RichTextStyleInner {
            style: component_style_from_entries(entries),
            component,
        })))
    }

    /// Builds an exact valid standalone Style value from a semantic
    /// projection. This is primarily useful for typed fixtures and local
    /// values; network decode always keeps the server's original NBT.
    pub fn try_from_style(style: ComponentStyle) -> std::result::Result<Self, String> {
        let component = NbtSummaryValue::Compound(style_entries_from_projection(&style)?);
        let rich = Self::try_from_style_nbt(component)?;
        if rich.0.style != style {
            return Err("component style could not be represented exactly".to_string());
        }
        Ok(rich)
    }

    pub fn style(&self) -> &ComponentStyle {
        &self.0.style
    }

    pub fn component(&self) -> &NbtSummaryValue {
        &self.0.component
    }
}

impl Serialize for RichTextStyle {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut state = serializer.serialize_struct("RichTextStyle", 2)?;
        state.serialize_field("style", &self.0.style)?;
        state.serialize_field("component", &self.0.component)?;
        state.end()
    }
}

impl Default for RichTextStyle {
    fn default() -> Self {
        Self::try_from_style_nbt(NbtSummaryValue::Compound(Vec::new()))
            .expect("empty component style is valid")
    }
}

impl<'de> Deserialize<'de> for RichTextStyle {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct RichTextStyleFields {
            style: ComponentStyle,
            component: NbtSummaryValue,
        }

        let fields = RichTextStyleFields::deserialize(deserializer)?;
        let style = Self::try_from_style_nbt(fields.component).map_err(de::Error::custom)?;
        if style.0.style != fields.style {
            return Err(de::Error::custom(
                "rich text style projection does not match exact style NBT",
            ));
        }
        Ok(style)
    }
}
