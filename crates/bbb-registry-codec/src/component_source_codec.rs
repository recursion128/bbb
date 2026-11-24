//! Source-aware text components projected to NBT with registry references.

use crate::persistent_component_nbt::source_codec::decode_item_stack_template_source;
use crate::registry_codec::dialog_source::decode_dialog_source;
use crate::registry_codec::{
    recursive_values::{
        required_field, CanonicalRegistryCodecValue, OwnedRegistryCodecReference,
        RegistryCodecDecodeContext, RegistryCodecRecord,
    },
    source_nbt::decode_nbtops_dynamic_source,
    RegistryCodecError, RegistryElementValueRef, RegistryResourceIdentifier,
};

use bbb_protocol::{
    component::{
        argb_from_channels, canonical_text_color_string, component_hover_uuid_nbt,
        is_valid_block_position_utf16, is_valid_entity_selector_utf16, is_valid_nbt_path_utf16,
        normalize_built_in_entity_type_id_with_registry, parse_untrusted_uri_units,
        resolve_component_record_type, BuiltInEntityIdError, ComponentHoverAction,
        ComponentRecordTypeError, ComponentStaticClickAction, COMPONENT_CONTENT_FIELDS,
        COMPONENT_DATA_SOURCE_FIELDS, COMPONENT_DEFAULT_ATLAS, COMPONENT_OBJECT_FIELDS,
        COMPONENT_STYLE_FIELDS,
    },
    nbt_summary::{
        json_number_to_javaops, uuid_from_words, JsonOpsNumber, JsonOpsNumberError, NbtSummaryValue,
    },
    normalize_resource_location,
};

pub(crate) mod profile;

const DIALOG_REGISTRY: &str = "minecraft:dialog";

impl RegistryCodecRecord {
    fn spread_object(
        &mut self,
        value: CanonicalRegistryCodecValue,
        field: &str,
        context: &RegistryCodecDecodeContext<'_>,
    ) -> Result<(), RegistryCodecError> {
        let (network_nbt, references) = value.into_parts();
        let NbtSummaryValue::Compound(mut fields) = network_nbt else {
            return Err(context.internal_error(field, "recursive owner did not encode an NBT map"));
        };
        self.fields.append(&mut fields);
        self.references.extend(references);
        Ok(())
    }
}

#[derive(Debug)]
struct DecodedComponent {
    content_type: &'static str,
    content: RegistryCodecRecord,
    siblings: Vec<NbtSummaryValue>,
    style: RegistryCodecRecord,
}

impl DecodedComponent {
    fn literal(text: &bbb_protocol::JavaString) -> Self {
        let mut content = RegistryCodecRecord::default();
        content.push_value("text", text.to_nbt());
        Self {
            content_type: "text",
            content,
            siblings: Vec::new(),
            style: RegistryCodecRecord::default(),
        }
    }

    fn into_canonical(mut self) -> CanonicalRegistryCodecValue {
        if self.content_type == "text" && self.siblings.is_empty() && self.style.fields.is_empty() {
            let text = self.content.fields.pop().unwrap().value;
            return CanonicalRegistryCodecValue::from_parts(text, self.content.references);
        }
        if !self.siblings.is_empty() {
            self.content
                .push_value("extra", NbtSummaryValue::List(self.siblings));
        }
        self.content.append(self.style);
        self.content.finish()
    }
}

pub(crate) fn decode_component_source(
    input: RegistryElementValueRef<'_>,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
) -> Result<CanonicalRegistryCodecValue, RegistryCodecError> {
    Ok(decode_component_node(input, field, context)?.into_canonical())
}

pub(crate) fn decode_style_source(
    input: RegistryElementValueRef<'_>,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
) -> Result<CanonicalRegistryCodecValue, RegistryCodecError> {
    context.charge_node(field)?;
    Ok(decode_style_fields(input, field, context)?.finish())
}

