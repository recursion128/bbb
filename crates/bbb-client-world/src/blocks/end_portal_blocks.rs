//! Client-side end portal/gateway block-entity render sources.
//!
//! Vanilla renders both blocks through `AbstractEndPortalRenderer`: end portals
//! submit only Y-axis faces (`TheEndPortalBlockEntity.shouldRenderFace`) with
//! `TheEndPortalRenderer.TRANSFORMATION`, while end gateways submit every
//! non-occluded cube face (`TheEndGatewayBlockEntity.shouldRenderFace`) on the
//! unit cube and may add a beacon-beam style spawn/cooldown beam from
//! `TheEndGatewayRenderer`.

use crate::chunks::BlockFamily;
use serde::{Deserialize, Serialize};

use bbb_protocol::block_state_types::{
    vanilla_block_state_name, vanilla_block_state_support_facts, VanillaBlockSupportDirection,
};

use crate::{chunks::EndGatewayBlockEntityData, BlockPos, ChunkColumn, WorldStore};

const END_GATEWAY_SPAWN_TICKS: i64 = 200;
const END_GATEWAY_COOLDOWN_TICKS: i32 = 40;
const END_GATEWAY_SPAWN_COLOR: u32 = 0xFFC74EBD;
const END_GATEWAY_COOLDOWN_COLOR: u32 = 0xFF8932B8;
const END_GATEWAY_FACE_NEIGHBORS: [(EndPortalFace, [i32; 3], VanillaBlockSupportDirection); 6] = [
    (
        EndPortalFace::Down,
        [0, -1, 0],
        VanillaBlockSupportDirection::Up,
    ),
    (
        EndPortalFace::Up,
        [0, 1, 0],
        VanillaBlockSupportDirection::Down,
    ),
    (
        EndPortalFace::North,
        [0, 0, -1],
        VanillaBlockSupportDirection::South,
    ),
    (
        EndPortalFace::South,
        [0, 0, 1],
        VanillaBlockSupportDirection::North,
    ),
    (
        EndPortalFace::West,
        [-1, 0, 0],
        VanillaBlockSupportDirection::East,
    ),
    (
        EndPortalFace::East,
        [1, 0, 0],
        VanillaBlockSupportDirection::West,
    ),
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EndPortalBlockKind {
    EndPortal,
    EndGateway,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EndPortalFace {
    Down,
    Up,
    North,
    South,
    West,
    East,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct EndPortalFaces(u8);

impl EndPortalFaces {
    pub const EMPTY: Self = Self(0);
    pub const Y_AXIS: Self = Self::from_faces(&[EndPortalFace::Down, EndPortalFace::Up]);
    pub const ALL: Self = Self::from_faces(&[
        EndPortalFace::Down,
        EndPortalFace::Up,
        EndPortalFace::North,
        EndPortalFace::South,
        EndPortalFace::West,
        EndPortalFace::East,
    ]);

    const fn bit(face: EndPortalFace) -> u8 {
        match face {
            EndPortalFace::Down => 1 << 0,
            EndPortalFace::Up => 1 << 1,
            EndPortalFace::North => 1 << 2,
            EndPortalFace::South => 1 << 3,
            EndPortalFace::West => 1 << 4,
            EndPortalFace::East => 1 << 5,
        }
    }

    pub const fn from_faces(faces: &[EndPortalFace]) -> Self {
        let mut bits = 0u8;
        let mut index = 0;
        while index < faces.len() {
            bits |= Self::bit(faces[index]);
            index += 1;
        }
        Self(bits)
    }

    pub const fn with(self, face: EndPortalFace) -> Self {
        Self(self.0 | Self::bit(face))
    }

    pub const fn contains(self, face: EndPortalFace) -> bool {
        (self.0 & Self::bit(face)) != 0
    }

    pub fn iter(self) -> impl Iterator<Item = EndPortalFace> {
        [
            EndPortalFace::Down,
            EndPortalFace::Up,
            EndPortalFace::North,
            EndPortalFace::South,
            EndPortalFace::West,
            EndPortalFace::East,
        ]
        .into_iter()
        .filter(move |&face| self.contains(face))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct EndGatewayBlockState {
    pub pos: BlockPos,
    pub age: i64,
    pub loaded_age: i64,
    pub teleport_cooldown: i32,
}

impl EndGatewayBlockState {
    fn new(pos: BlockPos, data: Option<EndGatewayBlockEntityData>) -> Self {
        let loaded_age = data.map(|data| data.age).unwrap_or(0);
        Self {
            pos,
            age: loaded_age,
            loaded_age,
            teleport_cooldown: 0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EndPortalModelSourceState {
    pub pos: BlockPos,
    pub kind: EndPortalBlockKind,
    pub faces: EndPortalFaces,
    pub gateway_beam: Option<EndGatewayBeamSourceState>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EndGatewayBeamSourceState {
    pub scale: f32,
    pub height: i32,
    pub color_argb: u32,
    pub animation_time: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct EndPortalBlockSource {
    pos: BlockPos,
    kind: EndPortalBlockKind,
    gateway_data: Option<EndGatewayBlockEntityData>,
}

pub fn end_portal_kind_for_block_name(block_name: &str) -> Option<EndPortalBlockKind> {
    match block_name {
        "minecraft:end_portal" => Some(EndPortalBlockKind::EndPortal),
        "minecraft:end_gateway" => Some(EndPortalBlockKind::EndGateway),
        _ => None,
    }
}

pub fn is_end_portal_block_name(block_name: &str) -> bool {
    end_portal_kind_for_block_name(block_name).is_some()
}

impl WorldStore {
    pub(crate) fn update_end_gateway_from_block_event(&mut self, pos: BlockPos, b0: u8) -> bool {
        if b0 != 1
            || self.block_view(pos).and_then(|block| block.block_name)
                != Some("minecraft:end_gateway")
        {
            return false;
        }
        let key = (pos.y, pos.z, pos.x);
        match self
            .end_gateways
            .binary_search_by_key(&key, |state| (state.pos.y, state.pos.z, state.pos.x))
        {
            Ok(index) => {
                self.end_gateways[index].teleport_cooldown = END_GATEWAY_COOLDOWN_TICKS;
            }
            Err(insert_index) => {
                let data = self.end_gateway_data_in_chunk(pos);
                let mut state = EndGatewayBlockState::new(pos, data);
                state.teleport_cooldown = END_GATEWAY_COOLDOWN_TICKS;
                self.end_gateways.insert(insert_index, state);
            }
        }
        true
    }

    /// Advances `TheEndGatewayBlockEntity.beamAnimationTick`: `age++` every
    /// client tick and `teleportCooldown--` while cooling down.
    pub fn advance_end_gateway_ticks(&mut self, ticks: u32) {
        let reconcile = self.reconcile_client_ticking_block_entities(BlockFamily::EndGateway);
        if ticks == 0 && !reconcile {
            return;
        }
        if !self.client_ticking_block_entity_presence().end_gateway {
            self.end_gateways.clear();
            return;
        }
        let mut states = std::mem::take(&mut self.end_gateways);
        if reconcile {
            let mut gateway_sources: Vec<_> = self
                .end_portal_block_sources()
                .into_iter()
                .filter(|source| source.kind == EndPortalBlockKind::EndGateway)
                .collect();
            gateway_sources.sort_by_key(|source| (source.pos.y, source.pos.z, source.pos.x));
            if !states.is_sorted_by_key(|state| (state.pos.y, state.pos.z, state.pos.x)) {
                states.sort_by_key(|state| (state.pos.y, state.pos.z, state.pos.x));
            }
            let mut merged = Vec::with_capacity(gateway_sources.len());
            let mut existing = states.into_iter().peekable();
            for source in &gateway_sources {
                let key = (source.pos.y, source.pos.z, source.pos.x);
                while existing
                    .peek()
                    .is_some_and(|state| (state.pos.y, state.pos.z, state.pos.x) < key)
                {
                    existing.next();
                }
                if existing
                    .peek()
                    .is_some_and(|state| (state.pos.y, state.pos.z, state.pos.x) == key)
                {
                    let mut state = existing.next().unwrap();
                    let loaded_age = source.gateway_data.map(|data| data.age).unwrap_or(0);
                    if state.loaded_age != loaded_age {
                        state.age = loaded_age;
                        state.loaded_age = loaded_age;
                    }
                    merged.push(state);
                } else {
                    merged.push(EndGatewayBlockState::new(source.pos, source.gateway_data));
                }
            }
            states = merged.into();
        }

        for state in &mut states {
            for _ in 0..ticks {
                state.age = state.age.saturating_add(1);
                if state.teleport_cooldown > 0 {
                    state.teleport_cooldown -= 1;
                }
            }
        }

        self.end_gateways = states;
    }

    pub fn end_gateway_states(&self) -> &[EndGatewayBlockState] {
        &self.end_gateways
    }

    /// Vanilla `TheEndGatewayBlockEntity.shouldRenderFace`: every face whose
    /// neighbouring full occlusion face does not hide the gateway.
    fn end_gateway_visible_faces(&self, pos: BlockPos) -> EndPortalFaces {
        let mut faces = EndPortalFaces::EMPTY;
        for (face, step, neighbor_face) in END_GATEWAY_FACE_NEIGHBORS {
            let neighbor = BlockPos {
                x: pos.x.saturating_add(step[0]),
                y: pos.y.saturating_add(step[1]),
                z: pos.z.saturating_add(step[2]),
            };
            if self
                .block_view(neighbor)
                .and_then(|block| vanilla_block_state_support_facts(block.block_state_id))
                .is_none_or(|facts| !facts.occlusion_face_full(neighbor_face))
            {
                faces = faces.with(face);
            }
        }
        faces
    }

    /// Vanilla `TheEndGatewayBlockEntity.getParticleAmount`: one particle for
    /// each direction whose neighbouring full occlusion face does not hide the
    /// gateway. The gateway itself has no occlusion shape and does not
    /// `skipRendering`, so `Block.shouldRenderFace` reduces exactly to this
    /// neighbour-full test.
    pub fn end_gateway_particle_amount(&self, pos: BlockPos) -> Option<usize> {
        self.end_gateways
            .binary_search_by_key(&(pos.y, pos.z, pos.x), |state| {
                (state.pos.y, state.pos.z, state.pos.x)
            })
            .is_ok()
            .then(|| self.end_gateway_visible_faces(pos).iter().count())
    }

    pub fn end_portal_model_source_states(
        &self,
        partial_tick: f32,
    ) -> Vec<EndPortalModelSourceState> {
        self.end_portal_model_source_states_matching(partial_tick, |_, _| true)
    }

    pub fn end_portal_model_source_states_matching(
        &self,
        partial_tick: f32,
        include: impl Fn(BlockPos, EndPortalBlockKind) -> bool,
    ) -> Vec<EndPortalModelSourceState> {
        let mut sources = Vec::new();
        let game_time = self.world_time().map(|time| time.game_time).unwrap_or(0);
        let max_y = self.dimension.min_y + self.dimension.height;
        for source in self.end_portal_block_sources_matching(include) {
            let (faces, gateway_beam) = if source.kind == EndPortalBlockKind::EndGateway {
                let state = self
                    .end_gateways
                    .binary_search_by_key(&(source.pos.y, source.pos.z, source.pos.x), |state| {
                        (state.pos.y, state.pos.z, state.pos.x)
                    })
                    .ok()
                    .map(|index| self.end_gateways[index])
                    .unwrap_or_else(|| EndGatewayBlockState::new(source.pos, source.gateway_data));
                (
                    self.end_gateway_visible_faces(source.pos),
                    end_gateway_beam_source(state, partial_tick, game_time, max_y),
                )
            } else {
                (EndPortalFaces::Y_AXIS, None)
            };
            sources.push(EndPortalModelSourceState {
                pos: source.pos,
                kind: source.kind,
                faces,
                gateway_beam,
            });
        }
        sources.sort_by_key(|source| (source.pos.y, source.pos.z, source.pos.x));
        sources
    }

    fn end_portal_block_sources(&self) -> Vec<EndPortalBlockSource> {
        self.end_portal_block_sources_matching(|_, _| true)
    }

    fn end_portal_block_sources_matching(
        &self,
        include: impl Fn(BlockPos, EndPortalBlockKind) -> bool,
    ) -> Vec<EndPortalBlockSource> {
        let mut sources = Vec::new();
        for chunk in self.chunks.with_family(BlockFamily::EndPortal) {
            self.collect_chunk_end_portal_block_sources(chunk, &include, &mut sources);
        }
        sources
    }

    fn collect_chunk_end_portal_block_sources(
        &self,
        chunk: &ChunkColumn,
        include: &impl Fn(BlockPos, EndPortalBlockKind) -> bool,
        sources: &mut Vec<EndPortalBlockSource>,
    ) {
        for (pos, state_id) in self.chunks.family_blocks(
            chunk.pos,
            BlockFamily::EndPortal,
            self.dimension.min_section_y(),
        ) {
            let block_name = vanilla_block_state_name(state_id).unwrap();
            let kind = end_portal_kind_for_block_name(block_name).unwrap();
            if !include(pos, kind) {
                continue;
            }
            sources.push(EndPortalBlockSource {
                pos,
                kind,
                gateway_data: self.end_gateway_data_in_chunk(pos),
            });
        }
    }

    fn end_gateway_data_in_chunk(&self, pos: BlockPos) -> Option<EndGatewayBlockEntityData> {
        self.block_entity_record_at(pos)?.end_gateway()
    }
}

fn end_gateway_beam_source(
    state: EndGatewayBlockState,
    partial_tick: f32,
    game_time: i64,
    max_y: i32,
) -> Option<EndGatewayBeamSourceState> {
    let spawning = state.age < END_GATEWAY_SPAWN_TICKS;
    let cooling_down = state.teleport_cooldown > 0;
    if !spawning && !cooling_down {
        return None;
    }
    let percent = if spawning {
        ((state.age as f32 + partial_tick) / END_GATEWAY_SPAWN_TICKS as f32).clamp(0.0, 1.0)
    } else {
        1.0 - ((state.teleport_cooldown as f32 - partial_tick) / END_GATEWAY_COOLDOWN_TICKS as f32)
            .clamp(0.0, 1.0)
    };
    let scale = (percent * std::f32::consts::PI).sin();
    let beam_distance = if spawning { max_y as f32 } else { 50.0 };
    let height = (scale * beam_distance).floor() as i32;
    if height <= 0 {
        return None;
    }
    Some(EndGatewayBeamSourceState {
        scale,
        height,
        color_argb: if spawning {
            END_GATEWAY_SPAWN_COLOR
        } else {
            END_GATEWAY_COOLDOWN_COLOR
        },
        animation_time: game_time.rem_euclid(40) as f32 + partial_tick,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::PaletteKind;
    use crate::{
        ChunkPos, ChunkSection, ChunkState, LightData, PaletteDomain, PalettedContainerData,
        WorldDimension,
    };
    use bbb_protocol::packets::{
        BlockEvent as ProtocolBlockEvent, BlockPos as ProtocolBlockPos, BlockUpdate, PlayTime,
    };
    use std::collections::BTreeMap;

    fn world_with_air_chunk() -> WorldStore {
        let mut world = WorldStore::with_dimension(WorldDimension {
            min_y: 0,
            height: 256,
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

    fn set_block(world: &mut WorldStore, pos: BlockPos, name: &str) {
        let properties = BTreeMap::new();
        let state_id = world
            .registries()
            .block_state_id_by_name_and_properties(name, &properties)
            .unwrap_or_else(|| panic!("no registered state for {name}"));
        assert!(world.apply_block_update(BlockUpdate {
            pos: ProtocolBlockPos {
                x: pos.x,
                y: pos.y,
                z: pos.z,
            },
            block_state_id: state_id,
        }));
    }

    fn send_block_event(world: &mut WorldStore, pos: BlockPos, b0: u8, b1: u8) {
        world.apply_block_event(ProtocolBlockEvent {
            pos: ProtocolBlockPos {
                x: pos.x,
                y: pos.y,
                z: pos.z,
            },
            b0,
            b1,
            block_id: 0,
        });
    }

    fn gateway_age_nbt(age: i64) -> Vec<u8> {
        let mut out = Vec::new();
        out.push(10);
        out.push(4);
        write_nbt_string(&mut out, "Age");
        out.extend_from_slice(&age.to_be_bytes());
        out.push(0);
        out
    }

    fn write_nbt_string(out: &mut Vec<u8>, value: &str) {
        let bytes = value.as_bytes();
        out.extend_from_slice(&(bytes.len() as u16).to_be_bytes());
        out.extend_from_slice(bytes);
    }

    #[test]
    fn maps_end_portal_block_families() {
        assert_eq!(
            end_portal_kind_for_block_name("minecraft:end_portal"),
            Some(EndPortalBlockKind::EndPortal)
        );
        assert_eq!(
            end_portal_kind_for_block_name("minecraft:end_gateway"),
            Some(EndPortalBlockKind::EndGateway)
        );
        assert_eq!(end_portal_kind_for_block_name("minecraft:stone"), None);
    }

    #[test]
    fn projects_end_portal_y_axis_faces_and_end_gateway_six_faces() {
        let mut world = world_with_air_chunk();
        set_block(
            &mut world,
            BlockPos { x: 2, y: 3, z: 4 },
            "minecraft:end_portal",
        );
        set_block(
            &mut world,
            BlockPos { x: 5, y: 3, z: 4 },
            "minecraft:end_gateway",
        );

        let sources = world.end_portal_model_source_states(0.5);

        assert_eq!(sources.len(), 2);
        assert_eq!(sources[0].kind, EndPortalBlockKind::EndPortal);
        assert_eq!(sources[0].faces, EndPortalFaces::Y_AXIS);
        assert_eq!(sources[1].kind, EndPortalBlockKind::EndGateway);
        assert_eq!(sources[1].faces, EndPortalFaces::ALL);

        // Full-face neighbors occlude matching gateway faces (Vanilla `shouldRenderFace`).
        set_block(&mut world, BlockPos { x: 5, y: 4, z: 4 }, "minecraft:stone");
        set_block(&mut world, BlockPos { x: 5, y: 2, z: 4 }, "minecraft:stone");
        let occluded = world.end_portal_model_source_states(0.5);
        assert_eq!(
            occluded[1].faces,
            EndPortalFaces::from_faces(&[
                EndPortalFace::North,
                EndPortalFace::South,
                EndPortalFace::West,
                EndPortalFace::East,
            ])
        );
    }

    #[test]
    fn gateway_age_loads_from_nbt_and_advances_spawn_beam() {
        let mut world = world_with_air_chunk();
        let pos = BlockPos { x: 2, y: 3, z: 4 };
        set_block(&mut world, pos, "minecraft:end_gateway");
        world
            .apply_block_entity_data(bbb_protocol::packets::BlockEntityData {
                pos: ProtocolBlockPos {
                    x: pos.x,
                    y: pos.y,
                    z: pos.z,
                },
                block_entity_type_id:
                    bbb_protocol::block_entity_types::VANILLA_BLOCK_ENTITY_TYPE_END_GATEWAY_ID,
                raw_nbt: gateway_age_nbt(10),
            })
            .unwrap();
        world.apply_world_time(PlayTime {
            game_time: 43,
            clock_updates: Vec::new(),
        });
        world.advance_end_gateway_ticks(2);

        assert_eq!(world.end_gateway_states()[0].age, 12);
        let beam = world.end_portal_model_source_states(0.25)[0]
            .gateway_beam
            .unwrap();
        assert_eq!(beam.color_argb, END_GATEWAY_SPAWN_COLOR);
        assert_eq!(beam.animation_time, 3.25);
        assert!(beam.height > 0);
    }

    #[test]
    fn gateway_block_event_starts_cooldown_beam() {
        let mut world = world_with_air_chunk();
        let pos = BlockPos { x: 2, y: 3, z: 4 };
        set_block(&mut world, pos, "minecraft:end_gateway");
        world
            .apply_block_entity_data(bbb_protocol::packets::BlockEntityData {
                pos: ProtocolBlockPos {
                    x: pos.x,
                    y: pos.y,
                    z: pos.z,
                },
                block_entity_type_id:
                    bbb_protocol::block_entity_types::VANILLA_BLOCK_ENTITY_TYPE_END_GATEWAY_ID,
                raw_nbt: gateway_age_nbt(2400),
            })
            .unwrap();
        world.advance_end_gateway_ticks(0);
        send_block_event(&mut world, pos, 1, 0);
        world.advance_end_gateway_ticks(1);

        assert_eq!(world.end_gateway_states()[0].teleport_cooldown, 39);
        let beam = world.end_portal_model_source_states(0.5)[0]
            .gateway_beam
            .unwrap();
        assert_eq!(beam.color_argb, END_GATEWAY_COOLDOWN_COLOR);
        assert!(beam.height > 0);
    }

    #[test]
    fn gateway_particle_amount_counts_only_non_occluding_neighbor_faces() {
        let mut world = world_with_air_chunk();
        let pos = BlockPos { x: 2, y: 3, z: 4 };
        set_block(&mut world, pos, "minecraft:end_gateway");
        assert_eq!(world.end_gateway_particle_amount(pos), None);
        world.advance_end_gateway_ticks(0);
        assert_eq!(world.end_gateway_particle_amount(pos), Some(6));

        set_block(
            &mut world,
            BlockPos {
                y: pos.y + 1,
                ..pos
            },
            "minecraft:stone",
        );
        set_block(
            &mut world,
            BlockPos {
                z: pos.z - 1,
                ..pos
            },
            "minecraft:stone",
        );
        assert_eq!(world.end_gateway_particle_amount(pos), Some(4));

        set_block(
            &mut world,
            BlockPos {
                x: pos.x + 1,
                ..pos
            },
            "minecraft:glass",
        );
        assert_eq!(
            world.end_gateway_particle_amount(pos),
            Some(4),
            "glass has no face occlusion shape"
        );

        set_block(&mut world, pos, "minecraft:stone");
        world.advance_end_gateway_ticks(0);
        assert_eq!(world.end_gateway_particle_amount(pos), None);
    }
}
