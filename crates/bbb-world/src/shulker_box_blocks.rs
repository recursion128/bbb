//! Client-side shulker box lid state and the shulker box block-model render
//! source.
//!
//! Vanilla drives the shulker box lid with a per-block-entity state machine
//! (`ShulkerBoxBlockEntity.java`): the server's opener counter fires
//! `Level.blockEvent(pos, block, 1, openCount)` on every open/close
//! (`ShulkerBoxBlockEntity.startOpen`/`stopOpen`, java:174/187), the client's
//! `Level.blockEvent` dispatches the event to the block state at that
//! position, and `ShulkerBoxBlockEntity.triggerEvent(1, count)` (java:140-155)
//! sets `animationStatus = OPENING` when the count is exactly `1` and
//! `CLOSING` when it is `0` (any other count only updates the opener count).
//! Each client tick `ShulkerBoxBlockEntity.updateAnimation` (java:66-101)
//! copies `progressOld = progress` and steps `progress` by `0.1` toward the
//! status target, latching `OPENED`/`CLOSED` at the clamps. bbb has no
//! per-position block-entity objects, so the same state machine lives here as
//! a flat `Vec<ShulkerBoxLidState>` on the `WorldStore`, keyed by block
//! position and fed by `apply_block_event`.
//!
//! The render projection (`shulker_box_model_source_states`) enumerates
//! shulker box blocks straight from the chunk block states — mirroring how
//! the vanilla client materialises a `ShulkerBoxBlockEntity` per shulker box
//! block state — with `ShulkerBoxRenderer.extractRenderState` fields: the
//! six-way `FACING`, the block-id dye color (`ShulkerBoxBlock.getColor`), and
//! `getProgress(partialTicks) = lerp(partial, progressOld, progress)`.

use serde::{Deserialize, Serialize};

use crate::{BlockPos, ChunkColumn, PaletteKind, RegistrySet, WorldStore};

/// Vanilla `ShulkerBoxBlockEntity.updateAnimation` per-tick progress step
/// (`0.1F`, `OPENING_TICK_LENGTH = 10`).
const VANILLA_SHULKER_BOX_PROGRESS_STEP: f32 = 0.1;
/// `updateAnimation` saturates within 10 steps and `progressOld` catches up
/// one tick later, so batching more than 11 ticks is indistinguishable from 11.
const SHULKER_BOX_SATURATION_TICKS: u32 = 11;

/// Vanilla `DyeColor` in id order: the sixteen `minecraft:<color>_shulker_box`
/// blocks. The undyed `minecraft:shulker_box` carries no color (vanilla
/// `ShulkerBoxBlockEntity.color == null` selects the default
/// `entity/shulker/shulker` sprite).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ShulkerBoxColorKind {
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

impl ShulkerBoxColorKind {
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

/// The shulker box block state's six-way `facing` property
/// (`ShulkerBoxBlock.FACING = DirectionalBlock.FACING`, default `UP` —
/// `ShulkerBoxBlock.java:48/60`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ShulkerBoxFacing {
    Down,
    Up,
    North,
    South,
    West,
    East,
}

impl ShulkerBoxFacing {
    fn parse(value: &str) -> Option<Self> {
        match value {
            "down" => Some(Self::Down),
            "up" => Some(Self::Up),
            "north" => Some(Self::North),
            "south" => Some(Self::South),
            "west" => Some(Self::West),
            "east" => Some(Self::East),
            _ => None,
        }
    }
}

/// Vanilla `ShulkerBoxBlockEntity.AnimationStatus`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ShulkerBoxAnimationStatus {
    Closed,
    Opening,
    Opened,
    Closing,
}

/// One shulker box's `ShulkerBoxBlockEntity` lid state (`animationStatus` /
/// `progress` / `progressOld`), keyed by the block position.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ShulkerBoxLidState {
    pub pos: BlockPos,
    pub status: ShulkerBoxAnimationStatus,
    pub progress: f32,
    pub o_progress: f32,
}