fn decode_component_node(
    input: RegistryElementValueRef<'_>,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
) -> Result<DecodedComponent, RegistryCodecError> {
    context.charge_node(field)?;
    if let Some(text) = input.java_string_ref() {
        return Ok(DecodedComponent::literal(text));
    }
    if let Some(items) = input.list_elements() {
        let len = items.len();
        context.check_collection(field, len)?;
        if len == 0 {
            return Err(context.codec_error(field, "component list is empty"));
        }
        let mut items = items.enumerate();
        let first_field = format!("{field}[0]");
        let mut first = context.with_nested(&first_field, |context| {
            decode_component_node(items.next().unwrap().1, &first_field, context)
        })?;
        for (index, item) in items {
            let child_field = format!("{field}[{index}]");
            let child = context.with_nested(&child_field, |context| {
                decode_component_node(item, &child_field, context)
            })?;
            let (child, references) = child.into_canonical().into_parts();
            first.siblings.push(child);
            first.content.references.extend(references);
        }
        return Ok(first);
    }
    if !input.is_compound() {
        return Err(context.codec_error(field, "expected component string, non-empty list, or map"));
    }
    context.check_collection(field, input.map_len().unwrap())?;

    let (content_type, mut content) = decode_content(input, field, context)?;
    let mut siblings = Vec::new();
    if let Some(extra) = input.field("extra") {
        let extra_field = format!("{field}.extra");
        let items = extra.list_elements().ok_or_else(|| {
            context.codec_error(&extra_field, "expected a non-empty component list")
        })?;
        let len = items.len();
        context.check_collection(&extra_field, len)?;
        if len == 0 {
            return Err(context.codec_error(&extra_field, "component list is empty"));
        }
        for (index, item) in items.enumerate() {
            let child_field = format!("{extra_field}[{index}]");
            let child = context.with_nested(&child_field, |context| {
                decode_component_node(item, &child_field, context)
            })?;
            let (child, references) = child.into_canonical().into_parts();
            siblings.push(child);
            content.references.extend(references);
        }
    }
    let style = decode_style_fields(input, field, context)?;
    Ok(DecodedComponent {
        content_type,
        content,
        siblings,
        style,
    })
}

fn decode_record_discriminator(
    input: RegistryElementValueRef<'_>,
    discriminator: &str,
    types: &[(&'static str, &'static str)],
    field: &str,
    unknown_label: &str,
    missing_reason: &str,
    context: &RegistryCodecDecodeContext<'_>,
) -> Result<(&'static str, &'static str), RegistryCodecError> {
    let explicit = input
        .field(discriminator)
        .map(|value| value.java_string_ref().ok_or(()));
    resolve_component_record_type(explicit, |key| input.field(key).is_some(), types).map_err(
        |error| match error {
            ComponentRecordTypeError::NonStringDiscriminator => {
                context.codec_error(&format!("{field}.{discriminator}"), "expected a string")
            }
            ComponentRecordTypeError::UnknownType(kind) => context.codec_error(
                &format!("{field}.{discriminator}"),
                format!("unknown {unknown_label} `{kind}`"),
            ),
            ComponentRecordTypeError::MissingField => context.codec_error(field, missing_reason),
        },
    )
}

fn decode_content(
    input: RegistryElementValueRef<'_>,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
) -> Result<(&'static str, RegistryCodecRecord), RegistryCodecError> {
    let (kind, _) = decode_record_discriminator(
        input,
        "type",
        &COMPONENT_CONTENT_FIELDS,
        field,
        "component content type",
        "no matching component content field",
        context,
    )?;
    Ok((kind, decode_content_type(kind, input, field, context)?))
}

