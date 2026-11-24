//! D5 structure template (`structure/*.nbt`) envelope surface.
//!
//! Vanilla anchor: `StructureTemplate.load` (`StructureTemplate.java:709-731`) reads
//! the `size`, `palette`/`palettes`, `blocks` and `entities` tags out of the gzip NBT
//! root compound; `ResourceManagerTemplateSource.load`
//! (`ResourceManagerTemplateSource.java:33-36`) decodes via `NbtIo.readCompressed`,
//! logs and drops the template on any failure. The datapack surface scans the
//! `structure/` tree eagerly at load with the same warn-only drop, winner-take-all
//! across the stacked pack view.
//!
//! Shared NBT decoding uses template budgets because Vanilla ships lists above
//! the network limit. Loading retains the gzip bytes and materializes every
//! palette and ordered block/entity record once.
//!
//! Envelope strictness vs vanilla: vanilla's `load` is lenient (missing `size`,
//! `palette` or `blocks` silently default to empty/zero, and entity entries are only
//! retained when an `nbt` compound is present). This module requires the envelope
//! fields every 26.1 built-in template actually carries — `size`, a `palette` (or
//! non-empty `palettes`) and `blocks` — so corrupt
//! pack input is attributed and dropped instead of silently becoming an empty
//! template. All 1202 vanilla built-in templates satisfy the stricter envelope
//! (covered by the in-file snapshot test).

use std::collections::BTreeMap;
use std::fmt;
use std::io::Read;
use std::sync::{Arc, OnceLock, RwLock};

use bbb_protocol::block_state_types::{
    vanilla_block_default_state_id, vanilla_block_property_values,
    vanilla_block_state_id_by_name_and_properties, vanilla_block_state_identity,
    vanilla_block_state_name, vanilla_block_state_static_collision_shape,
    vanilla_block_state_support_facts,
};
use bbb_protocol::block_types::vanilla_block_type_for_resource_id;
use bbb_protocol::content_registry::ContentRegistry;
use bbb_protocol::packets::BlockPos;
use bbb_protocol::{
    decode_file_nbt_summary_prefix, normalize_resource_location, NbtSummaryDecodeLimits,
    NbtSummaryEntry, NbtSummaryValue,
};
use bbb_server_types::block_state_parse::MOD_STATE_OFFSET_PROPERTY;
use flate2::read::GzDecoder;

mod serialization;

use super::registry_data_loader::DatapackLoadLimits;
use super::resource_overlay::{MergedServerDataView, PackOverlayInput};

/// Maximum nesting depth of compounds/lists in one template NBT document. Vanilla
/// structure NBT is flat (root → list → entry compound → payload), so 512 is far
/// beyond any legitimate template while still bounding recursive input.
pub const MAX_NBT_DEPTH: usize = 512;

/// Maximum item count of any single NBT list or array. Vanilla's largest 26.1
/// template list is the 29938-entry `blocks` list of
/// `bastion/hoglin_stable/air_base.nbt`; 1M leaves ~33x headroom.
pub const MAX_NBT_LIST_ITEMS: usize = 1_000_000;

/// Maximum decoded tags across one template NBT tree. Vanilla's largest 26.1
/// structure tree has 179661 tags; 1M leaves more than 5x headroom while bounding
/// compound-map and per-tag allocation when every field name is unique.
pub const MAX_NBT_NODES: usize = 1_000_000;

/// Maximum block records after applying the shared block list to every palette.
/// This closes the palette×blocks amplification that neither per-list limit sees.
pub const MAX_MATERIALIZED_TEMPLATE_BLOCKS: usize = 1_000_000;

/// Decompressed-byte budget for one template NBT document. Vanilla has no explicit
/// cap (`NbtIo.readCompressed` streams unbounded); the largest 26.1 template
/// decompresses to ~1.1 MiB, so 64 MiB is generous headroom while rejecting
/// gzip bombs. The compressed file is separately bounded by
/// `DatapackLoadLimits.max_element_bytes` in the loader.
pub const MAX_DECOMPRESSED_NBT_BYTES: usize = 64 * 1024 * 1024;

/// A template decode or envelope-validation failure, attributed by the loader.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StructureTemplateError {
    pub message: String,
}

impl fmt::Display for StructureTemplateError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for StructureTemplateError {}

fn error(message: impl Into<String>) -> StructureTemplateError {
    StructureTemplateError {
        message: message.into(),
    }
}

/// One decoded structure template: the validated envelope plus the raw (still
/// gzip-compressed) file bytes retained for the worldgen gameplay route.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StructureTemplate {
    pub size: [i32; 3],
    pub palette_size: usize,
    pub block_count: usize,
    pub entity_count: usize,
    // StructureTemplate.java:79-94,631-731: author is runtime metadata only.
    author: bbb_protocol::JavaString,
    raw: Arc<[u8]>,
    materialized: Arc<MaterializedStructureTemplate>,
}

/// One materialized template block in Vanilla's `Palette.blocks()` order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StructureTemplateBlock {
    pub position: BlockPos,
    pub state: i32,
    pub nbt: Option<Arc<NbtSummaryValue>>,
}

/// One materialized template palette. Palette selection remains the placement
/// owner's responsibility.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StructureTemplatePalette {
    pub blocks: Vec<StructureTemplateBlock>,
    pub jigsaws: Vec<StructureTemplateJigsaw>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StructureTemplateDirection {
    Down,
    Up,
    North,
    South,
    West,
    East,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StructureTemplateJigsawJoint {
    Rollable,
    Aligned,
}

/// The fields decoded by `StructureTemplate.JigsawBlockInfo.of`
/// (`StructureTemplate.java:783-803`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StructureTemplateJigsaw {
    pub position: BlockPos,
    pub front: StructureTemplateDirection,
    pub top: StructureTemplateDirection,
    pub joint: StructureTemplateJigsawJoint,
    pub name: String,
    pub pool: String,
    pub target: String,
    pub placement_priority: i32,
    pub selection_priority: i32,
}

/// One entity retained by `StructureTemplate.load`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StructureTemplateEntity {
    pub position_bits: [u64; 3],
    pub block_position: BlockPos,
    pub nbt: NbtSummaryValue,
}

/// The shared typed result consumed by feature and later structure-piece owners.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MaterializedStructureTemplate {
    pub size: [i32; 3],
    pub palettes: Vec<StructureTemplatePalette>,
    pub entities: Vec<StructureTemplateEntity>,
}

impl StructureTemplate {
    pub fn from_materialized(
        materialized: MaterializedStructureTemplate,
    ) -> Result<Self, StructureTemplateError> {
        Self::from_materialized_with_content_registry(materialized, ContentRegistry::vanilla())
    }

    /// Content-registry-aware form of [`Self::from_materialized`].
    pub fn from_materialized_with_content_registry(
        materialized: MaterializedStructureTemplate,
        content_registry: &ContentRegistry,
    ) -> Result<Self, StructureTemplateError> {
        let bytes =
            serialization::serialize_with_content_registry(&materialized, content_registry)?;
        parse_structure_template_with_content_registry(&bytes, content_registry)
    }

