//! Bed block render sources.
//!
//! Vanilla renders beds through `BlockEntityRenderDispatcher` + `BedRenderer`:
//! per bed block entity, a `BedRenderState` carrying the block entity's dye
//! color (`BedBlockEntity.getColor`, set from the `<color>_bed` block item),
//! the block state's `FACING`/`PART`, and light coords combined with the
//! other half via `DoubleBlockCombiner` + `BrightnessCombiner`. The bed has
//! no animation and no NBT the renderer reads (the color is a block-id fact),
//! so bbb derives everything from the chunk block states each frame like the
//! sign projection (`sign_blocks.rs`).

use serde::{Deserialize, Serialize};

use crate::{BlockPos, ChunkColumn, PaletteKind, RegistrySet, WorldStore};

/// Vanilla `DyeColor` in id order: the sixteen `minecraft:<color>_bed` blocks
/// / `entity/bed/<color>.png` sprites.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BedColorKind {
    White,
    Orange,
    Magenta,
    LightBlue,
    Yellow,
    Lime,
    Pink,
    Gray,
    LightGray,
    Cyan,
    Purple,
    Blue,
    Brown,
    Green,
    Red,
    Black,
}

impl BedColorKind {
    fn parse(name: &str) -> Option<Self> {
        match name {
            "white" => Some(Self::White),
            "orange" => Some(Self::Orange),
            "magenta" => Some(Self::Magenta),
            "light_blue" => Some(Self::LightBlue),
            "yellow" => Some(Self::Yellow),
            "lime" => Some(Self::Lime),
            "pink" => Some(Self::Pink),
            "gray" => Some(Self::Gray),
            "light_gray" => Some(Self::LightGray),
            "cyan" => Some(Self::Cyan),
            "purple" => Some(Self::Purple),
            "blue" => Some(Self::Blue),
            "brown" => Some(Self::Brown),
            "green" => Some(Self::Green),
            "red" => Some(Self::Red),
            "black" => Some(Self::Black),
            _ => None,
        }
    }
}

/// Vanilla `BedPart` (`BedBlock.PART`): each bed half is its own block and
/// renders its own `BedRenderer` piece mesh.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BedPartKind {
    Head,
    Foot,
}

/// The bed block state's horizontal `facing` property (`BedBlock.FACING` —
/// the direction the bed head points).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BedModelFacing {
    North,
    South,
    West,
    East,
}

impl BedModelFacing {
    fn parse(value: &str) -> Option<Self> {
        match value {
            "north" => Some(Self::North),
            "south" => Some(Self::South),
            "west" => Some(Self::West),
            "east" => Some(Self::East),
            _ => None,
        }
    }

    /// Vanilla `Direction.getOpposite()`.
    fn opposite(self) -> Self {
        match self {
            Self::North => Self::South,
            Self::South => Self::North,
            Self::West => Self::East,
            Self::East => Self::West,
        }
    }

    fn step(self) -> (i32, i32) {
        match self {
            Self::North => (0, -1),
            Self::South => (0, 1),
            Self::West => (-1, 0),
            Self::East => (1, 0),
        }
    }

    /// Vanilla `Direction.toYRot()` (SOUTH 0°, WEST 90°, NORTH 180°, EAST 270°).
    pub fn to_y_rot(self) -> f32 {
        match self {
            Self::South => 0.0,
            Self::West => 90.0,
            Self::North => 180.0,
            Self::East => 270.0,
        }
    }
}

/// One bed block's per-frame render source: everything the renderer's bed
/// model submission needs except light (sampled on the projection side).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BedModelSourceState {
    pub pos: BlockPos,
    pub color: BedColorKind,
    pub part: BedPartKind,
    pub facing: BedModelFacing,
    /// The other half's block position when both halves are placed, for the
    /// vanilla `DoubleBlockCombiner` + `BrightnessCombiner` per-component
    /// light max. `None` when the neighbour toward `getNeighbourDirection`
    /// is not the matching other half (vanilla falls back to the own-block
    /// light).
    pub partner_pos: Option<BlockPos>,
}