fn decode_content_type(
    kind: &'static str,
    input: RegistryElementValueRef<'_>,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
) -> Result<RegistryCodecRecord, RegistryCodecError> {
    let mut fields = RegistryCodecRecord::default();
    match kind {
        "text" => {
            let value = required_field(input, "text", field, context)?
                .java_string_ref()
                .ok_or_else(|| {
                    context.codec_error(&format!("{field}.text"), "expected a string")
                })?;
            fields.push("text", decoded_java_string(value));
        }
        "translatable" => {
            fields.push(
                "translate",
                decoded_java_string(
                    required_field(input, "translate", field, context)?
                        .java_string_ref()
                        .ok_or_else(|| context.codec_error(field, "translate is not a string"))?,
                ),
            );
            if let Some(fallback) = input.field("fallback") {
                if let Some(value) = fallback.java_string_ref() {
                    fields.push("fallback", decoded_java_string(value));
                }
            }
            if let Some(with) = input.field("with") {
                let with_field = format!("{field}.with");
                let items = with.list_elements().ok_or_else(|| {
                    context.codec_error(&with_field, "expected a translation argument list")
                })?;
                let len = items.len();
                context.check_collection(&with_field, len)?;
                if len != 0 {
                    let mut arguments = Vec::with_capacity(len);
                    let mut argument_references = Vec::new();
                    for (index, argument) in items.enumerate() {
                        let argument_field = format!("{with_field}[{index}]");
                        let value = match decode_java_primitive(argument, &argument_field, context)?
                        {
                            Some(primitive) => {
                                context.charge_node(&argument_field)?;
                                primitive
                            }
                            None => context
                                .with_nested(&argument_field, |context| {
                                    decode_component_node(argument, &argument_field, context)
                                })?
                                .into_canonical(),
                        };
                        let (value, references) = value.into_parts();
                        arguments.push(value);
                        argument_references.extend(references);
                    }
                    fields.push(
                        "with",
                        CanonicalRegistryCodecValue::from_parts(
                            NbtSummaryValue::List(arguments),
                            argument_references,
                        ),
                    );
                }
            }
        }
        "keybind" => fields.push(
            "keybind",
            decoded_java_string(
                required_field(input, "keybind", field, context)?
                    .java_string_ref()
                    .ok_or_else(|| context.codec_error(field, "keybind is not a string"))?,
            ),
        ),
        "score" => {
            let score_field = format!("{field}.score");
            let score = required_field(input, "score", field, context)?;
            if !score.is_compound() {
                return Err(context.codec_error(&score_field, "expected a score map"));
            }
            context.check_collection(&score_field, score.map_len().unwrap())?;
            let mut score_fields = RegistryCodecRecord::default();
            score_fields.push(
                "name",
                decoded_java_string(
                    required_field(score, "name", &score_field, context)?
                        .java_string_ref()
                        .ok_or_else(|| {
                            context.codec_error(&format!("{score_field}.name"), "expected a string")
                        })?,
                ),
            );
            score_fields.push(
                "objective",
                decoded_java_string(
                    required_field(score, "objective", &score_field, context)?
                        .java_string_ref()
                        .ok_or_else(|| {
                            context.codec_error(
                                &format!("{score_field}.objective"),
                                "expected a string",
                            )
                        })?,
                ),
            );
            fields.push("score", score_fields.finish());
        }
        "selector" => {
            let selector = required_string_value(input, "selector", field, context)?;
            if !is_valid_entity_selector_utf16(selector.units()) {
                return Err(
                    context.codec_error(&format!("{field}.selector"), "invalid entity selector")
                );
            }
            fields.push("selector", decoded_java_string(selector));
            decode_optional_component(input, "separator", field, context, &mut fields)?;
        }
        "nbt" => decode_nbt_content(input, field, context, &mut fields)?,
        "object" => decode_object_content(input, field, context, &mut fields)?,
        _ => unreachable!("content type is frozen above"),
    }
    Ok(fields)
}

fn decode_nbt_content(
    input: RegistryElementValueRef<'_>,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
    fields: &mut RegistryCodecRecord,
) -> Result<(), RegistryCodecError> {
    let path = required_string_value(input, "nbt", field, context)?;
    if !is_valid_nbt_path_utf16(path.units()) {
        return Err(context.codec_error(&format!("{field}.nbt"), "invalid NBT path"));
    }
    fields.push("nbt", decoded_java_string(path));
    let interpreting = lenient_bool(input, "interpret").unwrap_or(false);
    let plain = lenient_bool(input, "plain").unwrap_or(false);
    if interpreting && plain {
        return Err(context.codec_error(field, "interpret and plain cannot both be enabled"));
    }
    if interpreting {
        fields.push("interpret", decoded_bool(true));
    }
    if plain {
        fields.push("plain", decoded_bool(true));
    }
    decode_optional_component(input, "separator", field, context, fields)?;

    let (_, source) = decode_record_discriminator(
        input,
        "source",
        &COMPONENT_DATA_SOURCE_FIELDS,
        field,
        "data source",
        "no component data source",
        context,
    )?;
    fields.push(source, decode_data_source(source, input, field, context)?);
    Ok(())
}