    pub fn empty() -> Self {
        let materialized = MaterializedStructureTemplate {
            size: [0; 3],
            palettes: Vec::new(),
            entities: Vec::new(),
        };
        Self {
            size: [0; 3],
            palette_size: 0,
            block_count: 0,
            entity_count: 0,
            author: "?".into(),
            raw: Arc::from(
                serialization::serialize(&materialized).expect("empty template encodes"),
            ),
            materialized: Arc::new(materialized),
        }
    }

    pub fn author(&self) -> &bbb_protocol::JavaString {
        &self.author
    }

    pub fn set_author(&mut self, author: impl Into<bbb_protocol::JavaString>) {
        self.author = author.into();
    }

    pub fn raw(&self) -> &[u8] {
        &self.raw
    }

    /// Returns the load-time, bounded, fail-closed typed template product.
    pub fn materialized(&self) -> &MaterializedStructureTemplate {
        &self.materialized
    }
}

/// `StructureTemplate.load` envelope validation and single-pass materialization over
/// the gzip NBT root compound (`StructureTemplate.java:709-731`): `size` is a 3-int list
/// of non-negative values; `palette_size` summarizes the first `palettes` entry when
/// present, otherwise `palette`; `blocks` is a list of compounds each with a 3-int `pos`
/// list and an int `state`; `entities` is an optional list of compounds each with a
/// 3-double `pos` list. Materialization retains every palette and payload.
pub fn parse_structure_template(bytes: &[u8]) -> Result<StructureTemplate, StructureTemplateError> {
    parse_structure_template_with_content_registry(bytes, ContentRegistry::vanilla())
}

/// Content-registry-aware form of [`parse_structure_template`].
pub fn parse_structure_template_with_content_registry(
    bytes: &[u8],
    content_registry: &ContentRegistry,
) -> Result<StructureTemplate, StructureTemplateError> {
    let root_tag = decode_structure_template_root(bytes)?;
    let (materialized, palette_size, block_count, entity_count) =
        materialize_structure_template(&root_tag, content_registry)?;
    let size = materialized.size;
    Ok(StructureTemplate {
        size,
        palette_size,
        block_count,
        entity_count,
        author: "?".into(),
        raw: Arc::from(bytes),
        materialized: Arc::new(materialized),
    })
}

fn materialize_structure_template(
    root_tag: &NbtSummaryValue,
    content_registry: &ContentRegistry,
) -> Result<(MaterializedStructureTemplate, usize, usize, usize), StructureTemplateError> {
    let root = compound(root_tag, "NBT root")?;
    let size = int_list(root.get("size"), "size")?;
    let size: [i32; 3] = size.try_into().map_err(|values: Vec<i32>| {
        error(format!(
            "`size` must list exactly 3 ints, got {}",
            values.len()
        ))
    })?;
    if size.iter().any(|axis| *axis < 0) {
        return Err(error(format!("`size` axes must be >= 0, got {size:?}")));
    }
    let blocks = list_items(
        root.get("blocks")
            .ok_or_else(|| error("template lacks a `blocks` list"))?,
        "blocks",
    )?;
    let palette_lists = match root.get("palettes") {
        Some(palettes) => {
            let palettes = list_items(palettes, "palettes")?;
            if palettes.is_empty() {
                return Err(error("`palettes` must not be empty"));
            }
            palettes
                .iter()
                .enumerate()
                .map(|(index, palette)| list_items(palette, &format!("palettes[{index}]")))
                .collect::<Result<Vec<_>, _>>()?
        }
        None => vec![list_items(
            root.get("palette")
                .ok_or_else(|| error("template lacks `palette` and `palettes`"))?,
            "palette",
        )?],
    };
    let palette_size = palette_lists[0].len();
    let block_count = blocks.len();
    let materialized_blocks = block_count
        .checked_mul(palette_lists.len())
        .filter(|count| *count <= MAX_MATERIALIZED_TEMPLATE_BLOCKS)
        .ok_or_else(|| {
            error(format!(
                "materialized template exceeds the {MAX_MATERIALIZED_TEMPLATE_BLOCKS} block-record budget"
            ))
        })?;
    let block_skeletons = materialize_block_skeletons(blocks)?;
    let palettes = palette_lists
        .into_iter()
        .enumerate()
        .map(|(index, palette)| {
            materialize_palette(palette, &block_skeletons, index, content_registry)
        })
        .collect::<Result<Vec<_>, _>>()?;
    debug_assert_eq!(
        palettes
            .iter()
            .map(|palette| palette.blocks.len())
            .sum::<usize>(),
        materialized_blocks
    );
    let (entities, entity_count) = materialize_entities(root.get("entities"))?;
    Ok((
        MaterializedStructureTemplate {
            size,
            palettes,
            entities,
        },
        palette_size,
        block_count,
        entity_count,
    ))
}

#[derive(Clone)]
struct StructureTemplateBlockSkeleton {
    position: BlockPos,
    state_index: Option<usize>,
    nbt: Option<Arc<NbtSummaryValue>>,
}

fn materialize_block_skeletons(
    blocks: &[NbtSummaryValue],
) -> Result<Vec<StructureTemplateBlockSkeleton>, StructureTemplateError> {
    blocks
        .iter()
        .enumerate()
        .map(|(index, block)| {
            let path = format!("blocks[{index}]");
            let block = compound(block, &path)?;
            let position = int_list(block.get("pos"), &format!("{path}.pos"))?;
            let [x, y, z] = position.as_slice() else {
                return Err(error(format!(
                    "`{path}.pos` must list exactly 3 ints, got {}",
                    position.len()
                )));
            };
            let state_index = match block.get("state") {
                Some(NbtSummaryValue::Int(value)) => usize::try_from(*value).ok(),
                _ => return Err(error(format!("{path} lacks an int `state`"))),
            };
            let nbt = match block.get("nbt") {
                Some(nbt @ NbtSummaryValue::Compound(_)) => Some(Arc::new(nbt.clone())),
                _ => None,
            };
            Ok(StructureTemplateBlockSkeleton {
                position: BlockPos {
                    x: *x,
                    y: *y,
                    z: *z,
                },
                state_index,
                nbt,
            })
        })
        .collect()
}