/// Maps a bed block name to its dye color. `None` for every non-bed block
/// (including `minecraft:bedrock`, which does not end in `_bed`).
pub fn bed_color_for_block_name(block_name: &str) -> Option<BedColorKind> {
    let name = block_name.strip_prefix("minecraft:")?;
    BedColorKind::parse(name.strip_suffix("_bed")?)
}

impl WorldStore {
    /// Enumerates every bed block in the loaded chunks as a render source,
    /// deriving color/part/facing from the block state (the vanilla client
    /// materialises a `BedBlockEntity` per bed block state; the color it
    /// carries is the block id's dye color) plus the vanilla double-block
    /// partner for the light combine. Sections whose block palette holds no
    /// bed state are skipped wholesale, mirroring the chest/sign palette
    /// pre-check. Sorted by position for a deterministic frame order.
    pub fn bed_model_source_states(&self) -> Vec<BedModelSourceState> {
        let mut states = Vec::new();
        for chunk in &self.chunks {
            self.collect_chunk_bed_model_source_states(chunk, &mut states);
        }
        states.sort_by_key(|state| (state.pos.y, state.pos.z, state.pos.x));
        states
    }

    fn collect_chunk_bed_model_source_states(
        &self,
        chunk: &ChunkColumn,
        states: &mut Vec<BedModelSourceState>,
    ) {
        for (section_index, section) in chunk.sections.iter().enumerate() {
            if !section_palette_may_contain_bed(
                &section.block_states.palette_global_ids,
                section.block_states.palette_kind,
                &self.registries,
            ) {
                continue;
            }
            let Ok(section_offset) = i32::try_from(section_index) else {
                continue;
            };
            let section_min_y = (self.dimension.min_section_y() + section_offset) * 16;
            for index in 0..section.block_states.entry_count {
                let Some(value) = section.block_states.value_at(index) else {
                    continue;
                };
                let Some(block_state) = self.registries.block_state(value.global_id) else {
                    continue;
                };
                let Some(color) = bed_color_for_block_name(&block_state.name) else {
                    continue;
                };
                let pos = BlockPos {
                    x: chunk.pos.x * 16 + (index & 0xF) as i32,
                    y: section_min_y + (index >> 8) as i32,
                    z: chunk.pos.z * 16 + ((index >> 4) & 0xF) as i32,
                };
                let facing = block_state
                    .properties
                    .get("facing")
                    .and_then(|value| BedModelFacing::parse(value))
                    .unwrap_or(BedModelFacing::North);
                let part = match block_state.properties.get("part").map(String::as_str) {
                    Some("head") => BedPartKind::Head,
                    _ => BedPartKind::Foot,
                };
                let partner_pos = self.bed_partner_pos(pos, &block_state.name, facing, part);
                states.push(BedModelSourceState {
                    pos,
                    color,
                    part,
                    facing,
                    partner_pos,
                });
            }
        }
    }

    /// Vanilla `BedBlock.getNeighbourDirection(part, facing)` (`FOOT ->
    /// facing`, `HEAD -> facing.getOpposite()`) + the `DoubleBlockCombiner`
    /// pairing check (`DoubleBlockCombiner.java:42-46`): the neighbour must
    /// be the same block with the other `part` and the same `facing`.
    fn bed_partner_pos(
        &self,
        pos: BlockPos,
        block_name: &str,
        facing: BedModelFacing,
        part: BedPartKind,
    ) -> Option<BlockPos> {
        let toward = match part {
            BedPartKind::Foot => facing,
            BedPartKind::Head => facing.opposite(),
        };
        let (step_x, step_z) = toward.step();
        let partner_pos = BlockPos {
            x: pos.x + step_x,
            y: pos.y,
            z: pos.z + step_z,
        };
        let partner = self.probe_block(partner_pos)?;
        if partner.block_name.as_deref() != Some(block_name) {
            return None;
        }
        let expected_partner_part = match part {
            BedPartKind::Head => "foot",
            BedPartKind::Foot => "head",
        };
        let expected_facing = match facing {
            BedModelFacing::North => "north",
            BedModelFacing::South => "south",
            BedModelFacing::West => "west",
            BedModelFacing::East => "east",
        };
        (partner.block_properties.get("part").map(String::as_str) == Some(expected_partner_part)
            && partner.block_properties.get("facing").map(String::as_str) == Some(expected_facing))
        .then_some(partner_pos)
    }
}

