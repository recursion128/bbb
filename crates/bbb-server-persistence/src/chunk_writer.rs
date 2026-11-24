//! Whole-column Java 26.1 chunk document writers.

use bbb_protocol::chunk_light::SavedLightSection;
use bbb_protocol::content_registry::ContentRegistry;
use bbb_protocol::heightmap_types::{
    VANILLA_HEIGHTMAP_MOTION_BLOCKING_ID, VANILLA_HEIGHTMAP_MOTION_BLOCKING_NO_LEAVES_ID,
    VANILLA_HEIGHTMAP_OCEAN_FLOOR_ID, VANILLA_HEIGHTMAP_WORLD_SURFACE_ID,
};
use bbb_protocol::{
    block_state_types::vanilla_block_state_identity, nbt_summary::entry, NbtSummaryEntry,
    NbtSummaryValue,
};

use bbb_server_types::chunk_data::feature_heightmaps::{
    FeatureHeightmaps, ALL_HEIGHTMAPS_MASK, MOTION_BLOCKING_MASK, MOTION_BLOCKING_NO_LEAVES_MASK,
    OCEAN_FLOOR_MASK, WORLD_SURFACE_MASK,
};
use bbb_server_types::chunk_data::{
    pack_heightmap_values, ChunkColumn, ChunkGenerationState, ChunkPreCarversState,
    ChunkPreCarversStatus, SectionBiomes, SectionBlocks,
};

use super::chunk_schema::{
    sections::MOD_STATE_OFFSET_PROPERTY, ticks::saved_tick_value, write_chunk_ticks,
    Java26BelowZeroRetrogen, Java26BlendingData, Java26UpgradeData,
};
use super::schema_support::invalid_schema;
use super::Java26ChunkStatus;
use bbb_server_types::save::{NamedCompound, SaveError, SaveResult, JAVA_26_1_DATA_VERSION};

const SCHEMA: &str = "chunk_writer";
const FINAL_HEIGHTMAPS: [(&str, &str, i32, u8); 4] = [
    (
        "MOTION_BLOCKING",
        "MOTION_BLOCKING",
        VANILLA_HEIGHTMAP_MOTION_BLOCKING_ID,
        MOTION_BLOCKING_MASK,
    ),
    (
        "MOTION_BLOCKING_NO_LEAVES",
        "MOTION_BLOCKING_NO_LEAVES",
        VANILLA_HEIGHTMAP_MOTION_BLOCKING_NO_LEAVES_ID,
        MOTION_BLOCKING_NO_LEAVES_MASK,
    ),
    (
        "OCEAN_FLOOR",
        "OCEAN_FLOOR_WG",
        VANILLA_HEIGHTMAP_OCEAN_FLOOR_ID,
        OCEAN_FLOOR_MASK,
    ),
    (
        "WORLD_SURFACE",
        "WORLD_SURFACE_WG",
        VANILLA_HEIGHTMAP_WORLD_SURFACE_ID,
        WORLD_SURFACE_MASK,
    ),
];
const SECTION_VOLUME: usize = 16 * 16 * 16;
const SECTION_HEIGHT: i32 = 16;
const MIN_BLOCK_PALETTE_BITS: usize = 4;

#[derive(Debug, Clone, Copy, Default)]
pub struct Java26LegacyChunkFields<'a> {
    pub blending_data: Option<&'a Java26BlendingData>,
    pub upgrade_data: Option<&'a Java26UpgradeData>,
    pub below_zero_retrogen: Option<&'a Java26BelowZeroRetrogen>,
}

/// Emits loaded official 26.1 proto-generation / migration fields onto a whole
/// chunk document (`SerializableChunkData.java:455-466`).
pub fn write_legacy_chunk_fields(
    doc: &mut NamedCompound,
    status: Java26ChunkStatus,
    fields: Java26LegacyChunkFields<'_>,
) {
    doc.entries_mut().retain(|field| {
        field.name != "blending_data"
            && field.name != "UpgradeData"
            && field.name != "below_zero_retrogen"
    });
    if let Some(blending) = fields.blending_data {
        let mut entries = vec![
            entry("min_section", NbtSummaryValue::Int(blending.min_section())),
            entry("max_section", NbtSummaryValue::Int(blending.max_section())),
        ];
        if let Some(bits) = blending.height_bits() {
            entries.push(entry(
                "heights",
                NbtSummaryValue::List(bits.iter().copied().map(NbtSummaryValue::Double).collect()),
            ));
        }
        doc.entries_mut()
            .push(entry("blending_data", NbtSummaryValue::Compound(entries)));
    }
    if let Some(upgrade) = fields.upgrade_data {
        let mut entries = vec![entry("Sides", NbtSummaryValue::Byte(upgrade.sides() as i8))];
        if let Some(indices) = upgrade.indices() {
            entries.push(entry(
                "Indices",
                NbtSummaryValue::Compound(
                    indices
                        .iter()
                        .map(|(section_index, values)| NbtSummaryEntry {
                            name: section_index.to_string().into(),
                            value: NbtSummaryValue::IntArray(values.clone()),
                        })
                        .collect(),
                ),
            ));
        }
        if let Some(ticks) = upgrade.neighbor_block_ticks() {
            entries.push(entry(
                "neighbor_block_ticks",
                NbtSummaryValue::List(ticks.iter().map(saved_tick_value).collect()),
            ));
        }
        if let Some(ticks) = upgrade.neighbor_fluid_ticks() {
            entries.push(entry(
                "neighbor_fluid_ticks",
                NbtSummaryValue::List(ticks.iter().map(saved_tick_value).collect()),
            ));
        }
        doc.entries_mut()
            .push(entry("UpgradeData", NbtSummaryValue::Compound(entries)));
    }
    if let Some(retrogen) = fields
        .below_zero_retrogen
        .filter(|retrogen| status < retrogen.target_status())
    {
        let mut entries = vec![entry(
            "target_status",
            NbtSummaryValue::string(retrogen.target_status().as_str()),
        )];
        if let Some(words) = retrogen.missing_bedrock_words() {
            entries.push(entry(
                "missing_bedrock",
                NbtSummaryValue::LongArray(words.iter().map(|&word| word as i64).collect()),
            ));
        }
        doc.entries_mut().push(entry(
            "below_zero_retrogen",
            NbtSummaryValue::Compound(entries),
        ));
    }
}