/// `StructureTemplate.loadPalette` resolves states and records, then
/// `addToLists`/`buildInfoList` classify and sort them (`StructureTemplate.java:141-172,733-753`).
fn materialize_palette(
    palette: &[NbtSummaryValue],
    blocks: &[StructureTemplateBlockSkeleton],
    palette_index: usize,
    content_registry: &ContentRegistry,
) -> Result<StructureTemplatePalette, StructureTemplateError> {
    let mut states = Vec::with_capacity(palette.len());
    let mut full_flags = Vec::with_capacity(palette.len());
    let mut jigsaw_states = Vec::new();
    for (index, state) in palette.iter().enumerate() {
        let state_id = template_block_state(
            state,
            &format!("palettes[{palette_index}][{index}]"),
            content_registry,
        )?;
        let is_full = template_state_is_full_block(state_id, content_registry)?;
        let state_name = vanilla_block_state_name(state_id)
            .or_else(|| {
                content_registry
                    .block_by_state_id(state_id)
                    .map(|(spec, _)| spec.resource_id.as_str())
            })
            .ok_or_else(|| error(format!("unknown materialized state {state_id}")))?;
        if state_name == "minecraft:jigsaw" && !jigsaw_states.contains(&state_id) {
            jigsaw_states.push(state_id);
        }
        states.push(state_id);
        full_flags.push(is_full);
    }
    let air = template_default_block_state("minecraft:air")?;
    let air_is_full = template_state_is_full_block(air, content_registry)?;
    let mut full_blocks = Vec::new();
    let mut block_entities = Vec::new();
    let mut other_blocks = Vec::new();
    for block in blocks {
        // `SimplePalette.stateFor` answers air for an unmapped id
        // (`StructureTemplate.java:867-870`).
        let (state, is_full) = match block.state_index.and_then(|state_index| {
            states
                .get(state_index)
                .copied()
                .zip(full_flags.get(state_index).copied())
        }) {
            Some(pair) => pair,
            None => (air, air_is_full),
        };
        let block = StructureTemplateBlock {
            position: block.position,
            state,
            nbt: block.nbt.clone(),
        };
        if block.nbt.is_some() {
            block_entities.push(block);
        } else if is_full {
            full_blocks.push(block);
        } else {
            other_blocks.push(block);
        }
    }
    let position_key =
        |block: &StructureTemplateBlock| (block.position.y, block.position.x, block.position.z);
    full_blocks.sort_by_key(position_key);
    other_blocks.sort_by_key(position_key);
    block_entities.sort_by_key(position_key);
    full_blocks.append(&mut other_blocks);
    full_blocks.append(&mut block_entities);
    let jigsaws = if jigsaw_states.is_empty() {
        Vec::new()
    } else {
        materialize_jigsaws_filtered(&full_blocks, Some(&jigsaw_states), content_registry)?
    };
    Ok(StructureTemplatePalette {
        blocks: full_blocks,
        jigsaws,
    })
}

#[cfg(test)]
fn materialize_jigsaws(
    blocks: &[StructureTemplateBlock],
) -> Result<Vec<StructureTemplateJigsaw>, StructureTemplateError> {
    materialize_jigsaws_filtered(blocks, None, ContentRegistry::vanilla())
}

fn materialize_jigsaws_filtered(
    blocks: &[StructureTemplateBlock],
    jigsaw_states: Option<&[i32]>,
    content_registry: &ContentRegistry,
) -> Result<Vec<StructureTemplateJigsaw>, StructureTemplateError> {
    let mut jigsaws = Vec::new();
    for block in blocks {
        if let Some(allowed) = jigsaw_states {
            if !allowed.contains(&block.state) {
                continue;
            }
        }
        let Some(identity) = vanilla_block_state_identity(block.state) else {
            if content_registry.block_by_state_id(block.state).is_some() {
                continue;
            }
            return Err(error(format!("unknown materialized state {}", block.state)));
        };
        if identity.name != "minecraft:jigsaw" {
            continue;
        }
        let orientation = identity
            .properties
            .get("orientation")
            .ok_or_else(|| error("jigsaw state lacks `orientation`"))?;
        let (front, top) = parse_jigsaw_orientation(orientation)?;
        let Some(NbtSummaryValue::Compound(nbt)) = block.nbt.as_deref() else {
            return Err(error(format!(
                "jigsaw at {:?} lacks compound block NBT",
                block.position
            )));
        };
        let joint = match summary_string(nbt, "joint").as_deref() {
            Some("rollable") => StructureTemplateJigsawJoint::Rollable,
            Some("aligned") => StructureTemplateJigsawJoint::Aligned,
            _ if matches!(
                front,
                StructureTemplateDirection::North
                    | StructureTemplateDirection::South
                    | StructureTemplateDirection::West
                    | StructureTemplateDirection::East
            ) =>
            {
                StructureTemplateJigsawJoint::Aligned
            }
            _ => StructureTemplateJigsawJoint::Rollable,
        };
        jigsaws.push(StructureTemplateJigsaw {
            position: block.position,
            front,
            top,
            joint,
            name: summary_resource_id(nbt, "name"),
            pool: summary_resource_id(nbt, "pool"),
            target: summary_resource_id(nbt, "target"),
            placement_priority: summary_i32(nbt, "placement_priority").unwrap_or(0),
            selection_priority: summary_i32(nbt, "selection_priority").unwrap_or(0),
        });
    }
    Ok(jigsaws)
}

fn parse_jigsaw_orientation(
    orientation: &str,
) -> Result<(StructureTemplateDirection, StructureTemplateDirection), StructureTemplateError> {
    let Some((front, top)) = orientation.split_once('_') else {
        return Err(error(format!("invalid jigsaw orientation `{orientation}`")));
    };
    Ok((
        parse_template_direction(front)?,
        parse_template_direction(top)?,
    ))
}

fn parse_template_direction(
    direction: &str,
) -> Result<StructureTemplateDirection, StructureTemplateError> {
    match direction {
        "down" => Ok(StructureTemplateDirection::Down),
        "up" => Ok(StructureTemplateDirection::Up),
        "north" => Ok(StructureTemplateDirection::North),
        "south" => Ok(StructureTemplateDirection::South),
        "west" => Ok(StructureTemplateDirection::West),
        "east" => Ok(StructureTemplateDirection::East),
        _ => Err(error(format!("invalid jigsaw direction `{direction}`"))),
    }
}

fn summary_entry<'a>(entries: &'a [NbtSummaryEntry], name: &str) -> Option<&'a NbtSummaryValue> {
    entries
        .iter()
        .rev()
        .find(|entry| entry.name == name)
        .map(|entry| &entry.value)
}

fn summary_string(entries: &[NbtSummaryEntry], name: &str) -> Option<String> {
    match summary_entry(entries, name) {
        Some(NbtSummaryValue::String(value)) => value.try_to_string().ok(),
        _ => None,
    }
}

fn summary_i32(entries: &[NbtSummaryEntry], name: &str) -> Option<i32> {
    match summary_entry(entries, name) {
        Some(NbtSummaryValue::Byte(value)) => Some(i32::from(*value)),
        Some(NbtSummaryValue::Short(value)) => Some(i32::from(*value)),
        Some(NbtSummaryValue::Int(value)) => Some(*value),
        Some(NbtSummaryValue::Long(value)) => Some(*value as i32),
        Some(NbtSummaryValue::Float(value)) => Some(f32::from_bits(*value) as i32),
        Some(NbtSummaryValue::Double(value)) => Some(f64::from_bits(*value) as i32),
        _ => None,
    }
}

fn summary_resource_id(entries: &[NbtSummaryEntry], name: &str) -> String {
    summary_string(entries, name)
        .and_then(|value| normalize_resource_location(value.to_string()).ok())
        .unwrap_or_else(|| "minecraft:empty".to_string())
}

