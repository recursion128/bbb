//! Client-side skull/head block-entity render sources and animation ticks.
//!
//! Vanilla renders skull blocks through `SkullBlockRenderer`. Ground skulls use
//! `SkullBlock.ROTATION` with `createGroundTransformation`; wall skulls use
//! `WallSkullBlock.FACING` with `createWallTransformation`. Only dragon and
//! piglin skull/head block entities receive the client ticker, and their
//! animation counter advances only while the block state's inherited
//! `AbstractSkullBlock.POWERED` property is true.

use crate::chunks::BlockFamily;
use serde::{Deserialize, Serialize};

use bbb_protocol::packets::inventory::ResolvableProfileSummary;

use crate::terrain::{block_name_is_floor_skull, block_name_is_wall_skull};
use crate::{sign_blocks::sign_rotation_segment_to_degrees, BlockPos, ChunkColumn, WorldStore};

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

#[derive(Debug, Clone, PartialEq)]
pub struct SkullModelSourceState {
    pub pos: BlockPos,
    pub kind: SkullBlockKind,
    pub attachment: SkullBlockAttachment,
    pub animation_progress: f32,
    pub owner_profile: Option<ResolvableProfileSummary>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct SkullBlockSource {
    pos: BlockPos,
    kind: SkullBlockKind,
    attachment: SkullBlockAttachment,
}

pub fn skull_kind_for_block_name(block_name: &str) -> Option<SkullBlockKind> {
    match block_name {
        "minecraft:skeleton_skull" | "minecraft:skeleton_wall_skull" => {
            Some(SkullBlockKind::Skeleton)
        }
        "minecraft:wither_skeleton_skull" | "minecraft:wither_skeleton_wall_skull" => {
            Some(SkullBlockKind::WitherSkeleton)
        }
        "minecraft:player_head" | "minecraft:player_wall_head" => Some(SkullBlockKind::Player),
        "minecraft:zombie_head" | "minecraft:zombie_wall_head" => Some(SkullBlockKind::Zombie),
        "minecraft:creeper_head" | "minecraft:creeper_wall_head" => Some(SkullBlockKind::Creeper),
        "minecraft:dragon_head" | "minecraft:dragon_wall_head" => Some(SkullBlockKind::Dragon),
        "minecraft:piglin_head" | "minecraft:piglin_wall_head" => Some(SkullBlockKind::Piglin),
        _ => None,
    }
}

pub fn is_skull_block_name(block_name: &str) -> bool {
    block_name_is_floor_skull(block_name) || block_name_is_wall_skull(block_name)
}

impl WorldStore {
    /// Advances dragon/piglin skull client animation, mirroring
    /// `SkullBlockEntity.animation`: while powered, `isAnimating = true` and
    /// `animationTickCount++`; otherwise the counter is retained and
    /// `isAnimating = false`.
    pub fn advance_skull_block_ticks(&mut self, ticks: u32) {
        let reconcile = self.reconcile_client_ticking_block_entities(BlockFamily::Skull);
        if ticks == 0 && !reconcile {
            return;
        }
        if !self.client_ticking_block_entity_presence().skull {
            self.skull_blocks.clear();
            return;
        }
        let mut states = std::mem::take(&mut self.skull_blocks);
        if reconcile {
            let mut sources = self.skull_block_sources();
            sources.sort_by_key(|source| (source.pos.y, source.pos.z, source.pos.x));

            states.retain(|state| {
                sources
                    .binary_search_by_key(&(state.pos.y, state.pos.z, state.pos.x), |source| {
                        (source.pos.y, source.pos.z, source.pos.x)
                    })
                    .is_ok()
            });
            let existing_len = states.len();
            for source in &sources {
                let key = (source.pos.y, source.pos.z, source.pos.x);
                if states[..existing_len]
                    .binary_search_by_key(&key, |state| (state.pos.y, state.pos.z, state.pos.x))
                    .is_err()
                {
                    states.push(SkullBlockState::new(source.pos));
                }
            }
            if states.len() != existing_len {
                states.sort_by_key(|state| (state.pos.y, state.pos.z, state.pos.x));
            }
        }

        if ticks > 0 {
            for state in &mut states {
                let state_id = self.block_state_id_at(state.pos).unwrap();
                let block_state = self.registries.block_state(state_id).unwrap();
                let animating = skull_kind_for_block_name(&block_state.name)
                    .unwrap()
                    .is_animated()
                    && block_state
                        .properties
                        .get("powered")
                        .is_some_and(|value| value == "true");
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
        self.skull_model_source_states_matching(partial_tick, |_| true)
    }

    pub fn skull_model_source_states_matching(
        &self,
        partial_tick: f32,
        include: impl Fn(BlockPos) -> bool,
    ) -> Vec<SkullModelSourceState> {
        let mut states = Vec::new();
        for source in self.skull_block_sources_matching(include) {
            let key = (source.pos.y, source.pos.z, source.pos.x);
            let skull_state = self
                .skull_blocks
                .binary_search_by_key(&key, |state| (state.pos.y, state.pos.z, state.pos.x))
                .map(|index| self.skull_blocks[index])
                .unwrap_or_else(|_| SkullBlockState::new(source.pos));
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
                owner_profile: if source.kind == SkullBlockKind::Player {
                    self.block_entity_record_at(source.pos)
                        .and_then(|record| record.skull_profile().cloned())
                } else {
                    None
                },
            });
        }
        states.sort_by_key(|state| (state.pos.y, state.pos.z, state.pos.x));
        states
    }

