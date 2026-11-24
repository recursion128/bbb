//! The proto-generation fields a chunk keeps but this server does not fully
//! model: `UpgradeData`, `blending_data` and `below_zero_retrogen`.

use super::ticks::{parse_ticks_with_content_registry, TickKind};
use super::{
    Java26BelowZeroRetrogen, Java26BlendingData, Java26ChunkContext, Java26ChunkStatus,
    Java26UpgradeData, JAVA26_BLENDING_HEIGHT_COUNT,
};
use crate::schema_support::{compound_or_empty, field, int_or, int_value};
use bbb_protocol::content_registry::ContentRegistry;
use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue};

pub struct Java26OptionalGenerationFields {
    pub upgrade_data: Option<Java26UpgradeData>,
    pub blending_data: Option<Java26BlendingData>,
    pub below_zero_retrogen: Option<Java26BelowZeroRetrogen>,
}

pub fn parse_optional_generation_fields(
    entries: &[NbtSummaryEntry],
    context: Java26ChunkContext,
    status: Java26ChunkStatus,
    content_registry: &ContentRegistry,
) -> Java26OptionalGenerationFields {
    Java26OptionalGenerationFields {
        upgrade_data: match field(entries, "UpgradeData") {
            Some(NbtSummaryValue::Compound(fields)) => {
                Some(parse_upgrade_data(fields, context, content_registry))
            }
            _ => None,
        },
        blending_data: match field(entries, "blending_data") {
            Some(NbtSummaryValue::Compound(fields)) => parse_blending_data(fields),
            _ => None,
        },
        below_zero_retrogen: match field(entries, "below_zero_retrogen") {
            Some(NbtSummaryValue::Compound(fields)) => {
                parse_below_zero_retrogen(fields, context, status)
            }
            _ => None,
        },
    }
}

/// `UpgradeData(CompoundTag, LevelHeightAccessor)` (`UpgradeData.java:62-93`).
fn parse_upgrade_data(
    entries: &[NbtSummaryEntry],
    context: Java26ChunkContext,
    content_registry: &ContentRegistry,
) -> Java26UpgradeData {
    let sides = int_or(entries, "Sides", 0) as u8;
    let indices = match field(entries, "Indices") {
        Some(NbtSummaryValue::Compound(sections)) => Some(
            sections
                .iter()
                .filter_map(|entry| {
                    let index = entry.name.try_to_string().ok()?.parse::<usize>().ok()?;
                    let NbtSummaryValue::IntArray(values) = &entry.value else {
                        return None;
                    };
                    (index < context.section_count)
                        .then(|| (index, values.iter().map(|value| value & 0x0fff).collect()))
                })
                .collect(),
        ),
        _ => None,
    };
    let ticks = |name, kind| {
        field(entries, name).map(|value| {
            parse_ticks_with_content_registry(value, context, false, kind, content_registry)
        })
    };
    Java26UpgradeData {
        sides,
        indices,
        neighbor_block_ticks: ticks("neighbor_block_ticks", TickKind::Block),
        neighbor_fluid_ticks: ticks("neighbor_fluid_ticks", TickKind::Fluid),
    }
}

/// `BlendingData.Packed.CODEC` (`BlendingData.java:401-416`); a record that
/// does not decode is dropped, as `read(...).orElse(null)` does.
fn parse_blending_data(entries: &[NbtSummaryEntry]) -> Option<Java26BlendingData> {
    let min_section = field(entries, "min_section").and_then(int_value)?;
    let max_section = field(entries, "max_section").and_then(int_value)?;
    let height_bits = match field(entries, "heights") {
        Some(NbtSummaryValue::List(heights)) => {
            if heights.len() != JAVA26_BLENDING_HEIGHT_COUNT {
                return None;
            }
            let mut bits = [0_u64; JAVA26_BLENDING_HEIGHT_COUNT];
            for (slot, value) in bits.iter_mut().zip(heights) {
                let NbtSummaryValue::Double(value) = value else {
                    return None;
                };
                *slot = *value;
            }
            Some(bits)
        }
        _ => None,
    };
    Some(Java26BlendingData {
        min_section,
        max_section,
        height_bits,
    })
}

/// `BelowZeroRetrogen.CODEC` (`BelowZeroRetrogen.java:25-42`); work that
/// cannot apply to this chunk is dropped.
fn parse_below_zero_retrogen(
    entries: &[NbtSummaryEntry],
    context: Java26ChunkContext,
    status: Java26ChunkStatus,
) -> Option<Java26BelowZeroRetrogen> {
    let target_status = Java26ChunkStatus::parse(field(entries, "target_status")?)?;
    let max_section_y = context.min_section_y + context.section_count as i32;
    if target_status == Java26ChunkStatus::Empty
        || status >= target_status
        || context.min_section_y > -4
        || max_section_y < 0
    {
        return None;
    }
    let missing_bedrock = match field(entries, "missing_bedrock") {
        Some(NbtSummaryValue::LongArray(words)) => {
            Some(words.iter().take(4).map(|word| *word as u64).collect())
        }
        _ => None,
    };
    Some(Java26BelowZeroRetrogen {
        target_status,
        missing_bedrock,
    })
}

pub fn carving_mask(entries: &[NbtSummaryEntry]) -> Option<Vec<i64>> {
    match field(entries, "carving_mask") {
        Some(NbtSummaryValue::LongArray(values)) => Some(values.clone()),
        _ => None,
    }
}

pub fn structures(entries: &[NbtSummaryEntry]) -> NbtSummaryValue {
    NbtSummaryValue::Compound(compound_or_empty(entries, "structures").to_vec())
}