pub fn full_chunk_document(
    column: &ChunkColumn,
    chunk_x: i32,
    chunk_z: i32,
    biome_name: &dyn Fn(i32) -> Option<String>,
    surface_y: i32,
) -> SaveResult<NamedCompound> {
    full_chunk_document_with_content_registry(
        column,
        chunk_x,
        chunk_z,
        biome_name,
        surface_y,
        ContentRegistry::vanilla(),
    )
}

pub fn full_chunk_document_with_content_registry(
    column: &ChunkColumn,
    chunk_x: i32,
    chunk_z: i32,
    biome_name: &dyn Fn(i32) -> Option<String>,
    surface_y: i32,
    content_registry: &ContentRegistry,
) -> SaveResult<NamedCompound> {
    let _surface = surface_y
        .checked_sub(column.min_y())
        .and_then(|height| height.checked_add(1))
        .filter(|height| *height >= 0)
        .ok_or_else(|| invalid_schema(SCHEMA, "the column surface is outside the world"))?;
    full_chunk_document_with_light_and_content_registry(
        column,
        chunk_x,
        chunk_z,
        biome_name,
        None,
        None,
        &[],
        false,
        content_registry,
    )
}

#[allow(clippy::too_many_arguments)]
pub fn full_chunk_document_with_light(
    column: &ChunkColumn,
    chunk_x: i32,
    chunk_z: i32,
    biome_name: &dyn Fn(i32) -> Option<String>,
    final_heightmaps: Option<&FeatureHeightmaps>,
    structures: Option<&NbtSummaryValue>,
    light_sections: &[SavedLightSection],
    is_light_on: bool,
) -> SaveResult<NamedCompound> {
    full_chunk_document_with_light_and_content_registry(
        column,
        chunk_x,
        chunk_z,
        biome_name,
        final_heightmaps,
        structures,
        light_sections,
        is_light_on,
        ContentRegistry::vanilla(),
    )
}

#[allow(clippy::too_many_arguments)]
pub fn full_chunk_document_with_light_and_content_registry(
    column: &ChunkColumn,
    chunk_x: i32,
    chunk_z: i32,
    biome_name: &dyn Fn(i32) -> Option<String>,
    final_heightmaps: Option<&FeatureHeightmaps>,
    structures: Option<&NbtSummaryValue>,
    light_sections: &[SavedLightSection],
    is_light_on: bool,
    content_registry: &ContentRegistry,
) -> SaveResult<NamedCompound> {
    build_chunk_document_core(
        column,
        chunk_x,
        chunk_z,
        biome_name,
        final_heightmaps,
        ALL_HEIGHTMAPS_MASK,
        false,
        structures,
        light_sections,
        is_light_on,
        content_registry,
    )
}

