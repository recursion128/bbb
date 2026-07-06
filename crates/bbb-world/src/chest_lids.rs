//! Client-side chest lid state and the chest block-model render source.
//!
//! Vanilla drives chest lids with a per-block-entity `ChestLidController`
//! (`ChestLidController.java`): the server's `ContainerOpenersCounter` fires
//! `Level.blockEvent(pos, block, 1, openCount)` whenever the opener count
//! changes (`ChestBlockEntity.signalOpenCount`), the client's
//! `Level.blockEvent` (`Level.java:901-903`) dispatches the event to the block
//! state at that position, `BaseEntityBlock.triggerEvent` forwards it to the
//! block entity, and `ChestBlockEntity.triggerEvent(1, count)` sets
//! `shouldBeOpen(count > 0)`. Each client tick `ChestBlockEntity.lidAnimateTick`
//! steps the controller by `0.1` toward the target. bbb has no per-position
//! block-entity objects, so the same state machine lives here as a flat
//! `Vec<ChestLidState>` on the `WorldStore`, keyed by block position and fed by
//! `apply_block_event`.
//!
//! The render projection (`chest_model_source_states`) enumerates chest-family
//! blocks straight from the chunk block states — mirroring how the vanilla
//! client materialises a `ChestBlockEntity` for every chest block state in a
//! loaded chunk — and folds the double-chest neighbour combine
//! (`ChestBlock.combine` / `ChestBlock.opennessCombiner`: the shared openness
//! is the max of both halves' lerped lid openness).

use serde::{Deserialize, Serialize};

use crate::{BlockPos, ChunkColumn, PaletteKind, RegistrySet, WorldStore};

/// Vanilla `ChestLidController.tickLid` per-tick openness step (`0.1F`).
const VANILLA_CHEST_LID_OPENNESS_STEP: f32 = 0.1;
/// `tickLid` saturates within 10 steps and `o_openness` catches up one tick
/// later, so batching more than 11 ticks is indistinguishable from 11.
const CHEST_LID_SATURATION_TICKS: u32 = 11;

/// One chest lid's `ChestLidController` state (`shouldBeOpen` /
/// `openness` / `oOpenness`), keyed by the chest block position.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ChestLidState {
    pub pos: BlockPos,
    pub should_be_open: bool,
    pub openness: f32,
    pub o_openness: f32,
}

/// Vanilla `ChestRenderState.ChestMaterialType` projected from the block name
/// (`ChestRenderer.getChestMaterial`): the texture family the chest model
/// renders with. The waxed copper chests share their weathering stage's
/// texture. The christmas swap (`SpecialDates.isExtendedChristmas`, Dec 24-26)
/// is not modelled — bbb has no wall-clock input in the render state chain —
/// so normal chests always use [`ChestModelKind::Normal`] (ledgered).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ChestModelKind {
    Normal,
    Trapped,
    Ender,
    Copper,
    CopperExposed,
    CopperWeathered,
    CopperOxidized,
}

/// Vanilla `ChestType` (`ChestType.java`): which of the three chest meshes /
/// texture variants this block renders (`single`, `left`, `right`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ChestModelHalf {
    Single,
    Left,
    Right,
}

/// The chest block state's horizontal `facing` property
/// (`ChestBlock.FACING`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ChestModelFacing {
    North,
    South,
    West,
    East,
}

impl ChestModelFacing {
    fn parse(value: &str) -> Option<Self> {
        match value {
            "north" => Some(Self::North),
            "south" => Some(Self::South),
            "west" => Some(Self::West),
            "east" => Some(Self::East),
            _ => None,
        }
    }

    /// Vanilla `Direction.getClockWise` on the Y axis.
    fn clockwise(self) -> Self {
        match self {
            Self::North => Self::East,
            Self::East => Self::South,
            Self::South => Self::West,
            Self::West => Self::North,
        }
    }

