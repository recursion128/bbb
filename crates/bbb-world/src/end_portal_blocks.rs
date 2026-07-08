//! Client-side end portal/gateway block-entity render sources.
//!
//! Vanilla renders both blocks through `AbstractEndPortalRenderer`: only the
//! Y-axis faces are submitted. End portals apply the
//! `TheEndPortalRenderer.TRANSFORMATION`; end gateways keep the unit cube and
//! may add a beacon-beam style spawn/cooldown beam from
//! `TheEndGatewayRenderer`.

use serde::{Deserialize, Serialize};

use crate::{
    chunks::EndGatewayBlockEntityData, BlockPos, ChunkColumn, PaletteKind, RegistrySet, WorldStore,
};

const END_GATEWAY_SPAWN_TICKS: i64 = 200;
const END_GATEWAY_COOLDOWN_TICKS: i32 = 40;
const END_GATEWAY_SPAWN_COLOR: u32 = 0xFFC74EBD;
const END_GATEWAY_COOLDOWN_COLOR: u32 = 0xFF8932B8;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EndPortalBlockKind {
    EndPortal,
    EndGateway,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EndPortalFace {
    Down,
    Up,
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
    pub faces: [EndPortalFace; 2],
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
    pub(crate) fn update_end_gateway_from_block_event(&mut self, pos: BlockPos, b0: u8) {
        if b0 != 1
            || self
                .probe_block(pos)
                .and_then(|block| block.block_name)
                .as_deref()
                != Some("minecraft:end_gateway")
        {
            return;
        }
        if let Some(state) = self.end_gateways.iter_mut().find(|state| state.pos == pos) {
            state.teleport_cooldown = END_GATEWAY_COOLDOWN_TICKS;
            return;
        }
        let source = self
            .end_portal_block_sources()
            .into_iter()
            .find(|source| source.pos == pos);
        let data = source.and_then(|source| source.gateway_data);
        let mut state = EndGatewayBlockState::new(pos, data);
        state.teleport_cooldown = END_GATEWAY_COOLDOWN_TICKS;
        self.end_gateways.push(state);
        self.end_gateways
            .sort_by_key(|state| (state.pos.y, state.pos.z, state.pos.x));
    }

    /// Advances `TheEndGatewayBlockEntity.beamAnimationTick`: `age++` every
    /// client tick and `teleportCooldown--` while cooling down.
    pub fn advance_end_gateway_ticks(&mut self, ticks: u32) {
        let sources = self.end_portal_block_sources();
        let gateway_sources: Vec<_> = sources
            .into_iter()
            .filter(|source| source.kind == EndPortalBlockKind::EndGateway)
            .collect();
        let mut states = std::mem::take(&mut self.end_gateways);
        states.retain(|state| gateway_sources.iter().any(|source| source.pos == state.pos));
        for source in &gateway_sources {
            if let Some(state) = states.iter_mut().find(|state| state.pos == source.pos) {
                let loaded_age = source.gateway_data.map(|data| data.age).unwrap_or(0);
                if state.loaded_age != loaded_age {
                    state.age = loaded_age;
                    state.loaded_age = loaded_age;
                }
            } else {
                states.push(EndGatewayBlockState::new(source.pos, source.gateway_data));
            }
        }

        for state in &mut states {
            for _ in 0..ticks {
                state.age = state.age.saturating_add(1);
                if state.teleport_cooldown > 0 {
                    state.teleport_cooldown -= 1;
                }
            }
        }

        states.sort_by_key(|state| (state.pos.y, state.pos.z, state.pos.x));
        self.end_gateways = states;
    }

    pub fn end_gateway_states(&self) -> &[EndGatewayBlockState] {
        &self.end_gateways
    }

    pub fn end_portal_model_source_states(
        &self,
        partial_tick: f32,
    ) -> Vec<EndPortalModelSourceState> {
        let mut sources = Vec::new();
        let game_time = self.world_time().map(|time| time.game_time).unwrap_or(0);
        let max_y = self.dimension.min_y + self.dimension.height;
        for source in self.end_portal_block_sources() {
            let gateway_beam = if source.kind == EndPortalBlockKind::EndGateway {
                let state = self
                    .end_gateways
                    .iter()
                    .find(|state| state.pos == source.pos)
                    .copied()
                    .unwrap_or_else(|| EndGatewayBlockState::new(source.pos, source.gateway_data));
                end_gateway_beam_source(state, partial_tick, game_time, max_y)
            } else {
                None
            };
            sources.push(EndPortalModelSourceState {
                pos: source.pos,
                kind: source.kind,
                faces: [EndPortalFace::Down, EndPortalFace::Up],
                gateway_beam,
            });
        }
        sources.sort_by_key(|source| (source.pos.y, source.pos.z, source.pos.x));
        sources
    }

    fn end_portal_block_sources(&self) -> Vec<EndPortalBlockSource> {
        let mut sources = Vec::new();
        for chunk in &self.chunks {
            self.collect_chunk_end_portal_block_sources(chunk, &mut sources);
        }
        sources
    }

    fn collect_chunk_end_portal_block_sources(
        &self,
        chunk: &ChunkColumn,
        sources: &mut Vec<EndPortalBlockSource>,
    ) {
        for (section_index, section) in chunk.sections.iter().enumerate() {
            if !section_palette_may_contain_end_portal(
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
                let Some(kind) = end_portal_kind_for_block_name(&block_state.name) else {
                    continue;
                };
                let pos = BlockPos {
                    x: chunk.pos.x * 16 + (index & 0xF) as i32,
                    y: section_min_y + (index >> 8) as i32,
                    z: chunk.pos.z * 16 + ((index >> 4) & 0xF) as i32,
                };
                sources.push(EndPortalBlockSource {
                    pos,
                    kind,
                    gateway_data: self.end_gateway_data_in_chunk(chunk, pos),
                });
            }
        }
    }

    fn end_gateway_data_in_chunk(
        &self,
        chunk: &ChunkColumn,
        pos: BlockPos,
    ) -> Option<EndGatewayBlockEntityData> {
        let local_x = pos.x.rem_euclid(16) as u8;
        let local_z = pos.z.rem_euclid(16) as u8;
        let y = i16::try_from(pos.y).ok()?;
        chunk
            .block_entities
            .iter()
            .find(|entity| entity.local_x == local_x && entity.local_z == local_z && entity.y == y)
            .and_then(|entity| entity.end_gateway)
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

fn section_palette_may_contain_end_portal(
    palette_global_ids: &[i32],
    palette_kind: PaletteKind,
    registries: &RegistrySet,
) -> bool {
    match palette_kind {
        PaletteKind::SingleValue | PaletteKind::Local => {
            palette_global_ids.iter().any(|global_id| {
                registries
                    .block_state(*global_id)
                    .is_some_and(|state| is_end_portal_block_name(&state.name))
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
    use bbb_protocol::packets::{
        BlockEvent as ProtocolBlockEvent, BlockPos as ProtocolBlockPos, BlockUpdate, PlayTime,
    };
    use std::collections::BTreeMap;

    const VANILLA_AIR_BLOCK_STATE_ID: i32 = 0;

    fn world_with_air_chunk() -> WorldStore {
        let mut world = WorldStore::with_dimension(WorldDimension {
            min_y: 0,
            height: 256,
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
    fn projects_end_portal_and_gateway_y_axis_faces() {
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
        assert_eq!(sources[0].faces, [EndPortalFace::Down, EndPortalFace::Up]);
        assert_eq!(sources[1].kind, EndPortalBlockKind::EndGateway);
        assert_eq!(sources[1].faces, [EndPortalFace::Down, EndPortalFace::Up]);
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
                block_entity_type_id: 0,
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
                block_entity_type_id: 0,
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
}