#[allow(clippy::too_many_arguments)]
fn build_chunk_document_core(
    column: &ChunkColumn,
    chunk_x: i32,
    chunk_z: i32,
    biome_name: &dyn Fn(i32) -> Option<String>,
    final_heightmaps: Option<&FeatureHeightmaps>,
    heightmap_mask: u8,
    worldgen_heightmap_names: bool,
    structures: Option<&NbtSummaryValue>,
    light_sections: &[SavedLightSection],
    is_light_on: bool,
    content_registry: &ContentRegistry,
) -> SaveResult<NamedCompound> {
    let heightmaps = if heightmap_mask == 0 {
        Vec::new()
    } else {
        let primed;
        let maps = match final_heightmaps {
            Some(maps) => maps,
            None => {
                primed = FeatureHeightmaps::prime_with_mask(column, heightmap_mask);
                &primed
            }
        };
        packed_heightmaps(
            column,
            maps,
            heightmap_mask,
            worldgen_heightmap_names,
            &format!("full chunk ({chunk_x},{chunk_z}) persistence"),
        )?
    };
    let min_section_y = column.min_y() / SECTION_HEIGHT;
    let mut sections = Vec::with_capacity(column.section_count() + 2);
    for index in 0..column.section_count() {
        let y = min_section_y
            .checked_add(i32::try_from(index).map_err(|_| section_overflow())?)
            .ok_or_else(section_overflow)?;
        let mut section = vec![
            entry(
                "Y",
                NbtSummaryValue::Byte(i8::try_from(y).map_err(|_| section_overflow())?),
            ),
            entry(
                "block_states",
                block_states_payload(column.section(index), content_registry)?,
            ),
            entry(
                "biomes",
                biomes_payload(column.section_biomes(index), biome_name)?,
            ),
        ];
        if let Some(light) = light_sections.iter().find(|light| light.y == y) {
            append_light_payload(&mut section, light);
        }
        sections.push(NbtSummaryValue::Compound(section));
    }
    let min_light_section_y = min_section_y.checked_sub(1).ok_or_else(section_overflow)?;
    let max_section_y = min_section_y
        .checked_add(i32::try_from(column.section_count()).map_err(|_| section_overflow())?)
        .ok_or_else(section_overflow)?;
    for light in light_sections {
        if light.y < min_light_section_y || light.y > max_section_y {
            return Err(invalid_schema(
                SCHEMA,
                format!("light section {} is outside the dimension padding", light.y),
            ));
        }
        if (min_section_y..max_section_y).contains(&light.y) {
            continue;
        }
        let mut section = vec![entry(
            "Y",
            NbtSummaryValue::Byte(i8::try_from(light.y).map_err(|_| section_overflow())?),
        )];
        append_light_payload(&mut section, light);
        sections.push(NbtSummaryValue::Compound(section));
    }
    sections.sort_by_key(|section| match section {
        NbtSummaryValue::Compound(entries) => entries.iter().find_map(|entry| {
            (entry.name == "Y").then_some(match &entry.value {
                NbtSummaryValue::Byte(value) => *value,
                _ => unreachable!("writer emits byte section Y"),
            })
        }),
        _ => unreachable!("writer emits compound sections"),
    });

    let mut document = vec![
        entry("DataVersion", NbtSummaryValue::Int(JAVA_26_1_DATA_VERSION)),
        entry("xPos", NbtSummaryValue::Int(chunk_x)),
        entry("yPos", NbtSummaryValue::Int(min_section_y)),
        entry("zPos", NbtSummaryValue::Int(chunk_z)),
        entry(
            "InhabitedTime",
            NbtSummaryValue::Long(column.inhabited_time()),
        ),
        entry(
            "Status",
            NbtSummaryValue::string(Java26ChunkStatus::Full.as_str()),
        ),
        entry("sections", NbtSummaryValue::List(sections)),
        entry("block_entities", NbtSummaryValue::List(Vec::new())),
    ];
    if is_light_on {
        document.push(entry("isLightOn", NbtSummaryValue::Byte(1)));
    }
    document.extend([
        entry(
            "PostProcessing",
            NbtSummaryValue::List(
                (0..column.section_count())
                    .map(|_| NbtSummaryValue::List(Vec::new()))
                    .collect(),
            ),
        ),
        entry("Heightmaps", NbtSummaryValue::Compound(heightmaps)),
        entry(
            "structures",
            structures.cloned().unwrap_or_else(empty_structures),
        ),
    ]);
    let mut document = NamedCompound::new("", document);
    let block_ticks = column
        .block_ticks()
        .pack(0)
        .into_iter()
        .map(|tick| tick.to_saved())
        .collect::<Vec<_>>();
    let fluid_ticks = column
        .fluid_ticks()
        .pack(0)
        .into_iter()
        .map(|tick| tick.to_saved())
        .collect::<Vec<_>>();
    write_chunk_ticks(&mut document, 0, &block_ticks, &fluid_ticks);
    Ok(document)
}

#[allow(clippy::too_many_arguments)]
fn build_generation_chunk_document(
    column: &ChunkColumn,
    chunk_x: i32,
    chunk_z: i32,
    biome_name: &dyn Fn(i32) -> Option<String>,
    status: Java26ChunkStatus,
    positions: &[(i32, i32, i32)],
    carving_mask: Option<&[i64]>,
    final_heightmaps: Option<&FeatureHeightmaps>,
    proto_entities: &[Vec<NbtSummaryEntry>],
    structures: Option<&NbtSummaryValue>,
    light_sections: &[SavedLightSection],
    is_light_on: bool,
    content_registry: &ContentRegistry,
) -> SaveResult<NamedCompound> {
    let (heightmap_mask, worldgen_names) = match status {
        Java26ChunkStatus::Empty
        | Java26ChunkStatus::StructureStarts
        | Java26ChunkStatus::StructureReferences
        | Java26ChunkStatus::Biomes
        | Java26ChunkStatus::Carvers => (0, false),
        Java26ChunkStatus::Noise | Java26ChunkStatus::Surface => {
            (WORLD_SURFACE_MASK | OCEAN_FLOOR_MASK, true)
        }
        _ => (ALL_HEIGHTMAPS_MASK, false),
    };
    let mut doc = build_chunk_document_core(
        column,
        chunk_x,
        chunk_z,
        biome_name,
        final_heightmaps,
        heightmap_mask,
        worldgen_names,
        structures,
        light_sections,
        is_light_on,
        content_registry,
    )?;

    let mut post_processing = vec![Vec::new(); column.section_count()];
    for &(x, y, z) in positions {
        if x.div_euclid(16) != chunk_x || z.div_euclid(16) != chunk_z {
            return Err(invalid_schema(
                SCHEMA,
                "postprocessing position is outside the chunk",
            ));
        }
        let section = y
            .checked_sub(column.min_y())
            .filter(|offset| *offset >= 0)
            .and_then(|offset| usize::try_from(offset / SECTION_HEIGHT).ok())
            .filter(|section| *section < post_processing.len())
            .ok_or_else(|| {
                invalid_schema(SCHEMA, "postprocessing position is outside build height")
            })?;
        let packed = (x.rem_euclid(16) | y.rem_euclid(16) << 4 | z.rem_euclid(16) << 8) as i16;
        post_processing[section].push(NbtSummaryValue::Short(packed));
    }

    for field in doc.entries_mut() {
        if field.name == "Status" {
            field.value = NbtSummaryValue::string(status.as_str());
        } else if field.name == "PostProcessing" {
            field.value = NbtSummaryValue::List(
                post_processing
                    .iter()
                    .cloned()
                    .map(NbtSummaryValue::List)
                    .collect(),
            );
        }
    }

    if status != Java26ChunkStatus::Full {
        doc.entries_mut().push(entry(
            "entities",
            NbtSummaryValue::List(
                proto_entities
                    .iter()
                    .cloned()
                    .map(NbtSummaryValue::Compound)
                    .collect(),
            ),
        ));
        if let Some(mask) = carving_mask {
            doc.entries_mut().push(entry(
                "carving_mask",
                NbtSummaryValue::LongArray(mask.to_vec()),
            ));
        }
    }

    Ok(doc)
}

