//! Exact source-aware building blocks for `Dialog.DIRECT_CODEC`.
//!
//! Dialog is the registry-level owner that closes the recursive
//! Component/Dialog/`DataComponentPatch`/`ItemStackTemplate` cycle.  This
//! module keeps one decode budget and one ordered reference stream across that
//! cycle; named holders remain placeholders until the complete candidate
//! generation freezes.

mod action;
mod body_input;

use crate::component_source_codec::decode_component_source;
pub(super) use crate::registry_codec::recursive_values::{decode_compact_list, decode_list};
use crate::registry_codec::recursive_values::{
    required_field, CanonicalRegistryCodecValue, OwnedRegistryCodecReference,
    RegistryCodecDecodeContext, RegistryCodecRecord,
};
use bbb_protocol::{normalize_resource_location, NbtSummaryEntry, NbtSummaryValue};

use super::RegistryResourceIdentifier;
use super::{RegistryCodecError, RegistryElementValueRef};

pub(super) const DIALOG_REGISTRY: &str = "minecraft:dialog";
pub(super) const DIALOG_CODEC: &str = "Dialog.DIRECT_CODEC";

impl RegistryCodecRecord {
    pub(super) fn finish_dispatch(
        mut self,
        type_id: &str,
        field: &str,
        context: &mut RegistryCodecDecodeContext<'_>,
    ) -> Result<CanonicalRegistryCodecValue, RegistryCodecError> {
        self.push(
            "type",
            string_value(type_id, &format!("{field}.type"), context)?,
        );
        Ok(self.finish())
    }
}

pub(super) fn string_value(
    value: &str,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
) -> Result<CanonicalRegistryCodecValue, RegistryCodecError> {
    context.charge_node(field)?;
    Ok(CanonicalRegistryCodecValue::new(NbtSummaryValue::String(
        value.into(),
    )))
}

pub(super) fn java_string_value(value: &bbb_protocol::JavaString) -> CanonicalRegistryCodecValue {
    CanonicalRegistryCodecValue::new(value.to_nbt())
}

pub(super) fn bool_value(
    value: bool,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
) -> Result<CanonicalRegistryCodecValue, RegistryCodecError> {
    context.charge_node(field)?;
    Ok(CanonicalRegistryCodecValue::new(NbtSummaryValue::Byte(
        i8::from(value),
    )))
}

pub(super) fn i32_value(
    value: i32,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
) -> Result<CanonicalRegistryCodecValue, RegistryCodecError> {
    context.charge_node(field)?;
    Ok(CanonicalRegistryCodecValue::new(NbtSummaryValue::Int(
        value,
    )))
}

pub(super) fn builtin_dispatch_id(
    raw: &str,
    allowed_paths: &[&str],
    field: &str,
    context: &RegistryCodecDecodeContext<'_>,
) -> Result<(String, String), RegistryCodecError> {
    let normalized = normalize_resource_location(raw)
        .map_err(|_| context.codec_error(field, format!("invalid dispatch identifier `{raw}`")))?;
    let (namespace, path) = normalized
        .split_once(':')
        .expect("normalized identifiers always have a namespace");
    if namespace != "minecraft" || !allowed_paths.contains(&path) {
        return Err(context.codec_error(field, format!("unknown dispatch type `{raw}`")));
    }
    let path = path.to_string();
    Ok((normalized, path))
}