fn decode_data_source(
    source: &'static str,
    input: RegistryElementValueRef<'_>,
    field: &str,
    context: &RegistryCodecDecodeContext<'_>,
) -> Result<CanonicalRegistryCodecValue, RegistryCodecError> {
    match source {
        "entity" => {
            let selector = required_string_value(input, "entity", field, context)?;
            if !is_valid_entity_selector_utf16(selector.units()) {
                return Err(
                    context.codec_error(&format!("{field}.entity"), "invalid entity selector")
                );
            }
            Ok(decoded_java_string(selector))
        }
        "block" => {
            let block = required_string_value(input, "block", field, context)?;
            if !is_valid_block_position_utf16(block.units()) {
                return Err(
                    context.codec_error(&format!("{field}.block"), "invalid block coordinates")
                );
            }
            Ok(decoded_java_string(block))
        }
        "storage" => decoded_normalized_identifier(input, "storage", field, context),
        _ => unreachable!(),
    }
}

fn decode_object_content(
    input: RegistryElementValueRef<'_>,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
    fields: &mut RegistryCodecRecord,
) -> Result<(), RegistryCodecError> {
    let (object, _) = decode_record_discriminator(
        input,
        "object",
        &COMPONENT_OBJECT_FIELDS,
        field,
        "object type",
        "no matching component object field",
        context,
    )?;
    fields.append(decode_object_fields(object, input, field, context)?);
    decode_optional_component(input, "fallback", field, context, fields)
}

fn decode_object_fields(
    object: &'static str,
    input: RegistryElementValueRef<'_>,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
) -> Result<RegistryCodecRecord, RegistryCodecError> {
    match object {
        "atlas" => {
            let atlas = match input.field("atlas") {
                Some(_) => decoded_normalized_identifier(input, "atlas", field, context)?,
                None => decoded_string(COMPONENT_DEFAULT_ATLAS),
            };
            let mut fields = RegistryCodecRecord::default();
            if atlas.network_nbt_unchecked()
                != &NbtSummaryValue::String(COMPONENT_DEFAULT_ATLAS.into())
            {
                fields.push("atlas", atlas);
            }
            fields.push(
                "sprite",
                decoded_normalized_identifier(input, "sprite", field, context)?,
            );
            Ok(fields)
        }
        "player" => {
            let player_field = format!("{field}.player");
            let player = required_field(input, "player", field, context)?;
            let profile = context.with_nested(&player_field, |context| {
                profile::decode_resolvable_profile_source(player, &player_field, context)
            })?;
            let mut fields = RegistryCodecRecord::default();
            fields.push("player", profile);
            let hat = match input.field("hat") {
                Some(value) => value.as_bool().ok_or_else(|| {
                    context.codec_error(&format!("{field}.hat"), "expected a boolean")
                })?,
                None => true,
            };
            if !hat {
                fields.push("hat", decoded_bool(false));
            }
            Ok(fields)
        }
        _ => unreachable!(),
    }
}

fn decode_style_fields(
    input: RegistryElementValueRef<'_>,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
) -> Result<RegistryCodecRecord, RegistryCodecError> {
    if !input.is_compound() {
        return Err(context.codec_error(field, "style is not a map"));
    }
    context.check_collection(field, input.map_len().unwrap())?;
    let mut fields = RegistryCodecRecord::default();
    for name in COMPONENT_STYLE_FIELDS {
        if input.field(name).is_some() {
            let value = decode_style_member(input, name, field, context)?;
            fields.push(name, value);
        }
    }
    Ok(fields)
}