pub fn pre_carvers_chunk_document(
    column: &ChunkColumn,
    chunk_x: i32,
    chunk_z: i32,
    biome_name: &dyn Fn(i32) -> Option<String>,
    state: &ChunkPreCarversState,
    structures: Option<&NbtSummaryValue>,
    proto_entities: &[Vec<NbtSummaryEntry>],
) -> SaveResult<NamedCompound> {
    pre_carvers_chunk_document_with_content_registry(
        column,
        chunk_x,
        chunk_z,
        biome_name,
        state,
        structures,
        proto_entities,
        ContentRegistry::vanilla(),
    )
}

#[allow(clippy::too_many_arguments)]
pub fn pre_carvers_chunk_document_with_content_registry(
    column: &ChunkColumn,
    chunk_x: i32,
    chunk_z: i32,
    biome_name: &dyn Fn(i32) -> Option<String>,
    state: &ChunkPreCarversState,
    structures: Option<&NbtSummaryValue>,
    proto_entities: &[Vec<NbtSummaryEntry>],
    content_registry: &ContentRegistry,
) -> SaveResult<NamedCompound> {
    let status = match state.status() {
        ChunkPreCarversStatus::Empty => Java26ChunkStatus::Empty,
        ChunkPreCarversStatus::StructureStarts => Java26ChunkStatus::StructureStarts,
        ChunkPreCarversStatus::StructureReferences => Java26ChunkStatus::StructureReferences,
        ChunkPreCarversStatus::Biomes => Java26ChunkStatus::Biomes,
        ChunkPreCarversStatus::Noise => Java26ChunkStatus::Noise,
        ChunkPreCarversStatus::Surface => Java26ChunkStatus::Surface,
    };
    build_generation_chunk_document(
        column,
        chunk_x,
        chunk_z,
        biome_name,
        status,
        state.postprocessing_positions(),
        None,
        None,
        proto_entities,
        structures,
        &[],
        false,
        content_registry,
    )
}

pub fn generation_state_chunk_document(
    column: &ChunkColumn,
    chunk_x: i32,
    chunk_z: i32,
    biome_name: &dyn Fn(i32) -> Option<String>,
    state: &ChunkGenerationState,
    structures: Option<&NbtSummaryValue>,
    proto_entities: &[Vec<NbtSummaryEntry>],
) -> SaveResult<NamedCompound> {
    generation_state_chunk_document_with_content_registry(
        column,
        chunk_x,
        chunk_z,
        biome_name,
        state,
        structures,
        proto_entities,
        ContentRegistry::vanilla(),
    )
}

#[allow(clippy::too_many_arguments)]
pub fn generation_state_chunk_document_with_content_registry(
    column: &ChunkColumn,
    chunk_x: i32,
    chunk_z: i32,
    biome_name: &dyn Fn(i32) -> Option<String>,
    state: &ChunkGenerationState,
    structures: Option<&NbtSummaryValue>,
    proto_entities: &[Vec<NbtSummaryEntry>],
    content_registry: &ContentRegistry,
) -> SaveResult<NamedCompound> {
    if matches!(
        state,
        ChunkGenerationState::Light(_)
            | ChunkGenerationState::Spawn(_)
            | ChunkGenerationState::NoiseFull(_)
    ) {
        return Err(invalid_schema(
            SCHEMA,
            "LIGHT persistence requires the shared light-section projection",
        ));
    }
    generation_state_chunk_document_with_light_and_content_registry(
        column,
        chunk_x,
        chunk_z,
        biome_name,
        state,
        structures,
        proto_entities,
        &[],
        content_registry,
    )
}

#[allow(clippy::too_many_arguments)]
pub fn generation_state_chunk_document_with_light(
    column: &ChunkColumn,
    chunk_x: i32,
    chunk_z: i32,
    biome_name: &dyn Fn(i32) -> Option<String>,
    state: &ChunkGenerationState,
    structures: Option<&NbtSummaryValue>,
    proto_entities: &[Vec<NbtSummaryEntry>],
    light_sections: &[SavedLightSection],
) -> SaveResult<NamedCompound> {
    generation_state_chunk_document_with_light_and_content_registry(
        column,
        chunk_x,
        chunk_z,
        biome_name,
        state,
        structures,
        proto_entities,
        light_sections,
        ContentRegistry::vanilla(),
    )
}