    /// Vanilla `Direction.getCounterClockWise` on the Y axis.
    fn counter_clockwise(self) -> Self {
        match self {
            Self::North => Self::West,
            Self::West => Self::South,
            Self::South => Self::East,
            Self::East => Self::North,
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
}

/// One chest block's per-frame render source: everything the renderer's chest
/// model submission needs except light (sampled on the projection side, like
/// entity light).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ChestModelSourceState {
    pub pos: BlockPos,
    pub kind: ChestModelKind,
    pub half: ChestModelHalf,
    pub facing: ChestModelFacing,
    /// The raw combined lid openness for this frame: vanilla
    /// `ChestBlock.opennessCombiner` — `max` of both halves' lerped
    /// `ChestLidController.getOpenness(partialTick)` for a joined double
    /// chest, the own half's openness otherwise. The renderer applies the
    /// `1 - (1 - o)^3` easing and the `-o * 90°` lid rotation
    /// (`ChestRenderer.submit` / `ChestModel.setupAnim`).
    pub openness: f32,
    /// The other half's block position when this block is a joined double
    /// chest (same block, opposite `type`), for the vanilla
    /// `BrightnessCombiner` per-component light max. `None` for singles and
    /// for halves whose partner is missing or mismatched (vanilla falls back
    /// to the single-block combine).
    pub partner_pos: Option<BlockPos>,
}

/// Maps a chest-family block name to its texture family. `None` for every
/// non-chest block (including `minecraft:barrel` and shulker boxes, which are
/// not `ChestRenderer` blocks).
pub fn chest_model_kind_for_block_name(block_name: &str) -> Option<ChestModelKind> {
    match block_name {
        "minecraft:chest" => Some(ChestModelKind::Normal),
        "minecraft:trapped_chest" => Some(ChestModelKind::Trapped),
        "minecraft:ender_chest" => Some(ChestModelKind::Ender),
        "minecraft:copper_chest" | "minecraft:waxed_copper_chest" => Some(ChestModelKind::Copper),
        "minecraft:exposed_copper_chest" | "minecraft:waxed_exposed_copper_chest" => {
            Some(ChestModelKind::CopperExposed)
        }
        "minecraft:weathered_copper_chest" | "minecraft:waxed_weathered_copper_chest" => {
            Some(ChestModelKind::CopperWeathered)
        }
        "minecraft:oxidized_copper_chest" | "minecraft:waxed_oxidized_copper_chest" => {
            Some(ChestModelKind::CopperOxidized)
        }
        _ => None,
    }
}

impl WorldStore {
    /// Applies a `BlockEvent` to the chest lid tracker, transcribing the
    /// client dispatch chain `Level.blockEvent` -> `BaseEntityBlock.triggerEvent`
    /// -> `ChestBlockEntity.triggerEvent`: only event id `1`
    /// (`ChestBlockEntity.EVENT_SET_OPEN_COUNT`) on a block position whose
    /// *current* block state is a chest-family block reaches a lid controller,
    /// which sets `shouldBeOpen(count > 0)`.
    pub(crate) fn update_chest_lid_from_block_event(&mut self, pos: BlockPos, b0: u8, b1: u8) {
        if b0 != 1 {
            return;
        }
        let is_chest = self
            .probe_block(pos)
            .and_then(|probe| probe.block_name)
            .as_deref()
            .and_then(chest_model_kind_for_block_name)
            .is_some();
        if !is_chest {
            return;
        }
        let should_be_open = b1 > 0;
        if let Some(lid) = self.chest_lids.iter_mut().find(|lid| lid.pos == pos) {
            lid.should_be_open = should_be_open;
        } else {
            self.chest_lids.push(ChestLidState {
                pos,
                should_be_open,
                openness: 0.0,
                o_openness: 0.0,
            });
        }
    }

    /// Advances every tracked chest lid by `ticks` client ticks, transcribing
    /// `ChestLidController.tickLid`: `oOpenness = openness`, then step `0.1`
    /// toward the `shouldBeOpen` target with `[0, 1]` clamping. Entries whose
    /// block is no longer a chest (destroyed or unloaded — vanilla drops the
    /// block entity with the block/chunk) and entries fully at rest closed are
    /// pruned so the tracker only holds animating or open lids.
    pub fn advance_chest_lid_ticks(&mut self, ticks: u32) {
        if ticks == 0 || self.chest_lids.is_empty() {
            return;
        }
        let mut lids = std::mem::take(&mut self.chest_lids);
        lids.retain(|lid| {
            self.probe_block(lid.pos)
                .and_then(|probe| probe.block_name)
                .as_deref()
                .and_then(chest_model_kind_for_block_name)
                .is_some()
        });
        let steps = ticks.min(CHEST_LID_SATURATION_TICKS);
        for lid in &mut lids {
            for _ in 0..steps {
                lid.o_openness = lid.openness;
                if !lid.should_be_open && lid.openness > 0.0 {
                    lid.openness = (lid.openness - VANILLA_CHEST_LID_OPENNESS_STEP).max(0.0);
                } else if lid.should_be_open && lid.openness < 1.0 {
                    lid.openness = (lid.openness + VANILLA_CHEST_LID_OPENNESS_STEP).min(1.0);
                }
            }
        }
        lids.retain(|lid| lid.should_be_open || lid.openness > 0.0 || lid.o_openness > 0.0);
        self.chest_lids = lids;
    }