/// Whether a section's block palette can hold a bed state. Local and
/// single-value palettes are answered from the palette id list; a global
/// palette stores raw state ids, so it must be scanned.
fn section_palette_may_contain_bed(
    palette_global_ids: &[i32],
    palette_kind: PaletteKind,
    registries: &RegistrySet,
) -> bool {
    match palette_kind {
        PaletteKind::SingleValue | PaletteKind::Local => {
            palette_global_ids.iter().any(|global_id| {
                registries
                    .block_state(*global_id)
                    .is_some_and(|state| bed_color_for_block_name(&state.name).is_some())
            })
        }
        PaletteKind::Global => true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        ChunkPos, ChunkSection, ChunkState, LightData, PaletteDomain, PalettedContainerData,
        WorldDimension,
    };
    use bbb_protocol::packets::{BlockPos as ProtocolBlockPos, BlockUpdate};
    use std::collections::BTreeMap;

    const VANILLA_AIR_BLOCK_STATE_ID: i32 = 0;

    fn world_with_air_chunk() -> WorldStore {
        let mut world = WorldStore::with_dimension(WorldDimension {
            min_y: 0,
            height: 16,
        });
        world.insert_decoded_chunk(ChunkColumn {
            pos: ChunkPos { x: 0, z: 0 },
            state: ChunkState::Decoded,
            heightmaps: Vec::new(),
            sections: vec![ChunkSection {
                non_empty_block_count: 0,
                fluid_count: 0,
                block_states: single_value_container(
                    PaletteDomain::BlockStates,
                    4096,
                    VANILLA_AIR_BLOCK_STATE_ID,
                ),
                biomes: single_value_container(PaletteDomain::Biomes, 64, 0),
            }],
            block_entities: Vec::new(),
            light: LightData::default(),
        });
        world
    }

    fn single_value_container(
        domain: PaletteDomain,
        entry_count: usize,
        global_id: i32,
    ) -> PalettedContainerData {
        PalettedContainerData {
            domain,
            bits_per_entry: 0,
            palette_kind: PaletteKind::SingleValue,
            palette_global_ids: vec![global_id],
            packed_data: Vec::new(),
            entry_count,
        }
    }

    fn set_block(world: &mut WorldStore, pos: BlockPos, name: &str, properties: &[(&str, &str)]) {
        let properties: BTreeMap<String, String> = properties
            .iter()
            .map(|(key, value)| (key.to_string(), value.to_string()))
            .collect();
        let state_id = world
            .registries
            .block_state_id_by_name_and_properties(name, &properties)
            .unwrap_or_else(|| panic!("no registered state for {name} {properties:?}"));
        assert!(world.apply_block_update(BlockUpdate {
            pos: ProtocolBlockPos {
                x: pos.x,
                y: pos.y,
                z: pos.z,
            },
            block_state_id: state_id,
        }));
    }

    fn set_bed(world: &mut WorldStore, pos: BlockPos, name: &str, facing: &str, part: &str) {
        set_block(
            world,
            pos,
            name,
            &[("facing", facing), ("part", part), ("occupied", "false")],
        );
    }

    #[test]
    fn maps_bed_block_names_to_dye_colors() {
        assert_eq!(
            bed_color_for_block_name("minecraft:red_bed"),
            Some(BedColorKind::Red)
        );
        assert_eq!(
            bed_color_for_block_name("minecraft:light_blue_bed"),
            Some(BedColorKind::LightBlue)
        );
        assert_eq!(
            bed_color_for_block_name("minecraft:black_bed"),
            Some(BedColorKind::Black)
        );
        // Non-bed blocks (including bedrock) stay out.
        assert_eq!(bed_color_for_block_name("minecraft:bedrock"), None);
        assert_eq!(bed_color_for_block_name("minecraft:chest"), None);
        assert_eq!(bed_color_for_block_name("minecraft:stone_bed"), None);
    }

    #[test]
    fn enumerates_bed_sources_with_color_part_facing_and_pairing() {
        let mut world = world_with_air_chunk();
        // A south-facing red bed: the head sits toward facing (south, +z) of
        // the foot; from the head the partner is toward facing.getOpposite().
        let foot_pos = BlockPos { x: 3, y: 4, z: 5 };
        let head_pos = BlockPos { x: 3, y: 4, z: 6 };
        set_bed(&mut world, foot_pos, "minecraft:red_bed", "south", "foot");
        set_bed(&mut world, head_pos, "minecraft:red_bed", "south", "head");
        // A lone east-facing lime bed head with no matching foot.
        let lone_pos = BlockPos { x: 8, y: 4, z: 5 };
        set_bed(&mut world, lone_pos, "minecraft:lime_bed", "east", "head");

        let sources = world.bed_model_source_states();
        assert_eq!(sources.len(), 3);
        assert_eq!(
            sources[0],
            BedModelSourceState {
                pos: foot_pos,
                color: BedColorKind::Red,
                part: BedPartKind::Foot,
                facing: BedModelFacing::South,
                partner_pos: Some(head_pos),
            }
        );
        assert_eq!(
            sources[1],
            BedModelSourceState {
                pos: lone_pos,
                color: BedColorKind::Lime,
                part: BedPartKind::Head,
                facing: BedModelFacing::East,
                partner_pos: None,
            }
        );
        assert_eq!(
            sources[2],
            BedModelSourceState {
                pos: head_pos,
                color: BedColorKind::Red,
                part: BedPartKind::Head,
                facing: BedModelFacing::South,
                partner_pos: Some(foot_pos),
            }
        );
    }

    #[test]
    fn mismatched_neighbour_breaks_the_pairing() {
        let mut world = world_with_air_chunk();
        // The neighbour toward the connection direction is a *different*
        // color bed: vanilla `neighbourState.is(state.getBlock())` fails and
        // the light combine falls back to the single block.
        let foot_pos = BlockPos { x: 3, y: 4, z: 5 };
        let head_pos = BlockPos { x: 3, y: 4, z: 6 };
        set_bed(&mut world, foot_pos, "minecraft:red_bed", "south", "foot");
        set_bed(&mut world, head_pos, "minecraft:blue_bed", "south", "head");
        let sources = world.bed_model_source_states();
        assert_eq!(sources.len(), 2);
        assert_eq!(sources[0].partner_pos, None);
        assert_eq!(sources[1].partner_pos, None);

        // Same block but same `part` (two foot halves) also fails the
        // `type != neighbourType` check.
        set_bed(&mut world, head_pos, "minecraft:red_bed", "south", "foot");
        let sources = world.bed_model_source_states();
        assert!(sources.iter().all(|source| source.partner_pos.is_none()));

        // Same block, other part, but different facing fails the
        // `facingProperty` equality check.
        set_bed(&mut world, head_pos, "minecraft:red_bed", "north", "head");
        let sources = world.bed_model_source_states();
        assert!(sources.iter().all(|source| source.partner_pos.is_none()));
    }

    #[test]
    fn removed_bed_stops_enumerating() {
        let mut world = world_with_air_chunk();
        let pos = BlockPos { x: 3, y: 4, z: 5 };
        set_bed(&mut world, pos, "minecraft:cyan_bed", "west", "foot");
        assert_eq!(world.bed_model_source_states().len(), 1);
        set_block(&mut world, pos, "minecraft:air", &[]);
        assert!(world.bed_model_source_states().is_empty());
    }
}
