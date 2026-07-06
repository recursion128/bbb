//! Decorated pot block render sources: the sherd faces and the wobble.
//!
//! Vanilla renders decorated pots through `BlockEntityRenderDispatcher` +
//! `DecoratedPotRenderer`: per pot block entity, a `DecoratedPotRenderState`
//! carrying the block state's `HORIZONTAL_FACING`
//! (`DecoratedPotBlockEntity.getDirection`), the four-face `PotDecorations`
//! (BE NBT `sherds`, an up-to-4 item-id list in `back/left/right/front`
//! order — `PotDecorations.CODEC`/`ordered()`), and the wobble progress
//! `(gameTime - wobbleStartedAtTick + partialTicks) / style.duration`. The
//! wobble starts from `Level.blockEvent(pos, block, 1, style.ordinal())`
//! (`DecoratedPotBlockEntity.wobble`, java:160-164) and
//! `triggerEvent(1, data)` (java:167-175) records the start tick + style
//! (`WobbleStyle.POSITIVE.duration = 7`, `NEGATIVE.duration = 10`). bbb has
//! no per-position block-entity objects nor a client game-time clock, so the
//! wobble lives here as a flat `Vec<DecoratedPotWobbleState>` counting ticks
//! since the event (equivalent to `gameTime - wobbleStartedAtTick`), and the
//! sherds ride the chunk block-entity records
//! (`DecoratedPotSherdsState`) like the sign text.

use serde::{Deserialize, Serialize};

use crate::{BlockPos, ChunkColumn, PaletteKind, RegistrySet, WorldStore};

/// Vanilla `DecoratedPotBlockEntity.WobbleStyle`: `POSITIVE(7)` (an item was
/// stored/taken) and `NEGATIVE(10)` (the interaction failed).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DecoratedPotWobbleStyleKind {
    Positive,
    Negative,
}

impl DecoratedPotWobbleStyleKind {
    /// Vanilla `WobbleStyle.duration` in ticks (`POSITIVE(7)` / `NEGATIVE(10)`).
    pub fn duration_ticks(self) -> u32 {
        match self {
            Self::Positive => 7,
            Self::Negative => 10,
        }
    }
}

/// The longest wobble (`NEGATIVE`, 10 ticks) prunes within 10 steps, so
/// batching more ticks is indistinguishable from 10.
const DECORATED_POT_WOBBLE_SATURATION_TICKS: u32 = 10;

/// One decorated pot's wobble state (`wobbleStartedAtTick` /
/// `lastWobbleStyle` re-expressed as ticks since the block event), keyed by
/// the pot block position. Only wobbling pots are tracked — once the style
/// duration elapses the wobble angle is exactly zero, like vanilla's
/// `wobbleProgress > 1` render gate.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct DecoratedPotWobbleState {
    pub pos: BlockPos,
    pub ticks: u32,
    pub style: DecoratedPotWobbleStyleKind,
}

/// The decorated pot block state's `HORIZONTAL_FACING` property.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DecoratedPotFacing {
    North,
    South,
    West,
    East,
}

