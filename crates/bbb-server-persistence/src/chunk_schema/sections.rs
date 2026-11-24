//! Chunk sections: the block and biome palettes and their packed indices.

use super::{
    Java26BlockState, Java26ChunkContext, Java26ChunkSection, Java26Palette, BIOME_ENTRY_COUNT,
    BLOCK_ENTRY_COUNT, LIGHT_ARRAY_BYTES,
};
use crate::schema_support::{compound_or_empty, field, int_or};
use bbb_protocol::block_state_types::{
    vanilla_block_default_state_id, vanilla_block_property_values, vanilla_block_state_identity,
};
use bbb_protocol::block_types::vanilla_block_type_for_resource_id;
use bbb_protocol::content_registry::ContentRegistry;
use bbb_protocol::{normalize_resource_location, NbtSummaryEntry, NbtSummaryValue};
use std::collections::{BTreeMap, BTreeSet};

pub(crate) const MOD_STATE_OFFSET_PROPERTY: &str = "bbb_state_offset";

/// `SerializableChunkData.parse` (`SerializableChunkData.java:154-189`): an
/// in-height section without a container gets the empty one Vanilla creates,
/// and sections outside the light-border range are ignored.
pub fn parse_sections(
    entries: &[NbtSummaryEntry],
    context: Java26ChunkContext,
) -> Vec<Java26ChunkSection> {
    parse_sections_with_content_registry(entries, context, ContentRegistry::vanilla())
}

/// Registry-aware form of [`parse_sections`].
pub fn parse_sections_with_content_registry(
    entries: &[NbtSummaryEntry],
    context: Java26ChunkContext,
    content_registry: &ContentRegistry,
) -> Vec<Java26ChunkSection> {
    let values = match field(entries, "sections") {
        Some(NbtSummaryValue::List(values)) => values.as_slice(),
        _ => &[],
    };
    let min_y = context.min_section_y;
    let max_y = context.min_section_y + context.section_count as i32 - 1;
    let mut seen = BTreeSet::new();
    let mut sections = Vec::with_capacity(values.len());
    for value in values {
        let NbtSummaryValue::Compound(section) = value else {
            continue;
        };
        let y = int_or(section, "Y", 0) as i8;
        if i32::from(y) < min_y - 1 || i32::from(y) > max_y + 1 || !seen.insert(y) {
            continue;
        }
        let (block_states, biomes) =
            if (min_y..=max_y).contains(&i32::from(y)) {
                let block_states = field(section, "block_states").and_then(|value| {
                    parse_block_palette_with_content_registry(value, content_registry)
                });
                let biomes = field(section, "biomes").and_then(parse_biome_palette);
                if block_states.is_none() && biomes.is_none() {
                    (None, None)
                } else {
                    (
                        Some(block_states.unwrap_or_else(|| {
                            single_value_palette(air_state(), BLOCK_ENTRY_COUNT)
                        })),
                        Some(biomes.unwrap_or_else(|| {
                            single_value_palette("minecraft:plains".to_string(), BIOME_ENTRY_COUNT)
                        })),
                    )
                }
            } else {
                (None, None)
            };
        sections.push(Java26ChunkSection {
            y,
            block_states,
            biomes,
            block_light: parse_light(section, "BlockLight"),
            sky_light: parse_light(section, "SkyLight"),
        });
    }
    sections
}

fn parse_light(section: &[NbtSummaryEntry], name: &str) -> Option<Vec<i8>> {
    match field(section, name) {
        Some(NbtSummaryValue::ByteArray(bytes)) if bytes.len() == LIGHT_ARRAY_BYTES => {
            Some(bytes.clone())
        }
        _ => None,
    }
}

fn single_value_palette<T>(value: T, entry_count: usize) -> Java26Palette<T> {
    Java26Palette {
        palette: vec![value],
        data: None,
        bits_per_entry: 0,
        entry_count,
    }
}

fn air_state() -> Java26BlockState {
    Java26BlockState {
        name: "minecraft:air".to_string(),
        properties: BTreeMap::new(),
    }
}

pub fn parse_block_palette(value: &NbtSummaryValue) -> Option<Java26Palette<Java26BlockState>> {
    parse_block_palette_with_content_registry(value, ContentRegistry::vanilla())
}

pub fn parse_block_palette_with_content_registry(
    value: &NbtSummaryValue,
    content_registry: &ContentRegistry,
) -> Option<Java26Palette<Java26BlockState>> {
    let NbtSummaryValue::Compound(entries) = value else {
        return None;
    };
    let Some(NbtSummaryValue::List(values)) = field(entries, "palette") else {
        return None;
    };
    let palette = values
        .iter()
        .map(|entry| parse_block_state_with_content_registry(entry, content_registry))
        .collect();
    parse_palette(entries, palette, BLOCK_ENTRY_COUNT, PaletteKind::Blocks)
}

