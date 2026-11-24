use super::*;

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

/// Compares two serialized Component codec values after decoding and canonical
/// re-encoding them. This mirrors `Component.equals`: JSON/NBT shorthand may
/// differ while content, sibling structure, and style remain identity-bearing.
pub fn component_codec_values_equal(
    left: &NbtSummaryValue,
    right: &NbtSummaryValue,
) -> Result<bool> {
    if let (NbtSummaryValue::String(left), NbtSummaryValue::String(right)) = (left, right) {
        return Ok(left == right);
    }
    if left == right {
        if let NbtSummaryValue::Compound(entries) = left {
            if let [entry] = entries.as_slice() {
                if (entry.name == "text" || entry.name == "translate")
                    && matches!(&entry.value, NbtSummaryValue::String(_))
                {
                    validate_component_nbt(left, "component")
                        .map_err(ProtocolError::InvalidData)?;
                    return Ok(true);
                }
            }
        }
    }
    let left = canonical_component_codec_value(left).map_err(ProtocolError::InvalidData)?;
    let right = canonical_component_codec_value(right).map_err(ProtocolError::InvalidData)?;
    Ok(left == right)
}

#[cfg(test)]
pub(crate) fn decode_component_summary_from_decoder(decoder: &mut Decoder<'_>) -> Result<String> {
    decode_component_summary_from_decoder_with_registry(
        decoder,
        crate::content_registry::ContentRegistry::vanilla(),
    )
}

pub(crate) fn decode_component_summary_from_decoder_with_registry(
    decoder: &mut Decoder<'_>,
    registry: &crate::content_registry::ContentRegistry,
) -> Result<String> {
    let runs =
        decode_styled_component_summary_with_nbt_from_decoder_with_registry(decoder, registry)?.0;
    Ok(styled_runs_summary_text(&runs))
}

pub(crate) fn decode_rich_text_component_from_decoder(
    decoder: &mut Decoder<'_>,
) -> Result<RichTextComponent> {
    decode_rich_text_component_from_decoder_with_registry(
        decoder,
        crate::content_registry::ContentRegistry::vanilla(),
    )
}

pub(crate) fn decode_rich_text_component_from_decoder_with_registry(
    decoder: &mut Decoder<'_>,
    registry: &crate::content_registry::ContentRegistry,
) -> Result<RichTextComponent> {
    let (styled, component) =
        decode_styled_component_summary_with_nbt_from_decoder_with_registry(decoder, registry)?;
    let component = component.expect("validated component network NBT root is present");
    Ok(RichTextComponent::from_styled_and_component(
        styled, component,
    ))
}

pub(crate) fn decode_boxed_rich_text_component_from_decoder_with_registry(
    decoder: &mut Decoder<'_>,
    registry: &crate::content_registry::ContentRegistry,
) -> Result<Box<RichTextComponent>> {
    let (styled, component) =
        decode_styled_component_summary_with_nbt_from_decoder_with_registry(decoder, registry)?;
    let component = component.expect("validated component network NBT root is present");
    Ok(Box::new(RichTextComponent::from_styled_and_component(
        styled, component,
    )))
}

#[cfg(test)]
pub(crate) fn decode_rich_text_style_from_decoder(
    decoder: &mut Decoder<'_>,
) -> Result<RichTextStyle> {
    decode_rich_text_style_from_decoder_with_registry(
        decoder,
        crate::content_registry::ContentRegistry::vanilla(),
    )
}

pub(crate) fn decode_rich_text_style_from_decoder_with_registry(
    decoder: &mut Decoder<'_>,
    registry: &crate::content_registry::ContentRegistry,
) -> Result<RichTextStyle> {
    let style = decode_nbt_summary_from_decoder(decoder)?.ok_or_else(|| {
        ProtocolError::InvalidData("component style network nbt root is absent".to_string())
    })?;
    RichTextStyle::try_from_style_nbt_with_registry(style, registry)
        .map_err(ProtocolError::InvalidData)
}

/// Decodes a validated standalone vanilla `Style.Serializer.CODEC` value.
///
/// Chat-type registry entries use the persistent style codec while direct
/// chat-type holders use its trusted network form. Keeping both projections
/// here prevents semantic consumers from reconstructing style fields.
pub fn component_style_from_nbt_value(value: &NbtSummaryValue) -> Option<ComponentStyle> {
    component_style_from_nbt_value_with_registry(
        value,
        crate::content_registry::ContentRegistry::vanilla(),
    )
}