    /// Vanilla `ChestLidController.getOpenness(partialTick)`:
    /// `Mth.lerp(a, oOpenness, openness)`. `0.0` for untracked positions (a
    /// chest that never received an open-count event rests closed).
    pub fn chest_lid_openness_at(&self, pos: BlockPos, partial_tick: f32) -> f32 {
        self.chest_lids
            .iter()
            .find(|lid| lid.pos == pos)
            .map(|lid| lid.o_openness + (lid.openness - lid.o_openness) * partial_tick)
            .unwrap_or(0.0)
    }

    pub fn chest_lid_states(&self) -> &[ChestLidState] {
        &self.chest_lids
    }

    /// Enumerates every chest-family block in the loaded chunks as a render
    /// source, deriving position/facing/half from the block state (the vanilla
    /// client materialises a `ChestBlockEntity` per chest block state) and the
    /// per-frame openness from the lid tracker with the vanilla double-chest
    /// combine. Sections whose block palette holds no chest state are skipped
    /// wholesale, so the scan cost tracks the number of chest-bearing
    /// sections, not the world size. The result is sorted by position for a
    /// deterministic frame order.
    pub fn chest_model_source_states(&self, partial_tick: f32) -> Vec<ChestModelSourceState> {
        let mut states = Vec::new();
        for chunk in &self.chunks {
            self.collect_chunk_chest_model_source_states(chunk, partial_tick, &mut states);
        }
        states.sort_by_key(|state| (state.pos.y, state.pos.z, state.pos.x));
        states
    }