/// `BlockState.CODEC` as `StateHolder.codec` decodes it: unknown properties
/// are ignored and missing or invalid ones keep the default state's value.
/// An unknown block reads as air so the palette indices stay aligned.
pub fn parse_block_state(value: &NbtSummaryValue) -> Java26BlockState {
    parse_block_state_with_content_registry(value, ContentRegistry::vanilla())
}

/// Registry-aware form of [`parse_block_state`] preserving registered modded blocks.
pub fn parse_block_state_with_content_registry(
    value: &NbtSummaryValue,
    content_registry: &ContentRegistry,
) -> Java26BlockState {
    let NbtSummaryValue::Compound(state) = value else {
        return air_state();
    };
    let Some(normalized_name) = (match field(state, "Name") {
        Some(NbtSummaryValue::String(name)) => name
            .try_to_string()
            .ok()
            .and_then(|raw| normalize_resource_location(raw).ok()),
        _ => None,
    }) else {
        return air_state();
    };
    if let Some(identity) = vanilla_block_type_for_resource_id(&normalized_name)
        .and_then(|block| vanilla_block_default_state_id(block.id))
        .and_then(vanilla_block_state_identity)
    {
        let saved = compound_or_empty(state, "Properties");
        let mut properties = identity.properties.clone();
        for (name, value) in &mut properties {
            if let Some(NbtSummaryValue::String(saved)) = field(saved, name) {
                if vanilla_block_property_values(&identity.name, name)
                    .is_some_and(|values| values.iter().any(|value| saved == *value))
                {
                    *value = saved
                        .try_to_string()
                        .expect("matched a static block property");
                }
            }
        }
        return Java26BlockState {
            name: identity.name.clone(),
            properties,
        };
    }
    let Some(spec) = content_registry.block_by_resource_id(&normalized_name) else {
        return air_state();
    };
    let saved = compound_or_empty(state, "Properties");
    let mut properties = BTreeMap::new();
    if let Some(NbtSummaryValue::String(raw_offset)) = field(saved, MOD_STATE_OFFSET_PROPERTY) {
        if let Ok(offset) = raw_offset
            .try_to_string()
            .ok()
            .and_then(|raw| raw.parse::<u32>().ok())
            .ok_or(())
        {
            if offset > 0 && offset < u32::from(spec.state_count) {
                properties.insert(MOD_STATE_OFFSET_PROPERTY.to_string(), offset.to_string());
            }
        }
    }
    Java26BlockState {
        name: spec.resource_id.clone(),
        properties,
    }
}

pub fn parse_biome_palette(value: &NbtSummaryValue) -> Option<Java26Palette<String>> {
    let NbtSummaryValue::Compound(entries) = value else {
        return None;
    };
    let Some(NbtSummaryValue::List(values)) = field(entries, "palette") else {
        return None;
    };
    let palette = values
        .iter()
        .map(|value| match value {
            NbtSummaryValue::String(id) => id
                .try_to_string()
                .map(|id| normalize_resource_location(id.clone()).unwrap_or(id))
                .unwrap_or_else(|_| "minecraft:plains".into()),
            _ => "minecraft:plains".to_string(),
        })
        .collect();
    parse_palette(entries, palette, BIOME_ENTRY_COUNT, PaletteKind::Biomes)
}

#[derive(Debug, Clone, Copy)]
pub enum PaletteKind {
    Blocks,
    Biomes,
}

/// A palette whose packed `data` does not fit its declared width reads as the
/// first palette value everywhere; out-of-range indices read the same way.
pub fn parse_palette<T>(
    entries: &[NbtSummaryEntry],
    palette: Vec<T>,
    entry_count: usize,
    kind: PaletteKind,
) -> Option<Java26Palette<T>> {
    if palette.is_empty() {
        return None;
    }
    let bits_per_entry = palette_bits(palette.len(), kind);
    let data = match field(entries, "data") {
        Some(NbtSummaryValue::LongArray(values)) if bits_per_entry > 0 => {
            let values_per_long = 64 / usize::from(bits_per_entry);
            (values.len() == entry_count.div_ceil(values_per_long)).then(|| values.clone())
        }
        _ => None,
    };
    Some(Java26Palette {
        bits_per_entry: if data.is_some() { bits_per_entry } else { 0 },
        palette,
        data,
        entry_count,
    })
}

pub fn palette_bits(palette_size: usize, kind: PaletteKind) -> u8 {
    if palette_size == 1 {
        return 0;
    }
    let ceil_log2 = (usize::BITS - (palette_size - 1).leading_zeros()) as u8;
    match kind {
        PaletteKind::Blocks => ceil_log2.max(4),
        PaletteKind::Biomes => ceil_log2,
    }
}