fn materialize_entities(
    entities: Option<&NbtSummaryValue>,
) -> Result<(Vec<StructureTemplateEntity>, usize), StructureTemplateError> {
    let Some(entities) = entities else {
        return Ok((Vec::new(), 0));
    };
    let entities = list_items(entities, "entities")?;
    let entity_count = entities.len();
    let mut materialized = Vec::new();
    for (index, entity) in entities.iter().enumerate() {
        let path = format!("entities[{index}]");
        let entity = compound(entity, &path)?;
        let position = double_list(entity.get("pos"), &format!("{path}.pos"))?;
        let [x, y, z] = position.as_slice() else {
            return Err(error(format!(
                "`{path}.pos` must list exactly 3 doubles, got {}",
                position.len()
            )));
        };
        // Non-finite entity coordinates cannot become a bounded placement and
        // are rejected at this untrusted-input boundary.
        if !x.is_finite() || !y.is_finite() || !z.is_finite() {
            return Err(error(format!("`{path}.pos` must be finite")));
        }
        let block_position = entity
            .get("blockPos")
            .map(|block_position| int_list(Some(block_position), &format!("{path}.blockPos")))
            .transpose()?
            .unwrap_or_else(|| vec![0, 0, 0]);
        let [block_x, block_y, block_z] = block_position.as_slice() else {
            return Err(error(format!(
                "`{path}.blockPos` must list exactly 3 ints, got {}",
                block_position.len()
            )));
        };
        let Some(nbt @ NbtSummaryValue::Compound(_)) = entity.get("nbt") else {
            continue;
        };
        materialized.push(StructureTemplateEntity {
            position_bits: [x.to_bits(), y.to_bits(), z.to_bits()],
            block_position: BlockPos {
                x: *block_x,
                y: *block_y,
                z: *block_z,
            },
            nbt: nbt.clone(),
        });
    }
    Ok((materialized, entity_count))
}

/// `NbtUtils.readBlockState` (`NbtUtils.java:127-148`): an unknown block becomes
/// air and unknown or invalid properties leave the block's default value intact.
fn template_block_state(
    tag: &NbtSummaryValue,
    path: &str,
    content_registry: &ContentRegistry,
) -> Result<i32, StructureTemplateError> {
    let state = compound(tag, path)?;
    let raw_name = match state.get("Name") {
        Some(NbtSummaryValue::String(name)) => name,
        _ => return Err(error(format!("{path} lacks a `Name` string"))),
    };
    let Some(name) = raw_name
        .try_to_string()
        .ok()
        .and_then(|name| normalize_resource_location(name).ok())
    else {
        return template_default_block_state("minecraft:air");
    };
    let Some(block) = vanilla_block_type_for_resource_id(&name) else {
        if let Some(spec) = content_registry.block_by_resource_id(&name) {
            let mut offset = 0u16;
            if let Some(NbtSummaryValue::Compound(entries)) = state.get("Properties") {
                if let Some(NbtSummaryValue::String(raw)) =
                    summary_entry(entries, MOD_STATE_OFFSET_PROPERTY)
                {
                    if let Some(parsed) = raw
                        .try_to_string()
                        .ok()
                        .and_then(|value| value.parse::<u16>().ok())
                        .filter(|&parsed| parsed < spec.state_count)
                    {
                        offset = parsed;
                    }
                }
            }
            return Ok(spec.first_state_id + i32::from(offset));
        }
        return template_default_block_state("minecraft:air");
    };
    let default_state = vanilla_block_default_state_id(block.id)
        .ok_or_else(|| error(format!("known block `{name}` has no default state")))?;
    let mut properties = vanilla_block_state_identity(default_state)
        .ok_or_else(|| error(format!("known block `{name}` has no state identity")))?
        .properties
        .clone();
    if let Some(NbtSummaryValue::Compound(entries)) = state.get("Properties") {
        for (property, value) in &mut properties {
            let Some(NbtSummaryValue::String(raw)) = summary_entry(entries, property) else {
                continue;
            };
            if vanilla_block_property_values(&name, property)
                .is_some_and(|values| values.iter().any(|value| raw == *value))
            {
                *value = raw
                    .try_to_string()
                    .expect("matched a static block property");
            }
        }
    }
    vanilla_block_state_id_by_name_and_properties(&name, &properties).ok_or_else(|| {
        error(format!(
            "{path} does not resolve to a generated block state"
        ))
    })
}

fn template_default_block_state(name: &str) -> Result<i32, StructureTemplateError> {
    let block = vanilla_block_type_for_resource_id(name)
        .ok_or_else(|| error(format!("missing generated block `{name}`")))?;
    vanilla_block_default_state_id(block.id)
        .ok_or_else(|| error(format!("known block `{name}` has no default state")))
}

fn template_state_is_full_block(
    state: i32,
    content_registry: &ContentRegistry,
) -> Result<bool, StructureTemplateError> {
    let Some(facts) = vanilla_block_state_support_facts(state) else {
        if let Some((spec, _)) = content_registry.block_by_state_id(state) {
            return Ok(spec.full_cube_collision);
        }
        return Err(error("block state is missing generated support facts"));
    };
    if facts.dynamic_shape() {
        return Ok(false);
    }
    let shape = vanilla_block_state_static_collision_shape(state)
        .ok_or_else(|| error("non-dynamic block state lacks a generated collision shape"))?;
    let mut boxes = shape.boxes();
    let Some(first) = boxes.next() else {
        return Ok(false);
    };
    Ok(boxes.next().is_none()
        && first.min_x == 0.0
        && first.min_y == 0.0
        && first.min_z == 0.0
        && first.max_x == 1.0
        && first.max_y == 1.0
        && first.max_z == 1.0)
}

/// One loaded template, attributed to the pack that won the overlay merge.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StructureTemplateElement {
    pub id: String,
    pub source_pack: String,
    pub template: StructureTemplate,
}

/// One warn-only template loading event (`ResourceManagerTemplateSource.load` logs
/// "Couldn't load structure" and drops the element).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StructureTemplateLoadWarning {
    pub element_id: String,
    pub pack: String,
    pub message: String,
}

/// The template set for one datapack view: loaded templates in sorted id order plus
/// the warn-only drop log.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StructureTemplateSet {
    /// Templates sorted by id (`TreeMap` iteration order of the merged listing).
    pub templates: Vec<StructureTemplateElement>,
    pub warnings: Vec<StructureTemplateLoadWarning>,
}

impl StructureTemplateSet {
    pub fn template(&self, id: &str) -> Option<&StructureTemplateElement> {
        self.templates
            .binary_search_by(|template| template.id.as_str().cmp(id))
            .ok()
            .map(|index| &self.templates[index])
    }
}

fn cached_embedded_vanilla_template(
    namespace: &str,
    path: &str,
    element_id: &str,
    bytes: &[u8],
) -> Option<StructureTemplate> {
    static CACHE: OnceLock<RwLock<BTreeMap<String, StructureTemplate>>> = OnceLock::new();
    let cache = CACHE.get_or_init(|| RwLock::new(BTreeMap::new()));
    if let Some(cached) = cache
        .read()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .get(element_id)
    {
        if cached.raw.as_ref() == bytes {
            return Some(cached.clone());
        }
    }
    // Only cache genuine embedded Vanilla 26.1 templates so synthetic packs or
    // tests cannot grow the process-global cache.
    let snapshot = crate::vanilla_snapshot::VanillaDatapackSnapshot::embedded_shared().ok()?;
    let embedded_bytes =
        crate::pack::ServerPackResources::read(&snapshot.vanilla_pack(), namespace, path)?;
    if embedded_bytes.as_slice() != bytes {
        return None;
    }
    let parsed = parse_structure_template(bytes).ok()?;
    cache
        .write()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .insert(element_id.to_string(), parsed.clone());
    Some(parsed)
}