#[allow(clippy::too_many_arguments)]
pub fn generation_state_chunk_document_with_light_and_content_registry(
    column: &ChunkColumn,
    chunk_x: i32,
    chunk_z: i32,
    biome_name: &dyn Fn(i32) -> Option<String>,
    state: &ChunkGenerationState,
    structures: Option<&NbtSummaryValue>,
    proto_entities: &[Vec<NbtSummaryEntry>],
    light_sections: &[SavedLightSection],
    content_registry: &ContentRegistry,
) -> SaveResult<NamedCompound> {
    match state {
        ChunkGenerationState::Full {
            surface_y,
            postprocessing_positions,
        } => {
            let _surface = surface_y
                .checked_sub(column.min_y())
                .and_then(|height| height.checked_add(1))
                .filter(|height| *height >= 0)
                .ok_or_else(|| invalid_schema(SCHEMA, "the column surface is outside the world"))?;
            build_generation_chunk_document(
                column,
                chunk_x,
                chunk_z,
                biome_name,
                Java26ChunkStatus::Full,
                postprocessing_positions,
                None,
                None,
                &[],
                structures,
                light_sections,
                true,
                content_registry,
            )
        }
        ChunkGenerationState::Carvers {
            postprocessing_positions,
            carving_mask,
        } => build_generation_chunk_document(
            column,
            chunk_x,
            chunk_z,
            biome_name,
            Java26ChunkStatus::Carvers,
            postprocessing_positions,
            Some(carving_mask),
            None,
            proto_entities,
            structures,
            &[],
            false,
            content_registry,
        ),
        ChunkGenerationState::Features(features) => build_generation_chunk_document(
            column,
            chunk_x,
            chunk_z,
            biome_name,
            Java26ChunkStatus::Features,
            features.postprocessing_positions(),
            Some(features.carving_mask()),
            Some(features.final_heightmaps()),
            proto_entities,
            structures,
            &[],
            false,
            content_registry,
        ),
        ChunkGenerationState::InitializeLight(initialized) => build_generation_chunk_document(
            column,
            chunk_x,
            chunk_z,
            biome_name,
            Java26ChunkStatus::InitializeLight,
            initialized.features().postprocessing_positions(),
            Some(initialized.features().carving_mask()),
            Some(initialized.features().final_heightmaps()),
            proto_entities,
            structures,
            light_sections,
            false,
            content_registry,
        ),
        ChunkGenerationState::Light(light) => build_generation_chunk_document(
            column,
            chunk_x,
            chunk_z,
            biome_name,
            Java26ChunkStatus::Light,
            light.initialized().features().postprocessing_positions(),
            Some(light.initialized().features().carving_mask()),
            Some(light.initialized().features().final_heightmaps()),
            proto_entities,
            structures,
            light_sections,
            light.is_light_correct(),
            content_registry,
        ),
        ChunkGenerationState::Spawn(spawn) => build_generation_chunk_document(
            column,
            chunk_x,
            chunk_z,
            biome_name,
            Java26ChunkStatus::Spawn,
            spawn
                .light()
                .initialized()
                .features()
                .postprocessing_positions(),
            Some(spawn.light().initialized().features().carving_mask()),
            Some(spawn.light().initialized().features().final_heightmaps()),
            proto_entities,
            structures,
            light_sections,
            spawn.light().is_light_correct(),
            content_registry,
        ),
        ChunkGenerationState::NoiseFull(full) => build_generation_chunk_document(
            column,
            chunk_x,
            chunk_z,
            biome_name,
            Java26ChunkStatus::Full,
            full.light()
                .initialized()
                .features()
                .postprocessing_positions(),
            None,
            Some(full.light().initialized().features().final_heightmaps()),
            &[],
            structures,
            light_sections,
            full.light().is_light_correct(),
            content_registry,
        ),
    }
}

fn packed_heightmaps(
    column: &ChunkColumn,
    final_heightmaps: &FeatureHeightmaps,
    mask: u8,
    worldgen_names: bool,
    path: &str,
) -> SaveResult<Vec<NbtSummaryEntry>> {
    let world_height = column
        .section_count()
        .checked_mul(SECTION_HEIGHT as usize)
        .ok_or_else(|| invalid_schema(path, "column height overflows"))?;
    FINAL_HEIGHTMAPS
        .iter()
        .filter(|(_, _, _, kind_mask)| mask & kind_mask != 0)
        .map(|(final_key, wg_key, kind, _)| {
            let key = if worldgen_names { *wg_key } else { *final_key };
            let values = final_heightmaps
                .values(*kind)
                .expect("the final heightmap table contains only final kinds");
            Ok(entry(
                key,
                NbtSummaryValue::LongArray(pack_heightmap_values(
                    values,
                    column.min_y(),
                    world_height,
                    &format!("{path}.Heightmaps.{key}"),
                )?),
            ))
        })
        .collect()
}

fn append_light_payload(entries: &mut Vec<NbtSummaryEntry>, light: &SavedLightSection) {
    if let Some(bytes) = &light.block_light {
        entries.push(entry(
            "BlockLight",
            NbtSummaryValue::ByteArray(bytes.clone()),
        ));
    }
    if let Some(bytes) = &light.sky_light {
        entries.push(entry("SkyLight", NbtSummaryValue::ByteArray(bytes.clone())));
    }
}

fn empty_structures() -> NbtSummaryValue {
    NbtSummaryValue::Compound(vec![
        entry("starts", NbtSummaryValue::Compound(Vec::new())),
        entry("References", NbtSummaryValue::Compound(Vec::new())),
    ])
}

