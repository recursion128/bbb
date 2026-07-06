//! Client-side bell shake state and the bell block-model render source.
//!
//! Vanilla drives the bell swing with per-block-entity fields
//! (`BellBlockEntity.ticks` / `shaking` / `clickDirection`):
//! `BellBlock.attemptToRing` calls `BellBlockEntity.onHit`, which fires
//! `Level.blockEvent(pos, block, 1, clickDirection.get3DDataValue())`; the
//! client's `Level.blockEvent` dispatches the event to the block state at
//! that position and `BellBlockEntity.triggerEvent(1, b1)` sets
//! `clickDirection = Direction.from3DDataValue(b1)`, `ticks = 0`,
//! `shaking = true` (`BellBlockEntity.java:42-54`). Each client tick
//! (`BellBlockEntity.tick`, `java:56-66`) increments `ticks` while shaking
//! and stops at `DURATION = 50` (`shaking = false`, `ticks = 0`). bbb has no
//! per-position block-entity objects, so the same state machine lives here
//! as a flat `Vec<BellShakeState>` on the `WorldStore`, keyed by block
//! position and fed by `apply_block_event`.
//!
//! The render projection (`bell_model_source_states`) enumerates bell blocks
//! straight from the chunk block states; `BellRenderer.extractRenderState`
//! reads `ticks + partialTicks` and `shaking ? clickDirection : null`.

use serde::{Deserialize, Serialize};

use crate::{BlockPos, ChunkColumn, PaletteKind, RegistrySet, WorldStore};

/// Vanilla `BellBlockEntity.DURATION` (`50`): the tick at which the shake
/// stops and the counter resets.
const VANILLA_BELL_SHAKE_DURATION_TICKS: u32 = 50;

/// Vanilla `Direction` in `get3DDataValue()` order, as decoded by
/// `Direction.from3DDataValue(b1)` from the `BlockEvent(1, direction)`
/// payload. `Down`/`Up` are wire-representable; `BellModel.setupAnim` swings
/// only for the four horizontal directions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BellShakeDirectionKind {
    Down,
    Up,
    North,
    South,
    East,
    West,
}

impl BellShakeDirectionKind {
    /// Vanilla `Direction.from3DDataValue(i)` (`VALUES[Mth.abs(i % VALUES.length)]`).
    pub fn from_3d_data_value(value: u8) -> Self {
        match value % 6 {
            0 => Self::Down,
            1 => Self::Up,
            2 => Self::North,
            3 => Self::South,
            4 => Self::West,
            _ => Self::East,
        }
    }
}

/// One bell's `BellBlockEntity` shake state (`ticks` / `shaking` /
/// `clickDirection`), keyed by the bell block position. Only shaking bells
/// are tracked — a resting bell has `ticks = 0` like an untracked one.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct BellShakeState {
    pub pos: BlockPos,
    pub ticks: u32,
    pub shaking: bool,
    pub direction: BellShakeDirectionKind,
}

/// One bell block's per-frame render source: the vanilla
/// `BellRenderState` fields except light (sampled on the projection side).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BellModelSourceState {
    pub pos: BlockPos,
    /// Vanilla `BellRenderState.ticks = blockEntity.ticks + partialTicks`.
    pub ticks: f32,
    /// Vanilla `BellRenderState.shakeDirection = shaking ? clickDirection :
    /// null`.
    pub shake_direction: Option<BellShakeDirectionKind>,
}

/// Whether a block name is the bell block (`minecraft:bell` — the one
/// `BellBlock` registration; the attachment/facing variants are block state
/// properties, and only the block-model support frame reads them).
pub fn is_bell_block_name(block_name: &str) -> bool {
    block_name == "minecraft:bell"
}