/// One shulker box block's per-frame render source: the vanilla
/// `ShulkerBoxRenderState` fields except light (sampled on the projection
/// side, like entity light).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ShulkerBoxModelSourceState {
    pub pos: BlockPos,
    /// Vanilla `ShulkerBoxRenderState.color = blockEntity.getColor()` — the
    /// block id's dye color, `None` for the undyed `minecraft:shulker_box`.
    pub color: Option<ShulkerBoxColorKind>,
    /// Vanilla `ShulkerBoxRenderState.direction`
    /// (`state.getValueOrElse(FACING, Direction.UP)`).
    pub facing: ShulkerBoxFacing,
    /// Vanilla `ShulkerBoxRenderState.progress =
    /// blockEntity.getProgress(partialTicks)` — the raw `0..=1` lid progress
    /// the renderer turns into the lid lift/twist
    /// (`ShulkerBoxModel.setupAnim`).
    pub progress: f32,
}

/// Maps a shulker-box-family block name to its dye color: `Some(None)` for
/// the undyed `minecraft:shulker_box`, `Some(Some(color))` for the sixteen
/// `minecraft:<color>_shulker_box` blocks, `None` for every other block.
pub fn shulker_box_color_for_block_name(block_name: &str) -> Option<Option<ShulkerBoxColorKind>> {
    let name = block_name.strip_prefix("minecraft:")?;
    if name == "shulker_box" {
        return Some(None);
    }
    ShulkerBoxColorKind::parse(name.strip_suffix("_shulker_box")?).map(Some)
}

impl WorldStore {
    /// Applies a `BlockEvent` to the shulker box lid tracker, transcribing
    /// the client dispatch chain `Level.blockEvent` ->
    /// `BaseEntityBlock.triggerEvent` -> `ShulkerBoxBlockEntity.triggerEvent`
    /// (java:140-155): only event id `1` (`EVENT_SET_OPEN_COUNT`) on a block
    /// position whose *current* block state is a shulker box reaches the
    /// block entity; a count of exactly `1` starts `OPENING`, a count of `0`
    /// starts `CLOSING`, and any other count only updates the opener count
    /// (no animation change).
    pub(crate) fn update_shulker_box_lid_from_block_event(
        &mut self,
        pos: BlockPos,
        b0: u8,
        b1: u8,
    ) {
        if b0 != 1 {
            return;
        }
        let is_shulker_box = self
            .probe_block(pos)
            .and_then(|probe| probe.block_name)
            .as_deref()
            .and_then(shulker_box_color_for_block_name)
            .is_some();
        if !is_shulker_box {
            return;
        }
        let status = match b1 {
            0 => ShulkerBoxAnimationStatus::Closing,
            1 => ShulkerBoxAnimationStatus::Opening,
            _ => return,
        };
        if let Some(lid) = self.shulker_box_lids.iter_mut().find(|lid| lid.pos == pos) {
            lid.status = status;
        } else {
            self.shulker_box_lids.push(ShulkerBoxLidState {
                pos,
                status,
                progress: 0.0,
                o_progress: 0.0,
            });
        }
    }

