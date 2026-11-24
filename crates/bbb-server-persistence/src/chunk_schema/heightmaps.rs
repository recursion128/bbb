//! `PostProcessing` and `Heightmaps`, whose packed widths follow the
//! dimension's own column height.

use super::{Java26ChunkContext, Java26ChunkStatus};
use crate::schema_support::{compound_or_empty, field, int_value};
use bbb_protocol::{NbtSummaryEntry, NbtSummaryValue};
use std::collections::BTreeMap;

/// One offset list per world section, as `SerializableChunkData.parse`
/// (`SerializableChunkData.java:135-149`) reads them.
pub fn parse_post_processing(entries: &[NbtSummaryEntry], section_count: usize) -> Vec<Vec<i16>> {
    let sections = match field(entries, "PostProcessing") {
        Some(NbtSummaryValue::List(values)) => values.as_slice(),
        _ => &[],
    };
    (0..section_count)
        .map(|index| match sections.get(index) {
            Some(NbtSummaryValue::List(offsets)) => offsets
                .iter()
                .map(|value| int_value(value).unwrap_or(0) as i16)
                .collect(),
            _ => Vec::new(),
        })
        .collect()
}

/// The heightmaps the status keeps; a missing or mis-sized one is primed on
/// load (`SerializableChunkData.java:126-131`).
pub fn parse_heightmaps(
    entries: &[NbtSummaryEntry],
    status: Java26ChunkStatus,
    context: Java26ChunkContext,
) -> BTreeMap<String, Vec<i64>> {
    let expected_longs = expected_heightmap_longs(context);
    if expected_longs == 0 {
        return BTreeMap::new();
    }
    compound_or_empty(entries, "Heightmaps")
        .iter()
        .filter_map(|entry| {
            let key = entry.name.try_to_string().ok()?;
            let NbtSummaryValue::LongArray(values) = &entry.value else {
                return None;
            };
            (status.allows_heightmap_key(&key) && values.len() == expected_longs)
                .then(|| (key, values.clone()))
        })
        .collect()
}

pub fn expected_heightmap_longs(context: Java26ChunkContext) -> usize {
    let Some(height) = context.section_count.checked_mul(16) else {
        return 0;
    };
    if height == 0 {
        return 0;
    }
    let bits = (usize::BITS - height.leading_zeros()) as usize;
    let values_per_long = 64 / bits;
    256_usize.div_ceil(values_per_long)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zero_section_count_heightmap_longs_does_not_divide_by_zero() {
        let context = Java26ChunkContext {
            expected_x: 0,
            expected_z: 0,
            min_section_y: 0,
            section_count: 0,
        };
        assert_eq!(expected_heightmap_longs(context), 0);
        assert!(parse_heightmaps(&[], Java26ChunkStatus::Full, context).is_empty());
    }
}