fn decode_style_member(
    input: RegistryElementValueRef<'_>,
    name: &'static str,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
) -> Result<CanonicalRegistryCodecValue, RegistryCodecError> {
    let value = input.field(name).expect("style member was present");
    let member_field = format!("{field}.{name}");
    match name {
        "color" => {
            let raw = value
                .java_string_ref()
                .ok_or_else(|| context.codec_error(&member_field, "expected a string"))?;
            let encoded = canonical_text_color_string(raw)
                .ok_or_else(|| context.codec_error(&member_field, "invalid text color"))?;
            Ok(decoded_string(&encoded))
        }
        "shadow_color" => Ok(decoded_i32(decode_argb(value, &member_field, context)?)),
        "bold" | "italic" | "underlined" | "strikethrough" | "obfuscated" => value
            .as_bool()
            .map(decoded_bool)
            .ok_or_else(|| context.codec_error(&member_field, "expected a boolean")),
        "click_event" => context.with_nested(&member_field, |context| {
            decode_click_event(value, &member_field, context)
        }),
        "hover_event" => context.with_nested(&member_field, |context| {
            decode_hover_event(value, &member_field, context)
        }),
        "insertion" => {
            Ok(decoded_java_string(value.java_string_ref().ok_or_else(
                || context.codec_error(&member_field, "expected a string"),
            )?))
        }
        "font" => decoded_normalized_identifier(input, name, field, context),
        _ => unreachable!("style member belongs to the frozen field table"),
    }
}

fn decode_click_event(
    input: RegistryElementValueRef<'_>,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
) -> Result<CanonicalRegistryCodecValue, RegistryCodecError> {
    context.charge_record(input, field, "click event")?;
    context.charge_node(&format!("{field}.action"))?;
    let action = required_string_value(input, "action", field, context)?;
    if action == "open_file" {
        return Err(
            context.codec_error(field, "click action open_file is not allowed from a server")
        );
    }
    let mut fields = RegistryCodecRecord::default();
    fields.push("action", decoded_java_string(action));
    decode_static_click_action_payload(input, action, field, context, &mut fields)?;
    Ok(fields.finish())
}

/// Decodes one of the server-allowed static click-event payloads through the
/// same semantic owner used by `Style.Serializer.MAP_CODEC`, but writes the
/// built-in dialog-action registry discriminator after the payload.
///
/// `Action.CODEC` is a registry `KeyDispatchCodec`; its selected MapCodec is
/// encoded first and the normalized `type` key is appended last. Dynamic
/// dialog actions are owned by the dialog module and never enter this helper.
pub(crate) fn decode_dialog_static_action_source(
    input: RegistryElementValueRef<'_>,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
) -> Result<CanonicalRegistryCodecValue, RegistryCodecError> {
    context.charge_record(input, field, "dialog action")?;
    context.charge_node(&format!("{field}.type"))?;
    let raw_type = required_string_value(input, "type", field, context)?;
    let normalized = normalize_resource_location(
        raw_type
            .try_to_string()
            .map_err(|_| context.codec_error(field, "invalid resource identifier"))?,
    )
    .map_err(|_| {
        context.codec_error(
            &format!("{field}.type"),
            format!("invalid dialog action type `{raw_type}`"),
        )
    })?;
    let (namespace, action) = normalized
        .split_once(':')
        .expect("normalized dialog action identifiers have a namespace");
    if namespace != "minecraft" {
        return Err(context.codec_error(
            &format!("{field}.type"),
            format!("unknown static dialog action `{raw_type}`"),
        ));
    }

    let mut fields = RegistryCodecRecord::default();
    decode_static_click_action_payload(input, action, field, context, &mut fields)?;
    fields.push("type", decoded_string(&normalized));
    Ok(fields.finish())
}