    /// Advances every tracked shulker box lid by `ticks` client ticks,
    /// transcribing `ShulkerBoxBlockEntity.updateAnimation` (java:66-101):
    /// `progressOld = progress`, then step `0.1` toward the status target —
    /// `OPENING` latches `OPENED` at `1.0`, `CLOSING` latches `CLOSED` at
    /// `0.0` (the neighbour updates and collided-entity pushes on those
    /// transitions are server/physics side and not modelled). Entries whose
    /// block is no longer a shulker box (destroyed or unloaded — vanilla
    /// drops the block entity with the block/chunk) and entries fully at rest
    /// closed are pruned so the tracker only holds animating or open lids.
    pub fn advance_shulker_box_lid_ticks(&mut self, ticks: u32) {
        if ticks == 0 || self.shulker_box_lids.is_empty() {
            return;
        }
        let mut lids = std::mem::take(&mut self.shulker_box_lids);
        lids.retain(|lid| {
            self.probe_block(lid.pos)
                .and_then(|probe| probe.block_name)
                .as_deref()
                .and_then(shulker_box_color_for_block_name)
                .is_some()
        });
        let steps = ticks.min(SHULKER_BOX_SATURATION_TICKS);
        for lid in &mut lids {
            for _ in 0..steps {
                lid.o_progress = lid.progress;
                match lid.status {
                    ShulkerBoxAnimationStatus::Closed => lid.progress = 0.0,
                    ShulkerBoxAnimationStatus::Opened => lid.progress = 1.0,
                    ShulkerBoxAnimationStatus::Opening => {
                        lid.progress += VANILLA_SHULKER_BOX_PROGRESS_STEP;
                        if lid.progress >= 1.0 {
                            lid.status = ShulkerBoxAnimationStatus::Opened;
                            lid.progress = 1.0;
                        }
                    }
                    ShulkerBoxAnimationStatus::Closing => {
                        lid.progress -= VANILLA_SHULKER_BOX_PROGRESS_STEP;
                        if lid.progress <= 0.0 {
                            lid.status = ShulkerBoxAnimationStatus::Closed;
                            lid.progress = 0.0;
                        }
                    }
                }
            }
        }
        lids.retain(|lid| {
            lid.status != ShulkerBoxAnimationStatus::Closed
                || lid.progress > 0.0
                || lid.o_progress > 0.0
        });
        self.shulker_box_lids = lids;
    }

    /// Vanilla `ShulkerBoxBlockEntity.getProgress(partialTick)`:
    /// `Mth.lerp(a, progressOld, progress)`. `0.0` for untracked positions (a
    /// shulker box that never received an open-count event rests closed).
    pub fn shulker_box_progress_at(&self, pos: BlockPos, partial_tick: f32) -> f32 {
        self.shulker_box_lids
            .iter()
            .find(|lid| lid.pos == pos)
            .map(|lid| lid.o_progress + (lid.progress - lid.o_progress) * partial_tick)
            .unwrap_or(0.0)
    }

    pub fn shulker_box_lid_states(&self) -> &[ShulkerBoxLidState] {
        &self.shulker_box_lids
    }

    /// Enumerates every shulker box block in the loaded chunks as a render
    /// source, deriving color/facing from the block state (the vanilla client
    /// materialises a `ShulkerBoxBlockEntity` per shulker box block state;
    /// the color it carries is the block id's dye color) and the per-frame
    /// progress from the lid tracker. Sections whose block palette holds no
    /// shulker box state are skipped wholesale, mirroring the chest/sign
    /// palette pre-check. Sorted by position for a deterministic frame order.
    pub fn shulker_box_model_source_states(
        &self,
        partial_tick: f32,
    ) -> Vec<ShulkerBoxModelSourceState> {
        let mut states = Vec::new();
        for chunk in &self.chunks {
            self.collect_chunk_shulker_box_model_source_states(chunk, partial_tick, &mut states);
        }
        states.sort_by_key(|state| (state.pos.y, state.pos.z, state.pos.x));
        states
    }

    fn collect_chunk_shulker_box_model_source_states(
        &self,
        chunk: &ChunkColumn,
        partial_tick: f32,
        states: &mut Vec<ShulkerBoxModelSourceState>,
    ) {
        for (section_index, section) in chunk.sections.iter().enumerate() {
            if !section_palette_may_contain_shulker_box(
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
                let Some(color) = shulker_box_color_for_block_name(&block_state.name) else {
                    continue;
                };
                let pos = BlockPos {
                    x: chunk.pos.x * 16 + (index & 0xF) as i32,
                    y: section_min_y + (index >> 8) as i32,
                    z: chunk.pos.z * 16 + ((index >> 4) & 0xF) as i32,
                };
                // Vanilla `getValueOrElse(ShulkerBoxBlock.FACING, Direction.UP)`.
                let facing = block_state
                    .properties
                    .get("facing")
                    .and_then(|value| ShulkerBoxFacing::parse(value))
                    .unwrap_or(ShulkerBoxFacing::Up);
                states.push(ShulkerBoxModelSourceState {
                    pos,
                    color,
                    facing,
                    progress: self.shulker_box_progress_at(pos, partial_tick),
                });
            }
        }
    }
}