impl WorldStore {
    /// Applies a `BlockEvent` to the bell shake tracker, transcribing the
    /// client dispatch chain `Level.blockEvent` -> `BaseEntityBlock.triggerEvent`
    /// -> `BellBlockEntity.triggerEvent`: only event id `1` on a block
    /// position whose *current* block state is a bell reaches the block
    /// entity, which restarts the shake (`clickDirection = from3DDataValue(b1)`,
    /// `ticks = 0`, `shaking = true` — re-ringing a shaking bell resets the
    /// counter).
    pub(crate) fn update_bell_shake_from_block_event(&mut self, pos: BlockPos, b0: u8, b1: u8) {
        if b0 != 1 {
            return;
        }
        let is_bell = self
            .probe_block(pos)
            .and_then(|probe| probe.block_name)
            .as_deref()
            .is_some_and(is_bell_block_name);
        if !is_bell {
            return;
        }
        let direction = BellShakeDirectionKind::from_3d_data_value(b1);
        if let Some(shake) = self.bell_shakes.iter_mut().find(|shake| shake.pos == pos) {
            shake.direction = direction;
            shake.ticks = 0;
            shake.shaking = true;
        } else {
            self.bell_shakes.push(BellShakeState {
                pos,
                ticks: 0,
                shaking: true,
                direction,
            });
        }
    }

    /// Advances every tracked bell shake by `ticks` client ticks,
    /// transcribing `BellBlockEntity.tick`: `if (shaking) ticks++;
    /// if (ticks >= 50) { shaking = false; ticks = 0; }`. Entries whose block
    /// is no longer a bell (destroyed or unloaded — vanilla drops the block
    /// entity with the block/chunk) and finished shakes are pruned, so the
    /// tracker only holds shaking bells.
    pub fn advance_bell_shake_ticks(&mut self, ticks: u32) {
        if ticks == 0 || self.bell_shakes.is_empty() {
            return;
        }
        let mut shakes = std::mem::take(&mut self.bell_shakes);
        shakes.retain(|shake| {
            self.probe_block(shake.pos)
                .and_then(|probe| probe.block_name)
                .as_deref()
                .is_some_and(is_bell_block_name)
        });
        // A shake ends after DURATION steps; batching more is indistinguishable.
        let steps = ticks.min(VANILLA_BELL_SHAKE_DURATION_TICKS);
        for shake in &mut shakes {
            for _ in 0..steps {
                if shake.shaking {
                    shake.ticks += 1;
                }
                if shake.ticks >= VANILLA_BELL_SHAKE_DURATION_TICKS {
                    shake.shaking = false;
                    shake.ticks = 0;
                }
            }
        }
        shakes.retain(|shake| shake.shaking);
        self.bell_shakes = shakes;
    }

    pub fn bell_shake_states(&self) -> &[BellShakeState] {
        &self.bell_shakes
    }

    /// Enumerates every bell block in the loaded chunks as a render source,
    /// folding in the shake tracker with the vanilla `extractRenderState`
    /// projection (`ticks + partialTicks`, direction only while shaking; an
    /// untracked bell rests at `ticks = 0` like a fresh block entity).
    /// Sections whose block palette holds no bell state are skipped
    /// wholesale, mirroring the chest/sign palette pre-check. Sorted by
    /// position for a deterministic frame order.
    pub fn bell_model_source_states(&self, partial_tick: f32) -> Vec<BellModelSourceState> {
        let mut states = Vec::new();
        for chunk in &self.chunks {
            self.collect_chunk_bell_model_source_states(chunk, partial_tick, &mut states);
        }
        states.sort_by_key(|state| (state.pos.y, state.pos.z, state.pos.x));
        states
    }

    fn collect_chunk_bell_model_source_states(
        &self,
        chunk: &ChunkColumn,
        partial_tick: f32,
        states: &mut Vec<BellModelSourceState>,
    ) {
        for (section_index, section) in chunk.sections.iter().enumerate() {
            if !section_palette_may_contain_bell(
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
                if !is_bell_block_name(&block_state.name) {
                    continue;
                }
                let pos = BlockPos {
                    x: chunk.pos.x * 16 + (index & 0xF) as i32,
                    y: section_min_y + (index >> 8) as i32,
                    z: chunk.pos.z * 16 + ((index >> 4) & 0xF) as i32,
                };
                let shake = self.bell_shakes.iter().find(|shake| shake.pos == pos);
                states.push(BellModelSourceState {
                    pos,
                    ticks: shake.map_or(0, |shake| shake.ticks) as f32 + partial_tick,
                    shake_direction: shake
                        .filter(|shake| shake.shaking)
                        .map(|shake| shake.direction),
                });
            }
        }
    }
}