impl DecoratedPotFacing {
    fn parse(value: &str) -> Option<Self> {
        match value {
            "north" => Some(Self::North),
            "south" => Some(Self::South),
            "west" => Some(Self::West),
            "east" => Some(Self::East),
            _ => None,
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

/// One decorated pot's per-frame wobble source: the vanilla
/// `DecoratedPotRenderState.wobbleStyle` + `wobbleProgress` pair while the
/// wobble is running.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DecoratedPotWobbleSource {
    pub style: DecoratedPotWobbleStyleKind,
    /// Vanilla `(gameTime - wobbleStartedAtTick + partialTicks) /
    /// style.duration`, always within `(0, 1]` for a tracked wobble.
    pub progress: f32,
}

/// One decorated pot block's per-frame render source: the vanilla
/// `DecoratedPotRenderState` fields except light (sampled on the projection
/// side).
#[derive(Debug, Clone, PartialEq)]
pub struct DecoratedPotModelSourceState {
    pub pos: BlockPos,
    pub facing: DecoratedPotFacing,
    /// The four sherd faces (vanilla `PotDecorations` `back/left/right/front`
    /// item order): `Some(item_id)` for a real sherd item, `None` for an
    /// empty face (vanilla `Items.BRICK` / a missing list entry), both of
    /// which render the plain `decorated_pot_side` texture.
    pub back: Option<String>,
    pub left: Option<String>,
    pub right: Option<String>,
    pub front: Option<String>,
    pub wobble: Option<DecoratedPotWobbleSource>,
}

/// Whether a block name is the decorated pot block
/// (`minecraft:decorated_pot`, the single `DecoratedPotBlock` registration).
pub fn is_decorated_pot_block_name(block_name: &str) -> bool {
    block_name == "minecraft:decorated_pot"
}

impl WorldStore {
    /// Applies a `BlockEvent` to the pot wobble tracker, transcribing the
    /// client dispatch chain `Level.blockEvent` ->
    /// `BaseEntityBlock.triggerEvent` ->
    /// `DecoratedPotBlockEntity.triggerEvent` (java:167-175): only event id
    /// `1` (`EVENT_POT_WOBBLES`) with a data byte inside the `WobbleStyle`
    /// ordinal range (`0` POSITIVE / `1` NEGATIVE) on a block position whose
    /// *current* block state is a decorated pot restarts the wobble
    /// (`wobbleStartedAtTick = gameTime`, `lastWobbleStyle = values()[data]`).
    pub(crate) fn update_decorated_pot_wobble_from_block_event(
        &mut self,
        pos: BlockPos,
        b0: u8,
        b1: u8,
    ) {
        if b0 != 1 {
            return;
        }
        let is_pot = self
            .probe_block(pos)
            .and_then(|probe| probe.block_name)
            .as_deref()
            .is_some_and(is_decorated_pot_block_name);
        if !is_pot {
            return;
        }
        let style = match b1 {
            0 => DecoratedPotWobbleStyleKind::Positive,
            1 => DecoratedPotWobbleStyleKind::Negative,
            _ => return,
        };
        if let Some(wobble) = self
            .decorated_pot_wobbles
            .iter_mut()
            .find(|wobble| wobble.pos == pos)
        {
            wobble.style = style;
            wobble.ticks = 0;
        } else {
            self.decorated_pot_wobbles.push(DecoratedPotWobbleState {
                pos,
                ticks: 0,
                style,
            });
        }
    }

    /// Advances every tracked pot wobble by `ticks` client ticks. Vanilla has
    /// no per-tick pot step — the renderer reads `gameTime -
    /// wobbleStartedAtTick` — so this counter is that difference. Entries
    /// whose block is no longer a decorated pot and entries past their style
    /// duration (vanilla stops rendering the wobble once `wobbleProgress >
    /// 1`; at exactly `1.0` both style formulas evaluate to a zero angle) are
    /// pruned.
    pub fn advance_decorated_pot_wobble_ticks(&mut self, ticks: u32) {
        if ticks == 0 || self.decorated_pot_wobbles.is_empty() {
            return;
        }
        let mut wobbles = std::mem::take(&mut self.decorated_pot_wobbles);
        wobbles.retain(|wobble| {
            self.probe_block(wobble.pos)
                .and_then(|probe| probe.block_name)
                .as_deref()
                .is_some_and(is_decorated_pot_block_name)
        });
        let steps = ticks.min(DECORATED_POT_WOBBLE_SATURATION_TICKS);
        for wobble in &mut wobbles {
            wobble.ticks += steps;
        }
        wobbles.retain(|wobble| wobble.ticks < wobble.style.duration_ticks());
        self.decorated_pot_wobbles = wobbles;
    }

    pub fn decorated_pot_wobble_states(&self) -> &[DecoratedPotWobbleState] {
        &self.decorated_pot_wobbles
    }

    /// Enumerates every decorated pot block in the loaded chunks as a render
    /// source, deriving the facing from the block state, the four sherd
    /// faces from the stored block-entity records
    /// (`DecoratedPotSherdsState`; a pot without a record renders four
    /// plain sides, matching a fresh `PotDecorations.EMPTY` block entity),
    /// and the wobble progress from the wobble tracker
    /// (`DecoratedPotRenderer.extractRenderState`'s `(ticks + partialTicks) /
    /// duration`). Sections whose block palette holds no pot state are
    /// skipped wholesale, mirroring the chest/sign palette pre-check. Sorted
    /// by position for a deterministic frame order.
    pub fn decorated_pot_model_source_states(
        &self,
        partial_tick: f32,
    ) -> Vec<DecoratedPotModelSourceState> {
        let mut states = Vec::new();
        for chunk in &self.chunks {
            self.collect_chunk_decorated_pot_model_source_states(chunk, partial_tick, &mut states);
        }
        states.sort_by_key(|state| (state.pos.y, state.pos.z, state.pos.x));
        states
    }

    fn collect_chunk_decorated_pot_model_source_states(
        &self,
        chunk: &ChunkColumn,
        partial_tick: f32,
        states: &mut Vec<DecoratedPotModelSourceState>,
    ) {
        for (section_index, section) in chunk.sections.iter().enumerate() {
            if !section_palette_may_contain_decorated_pot(
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
                if !is_decorated_pot_block_name(&block_state.name) {
                    continue;
                }
                let pos = BlockPos {
                    x: chunk.pos.x * 16 + (index & 0xF) as i32,
                    y: section_min_y + (index >> 8) as i32,
                    z: chunk.pos.z * 16 + ((index >> 4) & 0xF) as i32,
                };
                let facing = block_state
                    .properties
                    .get("facing")
                    .and_then(|value| DecoratedPotFacing::parse(value))
                    .unwrap_or(DecoratedPotFacing::North);
                let sherds = self
                    .decorated_pot_sherds_at(pos)
                    .cloned()
                    .unwrap_or_default();
                let wobble = self
                    .decorated_pot_wobbles
                    .iter()
                    .find(|wobble| wobble.pos == pos)
                    .map(|wobble| DecoratedPotWobbleSource {
                        style: wobble.style,
                        progress: (wobble.ticks as f32 + partial_tick)
                            / wobble.style.duration_ticks() as f32,
                    });
                states.push(DecoratedPotModelSourceState {
                    pos,
                    facing,
                    back: sherds.back,
                    left: sherds.left,
                    right: sherds.right,
                    front: sherds.front,
                    wobble,
                });
            }
        }
    }
}

/// Whether a section's block palette can hold the decorated pot state. Local
/// and single-value palettes are answered from the palette id list; a global
/// palette stores raw state ids, so it must be scanned.
fn section_palette_may_contain_decorated_pot(
    palette_global_ids: &[i32],
    palette_kind: PaletteKind,
    registries: &RegistrySet,
) -> bool {
    match palette_kind {
        PaletteKind::SingleValue | PaletteKind::Local => {
            palette_global_ids.iter().any(|global_id| {
                registries
                    .block_state(*global_id)
                    .is_some_and(|state| is_decorated_pot_block_name(&state.name))
            })
        }
        PaletteKind::Global => true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        ChunkPos, ChunkSection, ChunkState, DecoratedPotSherdsState, LightData, PaletteDomain,
        PalettedContainerData, WorldDimension,
    };
    use bbb_protocol::packets::{
        BlockEntityData, BlockEvent as ProtocolBlockEvent, BlockPos as ProtocolBlockPos,
        BlockUpdate,
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

    fn set_decorated_pot(world: &mut WorldStore, pos: BlockPos, facing: &str) {
        set_block(
            world,
            pos,
            "minecraft:decorated_pot",
            &[
                ("cracked", "false"),
                ("facing", facing),
                ("waterlogged", "false"),
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

    /// A `DecoratedPotBlockEntity.saveAdditional` payload: a root compound
    /// with a `sherds` string list (`PotDecorations.CODEC` — the registry
    /// item-id list in `back/left/right/front` order).
    fn pot_nbt(sherds: &[&str]) -> Vec<u8> {
        let mut payload = vec![10];
        payload.push(9);
        write_mutf8(&mut payload, "sherds");
        payload.push(8);
        payload.extend_from_slice(&(sherds.len() as i32).to_be_bytes());
        for sherd in sherds {
            write_mutf8(&mut payload, sherd);
        }
        payload.push(0);
        payload
    }

    fn write_mutf8(out: &mut Vec<u8>, value: &str) {
        let bytes = value.as_bytes();
        out.extend_from_slice(&(bytes.len() as u16).to_be_bytes());
        out.extend_from_slice(bytes);
    }

    fn apply_pot_sherds(world: &mut WorldStore, pos: BlockPos, nbt: Vec<u8>) {
        assert!(world
            .apply_block_entity_data(BlockEntityData {
                pos: ProtocolBlockPos {
                    x: pos.x,
                    y: pos.y,
                    z: pos.z,
                },
                block_entity_type_id: 0,
                raw_nbt: nbt,
            })
            .unwrap());
    }

    #[test]
    fn parses_sherds_nbt_in_back_left_right_front_order() {
        let mut world = world_with_air_chunk();
        let pos = BlockPos { x: 3, y: 4, z: 5 };
        set_decorated_pot(&mut world, pos, "north");
        // PotDecorations.ordered(): [back, left, right, front], BRICK = empty.
        apply_pot_sherds(
            &mut world,
            pos,
            pot_nbt(&[
                "minecraft:angler_pottery_sherd",
                "minecraft:brick",
                "minecraft:skull_pottery_sherd",
                "minecraft:danger_pottery_sherd",
            ]),
        );
        let sherds = world.decorated_pot_sherds_at(pos).unwrap();
        assert_eq!(
            sherds,
            &DecoratedPotSherdsState {
                back: Some("minecraft:angler_pottery_sherd".to_string()),
                left: None,
                right: Some("minecraft:skull_pottery_sherd".to_string()),
                front: Some("minecraft:danger_pottery_sherd".to_string()),
            }
        );
        // A short list leaves the remaining faces empty
        // (`PotDecorations.getItem` returns empty past the list end).
        apply_pot_sherds(&mut world, pos, pot_nbt(&["minecraft:heart_pottery_sherd"]));
        let sherds = world.decorated_pot_sherds_at(pos).unwrap();
        assert_eq!(
            sherds,
            &DecoratedPotSherdsState {
                back: Some("minecraft:heart_pottery_sherd".to_string()),
                left: None,
                right: None,
                front: None,
            }
        );
        // Replacing the block drops the record (vanilla Level.removeBlockEntity).
        set_block(&mut world, pos, "minecraft:air", &[]);
        assert_eq!(world.decorated_pot_sherds_at(pos), None);
    }

    #[test]
    fn block_event_starts_and_expires_the_wobble_like_vanilla() {
        let mut world = world_with_air_chunk();
        let pos = BlockPos { x: 3, y: 4, z: 5 };
        set_decorated_pot(&mut world, pos, "north");
        // triggerEvent(1, 0): WobbleStyle.POSITIVE (7 ticks).
        send_block_event(&mut world, pos, 1, 0);
        assert_eq!(
            world.decorated_pot_wobble_states(),
            &[DecoratedPotWobbleState {
                pos,
                ticks: 0,
                style: DecoratedPotWobbleStyleKind::Positive,
            }]
        );
        world.advance_decorated_pot_wobble_ticks(6);
        assert_eq!(world.decorated_pot_wobble_states()[0].ticks, 6);
        // The 7th tick reaches the POSITIVE duration: progress >= 1, pruned.
        world.advance_decorated_pot_wobble_ticks(1);
        assert!(world.decorated_pot_wobble_states().is_empty());
        // triggerEvent(1, 1): NEGATIVE (10 ticks) restarts the counter; a
        // re-event mid-wobble resets it (wobbleStartedAtTick = gameTime).
        send_block_event(&mut world, pos, 1, 1);
        world.advance_decorated_pot_wobble_ticks(9);
        assert_eq!(world.decorated_pot_wobble_states()[0].ticks, 9);
        send_block_event(&mut world, pos, 1, 0);
        assert_eq!(
            world.decorated_pot_wobble_states(),
            &[DecoratedPotWobbleState {
                pos,
                ticks: 0,
                style: DecoratedPotWobbleStyleKind::Positive,
            }]
        );
    }

    #[test]
    fn block_event_ignores_non_pots_out_of_range_styles_and_other_ids() {
        let mut world = world_with_air_chunk();
        let pot_pos = BlockPos { x: 3, y: 4, z: 5 };
        set_decorated_pot(&mut world, pot_pos, "north");
        // Wrong event id on a pot.
        send_block_event(&mut world, pot_pos, 2, 0);
        // Style ordinal out of the WobbleStyle range (vanilla requires
        // `data < values().length`).
        send_block_event(&mut world, pot_pos, 1, 2);
        // Event id 1 on a non-pot position.
        send_block_event(&mut world, BlockPos { x: 1, y: 1, z: 1 }, 1, 0);
        assert!(world.decorated_pot_wobble_states().is_empty());
    }

    #[test]
    fn destroyed_pot_wobble_state_prunes_on_tick() {
        let mut world = world_with_air_chunk();
        let pos = BlockPos { x: 3, y: 4, z: 5 };
        set_decorated_pot(&mut world, pos, "north");
        send_block_event(&mut world, pos, 1, 1);
        world.advance_decorated_pot_wobble_ticks(2);
        assert_eq!(world.decorated_pot_wobble_states().len(), 1);
        set_block(&mut world, pos, "minecraft:air", &[]);
        world.advance_decorated_pot_wobble_ticks(1);
        assert!(world.decorated_pot_wobble_states().is_empty());
    }

    #[test]
    fn enumerates_pot_sources_with_facing_sherds_and_wobble_progress() {
        let mut world = world_with_air_chunk();
        let wobbling_pos = BlockPos { x: 3, y: 4, z: 5 };
        let plain_pos = BlockPos { x: 6, y: 4, z: 5 };
        set_decorated_pot(&mut world, wobbling_pos, "east");
        set_decorated_pot(&mut world, plain_pos, "north");
        apply_pot_sherds(
            &mut world,
            wobbling_pos,
            pot_nbt(&[
                "minecraft:brick",
                "minecraft:howl_pottery_sherd",
                "minecraft:brick",
                "minecraft:brick",
            ]),
        );
        send_block_event(&mut world, wobbling_pos, 1, 1);
        world.advance_decorated_pot_wobble_ticks(4);

        let sources = world.decorated_pot_model_source_states(0.5);
        assert_eq!(sources.len(), 2);
        let wobbling = &sources[0];
        assert_eq!(wobbling.pos, wobbling_pos);
        assert_eq!(wobbling.facing, DecoratedPotFacing::East);
        assert_eq!(wobbling.back, None);
        assert_eq!(
            wobbling.left,
            Some("minecraft:howl_pottery_sherd".to_string())
        );
        assert_eq!(wobbling.right, None);
        assert_eq!(wobbling.front, None);
        // (ticks + partialTicks) / NEGATIVE duration = (4 + 0.5) / 10.
        let wobble = wobbling.wobble.unwrap();
        assert_eq!(wobble.style, DecoratedPotWobbleStyleKind::Negative);
        assert!((wobble.progress - 0.45).abs() < 1e-6);
        // A pot without a BE record renders four plain sides and no wobble.
        let plain = &sources[1];
        assert_eq!(plain.facing, DecoratedPotFacing::North);
        assert_eq!(plain.back, None);
        assert_eq!(plain.left, None);
        assert_eq!(plain.right, None);
        assert_eq!(plain.front, None);
        assert_eq!(plain.wobble, None);
    }

    #[test]
    fn login_clears_pot_wobble_states() {
        let mut world = world_with_air_chunk();
        let pos = BlockPos { x: 3, y: 4, z: 5 };
        set_decorated_pot(&mut world, pos, "north");
        send_block_event(&mut world, pos, 1, 0);
        assert_eq!(world.decorated_pot_wobble_states().len(), 1);
        world.apply_login(&login_packet());
        assert!(world.decorated_pot_wobble_states().is_empty());
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