/// Whether a section's block palette can hold a shulker-box state. Local and
/// single-value palettes are answered from the palette id list; a global
/// palette stores raw state ids, so it must be scanned.
fn section_palette_may_contain_shulker_box(
    palette_global_ids: &[i32],
    palette_kind: PaletteKind,
    registries: &RegistrySet,
) -> bool {
    match palette_kind {
        PaletteKind::SingleValue | PaletteKind::Local => {
            palette_global_ids.iter().any(|global_id| {
                registries
                    .block_state(*global_id)
                    .is_some_and(|state| shulker_box_color_for_block_name(&state.name).is_some())
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

    fn set_shulker_box(world: &mut WorldStore, pos: BlockPos, name: &str, facing: &str) {
        set_block(world, pos, name, &[("facing", facing)]);
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
    fn maps_shulker_box_block_names_to_colors() {
        assert_eq!(
            shulker_box_color_for_block_name("minecraft:shulker_box"),
            Some(None)
        );
        assert_eq!(
            shulker_box_color_for_block_name("minecraft:light_blue_shulker_box"),
            Some(Some(ShulkerBoxColorKind::LightBlue))
        );
        assert_eq!(
            shulker_box_color_for_block_name("minecraft:black_shulker_box"),
            Some(Some(ShulkerBoxColorKind::Black))
        );
        // Non-shulker blocks (including the shulker mob's spawn egg items and
        // chests) stay out.
        assert_eq!(shulker_box_color_for_block_name("minecraft:chest"), None);
        assert_eq!(
            shulker_box_color_for_block_name("minecraft:stone_shulker_box"),
            None
        );
    }

    #[test]
    fn block_event_opens_and_ticks_the_lid_like_vanilla() {
        let mut world = world_with_air_chunk();
        let pos = BlockPos { x: 3, y: 4, z: 5 };
        set_shulker_box(&mut world, pos, "minecraft:shulker_box", "up");
        // ShulkerBoxBlockEntity.triggerEvent(1, 1): animationStatus = OPENING.
        send_block_event(&mut world, pos, 1, 1);
        assert_eq!(
            world.shulker_box_lid_states(),
            &[ShulkerBoxLidState {
                pos,
                status: ShulkerBoxAnimationStatus::Opening,
                progress: 0.0,
                o_progress: 0.0,
            }]
        );
        // updateAnimation: progressOld trails, progress steps 0.1.
        world.advance_shulker_box_lid_ticks(1);
        let lid = world.shulker_box_lid_states()[0];
        assert!((lid.progress - 0.1).abs() < 1e-6);
        assert_eq!(lid.o_progress, 0.0);
        // getProgress(0.5) = lerp(0.5, 0.0, 0.1).
        assert!((world.shulker_box_progress_at(pos, 0.5) - 0.05).abs() < 1e-6);
        // Saturates at 1.0 and latches OPENED regardless of extra batched ticks.
        world.advance_shulker_box_lid_ticks(100);
        let lid = world.shulker_box_lid_states()[0];
        assert_eq!(lid.status, ShulkerBoxAnimationStatus::Opened);
        assert_eq!(lid.progress, 1.0);
        assert_eq!(lid.o_progress, 1.0);
        // Close: count 0 -> CLOSING steps back down and the resting-closed
        // entry prunes.
        send_block_event(&mut world, pos, 1, 0);
        world.advance_shulker_box_lid_ticks(1);
        let lid = world.shulker_box_lid_states()[0];
        assert_eq!(lid.status, ShulkerBoxAnimationStatus::Closing);
        assert!((lid.progress - 0.9).abs() < 1e-6);
        assert_eq!(lid.o_progress, 1.0);
        world.advance_shulker_box_lid_ticks(100);
        assert!(world.shulker_box_lid_states().is_empty());
        assert_eq!(world.shulker_box_progress_at(pos, 0.5), 0.0);
    }

    #[test]
    fn open_count_above_one_changes_no_animation_status() {
        let mut world = world_with_air_chunk();
        let pos = BlockPos { x: 3, y: 4, z: 5 };
        set_shulker_box(&mut world, pos, "minecraft:red_shulker_box", "up");
        // triggerEvent(1, 2) on a closed box only records the opener count
        // (vanilla `if (b1 == 0) … if (b1 == 1) …` matches neither).
        send_block_event(&mut world, pos, 1, 2);
        assert!(world.shulker_box_lid_states().is_empty());
        // A second opener while OPENING keeps the animation running.
        send_block_event(&mut world, pos, 1, 1);
        world.advance_shulker_box_lid_ticks(3);
        send_block_event(&mut world, pos, 1, 2);
        let lid = world.shulker_box_lid_states()[0];
        assert_eq!(lid.status, ShulkerBoxAnimationStatus::Opening);
        assert!((lid.progress - 0.3).abs() < 1e-6);
    }

    #[test]
    fn block_event_ignores_non_shulker_boxes_and_other_event_ids() {
        let mut world = world_with_air_chunk();
        let box_pos = BlockPos { x: 3, y: 4, z: 5 };
        set_shulker_box(&mut world, box_pos, "minecraft:shulker_box", "up");
        // Wrong event id on a shulker box (only EVENT_SET_OPEN_COUNT = 1
        // reaches the block entity).
        send_block_event(&mut world, box_pos, 2, 1);
        // Event id 1 on a non-shulker position (Level.blockEvent dispatches on
        // the current block state, which is air here).
        send_block_event(&mut world, BlockPos { x: 1, y: 1, z: 1 }, 1, 1);
        assert!(world.shulker_box_lid_states().is_empty());
    }

    #[test]
    fn destroyed_shulker_box_lid_state_prunes_on_tick() {
        let mut world = world_with_air_chunk();
        let pos = BlockPos { x: 3, y: 4, z: 5 };
        set_shulker_box(&mut world, pos, "minecraft:shulker_box", "up");
        send_block_event(&mut world, pos, 1, 1);
        world.advance_shulker_box_lid_ticks(2);
        assert_eq!(world.shulker_box_lid_states().len(), 1);
        set_block(&mut world, pos, "minecraft:air", &[]);
        world.advance_shulker_box_lid_ticks(1);
        assert!(world.shulker_box_lid_states().is_empty());
    }

    #[test]
    fn enumerates_shulker_box_sources_with_color_facing_and_progress() {
        let mut world = world_with_air_chunk();
        let opening_pos = BlockPos { x: 3, y: 4, z: 5 };
        let resting_pos = BlockPos { x: 6, y: 4, z: 5 };
        set_shulker_box(&mut world, opening_pos, "minecraft:shulker_box", "north");
        set_shulker_box(
            &mut world,
            resting_pos,
            "minecraft:lime_shulker_box",
            "down",
        );
        send_block_event(&mut world, opening_pos, 1, 1);
        world.advance_shulker_box_lid_ticks(2);

        let sources = world.shulker_box_model_source_states(0.5);
        assert_eq!(sources.len(), 2);
        // getProgress(0.5) = lerp(0.5, 0.1, 0.2).
        assert_eq!(
            sources[0],
            ShulkerBoxModelSourceState {
                pos: opening_pos,
                color: None,
                facing: ShulkerBoxFacing::North,
                progress: 0.1 + (0.2 - 0.1) * 0.5,
            }
        );
        assert_eq!(
            sources[1],
            ShulkerBoxModelSourceState {
                pos: resting_pos,
                color: Some(ShulkerBoxColorKind::Lime),
                facing: ShulkerBoxFacing::Down,
                progress: 0.0,
            }
        );
    }

    #[test]
    fn login_clears_shulker_box_lid_states() {
        let mut world = world_with_air_chunk();
        let pos = BlockPos { x: 3, y: 4, z: 5 };
        set_shulker_box(&mut world, pos, "minecraft:shulker_box", "up");
        send_block_event(&mut world, pos, 1, 1);
        assert_eq!(world.shulker_box_lid_states().len(), 1);
        world.apply_login(&login_packet());
        assert!(world.shulker_box_lid_states().is_empty());
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