/// Winner-take-all scan of the `structure/` tree with warn-only per-template decode
/// (`ResourceManagerTemplateSource.load`), held in sorted id order. Element ids are
/// `namespace:path` from `structure/<path>.nbt` (`FileToIdConverter`).
pub fn load_structure_templates(
    view: &MergedServerDataView,
    packs: &[PackOverlayInput<'_>],
    limits: DatapackLoadLimits,
) -> StructureTemplateSet {
    load_structure_templates_with_content_registry(view, packs, limits, ContentRegistry::vanilla())
}

/// Content-registry-aware form of [`load_structure_templates`].
pub fn load_structure_templates_with_content_registry(
    view: &MergedServerDataView,
    packs: &[PackOverlayInput<'_>],
    limits: DatapackLoadLimits,
    content_registry: &ContentRegistry,
) -> StructureTemplateSet {
    let mut templates = Vec::new();
    let mut warnings = Vec::new();
    for namespace in view.namespaces() {
        for path in view.list(namespace, "structure") {
            let Some(element_path) = path
                .strip_prefix("structure/")
                .and_then(|path| path.strip_suffix(".nbt"))
            else {
                continue;
            };
            let element_id = format!("{namespace}:{element_path}");
            let Some((pack_id, resources)) = view.winner(packs, namespace, path) else {
                continue;
            };
            let Some(bytes) = resources.read(namespace, path) else {
                continue;
            };
            if bytes.len() > limits.max_element_bytes {
                warnings.push(StructureTemplateLoadWarning {
                    element_id,
                    pack: pack_id.to_string(),
                    message: format!("structure file exceeds the budget ({} bytes)", bytes.len()),
                });
                continue;
            }
            if pack_id == "vanilla" {
                if let Some(template) =
                    cached_embedded_vanilla_template(namespace, path, &element_id, &bytes)
                {
                    templates.push(StructureTemplateElement {
                        id: element_id,
                        source_pack: pack_id.to_string(),
                        template,
                    });
                    continue;
                }
            }
            match parse_structure_template_with_content_registry(&bytes, content_registry) {
                Ok(template) => templates.push(StructureTemplateElement {
                    id: element_id,
                    source_pack: pack_id.to_string(),
                    template,
                }),
                Err(decode) => warnings.push(StructureTemplateLoadWarning {
                    element_id,
                    pack: pack_id.to_string(),
                    message: format!("couldn't load structure: {decode}"),
                }),
            }
        }
    }
    templates.sort_by(|a, b| a.id.cmp(&b.id));
    StructureTemplateSet {
        templates,
        warnings,
    }
}

/// Decompress one gzip NBT document under the decompressed-byte budget
/// (`persistence/level_file.rs` pattern: `GzDecoder` + `take(budget + 1)`).
fn gunzip(bytes: &[u8]) -> Result<Vec<u8>, StructureTemplateError> {
    let mut decoder = GzDecoder::new(bytes).take(MAX_DECOMPRESSED_NBT_BYTES as u64 + 1);
    let mut decompressed = Vec::new();
    decoder
        .read_to_end(&mut decompressed)
        .map_err(|source| error(format!("gzip decode failed: {source}")))?;
    if decompressed.len() > MAX_DECOMPRESSED_NBT_BYTES {
        return Err(error(format!(
            "decompressed template exceeds the {MAX_DECOMPRESSED_NBT_BYTES} byte budget"
        )));
    }
    Ok(decompressed)
}

fn decode_structure_template_root(bytes: &[u8]) -> Result<NbtSummaryValue, StructureTemplateError> {
    let decompressed = gunzip(bytes)?;
    if decompressed.first() != Some(&10) {
        return Err(error("NBT root must be a compound"));
    }
    let limits = NbtSummaryDecodeLimits {
        max_depth: MAX_NBT_DEPTH,
        max_byte_array_items: MAX_NBT_LIST_ITEMS,
        max_int_array_items: MAX_NBT_LIST_ITEMS,
        max_long_array_items: MAX_NBT_LIST_ITEMS,
        max_list_items: MAX_NBT_LIST_ITEMS,
        max_compound_entries: MAX_NBT_NODES,
    };
    decode_file_nbt_summary_prefix(&decompressed, limits, MAX_NBT_NODES)
        .map_err(|source| error(format!("could not decode structure template: {source}")))?
        .ok_or_else(|| error("NBT root must be a compound"))
}

/// Envelope view over one decoded compound: last-value-wins key lookup, matching
/// vanilla's `CompoundTag.get` semantics for duplicate keys.
struct CompoundView<'a>(&'a [NbtSummaryEntry]);

impl CompoundView<'_> {
    fn get(&self, key: &str) -> Option<&NbtSummaryValue> {
        summary_entry(self.0, key)
    }
}

fn compound<'a>(
    tag: &'a NbtSummaryValue,
    context: &str,
) -> Result<CompoundView<'a>, StructureTemplateError> {
    match tag {
        NbtSummaryValue::Compound(entries) => Ok(CompoundView(entries)),
        _ => Err(error(format!("{context} must be a compound"))),
    }
}

fn list_items<'a>(
    tag: &'a NbtSummaryValue,
    context: &str,
) -> Result<&'a [NbtSummaryValue], StructureTemplateError> {
    match tag {
        NbtSummaryValue::List(items) => Ok(items),
        _ => Err(error(format!("`{context}` must be a list"))),
    }
}

fn int_list(
    tag: Option<&NbtSummaryValue>,
    context: &str,
) -> Result<Vec<i32>, StructureTemplateError> {
    let Some(tag) = tag else {
        return Err(error(format!("template lacks a `{context}` int list")));
    };
    let mut ints = Vec::new();
    for item in list_items(tag, context)? {
        match item {
            NbtSummaryValue::Int(value) => ints.push(*value),
            _ => return Err(error(format!("`{context}` must be a list of ints"))),
        }
    }
    Ok(ints)
}

