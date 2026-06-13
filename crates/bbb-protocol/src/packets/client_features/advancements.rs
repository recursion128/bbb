use serde::{Deserialize, Serialize};

use crate::codec::{Decoder, ProtocolError, Result};

use super::super::inventory::{self, DataComponentPatchSummary};

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
    pub requirements: Vec<Vec<String>>,
    pub sends_telemetry_event: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AdvancementDisplaySummary {
    pub title: String,
    pub description: String,
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
    pub component_patch: DataComponentPatchSummary,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AdvancementFrameType {
    Task,
    Challenge,
    Goal,
}

impl AdvancementFrameType {
    fn from_ordinal(ordinal: i32) -> Result<Self> {
        Ok(match ordinal {
            0 => Self::Task,
            1 => Self::Challenge,
            2 => Self::Goal,
            other => {
                return Err(ProtocolError::InvalidData(format!(
                    "invalid advancement frame type ordinal {other}"
                )))
            }
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AdvancementProgressSummary {
    pub id: String,
    pub criteria: Vec<AdvancementCriterionProgressSummary>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AdvancementCriterionProgressSummary {
    pub name: String,
    pub obtained_epoch_millis: Option<i64>,
}

pub(crate) fn decode_update_advancements(decoder: &mut Decoder<'_>) -> Result<UpdateAdvancements> {
    let reset = decoder.read_bool()?;

    let added_count = read_bounded_len(decoder, MAX_ADVANCEMENT_LIST, "advancements added")?;
    let mut added = Vec::with_capacity(added_count);
    for _ in 0..added_count {
        added.push(decode_advancement_summary(decoder)?);
    }

    let removed_count = read_bounded_len(decoder, MAX_ADVANCEMENT_LIST, "advancements removed")?;
    let mut removed = Vec::with_capacity(removed_count);
    for _ in 0..removed_count {
        removed.push(read_identifier(decoder)?);
    }

    let progress_count = read_bounded_len(decoder, MAX_ADVANCEMENT_LIST, "advancement progress")?;
    let mut progress = Vec::with_capacity(progress_count);
    for _ in 0..progress_count {
        progress.push(decode_advancement_progress_summary(decoder)?);
    }

    Ok(UpdateAdvancements {
        reset,
        added,
        removed,
        progress,
        show_advancements: decoder.read_bool()?,
    })
}

fn decode_advancement_summary(decoder: &mut Decoder<'_>) -> Result<AdvancementSummary> {
    let id = read_identifier(decoder)?;
    let parent = if decoder.read_bool()? {
        Some(read_identifier(decoder)?)
    } else {
        None
    };
    let display = if decoder.read_bool()? {
        Some(decode_advancement_display_summary(decoder)?)
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
) -> Result<AdvancementDisplaySummary> {
    let title = super::super::decode_component_summary_from_decoder(decoder)?;
    let description = super::super::decode_component_summary_from_decoder(decoder)?;
    let icon = decode_advancement_icon_summary(decoder)?;
    let frame_type = AdvancementFrameType::from_ordinal(decoder.read_var_i32()?)?;
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

fn decode_advancement_icon_summary(decoder: &mut Decoder<'_>) -> Result<AdvancementIconSummary> {
    Ok(AdvancementIconSummary {
        item_id: decoder.read_var_i32()?,
        count: decoder.read_var_i32()?,
        component_patch: inventory::decode_data_component_patch_summary(decoder)?,
    })
}

fn decode_advancement_requirements(decoder: &mut Decoder<'_>) -> Result<Vec<Vec<String>>> {
    let group_count = read_bounded_len(
        decoder,
        MAX_ADVANCEMENT_REQUIREMENT_GROUPS,
        "advancement requirement groups",
    )?;
    let mut requirements = Vec::with_capacity(group_count);
    for _ in 0..group_count {
        let name_count = read_bounded_len(
            decoder,
            MAX_ADVANCEMENT_REQUIREMENT_NAMES,
            "advancement requirement names",
        )?;
        let mut names = Vec::with_capacity(name_count);
        for _ in 0..name_count {
            names.push(decoder.read_string(MAX_ADVANCEMENT_STRING_CHARS)?);
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
    let mut criteria = Vec::with_capacity(criterion_count);
    for _ in 0..criterion_count {
        let name = decoder.read_string(MAX_ADVANCEMENT_STRING_CHARS)?;
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
    decoder.read_string(MAX_ADVANCEMENT_STRING_CHARS)
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