    fn skull_block_sources(&self) -> Vec<SkullBlockSource> {
        self.skull_block_sources_matching(|_| true)
    }

    fn skull_block_sources_matching(
        &self,
        include: impl Fn(BlockPos) -> bool,
    ) -> Vec<SkullBlockSource> {
        let mut sources = Vec::new();
        for chunk in self.chunks.with_family(BlockFamily::Skull) {
            self.collect_chunk_skull_block_sources(chunk, &include, &mut sources);
        }
        sources
    }

    fn collect_chunk_skull_block_sources(
        &self,
        chunk: &ChunkColumn,
        include: &impl Fn(BlockPos) -> bool,
        sources: &mut Vec<SkullBlockSource>,
    ) {
        for (pos, state_id) in self.chunks.family_blocks(
            chunk.pos,
            BlockFamily::Skull,
            self.dimension.min_section_y(),
        ) {
            if !include(pos) {
                continue;
            }
            let block_state = self.registries.block_state(state_id).unwrap();
            let kind = skull_kind_for_block_name(&block_state.name).unwrap();
            let Some(attachment) = skull_attachment_for_block_state(
                &block_state.name,
                block_state
                    .properties
                    .iter()
                    .map(|(key, value)| (key.as_str(), value.as_str())),
            ) else {
                continue;
            };
            sources.push(SkullBlockSource {
                pos,
                kind,
                attachment,
            });
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
    if block_name_is_wall_skull(block_name) {
        Some(SkullBlockAttachment::Wall {
            facing: facing.unwrap_or(SkullWallFacing::North),
        })
    } else if block_name_is_floor_skull(block_name) {
        Some(SkullBlockAttachment::Ground {
            rotation_segment: rotation.unwrap_or(0),
        })
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::PaletteKind;
    use crate::{
        ChunkPos, ChunkSection, ChunkState, LightData, PaletteDomain, PalettedContainerData,
        WorldDimension,
    };
    use bbb_protocol::block_entity_types::VANILLA_BLOCK_ENTITY_TYPE_SKULL_ID;
    use bbb_protocol::packets::inventory::{PlayerSkinPatchSummary, ResolvableProfileKindSummary};
    use bbb_protocol::packets::{BlockPos as ProtocolBlockPos, BlockUpdate};
    use std::collections::BTreeMap;

    fn world_with_air_chunk() -> WorldStore {
        let mut world = WorldStore::with_dimension(WorldDimension {
            min_y: 0,
            height: 16,
        });
        world.insert_decoded_chunk(ChunkColumn {
            pos: ChunkPos { x: 0, z: 0 },
            state: ChunkState::Decoded,
            heightmaps: Vec::new(),
            sections: vec![std::sync::Arc::new(ChunkSection {
                non_empty_block_count: 0,
                fluid_count: 0,
                block_states: single_value_container(
                    PaletteDomain::BlockStates,
                    4096,
                    crate::vanilla_air_block_state_id(),
                ),
                biomes: single_value_container(PaletteDomain::Biomes, 64, 0),
            })],
            block_entities: std::sync::Arc::default(),
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

    fn profile(name: &str) -> ResolvableProfileSummary {
        ResolvableProfileSummary {
            kind: ResolvableProfileKindSummary::Partial,
            uuid: None,
            name: Some(name.to_string()),
            properties: Vec::new(),
            profile_textures: None,
            skin_patch: PlayerSkinPatchSummary::default(),
        }
    }

    fn world_with_skull_profile(pos: BlockPos, profile: ResolvableProfileSummary) -> WorldStore {
        let mut world = WorldStore::with_dimension(WorldDimension {
            min_y: 0,
            height: 16,
        });
        let mut block_entity = crate::BlockEntityRecord::empty(
            pos.x.rem_euclid(16) as u8,
            i16::try_from(pos.y).unwrap(),
            pos.z.rem_euclid(16) as u8,
            VANILLA_BLOCK_ENTITY_TYPE_SKULL_ID,
        );
        block_entity.payload = crate::BlockEntityPayload::skull(None, Some(profile));
        world.insert_decoded_chunk(ChunkColumn {
            pos: ChunkPos {
                x: pos.x.div_euclid(16),
                z: pos.z.div_euclid(16),
            },
            state: ChunkState::Decoded,
            heightmaps: Vec::new(),
            sections: vec![std::sync::Arc::new(ChunkSection {
                non_empty_block_count: 0,
                fluid_count: 0,
                block_states: single_value_container(
                    PaletteDomain::BlockStates,
                    4096,
                    crate::vanilla_air_block_state_id(),
                ),
                biomes: single_value_container(PaletteDomain::Biomes, 64, 0),
            })],
            block_entities: std::sync::Arc::new(vec![block_entity]),
            light: LightData::default(),
        });
        world
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
        for block_name in [
            "minecraft:skeleton_skull",
            "minecraft:wither_skeleton_wall_skull",
            "minecraft:player_wall_head",
            "minecraft:piglin_head",
        ] {
            assert!(is_skull_block_name(block_name), "{block_name}");
        }
        assert!(!is_skull_block_name("minecraft:custom_wall_head"));
        assert!(!is_skull_block_name("example:player_head"));
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
    fn player_skull_source_projects_exact_block_entity_profile() {
        let pos = BlockPos { x: 2, y: 3, z: 4 };
        let owner = profile("SkullOwner");
        let mut world = world_with_skull_profile(pos, owner.clone());
        set_block(
            &mut world,
            pos,
            "minecraft:player_head",
            &[("powered", "false"), ("rotation", "0")],
        );

        let sources = world.skull_model_source_states(0.5);

        assert_eq!(sources.len(), 1);
        assert_eq!(sources[0].pos, pos);
        assert_eq!(sources[0].kind, SkullBlockKind::Player);
        assert_eq!(sources[0].owner_profile.as_ref(), Some(&owner));

        set_block(
            &mut world,
            pos,
            "minecraft:skeleton_skull",
            &[("powered", "false"), ("rotation", "0")],
        );
        assert_eq!(world.skull_model_source_states(0.5)[0].owner_profile, None);
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
