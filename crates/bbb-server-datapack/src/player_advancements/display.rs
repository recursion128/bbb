//! `DisplayInfo` (`net/minecraft/advancements/DisplayInfo.java`): the
//! client-facing half of an advancement definition, decoded the way
//! `DisplayInfo.CODEC` reads it.

use bbb_protocol::{
    content_registry::ContentRegistry, normalize_resource_location, NbtSummaryValue,
    RichTextComponent,
};
use bbb_registry_codec::{
    decode_item_stack_template_parameter_source_with_content_registry,
    decode_loot_text_parameter_source_with_content_registry, PersistentNbtDecodeLimits,
    RegistryElementSource, RegistryElementValueRef, RegistryJsonValue,
};

/// `AdvancementType` (`AdvancementType.java:11-13`).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum AdvancementFrame {
    #[default]
    Task,
    Challenge,
    Goal,
}

impl AdvancementFrame {
    /// `getSerializedName` (`:35-37`).
    pub fn serialized_name(self) -> &'static str {
        match self {
            Self::Task => "task",
            Self::Challenge => "challenge",
            Self::Goal => "goal",
        }
    }

    fn parse(raw: &str) -> Option<Self> {
        [Self::Task, Self::Challenge, Self::Goal]
            .into_iter()
            .find(|frame| frame.serialized_name() == raw)
    }
}

/// The eight fields `DisplayInfo.CODEC` reads (`DisplayInfo.java:14-26`) plus
/// the location `TreeNodePosition` lays out (`:38-39`), which stays at the
/// origin until `setLocation` positions it.
#[derive(Debug, Clone, PartialEq)]
pub struct AdvancementDisplay {
    pub icon: NbtSummaryValue,
    pub title: RichTextComponent,
    pub description: RichTextComponent,
    pub background: Option<String>,
    pub frame: AdvancementFrame,
    pub show_toast: bool,
    pub announce_to_chat: bool,
    pub hidden: bool,
    pub x: f32,
    pub y: f32,
}

/// `Advancement.CODEC`'s `display` arm (`Advancement.java:42`), read through a
/// non-lenient `optionalFieldOf`: a malformed display fails the advancement
/// instead of leaving it undisplayed.
pub(super) fn decode_with_content_registry(
    display: &RegistryJsonValue,
    content_registry: &ContentRegistry,
) -> Result<AdvancementDisplay, String> {
    if display.as_object().is_none() {
        return Err("`display` must be an object".to_string());
    }
    let icon = decode_icon(field(display, "icon")?, content_registry)?;
    let title = component(field(display, "title")?, "title", content_registry)?;
    let description = component(
        field(display, "description")?,
        "description",
        content_registry,
    )?;
    let background = display
        .get("background")
        .map(|value| identifier(value, "background"))
        .transpose()?;
    let frame = match display.get("frame") {
        Some(value) => RegistryElementValueRef::Json(value)
            .java_string_ref()
            .and_then(|frame| frame.with_utf8(AdvancementFrame::parse).flatten())
            .ok_or_else(|| format!("`frame` {value:?} names no advancement type"))?,
        None => AdvancementFrame::default(),
    };
    Ok(AdvancementDisplay {
        icon,
        title,
        description,
        background,
        frame,
        show_toast: flag(display, "show_toast", true)?,
        announce_to_chat: flag(display, "announce_to_chat", true)?,
        hidden: flag(display, "hidden", false)?,
        x: 0.0,
        y: 0.0,
    })
}

fn field<'a>(display: &'a RegistryJsonValue, name: &str) -> Result<&'a RegistryJsonValue, String> {
    display
        .get(name)
        .ok_or_else(|| format!("`display` lacks a `{name}`"))
}

fn decode_icon(
    value: &RegistryJsonValue,
    content_registry: &ContentRegistry,
) -> Result<NbtSummaryValue, String> {
    decode_item_stack_template_parameter_source_with_content_registry(
        RegistryElementValueRef::Json(value),
        RegistryElementSource::ServerDatapackJson,
        PersistentNbtDecodeLimits {
            max_depth: 127,
            max_nodes: usize::MAX,
            max_collection: usize::MAX,
        },
        PersistentNbtDecodeLimits {
            max_depth: 64,
            max_nodes: usize::MAX,
            max_collection: 4096,
        },
        content_registry,
    )
    .map_err(|error| format!("invalid advancement icon: {error}"))
}

fn component(
    value: &RegistryJsonValue,
    name: &str,
    content_registry: &ContentRegistry,
) -> Result<RichTextComponent, String> {
    let nbt = decode_loot_text_parameter_source_with_content_registry(
        RegistryElementValueRef::Json(value),
        RegistryElementSource::ServerDatapackJson,
        PersistentNbtDecodeLimits {
            max_depth: 64,
            max_nodes: usize::MAX,
            max_collection: 4096,
        },
        content_registry,
    )
    .map_err(|error| format!("`{name}` is not a text component: {error}"))?;
    RichTextComponent::try_from_component_nbt_with_registry(nbt, content_registry)
        .map_err(|error| format!("`{name}` is not a text component: {error}"))
}

fn identifier(value: &RegistryJsonValue, name: &str) -> Result<String, String> {
    let raw = value
        .utf8_string()
        .ok_or_else(|| format!("`{name}` must be a string"))?;
    normalize_resource_location(raw)
        .map_err(|_| format!("`{name}` {value:?} is not a resource location"))
}

fn flag(display: &RegistryJsonValue, name: &str, default: bool) -> Result<bool, String> {
    match display.get(name) {
        Some(value) => value
            .as_bool()
            .ok_or_else(|| format!("`{name}` must be a boolean")),
        None => Ok(default),
    }
}