/// Whether a section's block palette can hold the bell state. Local and
/// single-value palettes are answered from the palette id list; a global
/// palette stores raw state ids, so it must be scanned.
fn section_palette_may_contain_bell(
    palette_global_ids: &[i32],
    palette_kind: PaletteKind,
    registries: &RegistrySet,
) -> bool {
    match palette_kind {
        PaletteKind::SingleValue | PaletteKind::Local => {
            palette_global_ids.iter().any(|global_id| {
                registries
                    .block_state(*global_id)
                    .is_some_and(|state| is_bell_block_name(&state.name))
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
        BlockEvent as ProtocolBlockEvent, BlockPos as ProtocolBlockPos, BlockUpdate,
    };
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

    fn set_bell(world: &mut WorldStore, pos: BlockPos) {
        set_block(
            world,
            pos,
            "minecraft:bell",
            &[
                ("attachment", "floor"),
                ("facing", "north"),
                ("powered", "false"),
            ],
        );
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

    #[test]
    fn block_event_starts_and_ticks_the_shake_like_vanilla() {
        let mut world = world_with_air_chunk();
        let pos = BlockPos { x: 3, y: 4, z: 5 };
        set_bell(&mut world, pos);
        // BellBlockEntity.triggerEvent(1, direction): from3DDataValue(2) = NORTH,
        // ticks = 0, shaking = true.
        send_block_event(&mut world, pos, 1, 2);
        assert_eq!(
            world.bell_shake_states(),
            &[BellShakeState {
                pos,
                ticks: 0,
                shaking: true,
                direction: BellShakeDirectionKind::North,
            }]
        );
        // tick(): `if (shaking) ticks++`.
        world.advance_bell_shake_ticks(1);
        assert_eq!(world.bell_shake_states()[0].ticks, 1);
        world.advance_bell_shake_ticks(48);
        assert_eq!(world.bell_shake_states()[0].ticks, 49);
        assert!(world.bell_shake_states()[0].shaking);
        // The 50th tick hits DURATION: shaking = false, ticks = 0 -> pruned.
        world.advance_bell_shake_ticks(1);
        assert!(world.bell_shake_states().is_empty());
    }

    #[test]
    fn re_ring_resets_the_counter_and_updates_the_direction() {
        let mut world = world_with_air_chunk();
        let pos = BlockPos { x: 3, y: 4, z: 5 };
        set_bell(&mut world, pos);
        send_block_event(&mut world, pos, 1, 2);
        world.advance_bell_shake_ticks(20);
        assert_eq!(world.bell_shake_states()[0].ticks, 20);
        // Ringing again mid-shake restarts at 0 with the new click direction
        // (from3DDataValue(4) = WEST).
        send_block_event(&mut world, pos, 1, 4);
        assert_eq!(
            world.bell_shake_states(),
            &[BellShakeState {
                pos,
                ticks: 0,
                shaking: true,
                direction: BellShakeDirectionKind::West,
            }]
        );
    }

    #[test]
    fn block_event_ignores_non_bells_and_other_event_ids() {
        let mut world = world_with_air_chunk();
        let bell_pos = BlockPos { x: 3, y: 4, z: 5 };
        set_bell(&mut world, bell_pos);
        // Wrong event id on a bell.
        send_block_event(&mut world, bell_pos, 2, 2);
        // Event id 1 on a non-bell position (Level.blockEvent dispatches on
        // the current block state, which is air here).
        send_block_event(&mut world, BlockPos { x: 1, y: 1, z: 1 }, 1, 2);
        assert!(world.bell_shake_states().is_empty());
    }

    #[test]
    fn destroyed_bell_shake_state_prunes_on_tick() {
        let mut world = world_with_air_chunk();
        let pos = BlockPos { x: 3, y: 4, z: 5 };
        set_bell(&mut world, pos);
        send_block_event(&mut world, pos, 1, 3);
        world.advance_bell_shake_ticks(2);
        assert_eq!(world.bell_shake_states().len(), 1);
        set_block(&mut world, pos, "minecraft:air", &[]);
        world.advance_bell_shake_ticks(1);
        assert!(world.bell_shake_states().is_empty());
    }

    #[test]
    fn enumerates_bell_sources_with_partial_ticks_and_shake_direction() {
        let mut world = world_with_air_chunk();
        let ringing_pos = BlockPos { x: 3, y: 4, z: 5 };
        let resting_pos = BlockPos { x: 6, y: 4, z: 5 };
        set_bell(&mut world, ringing_pos);
        set_bell(&mut world, resting_pos);
        // from3DDataValue(5) = EAST.
        send_block_event(&mut world, ringing_pos, 1, 5);
        world.advance_bell_shake_ticks(10);

        let sources = world.bell_model_source_states(0.5);
        assert_eq!(sources.len(), 2);
        // BellRenderState.ticks = ticks + partialTicks; direction while shaking.
        assert_eq!(
            sources[0],
            BellModelSourceState {
                pos: ringing_pos,
                ticks: 10.5,
                shake_direction: Some(BellShakeDirectionKind::East),
            }
        );
        // A resting bell projects ticks = partialTicks with no direction.
        assert_eq!(
            sources[1],
            BellModelSourceState {
                pos: resting_pos,
                ticks: 0.5,
                shake_direction: None,
            }
        );
    }

    #[test]
    fn direction_data_values_match_vanilla_from_3d_data_value() {
        // Direction.VALUES order: DOWN, UP, NORTH, SOUTH, WEST, EAST.
        assert_eq!(
            BellShakeDirectionKind::from_3d_data_value(0),
            BellShakeDirectionKind::Down
        );
        assert_eq!(
            BellShakeDirectionKind::from_3d_data_value(1),
            BellShakeDirectionKind::Up
        );
        assert_eq!(
            BellShakeDirectionKind::from_3d_data_value(2),
            BellShakeDirectionKind::North
        );
        assert_eq!(
            BellShakeDirectionKind::from_3d_data_value(3),
            BellShakeDirectionKind::South
        );
        assert_eq!(
            BellShakeDirectionKind::from_3d_data_value(4),
            BellShakeDirectionKind::West
        );
        assert_eq!(
            BellShakeDirectionKind::from_3d_data_value(5),
            BellShakeDirectionKind::East
        );
        // Out-of-range payloads fold through the modulo like vanilla.
        assert_eq!(
            BellShakeDirectionKind::from_3d_data_value(6),
            BellShakeDirectionKind::Down
        );
    }

    #[test]
    fn login_clears_bell_shake_states() {
        let mut world = world_with_air_chunk();
        let pos = BlockPos { x: 3, y: 4, z: 5 };
        set_bell(&mut world, pos);
        send_block_event(&mut world, pos, 1, 2);
        assert_eq!(world.bell_shake_states().len(), 1);
        world.apply_login(&login_packet());
        assert!(world.bell_shake_states().is_empty());
    }

    fn login_packet() -> bbb_protocol::packets::PlayLogin {
        bbb_protocol::packets::PlayLogin {
            player_id: 42,
            hardcore: false,
            levels: vec!["minecraft:overworld".to_string()],
            max_players: 20,
            chunk_radius: 8,
            simulation_distance: 6,
            reduced_debug_info: false,
            show_death_screen: true,
            do_limited_crafting: false,
            common_spawn_info: bbb_protocol::packets::CommonPlayerSpawnInfo {
                dimension_type_id: 0,
                dimension: "minecraft:overworld".to_string(),
                seed: 1,
                game_type: 1,
                previous_game_type: -1,
                is_debug: false,
                is_flat: false,
                last_death_location: None,
                portal_cooldown: 0,
                sea_level: 63,
            },
            enforces_secure_chat: false,
        }
    }
}
