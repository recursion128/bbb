use std::{collections::BTreeMap, fmt, ops::Deref, sync::Arc};

use bbb_protocol::{
    block_state_types::{
        vanilla_block_state_identity, vanilla_block_state_render_facts,
        vanilla_block_state_support_facts, VanillaBlockStateRenderFacts,
    },
    content_registry::ContentRegistry,
};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

static NO_BLOCK_PROPERTIES: BTreeMap<String, String> = BTreeMap::new();

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TerrainMaterialClass {
    Empty,
    Invisible,
    Opaque,
    Cutout,
    Fluid,
    Translucent,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TerrainFluidKind {
    Water,
    Lava,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct TerrainFluidState {
    pub kind: TerrainFluidKind,
    pub amount: u8,
    pub falling: bool,
}

impl TerrainFluidState {
    pub fn new(kind: TerrainFluidKind, amount: u8, falling: bool) -> Self {
        Self {
            kind,
            amount: amount.clamp(1, 8),
            falling,
        }
    }

    pub fn own_height(self) -> f64 {
        f64::from(self.amount) / 9.0
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TerrainSkipRendering {
    pub same_block_key: u64,
    pub same_block_culls_all_faces: bool,
    #[serde(default)]
    pub same_block_culls_vertical_faces: bool,
    #[serde(default)]
    pub leaves_block: bool,
    pub iron_bars_block: bool,
    pub bars_tag: bool,
    pub north: bool,
    pub south: bool,
    pub west: bool,
    pub east: bool,
    #[serde(default)]
    pub connection_property_mask: u8,
}

impl TerrainSkipRendering {
    pub const NONE: Self = Self {
        same_block_key: 0,
        same_block_culls_all_faces: false,
        same_block_culls_vertical_faces: false,
        leaves_block: false,
        iron_bars_block: false,
        bars_tag: false,
        north: false,
        south: false,
        west: false,
        east: false,
        connection_property_mask: 0,
    };
}

pub fn terrain_skip_rendering_from_vanilla_facts(
    facts: VanillaBlockStateRenderFacts,
    bars_tag: bool,
) -> TerrainSkipRendering {
    TerrainSkipRendering {
        same_block_key: facts.block_identity(),
        same_block_culls_all_faces: facts.same_block_culls_all_faces(),
        same_block_culls_vertical_faces: facts.same_block_culls_vertical_faces(),
        leaves_block: facts.leaves_block(),
        iron_bars_block: facts.iron_bars_block(),
        bars_tag,
        north: facts.north(),
        south: facts.south(),
        west: facts.west(),
        east: facts.east(),
        connection_property_mask: facts.connection_property_mask(),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct TerrainLight {
    pub sky: u8,
    pub block: u8,
}

impl TerrainLight {
    pub const FULL_BRIGHT: Self = Self { sky: 15, block: 0 };
    pub const DARK: Self = Self { sky: 0, block: 0 };

    pub fn clamp(self) -> Self {
        Self {
            sky: self.sky.min(15),
            block: self.block.min(15),
        }
    }
}

/// Immutable block-state facts the terrain renderer reads per block state id.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TerrainBlockRenderState<'a> {
    pub block_state_id: i32,
    pub block_name: Option<&'a str>,
    pub block_properties: &'static BTreeMap<String, String>,
    pub material: TerrainMaterialClass,
    pub solid_render: bool,
    pub legacy_solid: bool,
    pub blocks_motion: bool,
    pub can_occlude: bool,
    pub view_blocking: bool,
    pub light_dampening: u8,
    pub shade_brightness: u8,
    pub skip_rendering: TerrainSkipRendering,
    pub face_occlusion_shape_indices: [u8; 6],
    pub fluid_overlay_neighbor: bool,
    pub state_light_emission: u8,
    pub emissive_rendering: bool,
    pub fluid: Option<TerrainFluidState>,
}

pub type TerrainBaseStateFactsFn =
    fn(i32) -> Option<(TerrainMaterialClass, Option<TerrainFluidState>)>;

/// The synchronized-tag and modded-content inputs of [`TerrainBlockRenderState`]:
/// the session's `minecraft:bars` block tag, the session [`ContentRegistry`], and
/// the client world's base material/fluid classifier.
#[derive(Debug, Clone)]
pub struct TerrainBlockRenderContext {
    pub bars_block_types: Arc<[i32]>,
    base_state_facts: TerrainBaseStateFactsFn,
    content_registry: Arc<ContentRegistry>,
}

impl PartialEq for TerrainBlockRenderContext {
    fn eq(&self, other: &Self) -> bool {
        (Arc::ptr_eq(&self.bars_block_types, &other.bars_block_types)
            || self.bars_block_types == other.bars_block_types)
            && self.base_state_facts as usize == other.base_state_facts as usize
            && (Arc::ptr_eq(&self.content_registry, &other.content_registry)
                || self.content_registry.blocks() == other.content_registry.blocks())
    }
}

impl Eq for TerrainBlockRenderContext {}

impl TerrainBlockRenderContext {
    pub fn new(
        bars_block_types: impl Into<Arc<[i32]>>,
        base_state_facts: TerrainBaseStateFactsFn,
    ) -> Self {
        Self {
            bars_block_types: bars_block_types.into(),
            base_state_facts,
            content_registry: Arc::clone(ContentRegistry::vanilla()),
        }
    }

    pub fn with_content_registry(mut self, content_registry: Arc<ContentRegistry>) -> Self {
        self.content_registry = content_registry;
        self
    }

    pub fn content_registry(&self) -> &Arc<ContentRegistry> {
        &self.content_registry
    }

    pub fn total_block_state_count(&self) -> i32 {
        self.content_registry.total_block_state_count()
    }

    pub fn block_state(&self, block_state_id: i32) -> TerrainBlockRenderState<'_> {
        self.block_state_with_registry(block_state_id, &self.content_registry)
    }

    pub fn block_state_with_registry<'a>(
        &self,
        block_state_id: i32,
        content_registry: &'a ContentRegistry,
    ) -> TerrainBlockRenderState<'a> {
        if let Some(info) = vanilla_block_state_identity(block_state_id) {
            let (material, fluid) = (self.base_state_facts)(block_state_id)
                .unwrap_or((TerrainMaterialClass::Opaque, None));
            let support_facts = vanilla_block_state_support_facts(block_state_id);
            let render_facts = vanilla_block_state_render_facts(block_state_id);
            let block_name = Some(info.name.as_str());
            // Vanilla `BlockStateBase.isSolid()` / `blocksMotion()` (`BlockBehaviour.java:540-548`).
            let legacy_solid = support_facts
                .map_or(matches!(material, TerrainMaterialClass::Opaque), |facts| {
                    facts.legacy_solid()
                });
            let blocks_motion = legacy_solid
                && !matches!(
                    block_name,
                    Some("minecraft:cobweb" | "minecraft:bamboo_sapling")
                );
            return TerrainBlockRenderState {
                block_state_id,
                material,
                solid_render: support_facts.is_some_and(|facts| facts.solid_render()),
                legacy_solid,
                blocks_motion,
                can_occlude: support_facts.is_some_and(|facts| facts.can_occlude()),
                view_blocking: support_facts.is_some_and(|facts| facts.view_blocking()),
                light_dampening: support_facts.map_or(0, |facts| facts.light_dampening()),
                shade_brightness: support_facts.map_or(255, |facts| facts.shade_brightness()),
                skip_rendering: render_facts
                    .map(|facts| {
                        terrain_skip_rendering_from_vanilla_facts(
                            facts,
                            self.bars_block_types.contains(&facts.block_type_id()),
                        )
                    })
                    .unwrap_or_default(),
                face_occlusion_shape_indices: render_facts
                    .map_or([0; 6], |facts| facts.face_occlusion_shape_indices()),
                fluid_overlay_neighbor: render_facts
                    .is_some_and(|facts| facts.fluid_water_overlay_neighbor()),
                state_light_emission: render_facts.map_or(0, |facts| facts.light_emission()),
                emissive_rendering: render_facts.is_some_and(|facts| facts.emissive_rendering()),
                fluid,
                block_name,
                block_properties: &info.properties,
            };
        }

        if let Some((spec, _state_offset)) = content_registry.block_by_state_id(block_state_id) {
            let full_cube = spec.full_cube_collision;
            let default_material = if full_cube {
                TerrainMaterialClass::Opaque
            } else {
                TerrainMaterialClass::Cutout
            };
            let (material, fluid) =
                (self.base_state_facts)(block_state_id).unwrap_or((default_material, None));
            let solid_cube = full_cube && matches!(material, TerrainMaterialClass::Opaque);
            return TerrainBlockRenderState {
                block_state_id,
                block_name: Some(spec.resource_id()),
                block_properties: &NO_BLOCK_PROPERTIES,
                material,
                solid_render: solid_cube,
                legacy_solid: full_cube,
                blocks_motion: full_cube,
                can_occlude: solid_cube,
                view_blocking: solid_cube,
                // Matches `light_facts_with_registry` and Vanilla `BlockBehaviour.getShadeBrightness`
                // (`BlockBehaviour.java:321-323`): 0.2 (51/255) for full cubes, 1.0 (255/255) otherwise.
                light_dampening: if full_cube { 15 } else { 0 },
                shade_brightness: if full_cube { 51 } else { 255 },
                skip_rendering: TerrainSkipRendering {
                    same_block_key: u64::try_from(spec.block_id).unwrap_or(0).saturating_add(1),
                    bars_tag: self.bars_block_types.contains(&spec.block_id),
                    ..TerrainSkipRendering::NONE
                },
                face_occlusion_shape_indices: if solid_cube { [1; 6] } else { [0; 6] },
                fluid_overlay_neighbor: false,
                state_light_emission: spec.light_emission.min(15),
                emissive_rendering: false,
                fluid,
            };
        }

        let (material, fluid) =
            (self.base_state_facts)(block_state_id).unwrap_or((TerrainMaterialClass::Opaque, None));
        let legacy_solid = matches!(material, TerrainMaterialClass::Opaque);
        TerrainBlockRenderState {
            block_state_id,
            material,
            solid_render: false,
            legacy_solid,
            blocks_motion: legacy_solid,
            can_occlude: false,
            view_blocking: false,
            light_dampening: 0,
            shade_brightness: 255,
            skip_rendering: TerrainSkipRendering::NONE,
            face_occlusion_shape_indices: [0; 6],
            fluid_overlay_neighbor: false,
            state_light_emission: 0,
            emissive_rendering: false,
            fluid,
            block_name: None,
            block_properties: &NO_BLOCK_PROPERTIES,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PaletteDomain {
    BlockStates,
    Biomes,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PaletteKind {
    SingleValue,
    Local,
    Global,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PalettedContainerData {
    pub domain: PaletteDomain,
    pub bits_per_entry: u8,
    pub palette_kind: PaletteKind,
    pub palette_global_ids: Vec<i32>,
    pub packed_data: Vec<i64>,
    pub entry_count: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct PaletteValue {
    pub global_id: i32,
    pub palette_index: Option<usize>,
}

impl PalettedContainerData {
    pub fn from_block_state_ids(ids: &[u32; 4096]) -> Self {
        let first = ids[0];
        if ids.iter().all(|&id| id == first) {
            return Self {
                domain: PaletteDomain::BlockStates,
                bits_per_entry: 0,
                palette_kind: PaletteKind::SingleValue,
                palette_global_ids: vec![first as i32],
                packed_data: Vec::new(),
                entry_count: 4096,
            };
        }
        let mut palette = Vec::with_capacity(16);
        let mut indices = [0u64; 4096];
        for (slot, &id) in indices.iter_mut().zip(ids.iter()) {
            let signed = id as i32;
            let idx = match palette.iter().position(|&existing| existing == signed) {
                Some(idx) => idx,
                None => {
                    let idx = palette.len();
                    palette.push(signed);
                    idx
                }
            };
            *slot = idx as u64;
        }
        let max_index = (palette.len() - 1) as u64;
        let bits = bits_needed(max_index);
        if palette_kind(PaletteDomain::BlockStates, bits) == PaletteKind::Global {
            let max_id = ids.iter().copied().max().unwrap_or(0) as u64;
            let global_bits = bits_needed(max_id);
            let values: Vec<u64> = ids.iter().map(|&id| u64::from(id)).collect();
            return Self {
                domain: PaletteDomain::BlockStates,
                bits_per_entry: global_bits,
                palette_kind: PaletteKind::Global,
                palette_global_ids: Vec::new(),
                packed_data: pack_values_to_longs(&values, global_bits as usize),
                entry_count: 4096,
            };
        }
        Self {
            domain: PaletteDomain::BlockStates,
            bits_per_entry: bits,
            palette_kind: PaletteKind::Local,
            palette_global_ids: palette,
            packed_data: pack_values_to_longs(&indices, bits as usize),
            entry_count: 4096,
        }
    }

    #[inline]
    pub fn global_id_at(&self, index: usize) -> Option<i32> {
        if index >= self.entry_count {
            return None;
        }
        match self.palette_kind {
            PaletteKind::SingleValue => self.palette_global_ids.first().copied(),
            PaletteKind::Local => {
                let palette_index =
                    read_packed_value(&self.packed_data, self.bits_per_entry, index)? as usize;
                self.palette_global_ids.get(palette_index).copied()
            }
            PaletteKind::Global => {
                Some(read_packed_value(&self.packed_data, self.bits_per_entry, index)? as i32)
            }
        }
    }

    pub fn unpack_u32_into(&self, out: &mut [u32]) {
        let len = out.len().min(self.entry_count);
        match self.palette_kind {
            PaletteKind::SingleValue => {
                let id = self.palette_global_ids.first().copied().unwrap_or(0) as u32;
                out[..len].fill(id);
            }
            PaletteKind::Local => {
                let bits = self.bits_per_entry as usize;
                if bits == 0 || bits > 64 {
                    out[..len].fill(0);
                    return;
                }
                let values_per_long = 64 / bits;
                let mask = if bits == 64 {
                    u64::MAX
                } else {
                    (1u64 << bits) - 1
                };
                let palette = self.palette_global_ids.as_slice();
                let mut cursor = 0;
                for &word in &self.packed_data {
                    let mut raw = word as u64;
                    let count = values_per_long.min(len - cursor);
                    for slot in &mut out[cursor..cursor + count] {
                        let idx = (raw & mask) as usize;
                        *slot = palette.get(idx).copied().unwrap_or(0) as u32;
                        raw >>= bits;
                    }
                    cursor += count;
                    if cursor >= len {
                        break;
                    }
                }
                if cursor < len {
                    out[cursor..len].fill(0);
                }
            }
            PaletteKind::Global => {
                let bits = self.bits_per_entry as usize;
                if bits == 0 || bits > 64 {
                    out[..len].fill(0);
                    return;
                }
                let values_per_long = 64 / bits;
                let mask = if bits == 64 {
                    u64::MAX
                } else {
                    (1u64 << bits) - 1
                };
                let mut cursor = 0;
                for &word in &self.packed_data {
                    let mut raw = word as u64;
                    let count = values_per_long.min(len - cursor);
                    for slot in &mut out[cursor..cursor + count] {
                        *slot = (raw & mask) as u32;
                        raw >>= bits;
                    }
                    cursor += count;
                    if cursor >= len {
                        break;
                    }
                }
                if cursor < len {
                    out[cursor..len].fill(0);
                }
            }
        }
    }

    pub fn value_at(&self, index: usize) -> Option<PaletteValue> {
        if index >= self.entry_count {
            return None;
        }

        match self.palette_kind {
            PaletteKind::SingleValue => Some(PaletteValue {
                global_id: self.palette_global_ids.first().copied()?,
                palette_index: Some(0),
            }),
            PaletteKind::Local => {
                let palette_index =
                    read_packed_value(&self.packed_data, self.bits_per_entry, index)?;
                let palette_index = usize::try_from(palette_index).ok()?;
                Some(PaletteValue {
                    global_id: *self.palette_global_ids.get(palette_index)?,
                    palette_index: Some(palette_index),
                })
            }
            PaletteKind::Global => Some(PaletteValue {
                global_id: read_packed_value(&self.packed_data, self.bits_per_entry, index)? as i32,
                palette_index: None,
            }),
        }
    }

    pub fn set_value_at(&mut self, index: usize, global_id: i32) -> bool {
        if index >= self.entry_count || global_id < 0 {
            return false;
        }

        match self.palette_kind {
            PaletteKind::SingleValue => {
                if self.palette_global_ids.first().copied() == Some(global_id) {
                    return true;
                }
                let Some(previous_id) = self.palette_global_ids.first().copied() else {
                    return false;
                };
                self.bits_per_entry = 1;
                self.palette_kind = PaletteKind::Local;
                self.palette_global_ids = vec![previous_id, global_id];
                self.packed_data = vec![0; packed_long_len(self.entry_count, 1)];
                self.packed_data[index / 64] |= 1i64 << (index % 64);
                true
            }
            PaletteKind::Local => {
                if let Some(palette_index) = self
                    .palette_global_ids
                    .iter()
                    .position(|id| *id == global_id)
                {
                    set_packed_value(
                        &mut self.packed_data,
                        self.bits_per_entry,
                        index,
                        palette_index as u64,
                    )
                } else {
                    let palette_index = self.palette_global_ids.len() as u64;
                    let bits = self.bits_per_entry.max(bits_needed(palette_index));
                    if palette_kind(self.domain, bits) == PaletteKind::Global {
                        return self.upgrade_to_global_with(index, global_id);
                    }
                    if bits > self.bits_per_entry {
                        let mut values = Vec::with_capacity(self.entry_count);
                        for entry_index in 0..self.entry_count {
                            let Some(value) = read_packed_value(
                                &self.packed_data,
                                self.bits_per_entry,
                                entry_index,
                            ) else {
                                return false;
                            };
                            values.push(value);
                        }
                        values[index] = palette_index;
                        self.packed_data = pack_values_to_longs(&values, bits as usize);
                        self.bits_per_entry = bits;
                    } else if !set_packed_value(&mut self.packed_data, bits, index, palette_index) {
                        return false;
                    }
                    self.palette_global_ids.push(global_id);
                    true
                }
            }
            PaletteKind::Global => {
                if set_packed_value(
                    &mut self.packed_data,
                    self.bits_per_entry,
                    index,
                    global_id as u64,
                ) {
                    true
                } else {
                    self.upgrade_to_global_with(index, global_id)
                }
            }
        }
    }

    fn upgrade_to_global_with(&mut self, index: usize, global_id: i32) -> bool {
        let mut values = Vec::with_capacity(self.entry_count);
        let mut max_value = global_id as u64;
        for entry_index in 0..self.entry_count {
            let Some(value) = self.value_at(entry_index) else {
                return false;
            };
            let Ok(global_id) = u64::try_from(value.global_id) else {
                return false;
            };
            max_value = max_value.max(global_id);
            values.push(global_id);
        }

        values[index] = global_id as u64;
        self.bits_per_entry = bits_needed(max_value);
        self.palette_kind = PaletteKind::Global;
        self.palette_global_ids.clear();
        self.packed_data = pack_values_to_longs(&values, self.bits_per_entry as usize);
        true
    }
}

pub fn palette_kind(domain: PaletteDomain, bits_per_entry: u8) -> PaletteKind {
    match (domain, bits_per_entry) {
        (_, 0) => PaletteKind::SingleValue,
        (PaletteDomain::BlockStates, 1..=8) => PaletteKind::Local,
        (PaletteDomain::Biomes, 1..=3) => PaletteKind::Local,
        _ => PaletteKind::Global,
    }
}

pub fn packed_long_len(entry_count: usize, bits_per_entry: usize) -> usize {
    let Some(values_per_long) = 64_usize.checked_div(bits_per_entry) else {
        return 0;
    };
    entry_count.div_ceil(values_per_long)
}

fn read_packed_value(packed_data: &[i64], bits_per_entry: u8, index: usize) -> Option<u64> {
    if bits_per_entry == 0 || bits_per_entry > 64 {
        return None;
    }

    let bits = bits_per_entry as usize;
    let values_per_long = 64 / bits;
    if values_per_long == 0 {
        return None;
    }

    let cell_index = index / values_per_long;
    let bit_index = (index - cell_index * values_per_long) * bits;
    let cell = *packed_data.get(cell_index)? as u64;
    let mask = if bits == 64 {
        u64::MAX
    } else {
        (1u64 << bits) - 1
    };
    Some((cell >> bit_index) & mask)
}

fn set_packed_value(packed_data: &mut [i64], bits_per_entry: u8, index: usize, value: u64) -> bool {
    if bits_per_entry == 0 || bits_per_entry > 64 {
        return false;
    }

    let bits = bits_per_entry as usize;
    let values_per_long = 64 / bits;
    if values_per_long == 0 {
        return false;
    }

    let cell_index = index / values_per_long;
    let bit_index = (index - cell_index * values_per_long) * bits;
    let Some(cell) = packed_data.get_mut(cell_index) else {
        return false;
    };
    let mask = if bits == 64 {
        u64::MAX
    } else {
        (1u64 << bits) - 1
    };
    if value & !mask != 0 {
        return false;
    }

    let raw = *cell as u64;
    *cell = ((raw & !(mask << bit_index)) | (value << bit_index)) as i64;
    true
}

fn pack_values_to_longs(values: &[u64], bits_per_entry: usize) -> Vec<i64> {
    if bits_per_entry == 0 {
        return Vec::new();
    }

    let values_per_long = 64 / bits_per_entry;
    if values_per_long == 0 {
        return Vec::new();
    }

    let mut packed = vec![0u64; values.len().div_ceil(values_per_long)];
    let mask = if bits_per_entry == 64 {
        u64::MAX
    } else {
        (1u64 << bits_per_entry) - 1
    };
    for (index, value) in values.iter().copied().enumerate() {
        let cell_index = index / values_per_long;
        let bit_index = (index - cell_index * values_per_long) * bits_per_entry;
        packed[cell_index] |= (value & mask) << bit_index;
    }

    packed.into_iter().map(|value| value as i64).collect()
}

fn bits_needed(max_value: u64) -> u8 {
    (u64::BITS - max_value.leading_zeros()).max(1) as u8
}

pub const LIGHT_ARRAY_BYTES: usize = 2048;

/// Reference-counted 2,048-byte section nibble array shared across COW column clones and meshers.
#[derive(Clone, PartialEq, Eq)]
pub struct PackedLightArray(pub Arc<[u8; LIGHT_ARRAY_BYTES]>);

impl fmt::Debug for PackedLightArray {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("PackedLightArray")
            .field(&(&self.0[..]))
            .finish()
    }
}

impl Deref for PackedLightArray {
    type Target = [u8; LIGHT_ARRAY_BYTES];

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl AsRef<[u8]> for PackedLightArray {
    #[inline]
    fn as_ref(&self) -> &[u8] {
        &self.0[..]
    }
}

impl From<[u8; LIGHT_ARRAY_BYTES]> for PackedLightArray {
    #[inline]
    fn from(bytes: [u8; LIGHT_ARRAY_BYTES]) -> Self {
        Self(Arc::new(bytes))
    }
}

impl PackedLightArray {
    #[inline]
    pub fn from_boxed(bytes: Box<[u8; LIGHT_ARRAY_BYTES]>) -> Self {
        Self(Arc::new(*bytes))
    }

    #[inline]
    pub fn from_slice_exact(bytes: &[u8]) -> Option<Self> {
        let array: [u8; LIGHT_ARRAY_BYTES] = bytes.try_into().ok()?;
        Some(Self(Arc::new(array)))
    }
}

impl Serialize for PackedLightArray {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        <[u8]>::serialize(&self.0[..], serializer)
    }
}

impl<'de> Deserialize<'de> for PackedLightArray {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let bytes = Vec::<u8>::deserialize(deserializer)?;
        let actual = bytes.len();
        let array: [u8; LIGHT_ARRAY_BYTES] = bytes.try_into().map_err(|_| {
            serde::de::Error::invalid_length(actual, &"a 2048-byte packed light array")
        })?;
        Ok(Self(Arc::new(array)))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TerrainSectionLightRef<'a> {
    Uniform(u8),
    Packed(&'a [u8; LIGHT_ARRAY_BYTES]),
}

impl TerrainSectionLightRef<'_> {
    #[inline]
    pub fn get(self, block_index: usize) -> u8 {
        match self {
            Self::Uniform(value) => value,
            Self::Packed(bytes) => (bytes[block_index >> 1] >> ((block_index & 1) << 2)) & 0x0f,
        }
    }
}

/// Borrowed paletted block/biome and packed-nibble light view of a single 16x16x16 section.
#[derive(Debug, Clone, Copy)]
pub struct TerrainSectionView<'a> {
    pub block_states: &'a PalettedContainerData,
    pub biomes: Option<&'a PalettedContainerData>,
    pub sky_light: TerrainSectionLightRef<'a>,
    pub block_light: TerrainSectionLightRef<'a>,
}

impl TerrainSectionView<'_> {
    #[inline]
    pub fn uniform_block_state_id(&self) -> Option<u32> {
        (self.block_states.palette_kind == PaletteKind::SingleValue).then(|| {
            self.block_states
                .palette_global_ids
                .first()
                .copied()
                .unwrap_or(0) as u32
        })
    }

    #[inline]
    pub fn block_state_id_at(&self, block_index: usize) -> u32 {
        self.block_states.global_id_at(block_index).unwrap_or(0) as u32
    }

    #[inline]
    pub fn biome_id_at(&self, quart_index: usize) -> Option<i32> {
        self.biomes?.global_id_at(quart_index)
    }

    #[inline]
    pub fn light_at(&self, block_index: usize) -> TerrainLight {
        TerrainLight {
            sky: self.sky_light.get(block_index),
            block: self.block_light.get(block_index),
        }
    }
}

/// Lock-free column backing sampled directly by the terrain mesher without 48 KiB expansion.
pub trait TerrainChunkSource: fmt::Debug + Send + Sync {
    fn section(&self, section_index: usize) -> Option<TerrainSectionView<'_>>;
}

#[cfg(test)]
mod tests {
    use bbb_protocol::{
        block_state_types::VANILLA_BLOCK_STATE_COUNT,
        content_registry::{ContentRegistryBuilder, ModBlockRegistration},
    };

    use super::*;

    fn test_mod_registry() -> Arc<ContentRegistry> {
        let mut builder = ContentRegistryBuilder::new();
        builder
            .register_block(ModBlockRegistration {
                resource_id: "example:glow_brick".to_string(),
                state_count: 2,
                light_emission: 14,
                destroy_speed: 2.0,
                full_cube_collision: true,
                dynamic_light_rgb: None,
            })
            .register_block(ModBlockRegistration {
                resource_id: "example:crystal_torch".to_string(),
                state_count: 1,
                light_emission: 12,
                destroy_speed: 0.0,
                full_cube_collision: false,
                dynamic_light_rgb: Some([0.3, 0.8, 1.0]),
            });
        Arc::new(builder.freeze_for_session().expect("valid mod registry"))
    }

    #[test]
    fn terrain_context_resolves_mod_block_render_facts_from_content_registry() {
        let registry = test_mod_registry();
        let glow = registry.block_by_resource_id("example:glow_brick").unwrap();
        let torch = registry
            .block_by_resource_id("example:crystal_torch")
            .unwrap();

        let no_base_facts: TerrainBaseStateFactsFn = |_| None;
        let vanilla_ctx = TerrainBlockRenderContext::new(Vec::new(), no_base_facts);
        let mod_ctx = TerrainBlockRenderContext::new(vec![glow.block_id], no_base_facts)
            .with_content_registry(Arc::clone(&registry));

        assert_eq!(
            vanilla_ctx.total_block_state_count(),
            VANILLA_BLOCK_STATE_COUNT
        );
        assert_eq!(
            mod_ctx.total_block_state_count(),
            VANILLA_BLOCK_STATE_COUNT + 3
        );
        assert_ne!(vanilla_ctx, mod_ctx);
        assert_eq!(
            TerrainBlockRenderContext::new(vec![glow.block_id], no_base_facts)
                .with_content_registry(test_mod_registry()),
            mod_ctx
        );

        // Full-cube modded block resolves opaque occlusion, light dampening/emission, and identity.
        for offset in 0..i32::from(glow.state_count) {
            let state = mod_ctx.block_state(glow.first_state_id + offset);
            assert_eq!(state.block_state_id, glow.first_state_id + offset);
            assert_eq!(state.block_name, Some("example:glow_brick"));
            assert_eq!(state.material, TerrainMaterialClass::Opaque);
            assert!(state.solid_render);
            assert!(state.legacy_solid);
            assert!(state.blocks_motion);
            assert!(state.can_occlude);
            assert!(state.view_blocking);
            assert_eq!(state.light_dampening, 15);
            assert_eq!(state.shade_brightness, 51);
            assert_eq!(state.face_occlusion_shape_indices, [1; 6]);
            assert_eq!(state.state_light_emission, 14);
            assert_eq!(
                state.skip_rendering.same_block_key,
                glow.block_id as u64 + 1
            );
            assert!(state.skip_rendering.bars_tag);
        }

        // Non-full-cube modded block resolves cutout non-occluding facts while retaining light emission.
        let torch_state = mod_ctx.block_state(torch.first_state_id);
        assert_eq!(torch_state.block_name, Some("example:crystal_torch"));
        assert_eq!(torch_state.material, TerrainMaterialClass::Cutout);
        assert!(!torch_state.solid_render);
        assert!(!torch_state.legacy_solid);
        assert!(!torch_state.blocks_motion);
        assert!(!torch_state.can_occlude);
        assert!(!torch_state.view_blocking);
        assert_eq!(torch_state.light_dampening, 0);
        assert_eq!(torch_state.shade_brightness, 255);
        assert_eq!(torch_state.face_occlusion_shape_indices, [0; 6]);
        assert_eq!(torch_state.state_light_emission, 12);
        assert!(!torch_state.skip_rendering.bars_tag);
        assert_ne!(
            torch_state.skip_rendering.same_block_key,
            glow.block_id as u64 + 1
        );

        // Explicit registry lookup works without replacing the context's attached registry.
        assert_eq!(
            vanilla_ctx.block_state_with_registry(torch.first_state_id, &registry),
            torch_state
        );

        // Vanilla block states remain unchanged on modded contexts, and out-of-range states stay default.
        assert_eq!(mod_ctx.block_state(1), vanilla_ctx.block_state(1));
        let unknown = mod_ctx.block_state(i32::MAX);
        assert_eq!(unknown.block_name, None);
        assert!(!unknown.can_occlude);
        assert_eq!(unknown.state_light_emission, 0);
    }
}