    fn collect_chunk_chest_model_source_states(
        &self,
        chunk: &ChunkColumn,
        partial_tick: f32,
        states: &mut Vec<ChestModelSourceState>,
    ) {
        for (section_index, section) in chunk.sections.iter().enumerate() {
            if !section_palette_may_contain_chest(
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
                let Some(kind) = chest_model_kind_for_block_name(&block_state.name) else {
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
                    .and_then(|value| ChestModelFacing::parse(value))
                    .unwrap_or(ChestModelFacing::North);
                let half = match block_state.properties.get("type").map(String::as_str) {
                    Some("left") => ChestModelHalf::Left,
                    Some("right") => ChestModelHalf::Right,
                    _ => ChestModelHalf::Single,
                };
                let partner_pos =
                    self.joined_chest_partner_pos(pos, &block_state.name, facing, half);
                let mut openness = self.chest_lid_openness_at(pos, partial_tick);
                if let Some(partner_pos) = partner_pos {
                    openness = openness.max(self.chest_lid_openness_at(partner_pos, partial_tick));
                }
                states.push(ChestModelSourceState {
                    pos,
                    kind,
                    half,
                    facing,
                    openness,
                    partner_pos,
                });
            }
        }
    }

    /// Vanilla `ChestBlock.getConnectedBlockPos` + the `DoubleBlockCombiner`
    /// pairing check: a `left` half connects toward `facing.getClockWise()`, a
    /// `right` half toward `facing.getCounterClockWise()`, and the pairing
    /// only holds when the neighbour is the same block with the opposite
    /// `type`. Returns `None` for singles and broken pairings.
    fn joined_chest_partner_pos(
        &self,
        pos: BlockPos,
        block_name: &str,
        facing: ChestModelFacing,
        half: ChestModelHalf,
    ) -> Option<BlockPos> {
        let connected = match half {
            ChestModelHalf::Single => return None,
            ChestModelHalf::Left => facing.clockwise(),
            ChestModelHalf::Right => facing.counter_clockwise(),
        };
        let (step_x, step_z) = connected.step();
        let partner_pos = BlockPos {
            x: pos.x + step_x,
            y: pos.y,
            z: pos.z + step_z,
        };
        let partner = self.probe_block(partner_pos)?;
        if partner.block_name.as_deref() != Some(block_name) {
            return None;
        }
        let expected_partner_type = match half {
            ChestModelHalf::Left => "right",
            ChestModelHalf::Right => "left",
            ChestModelHalf::Single => unreachable!(),
        };
        (partner.block_properties.get("type").map(String::as_str) == Some(expected_partner_type))
            .then_some(partner_pos)
    }
}

/// Whether a section's block palette can hold a chest-family state. Local and
/// single-value palettes are answered from the palette id list; a global
/// palette stores raw state ids, so it must be scanned.
fn section_palette_may_contain_chest(
    palette_global_ids: &[i32],
    palette_kind: PaletteKind,
    registries: &RegistrySet,
) -> bool {
    match palette_kind {
        PaletteKind::SingleValue | PaletteKind::Local => {
            palette_global_ids.iter().any(|global_id| {
                registries
                    .block_state(*global_id)
                    .is_some_and(|state| chest_model_kind_for_block_name(&state.name).is_some())
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
        BlockEvent as ProtocolBlockEvent, BlockPos as ProtocolBlockPos,
        BlockUpdate as ProtocolBlockUpdate,
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
        assert!(world.apply_block_update(ProtocolBlockUpdate {
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

    #[test]
    fn block_event_opens_and_ticks_chest_lid_like_vanilla() {
        let mut world = world_with_air_chunk();
        let pos = BlockPos { x: 3, y: 4, z: 5 };
        set_block(
            &mut world,
            pos,
            "minecraft:chest",
            &[
                ("facing", "north"),
                ("type", "single"),
                ("waterlogged", "false"),
            ],
        );
        // ChestBlockEntity.triggerEvent(1, count): shouldBeOpen(count > 0).
        send_block_event(&mut world, pos, 1, 1);
        assert_eq!(
            world.chest_lid_states(),
            &[ChestLidState {
                pos,
                should_be_open: true,
                openness: 0.0,
                o_openness: 0.0,
            }]
        );
        // ChestLidController.tickLid: 0.1 per tick with oOpenness trailing.
        world.advance_chest_lid_ticks(1);
        let lid = world.chest_lid_states()[0];
        assert!((lid.openness - 0.1).abs() < 1e-6);
        assert_eq!(lid.o_openness, 0.0);
        // getOpenness(0.5) = lerp(0.5, 0.0, 0.1).
        assert!((world.chest_lid_openness_at(pos, 0.5) - 0.05).abs() < 1e-6);
        // Saturates at 1.0 (Math.min clamp) regardless of extra batched ticks.
        world.advance_chest_lid_ticks(100);
        let lid = world.chest_lid_states()[0];
        assert_eq!(lid.openness, 1.0);
        assert_eq!(lid.o_openness, 1.0);
        // Close: count 0 -> steps back down and the resting-closed entry prunes.
        send_block_event(&mut world, pos, 1, 0);
        world.advance_chest_lid_ticks(1);
        let lid = world.chest_lid_states()[0];
        assert!((lid.openness - 0.9).abs() < 1e-6);
        assert_eq!(lid.o_openness, 1.0);
        world.advance_chest_lid_ticks(100);
        assert!(world.chest_lid_states().is_empty());
        assert_eq!(world.chest_lid_openness_at(pos, 0.5), 0.0);
    }

    #[test]
    fn block_event_ignores_non_chests_and_other_event_ids() {
        let mut world = world_with_air_chunk();
        let chest_pos = BlockPos { x: 3, y: 4, z: 5 };
        set_block(
            &mut world,
            chest_pos,
            "minecraft:chest",
            &[
                ("facing", "north"),
                ("type", "single"),
                ("waterlogged", "false"),
            ],
        );
        // Wrong event id on a chest (only EVENT_SET_OPEN_COUNT = 1 reaches the lid).
        send_block_event(&mut world, chest_pos, 2, 1);
        // Event id 1 on a non-chest position (Level.blockEvent dispatches on the
        // current block state, which is air here).
        send_block_event(&mut world, BlockPos { x: 1, y: 1, z: 1 }, 1, 1);
        assert!(world.chest_lid_states().is_empty());
    }

    #[test]
    fn destroyed_chest_lid_state_prunes_on_tick() {
        let mut world = world_with_air_chunk();
        let pos = BlockPos { x: 3, y: 4, z: 5 };
        set_block(
            &mut world,
            pos,
            "minecraft:chest",
            &[
                ("facing", "north"),
                ("type", "single"),
                ("waterlogged", "false"),
            ],
        );
        send_block_event(&mut world, pos, 1, 1);
        world.advance_chest_lid_ticks(2);
        assert_eq!(world.chest_lid_states().len(), 1);
        // Breaking the chest drops the block entity (and its lid controller).
        set_block(&mut world, pos, "minecraft:air", &[]);
        world.advance_chest_lid_ticks(1);
        assert!(world.chest_lid_states().is_empty());
    }

    #[test]
    fn enumerates_chest_sources_with_facing_half_kind_and_double_pairing() {
        let mut world = world_with_air_chunk();
        // A north-facing double chest: the LEFT half's partner sits toward
        // facing.getClockWise() = east (+x).
        let left_pos = BlockPos { x: 6, y: 4, z: 5 };
        let right_pos = BlockPos { x: 7, y: 4, z: 5 };
        set_block(
            &mut world,
            left_pos,
            "minecraft:chest",
            &[
                ("facing", "north"),
                ("type", "left"),
                ("waterlogged", "false"),
            ],
        );
        set_block(
            &mut world,
            right_pos,
            "minecraft:chest",
            &[
                ("facing", "north"),
                ("type", "right"),
                ("waterlogged", "false"),
            ],
        );
        let trapped_pos = BlockPos { x: 1, y: 2, z: 3 };
        set_block(
            &mut world,
            trapped_pos,
            "minecraft:trapped_chest",
            &[
                ("facing", "east"),
                ("type", "single"),
                ("waterlogged", "false"),
            ],
        );
        let ender_pos = BlockPos { x: 2, y: 2, z: 3 };
        set_block(
            &mut world,
            ender_pos,
            "minecraft:ender_chest",
            &[("facing", "south"), ("waterlogged", "false")],
        );
        let copper_pos = BlockPos { x: 3, y: 2, z: 3 };
        set_block(
            &mut world,
            copper_pos,
            "minecraft:waxed_exposed_copper_chest",
            &[
                ("facing", "west"),
                ("type", "single"),
                ("waterlogged", "false"),
            ],
        );
        // Open only the right half: opennessCombiner max animates both halves.
        send_block_event(&mut world, right_pos, 1, 2);
        world.advance_chest_lid_ticks(1);

        let sources = world.chest_model_source_states(1.0);
        assert_eq!(sources.len(), 5);
        // Sorted by (y, z, x): the three y=2 chests first.
        assert_eq!(
            sources[0],
            ChestModelSourceState {
                pos: trapped_pos,
                kind: ChestModelKind::Trapped,
                half: ChestModelHalf::Single,
                facing: ChestModelFacing::East,
                openness: 0.0,
                partner_pos: None,
            }
        );
        assert_eq!(sources[1].kind, ChestModelKind::Ender);
        assert_eq!(sources[1].half, ChestModelHalf::Single);
        assert_eq!(sources[1].facing, ChestModelFacing::South);
        assert_eq!(sources[2].kind, ChestModelKind::CopperExposed);
        assert_eq!(sources[2].facing, ChestModelFacing::West);
        let left = &sources[3];
        let right = &sources[4];
        assert_eq!(left.pos, left_pos);
        assert_eq!(left.half, ChestModelHalf::Left);
        assert_eq!(left.facing, ChestModelFacing::North);
        assert_eq!(left.partner_pos, Some(right_pos));
        assert_eq!(right.pos, right_pos);
        assert_eq!(right.half, ChestModelHalf::Right);
        assert_eq!(right.partner_pos, Some(left_pos));
        assert!((left.openness - 0.1).abs() < 1e-6);
        assert!((right.openness - 0.1).abs() < 1e-6);
    }

    #[test]
    fn broken_double_pairing_falls_back_to_single_combine() {
        let mut world = world_with_air_chunk();
        // A lone LEFT half whose partner position holds air: vanilla's
        // DoubleBlockCombiner falls back to the single combine.
        let pos = BlockPos { x: 6, y: 4, z: 5 };
        set_block(
            &mut world,
            pos,
            "minecraft:chest",
            &[
                ("facing", "north"),
                ("type", "left"),
                ("waterlogged", "false"),
            ],
        );
        let sources = world.chest_model_source_states(0.0);
        assert_eq!(sources.len(), 1);
        assert_eq!(sources[0].half, ChestModelHalf::Left);
        assert_eq!(sources[0].partner_pos, None);
    }

    #[test]
    fn login_clears_chest_lid_states() {
        let mut world = world_with_air_chunk();
        let pos = BlockPos { x: 3, y: 4, z: 5 };
        set_block(
            &mut world,
            pos,
            "minecraft:chest",
            &[
                ("facing", "north"),
                ("type", "single"),
                ("waterlogged", "false"),
            ],
        );
        send_block_event(&mut world, pos, 1, 1);
        assert_eq!(world.chest_lid_states().len(), 1);
        world.apply_login(&crate::chest_lids::tests::login_packet());
        assert!(world.chest_lid_states().is_empty());
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