fn decode_static_click_action_payload(
    input: RegistryElementValueRef<'_>,
    action: impl PartialEq<&'static str> + std::fmt::Display,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
    fields: &mut RegistryCodecRecord,
) -> Result<(), RegistryCodecError> {
    match ComponentStaticClickAction::parse(&action) {
        Some(ComponentStaticClickAction::OpenUrl) => {
            context.charge_node(&format!("{field}.url"))?;
            let url = required_field(input, "url", field, context)?
                .java_string_ref()
                .ok_or_else(|| context.codec_error(&format!("{field}.url"), "expected a string"))?;
            parse_untrusted_uri_units(url, "url")
                .map_err(|reason| context.codec_error(&format!("{field}.url"), reason))?;
            fields.push("url", decoded_java_string(url));
        }
        Some(
            ComponentStaticClickAction::RunCommand | ComponentStaticClickAction::SuggestCommand,
        ) => {
            context.charge_node(&format!("{field}.command"))?;
            let command = required_field(input, "command", field, context)?
                .java_string_ref()
                .ok_or_else(|| {
                    context.codec_error(&format!("{field}.command"), "expected a string")
                })?;
            bbb_protocol::component::validate_chat_string_units(command, "command")
                .map_err(|reason| context.codec_error(&format!("{field}.command"), reason))?;
            fields.push("command", decoded_java_string(command));
        }
        Some(ComponentStaticClickAction::ShowDialog) => {
            let dialog_field = format!("{field}.dialog");
            let dialog = required_field(input, "dialog", field, context)?;
            if let Some(identifier) = dialog
                .java_string_ref()
                .and_then(RegistryResourceIdentifier::parse_java_string)
            {
                context.charge_node(&dialog_field)?;
                let mut decoded = decoded_string(identifier.as_str());
                decoded.push_reference(OwnedRegistryCodecReference::element(
                    DIALOG_REGISTRY,
                    identifier,
                ));
                fields.push("dialog", decoded);
            } else {
                let decoded = context.with_nested(&dialog_field, |context| {
                    decode_dialog_source(dialog, &dialog_field, context)
                })?;
                fields.push("dialog", decoded);
            }
        }
        Some(ComponentStaticClickAction::ChangePage) => {
            context.charge_node(&format!("{field}.page"))?;
            let page = required_field(input, "page", field, context)?
                .codec_i32()
                .ok_or_else(|| {
                    context.codec_error(&format!("{field}.page"), "expected an integer")
                })?;
            if page <= 0 {
                return Err(context.codec_error(&format!("{field}.page"), "page must be positive"));
            }
            fields.push("page", decoded_i32(page));
        }
        Some(ComponentStaticClickAction::CopyToClipboard) => {
            let value_field = format!("{field}.value");
            context.charge_node(&value_field)?;
            fields.push(
                "value",
                decoded_java_string(
                    required_field(input, "value", field, context)?
                        .java_string_ref()
                        .ok_or_else(|| context.codec_error(&value_field, "expected a string"))?,
                ),
            );
        }
        Some(ComponentStaticClickAction::Custom) => {
            context.charge_node(&format!("{field}.id"))?;
            fields.push(
                "id",
                decoded_normalized_identifier(input, "id", field, context)?,
            );
            if let Some(payload) = input.field("payload") {
                let payload_field = format!("{field}.payload");
                let payload = context.with_nested(&payload_field, |context| {
                    decode_nbtops_dynamic_source(payload, &payload_field, context)
                })?;
                fields.push("payload", payload);
            }
        }
        Some(ComponentStaticClickAction::OpenFile) | None => {
            return Err(context.codec_error(
                &format!("{field}.action"),
                format!("unknown click action `{action}`"),
            ));
        }
    }
    Ok(())
}

