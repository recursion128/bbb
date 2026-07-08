//! Client-side skull/head block-entity render sources and animation ticks.
//!
//! Vanilla renders skull blocks through `SkullBlockRenderer`. Ground skulls use
//! `SkullBlock.ROTATION` with `createGroundTransformation`; wall skulls use
//! `WallSkullBlock.FACING` with `createWallTransformation`. Only dragon and
//! piglin skull/head block entities receive the client ticker, and their
//! animation counter advances only while the block state's inherited
//! `AbstractSkullBlock.POWERED` property is true.

use serde::{Deserialize, Serialize};

use crate::{
    sign_blocks::sign_rotation_segment_to_degrees, BlockPos, ChunkColumn, PaletteKind, RegistrySet,
    WorldStore,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SkullBlockKind {
    Skeleton,
    WitherSkeleton,
    Player,
    Zombie,
    Creeper,
    Dragon,
    Piglin,
}

impl SkullBlockKind {
    pub fn is_animated(self) -> bool {
        matches!(self, Self::Dragon | Self::Piglin)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SkullWallFacing {
    North,
    South,
    West,
    East,
}

impl SkullWallFacing {
    fn parse(name: &str) -> Option<Self> {
        match name {
            "north" => Some(Self::North),
            "south" => Some(Self::South),
            "west" => Some(Self::West),
            "east" => Some(Self::East),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SkullBlockAttachment {
    Ground { rotation_segment: i32 },
    Wall { facing: SkullWallFacing },
}

impl SkullBlockAttachment {
    pub fn ground_rotation_degrees(self) -> f32 {
        match self {
            Self::Ground { rotation_segment } => sign_rotation_segment_to_degrees(rotation_segment),
            Self::Wall { .. } => 0.0,
        }
    }
}

/// One animated skull block entity's client ticker state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SkullBlockState {
    pub pos: BlockPos,
    pub animation_tick_count: i32,
    pub is_animating: bool,
}

impl SkullBlockState {
    fn new(pos: BlockPos) -> Self {
        Self {
            pos,
            animation_tick_count: 0,
            is_animating: false,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SkullModelSourceState {
    pub pos: BlockPos,
    pub kind: SkullBlockKind,
    pub attachment: SkullBlockAttachment,
    pub animation_progress: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct SkullBlockSource {
    pos: BlockPos,
    kind: SkullBlockKind,
    attachment: SkullBlockAttachment,
    powered: bool,
}

pub fn skull_kind_for_block_name(block_name: &str) -> Option<SkullBlockKind> {
    let name = block_name.strip_prefix("minecraft:")?;
    let family = name
        .strip_suffix("_wall_skull")
        .or_else(|| name.strip_suffix("_wall_head"))
        .or_else(|| name.strip_suffix("_skull"))
        .or_else(|| name.strip_suffix("_head"))?;
    match family {
        "skeleton" => Some(SkullBlockKind::Skeleton),
        "wither_skeleton" => Some(SkullBlockKind::WitherSkeleton),
        "player" => Some(SkullBlockKind::Player),
        "zombie" => Some(SkullBlockKind::Zombie),
        "creeper" => Some(SkullBlockKind::Creeper),
        "dragon" => Some(SkullBlockKind::Dragon),
        "piglin" => Some(SkullBlockKind::Piglin),
        _ => None,
    }
}

pub fn is_skull_block_name(block_name: &str) -> bool {
    skull_kind_for_block_name(block_name).is_some()
}

impl WorldStore {
    /// Advances dragon/piglin skull client animation, mirroring
    /// `SkullBlockEntity.animation`: while powered, `isAnimating = true` and
    /// `animationTickCount++`; otherwise the counter is retained and
    /// `isAnimating = false`.
    pub fn advance_skull_block_ticks(&mut self, ticks: u32) {
        let mut sources = self.skull_block_sources();
        sources.sort_by_key(|source| (source.pos.y, source.pos.z, source.pos.x));

        let mut states = std::mem::take(&mut self.skull_blocks);
        states.retain(|state| {
            sources
                .binary_search_by_key(&(state.pos.y, state.pos.z, state.pos.x), |source| {
                    (source.pos.y, source.pos.z, source.pos.x)
                })
                .is_ok()
        });
        for source in &sources {
            if !states.iter().any(|state| state.pos == source.pos) {
                states.push(SkullBlockState::new(source.pos));
            }
        }
        states.sort_by_key(|state| (state.pos.y, state.pos.z, state.pos.x));

        if ticks > 0 {
            for state in &mut states {
                let Some(source) = sources.iter().find(|source| source.pos == state.pos) else {
                    continue;
                };
                let animating = source.kind.is_animated() && source.powered;
                for _ in 0..ticks {
                    state.is_animating = animating;
                    if state.is_animating {
                        state.animation_tick_count = state.animation_tick_count.saturating_add(1);
                    }
                }
            }
        }

        self.skull_blocks = states;
    }

    pub fn skull_block_states(&self) -> &[SkullBlockState] {
        &self.skull_blocks
    }

    pub fn skull_model_source_states(&self, partial_tick: f32) -> Vec<SkullModelSourceState> {
        let mut states = Vec::new();
        for source in self.skull_block_sources() {
            let skull_state = self
                .skull_blocks
                .iter()
                .find(|state| state.pos == source.pos)
                .copied()
                .unwrap_or_else(|| SkullBlockState::new(source.pos));
            let animation_progress = if skull_state.is_animating {
                skull_state.animation_tick_count as f32 + partial_tick
            } else {
                skull_state.animation_tick_count as f32
            };
            states.push(SkullModelSourceState {
                pos: source.pos,
                kind: source.kind,
                attachment: source.attachment,
                animation_progress,
            });
        }
        states.sort_by_key(|state| (state.pos.y, state.pos.z, state.pos.x));
        states
    }

    fn skull_block_sources(&self) -> Vec<SkullBlockSource> {
        let mut sources = Vec::new();
        for chunk in &self.chunks {
            self.collect_chunk_skull_block_sources(chunk, &mut sources);
        }
        sources
    }

    fn collect_chunk_skull_block_sources(
        &self,
        chunk: &ChunkColumn,
        sources: &mut Vec<SkullBlockSource>,
    ) {
        for (section_index, section) in chunk.sections.iter().enumerate() {
            if !section_palette_may_contain_skull(
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
                let Some(kind) = skull_kind_for_block_name(&block_state.name) else {
                    continue;
                };
                let Some(attachment) = skull_attachment_for_block_state(
                    &block_state.name,
                    block_state
                        .properties
                        .iter()
                        .map(|(key, value)| (key.as_str(), value.as_str())),
                ) else {
                    continue;
                };
                let pos = BlockPos {
                    x: chunk.pos.x * 16 + (index & 0xF) as i32,
                    y: section_min_y + (index >> 8) as i32,
                    z: chunk.pos.z * 16 + ((index >> 4) & 0xF) as i32,
                };
                sources.push(SkullBlockSource {
                    pos,
                    kind,
                    attachment,
                    powered: block_state
                        .properties
                        .get("powered")
                        .is_some_and(|value| value == "true"),
                });
            }
        }
    }
}

fn skull_attachment_for_block_state<'a>(
    block_name: &str,
    properties: impl Iterator<Item = (&'a str, &'a str)>,
) -> Option<SkullBlockAttachment> {
    let mut rotation = None;
    let mut facing = None;
    for (key, value) in properties {
        match key {
            "rotation" => rotation = value.parse::<i32>().ok(),
            "facing" => facing = SkullWallFacing::parse(value),
            _ => {}
        }
    }
    let name = block_name.strip_prefix("minecraft:")?;
    if name.ends_with("_wall_skull") || name.ends_with("_wall_head") {
        Some(SkullBlockAttachment::Wall {
            facing: facing.unwrap_or(SkullWallFacing::North),
        })
    } else {
        Some(SkullBlockAttachment::Ground {
            rotation_segment: rotation.unwrap_or(0),
        })
    }
}

fn section_palette_may_contain_skull(
    palette_global_ids: &[i32],
    palette_kind: PaletteKind,
    registries: &RegistrySet,
) -> bool {
    match palette_kind {
        PaletteKind::SingleValue | PaletteKind::Local => {
            palette_global_ids.iter().any(|global_id| {
                registries
                    .block_state(*global_id)
                    .is_some_and(|state| is_skull_block_name(&state.name))
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
            .registries()
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

    #[test]
    fn maps_skull_block_families() {
        assert_eq!(
            skull_kind_for_block_name("minecraft:skeleton_skull"),
            Some(SkullBlockKind::Skeleton)
        );
        assert_eq!(
            skull_kind_for_block_name("minecraft:wither_skeleton_wall_skull"),
            Some(SkullBlockKind::WitherSkeleton)
        );
        assert_eq!(
            skull_kind_for_block_name("minecraft:player_wall_head"),
            Some(SkullBlockKind::Player)
        );
        assert_eq!(
            skull_kind_for_block_name("minecraft:piglin_head"),
            Some(SkullBlockKind::Piglin)
        );
        assert_eq!(skull_kind_for_block_name("minecraft:stone"), None);
    }

    #[test]
    fn sources_project_ground_and_wall_attachment() {
        let mut world = world_with_air_chunk();
        set_block(
            &mut world,
            BlockPos { x: 2, y: 3, z: 4 },
            "minecraft:skeleton_skull",
            &[("powered", "false"), ("rotation", "4")],
        );
        set_block(
            &mut world,
            BlockPos { x: 5, y: 3, z: 4 },
            "minecraft:creeper_wall_head",
            &[("powered", "false"), ("facing", "east")],
        );

        let sources = world.skull_model_source_states(0.5);

        assert_eq!(sources.len(), 2);
        assert_eq!(sources[0].kind, SkullBlockKind::Skeleton);
        assert_eq!(
            sources[0].attachment,
            SkullBlockAttachment::Ground {
                rotation_segment: 4
            }
        );
        assert_eq!(sources[0].animation_progress, 0.0);
        assert_eq!(sources[1].kind, SkullBlockKind::Creeper);
        assert_eq!(
            sources[1].attachment,
            SkullBlockAttachment::Wall {
                facing: SkullWallFacing::East
            }
        );
    }

    #[test]
    fn powered_dragon_skull_advances_animation_until_unpowered() {
        let mut world = world_with_air_chunk();
        let pos = BlockPos { x: 2, y: 3, z: 4 };
        set_block(
            &mut world,
            pos,
            "minecraft:dragon_head",
            &[("powered", "true"), ("rotation", "2")],
        );

        world.advance_skull_block_ticks(3);

        assert_eq!(
            world.skull_block_states(),
            &[SkullBlockState {
                pos,
                animation_tick_count: 3,
                is_animating: true,
            }]
        );
        assert_eq!(
            world.skull_model_source_states(0.5)[0].animation_progress,
            3.5
        );

        set_block(
            &mut world,
            pos,
            "minecraft:dragon_head",
            &[("powered", "false"), ("rotation", "2")],
        );
        world.advance_skull_block_ticks(1);

        assert_eq!(
            world.skull_block_states(),
            &[SkullBlockState {
                pos,
                animation_tick_count: 3,
                is_animating: false,
            }]
        );
        assert_eq!(
            world.skull_model_source_states(0.75)[0].animation_progress,
            3.0
        );
    }

    #[test]
    fn powered_non_animated_skull_does_not_advance_animation() {
        let mut world = world_with_air_chunk();
        let pos = BlockPos { x: 2, y: 3, z: 4 };
        set_block(
            &mut world,
            pos,
            "minecraft:skeleton_skull",
            &[("powered", "true"), ("rotation", "0")],
        );

        world.advance_skull_block_ticks(2);

        assert_eq!(
            world.skull_block_states(),
            &[SkullBlockState {
                pos,
                animation_tick_count: 0,
                is_animating: false,
            }]
        );
        assert_eq!(
            world.skull_model_source_states(0.75)[0].animation_progress,
            0.0
        );
    }
}