fn double_list(
    tag: Option<&NbtSummaryValue>,
    context: &str,
) -> Result<Vec<f64>, StructureTemplateError> {
    let Some(tag) = tag else {
        return Err(error(format!("template lacks a `{context}` double list")));
    };
    let mut doubles = Vec::new();
    for item in list_items(tag, context)? {
        match item {
            NbtSummaryValue::Double(value) => doubles.push(f64::from_bits(*value)),
            _ => return Err(error(format!("`{context}` must be a list of doubles"))),
        }
    }
    Ok(doubles)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pack::ServerPackResources;
    use crate::vanilla_snapshot::VanillaDatapackSnapshot;
    use flate2::write::GzEncoder;
    use flate2::Compression;
    use std::collections::BTreeMap;
    use std::io::Write;

    const TAG_END: u8 = 0;
    const TAG_INT: u8 = 3;
    const TAG_DOUBLE: u8 = 6;
    const TAG_STRING: u8 = 8;
    const TAG_LIST: u8 = 9;
    const TAG_COMPOUND: u8 = 10;

    // --- minimal hand-rolled NBT/gzip encoders (no new dependencies) -----------

    fn gzip(bytes: &[u8]) -> Vec<u8> {
        let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(bytes).unwrap();
        encoder.finish().unwrap()
    }

    fn nbt_node_count(tag: &NbtSummaryValue) -> usize {
        1 + match tag {
            NbtSummaryValue::List(items) => items.iter().map(nbt_node_count).sum(),
            NbtSummaryValue::Compound(entries) => entries
                .iter()
                .map(|entry| nbt_node_count(&entry.value))
                .sum(),
            _ => 0,
        }
    }

    struct NbtWriter(Vec<u8>);

    impl NbtWriter {
        fn new() -> Self {
            Self(Vec::new())
        }

        fn u8(&mut self, value: u8) {
            self.0.push(value);
        }

        fn u16(&mut self, value: u16) {
            self.0.extend_from_slice(&value.to_be_bytes());
        }

        fn i32(&mut self, value: i32) {
            self.0.extend_from_slice(&value.to_be_bytes());
        }

        fn f64(&mut self, value: f64) {
            self.0.extend_from_slice(&value.to_bits().to_be_bytes());
        }

        fn name(&mut self, name: &str) {
            self.u16(name.len() as u16);
            self.0.extend_from_slice(name.as_bytes());
        }

        fn named(&mut self, tag_type: u8, name: &str) {
            self.u8(tag_type);
            self.name(name);
        }

        fn finish(self) -> Vec<u8> {
            self.0
        }
    }

    /// A tiny valid template: size [2,3,4], one-entry palette, one block with an
    /// unchecked `nbt` payload, one entity with `nbt`.
    fn tiny_template_nbt() -> Vec<u8> {
        let mut w = NbtWriter::new();
        w.u8(TAG_COMPOUND);
        w.name("");
        // size: [2, 3, 4]
        w.named(TAG_LIST, "size");
        w.u8(TAG_INT);
        w.i32(3);
        for value in [2, 3, 4] {
            w.i32(value);
        }
        // palette: [{Name: "minecraft:stone"}]
        w.named(TAG_LIST, "palette");
        w.u8(TAG_COMPOUND);
        w.i32(1);
        w.named(TAG_STRING, "Name");
        w.name("minecraft:stone");
        w.u8(TAG_END);
        // blocks: [{pos: [0, 1, 2], state: 0, nbt: {id: "minecraft:chest"}}]
        w.named(TAG_LIST, "blocks");
        w.u8(TAG_COMPOUND);
        w.i32(1);
        w.named(TAG_LIST, "pos");
        w.u8(TAG_INT);
        w.i32(3);
        for value in [0, 1, 2] {
            w.i32(value);
        }
        w.named(TAG_INT, "state");
        w.i32(0);
        w.named(TAG_COMPOUND, "nbt");
        w.named(TAG_STRING, "id");
        w.name("example:stale");
        w.named(TAG_STRING, "id");
        w.name("minecraft:chest");
        w.u8(TAG_END);
        w.u8(TAG_END);
        // entities: [{pos: [0.5, 1.0, 2.5], blockPos: [0, 1, 2], nbt: {id: "pig"}}]
        w.named(TAG_LIST, "entities");
        w.u8(TAG_COMPOUND);
        w.i32(1);
        w.named(TAG_LIST, "pos");
        w.u8(TAG_DOUBLE);
        w.i32(3);
        for value in [0.5, 1.0, 2.5] {
            w.f64(value);
        }
        w.named(TAG_LIST, "blockPos");
        w.u8(TAG_INT);
        w.i32(3);
        for value in [0, 1, 2] {
            w.i32(value);
        }
        w.named(TAG_COMPOUND, "nbt");
        w.named(TAG_STRING, "id");
        w.name("example:stale");
        w.named(TAG_STRING, "id");
        w.name("minecraft:pig");
        w.u8(TAG_END);
        w.u8(TAG_END);
        w.u8(TAG_END);
        w.finish()
    }

    fn single_block_template_nbt(
        name: &str,
        properties: &[(&str, &str)],
        state_index: i32,
    ) -> Vec<u8> {
        let mut w = NbtWriter::new();
        w.u8(TAG_COMPOUND);
        w.name("");
        w.named(TAG_LIST, "size");
        w.u8(TAG_INT);
        w.i32(3);
        for value in [1, 1, 1] {
            w.i32(value);
        }
        w.named(TAG_LIST, "palette");
        w.u8(TAG_COMPOUND);
        w.i32(1);
        w.named(TAG_STRING, "Name");
        w.name(name);
        if !properties.is_empty() {
            w.named(TAG_COMPOUND, "Properties");
            for (property, value) in properties {
                w.named(TAG_STRING, property);
                w.name(value);
            }
            w.u8(TAG_END);
        }
        w.u8(TAG_END);
        w.named(TAG_LIST, "blocks");
        w.u8(TAG_COMPOUND);
        w.i32(1);
        w.named(TAG_LIST, "pos");
        w.u8(TAG_INT);
        w.i32(3);
        for value in [0, 0, 0] {
            w.i32(value);
        }
        w.named(TAG_INT, "state");
        w.i32(state_index);
        w.u8(TAG_END);
        w.named(TAG_LIST, "entities");
        w.u8(TAG_END);
        w.i32(0);
        w.u8(TAG_END);
        w.finish()
    }

    fn cross_product_template_nbt(palette_count: i32, block_count: i32) -> Vec<u8> {
        let mut w = NbtWriter::new();
        w.u8(TAG_COMPOUND);
        w.name("");
        w.named(TAG_LIST, "size");
        w.u8(TAG_INT);
        w.i32(3);
        for value in [1, 1, 1] {
            w.i32(value);
        }
        w.named(TAG_LIST, "palettes");
        w.u8(TAG_LIST);
        w.i32(palette_count);
        for _ in 0..palette_count {
            w.u8(TAG_COMPOUND);
            w.i32(1);
            w.named(TAG_STRING, "Name");
            w.name("minecraft:stone");
            w.u8(TAG_END);
        }
        w.named(TAG_LIST, "blocks");
        w.u8(TAG_COMPOUND);
        w.i32(block_count);
        for _ in 0..block_count {
            w.named(TAG_LIST, "pos");
            w.u8(TAG_INT);
            w.i32(3);
            for value in [0, 0, 0] {
                w.i32(value);
            }
            w.named(TAG_INT, "state");
            w.i32(0);
            w.named(TAG_COMPOUND, "nbt");
            w.named(TAG_STRING, "id");
            w.name("minecraft:chest");
            w.u8(TAG_END);
            w.u8(TAG_END);
        }
        w.named(TAG_LIST, "entities");
        w.u8(TAG_END);
        w.i32(0);
        w.u8(TAG_END);
        w.finish()
    }

    #[test]
    fn parses_valid_template_envelope_and_retains_raw_bytes() {
        let mut bytes = tiny_template_nbt();
        bytes.push(99);
        let raw = gzip(&bytes);
        let template = parse_structure_template(&raw).unwrap();
        assert_eq!(template.size, [2, 3, 4]);
        assert_eq!(template.palette_size, 1);
        assert_eq!(template.block_count, 1);
        assert_eq!(template.entity_count, 1);
        assert_eq!(template.raw(), raw);
        let empty = StructureTemplate::empty();
        assert_eq!(empty.size, [0; 3]);
        assert!(empty.materialized().palettes.is_empty());
        let reloaded = parse_structure_template(empty.raw()).unwrap();
        assert_eq!(reloaded.size, [0; 3]);
        assert_eq!(reloaded.block_count, 0);
        assert_eq!(reloaded.entity_count, 0);
    }

    #[test]
    fn materializes_block_and_entity_payloads_once_in_the_template_owner() {
        let template = parse_structure_template(&gzip(&tiny_template_nbt())).unwrap();
        let materialized = template.materialized();
        assert_eq!(materialized.size, [2, 3, 4]);
        assert_eq!(materialized.palettes.len(), 1);
        let [block] = materialized.palettes[0].blocks.as_slice() else {
            panic!("tiny template does not contain one block");
        };
        assert_eq!(block.position, BlockPos { x: 0, y: 1, z: 2 });
        assert_eq!(
            vanilla_block_state_identity(block.state).unwrap().name,
            "minecraft:stone"
        );
        let Some(NbtSummaryValue::Compound(block_nbt)) = block.nbt.as_deref() else {
            panic!("block NBT was not retained");
        };
        let block_ids = block_nbt
            .iter()
            .filter(|entry| entry.name == "id")
            .collect::<Vec<_>>();
        assert_eq!(block_ids.len(), 1);
        assert_eq!(
            block_ids[0].value,
            NbtSummaryValue::string("minecraft:chest")
        );
        let [entity] = materialized.entities.as_slice() else {
            panic!("tiny template does not contain one entity");
        };
        assert_eq!(
            entity.position_bits,
            [0.5_f64.to_bits(), 1.0_f64.to_bits(), 2.5_f64.to_bits()]
        );
        assert_eq!(entity.block_position, BlockPos { x: 0, y: 1, z: 2 });
        let NbtSummaryValue::Compound(entity_nbt) = &entity.nbt else {
            panic!("entity NBT was not retained as a compound");
        };
        let entity_ids = entity_nbt
            .iter()
            .filter(|entry| entry.name == "id")
            .collect::<Vec<_>>();
        assert_eq!(entity_ids.len(), 1);
        assert_eq!(
            entity_ids[0].value,
            NbtSummaryValue::string("minecraft:pig")
        );

        let mut captured = materialized.clone();
        let payload = NbtSummaryEntry {
            name: vec![0xd800].into(),
            value: NbtSummaryValue::List(vec![
                NbtSummaryValue::string("capture\0😀"),
                NbtSummaryValue::string_from_utf16(vec![0xd801]),
                NbtSummaryValue::Int(7),
            ]),
        };
        let NbtSummaryValue::Compound(record) =
            Arc::make_mut(captured.palettes[0].blocks[0].nbt.as_mut().unwrap())
        else {
            panic!("captured block entity has a compound");
        };
        record.push(payload.clone());
        let NbtSummaryValue::Compound(record) = &mut captured.entities[0].nbt else {
            panic!("captured entity has a compound");
        };
        record.push(payload);
        let mut saved = StructureTemplate::from_materialized(captured.clone()).unwrap();
        assert_eq!(saved.materialized(), &captured);
        assert_eq!(saved.author(), "?");
        saved.set_author("Builder".to_owned());
        assert_eq!(saved.author(), "Builder");
        let reloaded = parse_structure_template(saved.raw()).unwrap();
        assert_eq!(reloaded.materialized(), &captured);
        assert_eq!(reloaded.author(), "?");
        let root = decode_structure_template_root(saved.raw()).unwrap();
        let root = compound(&root, "saved template").unwrap();
        assert!(root.get("author").is_none());
        assert_eq!(
            root.get("DataVersion"),
            Some(&NbtSummaryValue::Int(bbb_protocol::MC_DATA_VERSION))
        );
    }

    #[test]
    fn typed_jigsaws_apply_codec_defaults_joint_axis_and_numeric_priorities() {
        let jigsaw_state = |orientation: &str| {
            vanilla_block_state_id_by_name_and_properties(
                "minecraft:jigsaw",
                &BTreeMap::from([("orientation".to_string(), orientation.to_string())]),
            )
            .unwrap()
        };
        let blocks = vec![
            StructureTemplateBlock {
                position: BlockPos { x: 1, y: 2, z: 3 },
                state: jigsaw_state("north_up"),
                nbt: Some(Arc::new(NbtSummaryValue::Compound(Vec::new()))),
            },
            StructureTemplateBlock {
                position: BlockPos { x: 4, y: 5, z: 6 },
                state: jigsaw_state("up_north"),
                nbt: Some(Arc::new(NbtSummaryValue::Compound(vec![
                    NbtSummaryEntry {
                        name: "name".into(),
                        value: NbtSummaryValue::string("not a resource id"),
                    },
                    NbtSummaryEntry {
                        name: "pool".into(),
                        value: NbtSummaryValue::string("example:pool"),
                    },
                    NbtSummaryEntry {
                        name: "target".into(),
                        value: NbtSummaryValue::string("door"),
                    },
                    NbtSummaryEntry {
                        name: "joint".into(),
                        value: NbtSummaryValue::string("not-a-joint"),
                    },
                    NbtSummaryEntry {
                        name: "placement_priority".into(),
                        value: NbtSummaryValue::Byte(-7),
                    },
                    NbtSummaryEntry {
                        name: "selection_priority".into(),
                        value: NbtSummaryValue::Long(4_294_967_305),
                    },
                ]))),
            },
        ];
        let jigsaws = materialize_jigsaws(&blocks).unwrap();
        assert_eq!(jigsaws[0].joint, StructureTemplateJigsawJoint::Aligned);
        assert_eq!(jigsaws[0].name, "minecraft:empty");
        assert_eq!(jigsaws[0].pool, "minecraft:empty");
        assert_eq!(jigsaws[0].target, "minecraft:empty");
        assert_eq!(jigsaws[1].joint, StructureTemplateJigsawJoint::Rollable);
        assert_eq!(jigsaws[1].name, "minecraft:empty");
        assert_eq!(jigsaws[1].pool, "example:pool");
        assert_eq!(jigsaws[1].target, "minecraft:door");
        assert_eq!(jigsaws[1].placement_priority, -7);
        assert_eq!(jigsaws[1].selection_priority, 9);
    }

    #[test]
    fn materialization_keeps_nbt_utils_block_state_defaults() {
        let unknown = parse_structure_template(&gzip(&single_block_template_nbt(
            "example:not_a_block",
            &[],
            0,
        )))
        .unwrap();
        assert_eq!(
            unknown.materialized().palettes[0].blocks[0].state,
            template_default_block_state("minecraft:air").unwrap()
        );

        let invalid_property = parse_structure_template(&gzip(&single_block_template_nbt(
            "minecraft:redstone_torch",
            &[("lit", "not-a-value"), ("unknown", "ignored")],
            0,
        )))
        .unwrap();
        assert_eq!(
            invalid_property.materialized().palettes[0].blocks[0].state,
            template_default_block_state("minecraft:redstone_torch").unwrap()
        );

        let missing_palette_index = parse_structure_template(&gzip(&single_block_template_nbt(
            "minecraft:stone",
            &[],
            99,
        )))
        .unwrap();
        assert_eq!(
            missing_palette_index.materialized().palettes[0].blocks[0].state,
            template_default_block_state("minecraft:air").unwrap()
        );
    }

    #[test]
    fn palettes_share_one_materialized_block_nbt_payload() {
        let template = parse_structure_template(&gzip(&cross_product_template_nbt(2, 1))).unwrap();
        let materialized = template.materialized();
        let first = materialized.palettes[0].blocks[0].nbt.as_ref().unwrap();
        let second = materialized.palettes[1].blocks[0].nbt.as_ref().unwrap();
        assert!(Arc::ptr_eq(first, second));
        let mut materialized = materialized.clone();
        materialized.palettes[1].blocks[0].state = vanilla_block_state_id_by_name_and_properties(
            "minecraft:oak_log",
            &BTreeMap::from([("axis".to_owned(), "x".to_owned())]),
        )
        .unwrap();
        let saved = StructureTemplate::from_materialized(materialized.clone()).unwrap();
        assert_eq!(saved.materialized(), &materialized);
        let reloaded = parse_structure_template(saved.raw()).unwrap();
        assert_eq!(reloaded.materialized(), &materialized);
        assert!(Arc::ptr_eq(
            reloaded.materialized().palettes[0].blocks[0]
                .nbt
                .as_ref()
                .unwrap(),
            reloaded.materialized().palettes[1].blocks[0]
                .nbt
                .as_ref()
                .unwrap(),
        ));
    }

    #[test]
    fn palettes_first_entry_wins_over_palette() {
        // `palettes` present (one list of two entries): its first entry is used.
        let mut w = NbtWriter::new();
        w.u8(TAG_COMPOUND);
        w.name("");
        w.named(TAG_LIST, "size");
        w.u8(TAG_INT);
        w.i32(3);
        for value in [1, 1, 1] {
            w.i32(value);
        }
        w.named(TAG_LIST, "palettes");
        w.u8(TAG_LIST);
        w.i32(1);
        w.u8(TAG_COMPOUND);
        w.i32(2);
        for name in ["minecraft:air", "minecraft:stone"] {
            w.named(TAG_STRING, "Name");
            w.name(name);
            w.u8(TAG_END);
        }
        w.named(TAG_LIST, "blocks");
        w.u8(TAG_END);
        w.i32(0);
        w.u8(TAG_END);
        let template = parse_structure_template(&gzip(&w.finish())).unwrap();
        assert_eq!(template.palette_size, 2);
        assert_eq!(template.block_count, 0);
        assert_eq!(template.entity_count, 0);
    }

    #[test]
    fn parses_every_vanilla_26_1_structure_template() {
        let snapshot = VanillaDatapackSnapshot::embedded_shared().unwrap();
        let vanilla = snapshot.vanilla_pack();
        let paths = vanilla.list_paths("minecraft", "structure");
        assert_eq!(paths.len(), 1202);
        let mut total_blocks = 0usize;
        let mut materialized_palettes = 0usize;
        let mut materialized_blocks = 0usize;
        let mut materialized_entities = 0usize;
        let mut jigsaw_blocks = 0usize;
        let mut typed_jigsaws = 0usize;
        let mut first_multi_palette = None;
        let mut largest_materialized_template = None;
        let mut largest_nbt_tree = None;
        for path in &paths {
            let bytes = vanilla.read("minecraft", path).unwrap();
            let template = parse_structure_template(&bytes)
                .unwrap_or_else(|e| panic!("{path} must parse: {e}"));
            let node_count = nbt_node_count(&decode_structure_template_root(&bytes).unwrap());
            if largest_nbt_tree
                .as_ref()
                .is_none_or(|(_, largest)| node_count > *largest)
            {
                largest_nbt_tree = Some((path.clone(), node_count));
            }
            assert_eq!(template.raw(), bytes);
            total_blocks += template.block_count;
            let materialized = template.materialized();
            assert_eq!(materialized.size, template.size);
            assert!(materialized
                .palettes
                .iter()
                .all(|palette| palette.blocks.len() == template.block_count));
            if materialized.palettes.len() > 1 && first_multi_palette.is_none() {
                first_multi_palette = Some((path.clone(), materialized.palettes.len()));
            }
            materialized_palettes += materialized.palettes.len();
            let template_materialized_blocks = materialized
                .palettes
                .iter()
                .map(|palette| palette.blocks.len())
                .sum::<usize>();
            materialized_blocks += template_materialized_blocks;
            if largest_materialized_template
                .as_ref()
                .is_none_or(|(_, largest)| template_materialized_blocks > *largest)
            {
                largest_materialized_template = Some((path.clone(), template_materialized_blocks));
            }
            materialized_entities += materialized.entities.len();
            typed_jigsaws += materialized
                .palettes
                .iter()
                .map(|palette| palette.jigsaws.len())
                .sum::<usize>();
            for block in materialized
                .palettes
                .iter()
                .flat_map(|palette| &palette.blocks)
                .filter(|block| {
                    vanilla_block_state_identity(block.state)
                        .is_some_and(|identity| identity.name == "minecraft:jigsaw")
                })
            {
                jigsaw_blocks += 1;
                let Some(NbtSummaryValue::Compound(nbt)) = block.nbt.as_deref() else {
                    panic!("{path} jigsaw block lacks compound NBT");
                };
                for field in ["name", "pool", "target"] {
                    assert!(
                        nbt.iter().any(|entry| entry.name == field),
                        "{path} jigsaw block lacks {field}"
                    );
                }
            }
        }
        assert_eq!(materialized_palettes, 1_342);
        assert_eq!(materialized_blocks, 1_122_687);
        assert_eq!(materialized_entities, 64);
        assert_eq!(jigsaw_blocks, 3_759);
        assert_eq!(typed_jigsaws, jigsaw_blocks);
        assert_eq!(
            largest_materialized_template,
            Some((
                "structure/bastion/hoglin_stable/air_base.nbt".to_string(),
                29_938
            ))
        );
        assert_eq!(
            largest_nbt_tree,
            Some((
                "structure/bastion/hoglin_stable/air_base.nbt".to_string(),
                179_661
            ))
        );
        assert_eq!(
            first_multi_palette,
            Some((
                "structure/shipwreck/rightsideup_backhalf.nbt".to_string(),
                8
            ))
        );
        // Vanilla's largest template ships a 29938-entry blocks list
        // (`bastion/hoglin_stable/air_base.nbt`): the bounded reader must accept it.
        let air_base = vanilla
            .read("minecraft", "structure/bastion/hoglin_stable/air_base.nbt")
            .unwrap();
        assert_eq!(
            parse_structure_template(&air_base).unwrap().block_count,
            29938
        );
        assert!(total_blocks > 29938);
    }
}