fn decode_hover_event(
    input: RegistryElementValueRef<'_>,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
) -> Result<CanonicalRegistryCodecValue, RegistryCodecError> {
    context.charge_record(input, field, "hover event")?;
    let action = required_string_value(input, "action", field, context)?;
    let mut fields = RegistryCodecRecord::default();
    fields.push("action", decoded_java_string(action));
    match ComponentHoverAction::parse(action) {
        Some(ComponentHoverAction::ShowText) => {
            let value_field = format!("{field}.value");
            let value = required_field(input, "value", field, context)?;
            let decoded = context.with_nested(&value_field, |context| {
                decode_component_node(value, &value_field, context)
            })?;
            fields.push("value", decoded.into_canonical());
        }
        Some(ComponentHoverAction::ShowItem) => {
            let decoded = context.with_nested(field, |context| {
                decode_item_stack_template_source(input, field, context)
            })?;
            fields.spread_object(decoded, field, context)?;
        }
        Some(ComponentHoverAction::ShowEntity) => {
            let raw_id = required_string_value(input, "id", field, context)?;
            let id =
                normalize_built_in_entity_type_id_with_registry(raw_id, context.content_registry())
                    .map_err(|error| match error {
                        BuiltInEntityIdError::InvalidUtf8 => {
                            context.codec_error(field, "invalid resource identifier")
                        }
                        BuiltInEntityIdError::InvalidIdentifier => {
                            context.codec_error(&format!("{field}.id"), "invalid entity identifier")
                        }
                        BuiltInEntityIdError::UnknownEntityType => context
                            .codec_error(&format!("{field}.id"), "unknown built-in entity type"),
                    })?;
            fields.push("id", decoded_string(&id));
            let uuid = decode_lenient_uuid(
                required_field(input, "uuid", field, context)?,
                &format!("{field}.uuid"),
                context,
            )?;
            fields.push("uuid", decoded_uuid(uuid));
            if let Some(name) = input.field("name") {
                let name_field = format!("{field}.name");
                let decoded = context.with_nested(&name_field, |context| {
                    decode_component_node(name, &name_field, context)
                })?;
                fields.push("name", decoded.into_canonical());
            }
        }
        None => {
            return Err(context.codec_error(
                &format!("{field}.action"),
                format!("unknown hover action `{action}`"),
            ))
        }
    }
    Ok(fields.finish())
}

fn decode_optional_component(
    input: RegistryElementValueRef<'_>,
    name: &'static str,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
    fields: &mut RegistryCodecRecord,
) -> Result<(), RegistryCodecError> {
    let Some(value) = input.field(name) else {
        return Ok(());
    };
    let child_field = format!("{field}.{name}");
    let decoded = context.with_nested(&child_field, |context| {
        decode_component_node(value, &child_field, context)
    })?;
    fields.push(name, decoded.into_canonical());
    Ok(())
}

fn decode_argb(
    value: RegistryElementValueRef<'_>,
    field: &str,
    context: &mut RegistryCodecDecodeContext<'_>,
) -> Result<i32, RegistryCodecError> {
    if let Some(value) = value.codec_i32() {
        return Ok(value);
    }
    let values = value
        .list_elements()
        .ok_or_else(|| context.codec_error(field, "expected an integer or vector4f"))?;
    let len = values.len();
    context.check_collection(field, len)?;
    if len != 4 {
        return Err(context.codec_error(field, "vector4f must have four numbers"));
    }
    let mut channels = [0.0; 4];
    for (channel, value) in channels.iter_mut().zip(values) {
        *channel = value
            .codec_f32()
            .ok_or_else(|| context.codec_error(field, "vector4f contains a non-number"))?;
    }
    Ok(argb_from_channels(channels))
}

fn decode_lenient_uuid(
    input: RegistryElementValueRef<'_>,
    field: &str,
    context: &RegistryCodecDecodeContext<'_>,
) -> Result<uuid::Uuid, RegistryCodecError> {
    if let Some(value) = input.java_string_ref() {
        return value
            .with_utf8(bbb_protocol::component::parse_uuid_words)
            .flatten()
            .map(uuid_from_words)
            .ok_or_else(|| context.codec_error(field, "invalid UUID string"));
    }
    let words = decode_uuid_int_array(input, field, context)?;
    Ok(uuid_from_words(words))
}

fn decode_uuid_int_array(
    input: RegistryElementValueRef<'_>,
    field: &str,
    context: &RegistryCodecDecodeContext<'_>,
) -> Result<[i32; 4], RegistryCodecError> {
    let values = input
        .list_elements()
        .ok_or_else(|| context.codec_error(field, "expected four UUID integers"))?;
    let len = values.len();
    context.check_collection(field, len)?;
    if len != 4 {
        return Err(context.codec_error(field, "UUID must contain four integers"));
    }
    let mut words = [0; 4];
    for (word, value) in words.iter_mut().zip(values) {
        *word = value
            .codec_i32()
            .ok_or_else(|| context.codec_error(field, "UUID contains a non-number"))?;
    }
    Ok(words)
}