fn biomes_payload(
    section: &SectionBiomes,
    biome_name: &dyn Fn(i32) -> Option<String>,
) -> SaveResult<NbtSummaryValue> {
    if let SectionBiomes::Single(biome) = section {
        let name = biome_name(*biome).ok_or_else(|| {
            invalid_schema(
                SCHEMA,
                format!("biome id {biome} is absent from the frozen registry"),
            )
        })?;
        return Ok(NbtSummaryValue::Compound(vec![entry(
            "palette",
            NbtSummaryValue::List(vec![NbtSummaryValue::string(name)]),
        )]));
    }
    let mut palette_ids: Vec<i32> = Vec::new();
    let mut indices = [0_usize; 64];
    for (entry_index, slot) in indices.iter_mut().enumerate() {
        let x = entry_index & 3;
        let z = (entry_index >> 2) & 3;
        let y = entry_index >> 4;
        let biome = section.get(x, y, z);
        let palette_index = match palette_ids.iter().position(|&candidate| candidate == biome) {
            Some(index) => index,
            None => {
                palette_ids.push(biome);
                palette_ids.len() - 1
            }
        };
        *slot = palette_index;
    }
    let palette = palette_ids
        .iter()
        .map(|&biome| {
            let name = biome_name(biome).ok_or_else(|| {
                invalid_schema(
                    SCHEMA,
                    format!("biome id {biome} is absent from the frozen registry"),
                )
            })?;
            Ok(NbtSummaryValue::string(name))
        })
        .collect::<SaveResult<Vec<_>>>()?;
    let mut fields = vec![entry("palette", NbtSummaryValue::List(palette))];
    if palette_ids.len() > 1 {
        let bits = usize::BITS as usize - (palette_ids.len() - 1).leading_zeros() as usize;
        fields.push(entry(
            "data",
            NbtSummaryValue::LongArray(pack(&indices, bits)),
        ));
    }
    Ok(NbtSummaryValue::Compound(fields))
}

fn block_states_payload(
    section: &SectionBlocks,
    content_registry: &ContentRegistry,
) -> SaveResult<NbtSummaryValue> {
    match section {
        SectionBlocks::Single(state) => Ok(NbtSummaryValue::Compound(vec![entry(
            "palette",
            NbtSummaryValue::List(vec![block_state_entry(*state, content_registry)?]),
        )])),
        SectionBlocks::Indirect {
            bits,
            palette,
            data,
        } => {
            let mut entries = vec![entry(
                "palette",
                NbtSummaryValue::List(
                    palette
                        .iter()
                        .map(|state| block_state_entry(*state, content_registry))
                        .collect::<SaveResult<Vec<_>>>()?,
                ),
            )];
            if palette.len() > 1 {
                let target_bits = usize::BITS - (palette.len() - 1).leading_zeros();
                let target_bits = usize::try_from(target_bits)
                    .unwrap_or(usize::MAX)
                    .max(MIN_BLOCK_PALETTE_BITS);
                let expected_words = SECTION_VOLUME.div_ceil(64 / target_bits);
                let packed = if usize::from(*bits) == target_bits && data.len() == expected_words {
                    data.iter().map(|&word| word as i64).collect()
                } else {
                    let indices = (0..SECTION_VOLUME)
                        .map(|index| {
                            let (x, y, z) = (index & 15, index >> 8, (index >> 4) & 15);
                            let state = section.get(x, y, z);
                            palette
                                .iter()
                                .position(|entry| *entry == state)
                                .ok_or_else(|| {
                                    invalid_schema(SCHEMA, "a cell is outside its own palette")
                                })
                        })
                        .collect::<SaveResult<Vec<_>>>()?;
                    pack(&indices, target_bits)
                };
                entries.push(entry("data", NbtSummaryValue::LongArray(packed)));
            }
            Ok(NbtSummaryValue::Compound(entries))
        }
    }
}

fn block_state_entry(
    state: i32,
    content_registry: &ContentRegistry,
) -> SaveResult<NbtSummaryValue> {
    if let Some(identity) = vanilla_block_state_identity(state) {
        let mut fields = vec![entry(
            "Name",
            NbtSummaryValue::string(identity.name.clone()),
        )];
        if !identity.properties.is_empty() {
            fields.push(entry(
                "Properties",
                NbtSummaryValue::Compound(
                    identity
                        .properties
                        .iter()
                        .map(|(name, value)| entry(name, NbtSummaryValue::string(value.clone())))
                        .collect(),
                ),
            ));
        }
        return Ok(NbtSummaryValue::Compound(fields));
    }
    let Some((spec, offset)) = content_registry.block_by_state_id(state) else {
        return Err(invalid_schema(
            SCHEMA,
            format!("block state {state} is not in the catalog"),
        ));
    };
    let mut fields = vec![entry(
        "Name",
        NbtSummaryValue::string(spec.resource_id.clone()),
    )];
    if offset > 0 {
        fields.push(entry(
            "Properties",
            NbtSummaryValue::Compound(vec![entry(
                MOD_STATE_OFFSET_PROPERTY,
                NbtSummaryValue::string(offset.to_string()),
            )]),
        ));
    }
    Ok(NbtSummaryValue::Compound(fields))
}

fn pack(values: &[usize], bits: usize) -> Vec<i64> {
    let per_long = 64 / bits;
    let mut packed = vec![0_i64; values.len().div_ceil(per_long)];
    for (index, value) in values.iter().enumerate() {
        let shift = (index % per_long) * bits;
        packed[index / per_long] |= ((*value as u64) << shift) as i64;
    }
    packed
}

fn section_overflow() -> SaveError {
    invalid_schema(SCHEMA, "the column has more sections than a save can index")
}

#[cfg(test)]
mod tests {
    use bbb_protocol::block_types::VANILLA_BLOCK_STONE_ID;
    use bbb_protocol::packets::BlockPos;
    use bbb_protocol::registry_data_types::VanillaFluidIdentity;

    use super::*;
    use crate::{Java26ChunkContext, Java26ChunkData};
    use bbb_server_types::chunk_ticks::{BlockTickType, FluidTickType, TickPriority};

    fn column() -> ChunkColumn {
        ChunkColumn::new(-64, 24, 0, 0)
    }