pub(crate) fn decode_dialog_source(
    input: RegistryElementValueRef<'_>,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
) -> Result<CanonicalRegistryCodecValue, RegistryCodecError> {
    context.charge_record(input, field, "dialog")?;

    let type_field = format!("{field}.type");
    let raw_type = required_field(input, "type", field, context)?
        .utf8_string()
        .ok_or_else(|| context.codec_error(&type_field, "expected a dialog type string"))?;
    let (type_id, dialog_type) = builtin_dispatch_id(
        &raw_type,
        &[
            "notice",
            "server_links",
            "dialog_list",
            "multi_action",
            "confirmation",
        ],
        &type_field,
        context,
    )?;

    let mut object = RegistryCodecRecord::default();
    let common = decode_common_dialog(input, field, context, &mut object)?;
    match dialog_type.as_str() {
        "notice" => {
            if let Some(action) = input.field("action") {
                let action_field = format!("{field}.action");
                let decoded = context.with_nested(&action_field, |context| {
                    decode_action_button(action, &action_field, context)
                })?;
                if !is_default_notice_action(&decoded) {
                    object.push("action", decoded);
                }
            }
        }
        "confirmation" => {
            for name in ["yes", "no"] {
                let child_field = format!("{field}.{name}");
                let child = required_field(input, name, field, context)?;
                let decoded = context.with_nested(&child_field, |context| {
                    decode_action_button(child, &child_field, context)
                })?;
                object.push(name, decoded);
            }
        }
        "multi_action" => {
            let actions_field = format!("{field}.actions");
            let actions = required_field(input, "actions", field, context)?;
            let len = actions.list_len().ok_or_else(|| {
                context.codec_error(&actions_field, "expected a non-empty action list")
            })?;
            if len == 0 {
                return Err(context.codec_error(&actions_field, "action list is empty"));
            }
            let decoded = decode_list(actions, &actions_field, context, decode_action_button)?;
            object.push("actions", decoded);
            decode_optional_action_button(input, "exit_action", field, context, &mut object)?;
            decode_positive_default_i32(input, "columns", 2, field, context, &mut object)?;
        }
        "server_links" => {
            decode_optional_action_button(input, "exit_action", field, context, &mut object)?;
            decode_positive_default_i32(input, "columns", 2, field, context, &mut object)?;
            decode_range_default_i32(
                input,
                "button_width",
                150,
                1,
                1024,
                field,
                context,
                &mut object,
            )?;
        }
        "dialog_list" => {
            let dialogs_field = format!("{field}.dialogs");
            let dialogs = required_field(input, "dialogs", field, context)?;
            let decoded = context.with_nested(&dialogs_field, |context| {
                decode_dialog_holder_set(dialogs, &dialogs_field, context)
            })?;
            object.push("dialogs", decoded);
            decode_optional_action_button(input, "exit_action", field, context, &mut object)?;
            decode_positive_default_i32(input, "columns", 2, field, context, &mut object)?;
            decode_range_default_i32(
                input,
                "button_width",
                150,
                1,
                1024,
                field,
                context,
                &mut object,
            )?;
        }
        _ => unreachable!("builtin dialog type was validated above"),
    }

    if common.pause && common.after_action == DialogAfterAction::None {
        return Err(context.codec_error(
            field,
            "dialogs that pause must use an after_action that unpauses",
        ));
    }
    object.finish_dispatch(&type_id, field, context)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DialogAfterAction {
    Close,
    None,
    WaitForResponse,
}

#[derive(Debug, Clone)]
struct CommonDialog {
    pause: bool,
    after_action: DialogAfterAction,
}

fn decode_common_dialog(
    input: RegistryElementValueRef<'_>,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
    output: &mut RegistryCodecRecord,
) -> Result<CommonDialog, RegistryCodecError> {
    let title_field = format!("{field}.title");
    let title = required_field(input, "title", field, context)?;
    let title = context.with_nested(&title_field, |context| {
        decode_component_source(title, &title_field, context)
    })?;
    output.push("title", title);

    if let Some(external_title) = input.field("external_title") {
        let child_field = format!("{field}.external_title");
        let decoded = context.with_nested(&child_field, |context| {
            decode_component_source(external_title, &child_field, context)
        })?;
        output.push("external_title", decoded);
    }

    decode_default_bool(input, "can_close_with_escape", true, field, context, output)?;
    let pause = decode_default_bool(input, "pause", true, field, context, output)?;

    let after_action = match input.field("after_action") {
        None => DialogAfterAction::Close,
        Some(value) => {
            let child_field = format!("{field}.after_action");
            let value = value.utf8_string().ok_or_else(|| {
                context.codec_error(&child_field, "expected a dialog after_action string")
            })?;
            let after_action = match value.as_str() {
                "close" => DialogAfterAction::Close,
                "none" => DialogAfterAction::None,
                "wait_for_response" => DialogAfterAction::WaitForResponse,
                _ => {
                    return Err(context.codec_error(
                        &child_field,
                        format!("unknown dialog after_action `{value}`"),
                    ));
                }
            };
            let decoded = string_value(&value, &child_field, context)?;
            if value != "close" {
                output.push("after_action", decoded);
            }
            after_action
        }
    };

    if let Some(body) = input.field("body") {
        let child_field = format!("{field}.body");
        let decoded = context.with_nested(&child_field, |context| {
            body_input::decode_body_compact_list_source(body, &child_field, context)
        })?;
        if decoded.network_nbt_unchecked() != &NbtSummaryValue::List(Vec::new()) {
            output.push("body", decoded);
        }
    }

    if let Some(inputs) = input.field("inputs") {
        let child_field = format!("{field}.inputs");
        let decoded = context.with_nested(&child_field, |context| {
            body_input::decode_input_list_source(inputs, &child_field, context)
        })?;
        if decoded.network_nbt_unchecked() != &NbtSummaryValue::List(Vec::new()) {
            output.push("inputs", decoded);
        }
    }

    Ok(CommonDialog {
        pause,
        after_action,
    })
}

fn decode_action_button(
    input: RegistryElementValueRef<'_>,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
) -> Result<CanonicalRegistryCodecValue, RegistryCodecError> {
    context.charge_record(input, field, "action button")?;

    let mut output = RegistryCodecRecord::default();
    let label_field = format!("{field}.label");
    let label = required_field(input, "label", field, context)?;
    let label = context.with_nested(&label_field, |context| {
        decode_component_source(label, &label_field, context)
    })?;
    output.push("label", label);
    if let Some(tooltip) = input.field("tooltip") {
        let child_field = format!("{field}.tooltip");
        let decoded = context.with_nested(&child_field, |context| {
            decode_component_source(tooltip, &child_field, context)
        })?;
        output.push("tooltip", decoded);
    }
    decode_range_default_i32(input, "width", 150, 1, 1024, field, context, &mut output)?;
    if let Some(action) = input.field("action") {
        let child_field = format!("{field}.action");
        let decoded = context.with_nested(&child_field, |context| {
            action::decode_action_source(action, &child_field, context)
        })?;
        output.push("action", decoded);
    }
    Ok(output.finish())
}

fn decode_optional_action_button(
    input: RegistryElementValueRef<'_>,
    name: &'static str,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
    output: &mut RegistryCodecRecord,
) -> Result<(), RegistryCodecError> {
    let Some(value) = input.field(name) else {
        return Ok(());
    };
    let child_field = format!("{field}.{name}");
    let decoded = context.with_nested(&child_field, |context| {
        decode_action_button(value, &child_field, context)
    })?;
    output.push(name, decoded);
    Ok(())
}

fn decode_default_bool(
    input: RegistryElementValueRef<'_>,
    name: &'static str,
    default: bool,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
    output: &mut RegistryCodecRecord,
) -> Result<bool, RegistryCodecError> {
    let Some(value) = input.field(name) else {
        return Ok(default);
    };
    let child_field = format!("{field}.{name}");
    let value = value
        .as_bool()
        .ok_or_else(|| context.codec_error(&child_field, "expected a boolean"))?;
    let decoded = bool_value(value, &child_field, context)?;
    if value != default {
        output.push(name, decoded);
    }
    Ok(value)
}

fn decode_positive_default_i32(
    input: RegistryElementValueRef<'_>,
    name: &'static str,
    default: i32,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
    output: &mut RegistryCodecRecord,
) -> Result<i32, RegistryCodecError> {
    decode_range_default_i32(input, name, default, 1, i32::MAX, field, context, output)
}

#[allow(clippy::too_many_arguments)]
fn decode_range_default_i32(
    input: RegistryElementValueRef<'_>,
    name: &'static str,
    default: i32,
    minimum: i32,
    maximum: i32,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
    output: &mut RegistryCodecRecord,
) -> Result<i32, RegistryCodecError> {
    let Some(value) = input.field(name) else {
        return Ok(default);
    };
    let child_field = format!("{field}.{name}");
    let value = value
        .codec_i32()
        .ok_or_else(|| context.codec_error(&child_field, "expected an integer"))?;
    if !(minimum..=maximum).contains(&value) {
        return Err(context.codec_error(
            &child_field,
            format!("value must be in {minimum}..={maximum}"),
        ));
    }
    let decoded = i32_value(value, &child_field, context)?;
    if value != default {
        output.push(name, decoded);
    }
    Ok(value)
}

fn is_default_notice_action(value: &CanonicalRegistryCodecValue) -> bool {
    // CommonComponents: GUI_OK is an unstyled translatable component.
    value.references().next().is_none()
        && value.network_nbt_unchecked()
            == &NbtSummaryValue::Compound(vec![NbtSummaryEntry {
                name: "label".into(),
                value: NbtSummaryValue::Compound(vec![NbtSummaryEntry {
                    name: "translate".into(),
                    value: NbtSummaryValue::String("gui.ok".into()),
                }]),
            }])
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DialogHolderKind {
    Named,
    Inline,
}

fn decode_dialog_holder_set(
    input: RegistryElementValueRef<'_>,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
) -> Result<CanonicalRegistryCodecValue, RegistryCodecError> {
    if let Some(raw) = input
        .java_string_ref()
        .and_then(|raw| raw.units().strip_prefix(&[b'#' as u16]))
        .and_then(|units| String::from_utf16(units).ok())
    {
        context.charge_node(field)?;
        let identifier = RegistryResourceIdentifier::parse_owned(raw)
            .ok_or_else(|| context.codec_error(field, "invalid tag identifier"))?;
        let encoded = format!("#{}", identifier.as_str());
        let mut canonical =
            CanonicalRegistryCodecValue::new(NbtSummaryValue::String(encoded.into()));
        canonical.push_reference(OwnedRegistryCodecReference::tag(
            DIALOG_REGISTRY,
            identifier,
        ));
        return Ok(canonical);
    }

    if input.list_len().is_some() {
        let mut kind = None;
        return decode_compact_list(input, field, context, |item, child_field, context| {
            let (decoded, item_kind) = decode_dialog_holder(item, child_field, context)?;
            if kind.is_some_and(|expected| expected != item_kind) {
                return Err(context.codec_error(
                    child_field,
                    "dialog holder list mixes named and inline values",
                ));
            }
            kind = Some(item_kind);
            Ok(decoded)
        });
    }

    decode_dialog_holder(input, field, context).map(|(decoded, _)| decoded)
}

fn decode_dialog_holder(
    input: RegistryElementValueRef<'_>,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
) -> Result<(CanonicalRegistryCodecValue, DialogHolderKind), RegistryCodecError> {
    if let Some(identifier) = input
        .java_string_ref()
        .and_then(RegistryResourceIdentifier::parse_java_string)
    {
        context.charge_node(field)?;
        let mut canonical =
            CanonicalRegistryCodecValue::new(NbtSummaryValue::String(identifier.as_str().into()));
        canonical.push_reference(OwnedRegistryCodecReference::element(
            DIALOG_REGISTRY,
            identifier,
        ));
        return Ok((canonical, DialogHolderKind::Named));
    }
    let decoded = decode_dialog_source(input, field, context)?;
    Ok((decoded, DialogHolderKind::Inline))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::registry_codec::{
        decode_direct_registry_json, RegistryElementSource, RegistryReferenceKind,
    };

    fn context() -> RegistryCodecDecodeContext<'static> {
        RegistryCodecDecodeContext::new(
            DIALOG_REGISTRY,
            DIALOG_CODEC,
            RegistryElementSource::ServerDatapackJson,
        )
    }

    #[test]
    fn notice_defaults_encode_only_title_then_dispatch_type() {
        let data = decode_direct_registry_json(br#"{"type":"notice","title":"x"}"#).unwrap();
        let mut context = context();
        let decoded = decode_dialog_source(data.root(), "$", &mut context).unwrap();
        assert_eq!(
            decoded.network_nbt_unchecked(),
            &NbtSummaryValue::Compound(vec![
                NbtSummaryEntry {
                    name: "title".into(),
                    value: NbtSummaryValue::String("x".to_string().into()),
                },
                NbtSummaryEntry {
                    name: "type".into(),
                    value: NbtSummaryValue::String("minecraft:notice".to_string().into()),
                },
            ])
        );
    }

    #[test]
    fn all_five_dialog_dispatch_types_reach_their_complete_map_codecs() {
        let cases: &[(&[u8], &[&str], &str)] = &[
            (
                br#"{"type":"notice","title":"n"}"#,
                &["title", "type"],
                "minecraft:notice",
            ),
            (
                br#"{"type":"confirmation","title":"c","yes":{"label":"yes"},"no":{"label":"no"}}"#,
                &["title", "yes", "no", "type"],
                "minecraft:confirmation",
            ),
            (
                br#"{"type":"multi_action","title":"m","actions":[{"label":"go"}]}"#,
                &["title", "actions", "type"],
                "minecraft:multi_action",
            ),
            (
                br#"{"type":"server_links","title":"s"}"#,
                &["title", "type"],
                "minecraft:server_links",
            ),
            (
                br#"{"type":"dialog_list","title":"d","dialogs":[]}"#,
                &["title", "dialogs", "type"],
                "minecraft:dialog_list",
            ),
        ];
        for (json, field_names, expected_type) in cases {
            let data = decode_direct_registry_json(json).unwrap();
            let decoded = decode_dialog_source(data.root(), "$", &mut context())
                .unwrap_or_else(|error| panic!("{expected_type}: {error}"));
            let NbtSummaryValue::Compound(entries) = decoded.network_nbt_unchecked() else {
                panic!("{expected_type} did not encode a compound")
            };
            assert_eq!(
                entries.iter().map(|entry| &entry.name).collect::<Vec<_>>(),
                *field_names,
                "{expected_type}",
            );
            assert_eq!(
                entries.last().map(|entry| &entry.value),
                Some(&NbtSummaryValue::String(
                    (*expected_type).to_string().into()
                )),
            );
        }
    }

    #[test]
    fn common_dialog_validation_and_default_elision_follow_record_codec() {
        for invalid in [
            br#"{"type":"notice","title":"x","after_action":"none"}"#.as_slice(),
            br#"{"type":"notice","title":"x","pause":true,"after_action":"none"}"#,
        ] {
            let data = decode_direct_registry_json(invalid).unwrap();
            let error = decode_dialog_source(data.root(), "$", &mut context()).unwrap_err();
            assert!(error.reason.contains("must use an after_action"));
        }

        let data = decode_direct_registry_json(
            br#"{"type":"notice","title":"x","pause":false,"after_action":"none","can_close_with_escape":true}"#,
        )
        .unwrap();
        let decoded = decode_dialog_source(data.root(), "$", &mut context()).unwrap();
        assert_eq!(
            decoded.network_nbt_unchecked(),
            &NbtSummaryValue::Compound(vec![
                NbtSummaryEntry {
                    name: "title".into(),
                    value: NbtSummaryValue::String("x".to_string().into()),
                },
                NbtSummaryEntry {
                    name: "pause".into(),
                    value: NbtSummaryValue::Byte(0),
                },
                NbtSummaryEntry {
                    name: "after_action".into(),
                    value: NbtSummaryValue::String("none".to_string().into()),
                },
                NbtSummaryEntry {
                    name: "type".into(),
                    value: NbtSummaryValue::String("minecraft:notice".to_string().into()),
                },
            ])
        );
    }

    #[test]
    fn dialog_list_retains_named_holders_without_recursing() {
        let data = decode_direct_registry_json(
            br#"{"type":"dialog_list","title":"root","dialogs":["example:self","example:other"]}"#,
        )
        .unwrap();
        let decoded = decode_dialog_source(data.root(), "$", &mut context()).unwrap();
        let references = decoded.references().collect::<Vec<_>>();
        assert_eq!(references.len(), 2);
        assert!(references.iter().all(|reference| {
            reference.target_registry == DIALOG_REGISTRY
                && reference.kind == RegistryReferenceKind::Element
        }));
        assert_eq!(references[0].resource_id.as_str(), "example:self");
        assert_eq!(references[1].resource_id.as_str(), "example:other");
        assert!(matches!(
            decoded.network_nbt_unchecked(),
            NbtSummaryValue::Compound(entries)
                if matches!(&entries[1].value, NbtSummaryValue::List(values) if values.len() == 2)
        ));
    }
}