fn decode_java_primitive(
    input: RegistryElementValueRef<'_>,
    field: &str,
    context: &RegistryCodecDecodeContext<'_>,
) -> Result<Option<CanonicalRegistryCodecValue>, RegistryCodecError> {
    if let Some(value) = input.java_string_ref() {
        return Ok(Some(decoded_java_string(value)));
    }
    let number = match input {
        RegistryElementValueRef::Nbt(NbtSummaryValue::Byte(value)) => JsonOpsNumber::Byte(*value),
        RegistryElementValueRef::Nbt(NbtSummaryValue::Short(value)) => JsonOpsNumber::Short(*value),
        RegistryElementValueRef::Nbt(NbtSummaryValue::Int(value)) => JsonOpsNumber::Int(*value),
        RegistryElementValueRef::Nbt(NbtSummaryValue::Long(value)) => JsonOpsNumber::Long(*value),
        RegistryElementValueRef::Nbt(NbtSummaryValue::Float(value)) => {
            JsonOpsNumber::Float(f32::from_bits(*value))
        }
        RegistryElementValueRef::Nbt(NbtSummaryValue::Double(value)) => {
            JsonOpsNumber::Double(f64::from_bits(*value))
        }
        RegistryElementValueRef::NbtByte(value) => JsonOpsNumber::Byte(value),
        RegistryElementValueRef::NbtInt(value) => JsonOpsNumber::Int(value),
        RegistryElementValueRef::NbtLong(value) => JsonOpsNumber::Long(value),
        RegistryElementValueRef::Json(crate::registry_codec::RegistryJsonValue::Bool(value)) => {
            return Ok(Some(decoded_bool(*value)));
        }
        RegistryElementValueRef::Json(crate::registry_codec::RegistryJsonValue::Number(value)) => {
            match json_number_to_javaops(value) {
                Ok(value) => value,
                Err(JsonOpsNumberError::Invalid(_)) => return Ok(None),
                Err(JsonOpsNumberError::GsonLimit(reason)) => {
                    return Err(context.coverage_error(field, reason));
                }
            }
        }
        _ => return Ok(None),
    };
    Ok(Some(CanonicalRegistryCodecValue::new(number.into_nbt())))
}

fn decoded_string(value: &str) -> CanonicalRegistryCodecValue {
    CanonicalRegistryCodecValue::new(NbtSummaryValue::String(value.into()))
}

fn decoded_java_string(value: &bbb_protocol::JavaString) -> CanonicalRegistryCodecValue {
    CanonicalRegistryCodecValue::new(value.to_nbt())
}

fn decoded_bool(value: bool) -> CanonicalRegistryCodecValue {
    CanonicalRegistryCodecValue::new(NbtSummaryValue::Byte(i8::from(value)))
}

fn decoded_i32(value: i32) -> CanonicalRegistryCodecValue {
    CanonicalRegistryCodecValue::new(NbtSummaryValue::Int(value))
}

fn decoded_uuid(value: uuid::Uuid) -> CanonicalRegistryCodecValue {
    CanonicalRegistryCodecValue::new(component_hover_uuid_nbt(value))
}

fn lenient_bool(input: RegistryElementValueRef<'_>, name: &str) -> Option<bool> {
    input.field(name)?.as_bool()
}

fn required_string_value<'a>(
    input: RegistryElementValueRef<'a>,
    name: &str,
    field: &str,
    context: &RegistryCodecDecodeContext<'_>,
) -> Result<&'a bbb_protocol::JavaString, RegistryCodecError> {
    let value = required_field(input, name, field, context)?;
    value
        .java_string_ref()
        .ok_or_else(|| context.codec_error(&format!("{field}.{name}"), "expected a string"))
}

fn decoded_normalized_identifier(
    input: RegistryElementValueRef<'_>,
    name: &str,
    field: &str,
    context: &RegistryCodecDecodeContext<'_>,
) -> Result<CanonicalRegistryCodecValue, RegistryCodecError> {
    let raw = required_string_value(input, name, field, context)?;
    let value = normalize_resource_location(
        raw.try_to_string()
            .map_err(|_| context.codec_error(field, "invalid resource identifier"))?,
    )
    .map_err(|_| {
        context.codec_error(
            &format!("{field}.{name}"),
            format!("invalid resource identifier `{raw}`"),
        )
    })?;
    Ok(decoded_string(&value))
}

#[cfg(test)]
mod tests;