    #[test]
    fn write_legacy_chunk_fields_round_trips_blending_upgrade_and_retrogen() {
        let column = column();
        let biome = |_: i32| Some("minecraft:plains".to_owned());
        let context = Java26ChunkContext {
            expected_x: 0,
            expected_z: 0,
            min_section_y: -4,
            section_count: 24,
        };
        let mut fixture = full_chunk_document(&column, 0, 0, &biome, 0).unwrap();
        fixture
            .entries_mut()
            .iter_mut()
            .find(|e| e.name == "Status")
            .unwrap()
            .value = NbtSummaryValue::string("minecraft:carvers");
        fixture
            .entries_mut()
            .iter_mut()
            .find(|e| e.name == "Heightmaps")
            .unwrap()
            .value = NbtSummaryValue::Compound(Vec::new());
        fixture
            .entries_mut()
            .push(entry("entities", NbtSummaryValue::List(Vec::new())));
        fixture.entries_mut().push(entry(
            "blending_data",
            NbtSummaryValue::Compound(vec![
                entry("min_section", NbtSummaryValue::Int(-4)),
                entry("max_section", NbtSummaryValue::Int(20)),
                entry(
                    "heights",
                    NbtSummaryValue::List(
                        (0..16)
                            .map(|idx| NbtSummaryValue::Double((f64::from(idx) + 60.5).to_bits()))
                            .collect(),
                    ),
                ),
            ]),
        ));
        fixture.entries_mut().push(entry(
            "UpgradeData",
            NbtSummaryValue::Compound(vec![
                entry("Sides", NbtSummaryValue::Byte(0x15)),
                entry(
                    "Indices",
                    NbtSummaryValue::Compound(vec![entry(
                        "3",
                        NbtSummaryValue::IntArray(vec![7, 42]),
                    )]),
                ),
                entry(
                    "neighbor_block_ticks",
                    NbtSummaryValue::List(vec![NbtSummaryValue::Compound(vec![
                        entry("i", NbtSummaryValue::string("minecraft:stone")),
                        entry("x", NbtSummaryValue::Int(1)),
                        entry("y", NbtSummaryValue::Int(2)),
                        entry("z", NbtSummaryValue::Int(3)),
                        entry("t", NbtSummaryValue::Int(4)),
                        entry("p", NbtSummaryValue::Int(0)),
                    ])]),
                ),
                entry(
                    "neighbor_fluid_ticks",
                    NbtSummaryValue::List(vec![NbtSummaryValue::Compound(vec![
                        entry("i", NbtSummaryValue::string("minecraft:water")),
                        entry("x", NbtSummaryValue::Int(5)),
                        entry("y", NbtSummaryValue::Int(6)),
                        entry("z", NbtSummaryValue::Int(7)),
                        entry("t", NbtSummaryValue::Int(8)),
                        entry("p", NbtSummaryValue::Int(-1)),
                    ])]),
                ),
            ]),
        ));
        fixture.entries_mut().push(entry(
            "below_zero_retrogen",
            NbtSummaryValue::Compound(vec![
                entry("target_status", NbtSummaryValue::string("minecraft:spawn")),
                entry(
                    "missing_bedrock",
                    NbtSummaryValue::LongArray(vec![0x11, 0x22, 0x33, 0x44]),
                ),
            ]),
        ));
        let loaded = Java26ChunkData::parse(&fixture, context).unwrap();

        let mut proto_doc = generation_state_chunk_document(
            &column,
            0,
            0,
            &biome,
            &ChunkGenerationState::Carvers {
                postprocessing_positions: Vec::new(),
                carving_mask: vec![9],
            },
            None,
            &[],
        )
        .unwrap();
        write_legacy_chunk_fields(
            &mut proto_doc,
            Java26ChunkStatus::Carvers,
            Java26LegacyChunkFields {
                blending_data: loaded.blending_data(),
                upgrade_data: loaded.upgrade_data(),
                below_zero_retrogen: loaded.below_zero_retrogen(),
            },
        );
        crate::replace_block_entities(&mut proto_doc, vec![record(2, -60, 4, "minecraft:banner")]);
        let reparsed_proto = Java26ChunkData::parse(&proto_doc, context).unwrap();
        assert_eq!(reparsed_proto.blending_data(), loaded.blending_data());
        assert_eq!(reparsed_proto.upgrade_data(), loaded.upgrade_data());
        assert_eq!(
            reparsed_proto.below_zero_retrogen(),
            loaded.below_zero_retrogen()
        );
        assert_eq!(
            reparsed_proto.block_entities(),
            &[record(2, -60, 4, "minecraft:banner")]
        );

        let mut full_doc = full_chunk_document(&column, 0, 0, &biome, 0).unwrap();
        write_legacy_chunk_fields(
            &mut full_doc,
            Java26ChunkStatus::Full,
            Java26LegacyChunkFields {
                blending_data: loaded.blending_data(),
                upgrade_data: None,
                below_zero_retrogen: loaded.below_zero_retrogen(),
            },
        );
        let reparsed_full = Java26ChunkData::parse(&full_doc, context).unwrap();
        assert_eq!(reparsed_full.blending_data(), loaded.blending_data());
        assert!(reparsed_full.upgrade_data().is_none());
        assert!(reparsed_full.below_zero_retrogen().is_none());
    }

