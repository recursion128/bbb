//! StructureTemplate's production file encoding.

use std::collections::HashMap;
use std::io::Write;

use bbb_protocol::block_state_types::vanilla_block_state_identity;
use bbb_protocol::content_registry::ContentRegistry;
use bbb_protocol::nbt_summary::{encode_named_nbt_summary_with_limits, entry};
use bbb_protocol::{NbtSummaryValue, MC_DATA_VERSION};
use bbb_server_types::block_state_parse::MOD_STATE_OFFSET_PROPERTY;
use flate2::write::GzEncoder;
use flate2::Compression;

use super::{error, MaterializedStructureTemplate, StructureTemplateError};

// StructureTemplate.java:631-706: every palette shares the first palette's block ids.
pub(super) fn serialize(
    template: &MaterializedStructureTemplate,
) -> Result<Vec<u8>, StructureTemplateError> {
    serialize_with_content_registry(template, ContentRegistry::vanilla())
}

pub(super) fn serialize_with_content_registry(
    template: &MaterializedStructureTemplate,
    content_registry: &ContentRegistry,
) -> Result<Vec<u8>, StructureTemplateError> {
    let mut entries = Vec::new();
    let mut blocks = Vec::new();
    if let Some(first) = template.palettes.first() {
        let mut ids = HashMap::new();
        let mut states = Vec::new();
        let mut block_ids = Vec::with_capacity(first.blocks.len());
        for block in &first.blocks {
            let id = *ids.entry(block.state).or_insert_with(|| {
                let id = states.len();
                states.push(block.state);
                id
            });
            block_ids.push(id);
            let mut record = vec![
                entry(
                    "pos",
                    int_list([block.position.x, block.position.y, block.position.z]),
                ),
                entry("state", NbtSummaryValue::Int(id as i32)),
            ];
            if let Some(nbt) = &block.nbt {
                record.push(entry("nbt", nbt.as_ref().clone()));
            }
            blocks.push(NbtSummaryValue::Compound(record));
        }
        let mut palettes = vec![state_palette(&states, content_registry)?];
        for palette in template.palettes.iter().skip(1) {
            if palette.blocks.len() != first.blocks.len() {
                return Err(error("template palettes must share one block count"));
            }
            let mut states = vec![0; states.len()];
            for (block, id) in palette.blocks.iter().zip(&block_ids) {
                states[*id] = block.state;
            }
            palettes.push(state_palette(&states, content_registry)?);
        }
        if palettes.len() == 1 {
            entries.push(entry("palette", palettes.pop().expect("one palette")));
        } else {
            entries.push(entry("palettes", NbtSummaryValue::List(palettes)));
        }
    } else {
        entries.push(entry("palette", NbtSummaryValue::List(Vec::new())));
    }
    entries.push(entry("blocks", NbtSummaryValue::List(blocks)));
    let entities = template
        .entities
        .iter()
        .map(|entity| {
            NbtSummaryValue::Compound(vec![
                entry(
                    "pos",
                    NbtSummaryValue::List(
                        entity
                            .position_bits
                            .into_iter()
                            .map(NbtSummaryValue::Double)
                            .collect(),
                    ),
                ),
                entry(
                    "blockPos",
                    int_list([
                        entity.block_position.x,
                        entity.block_position.y,
                        entity.block_position.z,
                    ]),
                ),
                entry("nbt", entity.nbt.clone()),
            ])
        })
        .collect();
    entries.push(entry("entities", NbtSummaryValue::List(entities)));
    entries.push(entry("size", int_list(template.size)));
    entries.push(entry("DataVersion", NbtSummaryValue::Int(MC_DATA_VERSION)));
    let bytes = encode_named_nbt_summary_with_limits(
        "",
        &NbtSummaryValue::Compound(entries),
        super::MAX_DECOMPRESSED_NBT_BYTES,
        super::MAX_NBT_DEPTH,
        super::MAX_NBT_NODES,
    )
    .map_err(|source| error(format!("could not encode structure template: {source}")))?;
    let mut gzip = GzEncoder::new(Vec::new(), Compression::default());
    gzip.write_all(&bytes)
        .map_err(|source| error(format!("could not compress structure template: {source}")))?;
    gzip.finish()
        .map_err(|source| error(format!("could not finish structure template: {source}")))
}

fn state_palette(
    states: &[i32],
    content_registry: &ContentRegistry,
) -> Result<NbtSummaryValue, StructureTemplateError> {
    states
        .iter()
        .map(|state| {
            if let Some(state) = vanilla_block_state_identity(*state) {
                let mut entries = vec![entry("Name", NbtSummaryValue::string(&state.name))];
                if !state.properties.is_empty() {
                    entries.push(entry(
                        "Properties",
                        NbtSummaryValue::Compound(
                            state
                                .properties
                                .iter()
                                .map(|(key, value)| {
                                    entry(key, NbtSummaryValue::string(value.clone()))
                                })
                                .collect(),
                        ),
                    ));
                }
                return Ok(NbtSummaryValue::Compound(entries));
            }
            if let Some((spec, offset)) = content_registry.block_by_state_id(*state) {
                let mut entries = vec![entry("Name", NbtSummaryValue::string(&spec.resource_id))];
                if offset > 0 {
                    entries.push(entry(
                        "Properties",
                        NbtSummaryValue::Compound(vec![entry(
                            MOD_STATE_OFFSET_PROPERTY,
                            NbtSummaryValue::string(offset.to_string()),
                        )]),
                    ));
                }
                return Ok(NbtSummaryValue::Compound(entries));
            }
            Err(error(format!("unknown template block state {state}")))
        })
        .collect::<Result<Vec<_>, _>>()
        .map(NbtSummaryValue::List)
}

fn int_list(values: [i32; 3]) -> NbtSummaryValue {
    NbtSummaryValue::List(values.into_iter().map(NbtSummaryValue::Int).collect())
}
