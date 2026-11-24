use serde::{Deserialize, Serialize};

use crate::codec::{Decoder, Encoder, ProtocolError, Result};
use crate::component::decode_rich_text_component_from_decoder_with_registry;
use crate::content_registry::ContentRegistry;
use crate::ids;
use crate::nbt_summary::{encode_nbt_summary_bounded, MAX_NBT_DEPTH};
use crate::{JavaString, RichTextComponent};
pub use bbb_facts::advancement_frame_types::AdvancementFrameType;

use super::super::{
    data_components::{
        encode_data_component_patch_summary_with_framing_and_registry, ComponentValueFraming,
    },
    inventory::{self, SparseDataComponentPatchSummary},
    normalize_resource_location, read_resource_location,
};

/// Same trusted-component budget `encode_play_clientbound_system_chat` uses.
const COMPONENT_NBT_BYTES: usize = crate::nbt_summary::VANILLA_DEFAULT_NBT_QUOTA as usize;

const MAX_ADVANCEMENT_CRITERIA: usize = 4096;
const MAX_ADVANCEMENT_LIST: usize = 65_536;
const MAX_ADVANCEMENT_REQUIREMENT_GROUPS: usize = 4096;
const MAX_ADVANCEMENT_REQUIREMENT_NAMES: usize = 4096;
const MAX_ADVANCEMENT_STRING_CHARS: usize = 32767;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UpdateAdvancements {
    pub reset: bool,
    pub added: Vec<AdvancementSummary>,
    pub removed: Vec<String>,
    pub progress: Vec<AdvancementProgressSummary>,
    pub show_advancements: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AdvancementSummary {
    pub id: String,
    pub parent: Option<String>,
    pub display: Option<AdvancementDisplaySummary>,
    pub requirements: Vec<Vec<JavaString>>,
    pub sends_telemetry_event: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AdvancementDisplaySummary {
    pub title: RichTextComponent,
    pub description: RichTextComponent,
    pub icon: AdvancementIconSummary,
    pub frame_type: AdvancementFrameType,
    pub show_toast: bool,
    pub hidden: bool,
    pub background: Option<String>,
    pub x: f32,
    pub y: f32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AdvancementIconSummary {
    pub item_id: i32,
    pub count: i32,
    pub component_patch: SparseDataComponentPatchSummary,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AdvancementProgressSummary {
    pub id: String,
    pub criteria: Vec<AdvancementCriterionProgressSummary>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AdvancementCriterionProgressSummary {
    pub name: JavaString,
    pub obtained_epoch_millis: Option<i64>,
}

pub(crate) fn decode_update_advancements_with_registry(
    decoder: &mut Decoder<'_>,
    registry: &ContentRegistry,
) -> Result<UpdateAdvancements> {
    let reset = decoder.read_bool()?;

    let added_count = read_bounded_len(decoder, MAX_ADVANCEMENT_LIST, "advancements added")?;
    let mut added = Vec::with_capacity(decoder.bounded_capacity(added_count, 5));
    for _ in 0..added_count {
        added.push(decode_advancement_summary(decoder, registry)?);
    }

    let removed_count = read_bounded_len(decoder, MAX_ADVANCEMENT_LIST, "advancements removed")?;
    let mut removed = Vec::with_capacity(decoder.bounded_capacity(removed_count, 2));
    for _ in 0..removed_count {
        removed.push(read_identifier(decoder)?);
    }

    let progress_count = read_bounded_len(decoder, MAX_ADVANCEMENT_LIST, "advancement progress")?;
    let mut progress = Vec::with_capacity(decoder.bounded_capacity(progress_count, 3));
    for _ in 0..progress_count {
        progress.push(decode_advancement_progress_summary(decoder)?);
    }

    let packet = UpdateAdvancements {
        reset,
        added,
        removed,
        progress,
        show_advancements: decoder.read_bool()?,
    };
    if !decoder.is_empty() {
        return Err(ProtocolError::InvalidData(
            "trailing bytes after update advancements packet".to_string(),
        ));
    }
    Ok(packet)
}

fn decode_advancement_summary(
    decoder: &mut Decoder<'_>,
    registry: &ContentRegistry,
) -> Result<AdvancementSummary> {
    let id = read_identifier(decoder)?;
    let parent = if decoder.read_bool()? {
        Some(read_identifier(decoder)?)
    } else {
        None
    };
    let display = if decoder.read_bool()? {
        Some(decode_advancement_display_summary(decoder, registry)?)
    } else {
        None
    };
    let requirements = decode_advancement_requirements(decoder)?;
    let sends_telemetry_event = decoder.read_bool()?;

    Ok(AdvancementSummary {
        id,
        parent,
        display,
        requirements,
        sends_telemetry_event,
    })
}

fn decode_advancement_display_summary(
    decoder: &mut Decoder<'_>,
    registry: &ContentRegistry,
) -> Result<AdvancementDisplaySummary> {
    let title = decode_rich_text_component_from_decoder_with_registry(decoder, registry)?;
    let description = decode_rich_text_component_from_decoder_with_registry(decoder, registry)?;
    let icon = decode_advancement_icon_summary(decoder, registry)?;
    let ordinal = decoder.read_var_i32()?;
    let frame_type = AdvancementFrameType::from_ordinal(ordinal).ok_or_else(|| {
        ProtocolError::InvalidData(format!("invalid advancement frame type ordinal {ordinal}"))
    })?;
    let flags = decoder.read_i32()?;
    let background = if flags & 1 != 0 {
        Some(read_identifier(decoder)?)
    } else {
        None
    };

    Ok(AdvancementDisplaySummary {
        title,
        description,
        icon,
        frame_type,
        show_toast: flags & 2 != 0,
        hidden: flags & 4 != 0,
        background,
        x: decoder.read_f32()?,
        y: decoder.read_f32()?,
    })
}

fn decode_advancement_icon_summary(
    decoder: &mut Decoder<'_>,
    registry: &ContentRegistry,
) -> Result<AdvancementIconSummary> {
    Ok(AdvancementIconSummary {
        item_id: decoder.read_var_i32()?,
        count: decoder.read_var_i32()?,
        component_patch: inventory::decode_data_component_patch_summary_with_registry(
            decoder, registry,
        )?
        .into(),
    })
}

fn decode_advancement_requirements(decoder: &mut Decoder<'_>) -> Result<Vec<Vec<JavaString>>> {
    let group_count = read_bounded_len(
        decoder,
        MAX_ADVANCEMENT_REQUIREMENT_GROUPS,
        "advancement requirement groups",
    )?;
    let mut requirements = Vec::with_capacity(decoder.bounded_capacity(group_count, 1));
    for _ in 0..group_count {
        let name_count = read_bounded_len(
            decoder,
            MAX_ADVANCEMENT_REQUIREMENT_NAMES,
            "advancement requirement names",
        )?;
        let mut names = Vec::with_capacity(decoder.bounded_capacity(name_count, 1));
        for _ in 0..name_count {
            names.push(decoder.read_java_string(MAX_ADVANCEMENT_STRING_CHARS)?);
        }
        requirements.push(names);
    }
    Ok(requirements)
}

fn decode_advancement_progress_summary(
    decoder: &mut Decoder<'_>,
) -> Result<AdvancementProgressSummary> {
    let id = read_identifier(decoder)?;
    let criterion_count = read_bounded_len(
        decoder,
        MAX_ADVANCEMENT_CRITERIA,
        "advancement progress criteria",
    )?;
    let mut criteria = Vec::with_capacity(decoder.bounded_capacity(criterion_count, 2));
    for _ in 0..criterion_count {
        let name = decoder.read_java_string(MAX_ADVANCEMENT_STRING_CHARS)?;
        let obtained_epoch_millis = if decoder.read_bool()? {
            Some(decoder.read_i64()?)
        } else {
            None
        };
        criteria.push(AdvancementCriterionProgressSummary {
            name,
            obtained_epoch_millis,
        });
    }
    Ok(AdvancementProgressSummary { id, criteria })
}

fn read_identifier(decoder: &mut Decoder<'_>) -> Result<String> {
    read_resource_location(decoder)
}

/// `ClientboundUpdateAdvancementsPacket.write` (`ClientboundUpdateAdvancementsPacket.java:49-55`).
pub fn encode_play_clientbound_update_advancements(
    update: &UpdateAdvancements,
) -> Result<(i32, Vec<u8>)> {
    encode_play_clientbound_update_advancements_with_content_registry(
        update,
        ContentRegistry::vanilla(),
    )
}

pub fn encode_play_clientbound_update_advancements_with_content_registry(
    update: &UpdateAdvancements,
    registry: &ContentRegistry,
) -> Result<(i32, Vec<u8>)> {
    let mut encoder = Encoder::new();
    encoder.write_bool(update.reset);
    encoder.write_var_i32(i32::try_from(update.added.len()).map_err(|_| {
        ProtocolError::InvalidData("update advancements added list does not fit a varint".into())
    })?);
    for summary in &update.added {
        encode_advancement_summary(&mut encoder, summary, registry)?;
    }
    encoder.write_var_i32(i32::try_from(update.removed.len()).map_err(|_| {
        ProtocolError::InvalidData("update advancements removed list does not fit a varint".into())
    })?);
    for id in &update.removed {
        encoder.write_string(&normalize_resource_location(id.clone())?);
    }
    encoder.write_var_i32(i32::try_from(update.progress.len()).map_err(|_| {
        ProtocolError::InvalidData("update advancements progress list does not fit a varint".into())
    })?);
    for progress in &update.progress {
        encode_advancement_progress_summary(&mut encoder, progress)?;
    }
    encoder.write_bool(update.show_advancements);
    Ok((
        ids::play::CLIENTBOUND_UPDATE_ADVANCEMENTS,
        encoder.into_inner(),
    ))
}

fn encode_advancement_summary(
    encoder: &mut Encoder,
    summary: &AdvancementSummary,
    registry: &ContentRegistry,
) -> Result<()> {
    encoder.write_string(&normalize_resource_location(summary.id.clone())?);
    match &summary.parent {
        Some(parent) => {
            encoder.write_bool(true);
            encoder.write_string(&normalize_resource_location(parent.clone())?);
        }
        None => encoder.write_bool(false),
    }
    match &summary.display {
        Some(display) => {
            encoder.write_bool(true);
            encode_advancement_display_summary(encoder, display, registry)?;
        }
        None => encoder.write_bool(false),
    }
    encoder.write_var_i32(i32::try_from(summary.requirements.len()).map_err(|_| {
        ProtocolError::InvalidData("advancement requirement groups do not fit a varint".into())
    })?);
    for group in &summary.requirements {
        encoder.write_var_i32(i32::try_from(group.len()).map_err(|_| {
            ProtocolError::InvalidData("advancement requirement names do not fit a varint".into())
        })?);
        for name in group {
            encoder.write_java_string(name, MAX_ADVANCEMENT_STRING_CHARS)?;
        }
    }
    encoder.write_bool(summary.sends_telemetry_event);
    Ok(())
}

/// `DisplayInfo.serializeToNetwork` (`DisplayInfo.java:106-128`): flags are
/// background, toast and hidden; `announce_to_chat` never goes out.
fn encode_advancement_display_summary(
    encoder: &mut Encoder,
    display: &AdvancementDisplaySummary,
    registry: &ContentRegistry,
) -> Result<()> {
    encode_component(encoder, &display.title)?;
    encode_component(encoder, &display.description)?;
    encoder.write_var_i32(display.icon.item_id);
    encoder.write_var_i32(display.icon.count);
    encode_data_component_patch_summary_with_framing_and_registry(
        encoder,
        &display.icon.component_patch,
        ComponentValueFraming::Direct,
        registry,
    )?;
    encoder.write_var_i32(display.frame_type.ordinal());
    let mut flags = 0i32;
    if display.background.is_some() {
        flags |= 1;
    }
    if display.show_toast {
        flags |= 2;
    }
    if display.hidden {
        flags |= 4;
    }
    encoder.write_i32(flags);
    if let Some(background) = &display.background {
        encoder.write_string(&normalize_resource_location(background.clone())?);
    }
    encoder.write_f32(display.x);
    encoder.write_f32(display.y);
    Ok(())
}

fn encode_advancement_progress_summary(
    encoder: &mut Encoder,
    progress: &AdvancementProgressSummary,
) -> Result<()> {
    encoder.write_string(&normalize_resource_location(progress.id.clone())?);
    encoder.write_var_i32(i32::try_from(progress.criteria.len()).map_err(|_| {
        ProtocolError::InvalidData("advancement progress criteria do not fit a varint".into())
    })?);
    for criterion in &progress.criteria {
        encoder.write_java_string(&criterion.name, MAX_ADVANCEMENT_STRING_CHARS)?;
        match criterion.obtained_epoch_millis {
            Some(millis) => {
                encoder.write_bool(true);
                encoder.write_i64(millis);
            }
            None => encoder.write_bool(false),
        }
    }
    Ok(())
}

fn encode_component(encoder: &mut Encoder, component: &RichTextComponent) -> Result<()> {
    encoder.write_bytes(&encode_nbt_summary_bounded(
        Some(component.component()),
        COMPONENT_NBT_BYTES,
        MAX_NBT_DEPTH,
    )?);
    Ok(())
}

fn read_bounded_len(
    decoder: &mut Decoder<'_>,
    max_len: usize,
    _what: &'static str,
) -> Result<usize> {
    let len = decoder.read_len()?;
    if len > max_len {
        return Err(ProtocolError::PacketTooLarge(len, max_len));
    }
    Ok(len)
}