    #[test]
    fn chunk_writer_persists_live_inhabited_time_and_ticks() {
        let mut column = column();
        column.set_inhabited_time(42);
        column.block_ticks_mut().schedule_for_generation(
            BlockTickType::of_block_type(VANILLA_BLOCK_STONE_ID).unwrap(),
            BlockPos { x: 1, y: 2, z: 3 },
            TickPriority::High,
        );
        column.fluid_ticks_mut().schedule_for_generation(
            FluidTickType::of_fluid(VanillaFluidIdentity::Water),
            BlockPos { x: 4, y: 5, z: 6 },
            TickPriority::Normal,
        );
        let document =
            full_chunk_document(&column, 0, 0, &|_| Some("minecraft:plains".to_owned()), 0)
                .unwrap();
        let parsed = Java26ChunkData::parse(
            document,
            Java26ChunkContext {
                expected_x: 0,
                expected_z: 0,
                min_section_y: -4,
                section_count: 24,
            },
        )
        .unwrap();
        assert_eq!(parsed.inhabited_time(), 42);
        assert_eq!(parsed.block_ticks().len(), 1);
        assert_eq!(parsed.fluid_ticks().len(), 1);
    }

    #[test]
    fn full_writer_persists_each_columns_actual_final_heightmaps() {
        let mut column = column();
        let stone = default_state("minecraft:stone");
        column.set_block_state(0, 60, 0, stone).unwrap();
        column.set_block_state(1, 70, 0, stone).unwrap();

        let document =
            full_chunk_document(&column, 0, 0, &|_| Some("minecraft:plains".to_owned()), 70)
                .unwrap();
        let parsed = Java26ChunkData::parse(
            document,
            Java26ChunkContext {
                expected_x: 0,
                expected_z: 0,
                min_section_y: -4,
                section_count: 24,
            },
        )
        .unwrap();
        let restored = FeatureHeightmaps::from_saved(&column, parsed.heightmaps()).unwrap();

        for (_, _, kind, _) in FINAL_HEIGHTMAPS {
            assert_eq!(restored.first_available(kind, 0, 0), Some(61));
            assert_eq!(restored.first_available(kind, 1, 0), Some(71));
        }
    }

    #[test]
    fn final_heightmap_packing_matches_simple_bit_storage() {
        let mut values = [-64; bbb_server_types::chunk_data::feature_heightmaps::HEIGHTMAP_ENTRIES];
        values[1] = -63;
        values[2] = 320;
        let packed = pack_heightmap_values(&values, -64, 384, "test").unwrap();
        assert_eq!(packed.len(), 37);
        assert_eq!(packed[0] as u64, (1_u64 << 9) | (384_u64 << 18));
        assert_eq!((packed[36] as u64) >> 36, 0);
    }

    #[test]
    fn custom_content_registry_blocks_and_ticks_round_trip() {
        use bbb_protocol::content_registry::{ContentRegistryBuilder, ModBlockRegistration};

        let mut builder = ContentRegistryBuilder::new();
        builder.register_block(ModBlockRegistration {
            resource_id: "example_mod:neon_lamp".to_string(),
            state_count: 4,
            light_emission: 15,
            destroy_speed: 0.5,
            full_cube_collision: true,
            dynamic_light_rgb: None,
        });
        let registry = builder.freeze().unwrap();
        let mod_block = registry
            .block_by_resource_id("example_mod:neon_lamp")
            .unwrap();
        let state0 = mod_block.first_state_id;
        let state2 = mod_block.first_state_id + 2;

        let mut column = column();
        column.set_block_state(0, 64, 0, state0).unwrap();
        column.set_block_state(1, 64, 0, state2).unwrap();
        let tick_type = BlockTickType::of_block_type_with_registry(mod_block.block_id, &registry)
            .expect("custom block tick type");
        column.block_ticks_mut().schedule_for_generation(
            tick_type,
            BlockPos { x: 1, y: 64, z: 0 },
            TickPriority::Normal,
        );

        let document = full_chunk_document_with_content_registry(
            &column,
            0,
            0,
            &|_| Some("minecraft:plains".to_owned()),
            64,
            &registry,
        )
        .unwrap();
        let parsed = Java26ChunkData::parse_with_content_registry(
            &document,
            Java26ChunkContext {
                expected_x: 0,
                expected_z: 0,
                min_section_y: -4,
                section_count: 24,
            },
            &registry,
        )
        .unwrap();
        let section = parsed.sections().iter().find(|s| s.y == 4).unwrap();
        let palette = section.block_states.as_ref().unwrap();
        assert!(palette
            .palette
            .iter()
            .any(|s| s.name == "example_mod:neon_lamp" && s.properties.is_empty()));
        assert!(palette.palette.iter().any(|s| {
            s.name == "example_mod:neon_lamp"
                && s.properties
                    .get(MOD_STATE_OFFSET_PROPERTY)
                    .map(String::as_str)
                    == Some("2")
        }));
        assert_eq!(parsed.block_ticks().len(), 1);
        assert_eq!(parsed.block_ticks()[0].id, "example_mod:neon_lamp");

        // Without the custom content registry, unknown mod blocks degrade to air and ticks drop.
        let vanilla_parsed = Java26ChunkData::parse(
            &document,
            Java26ChunkContext {
                expected_x: 0,
                expected_z: 0,
                min_section_y: -4,
                section_count: 24,
            },
        )
        .unwrap();
        assert!(vanilla_parsed.block_ticks().is_empty());
    }

    fn record(x: i32, y: i32, z: i32, id: &str) -> Vec<NbtSummaryEntry> {
        vec![
            entry("x", NbtSummaryValue::Int(x)),
            entry("y", NbtSummaryValue::Int(y)),
            entry("z", NbtSummaryValue::Int(z)),
            entry("id", NbtSummaryValue::string(id)),
            entry("components", NbtSummaryValue::Compound(Vec::new())),
        ]
    }

    fn default_state(name: &str) -> i32 {
        let block = bbb_protocol::block_types::vanilla_block_type_for_resource_id(name).unwrap();
        bbb_protocol::block_state_types::vanilla_block_default_state_id(block.id).unwrap()
    }
}