pub fn component_style_from_nbt_value_with_registry(
    value: &NbtSummaryValue,
    registry: &crate::content_registry::ContentRegistry,
) -> Option<ComponentStyle> {
    codec::validate_component_style_nbt_with_registry(value, "component style", registry).ok()?;
    component_style_from_validated_nbt_value(value)
}

/// Projects a `Style.Serializer.CODEC` value after its source-aware owner has
/// already validated and canonically encoded it. Synchronized registry typed
/// projections use this seam so the older network presentation validator
/// cannot apply a second, narrower URI policy to an exact value.
pub fn component_style_from_validated_nbt_value(value: &NbtSummaryValue) -> Option<ComponentStyle> {
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

pub(crate) fn decode_styled_component_summary_from_decoder_with_registry(
    decoder: &mut Decoder<'_>,
    registry: &crate::content_registry::ContentRegistry,
) -> Result<Vec<StyledTextRun>> {
    decode_styled_component_summary_with_nbt_from_decoder_with_registry(decoder, registry)
        .map(|(runs, _)| runs)
}

#[cfg(test)]
pub(crate) fn decode_styled_component_summary_with_nbt_from_decoder(
    decoder: &mut Decoder<'_>,
) -> Result<(Vec<StyledTextRun>, Option<NbtSummaryValue>)> {
    decode_styled_component_summary_with_nbt_from_decoder_with_registry(
        decoder,
        crate::content_registry::ContentRegistry::vanilla(),
    )
}

pub(crate) fn decode_styled_component_summary_with_nbt_from_decoder_with_registry(
    decoder: &mut Decoder<'_>,
    registry: &crate::content_registry::ContentRegistry,
) -> Result<(Vec<StyledTextRun>, Option<NbtSummaryValue>)> {
    let root = decode_nbt_summary_from_decoder(decoder)?;
    let component = root.as_ref().ok_or_else(|| {
        ProtocolError::InvalidData("component network nbt root is absent".to_string())
    })?;
    validate_component_nbt_with_registry(component, "component network nbt", registry)
        .map_err(ProtocolError::InvalidData)?;
    let mut runs = RunCollector::default();
    append_component_runs(component, &ComponentStyle::default(), &mut runs);
    Ok((runs.runs, root))
}

pub(crate) fn skip_nbt_tag_from_decoder(decoder: &mut Decoder<'_>) -> Result<()> {
    crate::nbt_summary::skip_network_nbt_tag(decoder, crate::nbt_summary::SkipNbtLimits::NETWORK)
}

pub(crate) fn decode_component_summary_and_styled_with_registry(
    payload: &[u8],
    registry: &crate::content_registry::ContentRegistry,
) -> Result<(String, Vec<StyledTextRun>)> {
    let runs = decode_styled_component_summary_with_registry(payload, registry)?;
    let summary = styled_runs_summary_text(&runs);
    let reason = if summary == "component nbt" {
        format!("component nbt ({} bytes)", payload.len())
    } else {
        summary
    };
    Ok((reason, runs))
}

pub(crate) fn decode_component_summary_and_styled(
    payload: &[u8],
) -> Result<(String, Vec<StyledTextRun>)> {
    decode_component_summary_and_styled_with_registry(
        payload,
        crate::content_registry::ContentRegistry::vanilla(),
    )
}

#[cfg(test)]
pub(crate) fn decode_component_summary(payload: &[u8]) -> Result<String> {
    decode_component_summary_and_styled(payload).map(|(summary, _)| summary)
}

/// Decodes the lenient-JSON Component representation used by the login
/// disconnect packet, then delegates its text/style traversal to the same
/// semantic projection as network-NBT Components.
fn decode_styled_json_component_summary_inner(raw_json: &str) -> Result<Vec<StyledTextRun>> {
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
    let value = crate::json_value_to_nbt_summary(&value)?;
    validate_component_nbt(&value, "component json").map_err(ProtocolError::InvalidData)?;
    let mut runs = RunCollector::default();
    append_component_runs(&value, &ComponentStyle::default(), &mut runs);
    Ok(runs.runs)
}

pub(crate) fn decode_json_component_summary_and_styled(
    raw_json: &str,
) -> Result<(String, Vec<StyledTextRun>)> {
    let runs = decode_styled_json_component_summary_inner(raw_json)?;
    let summary = styled_runs_summary_text(&runs);
    Ok((summary, runs))
}

/// Decodes the lenient-JSON Component representation used by login disconnect
/// while retaining the same inherited style projection as network-NBT
/// Components. Both public styled decoders delegate to the shared Component
/// validator/traversal, so terminal-screen consumers cannot drift from packet
/// summary semantics.
pub fn decode_styled_json_component_summary(raw_json: &str) -> Result<Vec<StyledTextRun>> {
    decode_styled_json_component_summary_inner(raw_json)
}

/// Decodes a network chat component into flattened styled runs. The companion
/// of [`decode_component_summary`]: same NBT traversal and text extraction,
/// but each contiguous piece of text keeps its inherited vanilla `Style`
/// subset instead of being flattened to a bare string.
pub fn decode_styled_component_summary(payload: &[u8]) -> Result<Vec<StyledTextRun>> {
    decode_styled_component_summary_with_registry(
        payload,
        crate::content_registry::ContentRegistry::vanilla(),
    )
}

pub(crate) fn decode_styled_component_summary_with_registry(
    payload: &[u8],
    registry: &crate::content_registry::ContentRegistry,
) -> Result<Vec<StyledTextRun>> {
    let mut decoder = Decoder::new(payload);
    let runs = decode_styled_component_summary_from_decoder_with_registry(&mut decoder, registry)?;
    if !decoder.is_empty() {
        return Err(ProtocolError::InvalidData(
            "trailing bytes after component nbt".to_string(),
        ));
    }
    Ok(runs)
}

const MAX_PROJECTION_NODES: usize = 65_536;
const MAX_PROJECTION_RUNS: usize = 16_384;
const MAX_PROJECTION_UTF16_UNITS: usize = 1_048_576;

struct ProjectionBudget {
    remaining_nodes: usize,
    remaining_runs: usize,
    remaining_units: usize,
}

impl Default for ProjectionBudget {
    fn default() -> Self {
        Self {
            remaining_nodes: MAX_PROJECTION_NODES,
            remaining_runs: MAX_PROJECTION_RUNS,
            remaining_units: MAX_PROJECTION_UTF16_UNITS,
        }
    }
}

impl ProjectionBudget {
    #[inline]
    fn take_node(&mut self) -> bool {
        if self.remaining_nodes == 0 {
            return false;
        }
        self.remaining_nodes -= 1;
        true
    }
}

// Source projection preserves node boundaries; stored summaries coalesce equal styles.
#[derive(Default)]
pub(super) struct RunCollector {
    pub(super) runs: Vec<StyledTextRun>,
    pub(super) preserve_parts: bool,
}

impl RunCollector {
    fn push_budgeted(
        &mut self,
        text: &[u16],
        style: ComponentStyle,
        budget: &mut ProjectionBudget,
    ) {
        if text.is_empty() || budget.remaining_units == 0 {
            return;
        }
        let take = text.len().min(budget.remaining_units);
        budget.remaining_units -= take;
        let slice = &text[..take];
        if let Some(last) = self.runs.last_mut() {
            if !self.preserve_parts && last.style == style {
                last.text.extend(
                    char::decode_utf16(slice.iter().copied())
                        .map(|value| value.unwrap_or(char::REPLACEMENT_CHARACTER)),
                );
                return;
            }
        }
        if budget.remaining_runs == 0 {
            return;
        }
        budget.remaining_runs -= 1;
        self.runs.push(StyledTextRun {
            text: String::from_utf16_lossy(slice),
            style,
        });
    }

    fn is_empty(&self) -> bool {
        self.runs.is_empty()
    }
}

pub(super) fn append_component_runs(
    value: &NbtSummaryValue,
    inherited: &ComponentStyle,
    out: &mut RunCollector,
) {
    let mut budget = ProjectionBudget::default();
    let _ = append_component_runs_with_budget(value, inherited, out, &mut budget);
}

fn append_component_runs_with_budget(
    value: &NbtSummaryValue,
    inherited: &ComponentStyle,
    out: &mut RunCollector,
    budget: &mut ProjectionBudget,
) -> ComponentStyle {
    #[cfg(test)]
    crate::test_stack_budget::record();

    if !budget.take_node() {
        return inherited.clone();
    }

    match value {
        NbtSummaryValue::String(text) => {
            out.push_budgeted(text.units(), inherited.clone(), budget);
            inherited.clone()
        }
        NbtSummaryValue::List(items) => {
            // ComponentSerialization.java:74-82: list entries are siblings of the first root.
            // Evaluate the first element once to obtain its effective style without re-entering
            // its hover_event subtree on a second pass.
            let Some((first, rest)) = items.split_first() else {
                return inherited.clone();
            };
            let style = append_component_runs_with_budget(first, inherited, out, budget);
            for item in rest {
                if budget.remaining_nodes == 0 {
                    break;
                }
                let _ = append_component_runs_with_budget(item, &style, out, budget);
            }
            style
        }
        NbtSummaryValue::Compound(entries) => {
            // Vanilla component inheritance: the node's own style keys win,
            // unset keys inherit the parent chain (`Style.applyTo` inside
            // `Component.visit`); siblings in `extra` inherit this node's
            // effective style.
            let style =
                component_style_from_entries_with_budget(entries, budget).apply_to(inherited);
            append_primary_component_runs(entries, &style, out, budget);
            if let Some(extra) = find_entry(entries, "extra") {
                append_component_members(extra, &style, out, budget);
            }
            style
        }
        _ => inherited.clone(),
    }
}

fn append_component_members(
    value: &NbtSummaryValue,
    inherited: &ComponentStyle,
    out: &mut RunCollector,
    budget: &mut ProjectionBudget,
) {
    // Member arrays are not component roots; each member inherits the owning node.
    if let NbtSummaryValue::List(items) = value {
        for item in items {
            if budget.remaining_nodes == 0 {
                break;
            }
            let _ = append_component_runs_with_budget(item, inherited, out, budget);
        }
    } else {
        let _ = append_component_runs_with_budget(value, inherited, out, budget);
    }
}

fn append_primary_component_runs(
    entries: &[NbtSummaryEntry],
    style: &ComponentStyle,
    out: &mut RunCollector,
    budget: &mut ProjectionBudget,
) {
    if let Some(text) = find_entry(entries, "text") {
        let _ = append_component_runs_with_budget(text, style, out, budget);
        return;
    }

    if let Some(fallback) = find_entry(entries, "fallback") {
        let _ = append_component_runs_with_budget(fallback, style, out, budget);
    } else if let Some(translate) = find_entry(entries, "translate") {
        let _ = append_component_runs_with_budget(translate, style, out, budget);
    } else if let Some(keybind) = find_entry(entries, "keybind") {
        let _ = append_component_runs_with_budget(keybind, style, out, budget);
    } else if let Some(selector) = find_entry(entries, "selector") {
        let _ = append_component_runs_with_budget(selector, style, out, budget);
    } else if let Some(nbt) = find_entry(entries, "nbt") {
        let _ = append_component_runs_with_budget(nbt, style, out, budget);
    }

    if let Some(with) = find_entry(entries, "with") {
        if !out.is_empty() {
            out.push_budgeted(&[b' ' as u16], style.clone(), budget);
        }
        append_component_members(with, style, out, budget);
    }
}

/// Extracts the render-relevant vanilla `Style` keys from a component
/// compound (`Style.Serializer.MAP_CODEC` field names). `DynamicOps` may expose
/// booleans through any numeric tag width after registry NBT/JSON normalization;
/// the colour remains a string (`TextColor.CODEC`). Wire callers validate first;
/// direct projection keeps a defensive unset fallback for malformed values.
pub(super) fn component_style_from_entries(entries: &[NbtSummaryEntry]) -> ComponentStyle {
    let mut budget = ProjectionBudget::default();
    component_style_from_entries_with_budget(entries, &mut budget)
}

fn component_style_from_entries_with_budget(
    entries: &[NbtSummaryEntry],
    budget: &mut ProjectionBudget,
) -> ComponentStyle {
    // The value and the name it was spelled with come out of one parse so the
    // two `TextColor` fields cannot drift apart.
    let color = match find_entry(entries, "color") {
        Some(NbtSummaryValue::String(color)) => color
            .try_to_string()
            .ok()
            .as_deref()
            .and_then(parse_text_color),
        _ => None,
    };
    ComponentStyle {
        bold: style_flag(entries, "bold"),
        italic: style_flag(entries, "italic"),
        underlined: style_flag(entries, "underlined"),
        strikethrough: style_flag(entries, "strikethrough"),
        obfuscated: style_flag(entries, "obfuscated"),
        color: color.map(|(value, _)| value),
        color_name: color.and_then(|(_, name)| name),
        shadow_color: find_entry(entries, "shadow_color")
            .and_then(canonical_json::canonical_argb_color)
            .map(|value| value as u32),
        click_event: style_click_event(entries),
        hover_event: style_hover_event(entries, budget),
        insertion: find_entry(entries, "insertion").and_then(crate::JavaString::from_nbt),
        font: match find_entry(entries, "font") {
            Some(NbtSummaryValue::String(font)) => font.try_to_string().ok(),
            _ => None,
        },
    }
}

fn style_click_event(entries: &[NbtSummaryEntry]) -> Option<ComponentClickEvent> {
    codec::decode_component_click_event(find_entry(entries, "click_event")?, "click event").ok()
}

fn style_hover_event(
    entries: &[NbtSummaryEntry],
    budget: &mut ProjectionBudget,
) -> Option<ComponentHoverEvent> {
    let NbtSummaryValue::Compound(hover) = find_entry(entries, "hover_event")? else {
        return None;
    };
    let action = match find_entry(hover, "action")? {
        NbtSummaryValue::String(action) => action,
        _ => return None,
    };
    match action {
        value if value == "show_text" => {
            let value = find_entry(hover, "value")?;
            let mut runs = RunCollector::default();
            let _ = append_component_runs_with_budget(
                value,
                &ComponentStyle::default(),
                &mut runs,
                budget,
            );
            Some(ComponentHoverEvent::ShowText {
                value: runs.runs,
                component: Some(value.clone()),
            })
        }
        value if value == "show_item" => Some(ComponentHoverEvent::ShowItem {
            id: match find_entry(hover, "id")? {
                NbtSummaryValue::String(id) => id.try_to_string().ok()?,
                _ => return None,
            },
            count: find_entry(hover, "count")
                .and_then(codec::component_i32)
                .unwrap_or(1),
            components: find_entry(hover, "components").cloned(),
        }),
        value if value == "show_entity" => {
            let name_component = find_entry(hover, "name").cloned();
            let name = name_component.as_ref().map(|name| {
                let mut runs = RunCollector::default();
                let _ = append_component_runs_with_budget(
                    name,
                    &ComponentStyle::default(),
                    &mut runs,
                    budget,
                );
                runs.runs
            });
            Some(ComponentHoverEvent::ShowEntity {
                id: match find_entry(hover, "id")? {
                    NbtSummaryValue::String(id) => id.try_to_string().ok()?,
                    _ => return None,
                },
                uuid: find_entry(hover, "uuid")?.clone(),
                name,
                name_component,
            })
        }
        _ => None,
    }
}

fn style_flag(entries: &[NbtSummaryEntry], key: &str) -> Option<bool> {
    codec::component_bool(find_entry(entries, key)?)
}

/// Vanilla `TextColor.parseColor` (`TextColor.java:76-91`): `#`-prefixed hex
/// literal in `0..=0xFFFFFF`, otherwise an exact named `ChatFormatting` colour
/// (`getName()` = lowercased enum name, underscores kept — the `NAMED_COLORS`
/// map).
///
/// The name comes back beside the value because `NAMED_COLORS` hands out the
/// name-carrying `TextColor` instances (`TextColor.java:18-23`), and only those
/// re-serialize as a name.
pub fn parse_text_color(color: &str) -> Option<(u32, Option<ChatFormatting>)> {
    if let Some(hex) = color.strip_prefix('#') {
        // `Integer.parseInt(hex, 16)` accepts an explicit sign before the
        // subsequent `0..=0xFFFFFF` range check (`TextColor.java:79-91`).
        let value = i32::from_str_radix(hex, 16).ok()?;
        return (0..=0xFF_FF_FF)
            .contains(&value)
            .then_some((value as u32, None));
    }
    let named = VANILLA_CHAT_FORMATTINGS
        .iter()
        .find(|entry| entry.serialized_name == color)?;
    Some((named.color?, Some(named.formatting)))
}

pub(super) fn find_entry<'a>(
    entries: &'a [NbtSummaryEntry],
    key: &str,
) -> Option<&'a NbtSummaryValue> {
    entries
        .iter()
        .rev()
        .find(|entry| entry.name == key)
        .map(|entry| &entry.value)
}

#[cfg(test)]
mod list_hover_tests {
    use super::*;

    #[test]
    fn nested_list_hover_event_projects_in_linear_time() {
        let mut node = NbtSummaryValue::String("leaf".into());
        for _ in 0..32 {
            let hover = NbtSummaryValue::Compound(vec![
                NbtSummaryEntry {
                    name: "action".into(),
                    value: NbtSummaryValue::String("show_text".into()),
                },
                NbtSummaryEntry {
                    name: "value".into(),
                    value: node,
                },
            ]);
            let elem = NbtSummaryValue::Compound(vec![
                NbtSummaryEntry {
                    name: "text".into(),
                    value: NbtSummaryValue::String("a".into()),
                },
                NbtSummaryEntry {
                    name: "hover_event".into(),
                    value: hover,
                },
            ]);
            node = NbtSummaryValue::List(vec![elem]);
        }
        let mut runs = RunCollector::default();
        append_component_runs(&node, &ComponentStyle::default(), &mut runs);
        assert_eq!(runs.runs.len(), 1);
        assert_eq!(runs.runs[0].text, "a");
        assert!(runs.runs[0].style.hover_event.is_some());
    }
}
